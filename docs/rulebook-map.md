# Noesis Rulebook Map — Stateless-Verification Engagement, Phase 0

> Synthesis of nine read-only finder passes over `C:/Users/Will/noesis` (2026-07-12).
> Every `file:line` below is one a finder verified, or was re-verified this pass. Where finders
> disagreed or left a gap, the gap is surfaced, not silently resolved. No code was modified.

---

## 1. Purpose & scope

The goal is to give Noesis nodes a **trustless** way to verify (a) the *current state* and (b) the
*full history* of the chain, by expressing the block state transition as a pure, deterministic
`apply_block(state, block) -> Result<state, violation>`, proving each transition with an established
**zkVM** (recursive validity proofs), and backing the invariants with **formal verification**
(Isabelle/HOL theorems over the rule-set). The binding **engineer-not-cryptographer constraint**:
**no novel cryptography.** All proving goes through an off-the-shelf zkVM (RISC Zero / SP1); all
commitments use audited accumulator / Merkle libraries (the tree already in-repo is `blake2b-ref`
0.3 personalized `noesis-smt-v1`). We author *rule extraction, purity, wire formats, and proof
plumbing* — never a hand-rolled SNARK circuit or a bespoke signature scheme.

---

## 2. Repo & workspace layout

**Host workspace** (`Cargo.toml` root = `[node, onchain/noesis-core]`): builds on host stable
(`rustc 1.93.1`). The RISC-V on-VM crates (`onchain/{pom,finalization,commit-order,locksig}-typescript`,
`onchain/zk-finalize`) are **excluded** from the root workspace — they pin their own nightly + RISC-V
target and cannot build without risc0/CKB tooling absent on this box.

| Concern | Where it lives | file:line |
|---|---|---|
| Block acceptance (VALIDATE, 6 checks) | node (std, impure struct / pure logic) | `node/src/runtime.rs:516` |
| State transition (APPLY) | node (std, `&mut self`) | `node/src/runtime.rs:612-658` |
| Token conservation + within-block double-spend | node | `node/src/runtime.rs:535-550` |
| TokenTx ledger-validity (existence/conserve/auth) | node | `node/src/runtime.rs:293-314` |
| UTXO existence (linear Vec scan) | node | `node/src/runtime.rs:293-301` |
| UTXO retire + output persist | node | `node/src/runtime.rs:625-636` |
| Ledger state container (in-memory only) | node | `node/src/runtime.rs:101-133` |
| `state_digest` (consensus comparison tuple) | node | `node/src/runtime.rs:158-164` |
| Cumulative-work clock `now()` (NOT wall-clock) | node | `node/src/runtime.rs:170` |
| Finality decision (f64 reference) | node | `node/src/runtime.rs:701`, `:750` |
| `FINALITY_MIX` / `MIN_DIM_BPS` (f64) | node | `node/src/runtime.rs:726`, `:738` |
| NCI mix / `TWO_THIRDS_BPS` (production path) | node | `node/src/lib.rs:3713`, `:3715` |
| PoM attribution recompute (Q16.16) | node | `node/src/lib.rs:188-199` |
| Novelty SMT (order-independent root) | node | `node/src/lib.rs:8031-8083` |
| Soulbound `Standing`/`Op` cell state machine | node | `node/src/lib.rs:465-497` |
| **Shared no_std verify-core** (pure) | `onchain/noesis-core` | `onchain/noesis-core/src/lib.rs:1-14` |
| Finalization mirror (Q32.32) | core | `onchain/noesis-core/src/lib.rs:352`, `:481` |
| Commit-order + wire | core | `onchain/noesis-core/src/lib.rs:232-341` |
| Wire formats (finalization/votes/cells/tx) | core | `onchain/noesis-core/src/lib.rs:506-609`, `:702-853` |
| Lamport PQ signature | core | `onchain/noesis-core/src/lib.rs:619-700` |
| PoM JSON export (only serde surface, host-side) | node | `node/src/pom_export.rs:23-24` |
| ZK PoC (finalization verdict only) | on-VM (excluded) | `onchain/zk-finalize/methods/guest/src/main.rs:17-48` |

