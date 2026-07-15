//! Noesis index type-script — the novelty-index cell's root-transition rule running ON-VM
//! (RISC-V / CKB-VM). The WRITE half of the seen-set: the intake type-script READS the committed
//! novelty root (cell-dep 0) to floor redundant contributions; THIS script proves that root was
//! honestly MAINTAINED — the committed seen-shingle root may advance old -> new ONLY as an exact,
//! canonically-ordered chain of single-key SMT insertions, each proven against the ROLLING root.
//! The transition core is single-sourced from `noesis_core::index_rule::valid_ordered_root_transition`
//! (the same rule the node drift-guards through its maintainer-side `NoveltyIndex` producer).
//!
//! Exit codes (50s namespace — distinct from intake 0/11-23, finalization 0/30-34, ordering 0/40-42):
//!   0  = valid ordered root transition (accepted)
//!   50 = INVALID transition — either non-canonical cell order OR the rolling-root chain does not
//!        reach new_root (a dup / omitted / smuggled key / forged path). The two failures collapse
//!        to ONE exit deliberately: `valid_ordered_root_transition` gates order BEFORE any root math
//!        and returns a single verdict, so a producer learns nothing about WHICH check refused him.
//!   51 = malformed batch data (short header, partial cell/step record, or a zero-cell batch)
//!   52 = roots/coords not consensus-bound (sentinel-gated inert pre-deploy; reserved)
//!
//! Honest scope (the deploy-coupled point, same posture as the ordering + finalization ports):
//! DEMONSTRATED here is the ordered root-transition rule on-VM. PINNED, NOT YET ENFORCED, behind
//! `PROVENANCE_BOUND`: (a) `old_root`/`new_root` must be sourced from the actual INPUT/OUTPUT index
//! cells (not the batch's self-declared endpoints), and (b) each cell's `height`/`secret` coord must
//! be consensus-sourced (height from the commit-block header, secret from the block's reveals), never
//! trusted as the producer claims. That sourcing needs the index-cell + commit-reveal plumbing live,
//! so it is gated INERT pre-deploy, exactly like the index-dep F1/F2/F3 activated path, the ordering
//! `COORDS_BOUND`, and the finalization registry binding — an honest pin, not a claim of enforcement.

#![no_std]
#![no_main]

use ckb_std::{ckb_constants::Source, default_alloc, high_level::load_cell_data};
use noesis_core::index_rule::{parse_index_batch, valid_ordered_root_transition};

ckb_std::entry!(program_entry);
default_alloc!();

/// Provenance binding (roots + coords). When ACTIVE, the ELF sources `old_root`/`new_root` from the
/// input/output index cells and re-derives each coord from consensus, REFUSING any batch-declared
/// endpoint or coord it cannot reconstruct. INACTIVE pre-deploy (needs the index-cell + commit-reveal
/// plumbing): the batch carries the endpoints + coords and only the ORDERED TRANSITION over them is
/// enforced. Honest pin, like the ordering `COORDS_BOUND` and the finalization registry binding.
const PROVENANCE_BOUND: bool = false;

fn provenance_consensus_sourced() -> bool {
    if !PROVENANCE_BOUND {
        return true; // pre-deploy shape path; roots/coords provenance binding not yet activated
    }
    // post-deploy: old_root <- input index cell, new_root <- output index cell, coords <- header +
    // reveals; reject any batch-declared endpoint or coord. Lands with the index-cell deploy.
    false
}

pub fn program_entry() -> i8 {
    let data = match load_cell_data(0, Source::GroupInput) {
        Ok(d) => d,
        Err(_) => return 51,
    };
    let (old_root, new_root, cells) = match parse_index_batch(&data) {
        Some(t) => t,
        None => return 51,
    };
    if !provenance_consensus_sourced() {
        return 52;
    }
    // ONE verdict: order gate first (non-canonical => reject before root math, no probe signal), then
    // the rolling-root chain must reach new_root. Any failure — reorder, dup, omission, smuggle,
    // forged path — collapses to exit 50.
    if valid_ordered_root_transition(old_root, new_root, &cells) {
        0
    } else {
        50
    }
}
