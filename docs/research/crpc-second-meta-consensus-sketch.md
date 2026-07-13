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

Candidate mechanism: **CRPC — Commit-Reveal Pairwise Comparison, the protocol specified by Tim
Cotten.** (His original write-up has since been deleted / 404; it survives only because it was
archived offline via Wayback — a live example of the survivability discipline this repo cares about,
and the reason the credit is not lost.) VibeSwap implements the same commit-reveal +
pairwise-comparison DNA in its `CommitRevealAuction`. Pairwise comparison is exactly the right shape
for *graded* agreement: agents score relative claims, and the aggregate settles a soft ordering
without pretending it is a hard invariant.

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

## Where the AI plugs in — SLMs as CRPC-disciplined node-oracles (2026-07-13)

The question this axis quietly answers: *if Noesis is an AI/blockchain system, where does the model
live — on-chain, or just in the nodes?*

**The determinism wall settles it.** Consensus needs bit-identical replication; neural / SLM inference
is non-deterministic across hardware (float order, GPU, library versions). So a learned model **cannot
run inside the hard UTXO state transition** — the same constraint the value-oracle seam names
explicitly: `ValueOracle` (`node/src/lib.rs`) requires *pure, deterministic, integer,
replica-identical* output. The v0 fixed-point novelty scorer (`NoveltyOracleV0`) passes that contract
and is genuinely on-chain; a neural v(S) fails it. That failure is not a bug to fix — it is the reason
this axis exists.

**So SLMs are neither literally on-chain nor merely "nodes" — they are node-level oracles whose
*fuzzy* outputs reach consensus through CRPC, not through re-execution.** A node runs its SLM off-chain
and emits **pairwise judgments** ("is contribution A more valuable than B?"). CRPC (commit-reveal +
pairwise + aggregate over the natural provenance shards) settles the soft ordering un-gameably. The
chain never runs the model; it disciplines the model's *claims*.

**The chain is the harness for the SLM-nodes.** Commit-reveal defeats copying, stake/slash defeats
dishonesty, and pairwise-relative turns the intractable absolute ("is this *true*?") into the tractable
relative ("is A better than B?"). That is the same fortress-around-the-model pattern that lets a small
model match a frontier one when the harness carries the difficulty (empirically: arXiv 2607.08938,
*Better Harnesses, Smaller Models*), lifted from a single agent up to the protocol layer. The AI is
fallible and non-deterministic; the structure around it makes its output trustworthy.

**Two ways to make a node-oracle's output trustworthy, both already latent in Noesis:**
- *Optimistic* — commit-reveal + stake + dispute/slash. CRPC's native mode, and it reuses the
  accountable-safety machinery (the equivocation guard / slash-before-count already wired onto the
  finality path, `node/src/runtime.rs`).
- *Trustless (endgame)* — **zkML**: prove that a committed model (a weights hash) produced score `X` on
  input `Y`; the chain verifies the proof without re-running the model. Same machinery as the Phase-3
  zkVM work (RISC Zero, `onchain/zk-*`). This yields a *deterministically verifiable* output from a
  model that is *non-deterministic to run* — the clean resolution.

**This resolves the "Relationship to PoM" open question above.** PoM's v(S) is the **value instance** of
the graded-agreement engine; CRPC is the **general** graded-agreement engine over arbitrary fuzzy
claims. The value-oracle seam (`node/src/lib.rs`) is the exact plug point where a CRPC-settled or
zkML-attested score enters the hard layer — **but only as an integer that satisfies the deterministic
oracle contract.** That is non-interference made concrete: the soft layer feeds the hard layer through
a deterministic, disputable *value*, never by executing inside it.

Honest status: the seam, the fixed-point v0, and the equivocation-slashing are **built**; CRPC (Tim
Cotten's spec), the learned v(S), and zkML are **open** (data- / env- / spec-gated). Non-interference
remains the load-bearing safety property that must be proved before any of the soft layer is built.

## Downstream

The same opt-in graded-agreement layer is a candidate substrate for a **JARVIS / agentic mesh
network** — many agents coordinating on soft truth (reputation, task quality, which answer to trust)
without a capturable center. Captured separately in JARVIS memory; noted here only as the direction
this axis points.

---
*Reciprocity note: if any primitive built for this axis is useful to CKB/Nervos, offer it upstream —
we owe them the Cell/RISC-V/SMT foundation.*
