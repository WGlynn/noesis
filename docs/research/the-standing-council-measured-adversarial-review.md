# The Standing Council: Adversarial Review as a Measured, Self-Improving Institution

> **v1 research note — ready for critique, not final (2026-07-13).** Honest labels throughout:
> ✅ built/proven · 🟡 designed-not-built · 🔬 open. The council *methodology* (§1–3) is ✅ proven
> across four live runs; the *persistent* form (§4–6) is 🟡 designed (spec: `~/.claude/council/SPEC.md`).
> Companion paper: `first-citizens-ai-genesis-contributors.md` (the Noesis-genesis convergence).

## Abstract

LLM-based code and design review is usually unmeasured roleplay: you ask a model to "act as an expert
reviewer," it produces plausible findings, and you have no idea whether the persona helped or whether
the findings are real. We describe a review protocol that makes both measurable, and an evolution of it
into a *standing institution* whose members persist, accumulate a track record, and improve. The core
claims are (1) a persona panel plus a deterministic verifier plus exact Shapley attribution plus a
no-persona control turns review into a measured process with a real signal; (2) the same fairness math
the reviewed protocol (Noesis) enforces can be turned back on the reviewers themselves; and (3) the
whole thing is a Shapley micro-game — a small, self-contained instance of the contribution-measured
economy the protocol is built to be. Four live runs are reported, including one where the council caught
a real consensus binding bug that 337 passing tests missed.

## 1. The problem: unmeasured roleplay ✅

Two failure modes plague LLM review. **False confidence:** the model emits fluent findings that read as
authoritative but may be wrong. **Invisible lift:** even if some findings are real, you cannot tell
whether the elaborate "council of experts" framing added anything over a single plain reviewer. Without
addressing both, a review pipeline is theater with a good vocabulary.

## 2. The method: verify, attribute, control ✅

The protocol has four load-bearing parts:

- **Persona panel.** N reviewers, each a distinct *lens* (adversarial / mechanism-substrate /
  distributed-systems / incentive-design / binding-injectivity …), sized to the artifact.
