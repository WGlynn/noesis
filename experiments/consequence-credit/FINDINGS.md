# Consequence-Credit — findings (project: attribution-by-consequence, Fork B)

Started + first two experiments 2026-09-29. Status: **naive version FALSIFIED on available data.**

## Question
Does a consequence-based credit score over a dependency graph reproduce human worth judgments —
the premise behind "attribution by consequence, not declaration"?

## Data (local, real)
- graph: `data/deepfunding/dependency-graph/datasets/v2-graph/dependency-graph-v2.csv` (1770 nodes, 4222 edges, no timestamps)
- labels: `data/deepfunding/mini-contest/dataset.csv` (2387 human pairwise weights)
- baseline: `data/deepfunding/dependency-graph/datasets/oso/repo_and_funding_stats.csv` (star_count)

## Results
exp01 (head-to-head, predict human pairwise weight):
- consequence-credit (weighted PageRank): acc 0.489, Spearman 0.053, coverage 42.6%
- star_count baseline: acc 0.622, Spearman 0.261, coverage 82.3%  -> **dumb baseline WON**

exp02 (divergence — the fair test, popularity-tied slice + augmentation delta):
- all evaluable (n=826): stars 0.672, credit 0.475
- stars-tied slice: tau=0.25 credit 0.534 (n=103, within noise), tau=0.5 0.510, tau=1.0 0.492
- augmentation delta (stars vs stars+credit tie-break): **-0.011 (credit hurts)**

## Verdict
Weighted PageRank consequence-credit carries essentially NO signal about human worth on this
data. Best showing (0.534, n=103) is within noise. Below baseline => bias, by our own rule.

## Why (the load-bearing interpretation)
Third instance this session of the exogeneity wall (Noesis oracle-free limit; Hodge
circulation != fraud; now credit != worth). Worth is EXOGENOUS to dependency-graph topology.
Also: the human label is popularity-contaminated (stars predict it at rho=0.26), so head-to-head
on it is the wrong target — consequence-attribution is meant to win where popularity is WRONG.

## What is NOT yet falsified (both DATA-BLOCKED, not modeling gaps)
1. TEMPORAL: train graph@T, predict who gains funding/dependents after T. Popularity-today
   can't peek at the future; early-consequence might lead. Needs TIMESTAMPED edges the v2 graph
   lacks. Would require reconstructing a time-indexed dependency graph (git history / GitHub API)
   or using gitcoin round dates as time-separated outcomes.
2. WASH-REJECTION: does credit reject inflated-but-unused contributions stars would reward?
   Needs LABELED wash, absent here (wash_sim.rs is synthetic-only).

Also untested: the actual Noesis wash-resistant credit (submodular Shapley / identity-quotient
flow) rather than generic PageRank — but that needs per-node content/coverage the bare graph
lacks, so it is also data-blocked.

## Recommendation
Do NOT method-tweak to chase the contaminated label. The tractable-now version is dead. The
surviving hypotheses require acquiring temporal / wash-labeled data — a data-acquisition
commitment, not an afternoon of modeling. Shelve unless that commitment is made.
