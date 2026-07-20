//! Adaptive-adversary LADDER-REPLAY instrument (grain I-adaptive-1) — the correctly-specified
//! test for HCE-3 (adaptive-stability / Goodhart-robustness). Spec:
//! `docs/DESIGN-adaptive-adversary-instrument.md`.
//!
//! WHAT THIS IS. The two shipped sims are OPEN-LOOP: `moat_sim.rs` and `sybil_sim.rs` each play ONE
//! fixed attack against a FROZEN defense and score it once. That proves *static* robustness (the same
//! thing the 253/253 constructed fixtures prove). HCE-3 is a CLOSED-LOOP property: the adversary
//! adapts to each patch (`ISOMORPHISM-INVARIANCE-VS.md` §7 — "4 named axes, 4 new attacks"). This
//! harness is the first, smallest closed-loop grain: it replays the ACTUAL historical attacker ladder
//! against the ACTUAL current built defense in one run and prints where the defense frontier sits —
//! the first honest HCE-3 data point, regression-guarded (if a future change moves the frontier, the
//! numbers move).
//!
//! HONEST SCOPE (the boundary this grain does NOT cross, stated up front):
//!   1. This is LADDER-REPLAY, not GENERATIVE SEARCH. It drives the KNOWN rungs; it does not yet
//!      search for an unnamed axis. The generative attacker-oracle (best-response over a composable
//!      strategy space) is the NEXT grain (spec §2.1/§5). A frontier located here is "defended against
//!      the known ladder," not "robust against all adaptive attackers."
//!   2. Every rung here is a RELABELING / structure-faking attack (depth-split, forged edge,
//!      paraphrase). It therefore CANNOT see the wash-building attack — an AI sybil that GENUINELY
//!      builds (real content, real edges) cheaply. Relabel-invariance is orthogonal to cheap-genuine-
//!      building. That attack needs a cost-asymmetry mechanism the relabel frame does not supply, and
//!      is explicitly out of this instrument's reach. (Flagged for the theory, not measured here.)
//!
//! Not a toy: `noesis::value::value_v8`, `noesis::outcome::{coalition_features, train}`, and
//! `pom_scores_with_similarity_floor_q16` are the shipped functions; only the ladder is synthetic.
//! Run: `cargo run --release -p noesis --example adaptive_sim`.

use noesis::outcome::{coalition_features, train, N_FEATS};
use noesis::value::value_v8;
use noesis::{temporal_novelty, Cell, Script};
use std::collections::HashMap;

// Shipped value_v8 parameters (node/src/lib.rs test module: THETA 1587, DAMP 1588, ITERS 1589,
// HALF 1590, ENTROPY_THETA 1668, FLOOR 1886, THETA_Q16 2003).
const THETA: f64 = 0.8;
const ENTROPY_THETA: f64 = 0.95;
const THETA_Q16: u64 = 62259; // floor(0.95 · 2^16)
const DAMP: f64 = 0.85;
const ITERS: usize = 200;
const HALF: f64 = 8.0;
const FLOOR: u64 = 10;

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
fn noise(seed: u8, n: u8) -> Vec<u8> {
    (0..n).map(|i| seed.wrapping_add(i.wrapping_mul(41))).collect()
}

/// The trained outcome weights, reconstructed EXACTLY as the lib test `trained_outcome_w()` does
/// (lib.rs:2013) — examples cannot reach a `#[cfg(test)]` helper, so we rebuild it from the public
/// `outcome::{coalition_features, train}` seam. A value_set (connected honest lineage) preferred over
/// a garbage_set (orphaned noise), 8 identical prefs, 4000 iters, lr 0.3.
fn trained_outcome_w() -> [f64; N_FEATS] {
    let noise24 = |s: u8| -> Vec<u8> { (0u8..24).map(|i| s.wrapping_add(i.wrapping_mul(53))).collect() };
    let value_set = vec![
        cellc(100, 1, 0, None, b"alpha-bravo-charlie"),
        cellc(101, 1, 1, Some(100), b"delta-echo-foxtrot"),
        cellc(102, 1, 2, Some(101), b"golf-hotel-india"),
    ];
    let garbage_set = vec![
        cellc(110, 2, 0, None, &noise24(0x10)),
        cellc(111, 2, 1, None, &noise24(0x80)),
        cellc(112, 2, 2, None, &noise24(0xC0)),
    ];
    let feats = [
        coalition_features(&value_set, &[0, 1, 2]),
        coalition_features(&garbage_set, &[0, 1, 2]),
    ];
    train(&feats, &vec![(0usize, 1usize); 8], 4000, 0.3)
}

