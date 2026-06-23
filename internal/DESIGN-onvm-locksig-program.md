# DESIGN — on-VM lock-script PROGRAM (existence→control, enforced inside the VM)

> pom-roadmap-advance design tick 2026-06-23 (rr). No code (PCP-gate: an ELF lock-script that
> reconstructs the TokenTx on-VM is the spend-validation trust boundary at its highest blast radius,
> and the build carries a genuine design fork — the TokenTx↔CKB-cell mapping — that must be DECIDED
> cold, then built in fresh low-context). Same discipline as (v)/(dd)/(ii)/(kk): decide now, build clean.
> Advances the #1 on-VM frontier from *named* → **DECIDED + build contract**.

## Grounded state (what already exists — verified, not from memory)
- **Verify arithmetic, single-sourced:** `noesis_core::lamport::verify(root, msg, sig)` ((pp)) — no_std,
  builds riscv64imac. The node's `runtime::lamport` re-exports it.
- **Digest, single-sourced:** `noesis_core::tx::tx_digest(standard, code_hash, args, &[CellView], &[CellView])`
  ((qq)) — no_std, builds riscv; byte-identical to what the node signs/verifies over.
- **The node-side gate:** `TokenTx::spend_is_authorized(input, auth, &tx_digest)` ((nn)) verifies a
  presented `auth` for real against `input.lock.args`; wired into `is_valid_in_ledger`.
- **VM harness:** `node/tests/common/mod.rs::NoesisSyscalls` serves the Noesis `Cell` model
  (`inputs`/`outputs`/`deps`/`witnesses: Vec<Vec<u8>>`) to CKB-VM; `for_full_tx(...)` builds the tx shape;
  there is an end-to-end ELF runner (`pom_lock_script_consumes_syscalls...`, `ckb_vm_*` suites).
- **Sibling programs to mirror:** `onchain/finalization-typescript/src/main.rs` (group-iterate, header-`now`
  consensus-sourced, exit-code namespacing, sentinel-gated-inert deploy flag) and `commit-order-typescript`.

So BOTH ingredients the lock-script needs are already ported + riscv-built. What remains is one ELF that
loads the tx shape on-VM, rebuilds the digest, and verifies each input's signature.

## THE FORK (decided here): how does a Noesis `TokenTx` appear to a per-input lock script?
A CKB lock script runs once per input in its group and can load the WHOLE tx via `Source::Input` /
`Source::Output`. The `tx_digest` commits to `(standard, code_hash, args, inputs[], outputs[])`. Each is
sourced on-VM as follows — every field CONSENSUS-present, never attacker-choosable:
- **inputs[] / outputs[]** — iterate `Source::Input` / `Source::Output`; per cell load the digest fields
  (`id`, `lock.{code_hash,args}`, `type.{code_hash,args}`, `data`) into a `CellView`. (id/lock/type via
  `load_cell_by_field`; data via `load_cell_data`. In the reference harness these come straight off the
  served `Cell` model.)
- **code_hash / args (the token type)** — the TokenTx's `code_hash`+`args` ARE the cells' `type_script`
  fields (the harness sets `code_hash = cell.type_script.code_hash`, `script.args = type_script.args`). A
  Noesis token tx is single-type (one `type_script` governs the whole group), so the lock script reads them
  from the group's `type_script` (consensus-present), NOT from a witness. DECISION: assert all
  in-group cells share one `type_script` identity; use it as `(code_hash, args)`.
- **standard (the u8 tag)** — DERIVED from the `type_script.code_hash`, never a free field: each
  `TokenStandard` (Fungible/NFT/Multi) is a distinct type-script code. DECISION: a small const map
  `type_code_hash → standard`; an unknown code ⇒ reject (exit code). This closes the
  [dont-let-attacker-choose-critical-input] hole — `standard` enters the digest from consensus state.
- **auth (the per-input Lamport signature)** — `witness[input_index]` (GroupInput-aligned, exactly the
  `finalization-typescript` votes-witness idiom). The signature is NOT in the digest (the digest is
  auth-independent — confirmed in `noesis_core::tx`), so no circularity.

