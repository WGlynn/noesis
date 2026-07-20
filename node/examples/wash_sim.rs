//! Wash-building DISCRIMINATOR probe (grain HCE-4-1) — can any graph-internal signal tell a genuine
//! collaboration from a competently-built wash ring? Spec: `docs/DESIGN-mind-scarcity-asymmetry.md`.
//!
//! WHY. `adaptive_sim.rs` shows every RELABELING attack is defended (g=0). But the open frontier
//! (Will, 2026-07-20) is DISHONEST/SYBIL contributions that are NOT relabelings: distinct sybil minds
//! that GENUINELY build (novel content, real cross-identity edges) cheaply. This probe asks the exact
//! question — "we need mathematical discernment of dishonest/sybil contributions" — by constructing a
//! genuine collaboration and a wash operation with MATCHED graph statistics, running every discriminant
//! we ship, and measuring whether any of them separates the two.
//!
//! HONEST FRAMING. There is no "meaningful vs meaningless" knob available structurally — that is the
//! whole point. GENUINE and WASH differ only in the WORTH of their content, which no worth-blind
//! structural signal can see. So this probe measures precisely where the current math CAN and CANNOT
//! discern, and names what closing the gap would require. A separation of ~0 on the tree case is the
//! honest RED — the first HCE-4 number.
//!
//! Real fns: `noesis::{temporal_novelty, attribution_cycle_energy, attribution_circulation}`,
//! `noesis::synergy::sampled_value`, `noesis::value::value_v8`. Run:
//! `cargo run --release -p noesis --example wash_sim`.

use noesis::outcome::{coalition_features, train, N_FEATS};
use noesis::synergy;
use noesis::value::value_v8;
use noesis::{attribution_circulation, attribution_cycle_energy, temporal_novelty, Cell, Script};
use std::collections::HashMap;

const THETA: f64 = 0.8;
const ENTROPY_THETA: f64 = 0.95;
const THETA_Q16: u64 = 62259;
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
/// Deterministic distinct novel bytes for identity `who`, block `n`. Worth-blind: genuine and wash
/// content are BOTH generated this way, because no structural generator can encode "meaning" — that is
/// exactly the discriminant gap being measured.
fn distinct(who: u8, n: u8) -> Vec<u8> {
    (0..40u8).map(|i| who.wrapping_mul(97).wrapping_add(n.wrapping_mul(31)).wrapping_add(i.wrapping_mul(17))).collect()
}

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
    let feats = [coalition_features(&value_set, &[0, 1, 2]), coalition_features(&garbage_set, &[0, 1, 2])];
    train(&feats, &vec![(0usize, 1usize); 8], 4000, 0.3)
}

struct Disc {
    v8_total: f64,
    novelty_total: u64,
    synergy_total: f64,
    cycle_energy: f64,
    circulation: u64,
}
fn discriminants(cells: &[Cell], st: &HashMap<Vec<u8>, u64>, w: &[f64; N_FEATS]) -> Disc {
    Disc {
        v8_total: value_v8(cells, st, FLOOR, w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF).iter().sum(),
        novelty_total: temporal_novelty(cells).iter().sum(),
        synergy_total: synergy::sampled_value(cells, 3000, true).iter().sum(),
        cycle_energy: attribution_cycle_energy(cells),
        circulation: attribution_circulation(cells),
    }
}

