# Learned `v(S)` from a deep-ancestry, outcome-labelled seed — the moat research program

> Status discipline (CLAUDE.md): ✅ built · 🟡 designed · 🔬 open — **never round up.** This document
> reports what the code and the real-data runs actually show. Where a result is null or open, it says so.
>
> Turns `internal/RESEARCH-learned-vs-from-merged-prs.md` (commit `593bdc8`) into a build plan, executed.
> Reconciled first against `internal/STATUS-LEDGER.md` MOAT-1, `internal/MVP-SCOPE-JULY-2026.md` R1, and
> the value code — per [[verify-before-rebuild]]. Prior art it does NOT re-derive: the DeepFunding harness
> (`data/deepfunding/`), the `ValueOracle` seam (`node/src/lib.rs:283`), the adversarial test (2026-07-02),
> and the structural layered defense (proven, 253/253).

## 0. Reconciliation — what was already known, and the one thing that was open

The learned-`v(S)` predictive claim was tested on real DeepFunding jury labels **twice** (round 1 +
the faithful `coalition_features` port), and went **NULL both times**. The faithful port
(`data/deepfunding/RESULTS-FAITHFUL.md`) diagnosed the cause exactly, and it was **not** a label
problem — it was a **topology** problem:

> ANCESTOR closure (the SHIPPED object, `lib.rs:7092`): **singletons 95/115** ⇒ f1/f2/f3 DEGENERATE.

For 95 of 115 judged DeepFunding repos the provenance-ancestor coalition is `{r}` alone (they are
graph leaves), so synergy / connectedness / depth collapse to constants and the exact shipped quantity
**cannot be learned there for lack of ANCESTRY, not labels.**

Two things follow, and both are already settled in the ledger:

1. **The moat does not rest on the predictive win.** STATUS-LEDGER MOAT-1 + RESULTS-FAITHFUL §★: the
   un-gameability moat is the **structural layered defense** (semantic floor → Hodge residual slash →
   v6 identity pricing → endorsement slashing → escalation court → irreducible global-capture
   assumption), and it is **demonstrated** (253/253). The learned `v(S)` is a *separate, additional*
   quality-discrimination layer — upside, not foundation.
2. **The single named-open obligation is a dataset.** MOAT-1 open obligation, verbatim: *"a deep-ancestry
   outcome-labelled dataset ... commit/PR-level lineage, not repo-level leaf deps, so the shipped
   ancestor object is non-degenerate."* MVP-SCOPE R1 calls this "the moat data hunt ... calendar-bound,
   not effort-bound." **This program supplies that dataset and runs the test it unblocks.**

So the reframed program is precise: *not* "learn `v(S)` to beat the proxy" (mis-specified on honest
static labels — no adversary to resist ⇒ null is expected). It is: **supply a non-degenerate
deep-ancestry dataset, then run BOTH the predictive port (now meaningful) AND the adversarial /
isomorphism-invariance gate on real outcome labels** — the instruments that can actually move the
MOAT-1 and CLAUDE.md "learned-`v(S)` + isomorphism-invariance gate" rows.

### 0.1 The box `v(S)` must live in (structural constraints, from the code)

A replacement value function is only admissible if it preserves every property the consensus path and
the Sybil-resistance design already depend on. These are the constraints the learned model must satisfy
— the BOX, cited to source:

| # | constraint | source | why it is load-bearing |
|---|---|---|---|
| B1 | **pure + deterministic**: same `(cells, θ)` ⇒ bit-identical output on every replica (no floats on the consensus path, no wall-clock, no map-iteration-order dependence) | `ValueOracle` contract, `lib.rs:278-282` | replicas must agree on ONE canonical `v(S)`; nondeterminism forks the chain |
| B2 | **integer output, exactly one value per input cell, in commit order** | `lib.rs:284-286` | the aggregator (`pom_scores_with_oracle`, `lib.rs:304`) sums per-cell value into per-contributor standing |
| B3 | **submodular** coverage value: a redundant contribution adds little (`v(S)=|union coverage|`) | `synergy` module, `lib.rs:3283`, 3301 | padding / duplication cannot inflate value; diminishing returns is the anti-spam property |
| B4 | **Myerson-restricted**: value only from provenance-CONNECTED sub-coalitions (`v^g(S)` = sum of `v` over connected components under parent edges) | `lib.rs:3284`, 3340 `v_graph` | a disconnected coalition (forged/unrelated cells) cannot pool value; provenance is required to co-earn |
| B5 | **anonymity-RELAXED — NOT symmetric** | Sybil-resistance design; `value_v6` identity pricing `lib.rs:1219`; SCOPE-CF | a *fresh* identity is worth zero by construction; a symmetric (anonymous) value function would let a ring mint standing by renaming ⇒ symmetry is deliberately broken |
| B6 | **the 4 set-features are the only interface to structure** — breadth `ln(1+|union|)`, synergy `|union|/Σ|indiv|`, connectedness `frac(parent∈S)`, depth `longest-parent-chain/|S|` | `coalition_features`, `lib.rs:7092-7133` | all four are on-chain-derivable, need no oracle; the LABELS carry the outside signal (`lib.rs:7091`) |
| B7 | **floor can only LOWER, never rescue** — semantic floor AND-composes, incompressible-noise content ⇒ 0 regardless of structure | `v_outcome_floored`, `lib.rs:7216-7235` | structure cannot manufacture value from noise; single-sourced from the intake floor so a cell the chain won't mint can't score |
| B8 | **serialisable to a fixed canonical artifact, governance-pinned** (a version bump on the Constitution measurement-amendment frame, like `theta_sim_q16`), NOT a runtime plugin | `lib.rs:272-276` | the whole network agrees on ONE `v(S)`; a per-node model re-introduces the authority the design removed |

