# BUILD CONTRACT — canonical `tx_digest` serializer (next deploy-independent grain)

> Forward-intent, fully grounded 2026-06-19. Execute in a FRESH low-context window (moat PCP-gate +
> confidence-loop: needs a clean compile + break-on-purpose cycle). Zero re-grounding needed — the
> structs + placement + tests are pinned below. This is the prerequisite grain of the lock-sig
> existence→control mile (`DESIGN-locksig-binding.md`): the bytes the future lock-signature covers.

## Grounded facts (verified against source 2026-06-19)
- `Script { code_hash: [u8;32], args: Vec<u8> }` (lib.rs:32).
- `Cell { id: u64, lock: Script, type_script: Script, data: Vec<u8> }` (lib.rs:39, fields 43/49/55).
- `TokenTx { standard: TokenStandard, code_hash: [u8;32], args: Vec<u8>, inputs: Vec<Cell>, outputs: Vec<Cell> }` (runtime.rs:141).
- `TokenStandard { Fungible, Nft, Multi }` (runtime.rs:121).
- hasher in-tree = `blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-smt-v1\0\0\0").build()` — currently PRIVATE `fn blake2b` inside lib.rs smt module (lib.rs:6408). Node tests build Cells via `ft_cell()` (runtime.rs:510).
- existing canonical idiom to mirror: `canonical_order_is_invariant_to_presentation` (lib.rs:7745).

## Placement decision (lean / single-source — DECIDE at build, recommendation below)
- RECOMMEND: put the canonical serializer + a tx-domain `blake2b` in **noesis-core** (shared crate) so BOTH the node (replica determinism) and the eventual on-VM type-script (sig-verify) reuse ONE serializer — the (v) design's "needed by both" requirement. If noesis-core can't take a blake2b dep cleanly this tick, fall back to a `pub(crate) fn tx_digest` in runtime.rs with a LOCAL tx-domain hasher and a `// SINGLE-SOURCE DEBT: move to noesis-core at on-VM port` note (acceptable per CONTINUE "pay duplication debt"). Do NOT reuse the smt `blake2b` directly — tx digests MUST have a distinct domain (`personal(b"noesis-tx-v1\0\0\0\0")`, ≤16 bytes) so a tx digest can never collide with an smt node hash.

## The serializer (canonical, injective, presentation-invariant)
`fn tx_digest(tx: &TokenTx) -> [u8;32]`:
1. **Canonicalize order** (the signed digest = logical content, not array order): sort `inputs` and `outputs` each by the cell-identity key `(id, lock.code_hash, lock.args, type_script.code_hash, type_script.args, data)`. (Pure sort on a clone; do not mutate the tx.)
2. **Length-prefix EVERY variable field** (injective serialization — concatenation w/o length prefixes is ambiguous). Helper: `put(buf, bytes)` = append `(bytes.len() as u64).to_le_bytes()` then `bytes`.
3. Feed, in order: domain tag `b"noesis-tx-v1"`; `standard as u8`; `code_hash` (fixed 32, no prefix); `put(args)`; `inputs.len() as u64 LE`; for each sorted input cell `serialize_cell`; `outputs.len() as u64 LE`; for each sorted output cell `serialize_cell`.
4. `serialize_cell(c)` = `c.id.to_le_bytes()` ‖ `c.lock.code_hash` ‖ `put(c.lock.args)` ‖ `c.type_script.code_hash` ‖ `put(c.type_script.args)` ‖ `put(c.data)`.
5. blake2b-32 over the assembled buffer with the tx personalization ⇒ `[u8;32]`.

## Tests (confidence-loop discipline — break-on-purpose is the keystone)
1. `tx_digest_is_deterministic` — same tx hashed twice (and across a clone) ⇒ bit-identical. (×N loop, e.g. 16.)
2. `tx_digest_is_invariant_to_input_output_presentation` — reversing/shuffling the inputs vec AND the outputs vec ⇒ SAME digest (canonical sort). Mirror of lib.rs:7745.
3. `tx_digest_changes_iff_value_changes` — flipping any one of {an input's data, an input's lock.args, an output's data, the tx args, the standard, an input's id} ⇒ DIFFERENT digest; a no-op clone ⇒ same. (This is the sensitivity half.)
4. `tx_digest_no_field_boundary_collision` — two txs that concatenate to the same bytes WITHOUT length prefixes (e.g. args=`[1]`+data=`[2,3]` vs args=`[1,2]`+data=`[3]`) ⇒ DIFFERENT digests (proves the length-prefixing is load-bearing).
5. **BREAK-ON-PURPOSE (anti-theater, run + revert):** (a) remove the length-prefix in `put` (raw concat) ⇒ test 4 must go RED. (b) drop `data` from `serialize_cell` ⇒ test 3's data-flip case must go RED. (c) skip the canonical sort ⇒ test 2 must go RED. Confirm each reds the intended test, then revert. If any doesn't red, the test is theater — fix it before trusting.

## Acceptance
- node lib suite: was 225/225 at (u); after +5 tests target 230/230 (adjust to real baseline; honest-number the delta).
- 0 NEW clippy; my added lines fmt-clean; NO tree-wide `cargo fmt` (the roadmap's "stray rustfmt nearly leaked 2000 lines" incident — always `git diff --stat` after any formatter).
- commit + push origin master (private). ROADMAP (w) entry: BUILT — canonical tx_digest serializer (deploy-independent grain of the lock-sig mile); NEXT = wire the inert sentinel-gated `spend_is_authorized(input, tx_digest)` call-site per DESIGN-locksig-binding.md, then deploy-coupled verify_sig.

## After this grain
DESIGN-locksig-binding.md step 2 (inert auth call-site) → on-VM single-use (k) → learned-v(S)-on-real-labels (THE moat).
