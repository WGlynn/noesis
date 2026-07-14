//! Sub-blocks — the fast (~2 s), REVERTIBLE, contribution-gated tier under the 120 s ordering blocks.
//!
//! T9 (`internal/RESEARCH-NETWORK-CONSENSUS.md`), adapted from Ergo's "Matrix" sub-blocks — we adopt the
//! two-tier STRUCTURE but prove our OWN gate. The 120 s ordering cadence is a deliberate gift to
//! small-bandwidth / small-storage nodes and validators (the decentralization property); sub-blocks are
//! what PAY FOR that tradeoff on the UX side, soft-confirming value transactions in seconds so users do
//! not eat the 2-minute settlement latency (Will 2026-07-14 — the motivating requirement).
//!
//! ROLE SEPARATION (`docs/research/role-separation-as-design-law.md`): the two tiers split fast-soft from
//! slow-final. ORDERING blocks (the existing `Block`, 120 s) carry PoS+PoM finality; SUB-blocks carry only
//! value transactions and give OPTIMISTIC, REVERTIBLE soft-confirmation. In Noesis "confirmation" IS
//! PoS+PoM finality (PoW is finality-excluded), so a sub-block soft-confirm is revertible BY CONSTRUCTION —
//! Ergo's honest UX contract is truthful here for free.
//!
//! THE GATE, RE-DERIVED (`[[cross-port-fn-var-audit]]`): Ergo gates a sub-block at `H < T/64` (weaker PoW).
//! Our substrate's physics is CONTRIBUTION, not energy, so we gate on PoM standing instead: a sub-block
//! producer must hold finalized PoM standing ≥ a low threshold (the "low-threshold standing quorum"). This
//! keeps the fast tier Sybil-resistant on the dimension that actually secures Noesis.
//!
//! CONSENSUS-ISOLATED SHADOW (the `jul`/`wallclock`/`liveness` precedent): this slice is PURE validation
//! semantics — no networking, no gossip, and NO finalized-state mutation (revertible by construction, so it
//! never touches `Ledger::state_digest`). Slice 1 = the data model + the validity gate + the confirmation-
//! tier read side. Ordering-block ABSORPTION (soft → final) + the daemon gossip path are the next slices.
//! YAGNI v1: sub-blocks carry VALUE txs only, never contribution cells — a contribution accrues PoM over the
//! vesting window `W`, so there is nothing to fast-confirm; the first/second-class tx split is also deferred.

use crate::runtime::{Ledger, TokenTx};
use crate::{Cell, Script};
use std::collections::HashSet;

/// A sub-block: a fast, REVERTIBLE batch of value transactions proposed between two ordering blocks by a
/// contribution-weighted producer. Soft-confirmation only — never finalized state.
/// (No `Debug` derive: `TokenTx` carries none; add one to `TokenTx` first if a sub-block ever needs it.)
#[derive(Clone)]
pub struct SubBlock {
    /// The ordering-block height this sub-block builds on — the current tip. The sub-block lives in the
    /// interval AFTER ordering block `ordering_height` and before `ordering_height + 1` finalizes.
    pub ordering_height: u64,
    /// Sequence within the interval: `0, 1, 2, …` as sub-blocks accrue on top of one another. Must equal
    /// the number of already-accepted sub-blocks in this interval (a provisional soft-chain).
    pub seq: u64,
    /// The proposer's soulbound contributor id (the `Ledger::pom` / `type_script.args` key). The gate
    /// reads THIS producer's finalized PoM standing.
    pub producer: Vec<u8>,
    /// The value movements this sub-block soft-confirms. Value txs ONLY (YAGNI v1).
    pub txs: Vec<TokenTx>,
}

/// Why a sub-block is rejected by [`validate_sub_block`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubBlockViolation {
    /// The sub-block does not build on the current ordering tip (wrong interval).
    WrongOrderingHeight { tip: u64, got: u64 },
    /// The sub-block's `seq` is not the next slot in the interval (gap / reorder in the soft-chain).
    NonSequential { expected: u64, got: u64 },
    /// The producer's finalized PoM standing is below the contribution-weight gate threshold.
    InsufficientStanding { have: u64, need: u64 },
    /// A tx is non-conserving, spends a phantom/already-spent input, or double-spends within the interval.
    TxInvalidOrDoubleSpend,
}

/// The user-facing confirmation contract (T9 — Ergo's honest UX rule, verbatim intent). A value tx seen
/// in an accepted sub-block is `SoftConfirmed`: fast, but REVERTIBLE ("in the leader's working set", not
/// settled). It becomes `Final` only once an ordering block ABSORBS it AND the PoS+PoM finality gadget
/// finalizes that block. Wallets MAY show `SoftConfirmed` for low-value flows; high-value flows MUST wait
/// for `Final`. Revertibility is never hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmationTier {
    SoftConfirmed,
    Final,
}

