# DESIGN — the Pragma layer: Constitution-amendment coherence (the socket)

> **Status: design note (socket spec), NOT built.** The amendment rules are `pending` in code
> (`node/src/runtime.rs:37-38`, `Constitution` struct doc). This note defines the *surface* we build
> ourselves, with no partner terms; the Pragma Confluence engine plugs into it later
> (§7, terms-first — Will drives Tom Lindeman + Bernhard Mueller).
>
> **Status discipline:** ✅ built · 🟡 designed · 🔬 open. Never round up.
>
> **Anti-hallucination:** every `file:line` is a pointer to re-verify at source before relying
> (line numbers shift between builds). Do not assert a Noesis rule from this doc alone — open the file.
> The attribution properties in §5b are pinned to named `node/src/lib.rs` regressions (re-verify line
> numbers at source). The **deliberately-relaxed anonymity axiom** is load-bearing — see §5b's caution box.
>
> **Design constraint (Will 2026-06-13):** lean like Bitcoin. The socket is a typed enum + an
> obligation checklist + one gate function. It is NOT a framework, and it does NOT reimplement a
> confluence engine (§8).

---

## 1. Two lines of defence, and why this is the second one

Noesis already answers **"was the rulebook followed?"** and **"is the rulebook right?"** for a *fixed*
rulebook:

- **Line 1 — per-execution invariants (✅ machine-checked, Phase 4 complete).** `runtime::apply_block`
  preserves the value axioms I1–I5 (`docs/phase4-fv-plan.md §0`): value conservation, no double-spend
  (in- and cross-block), no spend of a nonexistent output, determinism. Discharged as property tests
  (`node/tests/fv_invariants.rs`), a spec-oracle differential, and an Isabelle/HOL proof
  (`internal/fv/Noesis_Rulebook.thy`: `conservation` + `no_double_spend` + `determinism`, 0 `sorry`).

Line 1 proves things about **one** rulebook — the one shipped. But the Constitution is *governable*:
governance can amend how value is measured. Line 1 says nothing about the **space** of rulebooks
governance can reach.

- **Line 2 — rule-set-mutation coherence (this note; 🟡 designed, socket 🔬 unbuilt).** When
  governance **amends** the Constitution, does the resulting rulebook (a) stay **confluent** (all honest
  replicas still converge to one schedule-independent state) *and* (b) **keep the axioms** (the value
  invariants I1–I5 *and* the attribution/Shapley axioms)?

Line 1 covers a point; line 2 covers the reachable set. This is the axis Pragma Confluence names.

---

## 2. Tom's danger quadrant — the cell nobody checks today

An amendment lands in one of four cells:

|                          | **Axiom-preserving**        | **Axiom-breaking**                         |
|--------------------------|-----------------------------|--------------------------------------------|
| **Confluent**            | safe (the target)           | ⚠ **THE DANGER — nobody checks this today** |
| **Non-confluent**        | replicas fork (loud; caught by consensus/replay) | forked *and* wrong (loud)      |

The bottom row is loud: a non-confluent rulebook makes replicas disagree, and the existing replay /
`state_digest` machinery catches it immediately. The dangerous cell is **Confluent + Axiom-breaking**:
the amended rulebook still converges — every replica agrees — but they agree on a state that violates an
axiom (value is silently created, or credit is routed to a non-contributor). Neither existing check sees
it:

- **Line-1 FV can't see it:** it proves a *fixed* rulebook; it is not re-run against the amended one.
- **Confluence-alone can't see it:** the amended rulebook *is* confluent — that's exactly why it's
  dangerous. Convergence is not correctness.

Closing this cell needs **both** an axiom-preservation check **and** a confluence check on the amendment
itself. That pair is the Pragma layer.

---

## 3. The amendment surface as it exists today

`Constitution` (`node/src/runtime.rs:40-79`) is governed in **three layers**
(`runtime.rs:28-38`, ROADMAP 2026-06-16 (c)):

| Layer | What it governs | Mutability | Fields (re-verify at `runtime.rs`) |
|---|---|---|---|
| **physics** | value anchors in realized downstream flow; the noise floor | near-immutable — encoded by the gate the runtime calls, not a tunable | *(not a struct field; it is the `apply_block` value gate itself)* |
| **constitutional** | the measurement **dimension set** — a dimension is admitted only if it predicts realized downstream value (verifier-gated); weights stay bounded | amendable **only through the verifier gate** | 🟡 `pending` — "a constitutional cell whose transitions obey the verifier gate" (`runtime.rs:37-38`) |
| **governance** | weights *within* the bounded set | fluid | `mix`, `threshold_bps`, `quorum_floor_bps`, `horizon`, `decay_pos`, `theta_sim_q16`, `vesting_w`, `max_mempool` |

