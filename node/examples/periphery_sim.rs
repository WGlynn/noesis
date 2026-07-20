//! Periphery EV model (grain HCE-4-2) — prices Layers A+B of `docs/DESIGN-periphery-solution.md` and
//! measures whether a wash-ring is negative-EV, plus the break-even independence-capital the protocol
//! must demand. Follow-on to `wash_sim.rs` (which proved graph-internal discernment is 0%).
//!
//! HONEST LABELING (load-bearing). The HARVEST `S` (per-identity standing a novel block earns) is
//! MEASURED on the real deployed franchise `pom_scores_with_similarity_floor_q16`. Everything else —
//! carrying-cost/rent (Layer B), the capital-independent vesting gate (Layer A), slash probability,
//! window, opportunity rate — is a DESIGN PARAMETER: Layers A+B are 🟡 designed-not-built. So this is a
//! parametric proof of the SOLUTION SHAPE and its break-even, NOT a shipped guarantee. It shows the
//! asymmetry the theory needed (rent, not time) as a number, and where it bottoms out (independent
//! capital = Bitcoin-51%-class).
//!
//! Run: `cargo run --release -p noesis --example periphery_sim`.

use noesis::vesting::independent_use_gate;
use noesis::{pom_scores_with_similarity_floor_q16, Cell, Script};
use std::collections::HashMap;

const THETA_Q16: u64 = 62259; // 0.95 — deployed franchise threshold

fn cell(id: u64, contributor: u8, data: &[u8]) -> Cell {
    cell_p(id, contributor, None, data)
}
fn cell_p(id: u64, contributor: u8, parent: Option<u64>, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [1u8; 32], args: vec![contributor] },
        type_script: Script { code_hash: [0xB0; 32], args: vec![contributor] },
        parent,
        timestamp: id,
        data: data.to_vec(),
    }
}
fn noise(seed: u8, n: u8) -> Vec<u8> {
    (0..n).map(|i| seed.wrapping_add(i.wrapping_mul(41))).collect()
}

/// A collaboration TREE: identity `ids[i]` builds on `ids[i-1]` (same topology for genuine and wash,
/// per `node/tests/discernment.rs` — so the ONLY difference the gate sees is capital-independence).
fn tree(ids: &[u8]) -> Vec<Cell> {
    ids.iter()
        .enumerate()
        .map(|(i, &who)| cell_p(i as u64, who, if i == 0 { None } else { Some(i as u64 - 1) }, &noise(who, 40)))
        .collect()
}

/// MEASURED vest fraction: run the built Layer A gate over `cells` with unit per-cell value and the
/// given capital-cluster map, return (vested / total) — the fraction of harvest that actually vests.
/// This replaces an ASSUMED vest fraction with the real function's output over a real graph.
fn measured_vest_fraction(cells: &[Cell], caps: &HashMap<Vec<u8>, u64>) -> f64 {
    let unit = vec![1u64; cells.len()];
    let vested: u64 = independent_use_gate(cells, &unit, caps).iter().sum();
    vested as f64 / cells.len() as f64
}

/// MEASURED harvest: per-identity standing a novel-junk block earns on the real deployed franchise.
fn measured_harvest_per_identity() -> f64 {
    // One identity, one novel block — the standing a wash sybil banks per key under v0.
    let cells = vec![cell(0, 1, &noise(0x55, 48))];
    let s = pom_scores_with_similarity_floor_q16(&cells, THETA_Q16);
    s.get(&vec![1u8]).copied().unwrap_or(0) as f64
}

/// EV per identity for the three populations, given design parameters.
/// - `vest`: fraction of harvest that VESTS (Layer A). Closed wash = 0 (no independent downstream use);
///   genuine = 1 (independent minds build on it for free); capital-faked wash = 1 but pays `cap_cost`.
/// - `rho`: total carrying cost over the window as a fraction of harvest (Layer B rent). Same for all.
/// - `p_slash`, `sigma`: challenge probability × slash fraction of harvest (Layer B). Wash is
///   challengeable (junk); genuine survives (p≈0).
/// - `cap_cost`: opportunity cost of the independent capital an identity must post to make its use count
///   as independent (Layer A), in harvest units. Free for genuine (independence is real); the lever the
///   protocol raises against a capital-faking ring.
fn ev(harvest: f64, vest: f64, rho: f64, p_slash: f64, sigma: f64, cap_cost: f64) -> f64 {
    harvest * (vest - rho - p_slash * sigma) - cap_cost
}