**Consequence for the model class:** the learned object is NOT a free-form net. It is a learned
*scoring of the four structural features* (the shipped Bradley-Terry estimator, `outcome::train`
`lib.rs:7143`), composed with the coverage/provenance structure that already enforces B3–B5, and floored
by B7. Training changes the 4 weights; it cannot change the box. Determinism (B1) is met by fixed-point
/ fixed-seed integer inference at deploy — training happens off-chain, only the pinned weight blob ships.

---

## 1. The dataset (Phase 1) — crates.io deep-ancestry provenance graph

_Builder: `data/crates/build_dataset.py`. Source: crates.io `db-dump.tar.gz` (public daily snapshot)._

**Why crates.io.** Noesis is Rust; the crates dependency graph is the cleanest public instance of the
MOAT-1 open obligation. Every crate has a REAL, DEEP transitive-dependency chain — ancestry that is
non-degenerate *by construction*, unlike DeepFunding's repo-level leaves. And a published crate that
many others depend on is a maintainer-and-ecosystem **authoritative value label** — the "borrow GitHub
authority off-chain as a training seed" unlock, realised as reverse-dependency reuse.

**Provenance reading (identical to the faithful port).** Edge `r → d` = *r depends on d* = *d is prior
work r built upon* = **d is a provenance ANCESTOR of r**. `out(r)` = r's dependencies (its ancestors);
`in(d)` = d's reverse-deps (its descendants / reuse). `coverage(x) = {x} ∪ out(x)`. Only NORMAL
(`kind=0`), non-optional, non-target-gated deps are kept (build/dev/platform deps are not runtime
provenance). Edges are crate-level (collapsed across versions), de-duplicated, self-edges dropped.

**Outcome labels.** (a) reverse-dependency count `|in(d)|` = direct reuse; (b) `downloads` =
usage-weighted reuse. Both are external and uncorrelated with any on-chain identity ring (poison-
resistant by construction — the seed is OFF-chain, never on-chain authority; it bootstraps the model,
it does not gate live submissions).

**Anti-degeneracy receipt (the whole point):** `data/crates/graph/stats.json`.

<!-- PHASE1-RESULTS:START -->
✅ **Built and non-degenerate** (snapshot `2026-07-15-020010`; `data/crates/RESULTS.md`):

| quantity | value |
|---|---|
| crates (nodes) | 299,775 |
| normal crate-level dependency edges | 1,940,630 |
| crates with ≥1 reverse-dep (reuse label) | 111,693 |

**Anti-degeneracy receipt — the bar is cleared decisively:**

| | ancestor-coalition singleton frac | median \|S\| | max |
|---|---|---|---|
| DeepFunding (old, null) | 0.826 | 1 | 3 |
| **crates.io (this)** | **0.180** (0/3000 on judged units) | **115 (127 judged)** | 1303 |

The topology block that invalidated both DeepFunding runs is gone; the shipped ancestor object is now
richly testable.
<!-- PHASE1-RESULTS:END -->

---

## 2. The model (Phase 2) — box-constrained learned scoring

_Trainer: `data/crates/moat_test.py`. Mirrors `outcome::{coalition_features, train, v_outcome,
pairwise_accuracy}` (`lib.rs:7078-7206`) EXACTLY, ANCESTOR direction._

Learn `f(coalition_features(ancestor-coalition(r))) → value` with the shipped Bradley-Terry estimator on
preference pairs derived from the reuse label (winner = the more-reused crate). Constrained to the
Phase-0 box: the four features (B6), submodular/Myerson coverage (B3–B5) baked into the coalition
construction, deterministic integer inference (B1) via the same fixed-point path the on-VM port uses.
Output = a versioned weight blob (B8), the canonical artifact that drops into `ValueOracle`.

<!-- PHASE2-RESULTS:START -->
✅ **Trained.** All four features now vary (non-degenerate): breadth 4.57±1.55, synergy 0.33±0.10,
connected 0.68±0.07, depth 0.27±0.25. Learned weights (last seed) `[breadth 0.08, synergy 1.56,
connected −0.12, depth −0.29]` — almost all weight on **synergy** (non-redundant coverage), the
intuitive reuse signal. A real determinism bug in the `depth` feature was found and fixed here (see
Phase 3.3).
<!-- PHASE2-RESULTS:END -->

---

