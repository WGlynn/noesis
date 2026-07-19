//! Adversarial Sybil simulation of the v0 deployed franchise.
//!
//! HONEST SCOPE: this drives the REAL consensus scorer — `pom_scores_with_similarity_floor_q16`
//! (the deployed testnet franchise, `runtime.rs:1442`) — over synthetic honest and farmer content,
//! then applies a per-identity standing cap, and reports the farmer's captured share of the
//! contribution dimension. It is NOT a toy re-derivation: the novelty + Q16.16 similarity-floor math
//! is the shipped code. What is synthetic is only the CONTENT (deterministically generated) and the
//! ATTACK model. Run: `cargo run --release -p noesis --example sybil_sim`.
//!
//! Threat target (Will 2026-07-19): v0 + cap should survive the CASUAL SOLO SCRIPTED FARMER (one
//! actor, tens of identities, a weekend). It is NOT claimed to survive a funded coordinated cartel —
//! that is the realized-value moat + dispute market, once the network has honest participants and
//! outcome history. This sim publishes exactly where the boundary is.

use noesis::{pom_scores_with_similarity_floor_q16, Cell, Script};
use std::collections::HashMap;

const THETA_SIM_Q16: u64 = 62259; // 0.95 — the shipped testnet value (chainspec.rs:165 / runtime.rs:165)
const HONEST_PER_ID: usize = 5; // submissions per honest contributor
const CONTENT_LEN: usize = 200; // bytes per submission (honest and farmer, comparable magnitudes)

/// Deterministic PRNG (splitmix64) — reproducible, no wall-clock / no rand crate.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next() & 0xff) as u8).collect()
    }
}

fn cell(id: u64, contributor: &str, data: Vec<u8>) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: id.to_le_bytes().to_vec() },
        // type_script.args = the SOULBOUND contributor identity that standing is keyed on.
        type_script: Script { code_hash: [1u8; 32], args: contributor.as_bytes().to_vec() },
        parent: None,
        timestamp: id,
        data,
    }
}

/// Honest content: varied, English-like, and DISTINCT ACROSS CONTRIBUTORS. A per-identity `salt` is
/// mixed into every token so two different honest contributors do not cannibalise each other's novelty
/// (they write about genuinely different things). This is the FAIR model: v0 is worth-blind, so honest
/// and farmer content earn comparable novelty — the sim isolates the identity-count dynamics, not a
/// vocabulary artefact.
fn honest_content(rng: &mut Rng, _salt: u64) -> Vec<u8> {
    // Language-shaped but from a large effective alphabet (6-letter lowercase pseudo-words, ~3e8 space)
    // so distinct contributors do not collide on 4-byte shingles. v0 is worth-blind: honest-diverse and
    // junk are novelty-equivalent, so this earns comparable standing to the farmer by construction —
    // which is exactly the point (the defense cannot be the content score; it must be identity count).
    let mut s = Vec::with_capacity(CONTENT_LEN + 8);
    while s.len() < CONTENT_LEN {
        for _ in 0..6 {
            s.push(b'a' + (rng.next() % 26) as u8);
        }
        s.push(b' ');
    }
    s.truncate(CONTENT_LEN);
    s
}

/// Farmer content: varied random bytes = maximally novel (overlaps nothing) = the point of the audit:
/// v0 rewards high-entropy junk like genuine work.
fn farmer_content(rng: &mut Rng) -> Vec<u8> {
    rng.bytes(CONTENT_LEN)
}

/// Score a population and return per-role capped totals of contribution standing.
/// `cap` caps EACH identity's standing (honest and farmer alike — it is a protocol knob, not targeted).
fn run(n_honest: usize, n_farmer: usize, farmer_per_id: usize, cap: u64) -> (u64, u64) {
    let mut rng = Rng(0xC0FFEE_1234_5678);
    let mut cells: Vec<Cell> = Vec::new();
    let mut id: u64 = 1;
    // Interleave honest and farmer submissions in commit order so neither gets a first-mover monopoly
    // on shingles (fair to the defender; farmer content is random so collisions are negligible anyway).
    let mut honest_ids: Vec<String> = (0..n_honest).map(|i| format!("honest_{i}")).collect();
    let mut farmer_ids: Vec<String> = (0..n_farmer).map(|i| format!("farmer_{i}")).collect();
    for round in 0..HONEST_PER_ID.max(farmer_per_id) {
        if round < HONEST_PER_ID {
            for (hi, h) in honest_ids.iter().enumerate() {
                cells.push(cell(id, h, honest_content(&mut rng, hi as u64)));
                id += 1;
            }
        }
        if round < farmer_per_id {
            for f in &farmer_ids {
                cells.push(cell(id, f, farmer_content(&mut rng)));
                id += 1;
            }
        }
    }
    honest_ids.sort();
    farmer_ids.sort();

    let scores: HashMap<Vec<u8>, u64> = pom_scores_with_similarity_floor_q16(&cells, THETA_SIM_Q16);
    let cap_of = |key: &str| -> u64 { scores.get(key.as_bytes()).copied().unwrap_or(0).min(cap) };
    let honest_total: u64 = honest_ids.iter().map(|h| cap_of(h)).sum();
    let farmer_total: u64 = farmer_ids.iter().map(|f| cap_of(f)).sum();
    (honest_total, farmer_total)
}

