//! Node runtime — the replicated state machine that turns the mechanism library into
//! something two participants can RUN and CONVERGE on. The library proves each rule in
//! isolation (235 unit tests); this module wires them into a node a peer can replicate.
//!
//! LEAN BY DESIGN (Bitcoin-simplicity, CONTINUE.md constraint): this is ORCHESTRATION
//! ONLY. It re-implements no mechanism. It composes four existing pieces —
//!   1. consensus-sourced ordering  (`commit_order::canonical_order`)
//!   2. the attribution gate        (`pom_scores` / temporal-novelty; the value gate is
//!                                    the v2 swap-in, see ROADMAP)
//!   3. the novelty index           (`smt::NoveltyIndex`)
//!   4. PoM-weighted finalization   (`consensus::finalizes_hybrid`)
//! into a deterministic block loop. Two honest nodes that finalize the same blocks hold
//! byte-identical ledgers — that is the convergence property the 2-node harness asserts.
//!
//! The block shape (a commit-reveal batch ordered by a consensus-sourced shuffle) is
//! derived from VibeSwap's CommitRevealAuction; the per-contributor PoM attribution it
//! settles is the same value/flow surface PsiNet's CRPC scores pairwise. The matrix that
//! governs HOW value is measured is carried as a genesis [`Constitution`]: physics-anchored
//! meta-rules near-immutable, the dimension SET verifier-gated-extensible, the WEIGHTS
//! governance-bounded (ROADMAP 2026-06-16 (c) — the completeness/weights cleavage).

use crate::commit_order::{canonical_order, is_canonical_order, Committed};
use crate::consensus::{self, Mix, Validator, NCI, TWO_THIRDS_BPS};
use crate::smt::{Hash, NoveltyIndex};
use crate::{coverage, pom_scores, Cell};
use std::collections::HashMap;

// ============ Constitution — how value is measured (the amendment frame) ============

/// Genesis-fixed parameters governing the measurement and its finalization. The
/// value-dimension matrix is governed in THREE layers (ROADMAP 2026-06-16 (c)):
///   - **physics** (near-immutable): value anchors in realized downstream flow; the
///     noise floor holds. Encoded by the gate the runtime calls, not by a tunable here.
///   - **constitutional**: the amendment rules — a dimension is admitted only if it
///     predicts realized downstream value (verifier-gated), weights stay bounded.
///   - **governance**: weights within the bounded set, fluid.
/// v1 carries only the finalization frame; the dimension-matrix amendment rules are the
/// next build (`pending`: a constitutional cell whose transitions obey the verifier gate).
#[derive(Clone, Copy, Debug)]
pub struct Constitution {
    /// PoM-weighted finalization mix (NCI as-built: 10/30/60 PoW/PoS/PoM).
    pub mix: Mix,
    /// finalization supermajority, BPS.
    pub threshold_bps: u64,
    /// minimum live weight that must participate, as BPS of total base weight.
    pub quorum_floor_bps: u64,
    /// liveness horizon for franchise decay (0 = no decay).
    pub horizon: u64,
    /// NCI as-built decays PoW+PoM only; `true` decays PoS too (the symmetric fix).
    pub decay_pos: bool,
}

impl Default for Constitution {
    fn default() -> Self {
        Constitution {
            mix: NCI,
            threshold_bps: TWO_THIRDS_BPS,
            quorum_floor_bps: 0,
            horizon: 0,
            decay_pos: false,
        }
    }
}

// ============ Ledger — the replicated state ============

