//! The Noesis finalization type-script (onchain/finalization-typescript, no_std, riscv64imac)
//! validated END TO END under the host harness — ON-VM-FINALIZATION.md build-order step 2: the
//! PoM-weighted finalize rule recomputed INSIDE the VM in the same Q32.32 form the node
//! drift-guards (`finalization_fixed`, single-sourced from `noesis_core::finalization`).
//!
//! Fixture: tests/fixtures/finalization-typescript — rebuild with
//!   cd onchain/finalization-typescript && cargo build --release --target riscv64imac-unknown-none-elf
//!   cp target/riscv64imac-unknown-none-elf/release/finalization-typescript ../../node/tests/fixtures/
//!
//! Exit-code contract (src/main.rs): 0 finalizes · 30 below threshold · 31 malformed cell/empty
//! group · 32 malformed votes · 33 header missing (now unsourced) · 34 registry unbound (inert).

mod common;

use common::{header_with_timestamp, run_typescript_finalization};
use noesis::finalization_fixed::{
    encode_finalization_cell, encode_votes, finalizes_fixed, FinalParams, MixQ, ValidatorQ,
};
use noesis::{Cell, Script};

const ELF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/finalization-typescript");
const TWO_THIRDS_BPS: u64 = 6667;

/// Q32.32 of a small fraction (2^32 = 4294967296). Test-side only; the on-VM inputs arrive fixed.
fn q(x: f64) -> u128 {
    (x * 4_294_967_296.0) as u128
}

/// A validator with equal pow/pos/pom = `w` and a liveness clock `hb`.
fn vq(id: u64, w: f64, hb: u64) -> ValidatorQ {
    ValidatorQ { id, pow: q(w), pos: q(w), pom: q(w), last_heartbeat: hb }
}

/// The NCI mix (0.10 / 0.30 / 0.60) in Q32.32.
fn nci() -> MixQ {
    MixQ { pow: q(0.10), pos: q(0.30), pom: q(0.60) }
}

/// Wrap encoded finalization-cell data in a Cell whose type-script is the script under test.
fn fin_cell(data: Vec<u8>) -> Cell {
    Cell {
        id: 1,
        lock: Script { code_hash: [1u8; 32], args: vec![] },
        type_script: Script { code_hash: [0xF0; 32], args: vec![] },
        parent: None,
        timestamp: 0,
        data,
    }
}

/// Build (cell, vote-witness) for a validator set + the indices that vote FOR.
fn scenario(validators: &[ValidatorQ], votes: &[u16], p: &FinalParams) -> (Cell, Vec<Vec<u8>>) {
    let data = encode_finalization_cell(nci(), p, validators);
    (fin_cell(data), vec![encode_votes(votes)])
}

fn params(horizon: u64, threshold: u64, floor: u64, decay_pos: bool) -> FinalParams {
    FinalParams { horizon, threshold_bps: threshold, quorum_floor_bps: floor, decay_pos }
}

#[test]
fn finalizes_when_supermajority_votes() {
    let vs = vec![vq(1, 0.9, 0), vq(2, 0.9, 0), vq(3, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 0, true);
    let (cell, wits) = scenario(&vs, &[0, 1, 2], &p);
    let (res, served) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], wits, header_with_timestamp(0));
    assert!(served >= 2, "script must consume load_script + load_cell_data (got {served})");
    assert_eq!(res.unwrap(), 0, "unanimous fresh vote finalizes on-VM");
    // Cross-check against the single-source reference at the same now.
    assert!(finalizes_fixed(&vs, &vs, nci(), 0, 100, true, TWO_THIRDS_BPS, 0));
}

#[test]
fn rejects_when_below_threshold() {
    let vs = vec![vq(1, 0.9, 0), vq(2, 0.9, 0), vq(3, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 0, true);
    let (cell, wits) = scenario(&vs, &[0], &p); // one of three ≈ 33% < 2/3
    let (res, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], wits, header_with_timestamp(0));
    assert_eq!(res.unwrap(), 30, "below-threshold vote is rejected on-VM");
    assert!(!finalizes_fixed(&vs[..1], &vs, nci(), 0, 100, true, TWO_THIRDS_BPS, 0));
}

/// THE adversarial fixture (ON-VM-FINALIZATION.md §3): `now` is read from the block HEADER and
/// nothing else. The SAME cell + SAME unanimous votes flips finalized→rejected purely by changing
/// the header timestamp — driven by the un-decayed quorum floor: at now=0 the live weight clears
/// the bar; once the validators go stale (now ≥ horizon) the decayed weight falls under the
/// constant floor·threshold. An attacker who WANTS finalization cannot inject a favorable `now`:
/// there is no witness/arg channel for it, so the late header governs and the claim is refused.
#[test]
fn now_is_header_sourced_not_tx_chosen() {
    let vs = vec![vq(1, 0.9, 0), vq(2, 0.9, 0), vq(3, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 5000, true); // 50% un-decayed quorum floor
    let (cell, wits) = scenario(&vs, &[0, 1, 2], &p);

    // Fresh header ⇒ live weight ⇒ finalizes (this is what the attacker would forge a now to get).
    let (early, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], wits.clone(), header_with_timestamp(0));
    assert_eq!(early.unwrap(), 0, "fresh header-now finalizes");
    assert!(finalizes_fixed(&vs, &vs, nci(), 0, 100, true, TWO_THIRDS_BPS, 5000));

    // Stale header ⇒ decayed weight under the floor ⇒ refused. ONLY the header changed.
    let (late, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], wits, header_with_timestamp(200));
    assert_eq!(late.unwrap(), 30, "stale header-now refuses — now comes from the header, not the tx");
    assert!(!finalizes_fixed(&vs, &vs, nci(), 200, 100, true, TWO_THIRDS_BPS, 5000));
}