The node **re-exports** the core rather than duplicating it (`node/src/lib.rs:8524-8528`, `:9319-9320`),
so producer(node) and decoder(on-VM type-script) share one definition by construction.

---

## 3. The complete state-transition rules `(prev_state, block) -> new_state`

### 3a. Block / cell validation — the acceptance gate (pure logic over `&self`)
`Node::validate` (`node/src/runtime.rs:516-523`) is an AND of six checks:
1. `b.height == ledger.height + 1` (extends by one)
2. `b.cells.len() == b.coords.len()`
3. `!b.cells.is_empty()`
4. `is_canonical_order(&b.coords)` — reject producer reorder (`onchain/noesis-core/src/lib.rs:304`)
5. all `coord.height == b.height`
6. `token_txs_conserve_and_single_use(b)` (`node/src/runtime.rs:535-550`)

Token validity per input: match a live cell on `(id, lock, type_script, data)` — the `data` bind
closes an amount-forgery hole (`node/src/runtime.rs:293-301`); conservation dispatches to
`tokens::{fungible,nft,multi}` (`node/src/tokens.rs:54-177`); auth via Lamport is **inert pre-deploy**
(`CONTROL_BINDING_ACTIVE=false`, `node/src/runtime.rs:381`).

### 3b. APPLY — the actual mutator (`node/src/runtime.rs:612-658`, re-verified this pass)
Fixed order: (a) retire consumed token inputs by identity via `Vec::retain`; (b) append tx outputs;
(c) insert `coverage(cell.data)` into novelty SMT + push cell; (d) `height = b.height`;
(e) `work = work.saturating_add(block_work(b))`; (f) stamp `finalized_at.entry(id).or_insert(work)`
(first-finalization-wins, idempotent); (g) **recompute** `pom = pom_scores_with_similarity_floor_q16(cells, theta)`
over the **entire** chain.

### 3c. PoM scoring — consensus-live path is integer-only
`apply` (g) and the finality bridge `finality_pom_weight` (`node/src/runtime.rs:588-605`) call **only**
`pom_scores_with_similarity_floor_q16` (`node/src/lib.rs:188-199`), which routes through the Q16.16
`value_fixed::temporal_novelty_with_similarity_floor_q16` (`node/src/lib.rs:6514-6531`): u128
cross-multiplied overlap comparison, no float, no division. **The entire `value_v4..v8` / `flow::*` /
`quality_scores` f64 family is reference-only — grep of `runtime.rs` for `value_v5..v8` is empty.**

### 3d. Standing updates — soulbound `Op` state machine (NOT wired into the block loop)
`Standing{contributor,pom}` + `Op::{Accrue,Decay,Slash,Burn}` (`node/src/lib.rs:465-497`); `apply`
uses saturating arithmetic; `valid_transition` (`:506`) forbids owner/contributor reassignment;
`valid_transition_under_dispute` (`:534`) caps decrease and blocks burn under an open Challenge.
**Honest gap (surfaced):** two finders (block-tx-validation, existing-zk) could **not** find a call
site wiring `Op` transitions into `Node::apply`/`validate`; treat this as a per-cell type-script rule
exercised by tests, **not** part of the block applier.

### 3e. Finality — a separate query, does NOT mutate state
`checkpoint_finalizes` → `finalizes` → `finalizes_pos_pom` (`node/src/runtime.rs:562`, `:701`, `:750`).
Two conjoined gates: (1) 2/3 supermajority via `finalizes_hybrid` using `FINALITY_MIX` (**PoW=0**,
PoS 1/3 : PoM 2/3 renormalized, `:726`); (2) anti-concentration `dim_ok` requiring **each** of PoS and
PoM to independently supply ≥ 50% (`MIN_DIM_BPS=5000`, `:738`/`:740`). This is upstream of `apply` and
its result is **not** folded into `state_digest`.

---

## 4. ★ CRITICAL FINDING — purity verdict