fn main() {
    let s = measured_harvest_per_identity();
    println!("== Periphery EV model (HCE-4-2): does pricing Layers A+B make a wash-ring negative-EV? ==");
    println!("   harvest S (MEASURED, real pom_scores, per novel-junk identity) = {s}");
    println!("   [rent/capital/window/slash are DESIGN PARAMS — Layers A+B are designed-not-built]\n");

    // Honest default design params.
    let rho = 0.30_f64; // carrying cost over the window = 30% of harvest
    let p_slash = 0.5_f64; // a junk contribution is challenged with prob 0.5 ...
    let sigma = 1.0_f64; //   ... and slashed for its full declared value
    let genuine_p = 0.02_f64; // genuine work is (almost) never successfully challenged

    println!("Design params: rho(rent)={rho}, p_slash(wash)={p_slash}, sigma={sigma}, p_slash(genuine)={genuine_p}\n");

    // ---- The three populations. Vest fractions are now MEASURED by running the built Layer A gate
    //      (independent_use_gate) over real graphs — not assumed. Same 4-node topology for both, so the
    //      only difference the gate sees is capital-independence. ----
    let genuine = tree(&[1, 2, 3, 4]); // 4 distinct minds
    let wash = tree(&[5, 6, 7, 8]); // 4 sybils, same topology
    // Genuine: each mind is its own capital cluster (real independence).
    let genuine_caps: HashMap<Vec<u8>, u64> =
        [(vec![1], 1u64), (vec![2], 2), (vec![3], 3), (vec![4], 4)].into_iter().collect();
    // Closed wash: all sybils share ONE capital cluster (one actor's keys).
    let wash_caps: HashMap<Vec<u8>, u64> =
        [(vec![5], 0u64), (vec![6], 0), (vec![7], 0), (vec![8], 0)].into_iter().collect();
    let vest_genuine = measured_vest_fraction(&genuine, &genuine_caps);
    let vest_wash = measured_vest_fraction(&wash, &wash_caps);
    println!("Vest fractions (MEASURED by independent_use_gate over real graphs, not assumed):");
    println!("  genuine (4 distinct capital clusters) : {vest_genuine:.3}  (< 1.0: the leaf cell has no");
    println!("      independent child YET — cold-start symmetry, paid by the novelty floor, not this gate)");
    println!("  closed wash (1 capital cluster)       : {vest_wash:.3}  (no independent use anywhere)\n");

    let ev_genuine = ev(s, vest_genuine, rho, genuine_p, sigma, 0.0); // measured vest, survives challenge
    let ev_wash_closed = ev(s, vest_wash, rho, p_slash, sigma, 0.0); // measured vest (0), pays rent
    println!("Population EV per identity (before any capital is posted to fake independence):");
    println!("  genuine        : {ev_genuine:>8.3}   (vests its realized-use portion, survives challenge — NET POSITIVE)");
    println!("  wash (closed)  : {ev_wash_closed:>8.3}   (Layer A: closed ring vests 0; pays rent — NET NEGATIVE)");
    println!("  => a closed wash-ring is negative-EV by construction: it can never earn independent");
    println!("     downstream use, so it only pays rent. This is the periphery (Layer A) doing the work.\n");

    // ---- The only way wash vests: post independent capital to fake independence. Find break-even. ----
    // Wash that posts capital vests fully but is still challengeable and pays rent + capital opportunity.
    // Profitable iff  S*(1 - rho - p_slash*sigma) > cap_cost.  Break-even capital cost per identity:
    let breakeven_cap = s * (1.0 - rho - p_slash * sigma);
    println!("Capital-faking wash: to make its use count as independent (Layer A), each identity must post");
    println!("independent capital. Wash stays profitable only while its opportunity cost < break-even:");
    println!("  break-even capital cost / identity = S*(1 - rho - p_slash*sigma) = {breakeven_cap:.3}");
    if breakeven_cap <= 0.0 {
        println!("  => break-even is <= 0: rent + expected slash ALREADY exceed the harvest. Wash is negative-EV");
        println!("     at ANY capital cost, before the independence requirement even bites. Layers A+B alone");
        println!("     dominate the junk ring.\n");
    } else {
        println!("  => the protocol makes wash negative-EV by requiring independence-capital whose opportunity");
        println!("     cost exceeds {breakeven_cap:.3}/identity. Below that, a FUNDED ring still profits —");
        println!("     the Bitcoin-51% residual, now priced.\n");
    }

    // ---- Layer C: grounded v(S) collapses the junk harvest itself ----
    // Once v(S) predicts external use, junk's VESTED value -> ~0 (it is never externally used). Model as
    // a harvest haircut h for junk (genuine unaffected).
    println!("Layer C (grounded v(S)) — junk's vested value collapses as v(S) learns to predict external use:");
    for h in [1.0_f64, 0.5, 0.2, 0.05] {
        let be = s * h * (1.0 - rho - p_slash * sigma);
        println!("  junk harvest haircut h={h:>4}: effective S={:>6.3}, break-even capital = {be:>7.3}", s * h);
    }
    println!("  => as Layer C grounds, the junk harvest -> 0, so the break-even capital -> 0: wash is");
    println!("     negative-EV at any capital. The three layers COMPOSE — A denies free vesting, B prices");
    println!("     the wait, C zeroes the prize.\n");

    // ---- Sweep: the rent (Layer B) needed to make closed wash negative even if Layer A partially leaks ----
    println!("Robustness — closed-wash EV if Layer A partially LEAKS (some intra-ring use slips as independent):");
    println!("  leak fraction | closed-wash EV/id | verdict");
    for leak in [0.0_f64, 0.1, 0.25, 0.5, 1.0] {
        let e = ev(s, leak, rho, p_slash, sigma, 0.0);
        println!("  {leak:>12.2} | {e:>17.3} | {}", if e < 0.0 { "negative-EV (pruned)" } else { "PROFITABLE — leak too high" });
    }
    println!("  => wash only turns profitable if Layer A leaks enough vesting to beat rent+slash; the");
    println!("     capital-independent vesting gate's job is to keep that leak below ~{:.0}%.\n", 100.0 * (rho + p_slash * sigma));

    println!("BOTTOM LINE (honest): with Layer A (capital-independent vesting) + Layer B (rent+slash), a");
    println!("closed wash-ring is negative-EV by construction, and a capital-faking ring is negative-EV");
    println!("below a NAMED break-even capital ({breakeven_cap:.3}/identity at these params). Layer C drives");
    println!("that break-even toward 0. The residual is a ring that commands genuine independent capital");
    println!("above break-even = the Bitcoin-51% class, priced not excluded. Harvest is measured; the rest");
    println!("is the design shape — build Layers A+B to convert this model into a guarantee.");
}
