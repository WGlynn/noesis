//! Moat demonstration: v0 → v5 → v6 on a Sybil ring, driving the REAL value functions.
//!
//! Empirically validates the 2026-07-19 moat correction (`data/crates/RESULTS.md`,
//! `docs/research/something-from-nothing-oracle-free-content-value.md`): the moat is the STRUCTURAL
//! layered defense, not a learned predictor. Here we show the *identity-pricing* layer specifically —
//! v6 denies the fresh-key Sybil ring that v0 (novelty-only, the deployed testnet franchise) pays and
//! that v5 (flow-gated, no identity price) still leaks. It also shows the honest cost: an *isolated*
//! newcomer earns ~0 under the structural defense too (cold-start), which is exactly why the deployed
//! floor + admission control bridge bootstrap (`docs/DESIGN-bootstrap-admission.md`).
//!
//! Not a toy: `noesis::value::{value_v5, value_v6}` and `pom_scores_with_similarity_floor_q16` are the
//! shipped functions; only the scenario is synthetic. Mirrors the shipped unit tests
//! (`value_v6_closes_the_sybil_identity_ring`, `unvested_newcomer_still_earns_...`).
//! Run: `cargo run --release -p noesis --example moat_sim`.

use noesis::value::{value_v5, value_v6};
use noesis::{pom_scores_with_similarity_floor_q16, Cell, Script};
use std::collections::HashMap;

// Shipped test parameters (node/src/lib.rs test module): the flow gate + vesting floor.
const THETA: f64 = 0.8; // float similarity floor used by value_v5/v6
const THETA_Q16: u64 = 62259; // 0.95 — the deployed v0 franchise threshold
const DAMP: f64 = 0.85;
const ITERS: usize = 200;
const HALF: f64 = 8.0;
const FLOOR: u64 = 10; // vesting standing floor: a seed counts only from an identity at/above it

fn cellc(id: u64, contrib: u8, ts: u64, parent: Option<u64>, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [1u8; 32], args: vec![contrib] },
        type_script: Script { code_hash: [0xB0; 32], args: vec![contrib] },
        parent,
        timestamp: ts,
        data: data.to_vec(),
    }
}
fn standing_of(pairs: &[(u8, u64)]) -> HashMap<Vec<u8>, u64> {
    pairs.iter().map(|&(k, s)| (vec![k], s)).collect()
}
/// Deterministic varied "garbage" (fresh novel coverage, no meaning) — the farmer's content.
fn noise(seed: u8, n: u8) -> Vec<u8> {
    (0..n).map(|i| seed.wrapping_add(i.wrapping_mul(41))).collect()
}

fn v0_total(cells: &[Cell]) -> f64 {
    pom_scores_with_similarity_floor_q16(cells, THETA_Q16).values().sum::<u64>() as f64
}
fn sum(vals: &[f64]) -> f64 {
    vals.iter().sum()
}

