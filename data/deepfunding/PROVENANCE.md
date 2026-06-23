# Provenance of ingested data — DeepFunding (the dogfood record)

> Noesis measures and attributes contribution. A contribution-attribution chain that does not attribute
> its OWN inputs is self-refuting. So this file gives the DeepFunding data what Noesis exists to give
> every contribution: **provenance.** It is both MIT-license compliance and a live demonstration of the
> commit-reveal provenance mechanism applied reflexively — Noesis's first real-data ingestion, recorded
> by Noesis's own rule. Principle captured in `internal/thesis/DESIGN-reflexive-provenance.md`.

## Source (attributed contributors)
- **Mechanism + program:** Deep Funding, designed by **Vitalik Buterin**, run via **Gitcoin** /
  deepfunding.org (initial $250k round on the Ethereum dependency graph).
- **Repositories ingested (public, MIT):**
  - `github.com/deepfunding/mini-contest` — jury pairwise comparisons (`dataset.csv`).
  - `github.com/deepfunding/dependency-graph` — the depth-2 Ethereum dependency graph.
- **License:** MIT, Copyright (c) 2024 deepfunding. Notice preserved per MIT terms; used unmodified as
  input, no claim of authorship over the source data.
- **Human jurors:** the distilled-judgment labels were produced by the Deep Funding jury
  (deepfundingjury.com). The jurors are attributed contributors to any model Noesis trains on these labels.
- **The 117 OSS repositories** compared in the jury set are themselves the contributions being valued;
  in a live Noesis chain each would hold standing for the value the graph attributes to it.

## Commit-reveal provenance record (the ingestion, recorded by our own rule)
Ingested 2026-06-23 into `C:/Users/Will/noesis/data/deepfunding/` (gitignored — reproducible by clone).

| artifact | sha256 (root) | size |
|---|---|---|
| `mini-contest/dataset.csv` (jury pairwise labels) | `d25fc30ed3ab5a59f08acf250cabe731…` | 2388 rows |
| `dependency-graph/graph/unweighted_graph.json` | `19e3f884485b4707b18fa596ef567d16…` | 109,667 lines (~109k edges) |

These hashes are the commit roots of the ingestion: the exact bytes Noesis trained on are pinned, so the
provenance of the moat experiment is verifiable, not asserted (the same timestamp-priority discipline the
chain uses for any contribution). Distinct OSS repos referenced in the comparisons: **117.**

## What "giving them provenance" means here (and at scale)
1. **Now (this experiment):** the source, the jurors, the repos, and the licence are recorded as the
   attributed lineage of any learned-`v(S)` weights trained on this data. The experiment's outputs cite
   this record.
2. **In a live chain:** external data ingested by Noesis is committed (hash + source) before use, and the
   data providers are first-class attributed contributors — they earn standing for the value their
   contribution adds downstream, measured by the very mechanism their data helped train. The system
   attributes its own inputs. That reflexive consistency is the point: it is what a contribution-honest
   chain owes the contributions that built it.
