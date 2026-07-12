# Phase 4 — Verification of the Rulebook (formal verification): PLAN

> Stateless-verification engagement, final phase. Phases 0–3 make a receipt prove **the rulebook was
> followed**. Phase 4 attacks a different question: **is the rulebook RIGHT?** A perfectly-proven
> receipt for a rule that permits inflation is worthless. This is the plan (not the build); it runs
> **on any machine** (no prover needed). Cheapest-first ladder, per the engagement spec.
>
> **Anti-hallucination:** every `file:line` below is a pointer to RE-VERIFY at source before relying
> (Phase-1/2 shifted numbers). Do not assert a Noesis rule from this doc alone — open the file.

## 0. The invariants to prove, and where the code enforces them today

The pure rulebook is `runtime::apply_block(state, block, params) -> Result<Ledger, Violation>`
(= `validate_block` + `apply_transition`, `node/src/runtime.rs`). The invariants Phase 4 must pin:

| Invariant | Informal statement | Enforced in code (re-verify) |
|---|---|---|
| **I1 — value conservation** | a token movement neither creates nor destroys value (except authorised mint/burn) | `TokenTx::is_valid` → `tokens::{fungible,nft,multi}` (`node/src/tokens.rs` ~54–177) |
| **I2 — no double-spend, in-block** | no input identity consumed twice within one block | `token_txs_conserve_and_single_use` (`runtime.rs` ~535, the consumed-identity set) |
| **I3 — no double-spend, cross-block** | a retired UTXO cannot be respent in a later block | `apply_transition` retires consumed inputs via `token_cells.retain` (`runtime.rs` ~612+) |
| **I4 — no spend of nonexistent/spent output** | every consumed input must match a live cell on full identity `(id,lock,type,data)` | `TokenTx::is_valid_in_ledger` input-is-live check (`runtime.rs` ~293) |
| **I5 — determinism under re-serialization** | decode∘encode is identity; same block ⇒ same `state_digest`, byte-for-byte | wire format (`onchain/noesis-core` parse/encode); `Ledger::state_digest` (`runtime.rs` ~158) |

Phase-2 also gives a *commitment-level* transition invariant (I6): a `TransitionWitness` verifies
against `old_root` and `new_root` iff the spend/create set is exactly applied — already host-tested
(`utxo_commitment::tests::utxo_transition_is_zk_verifiable_against_both_roots`). Phase 4 lifts I1–I4
to the *value* level, above I6's *identity* level.

## Step 1 — Property-based tests over `apply_block` (cheapest; do first)

**Tool:** Rust `proptest` (add as a `dev-dependency`; pure-Rust, runs here). **Location:**
`node/tests/fv_invariants.rs`.

**Generators (`prop_compose!`):**
- `arb_cell` — random `Cell` (id, lock/type `Script{code_hash,args}`, data). Keep the value-domain
  small so collisions/edge cases surface.
- `arb_token_tx` — inputs drawn from a supplied live-set + fresh outputs; a `conserving: bool` knob to
  generate BOTH conserving and non-conserving txs (so the negative assertions have teeth).
- `arb_block` — a canonical-ordered block at `height = ledger.height + 1` (reuse `Block::assemble` so
  ordering is valid by construction; a separate generator emits *non-canonical* blocks for I5/validate).

**Properties:**
- **P1 (I1):** for any block that `apply_block` ACCEPTS, `Σ value(inputs) == Σ value(outputs)` per
  standard, summed over the block. And: any tx with `Σin ≠ Σout` ⇒ `apply_block` returns
  `Err(Violation::TokenTxInvalidOrDoubleSpend)`.
- **P2 (I2/I3):** applying a block that spends cell K, then a second block that spends K again, ⇒ the
  second `apply_block` returns `Err(...)` (cross-block); a block with two txs consuming the same
  identity ⇒ `Err(...)` (in-block).
- **P3 (I4):** a tx spending an input not in the live set ⇒ `Err(...)`.
- **P4 (I5 determinism):** `apply_block(s, b)` run twice on clones ⇒ identical `state_digest`; and
  `decode(encode(cell)) == cell` for the wire types (round-trip). Also: `state_digest` is invariant
  under input/output *presentation* order where the ledger treats them as equal (mirror the existing
  `tx_digest` canonicalisation).
- **P5 (total supply):** over a random *sequence* of accepted blocks starting from a seeded issuance,
  total fungible supply per token == initial issuance ± authorised mint/burn (never drifts).