## 3. The un-gameability gate (Phase 3) — the OPEN theorem, on real labels

Three instruments, on the non-degenerate real data:

1. **Predictive (now meaningful):** held-out pairwise accuracy of the learned `v(S)` vs the fixed
   `proxy_value` (f0 breadth) baseline, in the ANCESTOR direction — the exact shipped object, which
   DeepFunding could not test. Reported with the sampling-noise band; null is reported as null.
2. **Adversarial (the correctly-specified moat instrument):** inject a gamed coalition that inflates the
   fixed proxy (orphan/padding dumps that pump raw coverage), show the proxy pays it above genuine work
   while the floored/learned measure denies it — measured held-out, not asserted. Mirrors
   `outcome::tests::gamed_coalition_pays_the_proxy_but_the_learned_measure_denies_it`, now on real data.
3. **Isomorphism-invariance (CLAUDE.md OPEN gate):** relabel identities / permute a coalition ⇒ value
   unchanged. The property that stops a ring minting value by renaming itself; the anonymity-relaxation
   (B5) must survive.

<!-- PHASE3-RESULTS:START -->
Full detail + tables in `data/crates/RESULTS.md`. Summary:

| instrument | result |
|---|---|
| **(1) Predictive** (learned vs proxy f0, held-out, 20 seeds) | **NULL** — learned 0.5201 vs proxy 0.5167, Δ**+0.0034** inside the ±0.0144 noise band, 12/20 wins. A *third* null, now on **non-degenerate** topology at scale ⇒ the null is a robust property, **not** a DeepFunding artifact. The four structural features explain only a sliver (~0.52 vs 0.50) of downstream reuse. **Strengthens** the reframe: the predictive win is upside, not the moat. |
| **(2) Adversarial** (gamed coalition pumps the proxy) | **PASS (direction)** — proxy gap `+0.0000` (cannot tell genuine from gamed), learned gap `+0.0500` (denies the gamed unit). Honest scope: constructed feature vectors at real percentiles (same *class* as the shipped 2026-07-02 fixture), not a real attack crate in the live graph. |
| **(3) Iso-invariance** (relabel + permute) | **INVARIANT — after a fix.** First FAILED: `depth` changed under permutation (0.144→0.212). Cause: `faithful_port`'s longest-path memo is order-dependent on large sets, and the crate-level graph *cycles* (version-collapsing) so longest-chain is ill-defined. Fixed via SCC-condensation longest-path (`_longest_chain_condensation`), deterministic in `S` alone. **The gate caught a real B1 determinism bug and forced its canonical fix.** Scope: this is the 4-feature permutation/relabel invariance, NOT the general graph-iso theorem (still open). |

**Design note (load-bearing):** Noesis's real Cell model is a single-parent DAG (`cell.parent: Option`,
acyclic) so consensus is not exposed to the cycle bug today — but any multi-parent value feature MUST
use the SCC-condensation rule or it forks replicas.
<!-- PHASE3-RESULTS:END -->

---

## 4. Integration + honest labelling (Phase 4)

The canonical weight artifact drops into the `ValueOracle` seam (`lib.rs:283`) as `LearnedOracleV1`,
selected by a governance-pinned protocol version (B8); `NoveltyOracleV0` (`lib.rs:293`) remains the
pre-seed fallback. **Launch copy NEVER claims the moat until Phase 3 passes on real data** (MVP-SCOPE:
"Launch copy = floor only, never 'un-gameable value chain'"). The STATUS-LEDGER MOAT-1 row is updated
with whatever Phase 3 actually shows — this document does not pre-write the verdict.

<!-- PHASE4-STATUS:START -->
**Status after this program (never rounded up):**
- ✅ **DONE:** the deep-ancestry outcome-labelled dataset (MOAT-1 open obligation #2 / MVP-SCOPE R1) —
  supplied, non-degenerate, reproducible.
- ✅ **DONE:** the iso-invariance gate ran on real data, caught a real determinism bug, and passes on
  the 4 features after the SCC-condensation fix.
- 🔬 **STILL NULL (decisively):** the learned-`v(S)` predictive win over the fixed proxy — third null,
  now with topology excluded as a cause. The correct conclusion is that this half is **not
  load-bearing** for the moat (the moat is the structural defense), not that more data will flip it.
- 🔬 **STILL OPEN:** (a) a real *adaptive* adversary (not a constructed fixture); (b) the general
  graph-isomorphism-invariance theorem; (c) HCE-3 adaptive convergence. None is unblocked by more crates
  data alone.

**Integration:** no `LearnedOracleV1` is wired into consensus by this program — a value function that
does not beat the proxy predictively has no reason to replace `NoveltyOracleV0` on the live path, and
launch copy stays **floor-only**. What this program delivers is the *instrument and the dataset* to keep
testing the moat honestly, plus the canonical cycle rule any future multi-parent `v(S)` needs. The
`ValueOracle` seam (`lib.rs:283`) remains the drop-in point when/if an adversarial-robustness win on a
real adaptive adversary is demonstrated.
<!-- PHASE4-STATUS:END -->
