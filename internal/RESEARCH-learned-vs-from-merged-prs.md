# RESEARCH BRIEF — the learned‑`v(S)` moat, seeded from real merged PRs

> Teed up 2026‑07‑15 for a fresh, research‑heavy window. This is THE moat — `🔬 open, data‑gated,
> undated` — not a lean increment. Read this, then RECONCILE against the anchors below before building
> ([[verify-before-rebuild]]): `internal/MVP-SCOPE-JULY-2026.md` (R1), `internal/STATUS-LEDGER.md`
> (MOAT‑1), and the value code (`node/src/lib.rs` value/`v(S)` region). Do not re‑derive what exists.

## Why this is the play

Everything Noesis claims ultimately rests on ONE function: `v(S)`, the value of a contribution
coalition. Today `v(S)` is the DESIGNED novelty proxy (temporal novelty + θ_sim similarity floor,
`lib.rs` `pom_scores_with_similarity_floor`), honestly labelled "does not yet model value beyond
novelty" (`lib.rs:284`). The adversarial council (2026‑07‑15, captured in
`docs/DESIGN-authorityless-contribution-value.md` §5b) sharpened the honest state of the moat to three
words: **data‑gated + data‑poisonable + cold‑start‑deadlocked.**

- **data‑gated:** an un‑gameable value function must be LEARNED on real *downstream‑outcome* labels
  (what actually got used / built on / mattered), which the early network cannot yet generate.
- **data‑poisonable:** if `v(S)` trains on on‑chain citation frequency, a Sybil ring poisons the labels
  so the model learns to score junk highly.
- **cold‑start‑deadlocked:** no downstream data early ⇒ fall back to novelty ⇒ junk scores > 0 ⇒ repels
  real users ⇒ no labels ⇒ the moat never forms.

**The unlock (Will 2026‑07‑15, the GitHub‑authority insight):** a merged PR is a maintainer's
*authoritative value label* — someone with authority judged the contribution worth accepting. Noesis
can't mint that authority on‑chain, but it can BORROW it OFF‑chain as a **training seed**: learn `v(S)`
from a corpus of real merged PRs (+ their downstream dependency/reuse graph) where value is observable,
then use the network's own honest data only for later fine‑tuning. This breaks all three problems at
once: it supplies labels (cold‑start), the labels are external/uncorrelated with any on‑chain Sybil ring
(poison‑resistant), and it is genuine outcome value, not novelty (data‑gate). Critically it stays a
*seed*, never consensus authority — the whole network agrees on ONE canonical learned `v(S)` behind the
existing swappable seam.

## The exact integration seam (already built — do NOT rebuild)

`node/src/lib.rs` ≈ 256‑298: the **`ValueOracle` trait / `v(S)` seam** — "the blockchain ↔ AI boundary,"
value function swappable WITHOUT touching consensus, `pom_scores_with_oracle`. The current impl is the
novelty proxy; the learned model is "the SAME trait, swapped implementation" (`lib.rs:266`). The whole
network must agree on ONE canonical `v(S)` (not a per‑node plugin). So the research output is: a
deterministic, portable, canonically‑versioned value function that drops into this trait.

## The research program (phases)

**Phase 0 — reconcile + frame (do first).** Read MVP‑SCOPE R1 + STATUS‑LEDGER MOAT‑1 + the value code +
`value_v5..v8`/`synergy`/Myerson‑Shapley (`lib.rs:3271‑3428`). Write down: what `v(S)` must satisfy
structurally (submodular; Myerson‑restricted i.e. value only from provenance‑connected coalitions;
anonymity‑RELAXED per the Sybil‑resistance design — NOT symmetric; deterministic + portable so every
replica agrees). These structural constraints are the BOX the learned model must live inside.

**Phase 1 — the dataset (the data‑gate).** Build a corpus of real contributions with observable
downstream value: merged PRs from public repos + their dependency/reuse/citation graph. Each datum =
(contribution content + its provenance context) → (downstream‑outcome value signal: merged? depended‑on?
reused? survived?). This is the "deep‑ancestry outcome‑labelled dataset" R1 has needed. Sources to weigh:
GitHub PR/commit graphs, package dependency graphs (npm/crates/PyPI), citation graphs. Honest hazard:
merge ≠ value (lazy/biased maintainers) — treat merge as a NOISY positive label, weight by downstream
reuse.

**Phase 2 — the model.** Learn `f(contribution, context) → value` on Phase‑1 labels, CONSTRAINED to the
Phase‑0 structural box (submodular, Myerson‑compatible, isomorphism‑invariant). Likely a learned quality
model composed with the existing coverage/provenance structure, NOT a free‑form net. Must be
deterministic + serialisable to a fixed canonical artifact (a versioned weight blob the network pins),
so it is replay‑identical across nodes.

**Phase 3 — the un‑gameability gate (THE open theorem, per CLAUDE.md).** Demonstrate the learned `v(S)`
resists the adversarial vectors (Sybil rings, paraphrase‑under‑θ, varied‑junk padding, self‑citation
rings) AND is **isomorphism‑invariant** (relabelling identities / permuting a coalition does not change
value — the property that stops a ring from minting value by renaming itself). This is the "learned‑v(S)
+ isomorphism‑invariance gate" that CLAUDE.md marks OPEN; the un‑gameability claim currently holds only
for *demonstrated* vectors, and this phase is what would extend it to *learned* value on *real* labels.

**Phase 4 — integration + honest labelling.** Drop the canonical artifact into the `ValueOracle` seam;
governance‑pin the version; keep the novelty proxy as the pre‑seed fallback. Launch copy NEVER claims the
moat until Phase 3 passes on real data. ✅ built · 🟡 designed · 🔬 open — never round up.

## First concrete step for the fresh window

Phase 0 + the start of Phase 1: (a) reconcile the structural constraints from the value code into a
one‑page "the box `v(S)` must live in," (b) scope a minimal merged‑PR dataset (one ecosystem, e.g.
crates.io + its dependency graph, since Noesis is Rust and the reuse signal is clean), (c) write a
`docs/research/learned-value-seed.md` spec turning this brief into a build plan with the dataset schema.
Compute note: real model training may want the same free‑CI / WSL2 Linux path we used for the zk receipt.

## Standing constraints (do not violate)

- `v(S)` is ONE canonical, deterministic, network‑agreed function — never a per‑node oracle (that
  re‑introduces the authority we removed; cf. the ingress screen, which is advisory precisely because it
  is node‑local).
- The seed is OFF‑chain training data, never on‑chain authority. Merged‑PR labels bootstrap the model;
  they do not gate live submissions.
- JUL is never minted for no work; VIBE (governance) is the validation/liveness reward. Orthogonal to
  this, but do not let a value‑model change leak into the money layer.
