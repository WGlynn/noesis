//! The Noesis index type-script (onchain/index-typescript, no_std, riscv64imac) validated END TO
//! END under the host harness — the WRITE half of the seen-set. The novelty-index root-transition
//! rule (`valid_ordered_root_transition`, single-sourced from `noesis_core::index_rule`) recomputed
//! INSIDE the VM: the index cell REFUSES any batch whose canonically-ordered chain of single-key SMT
//! insertions does not carry old_root to the announced new_root. The intake script READS this root
//! to floor redundancy; this script proves the root was honestly maintained.
//!
//! Fixture: tests/fixtures/index-typescript — rebuild with
//!   cd onchain/index-typescript && cargo build --release --target riscv64imac-unknown-none-elf
//!   cp target/riscv64imac-unknown-none-elf/release/index-typescript ../../node/tests/fixtures/
//!
//! Exit-code contract (src/main.rs): 0 valid · 50 invalid transition/order · 51 malformed · 52 unbound.

mod common;

use common::run_typescript;
use noesis::commit_order::{canonical_order, is_canonical_order, Committed};
use noesis::index_rule::{encode_index_batch, valid_ordered_root_transition, CellBatch, InsertStep};
use noesis::smt::NoveltyIndex;
use noesis::{Cell, Script};

const ELF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/index-typescript");

type Hash = [u8; 32];

fn coord(height: u64, secret_byte: u8) -> Committed {
    Committed { height, secret: [secret_byte; 32] }
}

/// Honest producer: present the cells in canonical commit order and build the batch with each step's
/// proof captured against the ROLLING root (the only way the transition is valid). Returns the
/// announced endpoints + the canonically-ordered cell batches.
fn honest_batch(cells: &[(Committed, Vec<u64>)]) -> (Hash, Hash, Vec<CellBatch>) {
    let coords: Vec<Committed> = cells.iter().map(|(c, _)| c.clone()).collect();
    let order = canonical_order(&coords);
    let mut idx = NoveltyIndex::new();
    let old_root = idx.root();
    let mut batch = Vec::new();
    for &i in &order {
        let (c, keys) = &cells[i];
        let mut steps = Vec::new();
        for &k in keys {
            steps.push(InsertStep { key: k, siblings: idx.proof(k) });
            idx.insert(k);
        }
        batch.push(CellBatch { coord: c.clone(), steps });
    }
    let new_root = idx.root();
    (old_root, new_root, batch)
}

fn batch_cell(old_root: Hash, new_root: Hash, cells: &[CellBatch]) -> Cell {
    Cell {
        id: 1,
        lock: Script { code_hash: [1u8; 32], args: vec![] },
        type_script: Script { code_hash: [0xC1; 32], args: vec![] },
        parent: None,
        timestamp: 0,
        data: encode_index_batch(old_root, new_root, cells),
    }
}

fn run(old_root: Hash, new_root: Hash, cells: &[CellBatch]) -> i8 {
    let cell = batch_cell(old_root, new_root, cells);
    let (res, served) = run_typescript(ELF, &cell, vec![cell.clone()]);
    assert!(served >= 1, "script must consume load_cell_data (got {served})");
    res.unwrap()
}

#[test]
fn honest_transition_accepted() {
    let cells = vec![(coord(10, 3), vec![7, 8]), (coord(10, 7), vec![9]), (coord(12, 1), vec![100])];
    let (old_root, new_root, batch) = honest_batch(&cells);
    assert!(valid_ordered_root_transition(old_root, new_root, &batch), "reference: honest is valid");
    assert_eq!(run(old_root, new_root, &batch), 0, "honest ordered transition accepted on-VM");
}

#[test]
fn omitted_key_moves_root_off_target() {
    // Drop the last step but keep the announced new_root: the computed end no longer matches.
    let cells = vec![(coord(5, 2), vec![1, 2, 3])];
    let (old_root, new_root, mut batch) = honest_batch(&cells);
    batch[0].steps.pop();
    assert!(!valid_ordered_root_transition(old_root, new_root, &batch), "reference: omission invalid");
    assert_eq!(run(old_root, new_root, &batch), 50, "omitted key rejected on-VM");
}

#[test]
fn smuggled_key_moves_root_off_target() {
    // Append an extra insertion (freshest possible path) the announced new_root does not contain.
    let cells = vec![(coord(5, 2), vec![1, 2, 3])];
    let (old_root, new_root, mut batch) = honest_batch(&cells);
    // rebuild the rolling index to get a valid non-membership path for the smuggled key
    let mut idx = NoveltyIndex::new();
    for k in [1u64, 2, 3] {
        idx.insert(k);
    }
    batch[0].steps.push(InsertStep { key: 999, siblings: idx.proof(999) });
    assert!(!valid_ordered_root_transition(old_root, new_root, &batch), "reference: smuggle invalid");
    assert_eq!(run(old_root, new_root, &batch), 50, "smuggled key rejected on-VM");
}