The load-bearing gap: the **constitutional layer's dimension-matrix amendment rules do not exist in code
yet** (`pending`). Today's `Constitution` carries only the finalization frame plus the governance-layer
params. The socket below is what makes the constitutional layer real — a *typed, inspectable* amendment
op with its obligations stated alongside it.

---

## 4. What the socket is (one sentence)

**An amendment is a typed, inspectable mutation of the measurement matrix, and every amendment carries
the axiom obligations it must discharge before it can be merged.**

Three parts, each minimal: (5a) the typed amendment op, (5b) the stated axiom obligations, (5c) the gate.

---

## 5. The socket, in three parts

### 5a. The typed amendment op (inspectable, not a byte-diff)

An amendment is **not** an opaque new `Constitution`. It is a typed enum of *what can change, at which
layer* — so the check has something structured to reason about. Sketch (illustrative, not final types):

```
enum Amendment {
    // governance layer — bounded-weight moves
    AmendParam   { field: ConstitutionField, old: Value, new: Value },  // e.g. theta_sim_q16, threshold_bps
    AmendMix     { old: Mix, new: Mix },                                 // NCI / finality mix reweight

    // constitutional layer — dimension-set moves (the `pending` surface)
    AddDimension    { id: DimId, predictor: PredictorRef, weight: Weight },
    RetireDimension { id: DimId },
    ReweightDimension { id: DimId, old: Weight, new: Weight },

    // physics layer — always rejected (present only so the gate can say WHY)
    AmendPhysics { .. },
}
```

