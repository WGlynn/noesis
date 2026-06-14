//! Noesis commit-order type-script — the consensus ORDERING rule running ON-VM (RISC-V / CKB-VM).
//! TEMPORAL-ORDER-ONCHAIN.md, the on-VM ordering port. The index cell's first-commit-wins outcome
//! is decided by the order in which committed cells are applied; a producer who REORDERS same- or
//! cross-height contenders can steal contested novelty. This script ASSERTS the presented batch is
//! already in canonical commit order (height ascending, then XOR-seeded in-block Fisher-Yates over
//! revealed secrets — neither producer-arrangeable) and REJECTS any other order BEFORE any root
//! math, denying a probe signal. The ordering core is single-sourced from `noesis_core::commit_order`
//! (the same `is_canonical_order` the node drift-guards).
//!
//! Exit codes (40s namespace — distinct from intake 0/11-23 and finalization 0/30-34):
//!   0  = the batch is in canonical commit order (accepted)
//!   40 = NON-canonical order (a producer-favorable reorder; rejected)
//!   41 = malformed batch data (not a whole number of 40-byte records, or empty)
//!   42 = coords not consensus-bound (sentinel-gated inert pre-deploy; reserved)
//!
//! Honest scope (the deploy-coupled point): this asserts the ORDER over the coords as presented in
//! the batch cell. The coords themselves — `height` and `secret` — MUST be consensus-sourced on-VM
//! (height re-derived from each cell's commit-block header, secret from the block's reveals), never
//! trusted as the producer claims them (the 7th attacker-input site, TEMPORAL-ORDER-ONCHAIN.md).
//! That re-derivation needs the commit-reveal block plumbing live, so it is gated INERT behind
//! `COORDS_BOUND` pre-deploy, exactly like the index-dep binding and the finalization registry
//! binding. Demonstrated here: the ordering rule on-VM. Pinned, not yet enforced: coord provenance.

#![no_std]
#![no_main]

use ckb_std::{ckb_constants::Source, default_alloc, high_level::load_cell_data};
use noesis_core::commit_order::{is_canonical_order, parse_batch};

ckb_std::entry!(program_entry);
default_alloc!();

/// Coord provenance binding. When ACTIVE, the ELF re-derives each cell's `height` from its
/// commit-block header (HeaderDep) and `secret` from the block's reveals, and REJECTS any coord it
/// cannot reconstruct from consensus — a producer-asserted height/secret is self-assertion, not a
/// check. INACTIVE pre-deploy (needs the commit-reveal block plumbing): the batch coords are taken
/// as presented and only the ORDER over them is enforced. Honest pin, like the index-dep F1/F2/F3
/// activated path and the finalization registry binding.
const COORDS_BOUND: bool = false;

fn coords_consensus_sourced() -> bool {
    if !COORDS_BOUND {
        return true; // pre-deploy shape path; provenance binding not yet activated
    }
    // post-deploy: height <- header(commit block), secret <- reveals; reject any claimed coord.
    false
}

pub fn program_entry() -> i8 {
    let data = match load_cell_data(0, Source::GroupInput) {
        Ok(d) => d,
        Err(_) => return 41,
    };
    let batch = match parse_batch(&data) {
        Some(b) => b,
        None => return 41,
    };
    if !coords_consensus_sourced() {
        return 42;
    }
    // The whole batch must be presented in canonical order. A reorder is rejected at this gate,
    // before any root transition is computed — no silent re-sort, so the producer learns nothing.
    if is_canonical_order(&batch) {
        0
    } else {
        40
    }
}
