# Crates.io deep-ancestry moat experiment — results

> The Phase-1/2/3 run of the learned-`v(S)` moat on a **non-degenerate** deep-ancestry dataset, the
> named-open obligation from STATUS-LEDGER MOAT-1 and MVP-SCOPE R1. Reproduce:
> `python build_dataset.py && python moat_test.py` (needs numpy; Python 3.12). Source:
> crates.io `db-dump.tar.gz` snapshot `2026-07-15-020010`.
>
> Honest-number-over-marketing-number. Null is reported as null.

## Why this dataset exists

The learned-`v(S)` predictive claim went NULL twice on DeepFunding. The faithful port
(`../deepfunding/RESULTS-FAITHFUL.md`) proved the cause was **topology, not labels**: 95/115 judged
repos are graph leaves, so the SHIPPED ancestor coalition (`{r} ∪ provenance-ancestors`, `lib.rs:7092`)
collapses to a singleton and the structural features degenerate to constants. MOAT-1's remaining open
obligation is verbatim *"a deep-ancestry outcome-labelled dataset ... so the shipped ancestor object is
non-degenerate."* The crates.io dependency graph is exactly that: real deep transitive-dependency chains
(ancestry) + reverse-dependency reuse (an external, Sybil-uncorrelated outcome label). A crate many
others depend on is a maintainer-and-ecosystem *authoritative value label* — the "borrow authority
off-chain as a training seed" unlock, realised.

## Phase 1 — the dataset (`build_dataset.py` → `graph/`)

| quantity | value |
|---|---|
| crates (nodes) | 299,775 |
| normal crate-level dependency edges | 1,940,630 |
| crates with ≥1 reverse-dep (reuse signal) | 111,693 |

**Anti-degeneracy receipt — the whole point** (`graph/stats.json`, ancestor-closure sample n=4000):

| | ancestor-coalition singleton frac | median \|S\| | p90 | max |
|---|---|---|---|---|
| DeepFunding (old, null) | **0.826** | 1 | — | 3 |
| **crates.io (this)** | **0.180** | **115** | 493 | 1303 |

The topology block that invalidated both DeepFunding runs is **gone**. On the 3000 judged crates
(those with a real ancestor coalition and reuse signal) the singleton fraction is **0/3000** and the
median ancestor coalition is **127** crates deep. The shipped ancestor object is now richly testable.

Edge semantics (identical to the faithful port): `r → d` = *r depends on d* = *d is a provenance
ANCESTOR of r*; `coverage(x) = {x} ∪ deps(x)`; only NORMAL, non-optional, non-target deps kept;
crate-level, de-duplicated. Outcome label = reverse-dependency count (usage-weighted tiebreak by
downloads).

## Phase 2 — box-constrained learned `v(S)` (`moat_test.py`)

Shipped Bradley-Terry estimator (`outcome::train`, `lib.rs:7143`) over the four coalition features
(`coalition_features`, `lib.rs:7092`), ANCESTOR direction, trained on preference pairs from the reuse
label (winner = more-reused crate). Feature distribution across the 3000 judged crates:

```
f0 breadth   mean 4.574 std 1.553   f1 synergy   mean 0.329 std 0.102
f2 connected mean 0.680 std 0.065   f3 depth     mean 0.274 std 0.249   (all now VARY — non-degenerate)
```

Learned weights (last seed): `[breadth 0.08, synergy 1.56, connected −0.12, depth −0.29]` — the model
puts almost all weight on **synergy** (non-redundant coverage), which is the intuitive reuse signal.

## Phase 3 — the un-gameability gate on real data

### (1) Predictive — held-out pairwise accuracy, 20 seeds, ANCESTOR direction

| scorer | mean acc | std |
|---|---|---|
| coin-flip floor | 0.5000 | — |
| best single feature | 0.5126 | 0.0116 |
| proxy f0 (breadth = `proxy_value`) | 0.5167 | 0.0135 |
| **LEARNED Bradley-Terry (4 feats)** | **0.5201** | 0.0103 |

| delta | mean | win-rate |
|---|---|---|
| learned − proxy f0 | **+0.0034** | 12/20 |
| learned − best single | +0.0075 | 14/20 |

1-SE noise band (1199-pair split): **±0.0144**.

