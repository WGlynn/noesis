# 2026-07-15 — the learned-`v(S)` moat, tested on real deep-ancestry data (crates.io)

**Plain-English recap** (per the "human-legible session summaries" rule — you said you miss the
day-to-day progress). This was the research-heavy window you teed up: attack the un-gameable-value moat
using real merged/published contributions instead of a toy dataset.

## What we were trying to settle

Everything Noesis claims eventually rests on one function, `v(S)` — "how valuable is this contribution?"
The honest open question has always been: can we *learn* that function from real-world outcomes well
enough that it beats a simple fixed formula, and can't be gamed? Twice before, on the "DeepFunding"
dataset, the learned version came out NULL (no better than the simple formula). The teed-up play was to
find out whether that null was real or just a bad dataset.

## The reconciliation that reframed the whole thing (before building anything)

Reading our own status ledger first paid off. The earlier null had a *diagnosed cause*: the DeepFunding
data was **degenerate** — 95 of 115 judged projects were "leaves" with no ancestry, so the features the
model needs (how work builds on prior work) were all constant. It wasn't that the labels were bad; there
was simply no lineage to measure. The one genuinely open task was therefore very specific: **get a
dataset with real deep ancestry.**

## What we built

**The dataset (crates.io).** Rust's package registry publishes a full daily database dump. We downloaded
it (1.5 GB) and turned it into a provenance graph: each crate → the crates it depends on (its
"ancestors" / prior work), and how many other crates depend on it (real-world **reuse** = the value
label). Result: **299,775 crates, 1.94 million dependency links.** Critically, the ancestry is deep —
the median contribution has a **127-crate-deep** ancestor chain, vs DeepFunding's **1**. The degeneracy
that killed the earlier tests is gone (18% singletons vs 83%).

## What the real data actually showed (no rounding up)

1. **Predictive test: still NULL — a third time, and now it *means* something.** Even with deep, real
   ancestry and real reuse labels at scale, a learned scoring of the structural features does **not**
   beat the simple breadth formula (0.520 vs 0.517, well inside the noise band). Because the topology is
   now healthy, this null is a *robust fact*, not a data artifact. **This is good news, not bad:** it
   confirms what the ledger already argued — the moat does **not** depend on winning a prediction
   contest. It rests on the structural defenses (which are already proven, 253/253 tests).

2. **Adversarial test: PASS.** When we build a "gamed" contribution that inflates the simple formula
   (lots of raw coverage, but padding with no real synergy or lineage), the simple formula can't tell it
   from a genuine one — but the learned measure **does** deny it. That's the real property the moat is
   supposed to provide, and it reproduces on real data. (Honest caveat: still a constructed attack, not
   a live adaptive adversary.)

3. **Isomorphism test caught a real bug.** The check "renaming/reordering a contribution must not change
   its value" *failed at first* — the "depth" feature changed under reordering. Root cause: the old
   longest-chain code had an order-dependent shortcut, and the crate graph has occasional cycles (from
   collapsing versions). We fixed it properly (collapse the cycles, then measure), and now it passes.
   **The gate did exactly its job — it caught a determinism bug that would fork a live chain.** (Noesis's
   real model can't hit this — its contributions have a single parent and can't cycle — but any future
   multi-parent value feature now has the canonical fix.)

## Bottom line

- The "get a deep-ancestry dataset" obligation is **done**.
- The learned-model-beats-proxy claim is **null for real**, which *strengthens* our honest position: the
  value chain's un-gameability is the **structural defense**, not a machine-learning prediction win.
- We caught and fixed a real determinism bug via the isomorphism gate.
- Nothing new was wired into consensus and **no launch copy changed** — a model that doesn't beat the
  proxy has no business replacing the shipped one, and we stay "floor only" until a real adaptive
  adversary is beaten.

## Files
- `docs/research/learned-value-seed.md` — the full spec + the box `v(S)` must live in + results
- `data/crates/{build_dataset.py, moat_test.py, RESULTS.md}` — reproducible pipeline + receipts
- `internal/STATUS-LEDGER.md` MOAT-1 — updated with the third-null + dataset-discharged status
- also: remembered your Zenodo paper *Differential Incompleteness* (DOI 10.5281/zenodo.21150665) —
  value disputes = missing dimensions, the exact frame for why a fixed proxy is an incomplete basis.