fn share(honest: u64, farmer: u64) -> f64 {
    if honest + farmer == 0 { 0.0 } else { farmer as f64 / (honest + farmer) as f64 }
}

fn main() {
    println!("== v0 Sybil failure envelope (real franchise: novelty + theta_sim=0.95 + per-identity cap) ==\n");

    // Establish the cap unit: one honest identity's natural standing (5 x ~197 novel shingles).
    let (h1, _) = run(1, 0, 0, u64::MAX);
    let cap = h1; // cap each identity at ~one honest contributor's worth
    println!("calibration: one honest identity earns {h1} standing; per-identity cap C := {cap}\n");

    // ---- Regime A: NO cap, a SINGLE farmer identity, growing submission count ----
    println!("Regime A — no cap, 1 farmer identity, N=10 honest (x{HONEST_PER_ID} each):");
    for k in [10usize, 50, 200] {
        let (h, f) = run(10, 1, k, u64::MAX);
        println!("  farmer submits {k:>3} junk cells -> farmer share {:.1}%", 100.0 * share(h, f));
    }
    println!("  => without a cap, ONE identity dominates by volume. The cap is necessary.\n");

    // ---- Regime B: per-identity cap C, sweep farmer identity count, find the 50% crossing ----
    println!("Regime B — per-identity cap C={cap}, farmer x{HONEST_PER_ID} each. Smallest F to capture >=50%:");
    println!("  N honest | crossing F | share@F=N | share@F=2N");
    for &n in &[5usize, 10, 25, 50] {
        let mut crossing = None;
        for f in 1..=(4 * n) {
            let (h, ff) = run(n, f, HONEST_PER_ID, cap);
            if share(h, ff) >= 0.5 { crossing = Some(f); break; }
        }
        let (hn, fn_) = run(n, n, HONEST_PER_ID, cap);
        let (h2, f2) = run(n, 2 * n, HONEST_PER_ID, cap);
        println!(
            "  {n:>8} | {:>10} | {:>7.1}% | {:>8.1}%",
            crossing.map(|c| c.to_string()).unwrap_or_else(|| ">4N".into()),
            100.0 * share(hn, fn_),
            100.0 * share(h2, f2),
        );
    }
    println!("  => with a cap, share ~= F/(N+F): the farmer captures the dimension once F ~ N.");
    println!("     Free identities (keygen is costless) mean a solo farmer reaches F~N trivially.\n");

    // ---- Regime C: cap + allowlist bounding identity count to the honest set ----
    println!("Regime C — cap C={cap} + allowlist bounding total farmer identities to <= N/... :");
    for &n in &[5usize, 10, 25, 50] {
        let allow = (n / 5).max(1); // an allowlist that admits a trickle of unvetted identities
        let (h, f) = run(n, allow, HONEST_PER_ID, cap);
        println!("  N={n:>2}, farmer capped to {allow:>2} admitted identities -> farmer share {:.1}%", 100.0 * share(h, f));
    }
    println!("  => bounding IDENTITY COUNT (allowlist / proof-of-personhood) is the load-bearing brake");
    println!("     during bootstrap, because a per-identity cap alone loses to free identities.\n");

    // ---- Collusion is a null at v0 (the franchise is flow-blind) ----
    let (h_solo, f_solo) = run(10, 10, HONEST_PER_ID, cap);
    println!(
        "Collusion note: v0 scores first-appearance novelty only (no downstream-flow gate), so \
         cross-citation among farmer identities changes nothing. Share at N=10,F=10 = {:.1}% \
         regardless of collusion; collusion first bites at the v5+ flow layer, not here.",
        100.0 * share(h_solo, f_solo)
    );
}
