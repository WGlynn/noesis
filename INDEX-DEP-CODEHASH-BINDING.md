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
