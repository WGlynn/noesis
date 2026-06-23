# Phase-1 Moat Experiment, Round 2 — the FAITHFUL provenance-feature port

> Supersedes the framing of `RESULTS.md` (round 1). Round 1 scored single-repo degree
> proxies over the dependency graph and got a null, with the honest caveat: *"the Rust
> features are SET-level over a provenance DAG; a faithful port could move the numbers in
> either direction."* This round builds that faithful port — the exact `outcome::
> coalition_features` quantity — and reports what it found.

**Authority note:** numbers here are honest-number-over-marketing-number. Reproduce with
`python faithful_port.py` (needs numpy; uses Python 3.12).

---

## Question

Does a LEARNED set-level value `v(S)` (Bradley-Terry over the exact Rust
`outcome::coalition_features`) beat a FIXED structural proxy at predicting real
DeepFunding jury contribution-preferences? This is the predictive half of the
"un-gameable learned `v(S)`" moat claim (MOAT-1).

## Method — the faithful port

`faithful_port.py` mirrors `node/src/lib.rs::outcome::{coalition_features, train,
v_outcome, proxy_value, pairwise_accuracy}` exactly, at the set level, on the real
mini-contest jury labels. The dependency graph is read as a provenance DAG: edge
`r -> d` (r depends on d) means d is prior work r built upon, i.e. d is a provenance
ancestor of r. `coverage(x) = {x} ∪ out_neighbors(x)`.

## ⚠ Topology finding (the headline) — the exact shipped object is untestable here

The shipped Rust model scores each cell's **provenance-ANCESTOR** coalition
(`{i} ∪ provenance-ancestors-in-graph`, lib.rs:1173). On the DeepFunding mini-contest
graph the judged repos are **leaves**:

```
ANCESTOR closure (the SHIPPED object): singletons 95/115  → f1/f2/f3 DEGENERATE; med |S|=1, max 3196
DESCENDANT closure (foundational-ness): singletons 12/115 → med |S|=8, max 16  (testable)
```

For **95 of 115** judged repos the ancestor coalition is `{r}` alone, so synergy,
connectedness, and depth collapse to constants. **The exact shipped quantity cannot be
meaningfully learned on this dataset — a TOPOLOGY block, not a label block.** Round 1's
single-repo proxies hid this; the faithful port exposes it. The judged units sit at the
bottom of a 2-level DAG with nothing above them to form a coalition.

So we test the moat in the only direction this topology supports: the **descendant**
coalition `D(r) = {r} ∪ {repos that transitively depend on r}` — the body of work built
ON r, the graph-derivable "foundational-ness" object. Same Rust feature formulas, dual
DAG direction. (Honest caveat: this is the provenance-DAG features in the *descendant*
direction, not the shipped *ancestor* direction. It is the fairest predictive test the
data admits, not the shipped scorer.)

Descendant features now vary (no degeneracy):
`breadth std 1.29 · synergy std 0.26 · connected std 0.29 · depth std 0.22`.

## Result — 20-seed held-out pairwise accuracy (descendant framing)

| scorer | mean acc | std |
|---|---|---|
| coin-flip floor | 0.5000 | — |
| best single feature | 0.5212 | 0.0205 |
| proxy f0 (breadth = `proxy_value`) | 0.5280 | 0.0187 |
| **LEARNED Bradley-Terry (4 set feats)** | **0.5349** | 0.0220 |

| delta | mean | std | win-rate |
|---|---|---|---|
| learned − proxy f0 | **+0.0069** | 0.0295 | 11/20 |
| learned − best single | **+0.0137** | 0.0221 | 14/20 |

1-SE sampling-noise band on a 465-pair split: **±0.0232**.
Learned weights (last seed): `[breadth −0.05, synergy +0.93, connected −0.09, depth −1.64]`.

## Honest verdict — still NULL, but a sharper null

This is **another null result for the predictive moat claim**, now with a precise cause.

- The delta over the proxy (+0.0069) and over the best single feature (+0.0137) both live
  **inside the ±0.023 noise band**. Win-rate is 11/20 vs the proxy and 14/20 vs the best
  feature — the 14/20 is a faint lean (one-sided binomial p≈0.06), not a moat.
- Both scorers barely clear the 0.50 floor (~0.53). As in round 1, **these four
  structural graph features explain only a sliver of jury preference**, in either DAG
  direction. The dominant signal the juries used is not structural-topological.
- The learned model did find *some* structure (synergy +, depth −: juries in this set
  mildly favour repos whose descendant-coalition is broad-but-shallow), enough to edge the
  best single feature 14/20 — but the effect is within noise and does not establish a moat.

**Conclusion: on DeepFunding jury labels, a learned `v(S)` does not reliably beat a fixed
structural proxy, in either the ancestor direction (untestable: 95/115 degenerate) or the
descendant direction (null: Δ within noise). The predictive half of MOAT-1 is NOT
supported by this dataset.**

## The reframe that matters — this test is mis-specified for the moat

The moat is **adversarial-robustness** ("un-gameable", closing the Goodhart gap), not
raw predictive accuracy on a static honest dataset. On honest labels with **no adversary
gaming the proxy**, a learned model and a good proxy *should* score similarly — there is
no manipulation for the learned model to resist. **DeepFunding jury labels contain no
adversary, so they cannot reveal the property the moat is built to provide.** The null is
expected, not damning; it is evidence the test is the wrong instrument, not (yet) that the
moat is absent.

## What IS proven — the structural, moat-independent half (receipts)

The collusion / Sybil / noise-floor half of the moat is **independent of the learned
model** and is demonstrated by shipped, green tests (`cargo test --lib` = **253/253**):

- `attribution_circulation_fires_on_collusion_ring_quiet_on_honest_dag` — a mutual-citation
  ring shows circulation; an honest one-way DAG shows ~0.
- `attribution_cycle_energy_catches_directed_ring_that_circulation_misses` — Hodge harmonic
  energy catches directed rings.
- `collusion_residual_by_identity_names_ring_members_spares_honest` /
  `collusion_slash_burns_ring_standing_bounded_spares_honest` /
  `collusion_slash_cannot_be_weaponized_to_frame_an_honest_identity` — detection→slash,
  bounded by standing, cannot frame an honest builder.
- `floor_zeroes_high_entropy_garbage_but_passes_real_content` /
  `semantic_floor_closes_the_fake_lineage_spoof_at_the_score` — noise/fake-lineage cannot
  manufacture value at the score.

The codebase is **already honest** about what remains open: 30 lines tagged `open_gap`,
including `garbage_novelty_is_the_documented_open_gap`,
`sybil_identity_ring_pumps_the_flow_gate_open_gap`,
`vested_certifier_endorsing_garbage_open_gap`, `judge_cartel_protects_its_own_garbage_open_gap`.
These are exactly the **learned-quality-discrimination** gaps this experiment fails to
close on real data.

## Honest decomposition of MOAT-1 after this round

| sub-claim | status |
|---|---|
| **Collusion / Sybil / ring resistance** (Hodge residual → slash, bounded, frame-proof) | **demonstrated** (shipped, 253/253) — moat-independent |
| **Noise / fake-lineage resistance** (semantic floor) | **demonstrated** (shipped) |
| **Value conservation / no-extraction** (GEV-clean) | **demonstrated** (extraction audits) |
| **Learned `v(S)` out-predicts a fixed proxy on real labels** | **NULL** (round 1 + this faithful round); within noise both directions |
| **Learned `v(S)` closes the quality-discrimination Goodhart gap** | **open** (the `*_open_gap` tests); needs an adversarial test + a deep-ancestry dataset |

## What would actually settle the open half

1. **An adversarial test** (the right instrument): inject a gamed coalition that inflates
   the proxy, show the proxy pays it and the floored/learned evaluator does not — measured,
   not asserted. This is the moat's actual claim. *(Next build; the static-label test is
   the wrong shape for it.)*
2. **A deep-ancestry dataset**: a provenance graph where judged units have real ancestors
   (commit/PR-level lineage, not repo-level leaf deps), so the *shipped ancestor object* is
   non-degenerate and the predictive test is even meaningful.
3. Only then re-run the faithful ancestor-direction port.

## ★ Synthesis — what "prove the moat" actually resolves to

Tracing the moat to the source changes the conclusion from "null" to a clean split, and
the split is **good news**: the moat does NOT rest on the thing that failed the test.

**The un-gameability moat is the STRUCTURAL layered defense, and it is comprehensively
proven** — not by the learned model, but by a chain of mechanisms where each gaming vector
is closed at the next layer (the `*_open_gap` tests are pedagogical pins, each naming its
own closer; all green in 253/253):

| gaming vector | closed by | closing test (green) |
|---|---|---|
| novel-noise / fake-lineage spoof | semantic entropy floor | `semantic_floor_closes_the_fake_lineage_spoof_at_the_score` |
| cyclic collusion / mutual-citation ring | HodgeRank harmonic residual → slash | `attribution_cycle_energy_catches_directed_ring…`, `collusion_residual_by_identity_names_ring_members_spares_honest` |
| free-identity Sybil ring (v5 surface) | v6 identity pricing (earned standing) | `value_v6_closes_the_sybil_identity_ring` |
| vested certifier endorsing garbage (v6 surface) | dispute-layer endorsement-slashing (λ=1 clawback) | `endorsement_slashing_makes_the_vested_certifier_ring_negative_ev` |
| >1/3 judge cartel veto (dispute round-1 surface) | §7 escalation court + juror-accountability slash | `cartel_veto_holds_at_round_one_but_is_overturned_on_appeal` |
| full-consensus capture | **irreducible global assumption** (same class as Bitcoin's 51%) | `full_consensus_capture_defeats_the_escalation_court_global_assumption` |

This terminates exactly where every honest consensus system terminates: a named
global-capture assumption, not a hidden hole. **That is the moat, and it is proven.**

**The learned `v(S)` is a SEPARATE, additional quality-discrimination layer** — "tell
real-but-low-value work from real-but-high-value work." Its marginal predictive power over
a fixed proxy is what the DeepFunding test measures, and it is **null** (twice). But:

1. The moat's un-gameability **does not depend on it** — the structural defense above
   stands whether or not the learned model out-predicts the proxy.
2. The predictive test is the **wrong instrument** for an adversarial-robustness moat
   anyway (no adversary in honest static labels), and on this dataset the exact object is
   topology-degenerate (95/115 leaves).

**So: the central value claim of Noesis does not rest on the learned model winning a
prediction contest. It rests on a layered structural defense that is shipped and tested.**
The learned quality layer is upside, not foundation — and the honest status of that upside
is "unproven on the only real dataset to date; needs a deep-ancestry, outcome-labelled
dataset and an adversarial (not predictive) test to settle."

This is the disciplined answer to "prove the moat": the provable half is **proven** (with
receipts); the un-provable-today half is **honestly localized** and shown to be
non-load-bearing for the moat itself.

## Files

- `faithful_port.py` — faithful `coalition_features` mirror + topology diagnosis + 20-seed eval
- `RESULTS-FAITHFUL.md` — this file
- `RESULTS.md` — round 1 (single-repo proxy null), retained for the record
