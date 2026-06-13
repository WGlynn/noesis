//! T7 #4 second half — the proven history floors running INSIDE the VM, end to end.
//! A mint is accepted only if every produced cell PROVES its novelty against the live
//! index root (cell-dep 0) via the canonical witness blob. The streaming on-VM verifier
//! and the node's batch verifier are the same single-source noesis-core functions, so
//! the verdicts below are cross-checked against host-side computation in each case.

mod common;

use common::{input_cell, proof_blob, root_dep, run_typescript_t7};
use noesis::{proven, smt::NoveltyIndex};

const ELF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pom-typescript");
const SIM: u64 = 52429;
const ENT: u64 = 62259;

fn seeded_index() -> NoveltyIndex {
    let mut idx = NoveltyIndex::new();
    for prior in [&b"alpha-bravo-charlie-delta"[..], b"echo-foxtrot-golf-hotel"] {
        for (k, _) in proven::unique_shingles(prior) {
            idx.insert(k);
        }
    }
    idx
}

fn mint_tx(idx: &NoveltyIndex, data: &[u8], blob: Vec<u8>) -> i8 {
    let consumed = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let minted = input_cell(10, 7, data);
    let (result, _) =
        run_typescript_t7(ELF, &consumed.clone(), vec![consumed], vec![minted], vec![root_dep(idx)], vec![blob]);
    result.unwrap()
}

#[test]
fn novel_mint_with_honest_proofs_is_accepted_on_vm() {
    let idx = seeded_index();
    let data: &[u8] = b"india-juliet-kilo-lima fresh contribution";
    // host-side ground truth says it has value
    let proofs: Vec<_> = proven::unique_shingles(data).iter().map(|(k, _)| idx.proof(*k)).collect();
    let host = proven::proven_floored_novelty_q16(data, idx.root(), &proofs, SIM, ENT).unwrap();
    assert!(host > 0);
    assert_eq!(mint_tx(&idx, data, proof_blob(&idx, data)), 0, "on-VM agrees: accepted");
}

#[test]
fn redundant_mint_is_denied_on_vm_exit_22() {
    // Exact duplicate of committed history: every shingle proves MEMBER, similarity
    // floor zeroes it — the chain refuses to mint value for replayed content.
    let idx = seeded_index();
    let dup: &[u8] = b"alpha-bravo-charlie-delta";
    let proofs: Vec<_> = proven::unique_shingles(dup).iter().map(|(k, _)| idx.proof(*k)).collect();
    assert_eq!(proven::proven_floored_novelty_q16(dup, idx.root(), &proofs, SIM, ENT), Some(0));
    assert_eq!(mint_tx(&idx, dup, proof_blob(&idx, dup)), 22, "mint denied on-VM");
}

#[test]
fn tampered_proof_rejects_on_vm_exit_21() {
    let idx = seeded_index();
    let data: &[u8] = b"november-oscar-papa-quebec";
    let mut blob = proof_blob(&idx, data);
    blob[40] ^= 0xFF; // corrupt one sibling hash byte
    assert_eq!(mint_tx(&idx, data, blob), 21, "classification fails, whole mint rejected");
}

#[test]
fn stale_root_rejects_on_vm_exit_21() {
    // Proofs generated against yesterday's index; the dep carries today's root.
    let mut idx = seeded_index();
    let data: &[u8] = b"romeo-sierra-tango-uniform";
    let blob = proof_blob(&idx, data); // proofs vs old root
    for (k, _) in proven::unique_shingles(b"new-block-landed-meanwhile") {
        idx.insert(k);
    }
    assert_eq!(mint_tx(&idx, data, blob), 21, "stale proofs cannot classify under the live root");
}

#[test]
fn short_witness_and_missing_root_reject_on_vm_exit_20() {
    let idx = seeded_index();
    let data: &[u8] = b"victor-whiskey-xray-yankee";
    let mut blob = proof_blob(&idx, data);
    blob.truncate(blob.len() - 64); // omission: too few path bytes
    assert_eq!(mint_tx(&idx, data, blob), 20, "length mismatch rejects before verifying");
    // No index dep at all:
    let consumed = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let minted = input_cell(10, 7, data);
    let (result, _) = run_typescript_t7(
        ELF,
        &consumed.clone(),
        vec![consumed],
        vec![minted],
        vec![],
        vec![proof_blob(&idx, data)],
    );
    assert_eq!(result.unwrap(), 20, "a mint without the index dep cannot validate");
}