fn main() {
    println!("== Moat demonstration: identity pricing (v6) closes the Sybil ring v0/v5 leave open ==\n");
    println!("(real value fns: pom_scores_with_similarity_floor_q16 [v0], value::value_v5, value::value_v6)\n");

    // ---- Scenario 1: the fresh-key Sybil ring (the moat's headline) ----
    // Two fresh identities (100, 101), novel garbage, cross-cited (child builds on parent) to
    // manufacture downstream flow. No vested standing. This is the exact shape of
    // value_v6_closes_the_sybil_identity_ring.
    let ring = vec![
        cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"), // an honest bystander, vested below
        cellc(10, 100, 1, None, &noise(0xA0, 48)),          // ring parent, fresh key, garbage
        cellc(11, 101, 2, Some(10), &noise(0x10, 48)),      // ring child, fresh key, builds on parent
    ];
    let bystander = standing_of(&[(1, 50)]); // only the honest bystander has earned standing
    let v5_ring = value_v5(&ring, THETA, DAMP, ITERS, HALF);
    let v6_ring = value_v6(&ring, &bystander, FLOOR, THETA, DAMP, ITERS, HALF);
    let ring_idx = [1usize, 2]; // the two ring cells
    let ring_v0: u64 = {
        let s = pom_scores_with_similarity_floor_q16(&ring, THETA_Q16);
        s.get(&vec![100u8]).copied().unwrap_or(0) + s.get(&vec![101u8]).copied().unwrap_or(0)
    };
    let ring_v5: f64 = ring_idx.iter().map(|&i| v5_ring[i]).sum();
    let ring_v6: f64 = ring_idx.iter().map(|&i| v6_ring[i]).sum();
    println!("Scenario 1 — fresh-key Sybil ring (2 identities, cross-cited garbage):");
    println!("  v0 (novelty, DEPLOYED franchise) pays the ring : {ring_v0}   (> 0 ⇒ farmable)");
    println!("  v5 (flow-gated, no identity price) pays the ring: {ring_v5:.4}   (> 0 ⇒ still leaks — the pinned gap)");
    println!("  v6 (flow + identity pricing) pays the ring      : {ring_v6:.4}   (= 0 ⇒ CLOSED: fresh keys seed nothing)");
    println!("  => the identity-pricing layer of the STRUCTURAL defense is what denies the farm.\n");

    // ---- Scenario 2: honest newcomer is PAID from day one (participation is not gated) ----
    // A newcomer (contrib 7, standing 0) ships honest work; a vested mind (contrib 1) builds on it.
    let paid = vec![
        cellc(0, 7, 0, None, b"newcomer-honest-work-kilo-lima"),
        cellc(1, 1, 1, Some(0), b"vested-mind-builds-on-newcomer"),
    ];
    let st2 = standing_of(&[(1, 50)]); // contrib 1 vested; newcomer 7 has nothing
    let v6_paid = value_v6(&paid, &st2, FLOOR, THETA, DAMP, ITERS, HALF);
    println!("Scenario 2 — honest newcomer, vested mind builds on them:");
    println!("  v6 pays the newcomer (idx 0): {:.4}   (> 0 ⇒ earning needs no standing; only CERTIFYING does)", v6_paid[0]);
    println!("  => v6 prices certification, never participation — a newcomer is not shut out.\n");

    // ---- Scenario 3: the cold-start cost (why the deployed floor + admission bridge bootstrap) ----
    // An ISOLATED honest newcomer: genuine work, but nobody has built on it yet.
    let isolated = vec![
        cellc(0, 7, 0, None, b"isolated-newcomer-genuine-but-unused-mike-november"),
    ];
    let st3 = standing_of(&[]); // no one vested yet — the true cold start
    let v5_iso = value_v5(&isolated, THETA, DAMP, ITERS, HALF);
    let v6_iso = value_v6(&isolated, &st3, FLOOR, THETA, DAMP, ITERS, HALF);
    let iso_v0 = v0_total(&isolated);
    println!("Scenario 3 — isolated honest newcomer (real work, no downstream use YET):");
    println!("  v0 pays it (novelty)      : {iso_v0}     (the deployed floor DOES pay early honest work)");
    println!("  v5 pays it (needs flow)   : {:.4}   (~0 ⇒ no realized use yet)", sum(&v5_iso));
    println!("  v6 pays it (needs flow)   : {:.4}   (~0 ⇒ same cold-start penalty)", sum(&v6_iso));
    println!("  => the structural defense under-pays HONEST newcomers at bootstrap exactly as it under-pays");
    println!("     farmers. That symmetry is why the chain ships the v0 FLOOR (pays early honest work) +");
    println!("     admission control (bounds farmer identity count) until realized-value history exists.\n");

    println!("Bottom line: v0 is farmable (novelty-only); the STRUCTURAL defense (v6 identity pricing) is");
    println!("the moat that denies the fresh-key ring — a BUILT mechanism, not a learned predictor (which is");
    println!("null on real data). The open frontier is its robustness vs a real ADAPTIVE adversary (HCE-3).");
}