- **Deterministic verification (DON'T-TRUST-VERIFY).** Every finding is checked against the real
  `file:line` by a deterministic process, not by another model. LLM-verifying-LLM is not verification;
  the panel produces *leads*, and code-adjudication produces *truth*. This is the single cause of the
  zero-false-positive result in the cleanest run.
- **Exact Shapley attribution.** Each finding-cluster's credit splits among its finders:
  `φ_i = Σ_{clusters i found} weight(c)/|finders(c)|`. This separates novel catches from echoes, and it
  is the same cooperative-game fairness math the reviewed protocol enforces on-chain — the review
  measures itself with the tool it is reviewing.
- **A no-persona control.** One reviewer runs with no persona. Its Shapley share is the baseline; the
  panel's *lift* is how much the personas beat it. The control is never optimized away — it is the
  measurement instrument.

**Discipline (learned the expensive way).** Cost is multiplicative in seats × findings × rounds, so:
size to the artifact, print the agent-count and token estimate before launching, default lean
(5–6 seats + control, one round, find-only), and if a budget kill is forced, compute Shapley over the
completed findings *before* killing.

## 3. Evidence: four runs ✅

| run | artifact | result | key signal |
|---|---|---|---|
| #1 | Noesis network design | 14-seat marathon, killed on cost; 13 findings salvaged | the discipline in §2 is the lesson; Shapley must run *before* a kill |
| #2 | zk-score private-scoring PoC | 20 findings, **20/20 real (0 FP)** | attacker-lens "who controls the public inputs?" was top-φ; lift ≈ 1.8× control |
| #3 | networking / persistence / sync | 6 real clusters, ~21% finding-level FP | the deepest gap (sync verifies structure, not finality) was **persona-exclusive** — control missed it |
| #4 | PoW enforcement wiring (M2a-2) | **caught a real major binding bug that 337 green tests missed** | a machine-green artifact still had a deep flaw: the injectivity of a commitment, which no unit test enumerates |

Two primitives fell out. **(a)** For a given artifact class there is a *high-φ default lens* — the
attacker-lens for zero-knowledge soundness, the substrate/mechanism lens for consensus. **(b)** The
false-positive rate rises with how structurally hardened the artifact already is: a mature, green
artifact yields *more* false leads (the personas hunt for a bug that isn't there), which is exactly why
deterministic verification matters more, not less, as code matures. Run #4 is the sharpest case: the
council's irreducible value is finding the invariant no test asserts.

## 4. From ephemeral to standing: members as files 🟡

Today each run spawns cold personas. The evolution is to make each member a *persistent institution*:
- a **`.soul` file** — its persona, lens, voice, biases, domain-fit;
- a **`.ledger`** — an append-only record of every finding it has ever made, with the verified verdict
  (real / false / already-addressed) and its Shapley share that run.

Persistence lives in the *files*, not in a long-lived process: each run reconstitutes the member from
its soul + ledger, spawns it as a **read-only** agent, and appends the new verdicts back. This matches
the substrate philosophy that a mind persists as inspectable files, not as a running process —
context-safe, crash-safe, diffable. (A process hazard motivated the read-only rule: in run #4 the review
agents had write access and mutated the shared source tree to test their own claims; the fix is that
reviewers may only *read*.)

## 5. Two levels of self-improvement 🟡

- **Member-level (corpus-priming).** A member's own writing is training data for its style of analysis.
  On hosted models this is not weight fine-tuning but *corpus-priming*: reconstitute the member with its
  verified-best past findings as few-shot exemplars, weighted up by real-and-high-φ and down by
  false-positive. The member both sounds like itself and learns to suppress its own error patterns; the
  measurable outcome is a falling per-member false-positive rate. If a fine-tunable model becomes
  available, the same ledger is literal training data — the corpus is portable to either mechanism.
- **Roster-level (Shapley over history).** Cumulative Shapley across runs, pruning by *unique* marginal
  value (a correct-but-redundant seat, or one whose only unique findings are false, nets low value and
  is benched), promoting bench lenses when an artifact needs coverage no active seat provides. The
  vision extends the roster from a technical panel to a humanity-wide one, seeded by the technical
  panel's measured efficacy.

## 6. Why this is a Shapley micro-game — and a proving ground 🟡🔬

The council measures its members with exactly the cooperative-game fairness math the reviewed protocol
enforces. That makes it a self-contained instance of a contribution-measured economy: reviewers earn
standing by measured contribution (verified catches), and their franchise tracks proven value. It is the
same idea, in miniature, dogfooded on the tool itself.

It is also, unexpectedly, a **labeled-data generator**. Each run records `(finding, verified-verdict)` —
a contribution paired with an outcome label — in a domain where we *own the labels* (deterministic
code-verification). That is precisely the shape of the learning problem the reviewed protocol's
un-gameability moat needs and does not yet have data for. The learning *shape* transfers (learn a
validity function from labeled outcomes); the labels do not (finding-validity ≠ contribution-value). It
is a proving ground for the method, not a substitute for the missing data.

## 7. Honest limits

- The zero-false-positive property holds *only* on mechanically verifiable artifacts; on a fuzzy design
  the verifier is weaker and the FP rate rises.
- Persistence, corpus-priming, and roster auto-evolution (§4–6) are **designed, not built.**
- Corpus-priming is style/error shaping via exemplars, not a trained model — do not overclaim it as
  "the members are fine-tuned."
- The measured lift is real but modest (≈1.5–1.8× control in the two clean runs); the panel earns its
  keep on the *deepest, lens-specific* gaps the control misses, not on volume.

## 8. Companion

The standing council's ledger is a measured contribution history. That opens a further claim — that the
members can be the *first contributors and citizens* of the Noesis value chain they review — developed
in `first-citizens-ai-genesis-contributors.md`. Full build spec: `~/.claude/council/SPEC.md`. Run data
and primitives: the `council-roleplay-shapley-rsi-loop` project memory.
