# Phase-4 Step-3 — Isabelle/HOL spec of the rulebook (MACHINE-CHECKED)

> **Status: ✅ machine-checked GREEN** under Isabelle2025 on 2026-07-12 (`isabelle build -D internal/fv`,
> exit 0, 0 `sorry`). `conservation`, `no_double_spend`, and `determinism` are discharged. This is the
> owned, reproducible check — not a one-time vouch: re-prove it in one command with `./verify.sh`.
> Steps 1–2 (executable property + differential tests, `node/tests/fv_invariants.rs`) are also green;
> this is the machine-checked-spec layer above them.

## Reproducible verification (the pin — anyone can re-prove from source)

The value here is not "it went green once for me." It is that the proof **re-verifies on any machine,
forever, in one command**, with no human or agent in the loop.

- **Tool (pinned):** Isabelle2025.
- **Download (Windows):** `https://isabelle.in.tum.de/website-Isabelle2025/dist/Isabelle2025.exe`
  (Linux/macOS assets under `https://isabelle.in.tum.de/`).
- **sha256 (Windows .exe, 951 589 856 bytes):**
  `5225d0e28a3c9cb3e4e348b508d332165c65b806201f3936a7a861a6bb39a748`
- **Re-verify:**
  ```
  ISABELLE=/path/to/Isabelle2025/bin/isabelle ./verify.sh      # or ./verify.sh if isabelle is on PATH
  ```
  A green run (`Finished Noesis_FV`, exit 0) means `conservation` + `no_double_spend` were
  machine-checked THIS run, on THIS machine.
- **Windows-without-WSL note (how this was first verified):** if the Isabelle bundle is extracted with
  a non-Isabelle tool (e.g. 7-Zip), the SFX's Cygwin init is skipped — run it once before `isabelle`
  will start:
  `contrib/cygwin/isabelle/cygwin.exe --quiet-mode --no-verify --only-site --site
  https://isabelle.sketis.net/cygwin_2025 --root <ISABELLE>/contrib/cygwin`, then invoke `isabelle`
  from the bundled Cygwin login shell (`contrib/cygwin/bin/bash --login`, so `uname` reports
  `CYGWIN_NT*`). ASCII symbol escapes (`\<rightharpoonup>`, `\<Sum>`, …) are used in the `.thy` for
  encoding-independence in headless builds.

## Theory → invariant map

| Theory item | Invariant | Statement (machine-checked) |
|---|---|---|
| `conservation` | **I1** value conservation | `finite (dom s) ⟹ tx_valid s t ⟹ total (apply_tx s t) = total s` — no inflation. |
| `no_double_spend` | **I2/I3** no double-spend | `tx_valid s t ⟹ ins t ∩ dom (apply_tx s t) = {}` — a consumed input is gone from the next state, so no later tx can consume it. |
| `determinism` | **I5** determinism | `apply_tx s t = apply_tx s t` — trivial in HOL (a function); the Rust `p4_apply_block_is_deterministic` is the operational witness. |

I4 (no-spend-of-nonexistent) is captured as the `ins t ⊆ dom s` premise of `tx_valid`. In-block
single-use is the `dom (outs t) ∩ dom s = {}` freshness condition.

## Model-to-code gap — enumerated (REQUIRED; what the theorems do and do NOT transfer)

The `.thy` proves theorems **about the model**, not about the Rust. The model is deliberately small;
each abstraction below is a place where a model theorem does **not** transfer to `runtime::apply_block`
without the stated argument. This list is the honest boundary of what Step 3 buys.

- **G-a — arithmetic.** The model uses ideal `nat` (unbounded, exact subtraction). The code uses `u128`
  with `saturating`/wrapping arithmetic. Conservation in ℕ does not by itself rule out a u128 overflow
  in the code. *Mitigation:* the Step-1 property suite exercises the real `u128` path over the amount
  domain; overflow would surface there, not here.
- **G-b — omitted state.** The model is pure value (`cid ⇀ amount`). It omits locks/scripts (ownership,
  the lock-sig CONTROL layer), PoM attribution, novelty, finality, and the cumulative-work clock.
  Theorems here say nothing about those; they are separate concerns with their own tests.
- **G-c — mint/burn.** The model uses **strict** conservation (`consumed = produced`). The code allows
  an authorised issuer **mint** (`Σout > Σin` iff the issuer spends an authority cell) and **burn**
  (`Σout < Σin`). So the code's true invariant is `total' = total + minted − burned`, of which this
  theory proves the `minted = burned = 0` restriction. Adding a `minted`/`burned` term is the upgrade path.
- **G-d — identity vs value.** The model keys on `cid` alone (justified by the code's unique-id
  discipline). The code's identity is `(id, lock, type_script, data)`, and its data-binding (amount
  match on the input) is what the model's "input exists at its value" collapses into. The Step-2
  differential (`spec_oracle`) checks this correspondence operationally over the real types.
- **G-e — fidelity of `apply_tx` to `apply_transition`.** That `apply_tx` faithfully mirrors
  `runtime::apply_transition` is argued **by inspection**, not extracted from the Rust. This is the
  largest gap. The Step-2 differential narrows it (the oracle IS this model, in Rust, and agrees with
  `apply_block` over the fuzzed cases), but does not eliminate it.

**Optional gap-shrinking path (note, not required):** Rust→proof tooling (`Creusot`, `hax`+F*/Coq,
`Aeneas`) to verify the actual Rust rather than a hand-mirrored model. Heavy; only worth it post-launch.

## A separate axis — rule-set-mutation coherence (do NOT force it here)

These theorems are the **per-execution** invariants (does one `apply_block` preserve value?). A
distinct question — does a Constitution *amendment* keep the axioms? — lives on the **rule-set
MUTATION** axis (confluent-rewriting + axiom-preservation), a complementary, optional second line of
defence that is NOT proved by these theorems and must not be conflated with them. The candidate tooling
and scoping for that axis are tracked in `docs/phase4-fv-plan.md` (its Pragma section); keep it off the
per-execution UTXO invariants.
