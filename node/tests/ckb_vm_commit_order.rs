//! The Noesis commit-order type-script (onchain/commit-order-typescript, no_std, riscv64imac)
//! validated END TO END under the host harness — TEMPORAL-ORDER-ONCHAIN.md, the on-VM ordering
//! port. The consensus ORDERING rule (`is_canonical_order`, single-sourced from
//! `noesis_core::commit_order`) recomputed INSIDE the VM: the index cell REFUSES any batch that is
//! not in canonical commit order, before any root math, denying a producer-favorable reorder.
//!
//! Fixture: tests/fixtures/commit-order-typescript — rebuild with
//!   cd onchain/commit-order-typescript && cargo build --release --target riscv64imac-unknown-none-elf
//!   cp target/riscv64imac-unknown-none-elf/release/commit-order-typescript ../../node/tests/fixtures/
//!
//! Exit-code contract (src/main.rs): 0 canonical · 40 non-canonical · 41 malformed · 42 coords unbound.

mod common;

use common::run_typescript;
use noesis::commit_order::{canonical_order, encode_batch, is_canonical_order, Committed};
use noesis::{Cell, Script};

const ELF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/commit-order-typescript");

fn c(height: u64, secret_byte: u8) -> Committed {
    Committed { height, secret: [secret_byte; 32] }
}

/// Reorder `items` into canonical commit order (what an honest producer presents).
fn canon(items: &[Committed]) -> Vec<Committed> {
    canonical_order(items).iter().map(|&i| items[i].clone()).collect()
}

fn batch_cell(items: &[Committed]) -> Cell {
    Cell {
        id: 1,
        lock: Script { code_hash: [1u8; 32], args: vec![] },
        type_script: Script { code_hash: [0xC0; 32], args: vec![] },
        parent: None,
        timestamp: 0,
        data: encode_batch(items),
    }
}

fn run(items: &[Committed]) -> i8 {
    let cell = batch_cell(items);
    let (res, served) = run_typescript(ELF, &cell, vec![cell.clone()]);
    assert!(served >= 1, "script must consume load_cell_data (got {served})");
    res.unwrap()
}

#[test]
fn canonical_batch_passes() {
    let items = vec![c(10, 3), c(10, 7), c(12, 1), c(11, 5)];
    let presented = canon(&items);
    assert!(is_canonical_order(&presented), "reference: canonical presentation is canonical");
    assert_eq!(run(&presented), 0, "canonical batch accepted on-VM");
}

#[test]
fn non_canonical_order_is_rejected() {
    let items = vec![c(10, 3), c(10, 7), c(12, 1), c(11, 5)];
    let mut presented = canon(&items);
    presented.reverse(); // a producer-favorable reordering
    assert!(!is_canonical_order(&presented), "reference: reversed is non-canonical");
    assert_eq!(run(&presented), 40, "non-canonical batch rejected on-VM");
}

#[test]
fn cross_block_height_must_ascend() {
    // Canonical order is height-ascending first. Presenting a later block before an earlier one
    // (the cross-block reorder that would let a redundant later cell bank contested novelty) is
    // rejected regardless of the in-block shuffle.
    let early = c(5, 9);
    let late = c(9, 2);
    assert_eq!(run(&[late.clone(), early.clone()]), 40, "later-height-first rejected");
    // The ascending presentation of the same two is canonical (two singleton blocks).
    let asc = canon(&[late, early]);
    assert_eq!(run(&asc), 0, "height-ascending presentation accepted");
}

#[test]
fn single_cell_is_trivially_canonical() {
    assert_eq!(run(&[c(7, 1)]), 0, "a one-cell batch is in order");
}

#[test]
fn malformed_batch_is_rejected() {
    // 39 bytes — not a whole 40-byte record.
    let cell = Cell {
        id: 1,
        lock: Script { code_hash: [1u8; 32], args: vec![] },
        type_script: Script { code_hash: [0xC0; 32], args: vec![] },
        parent: None,
        timestamp: 0,
        data: vec![0u8; 39],
    };
    let (res, _) = run_typescript(ELF, &cell, vec![cell.clone()]);
    assert_eq!(res.unwrap(), 41, "partial record is malformed");
    // Empty batch — nothing to order.
    let empty = Cell { data: vec![], ..cell };
    let (res2, _) = run_typescript(ELF, &empty, vec![empty.clone()]);
    assert_eq!(res2.unwrap(), 41, "empty batch is malformed");
}

#[test]
fn on_vm_matches_reference_across_presentations() {
    let items = vec![c(3, 4), c(3, 8), c(3, 1), c(8, 2), c(5, 6)];
    // Several presentations; the on-VM exit must equal the reference verdict for each.
    let canonical = canon(&items);
    let mut swapped = canonical.clone();
    swapped.swap(0, canonical.len() - 1);
    for presented in [canonical, swapped, items.clone(), vec![items[0].clone()]] {
        let expect = is_canonical_order(&presented);
        let code = run(&presented);
        assert_eq!(code == 0, expect, "on-VM ({code}) disagrees with reference ({expect})");
        assert!(code == 0 || code == 40, "must verdict, not error (code={code})");
    }
}
