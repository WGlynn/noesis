//! Parity harness for the zk-finalize PoC.
//!
//! Runs the EXACT function the RISC Zero guest proves — `finalization::finalizes_pos_pom_fixed`
//! — on the canonical fixtures, through the same wire format the guest decodes
//! (`encode_finalization_cell`/`parse_finalization_cell`, `encode_votes`/`parse_votes`). No
//! zkVM here: this is the host-side ground truth. The verdicts it prints are the values the
//! proof's journal must equal. If the guest ever disagrees with this, the guest is wrong.

use noesis_core::finalization::{
    encode_finalization_cell, encode_votes, finalizes_pos_pom_fixed, parse_finalization_cell,
    parse_votes, FinalParams, ValidatorQ, FINALITY_MIX_Q,
};

const ONE: u128 = 1 << 32; // Q32.32 unit

fn v(id: u64, pos_units: u128, pom_units: u128, hb: u64) -> ValidatorQ {
    ValidatorQ { id, pow: 0, pos: pos_units * ONE, pom: pom_units * ONE, last_heartbeat: hb }
}

/// Decode-then-evaluate exactly as the guest will: bytes in, verdict out.
fn eval_via_wire(cell: &[u8], votes: &[u8], now: u64) -> bool {
    let (_mix, params, validators) =
        parse_finalization_cell(cell).expect("cell decodes");
    let idxs = parse_votes(votes, validators.len()).expect("votes decode");
    let voters_for: Vec<ValidatorQ> = idxs.iter().map(|&i| validators[i].clone()).collect();
    finalizes_pos_pom_fixed(
        &voters_for,
        &validators,
        now,
        params.horizon,
        params.decay_pos,
        params.threshold_bps,
    )
}

fn main() {
    // now == every heartbeat => retention is fresh (ONE): decay is not what these fixtures test.
    let now = 100u64;
    let params = FinalParams {
        horizon: 1_000,
        threshold_bps: 6667, // 2/3 supermajority of the PoS+PoM fast-final set
        quorum_floor_bps: 0, // finalizes_pos_pom_fixed pins this to 0 internally; carried for format
        decay_pos: false,
    };

    // Whale = pure capital (all PoS, no contribution). Contributors carry the PoM (contribution) axis.
    let whale = v(1, 100, 0, now);
    let a = v(2, 1, 50, now);
    let b = v(3, 1, 50, now);
    let all = [whale.clone(), a.clone(), b.clone()];

    let cell = encode_finalization_cell(FINALITY_MIX_Q, &params, &all);
    // Round-trip parity: the producer encode and the guest decode agree.
    let (_m, _p, decoded) = parse_finalization_cell(&cell).expect("cell round-trips");
    assert_eq!(decoded.len(), all.len(), "validator count survives the wire");

    // idx 0 = whale, 1 = a, 2 = b (order of `all`).
    let cases: &[(&str, &[u16], bool)] = &[
        ("whale alone", &[0], false),        // capital cannot finalize without contribution's consent
        ("whale + 1 contributor", &[0, 1], false), // still short of the floor
        ("whale + 2 contributors", &[0, 1, 2], true), // contribution axis present => FINALIZES
    ];

    println!("zk-finalize parity — finalizes_pos_pom_fixed over the canonical fixtures");
    println!("  params: horizon={} threshold_bps={} decay_pos={}", params.horizon, params.threshold_bps, params.decay_pos);
    println!("  mix (Q32.32): pow={} pos={} pom={}\n", FINALITY_MIX_Q.pow, FINALITY_MIX_Q.pos, FINALITY_MIX_Q.pom);

    let mut anchors_ok = true;
    for (label, idxs, expected) in cases {
        let votes = encode_votes(idxs);
        let verdict = eval_via_wire(&cell, &votes, now);
        let mark = if verdict { "FINALIZES" } else { "REJECTED " };
        let expect_mark = if *expected == verdict { "ok" } else { "MISMATCH" };
        println!("  [{}] {:<24} voters_for={:?}  (expected {})", mark, label, idxs, if *expected { "FINALIZES" } else { "REJECTED" });
        // Anchor assertions: the two load-bearing outcomes the demo and the design doc claim.
        if *label == "whale alone" || *label == "whale + 2 contributors" {
            if *expected != verdict {
                anchors_ok = false;
                println!("      ^^ ANCHOR {} — the proof would be provably wrong", expect_mark);
            }
        }
    }

    assert!(anchors_ok, "anchor fixtures must match: capital-alone REJECTED, contribution-present FINALIZES");
    println!("\nanchors verified. these verdicts are the journal values the RISC Zero proof must commit.");
}