## Build contract (fresh low-context)
New crate `onchain/locksig-typescript` mirroring `finalization-typescript/Cargo.toml` (ckb-std + noesis-core).
`program_entry`:
1. Build `inputs`/`outputs` `CellView` vectors by iterating `Source::Input`/`Source::Output` to
   `IndexOutOfBound` (load fields per cell). Malformed/short load ⇒ exit 41.
2. Resolve `(code_hash, args)` from the single group `type_script`; resolve `standard` via the
   code_hash→standard map (unknown ⇒ exit 44 — the attacker-input pin).
3. `let digest = noesis_core::tx::tx_digest(standard, &code_hash, &args, &inputs, &outputs);`
4. For each input `i` in the script group: `auth = witness[i]`; `root = this input's lock.args` (32B; a
   non-32B lock.args ⇒ exit 43); `if !lamport::verify(&root, &digest, &auth) { return 42 }`.
5. All inputs verify ⇒ 0. Empty group ⇒ 41.
- **Exit-code namespace (40s, distinct from intake 0/11-23 and finalization 30s):** 0 pass · 42 a sig
  fails verification · 41 malformed cell/empty group · 43 lock.args not a 32-byte root · 44 unknown
  token-standard code_hash (attacker-chosen `standard` blocked).
- **Sentinel-gated-inert deploy flag** (`CONTROL_ENFORCED: bool = false`), same pattern as the
  finalization program's `REGISTRY_BINDING_ACTIVE` and the node's `CONTROL_BINDING_ACTIVE`: pre-deploy an
  EMPTY `witness[i]` ⇒ inert-pass (honest flows unchanged); a PRESENTED auth is verified for real. At
  deploy the flag flips and an empty auth no longer passes. Keeps the on-VM gate consistent with the node
  gate (nn).

## Tests (host ckb-vm harness, mirroring (nn) THROUGH the ELF)
Reuse `NoesisSyscalls::for_full_tx` + the ELF runner. With `seed→root` keygen on the host (the wallet
side), set an input cell's `lock.args = root`, sign the on-host-recomputed digest, put the sig in
`witness[i]`:
1. valid sig ⇒ VM exits 0 (control proven on-VM, end-to-end).
2. wrong-key sig over the same digest ⇒ exit 42 (existence ≠ control, on-VM — closes (o) at the VM layer).
3. tampered sig / non-32B lock.args / unknown type code_hash ⇒ 42 / 43 / 44.
4. **DIGEST PARITY (the load-bearing cross-check):** the digest the ELF computes on-VM must equal
   `TokenTx::digest` on the host for the same tx — assert a host-signed sig verifies inside the VM (it
   only can if the two digests are byte-equal). This is the on-VM analog of the (qq) regression proof.
- **Anti-theater:** an ELF whose verify is stubbed `true` ⇒ the wrong-key test (2) goes RED.

## LEAN / scope (PONYTAIL)
- Reuse the ported arithmetic verbatim — the ELF is glue (load → build views → digest → verify), no new crypto.
- One crate, one entry, 40s exit namespace. No new wire format beyond "witness[i] = auth" (the existing
  per-input witness idiom).
- 🔬 deferred: the code_hash→standard map hard-codes the 3 standards (fine — they're consensus constants);
  the 16 KiB auth per input is the Lamport size tradeoff already tracked. Group multi-type-script support
  is out of scope (a Noesis token tx is single-type by construction; asserted, not handled).

## NEXT after this build
The finalization PROGRAM's twin update — point `finalization-typescript` at `finalizes_pos_pom_fixed` ((oo))
so the on-VM finalization enforces the live (mm) PoS+PoM rule (its own design tick: the cell's `mix` field
becomes vestigial under the hardcoded `FINALITY_MIX_Q`; decide keep-and-assert vs drop). Then the lock-sig
GO-LIVE flip (both `CONTROL_BINDING_ACTIVE` + `CONTROL_ENFORCED`), then learned-v(S) (data-blocked).
