//! Peer-prediction theorems, made numeric (grain HCE-2-selfreport). Backs
//! `docs/DESIGN-harberger-peer-prediction-theorems.md` with running numbers, the way
//! `periphery_sim.rs` backs the periphery EV model. ANALYTIC + DETERMINISTIC (no RNG): every number is
//! a closed-form expectation over the signal model, so the run is bit-identical every time.
//!
//! HONEST LABELING (load-bearing). The signal model (prior on worth, signal accuracy) and the stake
//! params (rent, challenge prob, slash) are DESIGN PARAMETERS — the peer-prediction wrapper is 🟡
//! designed-not-built. This is a parametric proof of the SOLUTION SHAPE for the two open theorems, not a
//! shipped guarantee. It demonstrates: (T1) Correlated-Agreement separates genuine from collusion ONLY
//! when the reference set is capital-independent (CI holds) — the same predicate Layer A enforces; and
//! (T2) the truthful equilibrium is payoff-dominant but NOT unique in the bare inner game (the known
//! impossibility), and becomes the unique SURVIVING equilibrium once the Harberger stake is priced in.
//!
//! Run: `cargo run --release -p noesis --example peer_prediction_sim`.

/// Correlated-Agreement score (Dasgupta-Ghosh / Shnayder et al. spirit): same-task agreement minus the
/// cross-task (marginal) baseline. Truthful reporting is a best response iff the same-task correlation
/// (through the shared latent worth) exceeds the marginal agreement — i.e. iff the reference signal is
/// stochastically relevant AND independent given worth.
fn ca_score(same_task_agree: f64, cross_task_agree: f64) -> f64 {
    same_task_agree - cross_task_agree
}

/// Agreement that two {H,L} reports coincide, given each side's P(H).
fn agree(p_h_a: f64, p_h_b: f64) -> f64 {
    p_h_a * p_h_b + (1.0 - p_h_a) * (1.0 - p_h_b)
}