fn main() {
    let w = trained_outcome_w();

    // ---- GENUINE: 4 distinct minds, distinct novel content, a collaboration TREE (each builds on the
    // prior). This is what an honest collaboration looks like STRUCTURALLY. ----
    let genuine = vec![
        cellc(0, 1, 0, None, &distinct(1, 0)),
        cellc(1, 2, 1, Some(0), &distinct(2, 0)),
        cellc(2, 3, 2, Some(1), &distinct(3, 0)),
        cellc(3, 4, 3, Some(2), &distinct(4, 0)),
    ];
    let st_gen = standing_of(&[(1, FLOOR), (2, FLOOR), (3, FLOOR), (4, FLOOR)]);

    // ---- WASH-TREE: 4 fresh sybil minds (one actor's keys), distinct novel GARBAGE, SAME topology.
    // Structurally IDENTICAL to genuine — distinct ids, novel content, connected tree. ----
    let wash_tree = vec![
        cellc(0, 5, 0, None, &distinct(5, 0)),
        cellc(1, 6, 1, Some(0), &distinct(6, 0)),
        cellc(2, 7, 2, Some(1), &distinct(7, 0)),
        cellc(3, 8, 3, Some(2), &distinct(8, 0)),
    ];
    let st_wash = standing_of(&[(5, FLOOR), (6, FLOOR), (7, FLOOR), (8, FLOOR)]);

    // ---- WASH-RING: the SAME sybils but cross-citing in a CYCLE (adds a back-edge). This is the
    // *dumb* wash the cyclic defense is built for. ----
    let mut wash_ring = wash_tree.clone();
    wash_ring.push(cellc(4, 5, 4, Some(3), &distinct(5, 1))); // id5 builds on id8's cell → closes 5→6→7→8→5

    let g = discriminants(&genuine, &st_gen, &w);
    let wt = discriminants(&wash_tree, &st_wash, &w);
    let wr = discriminants(&wash_ring, &st_wash, &w);

    let sep = |a: f64, b: f64| -> String {
        let denom = a.abs().max(b.abs()).max(1e-9);
        let rel = (a - b).abs() / denom;
        if rel < 0.05 { format!("BLIND ({:.1}%)", 100.0 * rel) } else { format!("SEPARATES ({:.1}%)", 100.0 * rel) }
    };

    println!("== Wash-building DISCRIMINATOR probe (HCE-4): can any graph-internal signal tell");
    println!("   a genuine collaboration from a wash operation with matched graph statistics? ==\n");
    println!("  discriminant     | genuine | wash-tree | wash-ring | genuine vs wash-TREE (the hard case)");
    println!("  -----------------+---------+-----------+-----------+-------------------------------------");
    println!("  value_v8 total   | {:>7.3} | {:>9.3} | {:>9.3} | {}", g.v8_total, wt.v8_total, wr.v8_total, sep(g.v8_total, wt.v8_total));
    println!("  novelty total    | {:>7} | {:>9} | {:>9} | {}", g.novelty_total, wt.novelty_total, wr.novelty_total, sep(g.novelty_total as f64, wt.novelty_total as f64));
    println!("  synergy total    | {:>7.3} | {:>9.3} | {:>9.3} | {}", g.synergy_total, wt.synergy_total, wr.synergy_total, sep(g.synergy_total, wt.synergy_total));
    println!("  cycle_energy     | {:>7.3} | {:>9.3} | {:>9.3} | {}", g.cycle_energy, wt.cycle_energy, wr.cycle_energy, sep(g.cycle_energy, wt.cycle_energy));
    println!("  circulation      | {:>7} | {:>9} | {:>9} | {}", g.circulation, wt.circulation, wr.circulation, sep(g.circulation as f64, wt.circulation as f64));
    println!();

    // Verdict: does ANY discriminant separate genuine from wash-TREE (the matched, competent wash)?
    let tree_sep = [
        ("value_v8", g.v8_total, wt.v8_total),
        ("novelty", g.novelty_total as f64, wt.novelty_total as f64),
        ("synergy", g.synergy_total, wt.synergy_total),
        ("cycle_energy", g.cycle_energy, wt.cycle_energy),
        ("circulation", g.circulation as f64, wt.circulation as f64),
    ];
    let separators: Vec<&str> = tree_sep.iter()
        .filter(|(_, a, b)| (a - b).abs() / a.abs().max(b.abs()).max(1e-9) >= 0.05)
        .map(|(name, _, _)| *name)
        .collect();

    if separators.is_empty() {
        println!("VERDICT (HCE-4, honest RED): NO graph-internal discriminant separates a genuine");
        println!("  collaboration from a competently-built (acyclic) wash operation. They are structurally");
        println!("  identical — distinct minds, novel content, connected topology — and every shipped signal");
        println!("  is WORTH-BLIND, so it scores them the same. The cyclic defense (cycle_energy/circulation)");
        println!("  catches only the DUMB wash-RING (see wash-ring column); a wash-TREE that mimics honest");
        println!("  collaboration topology is invisible.");
    } else {
        println!("VERDICT: these discriminants separate genuine from wash-tree: {separators:?}");
        println!("  (Unexpected — inspect whether the wash-tree was constructed to truly match statistics.)");
    }
    println!();
    println!("  => Mathematical discernment of dishonest/sybil contributions is CLOSED for the relabel and");
    println!("     cyclic classes and OPEN for the acyclic-wash class. Closing it cannot come from a");
    println!("     worth-blind structural signal; it requires exactly ONE of:");
    println!("       (a) a SEMANTIC worth signal — learned v(S) — which is null on structural features;");
    println!("       (b) an EXTERNAL independence anchor — proof-of-personhood — a capturable authority;");
    println!("       (c) REALIZED future use by minds OUTSIDE the ring — which recurses (are THEY genuine?)");
    println!("           and is a time-lock (patient wash beats it).");
    println!("  This is the mind-scarcity base case (docs/DESIGN-mind-scarcity-asymmetry.md) made numeric:");
    println!("  the discriminant gap is real, and it is not reducible to current graph-internal math.");
}
