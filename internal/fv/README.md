# Phase-4 Step-3 — Isabelle/HOL spec of the rulebook (review artifact)

> **Status: 🟡 written, NOT machine-checked here.** Isabelle/HOL is not installed in the authoring
> environment, so the proofs in `Noesis_Rulebook.thy` are **undischarged** (`sorry` on the leaf
> finite-sum obligations) and written for the owner (Will, Isabelle-experienced) to load, adjust, and
> close. **Do not claim "conservation is proved" until this theory loads clean.** Steps 1–2 (the
> executable property + differential tests, `node/tests/fv_invariants.rs`) ARE verified and green;
> this step is the machine-checked-spec layer above them.

## Theory → invariant map

| Theory item | Invariant | Statement |
|---|---|---|
| `conservation` | **I1** value conservation | `tx_valid s t ⟹ total (apply_tx s t) = total s` — no inflation. |
| `no_double_spend` | **I2/I3** no double-spend | `tx_valid s t ⟹ ins t ∩ dom (apply_tx s t) = {}` — a consumed input is gone from the next state, so no later tx can consume it. |
| `determinism` | **I5** determinism | `apply_tx s t = apply_tx s t` — trivial in HOL (a function); the Rust `p4_apply_block_is_deterministic` is the operational witness. |

I4 (no-spend-of-nonexistent) is captured *inside* `tx_valid` as the premise `ins t ⊆ dom s`: an input
outside `dom s` makes the tx invalid, so no theorem quantifies over it. The single-use of I2 in-block
is the disjointness `ins ⊆ dom s ∧ dom (outs) ∩ dom s = {}`.

## Model-to-code gap — enumerated (REQUIRED; read before trusting any theorem)

The `.thy` proves theorems **about the model**, not about the Rust. The model is deliberately small;
each abstraction below is a place where a model theorem does **not** transfer to `runtime::apply_block`
without the stated argument. This list is the honest boundary of what Step 3 buys.

- **G-a — arithmetic.** The model uses ideal `nat` (unbounded, exact subtraction). The code uses `u128`
  with `saturating`/wrapping arithmetic. Conservation in ℕ does not by itself rule out a u128 overflow
  in the code. *Mitigation:* the Step-1 `proptest`-style suite exercises the real `u128` path over the
  amount domain; overflow would surface there, not here.
- **G-b — omitted state.** The model is pure value (`cid ⇀ value`). It omits locks/scripts (ownership,
  the lock-sig CONTROL layer), PoM attribution, novelty, finality, and the cumulative-work clock.
  Theorems here say nothing about those; they are separate concerns with their own tests.
- **G-c — mint/burn.** The model uses **strict** conservation (`consumed = produced`). The code allows
  an authorised issuer **mint** (`Σout > Σin` iff the issuer spends an authority cell) and **burn**
  (`Σout < Σin`). So the code's true invariant is `total' = total + minted − burned`, of which this
  theory proves the `minted = burned = 0` restriction. Extending the model with a `minted`/`burned`
  term is the stated upgrade path.
- **G-d — identity vs value.** The model keys on `cid` alone (justified by the code's unique-id
  discipline). The code's identity is `(id, lock, type_script, data)`, and its data-binding (amount
  match on the input) is what the model's "input exists at its value" collapses into. The differential
  (`spec_oracle` in Step 2) checks this correspondence operationally over the real types.
- **G-e — fidelity of `apply_tx` to `apply_transition`.** That `apply_tx`/`apply_block_model` faithfully
  mirrors `runtime::apply_transition` is argued **by inspection**, not extracted from the Rust. This is
  the largest gap. The Step-2 differential narrows it (the oracle IS this model, in Rust, and agrees
  with `apply_block` over the fuzzed cases), but does not eliminate it.

**Optional gap-shrinking path (note, not required):** Rust→proof tooling (`Creusot`, `hax`+F*/Coq,
`Aeneas`) to verify the actual Rust rather than a hand-mirrored model. Heavy; only worth it post-launch.

## Reviewer checklist (for discharging the `sorry`s)

1. `total_disjoint_union` — `sum.union_disjoint` after `dom (a ++ b) = dom a ∪ dom b` and the
   override behaviour of `++` on disjoint domains.
2. `total_restrict_diff` — `sum.subset_diff` to split `total s` over `I` and `dom s − I`; the `I` part
   is `consumed_value s I`; ℕ subtraction is exact because `consumed_value s I ≤ total s` (needs a
   `sum_mono`/`sum.subset` side-lemma).
3. `conservation` final step — `(total s − c) + c = total s` in ℕ requires `c ≤ total s`; discharge
   `consumed_value s (ins t) ≤ total s` from `ins t ⊆ dom s` + finiteness.
4. `finite (dom (outs t))` — add `finite (dom (outs t))` (or `finite`-map) as a well-formedness
   premise, or model `outs` as an `alist`/`fmap`; real txs are finite.

## A separate axis — rule-set-mutation coherence (do NOT force it here)

These theorems are the **per-execution** invariants (does one `apply_block` preserve value?). A
distinct question — does a Constitution *amendment* keep the axioms? — lives on the **rule-set
MUTATION** axis (confluent-rewriting + axiom-preservation), a complementary, optional layer that is
NOT proved by these theorems and must not be conflated with them. The candidate tooling and scoping
for that axis are tracked in `docs/phase4-fv-plan.md` (its Pragma section); keep it off the
per-execution UTXO invariants.