#[test]
fn missing_header_is_rejected() {
    let vs = vec![vq(1, 0.9, 0), vq(2, 0.9, 0), vq(3, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 0, true);
    let (cell, wits) = scenario(&vs, &[0, 1, 2], &p);
    // Empty header = no header-dep ⇒ the consensus time source is unavailable.
    let (res, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], wits, Vec::new());
    assert_eq!(res.unwrap(), 33, "no header ⇒ now cannot be sourced ⇒ reject");
}

#[test]
fn quorum_floor_path_matches_reference_across_now() {
    // Stale-set sweep through the floor regime; the on-VM exit must equal the reference verdict
    // at every header timestamp (cross-VM determinism through the un-decayed floor).
    let vs = vec![vq(1, 0.8, 0), vq(2, 0.8, 0), vq(3, 0.8, 50), vq(4, 0.8, 50)];
    let p = params(100, TWO_THIRDS_BPS, 5000, true);
    let (cell, wits) = scenario(&vs, &[0, 1], &p); // two fresh voters_for against a 4-set
    for now in [0u64, 50, 100, 150, 200] {
        let expect = finalizes_fixed(&vs[..2], &vs, nci(), now, 100, true, TWO_THIRDS_BPS, 5000);
        let (res, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], wits.clone(), header_with_timestamp(now));
        let code = res.unwrap();
        assert_eq!(code == 0, expect, "on-VM ({code}) disagrees with reference ({expect}) at now={now}");
        assert!(code == 0 || code == 30, "floor path must verdict, not error (now={now}, code={code})");
    }
}

#[test]
fn malformed_cell_data_is_rejected() {
    // Short header.
    let cell = fin_cell(vec![0u8; 10]);
    let (res, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], vec![vec![]], header_with_timestamp(0));
    assert_eq!(res.unwrap(), 31, "sub-PARAMS_LEN data is malformed");
    // Params present but a validator section that is not a whole number of records.
    let vs = vec![vq(1, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 0, true);
    let mut data = encode_finalization_cell(nci(), &p, &vs);
    data.truncate(data.len() - 5); // chop the last record short
    let cell2 = fin_cell(data);
    let (res2, _) = run_typescript_finalization(ELF, &cell2, vec![cell2.clone()], vec![vec![]], header_with_timestamp(0));
    assert_eq!(res2.unwrap(), 31, "partial validator record is malformed");
}

#[test]
fn malformed_votes_are_rejected() {
    let vs = vec![vq(1, 0.9, 0), vq(2, 0.9, 0), vq(3, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 0, true);
    let data = encode_finalization_cell(nci(), &p, &vs);
    let cell = fin_cell(data);
    // Vote index 5 is outside a 3-validator set.
    let (res, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], vec![encode_votes(&[5])], header_with_timestamp(0));
    assert_eq!(res.unwrap(), 32, "out-of-range vote index is malformed");
    // Odd-length witness can't be a u16 index list.
    let (res2, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], vec![vec![0u8]], header_with_timestamp(0));
    assert_eq!(res2.unwrap(), 32, "odd-length vote witness is malformed");
}

/// RSAW 2026-06-13: a repeated vote index would double-count that validator's effective weight and
/// forge finalization from a single real voter. The vote set must reject duplicates.
#[test]
fn duplicate_vote_indices_cannot_inflate_weight() {
    let vs = vec![vq(1, 0.9, 0), vq(2, 0.9, 0), vq(3, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 0, true);
    let cell = fin_cell(encode_finalization_cell(nci(), &p, &vs));
    // One honest vote is below the 2/3 bar — it must NOT finalize.
    let (single, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], vec![encode_votes(&[0])], header_with_timestamp(0));
    assert_eq!(single.unwrap(), 30, "one real vote is below threshold");
    // Tripling that same vote [0,0,0] would sum 3x its weight and clear the bar if counted —
    // it is rejected as malformed instead, so no inflation is possible.
    let (dup, _) = run_typescript_finalization(ELF, &cell, vec![cell.clone()], vec![encode_votes(&[0, 0, 0])], header_with_timestamp(0));
    assert_eq!(dup.unwrap(), 32, "duplicate vote indices rejected — weight cannot be inflated");
}

#[test]
fn empty_group_is_rejected() {
    let (res, _) = run_typescript_finalization(ELF, &fin_cell(vec![0u8; 64]), Vec::new(), Vec::new(), header_with_timestamp(0));
    assert_eq!(res.unwrap(), 31, "empty finalization group attests nothing");
}

/// Two finalization cells in one group: a false second claim cannot ride on a true first — the
/// script iterates the whole GroupInput, so the below-threshold cell at index 1 is caught.
#[test]
fn second_cell_cannot_smuggle_a_false_claim() {
    let vs = vec![vq(1, 0.9, 0), vq(2, 0.9, 0), vq(3, 0.9, 0)];
    let p = params(100, TWO_THIRDS_BPS, 0, true);
    let good = fin_cell(encode_finalization_cell(nci(), &p, &vs));
    let bad = fin_cell(encode_finalization_cell(nci(), &p, &vs));
    let wits = vec![encode_votes(&[0, 1, 2]), encode_votes(&[0])]; // cell 0 finalizes, cell 1 does not
    let (res, _) = run_typescript_finalization(ELF, &good, vec![good.clone(), bad], wits, header_with_timestamp(0));
    assert_eq!(res.unwrap(), 30, "the false second finalization cell is rejected");
}
