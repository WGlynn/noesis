# Phase 2 Report — Compact State (UTXO commitment, no ZK)

> Stateless-verification engagement. Phase 2 deliverable: an **audited commitment to the UTXO set**
> with membership proofs + an assumeutxo-style checkpoint — *no ZK yet* (the cheap 80%). Read
> `docs/rulebook-map.md` (Phase 0) and `docs/phase1-extraction-report.md` (Phase 1) first. Every
> number below came from a command actually run on this machine (Ryzen 5 1600, `--release`); the
> command is shown. No mocked or estimated figures.

## What Phase 2 required

> *Commit to the UTXO set (Merkle root / Utreexo-style accumulator), updated per block, committed in
> headers (or shadow-computed — say which and why). Nodes serve/verify membership proofs ("this UTXO
> is unspent") in KB, plus an assumeutxo-style checkpoint where a new node bootstraps from a snapshot
> + commitment. Report commitment cost/block, proof sizes, and the honest trust statement: this phase
> gives compact description, not trustless conviction; a checkpoint is trusted until Phase 3.*

## The accumulator decision — B (audited, vendored) — and why

The engagement rule is **"all accumulator/Merkle code uses audited libraries; never hand-roll
crypto."** Three options were weighed on real evidence, not preference:

- **A — extend Noesis's own `NoveltyIndex` SMT.** Leanest, but it is u64-keyed + add-only, and it
  would be *unaudited code securing value*.
- **B — the audited Nervos `sparse-merkle-tree`** (mainnet-proven on CKB). Chosen.
- **C — hunt for another pure-Rust SMT.** No candidate at CKB's assurance level.

**Execute-to-verify surfaced the real constraint:** the CKB crate hard-pulls a **C** blake2b
(`blake2b-rs` via `cc`), which (i) **won't build on this box at all** — there is no C compiler
(`gcc`/`cc`/`clang` all absent) — and (ii) would break Noesis's pure-Rust discipline that keeps the
code cross-compilable to RISC-V for the Phase-3 zkVM guest.

**Resolution — vendor + strip.** `onchain/vendor/sparse-merkle-tree/` is the upstream crate (MIT,
v0.6.1) with the C machinery removed: the `blake2b` (C), `ckb_smt` (C SMT + `build.rs`/`cc`), and
`trie` modules and the crate's own tests/benches are gone. **The audited tree/merge/proof math is
unchanged.** We supply the `Hasher` ourselves — Noesis's pure-Rust `blake2b-ref` — via the crate's
own `Hasher` trait (`node/src/utxo_commitment.rs::Blake2bRefHasher`). Result: the audited algorithm,
**pure Rust, no C toolchain, RISC-V-clean.**

**Does this add a trusted third party?** No — not in the sense that matters. An SMT is a
*verification* primitive, not a *trust* one: a proof checks by **independent re-hashing**, so no
runtime party enters the trust path, and in Noesis every node recomputes the root itself (a bad SMT
implementation causes **consensus divergence**, caught — it can never forge a passing proof, which
would require breaking blake2b). The only trust B adds is *supply-chain* ("is the code correct"), and
there it is **lower-risk than A**: audited-and-vendored (pinned, reviewable) beats fresh in-house code
for the accumulator that secures coins.

## What was built (`node/src/utxo_commitment.rs`)

- `UtxoCommitment` over the live token-cell set: `from_cells` / `insert` / `remove` / `root`.
- **Leaf key** = the consensus identity `(id, lock, type_script, data)` — the *same* tuple
  `TokenTx::is_valid_in_ledger` resolves existence on (`parent`/`timestamp` excluded, matching the
  ledger). Domain-separated blake2b (`noesis-utxoid`, `noesis-utxo-v1`) so a UTXO hash can never
  collide with the novelty SMT (`noesis-smt-v1`), the tx digest, or the Lamport lock.
- **Membership proof** (`prove` + `verify`, `unspent = true`): "this coin is in the committed set."
- **Non-membership proof** (`unspent = false`): "this coin is spent / never existed." Mutually
  exclusive with membership by construction.
- **assumeutxo checkpoint** (`verify_snapshot`): does a snapshot of cells reproduce a committed root?
  A new node bootstraps from {snapshot, root} — **trusted until Phase 3** proves the snapshot is the
  result of a valid history.

### Tests — RED→GREEN (7, all passing)
`cargo test -p noesis --lib utxo_commitment` →
- root is a **set** commitment (order-independent); different set ⇒ different root; empty ≠ non-empty.
- unspent coin ⇒ verifiable **membership**; the same proof does **not** verify as non-membership.
- absent coin ⇒ verifiable **non-membership**; does **not** verify as membership.
- **spend retires**: after `remove`, the coin proves non-membership, its neighbours still prove
  membership, and the root moves. Incremental `insert` == batch `from_cells` (`root` identical).
- **checkpoint** verifies an honest snapshot and rejects a missing coin, an extra coin, and a
  one-bit-flipped root.
- a **tampered proof** does not verify.

Full suite after Phase 2: **`cargo test -p noesis` → 292 lib + 66 integration = 358 passed / 0
failed** (was 351; +7 commitment tests). `cargo clippy -p noesis --tests` → 0 warnings in the new
code. Phase-1 replay parity untouched (this module is additive — see Scope).

## Measured numbers (honest)

```
$ cargo test -p noesis --release --lib utxo_commitment::tests::report_metrics -- --ignored --nocapture
n=   100  build_full_root=  12.99ms  one_insert= 77.80µs  proof_bytes=469  prove= 72.50µs  verify_ok=true
n=  1000  build_full_root= 193.62ms  one_insert= 82.40µs  proof_bytes=300  prove= 66.50µs  verify_ok=true
n= 10000  build_full_root=   2.02s   one_insert=119.60µs  proof_bytes=467  prove=116.60µs  verify_ok=true
```
(Ryzen 5 1600, release. The `report_metrics` test is kept in-tree, `#[ignore]`d, so the numbers are
reproducible with the command above.)

- **Proof size: ~300–470 bytes** — sub-KB as required. It is O(log n)·(hashes), so it stays small
  regardless of set size (non-monotone in n because the compiled path depends on the key's tree
  neighbourhood, not just |set|). *Verify* runs the same `compute_root` work as a single `prove`
  (~tens–hundreds of µs); it was not separately isolated in this run.
- **Per-block commitment update:** an incremental `insert`/`remove` is **~78–120 µs, flat in n**
  (O(log n)). This is the real per-block delta cost.
- **⚠ Honest finding — the shadow full-rebuild does NOT scale.** `from_cells` (rebuild the whole
  root) is O(n): **2.02 s at 10k cells**. The v1 here **shadow-computes** the root/proofs from
  `token_cells` on demand, which is correct and simple but O(n) per query. **Production must maintain
  the SMT incrementally** (insert on output, remove on spend, inside the transition) — the ~80 µs
  path — rather than rebuild. The code already supports it (`insert`/`remove`); wiring it into the
  per-block transition is the deploy step (below).

## Trust statement (required, stated plainly)

This phase gives a **compact, self-verifying description** of the UTXO set — *not trustless
conviction of validity*. A membership proof proves *"this coin is in the set the root commits to,"*
not *"this set is the correct result of a valid history."* An assumeutxo **checkpoint is trusted
until Phase 3** replaces that trust with a zkVM validity receipt.

## Boundaries (restated every report)

Even once Phase 3 lands, a validity receipt proves **validity only** — never **canonicality** (a
valid fork is still a fork; fork-choice is untouched) and never **data availability** (a node
convinced of a root still needs the underlying cells). No artifact here claims "trustless full
verification" without naming both.

## Scope — what Phase 2 deliberately did NOT do

- **Shadow-computed, not header-committed.** The root is derived from `token_cells` on demand, not
  yet embedded in a block header (Noesis has no header commitment field today) nor folded into
  `state_digest`. Both are additive consensus changes deferred so Phase 1's replay parity stays
  exactly intact. Folding the root into `state_digest` (so every replica agrees on the commitment) is
  the natural first consensus wiring.
- **No incremental maintenance yet** — see the honest finding above; the O(80 µs) path exists but is
  not wired into `apply_transition`.
- **Host-side only.** The commitment is pure-Rust and therefore *ready* for the Phase-3 guest, but has
  not been compiled to RISC-V here (no prover env on this box; Phase 3 runs on Linux/CI).

## Next: Phase 3 — Provable history (the ZK core)

The commitment is now the object a zkVM receipt will bind to: prove `apply_block(S_n, B) = S_{n+1}`
against these roots, recurse over history, emit a sync artifact. **Footnote carried forward:** RISC
Zero hardware-accelerates SHA-256, not blake2b, so a blake2b commitment proves slower there — a real
Phase-3 cost input to measure, not a blocker.
