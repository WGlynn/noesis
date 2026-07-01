//! zk-finalize host — prove + verify the canonical fixtures.
//!
//! Encodes each fixture with the core's OWN producer (`encode_finalization_cell`/`encode_votes`),
//! proves the guest execution, verifies the receipt against the guest image id, and decodes the
//! journal verdict. The printed verdicts must equal the `parity` harness (host stable, no zkVM):
//!   whale alone -> REJECTED, whale + 1 -> REJECTED, whale + 2 -> FINALIZES.

use risc0_zkvm::sha::Digest;
use risc0_zkvm::{default_prover, ExecutorEnv};
use zk_finalize_methods::{ZK_FINALIZE_ELF, ZK_FINALIZE_ID};

use noesis_core::finalization::{
    encode_finalization_cell, encode_votes, FinalParams, ValidatorQ, FINALITY_MIX_Q,
};

const ONE: u128 = 1 << 32;

fn v(id: u64, pos_units: u128, pom_units: u128, hb: u64) -> ValidatorQ {
    ValidatorQ { id, pow: 0, pos: pos_units * ONE, pom: pom_units * ONE, last_heartbeat: hb }
}

fn prove_case(cell: &[u8], votes: &[u8], now: u64) -> (Digest, bool) {
    let env = ExecutorEnv::builder()
        .write(&cell.to_vec())
        .unwrap()
        .write(&votes.to_vec())
        .unwrap()
        .write(&now)
        .unwrap()
        .build()
        .unwrap();

    let receipt = default_prover().prove(env, ZK_FINALIZE_ELF).unwrap().receipt;
    // The proof is worthless unless it checks against the guest we think we ran.
    receipt.verify(ZK_FINALIZE_ID).unwrap();
    receipt.journal.decode().unwrap()
}

fn main() {
    let now = 100u64;
    let params = FinalParams { horizon: 1_000, threshold_bps: 6667, quorum_floor_bps: 0, decay_pos: false };

    let all = [v(1, 100, 0, now), v(2, 1, 50, now), v(3, 1, 50, now)];
    let cell = encode_finalization_cell(FINALITY_MIX_Q, &params, &all);

    let cases: &[(&str, &[u16], bool)] = &[
        ("whale alone", &[0], false),
        ("whale + 1 contributor", &[0, 1], false),
        ("whale + 2 contributors", &[0, 1, 2], true),
    ];

    println!("zk-finalize host — proving finalizes_pos_pom_fixed in the RISC Zero zkVM\n");
    for (label, idxs, expected) in cases {
        let votes = encode_votes(idxs);
        let (digest, verdict) = prove_case(&cell, &votes, now);
        assert_eq!(
            verdict, *expected,
            "proven verdict for '{label}' disagrees with the parity ground truth"
        );
        println!(
            "  [{}] {:<24} proven+verified  journal_digest={}",
            if verdict { "FINALIZES" } else { "REJECTED " },
            label,
            hex(digest.as_bytes())
        );
    }
    println!("\nall receipts verified against the guest image id; verdicts match the parity harness.");
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
