# CRPC as an optional second meta-consensus over the UTXO shard graph — SKETCH

> Status: 🔬 **research direction, not built, not specified.** A note captured 2026-07-12 (Will) so
> the idea is not lost. Honest label: this is a *where-could-it-fit* sketch, not a mechanism. Do not
> cite it as a Noesis capability.

## The observation

Noesis settles **objective** truth: the per-execution UTXO invariants (value conservation,
no-double-spend, no-spend-of-nonexistent) — the rules a receipt proves *followed* (Phases 0–3) and a
formal spec proves *right* (Phase 4, `docs/phase4-fv-plan.md`). Those are hard, binary, and local to a
transaction.

But a network of nodes/agents also needs to coordinate on **fuzzy, semi-true** things: reputation, the
quality of a contribution, whether a claim is *probably* right, which of several subjective framings a
community endorses. That is a *different* kind of agreement — graded, not binary — and forcing it
through the hard-invariant path would corrupt the thing that makes the hard path trustworthy.

## The idea

Treat the **UTXO set as a natural sharding network** (each cell / provenance sub-graph is an
independently-verifiable shard) and layer an **optional, opt-in second meta-consensus** on top of it,
whose job is coordinating on the *fuzzy* claims — separate from, and never overriding, the hard
rulebook.

Candidate mechanism: **CRPC** (the pairwise-scoring consensus from the PsiNet / VibeSwap lineage —
same commit-reveal + pairwise-comparison DNA as VibeSwap's `CommitRevealAuction`). Pairwise
comparison is exactly the right shape for *graded* agreement: agents score relative claims, and the
aggregate settles a soft ordering without pretending it is a hard invariant.

Sibling framing: this is the **third axis** of Noesis verification, alongside
1. per-execution UTXO invariants (Isabelle/proptest — Phase 4), and
2. governance-mutation coherence (Pragma Confluence — the amendment surface).

Axis 3 = **soft-claim coordination** (CRPC). All three are *separate axes*; the discipline is to keep
them separate — the hard invariants must not depend on the soft layer, and the soft layer is opt-in.

## Open questions (what "find where it fits" means)

- **The claim object.** What, concretely, is a "fuzzy claim" as a cell/shard? A soulbound assertion?
  An attestation cell whose `data` is a graded score rather than a token amount?
- **Which shards.** Does CRPC run per-provenance-subgraph (natural sharding) or over a sampled global
  panel? UTXO locality suggests per-subgraph, aggregated.
- **Non-interference proof.** The hard rulebook (I1–I4) must be provably independent of the CRPC layer
  — a compromised soft layer can degrade coordination but must never mint value or finalize a bad
  cell. This is the load-bearing safety property before any build.
- **Relationship to PoM.** PoM already scores contribution value; is CRPC a generalization of that
  scoring to arbitrary fuzzy claims, or a distinct layer? Likely: PoM is the *value* instance; CRPC is
  the *general graded-agreement* engine.

## Downstream

The same opt-in graded-agreement layer is a candidate substrate for a **JARVIS / agentic mesh
network** — many agents coordinating on soft truth (reputation, task quality, which answer to trust)
without a capturable center. Captured separately in JARVIS memory; noted here only as the direction
this axis points.

---
*Reciprocity note: if any primitive built for this axis is useful to CKB/Nervos, offer it upstream —
we owe them the Cell/RISC-V/SMT foundation.*