**Acceptance:** all properties green at ≥10k cases each; every negative property demonstrably RED if
the corresponding check is deleted (anti-theater — a property that can't fail proves nothing).

## Step 2 — Differential testing

- **Old-vs-new is largely already covered:** Phase 1 unified the paths (`Node::apply` routes through
  the same `apply_transition` as `apply_block`), and `node/tests/apply_block_parity.rs` +
  `core_drift_guard.rs` already diff node-vs-core over fixtures. So there is no independent legacy path
  to fuzz against here — **state that honestly, don't invent one.**
- **The useful differential:** fuzz `apply_block` against a tiny INDEPENDENT reference model of the
  invariants (a ~50-line "spec oracle" in the test crate: a `BTreeMap<utxo_id, value>` ledger with a
  naive apply). For each generated block, assert `apply_block` accepts iff the oracle accepts, and the
  resulting value-multisets agree. Divergence = a real bug in one of the two. This doubles as the
  bridge to the Isabelle model (the oracle IS the model, in Rust).

## Step 3 — Machine-checked spec (Isabelle/HOL) — structured for the owner's review

Will has Isabelle/HOL experience; the goal is a spec HE reviews, **not** full code-level verification.
**Location:** `internal/fv/Noesis_Rulebook.thy` (+ a `README` mapping theory names → the invariants).

**Model (abstract, deliberately small):**
- `state = utxo :: (id ⇀ value)` (a partial map / finite multiset of live UTXOs) — the *value* view,
  abstracting away locks/scripts/PoM (those are separate concerns; state the abstraction).
- `tx = (inputs :: id set, outputs :: (id × value) list)`.
- `apply_tx`, `apply_block` as pure functions on `state`, mirroring `validate_block`+`apply_transition`
  at the value level.

**Theorems (the minimum the spec must prove):**
1. **`conservation`:** `apply_block s b = Some s' ⟹ total_value s' = total_value s` (no inflation;
   with an explicit `+ minted − burned` term if mint/burn authority is modelled).
2. **`no_double_spend`:** an input `i` consumed by an accepted block is absent from `s'` (`i ∉ dom s'`
   via its identity), so no subsequent block can consume it again; and within a block, two txs sharing
   an input ⇒ `apply_block = None`.
3. (stretch) **`determinism`:** `apply_block` is a function (trivial in HOL, but state it to anchor I5).

**Model-to-code gap — stated explicitly (required):** the `.thy` proves the theorems *about the model*,
not about the Rust. The gap = (a) the model uses ideal integers / maps where the code uses
`u64`/`u128` saturating arithmetic + `HashMap`; (b) the model omits locks/PoM/finality; (c) fidelity of
`apply_block_model` to `runtime::apply_block` is argued by inspection, not extracted. **Name each gap
line in the README.** Optional upgrade path to shrink the gap (note, don't require): Rust→proof tools
(`Creusot` / `hax`+F*/Coq / `Aeneas`) to verify the actual Rust; heavy, and only worth it post-launch.

## Pragma Coherence — where it fits cleanly (and where it does not)

Pragma **Confluence** verifies **rule-set MUTATION coherence** (Knuth-Bendix / confluent-rewriting +
axiom-preservation): *"when the rules change, do they stay confluent and keep the axioms?"* — Tom
Lindeman's dangerous quadrant is **Confluent + Axiom-breaking** ("nobody checks that today").

- **✅ Fits:** the Noesis **Constitution-amendment** surface — governable params (`theta_sim_q16`,
  `MIN_DIM_BPS`, `vesting_w`, `threshold_bps`, the NCI/finality mix). When governance mutates these,
  does the *rulebook* stay coherent and keep I1–I4? That is exactly Confluence's job, and it is a
  DIFFERENT axis from the per-execution invariants — a genuine complement.
- **❌ Does NOT fit:** the per-execution UTXO invariants (conservation / no-double-spend). Those are
  Isabelle/HOL theorems about `apply_block`, not rule-set mutations. **Do not force Pragma onto them.**

**Recommendation:** Isabelle/HOL for I1–I5 (execution invariants); evaluate Pragma Confluence for the
governance-mutation layer as a *separate, optional* deliverable. First read
`docs/Pragma Overlaps/noesis-pragma-overlap.md`; the collaboration is live (Tom Lindeman + Bernhard
Mueller, `coherence.pragmaresearch.ai`) — a real POC opportunity, not a hypothetical.

## Boundaries (restated)

Phase 4 proves the rulebook is **internally correct**; it does not touch **canonicality** (which fork)
or **data availability** (a root is not the data). "Trustless full verification" is never claimed
without naming both. FV closes the *"is the rule right"* gap that a ZK receipt cannot.

## Sequencing for a fresh window

1. Step 1 property tests (`node/tests/fv_invariants.rs`, `proptest`) — a day's work, high value, all
   here. **Do this first; it will likely surface real edge cases before any Isabelle.**
2. Step 2 spec-oracle differential (extend the same test crate).
3. Step 3 Isabelle model + the 2 theorems + the gap README — hand to Will for review.
4. (optional) Pragma Confluence on the Constitution-amendment surface.

**Acceptance for "Phase 4 done":** P1–P5 green + anti-theater-checked; the oracle differential green
over ≥10k cases; `Noesis_Rulebook.thy` proving `conservation` + `no_double_spend` with the model-to-code
gap enumerated. Report → `docs/phase4-fv-report.md`.