/// The relabel-invariant functional the ladder tracks: the set's TOTAL earned v8 (the
/// franchise-driving quantity), summed because a relabeling permutes WHICH identity earns.
fn total_v8(order: &[Cell], st: &HashMap<Vec<u8>, u64>, w: &[f64; N_FEATS]) -> f64 {
    value_v8(order, st, FLOOR, w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF)
        .iter()
        .sum()
}

/// A rung result: the gaming energy g = v(attack) − v(honest_counterfactual), a verdict, and the
/// built layer credited with defending it (or "LIVE" if g>0).
struct Rung {
    name: &'static str,
    g: f64,
    defends: &'static str,
}
fn verdict(g: f64) -> &'static str {
    // g ≤ 0 (within fp noise): the maneuver minted no value ⇒ DEFENDED. g > 0: LIVE (mints value).
    if g <= 1e-9 { "DEFENDED" } else { "LIVE" }
}

fn main() {
    let w = trained_outcome_w();
    println!("== Adaptive-adversary LADDER-REPLAY (HCE-3 frontier locator, grain I-adaptive-1) ==");
    println!("   functional: total earned value_v8 (real fn); g = v(attack) − v(honest counterfactual)");
    println!("   g ≤ 0 ⇒ maneuver mints nothing (DEFENDED);  g > 0 ⇒ LIVE (mints value)\n");

    let mut ladder: Vec<Rung> = Vec::new();

    // ---- Rung 0 — DEPTH-SPLIT (I-1): peel a self-built child onto a fresh identity. ----
    // Historical: was g≈+16.7 on value_v8; CLOSED by canonical identity-quotient flow (I-2,
    // lib.rs:3111). Honest counterfactual = the same lineage under one identity.
    {
        let base = vec![
            cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
            cellc(1, 1, 1, Some(0), b"echo-foxtrot-golf-hotel-built-on-root"),
            cellc(2, 2, 2, Some(1), b"india-juliet-kilo-lima-extends-lineage"),
        ];
        let st = standing_of(&[(1, FLOOR), (2, FLOOR)]);
        let v_base = total_v8(&base, &st, &w);
        // Peel cell 1 (authored by id 1) onto a fresh sybil identity 3, vested at FLOOR.
        let mut atk = base.clone();
        atk[1].type_script.args = vec![3];
        atk[1].lock.args = vec![3];
        let mut st2 = st.clone();
        st2.insert(vec![3], FLOOR);
        let g = total_v8(&atk, &st2, &w) - v_base;
        ladder.push(Rung { name: "0 depth-split (I-1 self-flow launder)", g, defends: "canonical identity-quotient flow (I-2)" });
    }

    // ---- Rung 1 — FORGED PARENT, NO NEW COVERAGE (A1-synergy): forge an edge to a rich block, ----
    // copy a subset of its content. Honest counterfactual = the SAME forger as an orphan (no edge).
    // CLOSED by submodular union over the connected component (lib.rs:7894). g measured through v8.
    {
        let rich = b"alpha-bravo-charlie-delta-echo-foxtrot-golf-hotel";
        let honest_root = cellc(0, 1, 0, None, rich);
        let st = standing_of(&[(1, FLOOR), (9, FLOOR)]);
        // Counterfactual: forger orphan, subset content.
        let orphan = vec![honest_root.clone(), cellc(1, 9, 1, None, &rich[..12].to_vec())];
        let v_base = total_v8(&orphan, &st, &w);
        // Attack: same forger, FORGED parent edge to the rich root.
        let mut forged = orphan.clone();
        forged[1].parent = Some(0);
        let g = total_v8(&forged, &st, &w) - v_base;
        ladder.push(Rung { name: "1 forged edge, no new coverage (A1-synergy)", g, defends: "submodular union / Myerson restriction" });
    }

    // ---- Rung 2 — FORGED PARENT, NOVEL CONTENT (A1 via coalition_features): the possibly-live rung. ----
    // The forger's content is GENUINELY NOVEL (not a subset), so submodularity does not zero it; the
    // forged edge raises connectedness/depth in coalition_features, which the trained outcome model
    // rewards. Honest counterfactual = the SAME novel content as an ORPHAN. g > 0 ⇒ topology-trust
    // leaks into v8 through the outcome gate (NOT separately pinned in the suite — this is the new
    // measurement).
    {
        let honest_root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta-echo-foxtrot-golf-hotel");
        let st = standing_of(&[(1, FLOOR), (9, FLOOR)]);
        let novel = noise(0x55, 40); // genuinely novel coverage, meaningless
        let orphan = vec![honest_root.clone(), cellc(1, 9, 1, None, &novel)];
        let v_base = total_v8(&orphan, &st, &w);
        let mut forged = orphan.clone();
        forged[1].parent = Some(0); // forged derivation → connectedness/depth rise
        let g = total_v8(&forged, &st, &w) - v_base;
        ladder.push(Rung { name: "2 forged edge, NOVEL content (A1 via coalition_features)", g, defends: "outcome gate + submodular (measuring residual)" });
    }

    // ---- Rung 3 — NEAR-DUPLICATE / PARAPHRASE (A3): the KNOWN OPEN gap (lib.rs:7913). ----
    // Flip a few bytes of an existing block → a few new shingles → small NONZERO novelty. Measured on
    // temporal_novelty (the coverage proxy where the residual lives) — the deployed franchise surface.
    {
        let honest_root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta-echo-foxtrot-golf-hotel");
        let mut near = honest_root.data.clone();
        let mid = near.len() / 2;
        for k in 0..3 { if mid + k < near.len() { near[mid + k] ^= 0x20; } }
        let order = vec![honest_root.clone(), cellc(1, 9, 1, None, &near)];
        let nov = temporal_novelty(&order);
        let g = *nov.last().unwrap() as f64; // residual novelty the paraphrase banks (> 0 = known gap)
        ladder.push(Rung { name: "3 near-duplicate / paraphrase (A3)", g, defends: "coverage proxy (KNOWN OPEN — needs semantic floor / Rosetta)" });
    }

    // ---- Print the ladder + the frontier ----
    println!("  rung                                                     |      g | verdict  | defended by");
    println!("  ---------------------------------------------------------+--------+----------+-----------------------------");
    let mut frontier: Option<usize> = None;
    for (i, r) in ladder.iter().enumerate() {
        let v = verdict(r.g);
        if frontier.is_none() && v == "LIVE" { frontier = Some(i); }
        println!("  {:<56} | {:>6.3} | {:<8} | {}", r.name, r.g, v, r.defends);
    }
    println!();

    match frontier {
        None => println!("FRONTIER: every known rung DEFENDED. The relabel-ladder is fully closed at the current\n          value_v8 defense. (Does NOT imply HCE-3 — the generative-search attacker + the\n          wash-building class are untested; see header scope + spec §4.)"),
        Some(k) => {
            println!("FRONTIER: rungs 0..{k} DEFENDED, rung {k} LIVE.");
            println!("          => the current defense frontier sits at rung {k}. That is the first honest HCE-3");
            println!("             data point: the co-evolution has closed everything below it and rung {k} is where");
            println!("             the next completion patch (or an honest 'this needs the learned/off-chain layer')");
            println!("             is owed. Regression-guarded: if a change moves the frontier, this number moves.");
        }
    }
    println!("\nBOUNDARY (do not round up): this is LADDER-REPLAY over RELABELING attacks. It cannot see the");
    println!("wash-building attack (AI sybils that genuinely build cheaply) — that is orthogonal to relabel-");
    println!("invariance and needs a build-cost-asymmetry mechanism the theory does not yet name. Next grain:");
    println!("the generative best-response attacker-oracle (spec §2.1).");
}
