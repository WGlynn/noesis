# Decorrelation anchor — a gated design pass (the third uncorrelated toll)

> **Status: design, ready-for-critique, NOT built.** Solo pass (no fan-out), self-adversarially gated.
> Grounds verified against source this session. The honest verdict up front: this does **not** make the
> A7 attack impossible — it **forces A7 (cheap coordination) up into A9 (funded-majority with *real*
> divergence), priced**, at the cost of a false-positive penalty on genuinely tight-knit niche
> communities. That is a cost-raise, not a close. Everything below earns exactly that.

## 1. The grain, stated in the protocol's own terms

Capital-independence (`independent_use_gate`, `node/src/lib.rs:7191-7194`,
`(Some(a), Some(b)) => a != b`) closes exactly one conditional-independence breaker: **shared controlling
capital** (the closed wash ring). It does not close the others the elicitation frame names — shared
prior, herding on public info, semantic copying, sybil-of-a-third-party — all of which pass `a != b` yet
violate `s_i ⊥ s_j | ω_x` (`CALIBRATION-ci-argument-2026-07-21.md:37-58`, removal test at `:46-48`). The
sim pins the damage: past **γ\* ≈ 0.70** task-specific coordination, a junk cell's Correlated-Agreement
beats a genuine one (`node/examples/peer_prediction_sim.rs:124-143`). Detail-free CA closes the
task-*constant* common bias but **not** the task-*specific* fabricated correlation — that is the open
residual (`peer_prediction_sim.rs:109-122` vs `:124-143`).

So A7 is: an attacker fields K adopters in distinct capital clusters (passes `a != b`), coordinates them
to *look* decorrelated on the target cell, and vests the wash. The decorrelation anchor must price that
coordination with a toll whose forgery cost does **not** share a factor with capital.

## 2. The key insight — independence is cheap to fake *narrow*, expensive to fake *broad*

Fabricating `s_i ⊥ s_j | ω_x` on **one cell, one report** is cheap: a puppet-master just issues K
coordinated-to-look-independent reports. The cost does not come from capital; it comes from *breadth*.
The design move is therefore: **do not test conditional independence on the target cell. Test it across
the adopters' entire on-chain behavioral trajectory.**

- Genuinely independent minds have *divergent* trajectories: they adopt diverse things, build in different
  subgraphs, and their action histories carry high entropy relative to one another.
- Coordinated puppets (one controller) or herders (one shared prior) are *mutually predictable* across
  their whole history, not just on the target cell — because the common cause that makes them agree on the
  wash also shapes everything else they do.

The toll this creates is **coordination cost**: to pass a trajectory-wide test, the attacker must run K
identities that are each coherent *and* mutually divergent *and* all still adopt the wash. That is the
cost of actually being K independent minds — and it is **uncorrelated with capital** (a whale buys K
distinct clusters cheaply, but one controller cannot cheaply manufacture K divergent behavioral lives).

**Candidate mechanism (sketch, not spec):** a *decorrelation residual* `D ∈ [0,1]` computed over the
adopter set of a cell, from the mutual predictability of their *full* action histories (not their report
on this cell) — high mutual information / low trajectory entropy ⇒ `D → 0`. Vesting on an independent-use
edge is scaled by `D`: `vest_edge = independent_use_gate(...) · D`. Composed **multiplicatively** with the
existing floors (it can only *lower* vest, never mint), and **fail-closed** (unavailable or
too-correlated ⇒ `D → 0` ⇒ deny, never credit). This is the *breadth-generalization* of the corpus's own
open "detail-free CA" proposal (`DESIGN-harberger-peer-prediction-theorems.md:79-82`, flagged open): CA
across the whole trajectory instead of per-neighborhood.

## 3. Self-adversarial gate (the six hard criteria)

