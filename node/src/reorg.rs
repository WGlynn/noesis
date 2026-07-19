//! reorg (inc-1) — the reorgeable PoW tip beneath the PoS+PoM finality gadget.
//!
//! DESIGN-multi-producer-nakamoto.md. Single-producer instant finality let the node skip reorg
//! (`chainspec.rs` "single … proposer"); multi-producer Nakamoto needs the probabilistic layer the
//! finality mix was always built to sit on (PoW is finality-EXCLUDED, `runtime::finality::FINALITY_MIX`,
//! precisely because it is reorgeable). This module is that layer, and ONLY that layer: a reorgeable
//! tip over an immutable finalized base, with heaviest-cumulative-work fork choice. It adds NO new
//! consensus rule — it replays blocks through the ONE rulebook (`validate_block` + `apply_transition`),
//! exactly as a joiner (`sync`) and the durable store (`store`) already do.
//!
//! THE load-bearing property (§3.4): a finality snapshot is a FULL [`Ledger`] clone, so switching the
//! reorgeable suffix restores EVERY finalized-state field — PoM standing, the novelty index, the token
//! set, the work clock — structurally. Rolling standing + novelty back with the chain is not
//! hand-maintained per-field undo (where a missed field is a silent consensus split); it falls out of
//! cloning the whole state. An orphaned contribution loses its standing and frees its novelty slot
//! because the state it lived in is simply discarded.
//!
//! Scope of inc-1: the tip mechanism + fork choice + the finality floor. It does NOT decide WHEN to
//! finalize (that is the PoS+PoM gadget, wired in inc-3) nor move blocks between nodes (gossip, inc-4).

use crate::runtime::{apply_transition, validate_block, Block, Constitution, Ledger};

/// A canonical chain tip built on an immutable finalized base.
///
/// * `base` — the finalized snapshot; its `height` (`base_height`) is the **finality floor**: reorg
///   may never rewrite a block at or below it.
/// * `suffix` — the reorgeable blocks applied above the floor, in order.
/// * `tip` — `base` with `suffix` applied; the state the node currently serves.
///
/// Fork choice is heaviest CUMULATIVE WORK (`tip.work`, the mined-difficulty clock). Everything is a
/// replay through the shared rulebook, so a tip the node adopts is one its own `validate_block` accepts
/// (it trusts the rules, never the producer — the `sync`/`store` discipline).
pub struct ReorgTip {
    base: Ledger,
    base_height: u64,
    suffix: Vec<Block>,
    tip: Ledger,
    c: Constitution,
}

impl ReorgTip {
    /// Start a tip from a finalized snapshot. The snapshot's height is the finality floor; the tip
    /// begins equal to it with an empty reorgeable suffix.
    pub fn from_finalized(base: Ledger, c: Constitution) -> Self {
        let base_height = base.height;
        let tip = base.clone();
        Self { base, base_height, suffix: Vec::new(), tip, c }
    }

    /// The state the node currently serves (base + suffix).
    pub fn tip(&self) -> &Ledger {
        &self.tip
    }
    /// Cumulative mined work on the current tip — the fork-choice weight.
    pub fn work(&self) -> u64 {
        self.tip.work
    }
    /// Current tip height.
    pub fn height(&self) -> u64 {
        self.tip.height
    }
    /// The finality floor: reorg can never touch blocks at or below this height.
    pub fn finalized_height(&self) -> u64 {
        self.base_height
    }

    /// Extend the current tip by one block (ordinary forward progress). Validated against the tip via
    /// the shared rulebook. Returns `false` (a no-op) if the block fails validation, so a bad block
    /// never mutates state.
    pub fn extend(&mut self, b: Block) -> bool {
        if validate_block(&self.tip, &b, &self.c).is_err() {
            return false;
        }
        apply_transition(&mut self.tip, &b, &self.c);
        self.suffix.push(b);
        true
    }

    /// Attempt a REORG to a competing suffix of blocks above the finality floor. The candidate is
    /// replayed from the IMMUTABLE base (its first block must extend `base_height + 1`, so a candidate
    /// that tries to rewrite a finalized block fails validation and is rejected). The new tip is
    /// adopted iff it is fully valid AND has STRICTLY greater cumulative work than the current tip
    /// (heaviest-work fork choice; ties keep the incumbent, so fork choice is deterministic and does
    /// not thrash). Returns `true` iff the reorg was adopted.
    ///
    /// Because adoption REPLACES `tip` with a fresh replay of `base + candidate`, all state the old
    /// suffix produced (orphaned standing, novelty-index entries, token movements) is discarded in one
    /// step — §3.4, for free.
    pub fn try_reorg(&mut self, candidate: Vec<Block>) -> bool {
        let mut cand = self.base.clone();
        for b in &candidate {
            if validate_block(&cand, b, &self.c).is_err() {
                return false;
            }
            apply_transition(&mut cand, b, &self.c);
        }
        if cand.work <= self.tip.work {
            return false;
        }
        self.tip = cand;
        self.suffix = candidate;
        true
    }

    /// Advance the finality floor to `new_height` (must satisfy `base_height < new_height <= tip
    /// height`): fold the suffix prefix at or below it into the immutable base. After this, those
    /// blocks can never be reorged away. The tip is unchanged (it already includes them). A
    /// `new_height` outside the valid range is a no-op (the caller — the inc-3 gadget — only ever
    /// finalizes a height it has actually reached).
    pub fn finalize_to(&mut self, new_height: u64) {
        if new_height <= self.base_height || new_height > self.tip.height {
            return;
        }
        let n = (new_height - self.base_height) as usize; // suffix blocks now finalized
        let mut new_base = self.base.clone();
        for b in self.suffix.iter().take(n) {
            apply_transition(&mut new_base, b, &self.c);
        }
        self.base = new_base;
        self.base_height = new_height;
        self.suffix.drain(0..n);
    }
}
