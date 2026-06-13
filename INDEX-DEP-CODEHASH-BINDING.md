# Index-dep code_hash binding (PRIVATE) — design, ready to build

> Closes the standing production-binding pin (CONTINUE.md PM-17 / HANDOFF): the on-VM
> program accepts the novelty-index root by SHAPE, not by code identity. Spec only;
> implementation deferred to a budgeted session (RISC-V ELF rebuild + ckb-vm fixtures).

## The gap (live)
`onchain/pom-typescript/src/main.rs:164` reads the index root with:
```rust
match load_cell_data(0, Source::CellDep) {
    Ok(rd) if rd.len() == 32 => { /* accept as root */ }
    _ => return 20,
}
```
Any cell-dep at slot 0 whose data is 32 bytes is accepted as the live index root. An
attacker who controls tx assembly can point cell-dep 0 at a forged cell carrying a root
of their choosing (e.g. an empty/rolled-back root under which their garbage classifies as
novel). The rolling-root transition rule (`index_rule::valid_root_transition`, T7 #3)
binds how the root EVOLVES, but nothing binds the SOURCE of the root the mint proves
against. Shape != identity.

## The binding
Identity of the index cell = its **type-script code_hash** (the validation code that
maintains the rolling root). Bind the dep to that code_hash:

1. **Expected hash via config** — the consuming type-script carries the expected index
   code_hash in its own `args` (deploy-time config). Read once via `load_script()` →
   `.args()`; the trailing 32 bytes (after any existing arg layout) = `expected_index_code_hash`.
2. **Load the dep's type-script code_hash** — `high_level::load_cell_type(0, Source::CellDep)`
   → `Some(script)` → `script.code_hash()` (32 bytes). `None` (dep has no type-script) ⇒
   reject.
3. **Compare** — `dep_code_hash != expected_index_code_hash` ⇒ `return 23` (new exit:
   index-dep identity unbound/mismatch). Equal ⇒ proceed to the existing 32-byte data read.
4. **Backward-compat (optional-when-configured)** — if `args` carry NO expected hash
   (legacy/unset), keep the current shape-only path. The binding activates the moment the
   index type-script deploys and its code_hash is written into consumer args; the existing
   suite stays green until then. (Production deploy will REQUIRE it; a build flag flips
   optional→mandatory.)

Why type-script and not lock: the index cell's job is enforced by its type-script
(`valid_root_transition`); the lock only gates who can spend it. Provenance of the ROOT is
a type-script-identity question.

## Exit-code addition
- `23` = index cell-dep type-script identity unbound or mismatched (distinct from `20`
  = root missing/malformed, so triage separates "no root" from "wrong root source").
  Update the `//!` exit-code legend at the top of `main.rs`.

## Test plan (ckb-vm integration, fresh session)
Fixtures in the existing T7 harness shape:
- **bound-match**: consumer args carry `H`; cell-dep 0 type-script code_hash == `H` ⇒ mint
  proceeds exactly as today (novel mint, replay still 22, etc. unchanged).
- **bound-mismatch**: args carry `H`; dep type-script code_hash == `H'` != `H` ⇒ `23`,
  even when the 32-byte root data would otherwise validate the mint. The forged-root attack
  is denied at the source.
- **dep-no-type-script**: args carry `H`; cell-dep 0 has no type-script ⇒ `23`.
- **legacy-unset**: args carry no expected hash ⇒ current shape path; all existing on-VM
  tests pass unmodified (regression guard for backward-compat).

## Deploy dependency (the honest gate)
The concrete 32-byte value of `expected_index_code_hash` is the index type-script's own
code_hash, known only once that script is compiled+deployed. The MECHANISM + the four
fixtures land now with a fixture hash; the real value is a deploy-time arg, no code change.
So this is "designed + buildable against a fixture", not "blocked" — the only thing waiting
on deploy is the literal hash constant, not the binding logic.

## Composition
- Pairs with `index_rule::valid_root_transition` (T7 #3): transitions structurally bound +
  source now identity-bound ⇒ the full index path is provenance-closed.
- Same shape as a pinned-`code_hash` cell-dep in standard CKB scripts (vs `hash_type: data`
  by-shape). We are moving from data-shape acceptance to type-identity acceptance.

## Critical-QA (adversarial pass, 2026-06-13) — run the adversary against the design before building

Three findings; the first inverts part of the spec above.

**F1 (CRITICAL — the args-sourced expected-hash is itself gameable).** The spec sources
`expected_index_code_hash` from the consuming type-script's own `args`. But a cell's args
are chosen by whoever assembles that cell. An attacker minting through this program deploys
their own mint cell, so they pick its args freely — they set `expected_index_code_hash` to
the code_hash of a *forged* index cell they also deploy, point cell-dep 0 at it, and
`bound-match` passes trivially. A free arg the attacker controls is not a binding; it is a
self-assertion. **Fix:** the expected index identity must be a value the attacker cannot
choose — either (a) a **compile-time constant baked into the pom-typescript binary**
(`const EXPECTED_INDEX_SCRIPT_HASH`), so every instance of this script enforces the SAME
index identity, filled at the deploy that fixes the index script; or (b) pinned by the
consensus/governance layer (a well-known governance cell whose own identity is
consensus-rooted). Prefer (a) on-VM. The arg path is demoted to dev/test ONLY (never the
production binding). This flips the spec: the binding lives in the binary, not in args.

**F2 (bind the full Script identity, not code_hash alone).** Two scripts with the same
`code_hash` but different `hash_type` (`type` vs `data` vs `data1`) are distinct programs.
Comparing `code_hash` only is a partial check. **Fix:** compare the dep's computed
**script hash** (the blake2b of the full `Script` molecule: code_hash ‖ hash_type ‖ args),
or at minimum the `(code_hash, hash_type)` pair. The constant in F1 becomes a 32-byte
script hash, not a code_hash.

**F3 (code-binding ≠ instance/freshness-binding — the survivor).** Even a correctly
identity-bound dep can be a STALE instance: a cell carrying the right index type-script but
an OLD (rolled-back) root is still code-valid, and the mint then proves against that stale
root under which the attacker's garbage classifies as novel. `valid_root_transition`
(T7 #3) binds how the root EVOLVES but the mint reads whatever root the presented dep
carries. **Fix (next layer):** bind the CANONICAL instance — make the index a **singleton**
via the standard CKB type-id pattern (exactly one live cell carries that type-id), and/or
require the dep's root to equal the consensus head commitment. This is the surviving attack
that names the next increment (adversarial-layering): identity-binding is necessary, not
sufficient; freshness/singleton-binding is the follow-on.

**Net:** the binding is real and worth building, but the args-sourced expected-hash was
gameable (F1) and code_hash-only was partial (F2). Revised design = compile-time (or
consensus-pinned) **script-hash** constant, full-Script identity, with **instance/freshness
binding (F3) pinned as the next layer**. Backward-compat unchanged: production builds embed
the constant; dev/test may leave it unset (shape path) for the existing fixtures.

## Critical-QA of the on-VM port (2026-06-13) — two real gaps in the shipped code

The binding logic is on-VM and regression-green, but an adversarial pass on the actual
`main.rs` code finds two things to close in the ACTIVATED build (both are functionally
inert pre-deploy, since the binding is sentinel-inactive either way, so no rebuild now):

**QA-port-1 (F2 incomplete on-VM — real). HOST-SIDE CLOSED 2026-06-13; on-VM mirror pending.**
The shipped check compares `code_hash` and the `args` (type-id) but NOT `hash_type`. A CKB
`Script` is `(code_hash, hash_type, args)`, and two scripts sharing code_hash+args but differing
in `hash_type` (Data / Type / Data1) are distinct programs. A forged dep reusing the canonical
code_hash+type-id under a different hash_type would currently pass.
- **DONE (reference model):** `index_binding` now carries `HashType{Data,Type,Data1}`, the dep
  is modeled as a full `DepScript{code_hash, hash_type, args}` triple, and `dep_accepted`
  compares all three. Regression `bound_wrong_hash_type_rejects` pins it (same code_hash + same
  type-id + Data-instead-of-Type ⇒ reject; Data1 too). node 197/197.
- **PENDING (on-VM, deploy-coupled, still inert):** add `EXPECTED_INDEX_HASH_TYPE` to
  `main.rs` and compare `r.hash_type()` in `index_dep_bound` (verify the `ScriptReader`
  hash_type accessor against the local ckb-std source before coding it — do not guess the API).
  Lands in the activated build with QA-port-2 + the activated-path fixture; the binding stays
  sentinel-inactive until the index script deploys, so existing on-VM fixtures stay green.

**QA-port-2 (sentinel overload — robustness).** `EXPECTED_INDEX_CODE_HASH == [0u8;32]` is
overloaded to mean "unset / legacy shape path." But all-zero is also a syntactically valid
code_hash; if a real index script ever had it (vanishingly unlikely for a blake2b output,
but a smell), "bound to all-zero" would be misread as "unset." FIX: replace the zero-value
sentinel with an explicit `const BINDING_ACTIVE: bool` flag; the activation state should be
its own bit, not a magic value of the data being checked.

Both land in the activated build alongside the activated-path fixture (exit-23-fires under a
live mismatch). The host reference model `index_binding` should grow a hash_type field at the
same time so it keeps mirroring the on-VM check.

## F3 resolution (next-layer advance, 2026-06-13) — freshness is free from the cell model

The F3 survivor ("a correctly-identity-bound dep can still be a stale rolled-back instance")
dissolves once you bind the **canonical instance**, and the CKB cell model gives that almost
for free:

1. **Which cell — type-id singleton.** Create the index cell once with a CKB **type-id**
   (`type_id = blake2b(first_input_outpoint ‖ output_index)`), enforced by the standard
   type-id rule so NO second cell can ever carry it. The canonical index identity is that
   type-id. The consumer binds two things: the dep's type-script **script-hash** == the
   compile-time constant (F1/F2, "right code") AND the dep's type-id arg == the canonical
   `INDEX_TYPE_ID` ("the one cell"). Only the canonical index satisfies both.
2. **Is it current — UTXO liveness (the free part).** A cell-dep can only reference a
   **live** (unspent) cell. The index is a singleton that updates by spend-old + create-new
   (cells are immutable), so every prior root lives in an already-**spent** cell and is
   therefore **uneferenceable as a dep**. The attacker cannot point cell-dep 0 at an old
   root because old roots are not live. "Which cell" (type-id) + "is live" (UTXO) ⇒
   "current root" — no extra freshness machinery, no consensus-head lookup needed.
3. **Same-tx rollback?** If the attacker also produces a new index version in the same tx,
   the index type-script's own `valid_root_transition` (T7 #3) rejects any non-forward
   transition; and the mint proves against the **input/dep** root, not the attacker's
   proposed output — so there is no rollback surface.

**Convergence (adversarial-layering).** F1 (binary-pinned script-hash) + F2 (full Script
identity) + F3 (type-id singleton, freshness-free via liveness) fully provenance-bind the
index dep. The remaining survivor reduces to the correctness of the type-id rule and
`valid_root_transition` themselves — already-built/assumed layers — i.e. the chain
terminates at the substrate's own guarantees, not a new dispute-able assumption. This is the
method's stop condition (survivor = global/substrate assumption). **Build order, fresh
session:** F1/F2 script-hash const + exit `23` + the four fixtures, then the type-id arg
check (F3) with a singleton/duplicate fixture.