> **VERDICT: The state transition `Node::apply` is ALREADY a pure, deterministic, integer/byte
> function. It reads no clock (only the derived cumulative-work counter), no rand, no fs/net/DB/env,
> no mutable globals, and no f64. The one real float entanglement (the finality *decision*) is a
> SEPARATE query that `apply` never calls, and it already has a bit-exact Q32.32 fixed-point twin in
> the no_std core. Phase 1 is an EXTRACTION-and-RESHAPE (std→no_std, `&mut self`→value-returning,
> HashMap→BTreeMap/sorted-Vec), NOT an I/O-untangling.**

Crate-wide greps for `SystemTime|Instant|chrono`, `rand|thread_rng|getrandom|OsRng`,
`std::fs|File|sled|rocksdb|leveldb`, `std::net|tokio|reqwest`, `std::env|static mut|lazy_static|OnceCell|Mutex`
over `node/src` returned **zero** hits on the transition path (four finders, independent greps). The
only PRNG is deterministic `splitmix64` seeded from consensus data (`node/src/lib.rs:3150`,
`onchain/noesis-core/src/lib.rs:250`). The only `now` is `Ledger::now() = self.work`
(`node/src/runtime.rs:170`).

### Entanglement table (merged, all finders)

| Kind | file:line | Detail | Severity |
|---|---|---|---|
| **float** | `node/src/runtime.rs:740-744`, `:750-786` | Finality gadget (`dim_ok`, `finalizes_pos_pom`) sums PoS/PoM weights as f64. Order/platform-sensitive at ties. **MITIGATED**: not called by `apply`; exact twin `finalizes_pos_pom_fixed` at `onchain/noesis-core/src/lib.rs:481` (re-verified), rounds UP against finalization. | moderate |
| **float** | `node/src/lib.rs:3731-3754`, `:3852-3871` | `base_weight`/`retention`/`effective_weight`/`finalizes_hybrid` all f64. Live finality rule; off the apply/state path; Q32.32 mirror `finalizes_fixed` exists + drift-guarded. | moderate |
| **float** | `node/src/lib.rs:864-961`, `:980-1295` | `value_v4..v8`, `production_value`, `quality_scores` (Bradley-Terry: `.ln/.exp/sigmoid`), `flow::*` (damped Jacobi, `RHO=1/φ`, `1e-9` epsilon). **REFERENCE-ONLY**, not consensus-wired. | moderate |
| **float — BLOCKER (research)** | `node/src/lib.rs:1234-1295` | `value_v8` outcome-gate is f64-only with **NO fixed-point port at all** (`settlement_fixed` t6 comment `:8447`). Hardest gap **if** a learned `v(S)` is ever promoted into the live floor. Not consensus-wired today. | blocker (conditional) |
| **float** | `node/src/lib.rs:115-131` | f64 similarity-floor `temporal_novelty_with_similarity_floor` — the reference spec the Q16.16 consensus twin mirrors. Off-path. | cosmetic |
| **float** | `node/src/lib.rs:276`, `:385`, `:306-357` | `attribution_cycle_energy`, `collusion_residual_by_identity`, `solve_psd_cg` (conjugate-gradient) — f64 collusion **diagnostics** in the `value_fixed` namespace; not on the consensus novelty/floor path (inferred from names/return types, see gap). | moderate |
| **float** | `node/src/lib.rs:8530-8533` | `to_q(f64)->u128` — test/loader helper; on-VM inputs "arrive already fixed". Not on wire/consensus path. | cosmetic |
| **clock** | `node/src/runtime.rs:170` → `:562`, `:588` | Finality + vesting bridge read `now`. `now() == self.work`, a monotone cumulative-work counter (`block_work=1` pre-PoW), folded into `state_digest`. Replica-deterministic, **not** wall-clock. | cosmetic |
| **clock** | `onchain/noesis-core/src/lib.rs:415` | Fixed finalize decays with `now`; guest reads it via `env::read` and it **MUST** be consensus-sourced from a block header at deploy (inert pre-deploy). | moderate |
| **storage — BLOCKER** | `node/src/runtime.rs:101-133` | State is **in-memory only**: no sled/rocksdb/leveldb, no serde/bincode on state types, no save/load/snapshot. Process restart loses all chain state. By-design for the reference node; a blocker for persistent deployment. | blocker |
| **storage** | `node/src/runtime.rs:293-301`, `:625-636` | UTXO set is a plain `Vec<Cell>`; existence = O(n) linear scan, retirement = O(n) `retain`. Deterministic but unindexed. | moderate |
| **global-mut (reshape)** | `node/src/runtime.rs:107`, `:132`, `:536` | `pom`/`finalized_at` are std `HashMap`; validation uses std `HashSet`. Digest is safe (`pom.sort()` at `:161`, `finalized_at` excluded), but a no_std/zkVM `apply_block` **cannot use std HashMap/HashSet at all** — must become BTreeMap/sorted-Vec. | blocker (reshape) |
| **hashmap-order** | `node/src/lib.rs:8031`, `:2597-2638` | Three consensus-adjacent maps (`pom`, `NoveltyIndex.nodes`, `finalized_at`) are **all proven order-safe**: `pom` Vec-driven + sorted into digest; SMT `root()` position-hashed (`insertion_order_cannot_change_the_root`, `:8091`); `finalized_at` excluded. Regression test asserts bit-identical v8 across 32 fresh seeds (`:2597`). Known risk, actively guarded. | moderate |
| **other (cost)** | `node/src/runtime.rs:656-657` | `apply` re-folds `pom` over the **entire** `ledger.cells` every block (O(chain)/block, cross-cell similarity floor). A zkVM guest would re-prove history each block unless made an incremental bounded delta. Flagged by `ZK-INTEGRATION.md:32`. | moderate |
| **storage (structural)** | `node/src` is std (`use std::collections`) | The full transition + conservation + double-spend live in the **std** node crate; only verify-side slices are extracted to no_std. Extracting them is the bulk of Phase 1. | blocker (scope) |

