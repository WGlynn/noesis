//! Discernment / wash-building findings, PINNED into the suite (HCE-4, 2026-07-20).
//!
//! Companion to `node/examples/{wash_sim,adaptive_sim,periphery_sim}.rs` and
//! `docs/DESIGN-{mind-scarcity-asymmetry,periphery-solution}.md`. Examples don't run under
//! `make test`; these regression-guard the measured findings so a future change that MOVES the
//! frontier surfaces loudly:
//!
//!   (1) OPEN GAP — a competently-built (acyclic) wash-TREE is graph-internally indistinguishable
//!       from a genuine collaboration: every worth-blind structural signal scores them identically.
//!       Pinned so that if a future discriminant separates them, this test FLIPS and we learn the
//!       gap closed (the (aa)-ring RED-as-designed pattern, applied to the discernment frontier).
//!   (2) POSITIVE CONTROL — the cyclic defense DOES catch a wash-RING. The defense that works,
//!       guarded so it can't silently regress to zero.
//!
//! Drives the real public functions (`pom_scores_with_similarity_floor_q16`, `temporal_novelty`,
//! `attribution_cycle_energy`); only the scenarios are synthetic. Not a consensus path.

use noesis::{attribution_cycle_energy, pom_scores_with_similarity_floor_q16, temporal_novelty, Cell, Script};

const THETA_Q16: u64 = 62259; // 0.95 — deployed franchise near-dup floor

fn cell(id: u64, contributor: u8, parent: Option<u64>, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [1u8; 32], args: vec![contributor] },
        type_script: Script { code_hash: [0xB0; 32], args: vec![contributor] },
        parent,
        timestamp: id,
        data: data.to_vec(),
    }
}
/// Distinct novel bytes per (identity, block) — matched across genuine and wash so the ONLY
/// difference is worth, which no structural signal can see. (Same generator as `wash_sim.rs`.)
fn distinct(who: u8, n: u8) -> Vec<u8> {
    (0..40u8)
        .map(|i| who.wrapping_mul(97).wrapping_add(n.wrapping_mul(31)).wrapping_add(i.wrapping_mul(17)))
        .collect()
}
/// A collaboration TREE: each identity builds on the prior. `ids` are the soulbound contributors.
fn tree(ids: &[u8]) -> Vec<Cell> {
    ids.iter()
        .enumerate()
        .map(|(i, &who)| cell(i as u64, who, if i == 0 { None } else { Some(i as u64 - 1) }, &distinct(who, 0)))
        .collect()
}

#[test]
fn wash_tree_is_graph_internally_indistinguishable_from_genuine_pinned_gap() {
    // Genuine collaboration (distinct minds, novel content, connected tree) vs a wash TREE (fresh
    // sybil keys, distinct novel garbage, SAME topology). Structurally identical ⇒ worth-blind
    // signals score them the same. THE OPEN GAP (docs/DESIGN-mind-scarcity-asymmetry.md).
    let genuine = tree(&[1, 2, 3, 4]);
    let wash = tree(&[5, 6, 7, 8]);

    // Novelty: identical (both fully-novel distinct content, equal magnitudes).
    let nov_g: u64 = temporal_novelty(&genuine).iter().sum();
    let nov_w: u64 = temporal_novelty(&wash).iter().sum();
    assert!(nov_g > 0, "control: honest work earns novelty (equality below is meaningful)");
    assert_eq!(
        nov_g, nov_w,
        "PINNED GAP (HCE-4): novelty cannot tell a genuine collaboration from a wash tree. \
         If this ever differs, a worth-blind signal started separating them — investigate the close."
    );

    // Deployed franchise standing: identical too (per-cell novelty summed by identity, worth-blind).
    let pom_total = |c: &[Cell]| -> u64 { pom_scores_with_similarity_floor_q16(c, THETA_Q16).values().sum() };
    assert_eq!(
        pom_total(&genuine),
        pom_total(&wash),
        "PINNED GAP (HCE-4): the deployed pom_scores franchise scores genuine and wash identically."
    );

    // The cyclic defense has nothing to bite on an acyclic tree — for EITHER population.
    assert_eq!(attribution_cycle_energy(&genuine), 0.0, "acyclic genuine tree: no cyclic signature");
    assert_eq!(
        attribution_cycle_energy(&wash),
        0.0,
        "PINNED GAP (HCE-4): an acyclic wash tree has no cyclic signature to catch — the discriminant \
         is blind precisely where the competent adversary lives."
    );
}