| Gate | Verdict | Why |
|---|---|---|
| Closes A7 **without** the capital oracle | **PARTIAL PASS** | `D` reads behavioral correlation, orthogonal to cluster-id — a genuinely new axis. But it *raises cost*, does not *exclude* (see §4). |
| Forgery cost **uncorrelated** with capital | **PASS (largely)** | Capital buys clusters; it does not buy K divergent decision-makers. A one-controller whale still shares a behavioral fingerprint across its clusters. |
| **No** capturable authority | **PASS** | `D` is computable on-chain from observable activity (entropy / mutual information of action histories). No human or model judge. |
| **Patient-adversary** safe | **PASS** | Waiting alone does not help — the toll is breadth-of-behavior, not elapsed time. A patient attacker beats it only by *actually* behaving divergently, i.e. by being independent. |
| **Worth-blind-safe** | **PASS, with caveat** | Reads behavior, not content worth or target-cell topology — so the acyclic-wash 0% blindness does not apply. Caveat: defeatable by injecting coherent divergent noise (§4). |
| **Fail-closed / non-interference** | **PASS** | Multiplicative, `D → 0` on uncertainty ⇒ under-vest only; can never mint or finalize. Inherits a Samuelson under-credit cost (§4). |

## 4. The honest boundary (where it breaks, exactly)

Two concrete break attempts, neither of which the mechanism survives cleanly:

1. **Simulated K-mind farm.** A compute-rich attacker generates K *coherently divergent* synthetic
   histories — sock-puppets that each look like a real, differently-behaving participant. Trajectory-wide
   `D` is defeated by a convincing simulation of independence. The cost is no longer "coordinate one
   report" but "simulate K divergent lives that fool *every* floor jointly" — a real, large cost increase,
   but **compute-bounded, not impossible.** This is the same adaptive-adversary frontier as the pinned
   `vested_certifier_endorsing_garbage_open_gap`, re-met from the correlation side.
2. **Genuine niche community (false positive).** Five real, independent researchers in an obscure field
   genuinely cite and build on each other. Their trajectories *are* legitimately correlated (one small
   field), so `D` under-credits them. Fail-closed means they under-vest rather than get slashed — but it is
   a real harm, and it penalizes exactly the tight-knit genuine collaboration the protocol most wants to
   reward. Mitigation is a research question, not a settled knob.

**Net:** the decorrelation anchor forces A7 to pay the cost of either (a) actually being K divergent minds,
or (b) convincingly simulating them across their whole histories — which is to say it **collapses A7 into
A9**, the funded-majority-with-manufactured-diversity residual, now *priced* on a third axis instead of
unpriced. That is the win condition named in `resolving-worth-blindness-exogenous-anchors.md` §2.2 — but it
is a ceiling (51%-class, priced), not a proof of exclusion, and it carries a genuine niche-penalty cost.

## 5. Status and what would move it

- 🔬 **Open (design only):** the `D` statistic itself — a trajectory-divergence measure that is
  cheap-to-verify on-chain, hard-to-fake-at-breadth, and does not over-penalize niche-genuine clusters.
  These three pull against each other; the sweet spot is unproven.
- 🔬 **Open:** a non-interference proof that a compromised `D` layer can only deny, never mint (sketched as
  fail-closed/multiplicative here; must be proven before any soft layer ships).
- 🟡 **Composes with:** the time toll (re-opening trigger / provisional finality) and the capital toll
  (Layer A) as the third of three uncorrelated anchors required jointly.
- **Done (2026-10-06):** `node/examples/decorrelation_sim.rs` — the numeric check on the ACTIVE form.
  Result: a separating window **exists** in the toy model. Bonding on NOVEL-work mutual prediction, at
  reward `R > 2.222 = (V+sigma)/reward_frac(ring)`, the ring is forced to reveal (`D=0`) while
  genuine-independent, niche-genuine(novel), and a truly-divergent farm all pass (`D=1`). The
  filter-coincidence is numeric: the ring's cost is forfeited prediction reward (knowledge, not capital);
  the honest adopter forfeits the same reward for free. Honest caveats carried in the sim output:
  novel-prediction elicitation is UNBUILT (but recurses to similarity + novelty, both built — NOT to
  worth, cycle 4); `R` is a paid subsidy. Deterministic, no `noesis` imports (does not depend on the
  uncommitted `lib.rs`).