---

## 5. Hash & signature inventory + zkVM-friendliness

| Primitive | file:line | On transition/finality path? | zkVM-friendly? |
|---|---|---|---|
| **blake2b-256** (`blake2b-ref` 0.3, personal `noesis-smt-v1`) | `onchain/noesis-core/src/lib.rs:121` | **YES** — SMT, Lamport, `tx_digest` (`:744`), pom_export commitment | ✅ tolerable, zkVM-friendly |
| **FNV-1a 64** (non-crypto shingle keys) | `onchain/noesis-core/src/lib.rs:79` | YES — coverage `CovId` = SMT keys (intake) | ✅ integer-only |
| **splitmix64** (deterministic PRNG) | `onchain/noesis-core/src/lib.rs:250` | YES — seeds commit-order `block_shuffle` (`:260`) | ✅ integer-only, no entropy |
| **Lamport** PQ one-time sig (personal `noesis-lamp-v1`) | `onchain/noesis-core/src/lib.rs:619` | Lock-script auth (inert pre-deploy, `runtime.rs:381`) | ✅ hash-based; not in a zk guest today |
| **SHA-256** (risc0 accelerator) | `onchain/zk-finalize/methods/guest/src/main.rs:23` | Only binds the guest journal (input digest) | ✅ risc0-accelerated |
| **keccak256** (`tiny-keccak` 2.0, OZ/EVM interop) | `node/src/pom_export.rs:202` | **NO** — JSON export layer only, OFF finality/ZK path; commitment uses blake2b not keccak | ⚠️ SNARK-unfriendly, but off-path |
| secp256k1 / ed25519 / BLS / schnorr | — | **absent from Rust** (grep-verified negative; only in docs + one `.py`) | n/a |

**Net:** everything on the transition/finality path is blake2b + FNV + splitmix64 + (deploy-time)
Lamport — all zkVM-friendly. The only unfriendly primitive (keccak) is confined to the host-side
JSON export and never re-enters a consensus value.

---

## 6. Environment & measurables (honest)