#[test]
fn layer_a_vesting_gate_separates_wash_from_genuine_where_structure_is_blind() {
    // Layer A (docs/DESIGN-periphery-solution.md): given an EXTERNAL capital-independence signal, the
    // vesting gate is the FIRST code that separates wash from genuine — closing (with an external
    // anchor, not graph structure) the gap the test above pins open. It does NOT read structure to
    // discern; it is GIVEN capital clusters (the periphery). Independence = different capital cluster.
    use noesis::vesting::independent_use_gate;
    use std::collections::HashMap;

    let genuine = tree(&[1, 2, 3, 4]); // 4 minds
    let wash = tree(&[5, 6, 7, 8]); // 4 sybils
    let unit = vec![1u64; 4]; // flat unit value ⇒ isolate the GATE's effect, not a value fn

    // Genuine: 4 DISTINCT capital clusters (real independent minds).
    let genuine_caps: HashMap<Vec<u8>, u64> =
        [(vec![1], 1u64), (vec![2], 2), (vec![3], 3), (vec![4], 4)].into_iter().collect();
    // Wash: ONE capital cluster (one actor's keys — a closed ring).
    let wash_caps: HashMap<Vec<u8>, u64> =
        [(vec![5], 0u64), (vec![6], 0), (vec![7], 0), (vec![8], 0)].into_iter().collect();

    let vested_genuine: u64 = independent_use_gate(&genuine, &unit, &genuine_caps).iter().sum();
    let vested_wash: u64 = independent_use_gate(&wash, &unit, &wash_caps).iter().sum();

    // Genuine: cells 0,1,2 each have a capital-independent child ⇒ vest; cell 3 is a leaf ⇒ 0 ⇒ total 3.
    assert_eq!(vested_genuine, 3, "genuine work vests: it has capital-independent downstream use");
    // Wash: all one cluster ⇒ NO independent use anywhere ⇒ vests nothing.
    assert_eq!(vested_wash, 0, "closed wash ring (one capital cluster) vests NOTHING — Layer A's anchor");
    assert!(
        vested_genuine > vested_wash,
        "Layer A SEPARATES genuine from wash where every graph-internal signal was blind — because it \
         is given the external capital-independence signal (the periphery), not because it reads structure"
    );
}

#[test]
fn cyclic_wash_ring_is_caught_by_cycle_energy_positive_control() {
    // The DUMB wash that closes an attribution cycle IS detectable — the structural defense that
    // works, guarded so it can't silently regress to zero. (This is why the frontier is the
    // acyclic wash TREE, not the ring.)
    let mut ring = tree(&[5, 6, 7, 8]);
    // Add a back-edge: a 5th cell by id 5 building on id 8's cell ⇒ closes 5→6→7→8→5.
    ring.push(cell(4, 5, Some(3), &distinct(5, 1)));
    assert!(
        attribution_cycle_energy(&ring) > 0.0,
        "positive control: a cyclic wash ring fires attribution_cycle_energy (the defense that works)"
    );
}

#[test]
fn periphery_ev_model_pinned_closed_wash_negative_genuine_positive() {
    // Pins the periphery-solution EV result (docs/DESIGN-periphery-solution.md, node/examples/periphery_sim.rs)
    // as a regression guard. With MEASURED harvest (pom_scores) and MEASURED vest fractions (the Layer A gate
    // over identical-topology graphs), at honest design params a CLOSED WASH RING is negative-EV while GENUINE
    // work is positive-EV. If a value/gate change ever flips either sign, this fails loudly. The signs hold for
    // ANY harvest > 0 (genuine coeff 0.75-0.30-0.02 > 0; wash coeff 0-0.30-0.50 < 0), so no magic number is pinned.
    use noesis::vesting::independent_use_gate;
    use std::collections::HashMap;

    // MEASURED harvest S: per-identity standing one novel-junk block earns on the deployed franchise.
    let harvest = *pom_scores_with_similarity_floor_q16(&[cell(0, 1, None, &distinct(1, 0))], THETA_Q16)
        .get(&vec![1u8])
        .unwrap_or(&0) as f64;
    assert!(harvest > 0.0, "control: a novel block earns positive harvest (else the EV signs are vacuous)");

    // MEASURED vest fractions via the gate over identical-topology genuine vs closed-wash graphs.
    let unit4 = vec![1u64; 4];
    let genuine = tree(&[1, 2, 3, 4]);
    let wash = tree(&[5, 6, 7, 8]);
    let genuine_caps: HashMap<Vec<u8>, u64> =
        [(vec![1], 1u64), (vec![2], 2), (vec![3], 3), (vec![4], 4)].into_iter().collect();
    let wash_caps: HashMap<Vec<u8>, u64> =
        [(vec![5], 0u64), (vec![6], 0), (vec![7], 0), (vec![8], 0)].into_iter().collect();
    let vest_genuine =
        independent_use_gate(&genuine, &unit4, &genuine_caps).iter().sum::<u64>() as f64 / 4.0;
    let vest_wash = independent_use_gate(&wash, &unit4, &wash_caps).iter().sum::<u64>() as f64 / 4.0;
    assert_eq!(vest_wash, 0.0, "closed wash (one capital cluster) vests nothing — Layer A's anchor");
    assert!(vest_genuine > 0.0, "genuine work vests its realized-use portion");

    // Honest Layer B design params. EV per identity = S*(vest - rho - p_slash*sigma).
    let (rho, sigma) = (0.30_f64, 1.0_f64);
    let ev = |vest: f64, p_slash: f64| harvest * (vest - rho - p_slash * sigma);
    let ev_genuine = ev(vest_genuine, 0.02); // genuine almost never successfully challenged
    let ev_wash = ev(vest_wash, 0.5); // junk challenged with prob 0.5

    assert!(ev_genuine > 0.0, "PERIPHERY PIN: genuine is net-positive EV (got {ev_genuine})");
    assert!(
        ev_wash < 0.0,
        "PERIPHERY PIN: a closed wash ring is net-negative EV (got {ev_wash}) — the solution's core claim"
    );
    assert!(ev_genuine > ev_wash, "genuine strictly dominates closed wash under the priced periphery");
}