/// The provisional token-cell live set: `ledger.token_cells` with the effects of the prior accepted
/// sub-blocks in THIS interval folded in — retire each consumed input on its FULL identity (id + lock +
/// type_script + data, the amount-binding the ordering-block gate uses), then append outputs. This is the
/// exact [`crate::runtime`] `apply_transition` token discipline, run on a NON-FINALIZED copy: the overlay
/// is discarded if the interval's ordering block does not absorb these sub-blocks (revertible).
fn provisional_live(ledger: &Ledger, prior: &[SubBlock]) -> Vec<Cell> {
    let mut live = ledger.token_cells.clone();
    for sub in prior {
        for tx in &sub.txs {
            for inp in &tx.inputs {
                live.retain(|c| {
                    !(c.id == inp.id
                        && c.lock == inp.lock
                        && c.type_script == inp.type_script
                        && c.data == inp.data)
                });
            }
        }
        for tx in &sub.txs {
            for out in &tx.outputs {
                live.push(out.clone());
            }
        }
    }
    live
}

/// Conserve + single-use for a batch of txs against a provisional live set. Mirrors the ordering-block
/// gate `token_txs_conserve_and_single_use` (a small, deliberate duplication — the sub-block tier is a
/// SEPARATE consensus surface; single-sourcing both onto one `&[Cell] × &[TokenTx]` helper is a follow).
fn txs_conserve_and_single_use(live: &[Cell], txs: &[TokenTx]) -> bool {
    let mut consumed: HashSet<(u64, Script, Script)> = HashSet::new();
    for tx in txs {
        if !tx.is_valid_in_ledger(live) {
            return false;
        }
        for inp in &tx.inputs {
            // full-identity key incl. no `data` matches the ordering-block gate's single-use key
            // (id, lock, type_script) — `data` binding is enforced by `is_valid_in_ledger` above.
            if !consumed.insert((inp.id, inp.lock.clone(), inp.type_script.clone())) {
                return false;
            }
        }
    }
    true
}

/// Validate a sub-block against the current ledger + the provisional overlay of prior accepted sub-blocks
/// in this interval. REVERTIBLE by construction — touches no finalized state, never enters `state_digest`.
///
/// `min_standing` is the CONTRIBUTION-WEIGHT gate (the Ergo-`T/64` → PoM-standing re-derivation): the
/// producer must hold finalized PoM standing ≥ `min_standing`. It is a NODE/config param for now (the
/// `wallclock`-δ precedent) — a `Constitution` field once sub-blocks wire into consensus.
pub fn validate_sub_block(
    ledger: &Ledger,
    prior: &[SubBlock],
    sub: &SubBlock,
    min_standing: u64,
) -> Result<(), SubBlockViolation> {
    // (1) builds on the current ordering tip — the interval this sub-block belongs to.
    if sub.ordering_height != ledger.height {
        return Err(SubBlockViolation::WrongOrderingHeight { tip: ledger.height, got: sub.ordering_height });
    }
    // (2) sequential within the interval: seq == number of prior accepted sub-blocks.
    let expected = prior.len() as u64;
    if sub.seq != expected {
        return Err(SubBlockViolation::NonSequential { expected, got: sub.seq });
    }
    // (3) contribution-weight gate — the fast tier's Sybil resistance on the dimension that secures Noesis.
    let standing = ledger.pom.get(&sub.producer).copied().unwrap_or(0);
    if standing < min_standing {
        return Err(SubBlockViolation::InsufficientStanding { have: standing, need: min_standing });
    }
    // (4) txs conserve + single-use against the provisional overlay (multi-hop soft-chain allowed; a
    //     double-spend of an input already consumed earlier in the interval is rejected).
    if !txs_conserve_and_single_use(&provisional_live(ledger, prior), &sub.txs) {
        return Err(SubBlockViolation::TxInvalidOrDoubleSpend);
    }
    Ok(())
}

/// The read side of the honest UX contract: classify a produced output cell id by confirmation tier.
/// `Final` iff it is live in the FINALIZED token set (`ledger.token_cells` — an ordering block absorbed it
/// and it survived); else `SoftConfirmed` iff it appears in an accepted sub-block; `None` if unknown to
/// both. A finalized output outranks a soft one (the soft → final transition is monotone).
pub fn tier_of_output(ledger: &Ledger, accepted: &[SubBlock], output_id: u64) -> Option<ConfirmationTier> {
    if ledger.token_cells.iter().any(|c| c.id == output_id) {
        return Some(ConfirmationTier::Final);
    }
    if accepted
        .iter()
        .any(|s| s.txs.iter().any(|tx| tx.outputs.iter().any(|o| o.id == output_id)))
    {
        return Some(ConfirmationTier::SoftConfirmed);
    }
    None
}