#[test]
fn duplicate_insertion_rejected() {
    // A second insertion of the same key cannot prove non-membership under the rolling root that now
    // contains it — structurally impossible, no dedup bookkeeping needed.
    let mut idx = NoveltyIndex::new();
    let old_root = idx.root();
    let mut steps = vec![InsertStep { key: 42, siblings: idx.proof(42) }];
    idx.insert(42);
    steps.push(InsertStep { key: 42, siblings: idx.proof(42) }); // forged second insert
    idx.insert(42); // no-op on the real tree
    let new_root = idx.root();
    let batch = vec![CellBatch { coord: coord(5, 1), steps }];
    assert!(!valid_ordered_root_transition(old_root, new_root, &batch), "reference: dup invalid");
    assert_eq!(run(old_root, new_root, &batch), 50, "duplicate insertion rejected on-VM");
}

#[test]
fn forged_sibling_path_rejected() {
    let cells = vec![(coord(5, 2), vec![1, 2, 3])];
    let (old_root, new_root, mut batch) = honest_batch(&cells);
    batch[0].steps[1].siblings[3] = [0xCC; 32]; // corrupt one node of one path
    assert_eq!(run(old_root, new_root, &batch), 50, "forged sibling path rejected on-VM");
}

#[test]
fn non_canonical_order_rejected() {
    // Same honest cells + proofs, but presented in a producer-favorable (non-canonical) order. The
    // order gate refuses BEFORE any root math — no probe signal — so this is exit 50, not a partial.
    let cells = vec![(coord(5, 9), vec![1]), (coord(9, 2), vec![2])];
    let (old_root, new_root, batch) = honest_batch(&cells);
    let mut reordered = batch.clone();
    reordered.reverse();
    assert!(
        !is_canonical_order(&reordered.iter().map(|c| c.coord.clone()).collect::<Vec<_>>()),
        "reference: reversed cross-height presentation is non-canonical"
    );
    assert_eq!(run(old_root, new_root, &reordered), 50, "non-canonical order rejected on-VM");
}

#[test]
fn malformed_rejected() {
    // Short header (< 64 bytes for the two roots).
    let short = Cell {
        id: 1,
        lock: Script { code_hash: [1u8; 32], args: vec![] },
        type_script: Script { code_hash: [0xC1; 32], args: vec![] },
        parent: None,
        timestamp: 0,
        data: vec![0u8; 40],
    };
    let (res, _) = run_typescript(ELF, &short, vec![short.clone()]);
    assert_eq!(res.unwrap(), 51, "short data is malformed");
    // Zero-cell batch: exactly the two roots, no cells — transitions nothing.
    let empty = Cell { data: vec![0u8; 64], ..short };
    let (res2, _) = run_typescript(ELF, &empty, vec![empty.clone()]);
    assert_eq!(res2.unwrap(), 51, "zero-cell batch is malformed");
}

#[test]
fn forged_step_count_is_malformed_not_a_trap() {
    // A cell header claiming a huge n_steps must NOT drive a multi-TB pre-allocation that traps the
    // VM — the codec bounds n_steps by the remaining bytes and returns a clean malformed reject.
    let mut data = vec![0u8; 64]; // old_root ‖ new_root
    data.extend_from_slice(&5u64.to_le_bytes()); // height
    data.extend_from_slice(&[1u8; 32]); // secret
    data.extend_from_slice(&u32::MAX.to_le_bytes()); // n_steps = 0xFFFFFFFF, zero step bytes follow
    let cell = Cell {
        id: 1,
        lock: Script { code_hash: [1u8; 32], args: vec![] },
        type_script: Script { code_hash: [0xC1; 32], args: vec![] },
        parent: None,
        timestamp: 0,
        data,
    };
    let (res, _) = run_typescript(ELF, &cell, vec![cell.clone()]);
    assert_eq!(res.unwrap(), 51, "forged step count is a clean malformed reject, not a VM trap");
}

#[test]
fn on_vm_matches_reference_across_tamperings() {
    let cells = vec![(coord(3, 4), vec![10, 11]), (coord(3, 8), vec![12]), (coord(8, 2), vec![13])];
    let (old_root, new_root, batch) = honest_batch(&cells);

    // Honest, plus three tamperings; the on-VM verdict must equal the reference for each.
    let mut omit = batch.clone();
    omit[0].steps.pop();
    let mut smuggle = batch.clone();
    let mut idx = NoveltyIndex::new();
    for k in [10u64, 11, 12, 13] {
        idx.insert(k);
    }
    smuggle[2].steps.push(InsertStep { key: 777, siblings: idx.proof(777) });
    let mut reorder = batch.clone();
    reorder.reverse();

    for candidate in [batch.clone(), omit, smuggle, reorder] {
        let expect = valid_ordered_root_transition(old_root, new_root, &candidate);
        let code = run(old_root, new_root, &candidate);
        assert_eq!(code == 0, expect, "on-VM ({code}) disagrees with reference ({expect})");
        assert!(code == 0 || code == 50, "must verdict, not error (code={code})");
    }
}