| Item | Value |
|---|---|
| Toolchain | `rustc 1.93.1 (01f6ddf75 2026-02-11)`; `cargo 1.93.1` |
| **STARK prover on this box?** | **NO.** Windows (win32); no WSL2 distro (`wsl --status`/`wsl -l -v` return only help text), no Docker (`docker --version` → not found), no `r0vm`/`rzup`/`cargo-risczero` on PATH. RISC Zero proving needs Linux/WSL2/Docker — none present. |
| Build | **PASS** — `cargo build -p noesis` → exit 0. ⚠️ **INCREMENTAL** (3.75s), not clean; no `cargo clean` first ⇒ cold-build time unknown. |
| **Test green count** | **349 passed / 0 failed / 0 ignored** — `cargo test -p noesis`. Breakdown: lib 285; byzantine 5; ckb_vm_adversarial 6; commit_order 6; finalization 10; locksig 12; pom_typescript 4; proven_e2e 8; smoke 1; syscalls 3; core_drift_guard 3; gaming 3; two_node 3; doc-tests 0. Wall-time ~1–1.5 min. |
| Parity harness (host-stable, no risc0) | RAN and GREEN: whale-alone REJECTED, whale+1 REJECTED, whale+2 FINALIZES; "anchors verified". Mix printed pow=0 / pos=1431655765 / pom=2863311531 (Q32.32, PoW-excluded). |
| **Initial-sync time** | **N/A** — no deployed/testnet chain. `node` is `[lib]`-only (no `[[bin]]`, no `main.rs`, no built executable), no P2P/RPC config (grep bootnode/rpc/p2p = 0). Only "chain" is an in-memory reference model in tests. |
| **UTXO-set size** | **N/A** — no persisted state. `data/` (31M) is `deepfunding/` research data only; no chaindata/rocksdb/db anywhere. The value-UTXO set exists only in-code as `token_cells: Vec<Cell>` (`node/src/runtime.rs:123`). |

No numbers were fabricated; both sync-time and set-size are honestly N/A with the reason stated.

---

## 7. Existing assets vs the ask (the gap)

**Assets that exist:**
- **`onchain/noesis-core`** — pure `#![no_std]` + alloc verify-core (`onchain/noesis-core/src/lib.rs:1-14`).
  Holds: Q16.16 floors, coverage/shingles, SMT verify+insert, proven-intake floor, commit-order
  (`:232-341`), **finalization in Q32.32** (`finalizes_pos_pom_fixed`, `:481`, re-verified this pass),
  Lamport (`:619`), `tx_digest` (`:702-853`), and **fixed-layout LE wire formats** (`:506-609`). Explicitly
  **excludes** maintainer-side state (`NoveltyIndex`, flow, settlement stay in the node).
- **`onchain/zk-finalize`** — a real, honestly-scoped RISC Zero PoC that proves **only** the
  finalization verdict for one checkpoint: guest decodes `(cell, votes, now)`, calls the **real**
  `finalizes_pos_pom_fixed`, commits `(sha256(inputs), verdict:bool)` (`.../guest/src/main.rs:17-48`).
  **Honest status:** parity harness GREEN ✅; risc0 guest **NOT compiled** 🟡; **NO verifying receipt
  produced** 🔬 (`onchain/zk-finalize/README.md:52-59`). README explicitly says do not claim ZK ships.
- **`docs/ZK-INTEGRATION.md`** — a Fit-1..4 plan (prove finalize floor / novelty floor / set-non-membership /
  selective disclosure). All items 🟡 designed / 🔬 unbuilt by its own admission. It is **function-level**;
  it does **not** propose proving `apply_block` over the full transition, nor recursion/rollup.

**The gap (explicit):**

```
  HAVE:  finalization-verdict PoC (one bool)  +  pure no_std verify-core (floors/wire/finalize)
                                       │
                                       ▼   ← the engagement
  NEED:  pure apply_block(state, block) -> Result   over the FULL transition
         (token conservation + double-spend retirement + coverage-index insert +
          full-chain PoM recompute + state_digest)
              → PROVEN by a zkVM guest
              → RECURSED / rolled up over block history
              → a sync artifact a light node can verify
```

