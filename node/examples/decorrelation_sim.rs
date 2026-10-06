//! Decorrelation anchor — the ACTIVE form, made numeric (grain HCE-A7). Backs
//! `docs/research/DESIGN-decorrelation-anchor-gated-2026-10-06.md` with running numbers, the way
//! `periphery_sim.rs` backs the periphery EV model and `peer_prediction_sim.rs` backs the CA theorems.
//! ANALYTIC + DETERMINISTIC (no RNG): every number is a closed-form EV over the stake model, so the run
//! is bit-identical every time.
//!
//! WHAT THIS TESTS. The A7 residual: adopters in DISTINCT capital clusters (they pass Layer A's a!=b)
//! who are nonetheless NOT independent minds — a shared-controller ring, or herders on a shared prior.
//! The sharpening loop (design doc §6) concluded a PASSIVE divergence statistic is Goodhart one level up,
//! so the anchor must be ACTIVE: adopters bond and stake on predicting each other's NOVEL future work.
//! The question this sim answers: is there a reward level R at which the ring is forced to either REVEAL
//! its correlation (predict well => caught) or FORGO the reward, while genuine-independent and
//! niche-genuine adopters pass for free? i.e. does a filter-coincidence window actually exist?
//!
//! HONEST LABELING (load-bearing). The per-population predictive capability on NOVEL work, and the stake
//! params (bond carry rho, slash sigma, vest value V), are DESIGN PARAMETERS. The elicitation-and-scoring
//! of "prediction of novel future work" is 🔬 UNBUILT and non-trivial (scoring novelty may itself lean on
//! the worth signal — a partial recursion this sim does NOT resolve). So this is a parametric existence
//! check for the SOLUTION SHAPE, not a shipped guarantee.
//!
//! Run: `cargo run --release -p noesis --example decorrelation_sim`.

/// Reward fraction for reporting predictive accuracy `a` of a peer's novel work.
/// 0.5 = chance (no predictive power); 1.0 = perfect. Linear, clamped, 0 at or below chance.
fn reward_frac(a: f64) -> f64 {
    ((a.clamp(0.5, 1.0) - 0.5) / 0.5).max(0.0)
}

struct Pop {
    name: &'static str,
    a_novel: f64, // true capability to predict a PEER's novel future work
    note: &'static str,
}