**Verdict: NULL — a third time, now decisively.** The delta over the proxy (+0.0034) is well inside the
±0.0144 noise band; 12/20 vs the proxy is a coin-flip. Crucially this null is **no longer explainable by
degeneracy** — the ancestor object is fully non-degenerate here (0/3000 singletons, median 127). So the
null is a **robust property, not a DeepFunding artifact**: on real reuse labels at scale, the four
structural graph features explain only a sliver (~0.52 vs 0.50 floor) of downstream reuse, and a learned
scoring of them does not reliably beat the fixed breadth proxy. This **strengthens** the settled reframe:
the moat does not rest on the predictive win.

### (2) Adversarial — the correctly-specified moat instrument

Construct a *genuine* high-value unit (high breadth **and** healthy synergy/lineage) and a *gamed* unit
(SAME inflated breadth, but padding kills synergy → ~0, no lineage), from real-feature percentiles:

```
proxy_value(genuine) − proxy_value(gamed) = +0.0000   (proxy CANNOT tell them apart)
learned(genuine)     − learned(gamed)     = +0.0500   (learned DENIES the gamed unit)
```

**Verdict: PASS (direction).** The fixed proxy conflates the gamed and genuine units (identical
breadth); the learned measure separates them (genuine scores strictly higher). This is the moat's actual
property — resist proxy-gaming — reproduced with the real crates feature distribution. **Honest scope:**
these are constructed feature vectors at real percentiles, the same *class* as the shipped constructed
adversarial fixture (`gamed_coalition_pays_the_proxy_but_the_learned_measure_denies_it`, 2026-07-02) —
NOT a real attack crate injected into the live graph. It demonstrates the resistance property; it does
not yet measure it against an adaptive real-world adversary.

### (3) Isomorphism-invariance — the gate that caught a real bug

Relabel every identity (bijection) + permute the coalition; a valid `v(S)` must not change.

```
base   : [4.654, 0.3032, 0.7404, 0.2212]
permute: [4.654, 0.3032, 0.7404, 0.2212]
relabel: [4.654, 0.3032, 0.7404, 0.2212]
Verdict: INVARIANT
```

**This gate first FAILED** — `depth` changed under permutation (0.144 → 0.212). Root cause, in
descending severity:
1. `faithful_port.coalition_features`'s longest-path uses an **order-dependent memo** its own comment
   concedes is only "recompute-safe" for *tiny* sets. DeepFunding descendant sets were tiny; crates
   ancestor coalitions are large (median 127) ⇒ the memo produced order-dependent `depth`.
2. Deeper cause: the crate-level provenance graph **contains cycles** (collapsing deps across versions
   makes `A→B→A` possible), so "longest chain" is *ill-defined* without a canonical cycle rule.

**Fix (`_longest_chain_condensation`):** collapse strongly-connected components (iterative Tarjan) and
take the longest path in the resulting DAG, each component weighted by member count — deterministic in
`S` alone. After the fix all four features are permutation- **and** relabel-invariant.

**Load-bearing design note:** this is a real determinism constraint (B1) for any multi-parent `v(S)`.
Noesis's actual Cell model is a **single-parent DAG** (`cell.parent: Option`, acyclic by construction),
so consensus is not exposed to this today; but the moment a value feature reads a graph that can cycle,
it MUST use the canonical SCC-condensation rule or it forks replicas. The iso-invariance gate is what
surfaced it — exactly its job.

**Honest scope:** this demonstrates the four shipped coalition features are permutation + identity-
relabel invariant (after the depth fix). It is NOT the general graph-isomorphism-invariance theorem
CLAUDE.md marks open (§6.1 loop invariant + §6.2 probe-diversity, MVP-SCOPE) — that remains open.

## Bottom line (what changed, honestly)

- MOAT-1's **deep-ancestry dataset obligation is DISCHARGED** (crates.io, non-degenerate: 0.18 vs 0.83).
- The **predictive half is NULL a third time**, now decisively (topology no longer a confound) ⇒ the
  reframe is confirmed: the learned predictive win is upside, not the moat.
- The **adversarial property reproduces** (directional) on real feature distributions.
- The **iso-invariance gate passed after fixing a real order-dependence bug** in the depth feature +
  produced a canonical cycle rule for multi-parent `v(S)`.
- The moat continues to rest on the **structural layered defense** (demonstrated, 253/253), unchanged.

## Files
- `build_dataset.py` — crates.io dump → provenance DAG + reuse labels + anti-degeneracy stats
- `moat_test.py` — faithful ancestor-direction port + adversarial + iso-invariance, corrected depth
- `graph/stats.json`, `graph/moat_results.json` — machine-readable receipts (committed)
- `db-dump.tar.gz`, `graph/*.tsv` — gitignored (1.5 GB source + 44 MB derived; rebuild from the dump)