The conservation / no-double-spend / coverage-insert / PoM-recompute logic — exactly the Phase-4
"rulebook" invariants — is **node-side std and un-extracted**; only `tx_digest` (the *identity* of a
movement, not the *value* invariant) is in the core today.

---

## 8. Phase-1 readiness assessment

**Size of the extraction to `apply_block(state, block) -> Result<state, violation>`:** **Medium, not
large.** The *logic* is already pure/deterministic/integer (§4 verdict); the work is mechanical
reshape, not algorithm design:

1. **Move std → no_std**: lift the transition body (`runtime.rs:612-658`) + conservation
   (`:535-550`) + `is_valid_in_ledger` (`:293-314`) + `tokens::*` into (or beside) `noesis-core`.
2. **Reshape signature**: `&mut self` in-place → value-returning `(state, block) -> Result`.
3. **Swap containers**: std `HashMap`/`HashSet` → `BTreeMap`/sorted-`Vec` (the digest already
   canonicalizes, but a no_std guest cannot link std hashers at all).
4. **Swap the ONE float rule if finality is proven in-guest**: replace the f64 finality reference
   with the existing bit-exact `finalizes_pos_pom_fixed` (`noesis-core:481`) — a swap, not a rewrite.
5. **Cost/tractability**: decide whether the O(chain) full-chain PoM recompute (`:656-657`) becomes a
   bounded per-block delta before it enters a guest (else each block re-proves history).

**Replay-parity strategy:** strong. There are on-VM integration test vectors and fixtures already:
`node/tests/fixtures/` holds `{commit-order,finalization,locksig,pom}-typescript` + `proof-of-mind-lock-script`
ELF/vectors; `node/tests/core_drift_guard.rs` already asserts `noesis-core` ≡ node lib on every
canonical fixture (`ENT=62259`, `SIM=52429`), and the finalization/commit-order/locksig integration
tests round-trip encode(host)→decode+verify(CKB-VM). The parity strategy for `apply_block` is to
extend this pattern: run the existing node `Node::apply` and the new pure `apply_block` over the same
fixtures + the `two_node.rs` convergence vectors and assert identical `state_digest`.

---

## 9. zkVM choice — preliminary input (Phase-3 decision, do not over-claim)

| | RISC Zero | SP1 |
|---|---|---|
| Already wired here | **YES** — `onchain/zk-finalize` guest runs the *real* `finalizes_pos_pom_fixed` unchanged; host prove+verify path exists | No in-repo integration |
| Accelerated primitive | SHA-256 (already used to bind the journal) | (comparable; not evaluated here) |
| Our hash workhorse | blake2b (not the accelerated one, but proof-tolerable) | same |

**Current lean: RISC Zero**, for one non-ideological reason — the guest already links and runs the
**real no_std core** (`finalizes_pos_pom_fixed`), so extending from the finalize slice to `apply_block`
reuses the exact same extraction discipline, and the parity harness is already GREEN. This is a
**Phase-3 decision**; SP1 is not disqualified. No claim is made that RISC Zero is faster/cheaper for
the heavier novelty path — that is unbenchmarked (no prover on this box). Treat this as a default to
revisit once a receipt is produced on Linux/CI.

---

## 10. Pragma Coherence — Phase-4 fit note