fn main() {
    // ---- Stake model (DESIGN PARAMS) ----
    let tau = 0.60_f64; // independence threshold: report accuracy <= tau keeps independence vest (D=1)
    let v = 1.0_f64; // value of the independence vest an adopter protects by staying below tau
    let rho = 0.05_f64; // bond carrying cost
    let sigma = 1.0_f64; // slash on a CI-breach (reporting accuracy > tau reveals the shared cause)

    // ---- Populations. a_novel = how well each can predict a PEER's NOVEL work ----
    // Genuine independents cannot predict each other's novel ideas; a controller predicts its own
    // output; niche collaborators are friendly but still cannot predict each other's genuinely novel
    // work; the simulated farm only reaches chance by ACTUALLY running divergent minds (= A9).
    let pops = [
        Pop { name: "genuine-independent", a_novel: 0.50, note: "cannot predict peers (chance)" },
        Pop { name: "niche-genuine (novel)", a_novel: 0.55, note: "friendly but novel work still diverges" },
        Pop { name: "ring (one controller)", a_novel: 0.95, note: "controller predicts its own novel output" },
        Pop { name: "simulated farm", a_novel: 0.50, note: "only reaches chance by BEING K real minds (A9)" },
        Pop { name: "[ref] niche on GENERAL behavior", a_novel: 0.80, note: "why we bond on NOVEL, not general" },
    ];

    // Best response of an adopter with true capability a_novel, given reward R:
    //   HIDE   (report <= tau): keep vest, no reward.           EV = V - rho
    //   REVEAL (report a_novel > tau): forgo vest, get slashed,  EV = R*reward_frac(a_novel) - rho - sigma
    //          but collect the prediction reward.
    // REVEAL is only an option if a_novel > tau. caught == adopter's best response is REVEAL (D=0).
    let decide = |a_novel: f64, r: f64| -> (bool, f64, f64) {
        let hide_ev = v - rho;
        if a_novel <= tau {
            return (false, hide_ev, hide_ev); // cannot exceed tau => always passes
        }
        let reveal_ev = r * reward_frac(a_novel) - rho - sigma;
        let caught = reveal_ev > hide_ev;
        (caught, hide_ev.max(reveal_ev), if caught { reveal_ev } else { hide_ev })
    };

    println!("== Decorrelation anchor (HCE-A7): does the ACTIVE stake separate a ring from genuine minds? ==");
    println!("   params: tau={tau}, V={v}, rho={rho}, sigma={sigma}  [all DESIGN PARAMS; elicitation UNBUILT]\n");

    // Catch threshold per population: R at which REVEAL overtakes HIDE  <=> R*reward_frac > V+sigma.
    println!("Per-population catch threshold R* (reward above which the adopter is FORCED to reveal => D=0):");
    for p in &pops {
        if p.a_novel <= tau {
            println!("  {:<32} a_novel={:.2}  R* = n/a   (never trips: cannot exceed tau) -> {}", p.name, p.a_novel, p.note);
        } else {
            let r_star = (v + sigma) / reward_frac(p.a_novel);
            println!("  {:<32} a_novel={:.2}  R* = {:>5.3}            -> {}", p.name, p.a_novel, r_star, p.note);
        }
    }

    // The window: bonding on NOVEL work, we want R > R*(ring) while niche-genuine(novel) still passes.
    let ring = pops.iter().find(|p| p.name.starts_with("ring")).unwrap();
    let r_star_ring = (v + sigma) / reward_frac(ring.a_novel);
    println!("\nRing is forced to reveal (caught) at any R > {r_star_ring:.3}. niche-genuine(novel) and genuine");
    println!("NEVER trip (a_novel <= tau). So a separating window EXISTS for R > {r_star_ring:.3} when the stake");
    println!("is bonded on NOVEL-work prediction (the [ref] general-behavior row shows why: friends CAN predict");
    println!("general behavior, so bonding on general would false-flag them).\n");

    // Evaluate everyone at a chosen operating point in the window.
    let r_op = 3.0_f64;
    println!("At operating reward R = {r_op} (inside the window):");
    println!("  {:<32} | {:^7} | {:^9} | EV", "population", "D", "verdict");
    println!("  ---------------------------------+---------+-----------+-----------------------------");
    for p in &pops {
        let (caught, _best, ev) = decide(p.a_novel, r_op);
        let d = if caught { 0 } else { 1 };
        let verdict = if caught { "REVEALS => caught" } else { "passes (independent)" };
        println!("  {:<32} | {:^7} | {:<9} | {:+.3}", p.name, d, verdict, ev);
    }

    println!("\nFILTER-COINCIDENCE (the point): the ring's cost is NOT capital — it is the forfeited reward it");
    println!("could have earned BECAUSE it genuinely can predict its own output. The honest independent");
    println!("forfeits that same reward for FREE (it never had the predictability). Same stake: free for the");
    println!("honest, costly for the ring. The cost axis is KNOWLEDGE, uncorrelated with capital.\n");

    println!("BOTTOM LINE (honest, parametric): a separating window EXISTS in this toy model — bond on");
    println!("NOVEL-work mutual prediction, set R > {r_star_ring:.3}, and the ring is forced to reveal (D=0)");
    println!("while genuine and niche-genuine pass (D=1). The simulated farm passes ONLY by actually being K");
    println!("divergent minds (= A9, the funded ceiling, priced). CAVEATS, not rounded away: (1) the");
    println!("elicitation+scoring of 'novel-work prediction' is UNBUILT and may partially recurse to the");
    println!("worth signal; (2) the ring can coordinate on the collusive all-throttle equilibrium — R >");
    println!("R*(ring) only makes INDIVIDUAL reveal dominant, so this inherits the T2 stake+dispute scaffold");
    println!("(peer_prediction_sim.rs); (3) R is a paid subsidy; (4) toy, no adaptive adversary. So: the");
    println!("shape holds and the window is real; the load-bearing unbuilt piece is honest novel-prediction");
    println!("elicitation, and the ceiling is unchanged (A7 collapses into A9, priced).");
}