- **Adaptive-adversary pass (2026-10-06, same sim, HCE-3):** the static window is **not adaptively robust
  on its own.** A ring optimizing its *real* objective (vest the wash, not the prediction-game payoff)
  simply **throttles** — reports accuracy ≤ τ, indistinguishable from an honest adopter's genuine
  inability — and vests for free. The static "window" only bit because it assumed the ring was tempted by
  reward `R` to reveal. So the honest dependency is **not** `R` but the **T2 scaffold**: the window holds
  only if T2 catches a deliberate (collusive, uninformative) throttle with probability `q > W/(W+σ_t)`
  (= 0.5 at `W=σ_t=1`). Below that, the anchor leaks. This downgrades the earlier "window exists" to
  "window exists *conditional on an unbuilt T2 enforcement reaching q > 0.5*." The collusive-throttle
  equilibrium is the live threat, now quantified — not a footnote.

## 5.5 Pushing on T2 — the throttle reduces to the TIME anchor (the honest consolidation)

The adaptive pass (§5) left the whole result resting on "break the collusive-throttle equilibrium
(reach `q > 0.5`)." Pushing on that:

1. **The throttle is worth-blindness re-instantiated.** A throttling ring reports chance-level
   prediction; an honest independent reports chance-level because it genuinely cannot predict. The two are
   **observationally identical on the report** — they differ only in a hidden capability. No mechanism
   reading only the prediction reports can separate them (the same impossibility as the interior wash,
   one layer out). So T2-by-report cannot catch the throttle. The existing T2 result
   (`peer_prediction_sim.rs:72-102`) makes truthful the unique *surviving* equilibrium only "conditional
   on ≥1 live challenger" — and a challenger needs *evidence* the reporter threw the prediction, which the
   report alone cannot supply.

2. **The only escape is post-hoc, over the clawback window.** You cannot tell at report-time, but over
   time you can check whether the "independent" adopters' *realized novel outputs actually diverged*.
   Honest independents diverge (different minds); a throttling ring's outputs stay controller-correlated
   **unless the ring actually runs K divergent minds** (= A9). This detection is a `similarity`+`novelty`
   computation (both built, worth-blind — cycle 4), evaluated at clawback/re-open time, not at mint.

3. **Consequence (honest downgrade of the "three independent tolls" framing):** the decorrelation anchor's
   *enforcement* is the TIME anchor. Decorrelation is not a fully independent third toll — it is the
   **readout that time provides**. The genuinely uncorrelated tolls reduce to **{capital, time}**, with
   time doing double duty: provisional-finality clawback *and* throttle-detection via realized-divergence.
   This consolidates the frontier (fewer independent axes than `resolving-worth-blindness §2.2` claimed)
   rather than adding one.

4. **Ceiling unchanged, reached via time not reward.** Gaming realized-divergence = run K genuinely
   divergent minds across the window = A9, priced. A patient ring that injects divergent-looking realized
   noise pays toward that same A9 cost. So the throttle is broken by time at the cost of forcing A9 — the
   same ceiling, now reached through the time anchor instead of the reward `R`.

Status: 🔬 reduction *argument*, not proven; the latency cost is real (you learn late, bounded by how long
correlation takes to manifest), and the realized-divergence statistic has the same
specificity/noise-injection design surface as §4.1. Next cheap check: a longitudinal sim —
honest-set-diverges vs throttling-ring-stays-correlated over N periods — to see whether a cheap
realized-divergence readout separates them before forcing full A9.

## 6. Design-sharpening loop (self-critique → fix, to convergence)

### Cycle 1 — the fatal flaw in §2: a *passive* statistic is Goodhart one level up
The §2 sketch defines `D` as the observable mutual predictability of adopter histories. Critique: any
*passive, published* statistic is optimized against offline. An attacker who knows `D` generates K
synthetic histories that score maximally divergent — the §4.1 simulated-farm break is not an edge case,
it is the *generic* defeat of every observational measure. We did exactly this to the interior signals in
`wash_sim` (0%); a passive `D` just relocates that same Goodhart to the trajectory layer. **A passive
decorrelation measure cannot be the anchor.**