Each variant names the layer it touches. Inspectability is the point: a reviewer (human, our gate, or
Pragma's engine) reads the amendment as a mutation, not as a 500-field struct they must diff.

### 5b. The stated axiom obligations (what an amendment must preserve)

Two families. An amendment discharges the obligations in whichever family it can reach.

**Family A — value invariants I1–I5 (✅ already machine-checked for the base rulebook).**
The value path (`apply_block`) is **token-blind to the measurement matrix**: PoM attribution folds over
`cells`, value movement folds over the separate `token_cells` set (`runtime.rs:118-121`). So a
governance-layer amendment (weights, `theta_sim_q16`, `vesting_w`, mix) **structurally cannot reach the
I1–I5 path** — the obligation for those amendments is *"prove the amendment does not touch the value
gate,"* which for weight/param moves is true by construction. The obligation has teeth only for a
constitutional amendment that would alter what `apply_block` accepts as a valid token transition; that
one must re-discharge I1–I5 (re-run the Phase-4 ladder against the amended rulebook).

**Family B — attribution properties (the new surface; grounded in named regressions, not "canonical Shapley").**
The measurement matrix (dimension set + weights) *defines the PoM attribution map*, which is a **Myerson
graph-restricted Shapley value** over the provenance graph — the coalition value is summed over the
*connected components* of a coalition under provenance edges (so disconnected coalitions cannot pool
value), estimated by **seeded Data-Shapley permutation sampling** so replicas converge bit-for-bit (see
the patent §attribution and the `lib.rs` tests below). An amendment must preserve the properties that make
it a fair, un-gameable credit assignment. The danger cell (§2) lives here: an amendment that keeps the
ledger perfectly confluent while silently corrupting who-earns-what.

Properties to preserve — each **pinned to a live regression** (test names are stable; re-verify line numbers at source):

| Property | What an amendment must not break | Named test (`node/src/lib.rs`) |
|---|---|---|
| **Efficiency** | credited shares sum to the coalition value (the split creates/destroys no value) | attribution-total conservation |
| **Null / dummy** | a redundant or no-value contribution earns ~0 marginal | `redundant_cell_gets_low_shapley_marginal` |
| **Synergy (non-additivity)** | value stays a cooperative Shapley split, never a naive additive win-share | `synergy_shapley_differs_from_additive_copeland` |
| **Myerson provenance-restriction** | provenance edges change credit; disconnected coalitions cannot pool value | `myerson_restricts_value_to_provenance` |
| **Determinism / replica-convergence** | the seeded Data-Shapley estimate is bit-identical across replicas | seeded-PRNG `sampled_value` |

> ⚠ **DELIBERATELY-RELAXED axiom — do NOT list symmetry/anonymity as "to preserve."** Noesis's entire
> Sybil-resistance is that it **relaxes the anonymity axiom on purpose** (Cheng-Friedman 2005: *any*
> symmetric/anonymous reputation mechanism is Sybil-attackable). A fresh identity is worth **zero by
> construction** — JUL/PoW-anchored identity + commit-reveal timestamp priority make a false name unable
> to inherit standing (`internal/thesis/DESIGN-wills-equilibrium.md` §Cheng-Friedman). So the coherence
> obligation is the **inverse** of the textbook one: an amendment must **preserve the intentional
> anonymity-relaxation**, and the danger cell includes an amendment that quietly *re-introduces*
> symmetry/anonymity — that silently re-opens the Sybil hole while staying perfectly confluent. This is
> the single most important Family-B check, and it is why "canonical Shapley 5-axiom" is the wrong frame:
> the object is a *graph-restricted, anonymity-relaxed* Shapley value, and the amendment surface must hold
> **that** shape, not restore textbook symmetry.

("Shapley 5-axiom" is the shorthand from the Pragma Confluence discussion (Tom Lindeman's framing);
the precise object is the Myerson-restricted, anonymity-relaxed value above. See
`docs/Pragma Overlaps/noesis-pragma-overlap.md`.)

### 5c. The gate (one layered function)

```
fn verify_amendment(old: &Constitution, a: &Amendment) -> Result<(), ObligationBreach>
```

- **physics amendment** ⇒ reject with a stated reason (near-immutable by design).
- **constitutional amendment** ⇒ run the verifier gate (the dimension predicts realized downstream
  value) **and** the Family-A and Family-B obligation checks.
- **governance amendment** ⇒ run the bounded-weight check **and** Family-B (weights reshape the
  attribution map even without a new dimension).

The gate *states and, where cheap, discharges* the obligations. The expensive discharge — a full
confluence + axiom-preservation proof — is where Pragma attaches (§6).

---

## 6. The confluence attach point

Pragma **Confluence** verifies rule-set-mutation coherence: Knuth-Bendix / confluent-rewriting +
axiom-preservation (`docs/phase4-fv-plan.md:101-103`). Its design (per Tom) is a **sub-second pre-merge
CI hook**, not an offline audit — the amendment is a *proposed change to the rulebook*, and the hook runs
before it merges, the same shape as a PR gate.

The clean split, two-sided:

- **We provide (the socket):** the typed amendment op (5a) + the stated obligations (5b). This is the
  *interface contract* — the thing that says "here is the mutation, and here is exactly what it must not
  break."
- **Pragma provides (the engine):** the confluence proof (the amended rewrite system still normalizes to
  a unique form — Newman: local confluence + termination) + the axiom-preservation proof (the obligations
  in 5b survive the mutation).

Grounding for the confluence half already exists as a term-for-term bridge
(`docs/Pragma Overlaps/noesis-pragma-overlap.md`): the Noesis converged ledger = the zero set of an
inconsistency potential Φ; `apply` = a Lyapunov-descent repair map; convergence = Newman's lemma. So
"confluence of the amended rulebook" is not a foreign concept bolted on — it is the *governance-layer*
statement of the property Noesis already has at the execution layer.

---

## 7. Why socket-first, and why it stands alone

**Socket-first = no partner terms.** The socket (5a–5c) is ours, public, build-in-the-open. It is useful
*before* any Pragma integration:

1. **Self-audit.** It lets us check our *own* amendments (every `theta_sim_q16` / `MIN_DIM_BPS` /
   `vesting_w` / mix change we have shipped) against stated obligations — turning today's informal
   "is this weight change safe?" into a typed, gated question.
2. **The reference-client corpus.** Noesis amendments + the VibeSwap audit-arsenal rule-mutation history
   are a real corpus of rule-set mutations — "real client data" that makes Will the POC reference client
   for Pragma, rather than a hypothetical.

**Actual Pragma integration = terms-first** (business material; Will drives the relationship with the
Pragma Confluence team — Tom Lindeman + Bernhard Mueller, `coherence.pragmaresearch.ai`; see
`docs/phase4-fv-plan.md` §"Pragma Coherence"). The socket is the technical thing that makes that
conversation concrete: it hands them a live amendment surface with obligations already stated, and asks
their engine to discharge them.

---

## 8. Lean-like-Bitcoin scoping — what NOT to build

- **Build:** a typed `Amendment` enum, an `Obligation` checklist (Family A trivial-by-construction for
  weight moves; Family B the real content), one `verify_amendment` gate.
- **Do NOT build:** a confluence engine. Knuth-Bendix / confluent-rewriting is Pragma's product; we
  provide the surface and the obligations, not a re-implementation.
- **Do NOT conflate with line 1.** The per-execution UTXO invariants stay Isabelle/HOL theorems about a
  *fixed* `apply_block` (`docs/phase4-fv-plan.md:109-110`). Do not force Pragma onto them; do not force
  I1–I5's proof style onto the mutation surface.

---

## 9. Honest status and open grains

| Item | Status |
|---|---|
| Per-execution value axioms I1–I5 (line 1) | ✅ machine-checked (`Noesis_Rulebook.thy`, `fv_invariants.rs`) |
| Three-layer Constitution governance model | ✅ documented in code (`runtime.rs:28-38`) |
| Constitutional-layer dimension-matrix amendment rules | 🟡 `pending` — not in code yet |
| The socket (typed amendment + obligations + gate) | **governance slice ✅ built** — `node/src/amendment.rs` (typed `Amendment` + `obligations()` checklist + `verify_amendment`; 13 tests w/ RED twins, full lib suite green, 0 new clippy). Constitutional/physics layers = reject/pending. |
| Family-B attribution obligations, formalized | **✅ built** — `attribution_verdicts()` in `node/src/amendment.rs`: per-property verdict (PreservedByConstruction / AtRisk / DeferredToPragma), grounded in `runtime.rs:748` (only `theta_sim_q16` reaches `pom_scores`). A `theta_sim` RAISE = AtRisk on null-player; all else preserved-by-construction. The full preservation *proof* stays Pragma's. |
| Confluence engine integration | 🟡 terms-first, Will-driven (Tom + Bernhard) |

**Next grains, cheapest-first:**
1. **Pin the attribution properties — DONE (§5b).** Grounded in named `lib.rs` regressions and the
   deliberately-relaxed anonymity axiom (Cheng-Friedman). Family B is no longer a placeholder; the
   remaining work is a *build*: encode each property as an amendment-preservation obligation the
   `verify_amendment` gate can evaluate.
2. **Draft the minimal typed `Amendment` enum** — ✅ DONE (`node/src/amendment.rs`): governance
   `AmendParam`/`AmendMix` over the real `Constitution` fields, constitutional dimension moves
   (`Add`/`Retire`/`ReweightDimension`), physics (`AmendPhysics`, present only to say WHY it is refused).
3. **`verify_amendment` for the governance layer only** — ✅ DONE (`node/src/amendment.rs`): real safety
   bounds (2/3 `threshold_bps` floor, `theta_sim_q16` ≤ 1.0, `mix` non-negative & normalized,
   `max_mempool` ≥ 1) + stale-base rejection + Family-A trivial-by-construction; physics→immutable,
   constitutional dimension moves→`ConstitutionalPending`. `obligations()` tags each row
   `Socket` vs `Pragma`; `Ok(())` = no socket-detectable breach, NOT proven-coherent.
   **Family-B ✅ DONE** (`18b7c28`): `attribution_verdicts()` + `family_b_at_risk()` give per-property
   verdicts grounded in `runtime.rs:748` (only `theta_sim_q16` reaches `pom_scores`). Anti-theater:
   the sole non-trivial case is a `theta_sim` RAISE → AtRisk on null-player; mix + finalization params
   preserved-by-construction; constitutional moves deferred to the confluence engine. **What remains is
   NOT ours to build:** the confluence discharge + full attribution-preservation *proof* are Pragma's
   (terms-first, Will drives Tom + Bernhard), and the constitutional dimension-set surface is `pending`
   upstream. The socket is feature-complete for the deploy-independent, no-partner-terms slice.
4. **The joint-paper question** (`noesis-pragma-overlap.md:38-42`): is there a single fixed-point theorem
   under which observer-overlap consistency (OPH) and contribution-overlap finalization (PoM) are both
   instances? That is the research half; it does not block the socket build.

---

*Companion reading (read before building): `docs/phase4-fv-plan.md` §"Pragma Coherence",
`docs/Pragma Overlaps/noesis-pragma-overlap.md`, the `Constitution` struct doc at
`node/src/runtime.rs:28-79`, and the machine-checked value axioms in `internal/fv/Noesis_Rulebook.thy`.*