/// The replicated state: finalized cells in canonical commit order, the novelty index
/// over their coverage, the PoM standing each contributor has earned, and the height.
/// Two honest nodes applying the same finalized blocks hold equal [`Ledger::state_digest`].
pub struct Ledger {
    /// finalized cells, globally canonical (height ascending, then in-block shuffle slot).
    pub cells: Vec<Cell>,
    /// sparse-Merkle novelty index over inserted coverage shingles.
    pub index: NoveltyIndex,
    /// attribution output: PoM per soulbound contributor (`type_script.args`).
    pub pom: HashMap<Vec<u8>, u64>,
    /// height of the last finalized block.
    pub height: u64,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            cells: Vec::new(),
            index: NoveltyIndex::new(),
            pom: HashMap::new(),
            height: 0,
        }
    }

    /// Compact, comparable digest of replica state for convergence checks: the finalized
    /// cell-id sequence, the novelty-index root, and the sorted PoM attribution map.
    pub fn state_digest(&self) -> (Vec<u64>, Hash, Vec<(Vec<u8>, u64)>) {
        let ids: Vec<u64> = self.cells.iter().map(|c| c.id).collect();
        let mut pom: Vec<(Vec<u8>, u64)> = self.pom.iter().map(|(k, v)| (k.clone(), *v)).collect();
        pom.sort();
        (ids, self.index.root(), pom)
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Block — a commit-reveal batch in canonical order ============

/// A proposed block: cells paired with their consensus-sourced ordering coordinates,
/// already in canonical commit order. Assembly is presentation-independent — no producer
/// can bias which cell lands in which slot (the temporal-order strategyproofness guarantee).
#[derive(Clone)]
pub struct Block {
    pub height: u64,
    pub cells: Vec<Cell>,
    pub coords: Vec<Committed>,
}

impl Block {
    /// Leader assembly: order the proposals canonically (consensus shuffle, not producer
    /// presentation) and pair each cell with its coordinate in that order.
    pub fn assemble(height: u64, proposals: &[(Cell, Committed)]) -> Block {
        let raw: Vec<Committed> = proposals.iter().map(|(_, c)| c.clone()).collect();
        let order = canonical_order(&raw);
        let cells = order.iter().map(|&i| proposals[i].0.clone()).collect();
        let coords = order.iter().map(|&i| raw[i].clone()).collect();
        Block { height, cells, coords }
    }
}

// ============ Node — a single replica ============

/// One node: its validator identity is implicit in `validators`; it holds a [`Ledger`]
/// replica, the agreed validator set, the [`Constitution`], and a local mempool of
/// pre-consensus proposals.
pub struct Node {
    pub id: u64,
    pub ledger: Ledger,
    pub validators: Vec<Validator>,
    pub constitution: Constitution,
    pub mempool: Vec<(Cell, Committed)>,
}

impl Node {
    pub fn new(id: u64, validators: Vec<Validator>, constitution: Constitution) -> Self {
        Node {
            id,
            ledger: Ledger::new(),
            validators,
            constitution,
            mempool: Vec::new(),
        }
    }

    /// Gossip a proposal into the local mempool (pre-consensus).
    pub fn submit(&mut self, cell: Cell, coord: Committed) {
        self.mempool.push((cell, coord));
    }

    /// Leader move: assemble the next block from the local mempool.
    pub fn propose(&self) -> Block {
        Block::assemble(self.ledger.height + 1, &self.mempool)
    }

    /// Honest-verifier at the block level: a node votes YES iff the block passes every
    /// check it can verify against its OWN replica. (1) it extends our chain by one,
    /// (2) cells and coords align, (3) the coords are in canonical commit order (a
    /// producer-favorable reorder is rejected before any state math — no probe signal),
    /// (4) every coordinate's height matches the block (single-block batch in v1).
    pub fn validate(&self, b: &Block) -> bool {
        b.height == self.ledger.height + 1
            && b.cells.len() == b.coords.len()
            && !b.cells.is_empty()
            && is_canonical_order(&b.coords)
            && b.coords.iter().all(|c| c.height == b.height)
    }

    /// A node's vote on a proposal is its honest local validation.
    pub fn vote(&self, b: &Block) -> bool {
        self.validate(b)
    }

    /// Apply a finalized block. DETERMINISTIC state transition: append the canonical-ordered
    /// cells, insert their coverage into the novelty index (idempotent — mirrors the on-chain
    /// index rule), advance the height, and recompute PoM attribution over the full chain.
    /// Identical inputs ⇒ identical (cells, index.root(), pom) ⇒ replicas converge.
    pub fn apply(&mut self, b: &Block) {
        for cell in &b.cells {
            for key in coverage(&cell.data) {
                self.ledger.index.insert(key);
            }
            self.ledger.cells.push(cell.clone());
        }
        self.ledger.height = b.height;
        self.ledger.pom = pom_scores(&self.ledger.cells);
    }

    /// Drop mempool entries (called after a block finalizes; v1 clears the whole pool since
    /// the round's proposals are all included).
    pub fn clear_mempool(&mut self) {
        self.mempool.clear();
    }
}

// ============ Finalization decision ============

/// Does a block finalize under the constitution, given the validators that voted for it?
/// Thin wrapper over `consensus::finalizes_hybrid` so the runtime reads the rule from the
/// constitution rather than threading eight parameters at every call site.
pub fn finalizes(c: &Constitution, voters_for: &[Validator], all: &[Validator], now: u64) -> bool {
    consensus::finalizes_hybrid(
        voters_for,
        all,
        c.mix,
        now,
        c.horizon,
        c.decay_pos,
        c.threshold_bps,
        c.quorum_floor_bps,
    )
}

// ============ PoS+PoM finality gadget (T3 — PoW out of finality) ============
//
// Research T3 (RESEARCH-NETWORK-CONSENSUS.md) found a latent bug: the face-value 3-way sum counts
// PoW weight as final the instant it is mined, but PoW finality is probabilistic/reorgeable — so
// PoW lag becomes a finality-SAFETY vector. The production pattern (Casper-FFG / GRANDPA / Babylon /
// Decred) keeps the probabilistic layer OUT of the immediate finality weight. This module is the
// runtime-level fix; the 235-test core `consensus::finalizes_hybrid` rule is left intact and reused.
pub mod finality {
    use crate::consensus::{self, Mix, Validator};

    /// Finality mix — PoW REMOVED (it secures production / ordering / sybil-cost, never finality).
    /// PoS:PoM = 30:60 renormalized so the set sums to 1, so the 2/3 bar is 2/3 OF THE FAST-FINAL
    /// SET (PoS+PoM), not of a mixed-confidence global total.
    pub const FINALITY_MIX: Mix = Mix { pow: 0.0, pos: 1.0 / 3.0, pom: 2.0 / 3.0 };

    /// Anti-concentration floor (T3 + T11): each fast-final DIMENSION must independently supply at
    /// least this fraction (BPS) of its OWN dimension total for a checkpoint to finalize. This forces
    /// BOTH the objective capital axis (PoS) AND the subjective value axis (PoM) to participate, so
    /// PoM's 60% cannot unilaterally finalize — capital-orthogonality (T11) enforced in code. A
    /// CONSTITUTIONAL constant (physics/constitutional layer of the value-matrix governance, not
    /// governance-tunable).
    pub const MIN_DIM_BPS: u64 = 5000;

    fn dim_ok(weight_for: f64, weight_all: f64) -> bool {
        // a dimension absent from the whole set can't gate (avoids div-by-zero); otherwise the
        // voting weight in that dimension must clear MIN_DIM_BPS of the dimension's total.
        weight_all <= 0.0 || weight_for >= weight_all * MIN_DIM_BPS as f64 / 10_000.0
    }

    /// Finalize a checkpoint on PoS+PoM only, with the anti-concentration rule. PoW is excluded by
    /// the mix; `now`/`horizon`/`decay_pos`/`threshold_bps` carry the usual liveness/threshold
    /// semantics. Returns true iff (1) PoS+PoM voting weight ≥ 2/3 of the fast-final set AND (2)
    /// each of PoS and PoM independently clears the anti-concentration floor.
    pub fn finalizes_pos_pom(
        voters_for: &[Validator],
        all: &[Validator],
        now: u64,
        horizon: u64,
        decay_pos: bool,
        threshold_bps: u64,
    ) -> bool {
        if !consensus::finalizes_hybrid(
            voters_for,
            all,
            FINALITY_MIX,
            now,
            horizon,
            decay_pos,
            threshold_bps,
            0,
        ) {
            return false;
        }
        let pos_for: f64 = voters_for.iter().map(|v| v.pos).sum();
        let pos_all: f64 = all.iter().map(|v| v.pos).sum();
        let pom_for: f64 = voters_for.iter().map(|v| v.pom).sum();
        let pom_all: f64 = all.iter().map(|v| v.pom).sum();
        dim_ok(pos_for, pos_all) && dim_ok(pom_for, pom_all)
    }
}

#[cfg(test)]
mod tests {
    use super::finality::{finalizes_pos_pom, MIN_DIM_BPS, FINALITY_MIX};
    use crate::consensus::{Validator, TWO_THIRDS_BPS};

    fn v(id: u64, pow: f64, pos: f64, pom: f64) -> Validator {
        Validator { id, pow, pos, pom, last_heartbeat: 0, staked_balance: 0.0 }
    }

    #[test]
    fn pos_pom_both_dims_finalize() {
        // two validators each carrying capital (pos) AND contribution (pom); both vote.
        let all = vec![v(0, 0.0, 100.0, 100.0), v(1, 0.0, 100.0, 100.0)];
        assert!(finalizes_pos_pom(&all, &all, 1, 0, false, TWO_THIRDS_BPS));
    }

    #[test]
    fn pom_alone_cannot_finalize_anti_concentration() {
        // a PoM "whale" (no capital) + a capital holder (no PoM). The whale alone clears 2/3 of the
        // PoS+PoM SET, but contributes ZERO to the capital axis ⇒ anti-concentration must reject it.
        let whale = v(0, 0.0, 0.0, 200.0);
        let capital = v(1, 0.0, 100.0, 0.0);
        let all = vec![whale.clone(), capital.clone()];
        let pom_only = vec![whale];
        assert!(
            !finalizes_pos_pom(&pom_only, &all, 1, 0, false, TWO_THIRDS_BPS),
            "PoM unilaterally finalized — capital-orthogonality not enforced"
        );
        // but both axes participating DOES finalize.
        assert!(finalizes_pos_pom(&all, &all, 1, 0, false, TWO_THIRDS_BPS));
    }

    #[test]
    fn pow_is_excluded_from_finality() {
        // a PoW giant with no stake/contribution must not help finalize — PoW is out of the gadget.
        let pow_giant = v(0, 1_000_000.0, 0.0, 0.0);
        let pos = v(1, 0.0, 100.0, 0.0);
        let pom = v(2, 0.0, 0.0, 100.0);
        let all = vec![pow_giant.clone(), pos.clone(), pom.clone()];
        // PoW giant alone: FINALITY_MIX zeroes pow ⇒ contributes nothing ⇒ cannot finalize.
        assert!(!finalizes_pos_pom(&[pow_giant], &all, 1, 0, false, TWO_THIRDS_BPS));
        // the two fast-final dimensions together finalize, PoW irrelevant.
        assert!(finalizes_pos_pom(&[pos, pom], &all, 1, 0, false, TWO_THIRDS_BPS));
        // the mix really does exclude PoW.
        assert_eq!(FINALITY_MIX.pow, 0.0);
        assert_eq!(MIN_DIM_BPS, 5000);
    }
}