**Fix:** make the signal *active and costly*, not observational. Tie decorrelation to a cost the attacker
pays *even knowing the test*. Concretely — **bond-on-mutual-unpredictability**: adopters stake on
predicting each other's *future novel* actions; the payout structure rewards genuine *inability* to
predict. This converts "measure correlation" (gameable offline) into "pay for the correlation you hide."

### Cycle 2 — why the active form is a *filter-coincidence*, and where it still leaks
Re-examine the fixed design. A coordinated ring (one controller) genuinely *can* predict its own members.
Under the stake it faces a fork: predict well ⇒ reveal the shared cause ⇒ fail CI, caught; or predict
badly on purpose ⇒ forfeit the stake. **Either branch costs.** A genuinely independent set predicts each
other badly *for free* — they really cannot. So the stake is **free for the honest, costly for the ring**:
an economic filter that is simultaneously the independence filter ([filter-coincidence] — the risk filter
*is* the economic filter, so there is no honest-cost tradeoff). And the cost is knowledge/prediction, not
money ⇒ it passes the uncorrelated-with-capital gate cleanly.

Critique of the fix: (a) the ring escapes by running K *genuinely* separate decision processes so its
members truly cannot predict each other — but that is *being* K independent minds = **A9**, the funded
ceiling. Consistent: the anchor collapses A7→A9 as intended, now Goodhart-resistant rather than offline-
beatable. (b) **Niche penalty persists and may worsen**: friendly collaborators who talk *can* predict
each other and would be taxed. Partial fix: bond on predicting each other's *novel/divergent* future work,
not general behavior — genuine collaborators still diverge in what they independently create, a ring's
novel output all serves the ring. This *softens* the niche penalty; it does not remove it.

### Cycle 3 — convergence check (diminishing returns)
Remaining critiques are second-order: the elicitation game inherits peer-prediction's non-unique-
equilibrium problem (T2), so it needs the same stake+dispute scaffold already designed for the worth layer
— i.e. the decorrelation anchor is *not* a free-standing mechanism, it reuses the T2 resolution
(`peer_prediction_sim.rs:72-102`). No new structural break found in this cycle ⇒ converged.

### Cycle 4 — the load-bearing check: does the elicitation recurse to *worth*? (No)
If scoring "did A predict B's novel work" needed a worth judgment, the anchor would collapse back onto
the unbuilt worth oracle and would NOT be an independent third toll — killing the additivity claim. It
does not. Scoring prediction accuracy is `similarity(predicted_X, realized_Y)` — a *match*, worth-blind:
"did A guess WHAT B would do," not "was it good." The ingredients are all worth-blind and mostly built:
- **similarity** — `pom_scores_with_similarity_floor_q16` (built; used in `periphery_sim.rs`),
- **novelty** (to restrict to *novel* work) — `temporal_novelty` (built; used in `wash_sim.rs`),
- **time** (B's work must be realized to score the prediction) — the clawback/provisional-finality window.

None is the worth oracle. So the decorrelation anchor is the **most buildable** of the three tolls and is
genuinely independent of the capital oracle and the learned `v(S)`. Residual (a real design knob, not a
recursion): the prediction must be *specific* enough that chance-matching is hard — "B will produce
something novel" is free; "B will produce novel work near region Z" is not. Specificity is the tuning
surface; it does not reintroduce worth.

**Sharpened verdict (supersedes §4's net):** the decorrelation anchor should be an **active
bond-on-mutual-unpredictability filter**, not a passive divergence statistic — because only the active
form resists the offline-Goodhart defeat that kills every observational measure. In active form it is a
filter-coincidence (free for the honest, costly for the ring), uncorrelated with capital, oracle-free, and
fail-closed. Its honest ceiling is unchanged — it collapses A7 into A9 (priced, not excluded) — and it
still carries a *softened-but-nonzero* niche penalty and a dependence on the T2 stake+dispute scaffold.
The `decorrelation_sim` RED (§5) should now test the **active** form: can the stake separate a real set
from a ring without over-taxing niche-genuine, and does the ring's best response reduce to "run K real
minds" (A9)?
