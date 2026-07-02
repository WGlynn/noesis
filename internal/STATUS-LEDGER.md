# STATUS LEDGER - the single source of truth for HCE + moat claim status (PRIVATE / stealth)

> Created 2026-06-23 as the structural fix from the Phase-5 RSAW audit: caveats leaked because every
> doc restated status in its own words. From now on, **every other doc cites a row of this ledger by
> ID rather than restating the status.** If a claim's status changes, it changes HERE, once.
>
> Authority for: `docs/whitepaper/noesis-whitepaper.tex`, `docs/WHITEPAPER.md`, `internal/thesis/MOAT-STACK.md`,
> `internal/thesis/DESIGN-wills-equilibrium.md`, `internal/thesis/DESIGN-peg-proof-template-for-hce.md`,
> `internal/thesis/DESIGN-adaptive-convergence-theorem.md`, `ROADMAP.md` Phase 1.
>
> Status vocabulary (use exactly these words everywhere):
> - **demonstrated** - runs and is tested in the open reference node (cite the test/module).
> - **designed** - specified end-to-end, not built; a deterministic build remains.
> - **proof-templated** - a published result supplies a proof template; named open theorems remain
> before it is a theorem for our object. NOT a synonym for "proven". (Replaces the older, looser
> phrase "proven-in-template".)
> - **conjecture** - stated, openly labeled, no proof and no template that closes it.
> - **null-tested** - a real-data experiment was run and did NOT support the claim (unsupported);
> distinct from refuted (see the row's evidence for why).
> - **open** - named obligation, neither built nor proof-templated yet.

## A. HCE property rows

| ID | claim | status | evidence | open obligation |
|---|---|---|---|---|
| **HCE-1-contrib** | (1) Nash: honest contribution is the no-regret unilateral action | demonstrated | novelty->0 padding/sybil, geometric saturation, standing-gating; `value` module + gaming suite | none for contribution; see HCE-1-report for the report half |
| **HCE-1-report** | (1) Nash: honest self-report is incentive-compatible, `p*b >= (1-p)*g` | designed; proof-templated by PEG/SD-PP, with two named open theorems (graph-generalization + C4 inner-uniqueness) | `nash_honesty` (4 tests) proves the IC inequality CONDITIONAL on the catch-probability `p`; the layer that supplies a high `p` with no oracle is the PEG instantiation (thesis/DESIGN-peg-proof-template-for-hce.md), not built (M3) | build the peer-elicitation `p`-supplier (M3); discharge the two open theorems below |
| **HCE-2-cyclic** | (2) Coalition-proof against cyclic collusion (rings / mutual-citation / manufactured flow) | demonstrated | HodgeRank harmonic-energy certificate detects circulation on topology alone, wired to `collusion_slash`/`unified_slash`; tested | none for the cyclic half |
| **HCE-2-selfreport** | (2) Coalition-proof against the symmetric-lie self-report collusion equilibrium | designed; proof-templated by SD-PP (removes the risk-attitude loophole only), open for the joint deviation | SD-PP SD-truthfulness makes truth payoff-dominant under any monotone utility (a UNILATERAL property); the symmetric-lie co-equilibrium is a JOINT deviation SD-truthfulness does NOT kill; bonded BTS backstop is designed (M4) | prove elimination of the symmetric-lie co-equilibrium (M4); graph-generalization + C4 |
| **HCE-3-adaptive** | (3) Adaptive-stability / Goodhart-robust: honest stays an equilibrium under the retraining dynamic vs an adversary who learns | conjecture (designed harness; convergence open; **predictive real-data test null TWICE incl. faithful port**) | retraining harness wired (`load_prefs -> train -> v_outcome_floored -> seed`); the learned `v(S)` real-data test (DeepFunding) is null in both round 1 and the faithful `coalition_features` port (see MOAT-1, `RESULTS-FAITHFUL.md`); convergence theorem open (M2). Note: the predictive-on-honest-labels test is mis-specified for this adversarial-robustness claim ⇒ the adversarial test (MOAT-1 obligation 1) is the correct instrument | the adaptive-convergence theorem (M2, uniqueness of the fixed point); the ADVERSARIAL test + a deep-ancestry dataset (see MOAT-1) |

### Two named open theorems gating HCE-1-report and HCE-2-selfreport
| ID | theorem | status | evidence | open obligation |
|---|---|---|---|---|
| **TH-graph-MI** | graph-generalization of the determinant-based mutual-information score from a single report to cooperative-game value `v(S)` over a provenance DAG | open | PEG proves it for single-fact reports among symmetric discriminators; the DAG-valued object is unproven | derive truthfulness/convergence for the graph-structured score (candidate: HodgeRank-determinant operator) |
| **TH-C4-unique** | inner-equilibrium uniqueness (M2 condition C4): the inner reporter game has a unique equilibrium so the retraining map `T` is single-valued | open | PEG narrows it (its game converges) but does not discharge it for the graph game | potential-game / monotone argument with the Hodge potential |

### Self-report IC layer notes (cross-referenced)
- **M1 existence Proposition** (thesis/DESIGN-wills-equilibrium.md §5): the honest profile satisfies (1) for
 self-report only **conditional on the catch-probability `p` (supplied by M3, not yet built)**;
 proof-templated. It does NOT "hold today" as an unconditional result.
- **Cheng-Friedman escape** (HCE-2-cyclic / Sybil half): see SCOPE-CF below; that line is about
 *identity-multiplication* Sybils only, not self-report collusion.

## B. Moat / positioning rows

| ID | claim | status | evidence | open obligation |
|---|---|---|---|---|
| **MOAT-1** | cooperative-economic structure: an un-gameable LEARNED `v(S)` beats a fixed structural proxy and closes the Goodhart gap | **predictive half NULL twice (round 1 + faithful port); structural half DEMONSTRATED** | round 1 (`data/deepfunding/RESULTS.md`, single-repo proxies): Δ+0.0021/20-seed, 11/20. **Faithful port (`data/deepfunding/RESULTS-FAITHFUL.md`, the exact `coalition_features` mirror): two findings.** (a) TOPOLOGY block — the shipped ANCESTOR object is degenerate on DeepFunding: 95/115 judged repos are leaves ⇒ singleton coalitions ⇒ f1/f2/f3 constant; the exact quantity is untestable here for lack of ANCESTRY, not labels. (b) In the only testable direction (descendant/foundational coalition), learned Δ+0.0069 vs proxy-f0 (11/20) and +0.0137 vs best-single (14/20), both **inside** the ±0.023 noise band, all scorers ~0.53 vs 0.50 floor ⇒ **NULL**. KEY REFRAME: the predictive-accuracy-on-honest-static-labels test is **mis-specified** for an adversarial-robustness moat (no adversary present ⇒ nothing for a learned model to resist ⇒ null is expected, not damning). **SYNTHESIS (RESULTS-FAITHFUL §★): the moat = the STRUCTURAL layered defense, and it is PROVEN; the learned `v(S)` predictive win is upside, NOT load-bearing for un-gameability.** Layered defense (each `*_open_gap` test names its closer, all green in 253/253): noise-spoof→semantic floor; cyclic ring→Hodge residual slash; free-id Sybil→v6 identity pricing (`value_v6_closes_the_sybil_identity_ring`); vested certifier→dispute endorsement-slashing (`endorsement_slashing_makes_the_vested_certifier_ring_negative_ev`); judge-cartel→§7 escalation court (`cartel_veto_holds_at_round_one_but_is_overturned_on_appeal`); terminates in the irreducible global-capture assumption (`full_consensus_capture_…_global_assumption`, same class as Bitcoin 51%) | (1) the predictive upside: ✅ **ADVERSARIAL test BUILT** (`outcome::tests::gamed_coalition_pays_the_proxy_but_the_learned_measure_denies_it`, 2026-07-02) — supplies the adversary the honest-static-label pull lacked: an injected gamed coalition (6 orphan dumps flooding raw coverage) makes the fixed proxy WORSE than a coin-flip held-out (accuracy ≤0.1 — it pays the attacker above genuine work) while the learned/floored `v(S)` denies it (accuracy ≥0.9 held-out). The denial is the STRUCTURE measure's, not the floor's — the gamed content is compressible so the entropy floor keeps it (floored score >0). HONEST SCOPE: a CONSTRUCTED adversarial fixture, not real-outcome data — it demonstrates the learned measure resists a proxy-gaming attack (the correctly-specified claim), not that a learned model beats the proxy on real DeepFunding labels. STILL OPEN: a deep-ancestry outcome-labelled dataset so the ancestor object is non-degenerate; (2) then HCE-3. NOT blocking the moat claim, which stands on the structural defense |
| **MOAT-1-anchor** | "no ground-truth oracle" headline | demonstrated (as scoped) | there is **no IMMEDIATE per-decision oracle**; the design DOES anchor on aggregate realized outcomes, which retrain `v(S)` over time. A competitor can fairly say "you also anchor on outcomes, in aggregate" - so the honest headline is "no immediate per-decision oracle; aggregate realized outcomes retrain `v(S)`" | none; keep the aggregate-outcome-anchor caveat attached wherever the headline appears |
| **MOAT-2** | cybernetic governor: per-block health sensor + pre-finalization effectors | demonstrated (static sensor) / designed (adaptive loop) | HodgeRank residual sensor built + tested; adaptive control loop is HCE-3 (open) | the adaptive loop = HCE-3 |
| **MOAT-3** | convergence-native substrate ("AI is blockchain and blockchain is AI") | designed (architecturally committed), not demonstrated | the fusion is real in the DESIGN end-to-end; what makes it bite (learned `v(S)` on real downstream outcomes) is data-blocked and shares MOAT-1's null status | MOAT-1 port; learned-measure-as-consensus on real outcomes |

## C. Shared open attacks (NOT differentiators - record honestly, claim nothing)

| ID | attack | status | note |
|---|---|---|---|
| **ATK-bribery** | `p + epsilon` bribery / out-of-band payment to a reporter or validator to lie | **open shared attack** | This is a shared weakness of bonded peer-prediction / reputation mechanisms generally; HCE's bond raises the cost of the bribe but does **not** provably close it. Do NOT list bribery-resistance as a differentiation in any doc. |

## D. Scope caveats that must travel with their quotable lines

| ID | line | scope caveat that must be attached |
|---|---|---|
| **SCOPE-CF** | the Cheng-Friedman Sybil-escape line ("a fresh identity is worth zero by construction, so false names cannot inherit standing") | This defeats *identity-multiplication* Sybils only. It does NOT defeat a single-identity self-report collusion ring (everyone agrees on the same lie) - that is the separate HCE-2-selfreport / M4 obligation. The quote must not be lifted without this scope. |
| **SCOPE-MI-anchor** | any "no ground-truth oracle" headline | attach the MOAT-1-anchor caveat: no IMMEDIATE per-decision oracle; aggregate realized outcomes retrain `v(S)`. |

## How to cite this ledger
- Public whitepaper files (`.tex`, `docs/WHITEPAPER.md`): do NOT name this internal file or use the
 words "moat" / "compete with X" / "front-run"; instead carry the *status wording* from the matching
 row verbatim (demonstrated / designed / proof-templated / conjecture / null-tested) so the public text
 and this ledger never diverge.
- Internal docs: cite by ID, e.g. "status: see STATUS-LEDGER HCE-2-selfreport".
- One change rule: if a status changes, edit the row here first, then propagate the wording.