Pragma certifies, **from the outside**, that an agent's actions stayed inside a declared **intent
manifest** — a Coherence License / external verifier over the action graph ("did the agent stay
inside its declared intent," Proof-of-Intended-Execution) (`docs/Pragma Overlaps/noesis-pragma-overlap.md:9`).
The doc's own honest distinction (`:30`): OPH's integrity model is **redundancy/code-distance** based
(many observers, correctable), while Noesis's is **cryptographic/hash-rooted** (collision-resistance).

**Does it map onto the UTXO invariants (value conservation, no-double-spend, determinism)?** **No —
it is a complementary axis, not a substitute.** Pragma is an *authorization / intent-coherence* axis;
value-conservation and no-double-spend are Noesis-native UTXO invariants (a *credit/value* axis). The
overlap doc itself frames them as two halves of a stack, not interchangeable. The real overlap sits at
the "proof-of-intended-execution / fixed-point of a disagreement-lowering dynamic" level (`:20-28`,
`:38-42`) — a convergence-theory bridge, not a UTXO-invariant checker.

**Recommendation:** **use where it fits, don't force.** For Phase-4, the value/conservation/determinism
invariants belong in **Isabelle/HOL theorems** over the extracted rule-set; Pragma Coherence is an
appropriate *complementary* layer for rule-set-mutation coherence (confluence / axiom-preservation on
governance changes to the rulebook), **not** a drop-in for the UTXO invariant proofs.

---

## 11. Boundaries (must appear)

A validity receipt proves **VALIDITY ONLY**. It does **not** provide:

1. **Canonicality** — a *valid* fork is still a fork. Recursive validity proofs say "this chain of
   transitions is internally valid"; they say nothing about *which* valid chain is canonical.
   **Fork-choice is untouched** by this engagement (finality/NCI selection remain the node's job).
2. **Data availability** — a node convinced of a state *root* still needs the underlying *data* to
   act on it. A proof that state root R is valid does not deliver the cells behind R.

**No claim of "trustless full verification" is made without naming both gaps.** The deliverable of
this engagement is *stateless validity verification of the transition function*; canonicality and DA
remain separate, unclosed problems.

---

## Appendix — file:line a finder could NOT verify (honesty)

- **`pom_scores_with_similarity_floor_q16` body / O(chain) claim** — inferred from the call site
  "over `self.ledger.cells`" (`runtime.rs:657`), not from reading the function internals (existing-zk finder).
- ~~**`temporal_novelty_with_similarity_floor_q16` end-to-end float-freeness**~~ — **VERIFIED this pass
  (2026-07-12, post-synthesis):** body read at `lib.rs:6514-6531`. Pure `u128` cross-multiply
  `(overlap << Q) > (theta_q16 as u128) * len`, `.count()`, no `f64`, no division. `HashSet` used only
  for membership/count (order-invariant). The consensus PoM path is float-free — CONFIRMED, not inferred.
- ~~**SMT `insert`/`root` byte-order-invariance**~~ — **VERIFIED this pass (2026-07-12, post-synthesis):**
  `insert` read at `lib.rs:8070-8083`, `root` at `:8051`. Positional-prefix `blake2b` combine
  (`prefix & 1` selects left/right); `root()` returns the fixed node `(DEPTH, 0)`, so the backing
  `HashMap<(usize,u64),Hash>`'s iteration order cannot affect the root. Root commits to the SET of
  keys, order-independent — CONFIRMED at source, not just via the `:8091` test. (There is no separate
  `smt.rs`; the SMT lives inline in `lib.rs`.)
- **On-VM type-scripts actually link the fixed core** — `finalization_fixed` re-export confirmed
  (`lib.rs:8524`), but the type-script crates that *consume* it were not opened; 🟡 designed/forward-parity,
  not confirmed-built (consensus-finality + serialization finders).
- **`value_v7` interior floats** — read only through signature/start (`lib.rs:~1179`); float
  characterization relies on doc + its Q32 twin (pom-scoring finder).
- **f64 collusion diagnostics off every consensus path** — `attribution_cycle_energy` / `collusion_residual_by_identity`
  / `solve_psd_cg` confirmed f64 and not called from `runtime.rs`, but not grep-proven absent from
  *every* consensus path anywhere (pom-scoring + purity finders).
- **No round-trip unit test in `noesis-core` wire module** — no `encode(parse(x))==x` assertion inside
  the core; round-trip stability inferred from hand-written inverses + host→VM integration tests
  (serialization finder).
- **`Op` state-machine call site** — no wiring of soulbound `Op` transitions into the block loop was
  found; if one exists elsewhere it was not seen (block-tx-validation + existing-zk finders).
- **Cold/clean build time** — build was incremental only; no `cargo clean` baseline (env finder).
- **Executed proof / receipt** — no build or proof was run; parity-GREEN and finalize-slice claims
  rest on source reads + README, not on an executed receipt (all finders; no prover env on this box).
