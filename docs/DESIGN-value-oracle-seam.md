# The v(S) Value-Oracle Seam — the blockchain ↔ AI boundary

> Status: the seam is **built** (`node/src/lib.rs`, tested in `node/tests/value_oracle_seam.rs`). The
> v0 oracle is a **designed** heuristic. The learned v(S) it makes room for is **open** and data-gated.
> Never round those up.

## Why this exists

Noesis is an AI/blockchain system: contribution flows through a value function `v(S)` whose per-cell
scores become **Proof-of-Mind standing**, which weights finality. The temptation is to weld the AI
(how value is scored) into consensus. This seam refuses that. It isolates the one thing consensus
consumes from the value layer — *a deterministic integer value per finalized cell* — behind a single
interface, so the value function can be replaced without touching consensus.

This is the concrete resolution of "get it right vs get it done": ship the honest heuristic **now**;
drop the learned model in **later**, behind the same seam, with no rebuild.

## The seam in code

| Role | Item | Location |
|------|------|----------|
| Interface | `trait ValueOracle { fn cell_values(&self, cells, theta_sim_q16) -> Vec<u64> }` | `node/src/lib.rs` |
| v0 impl (BUILT) | `struct NoveltyOracleV0` — temporal novelty + Q16.16 similarity floor | `node/src/lib.rs` |
| Aggregator | `pom_scores_with_oracle<O: ValueOracle>(oracle, cells, theta) -> HashMap<contributor, u64>` | `node/src/lib.rs` |
| Consensus entry | `pom_scores_with_similarity_floor_q16(cells, theta)` — delegates to `NoveltyOracleV0` | `node/src/lib.rs` |
| Consumer | `Node::finality_pom_weight()` — cleared cells → scores → finality weight | `node/src/runtime.rs` |

Dispatch is **generic (static), not `dyn`** — zero-cost and no_std-friendly for the eventual RISC-V /
CKB-VM port. The aggregation (per-cell value → per-contributor standing, keyed on
`cell.type_script.args`) is oracle-agnostic: swap the oracle, keep the aggregator.

## The contract a replacement MUST preserve

A different `ValueOracle` may change *what* work is worth, but it may not break consensus determinism.
`cell_values` must be:

1. **Pure + deterministic** — same `(cells, theta_sim_q16)` yields **bit-identical** output on every
   replica. No floats on the consensus path, no wall-clock, no map-iteration-order dependence.
2. **Shape-preserving** — exactly one `u64` per input cell, in the **same commit order** as the input
   (`out.len() == cells.len()`).
3. **Attribution-neutral** — value is attributed to the contributor by the aggregator, not the oracle;
   the oracle scores *cells*, not identities.

A replacement that violates (1) forks the chain; (2)/(3) corrupt attribution. These are load-bearing.

## Honest status of the two oracles

- **v0 `NoveltyOracleV0` — BUILT, designed, not learned.** Rewards first-appearance coverage; zeroes
  near-duplicates via the deterministic Q16.16 floor. It models *novelty*, which is a proxy for value,
  not value itself. It is honest exactly because it does not claim to be the learned model.
- **Learned v(S) — OPEN, data-gated (the moat).** `learned-v(S)-on-real-labels` + the
  isomorphism-invariance gate remain unbuilt because they need real value labels we do not yet have.
  Building harder does not close this; **data** does. The seam ensures that when the data exists, the
  upgrade is a component swap, not a re-architecture.

## How the swap happens (governance-gated, not a runtime plugin)

The network must agree on **one** canonical `v(S)` — you cannot let nodes run different value functions
and still converge. So the swap is **not** a per-node plugin; it is a **protocol version bump**:

1. Implement the new scorer as a `ValueOracle`.
2. Make it the canonical oracle behind `pom_scores_with_similarity_floor_q16` under a new
   value-function **version carried by the `Constitution`'s measurement-amendment frame** (the same
   frame that already carries `theta_sim_q16`).
3. **Activate via a constitutional amendment**, checked by the amendment coherence gate
   (`node/src/amendment.rs`) — the amendment surface is precisely where a rule-set mutation like
   "change the value function" is verified for coherence before it can take effect.

Until an amendment activates a successor, **v0 stays canonical**.

## Anti-theater proof

`node/tests/value_oracle_seam.rs`:

- **Parity** — the v0 oracle path is byte-identical to the pre-seam floor entry ⇒ introducing the seam
  changed no consensus behaviour.
- **Swappability** — a trivial `ConstantOracle` plugged into the aggregator produces different,
  correct, per-contributor scores ⇒ the swap is real, not decorative.

If either test is deleted the property it guards is unproven; do not remove them without replacement.