fn main() {
    // ---- Signal model (DESIGN PARAMS) ----
    let prior_high = 0.5_f64; // P(worth = high)
    let p_h_given_high = 0.9_f64; // an independent honest reporter says H on a high-worth cell w.p. 0.9
    let p_h_given_low = 0.2_f64; //   ... and on a low-worth (junk) cell w.p. 0.2  (stochastic relevance)
    let marginal_h = prior_high * p_h_given_high + (1.0 - prior_high) * p_h_given_low;

    println!("== Peer-prediction theorems, numeric (HCE-2-selfreport) ==");
    println!(
        "Signal model [DESIGN PARAMS]: P(high)={prior_high}, P(H|high)={p_h_given_high}, P(H|low)={p_h_given_low}, marginal P(H)={marginal_h:.3}\n"
    );

    // ---- T1: does Correlated-Agreement separate genuine from collusion? Depends on the REFERENCE. ----
    // Genuine truthful reporter, reference = an INDEPENDENT honest peer (CI holds): same-task agreement is
    // correlated through the shared worth ω; cross-task agreement is only marginal.
    let same_genuine = prior_high * agree(p_h_given_high, p_h_given_high)
        + (1.0 - prior_high) * agree(p_h_given_low, p_h_given_low);
    let cross_genuine = agree(marginal_h, marginal_h);
    let score_genuine = ca_score(same_genuine, cross_genuine);

    // A closed ring on a junk (low) cell, reference = an INDEPENDENT honest peer. Ring pumps "H" always;
    // the honest reference reports H only w.p. P(H|low). Cross-task uses the ring's constant H vs marginal.
    let same_ring_indep = agree(1.0, p_h_given_low); // ring always-H vs honest-on-junk
    let cross_ring_indep = agree(1.0, marginal_h);
    let score_ring_indep = ca_score(same_ring_indep, cross_ring_indep);

    // The SAME ring, but reference = ANOTHER RING MEMBER (CI FAILS — one controller). They fabricate a
    // fake worth-correlation: coordinate identical reports per task (same-task agree = 1) and coordinate to
    // differ across tasks (cross-task agree = 0.5). This manufactures the correlation CA rewards.
    let same_ring_corr = 1.0;
    let cross_ring_corr = 0.5;
    let score_ring_corr = ca_score(same_ring_corr, cross_ring_corr);

    println!("T1 (graph-generalization) — CA score by reference-set choice:");
    println!("  genuine truthful (independent ref)      : {score_genuine:+.3}  (the honest premium = Var_ω of P(H|ω))");
    println!("  closed ring     (INDEPENDENT ref)       : {score_ring_indep:+.3}  (CI holds ⇒ collusion scores BELOW zero)");
    println!("  closed ring     (CORRELATED ref, CI✗)   : {score_ring_corr:+.3}  (ring fabricates the correlation ⇒ scores ABOVE genuine)");
    let sep_indep = score_genuine - score_ring_indep;
    let sep_corr = score_genuine - score_ring_corr;
    println!("  separation (genuine − ring): independent ref = {sep_indep:+.3} (>0 ✓ separates) | correlated ref = {sep_corr:+.3} (≤0 ✗ blind)");
    println!("  => CA separates truth from collusion ONLY on capital-INDEPENDENT references — identically");
    println!("     Layer A's independent_use_gate predicate. Same condition, bought once.\n");

    // ---- T2: is the truthful equilibrium unique in the bare inner game G0? NO. Priced game G+? YES. ----
    // G0 (no stake): the all-H uninformative profile is a Nash equilibrium — if every reference always
    // reports H, any single reporter scores CA(agree(·,1), agree(·,1)) = same − cross where both legs use
    // the constant reference. same = P(r_i=H), cross = P(r_i=H) ⇒ score = 0 regardless of r_i. No
    // unilateral improvement ⇒ collusion is an equilibrium, paying 0. Truthful equilibrium pays the premium.
    let payoff_truthful_eq = score_genuine; // > 0
    let payoff_collusive_eq = 0.0_f64; // uninformative all-H: everyone scores exactly 0, still Nash
    println!("T2 (inner-equilibrium uniqueness) — bare inner game G0 (peer-prediction alone, NO stake):");
    println!("  truthful equilibrium payoff  : {payoff_truthful_eq:+.3}  (payoff-DOMINANT)");
    println!("  all-H uninformative payoff   : {payoff_collusive_eq:+.3}  (still a Nash equilibrium — no unilateral gain)");
    println!("  => UNIQUENESS IS FALSE in G0: truth pays more but the collusive equilibrium coexists");
    println!("     (the known impossibility). So 'unique truthful equilibrium' must be STRUCK as stated.\n");

    // G+ (stake): the CA score's SIGN is the survival gate (and it is ≤0 for the ring precisely because
    // the protocol draws references from the INDEPENDENT set, T1). score>0 ⇒ the declaration is
    // peer-supported and survives challenge ⇒ standing V is RETAINED; score≤0 ⇒ unsupported ⇒ the full
    // declared V is slashed ⇒ standing retained = 0. (Same 1/0 retention logic as periphery_sim's vest.)
    let rho = 0.30_f64; // Harberger carrying cost as a fraction of declared V  [DESIGN PARAM]
    let p_challenge = 0.5_f64; // junk is challenged w.p. 0.5                     [DESIGN PARAM]
    let sigma = 1.0_f64; // slash = full declared V on a successful challenge     [DESIGN PARAM]
    let genuine_p_challenge = 0.02_f64; // genuine work is (almost) never successfully challenged
    let v = 1.0_f64; // unit declared value (EV is linear in V; only the sign matters)
    let retained = |score: f64| if score > 0.0 { 1.0 } else { 0.0 };
    let ev = |score: f64, p_ch: f64| retained(score) * v - rho * v - p_ch * sigma * v;
    let ev_ring_plus = ev(score_ring_indep, p_challenge); // score ≤0 (independent ref) ⇒ retained 0 ⇒ pure cost
    let ev_genuine_plus = ev(score_genuine, genuine_p_challenge); // score >0 ⇒ retained 1 ⇒ net positive
    println!("T2 — priced game G+ (peer-prediction ⊕ Harberger rent+slash ⊕ dispute):");
    println!("  ring / collusive EV per identity : {ev_ring_plus:+.3}  (0 peer-support ⇒ over-declare ⇒ rent+slash only ⇒ NEGATIVE)");
    println!("  genuine truthful EV per identity : {ev_genuine_plus:+.3}  (premium-supported V, survives challenge ⇒ POSITIVE)");
    println!("  => the stake removes the indifference that made collusion a Nash equilibrium in G0.");
    println!("     Truthful is the UNIQUE SURVIVING equilibrium of G+ (conditional on ≥1 live challenger).\n");

    // ---- T1-RESIDUAL: what does DETAIL-FREE CA close, of the channels capital-independence does NOT? ----
    // The calibration (CALIBRATION-ci-argument-2026-07-21.md) listed shared-prior/herding/semantic-copy as
    // CI-breakers Layer A misses. Detail-free CA = CA with the cross-task (marginal) term subtracted, so it
    // needs no known prior. It closes a TASK-CONSTANT common bias but NOT a TASK-SPECIFIC ω-external one.
    // Partition the residual numerically. (Attacker goal: inflate a JUNK cell's CA to ≥ genuine's +0.245.)
    println!("T1-RESIDUAL — detail-free CA (cross-task subtraction) partitions the leftover CI channels:\n");

    // Channel A — TASK-CONSTANT common bias (a shared belief "work here tends to be good", same on every
    // task). It inflates same-task AND cross-task agreement EQUALLY ⇒ the CA subtraction cancels it. On a
    // junk cell it cannot manufacture a task-specific spike ⇒ cannot lift junk to genuine. CA robust.
    println!("  Channel A: task-CONSTANT common bias (one flavor of 'shared prior') — CA on a JUNK cell:");
    let junk_honest_ca = score_ring_indep; // -0.35: honest-on-junk baseline (independent ref)
    for bias in [0.0_f64, 0.25, 0.5, 0.9] {
        // bias forces H on BOTH same-task and cross-task equally ⇒ contributes 0 to (same − cross).
        let ca_junk_a = (1.0 - bias) * junk_honest_ca + bias * 0.0;
        let verdict = if ca_junk_a >= score_genuine { "BEATS genuine ✗" } else { "stays below genuine ✓" };
        println!("    bias={bias:>4}: CA(junk) = {ca_junk_a:+.3}  ({verdict})");
    }
    println!("    => a task-constant bias only ATTENUATES toward 0; it never lifts junk above genuine. CLOSED.\n");

    // Channel B — TASK-SPECIFIC ω-external correlation (herding on a per-cell public signal, or semantic
    // copying / coordinated sybil reports on THIS cell). It spikes same-task agreement without touching the
    // cross-task term (the external signal is independent across tasks) ⇒ SURVIVES the subtraction.
    println!("  Channel B: task-SPECIFIC ω-external correlation (herding / semantic-copy) — CA on a JUNK cell:");
    let fabricated_ca = score_ring_corr; // +0.5: coordinate-identical-this-task, coordinate-to-differ-across
    let mut threshold = None;
    for gamma in [0.0_f64, 0.25, 0.5, 0.7, 0.75, 1.0] {
        let ca_junk_b = gamma * fabricated_ca + (1.0 - gamma) * junk_honest_ca;
        let beats = ca_junk_b >= score_genuine;
        if beats && threshold.is_none() {
            threshold = Some(gamma);
        }
        let verdict = if beats { "BEATS genuine ✗" } else { "below genuine ✓" };
        println!("    coordination γ={gamma:>4}: CA(junk) = {ca_junk_b:+.3}  ({verdict})");
    }
    // Solve the crossover exactly: γ*fab + (1-γ)*honest = genuine  ⇒  γ* = (genuine-honest)/(fab-honest).
    let gamma_star = (score_genuine - junk_honest_ca) / (fabricated_ca - junk_honest_ca);
    println!("    => crossover γ* = {gamma_star:.3}: past ~{:.0}% task-specific coordination, JUNK BEATS GENUINE.", 100.0 * gamma_star);
    println!("       Detail-free CA does NOT close this — it is the true residual (with the 3rd-party-sybil");
    println!("       gap in the capital proxy). This is the sharpened open problem, not 4 flat channels.\n");

    // ---- T1-RESIDUAL-BACKSTOP: CORRECTED by CALIBRATION-backstop-conjunctive-2026-07-21.md ----
    // My first pass claimed a conjunctive vest gate defeats the cheap coordination attack (standing → 0).
    // The 10-agent calibration found TWO overclaims + a not-built premise; this section now shows them.
    //
    //  (a) NOT BUILT. The conjunctive v(S) = retained(CA)*vest is a DESIGN PROPOSAL, not the running value
    //      function. value_v5..v8 (lib.rs:1182-1445) compose novelty * flow_gate and gate the SEED on
    //      soulbound STANDING (>= FLOOR), NOT on independent_use_gate. Grep: `independent_use_gate` has
    //      exactly ONE hit in lib.rs — its own definition (7164); its docstring says NOT wired into
    //      consensus. So "v(S) requires the capital-independence vest" credits nothing deployed.
    //  (b) BK-1 OVERSTATED. "vest=0 for the ring" is a strictly-CLOSED-ring special case. A SEMI-FUNDED
    //      ring that rents ONE 3rd-party distinct-cluster identity to build on ONE of its cells vests that
    //      cell in FULL — independent_use_gate is a binary per-cell cluster-id compare (lib.rs:7191-7200),
    //      so one distinct-cluster child ⇒ full vest on that cell. Partial standing, bought cheaply.
    //  (c) BK-2 UNSOUND. Do NOT conflate the per-identity vesting break-even (~9/identity, periphery_sim)
    //      with the NETWORK 51% floor — different quantities. The dual attack (coordinate CA for free +
    //      rent one distinct-cluster sybil) defeats BOTH gates well below any 51% capture.
    let vest_genuine_frac = 0.75_f64; // measured, periphery_sim
    let vest_closed_ring = 0.0_f64; // strictly-closed ring: no independent child ⇒ vests nothing
    let vest_semifunded = 0.25_f64; // rents 1 distinct-cluster identity ⇒ 1 of 4 ring cells vests FULL
    let standing = |ca: f64, vest: f64| retained(ca) * vest * v; // the (designed, not-built) conjunctive form
    println!("T1-RESIDUAL-BACKSTOP — CORRECTED: the vest backstop is designed-not-built AND leaks to a semi-funded ring:\n");
    println!("  Conjunctive v(S) = retained(CA)*vest is a DESIGN PROPOSAL (value_v5..v8 gate on STANDING, not this gate):");
    println!("    genuine                    : CA {score_genuine:+.3} * vest {vest_genuine_frac}  = standing {:.3}  (earns)", standing(score_genuine, vest_genuine_frac));
    println!("    strictly-closed ring       : CA {score_ring_corr:+.3} * vest {vest_closed_ring}  = standing {:.3}  (defeated — but this is the SPECIAL CASE)", standing(score_ring_corr, vest_closed_ring));
    println!("    SEMI-FUNDED ring (rent 1)  : CA {score_ring_corr:+.3} * vest {vest_semifunded} = standing {:.3}  (LEAKS — cheap, one 3rd-party identity, << 51%)", standing(score_ring_corr, vest_semifunded));
    println!("  => the backstop only zeroes the STRICTLY-closed ring; a semi-funded ring buys partial vest cheaply.");
    println!("     And none of this is deployed: the built anti-coordination defense is the STANDING-vest gate");
    println!("     ('an unvested identity pumps nothing', value_v7) + semantic + outcome floors, composed");
    println!("     MULTIPLICATIVELY. The real open residual is the built system's own pinned gap — a VESTED");
    println!("     identity certifying garbage (adversary::vested_certifier_endorsing_garbage_open_gap),");
    println!("     closed only by the learned v(S) on REAL realized-use labels (the crown-jewel open).\n");
    println!("  RECONCILIATION: peer-prediction here is a candidate for the v8 OUTCOME factor (an oracle-free");
    println!("     content floor, ∈[0,1], multiply-only-lowers) — NOT a separate mechanism. My backstop had");
    println!("     re-derived, under new names, a composition the value layer already ships differently.\n");

    println!("BOTTOM LINE (honest, calibrated): T1 — capital-independence is the NECESSARY shared-controller");
    println!("filter (not all of CI; detail-free CA closes task-constant bias, task-specific correlation open).");
    println!("T2 — uniqueness FALSE standalone, resolved by the stake (built as multiplicative floors in v5..v8).");
    println!("BACKSTOP — DESIGNED-not-built + leaks to a semi-funded ring; the deployed defense is standing-vest,");
    println!("the real residual is the vested-certifier-garbage gap. Everything bottoms out at 51%-class capture.");
}
