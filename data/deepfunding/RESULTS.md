# Phase-1 Moat Experiment — Learned v(S) vs Fixed Structural Proxy on REAL DeepFunding Labels

**Question.** Does a LEARNED value measure (Bradley-Terry over set-level structural
features — the Rust `outcome` module) beat a FIXED structural proxy (the single best
feature) at predicting human-jury contribution preferences? This is the core
"un-gameable learned v(S)" moat claim. We test it on REAL DeepFunding mini-contest
jury labels, not synthetic fixtures.

**Headline (single seeded 80/20 split, seed=1234):**

| scorer | held-out pairwise accuracy |
|---|---|
| coin-flip floor | 0.5000 |
| proxy `breadth` (+) | 0.5280 |
| proxy `synergy` (+) | 0.5344 |
| proxy `connected` (+) | 0.5387 |
| proxy `depth` (−) | 0.5065 |
| **BEST single-feature proxy** (`connected`) | **0.5387** |
| **LEARNED Bradley-Terry (4 feats)** | **0.5419** (train 0.5656) |
| delta (learned − best proxy) | **+0.0032** |
| delta (learned − coin-flip) | +0.0419 |

Learned weights: `[breadth 0.020, synergy 1.280, connected 0.267, depth −0.696]`.

**Robustness (20 seeds, same 80/20):**

| metric | value |
|---|---|
| learned mean accuracy | 0.5616 (std 0.0176) |
| best-proxy mean accuracy | 0.5595 (std 0.0187) |
| mean delta (learned − best proxy) | **+0.0021** (std 0.0049, range −0.0054 … +0.0129) |
| seeds where learned strictly beats proxy | **11 / 20** |
| 1-SE sampling-noise band on a 465-pair accuracy estimate | ±0.0232 |

## Honest verdict

**This is a NULL result, not a moat win.**

- Yes, on the single headline seed the learned model edges the proxy by +0.0032 — but
  that "+0.0032 ⇒ LEARNED BEATS PROXY" verdict the script prints is a sampling-noise
  artifact. The mean delta over 20 seeds is **+0.0021**, with a std (0.0049) of the same
  order, and the learned model wins in only **11/20 seeds** — indistinguishable from a
  coin flip. The per-split sampling-noise band (±0.023 at 1 SE on 465 pairs) is roughly
  **10× larger** than the delta. There is no evidence here that the learned v(S) extracts
  signal the best fixed structural feature cannot.
- Both scorers also barely clear the coin-flip floor (~0.54–0.56). These four graph-derived
  features — whether learned-weighted or used singly — explain only a sliver of jury
  preference. The dominant signal in DeepFunding jury labels is **not** captured by these
  structural proxies at all.
- The honest reading: on REAL jury labels with THESE proxy features, **a learned model does
  not beat a fixed structural proxy.** The moat claim is not supported by this experiment.

## What WAS demonstrated (the real, smaller win)

The `load_prefs` **seam consumed real DeepFunding data end-to-end.** `build_labels.py`
emitted `outcome_labels_deepfunding.txt` in the exact on-disk contract
(`node/src/fixtures/outcome_labels_demo.txt` shape: 4 floats/row, file-order indexed,
`pref <winner> <loser>` lines), and `train_eval.py`'s parser — a faithful mirror of the
Rust `outcome::load_prefs` — read it, trained the same Bradley-Terry estimator, and scored
held-out pairs via the same `pairwise_accuracy` rule. So the **data pathway** is validated:
real jury labels flow through the contract unchanged. What is NOT validated is that the
learned model on top of that pathway clears the proxy.

## Feature definitions and justification

The Rust features f0..f3 are SET-level over a coalition's *provenance* DAG. Here we
approximate their single-repo analogues over the *dependency* graph. **These are honest
proxies, not the same quantity** — see caveats.

- **f0 breadth = ln(1 + in_degree)** — Rust f0 = ln(1+|union coverage|): how much ground
  the work covers. Proxy: in-degree = # repos that DEPEND ON this repo (its dependents);
  a widely-depended-on repo covers more ecosystem surface.
- **f1 synergy = distinct_neighbors / (1 + in_deg + out_deg)** — Rust f1 = |union|/Σ|indiv|
  ∈ [0,1]: non-redundancy. Proxy: distinct-neighbor fraction of total degree.
- **f2 connected = (in_deg + out_deg) / max_degree_in_joined_set** — Rust f2 = fraction of
  S whose parent is in S: work-built-on-work vs orphan. Proxy: normalized total
  connectivity; an isolated node (degree 0) is the orphan analogue.
- **f3 depth = node level / 2** — Rust f3 = longest in-S parent chain / |S|: real lineage
  depth. Proxy: the graph's own `level` (BFS depth from Ethereum), the most direct
  lineage-depth signal available; level 1 = direct dep, level 2 = transitive.

None needs an oracle; all derive from the shipped graph. The labels carry the outside signal.

## Join coverage (honest)

- dataset comparison rows: **2387**
- distinct repos in dataset: 117 | distinct repos in graph: 5024
- **repos in BOTH (survived join): 115 / 117**
- dataset repos NOT in graph: 2 — `vercel/swr`, `web3/web3.js`
- **comparison rows kept (prefs): 2325 / 2387** ; dropped (≥1 missing repo): 62
- total project refs 4774, of which 62 reference a missing repo.

Coverage is high (97% of repos, 97% of pairs). The null result is not a coverage artifact.

## Caveats

1. **The proxies are approximations of the Rust DAG semantics, not the same quantity.** The
   Rust features are SET-level over a provenance DAG; ours are single-repo over a dependency
   graph. A more faithful port (coalition-level features over a true provenance DAG) could
   move the numbers — in either direction.
2. **The dependency graph ≠ a provenance DAG.** "A depends on B" is not "B is an ancestor of
   A's contribution." f2/f3 in particular lean on this mismatch.
3. **Sample size.** 465 held-out pairs ⇒ ±0.023 noise at 1 SE; the observed delta lives
   well inside that band, which is exactly why we ran 20 seeds and report 11/20.
4. The single-seed script prints "LEARNED BEATS PROXY" because +0.0032 > 0; that label is
   technically true for that seed and substantively meaningless. The 20-seed table is the
   real answer.

## One-line verdict

On real DeepFunding jury labels, the learned v(S) does **not** reliably beat the best fixed
structural proxy (mean Δ +0.0021, wins 11/20 seeds, both ~0.56 vs 0.50 floor) — a NULL
result for the Phase-1 moat claim under these proxy features; the only thing validated is
that the `load_prefs` seam consumes real DeepFunding data end-to-end.

## Files written

- `build_labels.py` — ETL: graph + comparisons → `outcome_labels_deepfunding.txt`
- `train_eval.py` — numpy Bradley-Terry replica + proxy baselines + held-out accuracy
- `outcome_labels_deepfunding.txt` — 115 feature rows + 2325 `pref` lines (load_prefs format)
- `RESULTS.md` — this file
