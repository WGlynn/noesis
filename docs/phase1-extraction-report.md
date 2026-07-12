# Phase 1 Report — The Extraction (pure `apply_block`)

> Stateless-verification engagement. Phase 1 deliverable: the load-bearing refactor to a pure,
> deterministic state-transition function, with **replay-parity evidence**. Read `docs/rulebook-map.md`
> (Phase 0) first. Every number below came from a command actually run on this machine; the command
> is shown. No mocked results.

## What Phase 1 required

> *"Extract the state-transition function: `apply_block(state, block) -> Result<state, violation>`,
> pure and deterministic — no I/O, no clock, no randomness, no floats. Same inputs, same output,
> byte-for-byte, forever. The existing node becomes a caller of this function; consensus behavior must
> be bit-identical before and after (prove it by replaying vectors through old and new paths and
> diffing state roots). Do not proceed until replay parity passes."*

## What was built (commit `592d66e`)

A single pure rulebook in `node/src/runtime.rs`:

| Item | Signature | Purity |
|---|---|---|
| `apply_block` | `(state: Ledger, b: &Block, params: &Constitution) -> Result<Ledger, Violation>` | **owned-in / owned-out**; validate-then-transition; no I/O, clock, rand, or float |
| `validate_block` | `(&Ledger, &Block) -> Result<(), Violation>` | the 6 acceptance checks, returning a **typed reason** instead of a bare `bool` |
| `apply_transition` | `(&mut Ledger, &Block, &Constitution)` | the **exact** old `Node::apply` body, threaded explicitly; `&mut` is a zero-copy detail of `apply_block` |
| `Violation` | `enum` (6 variants) | one per acceptance check — the `Result` payload Phases 3–4 consume |

`Node::validate` and `Node::apply` are now **thin callers** of these functions, so consensus behaviour
is defined in exactly one place. `Node::apply` still routes through `apply_transition` (no validation),
so its behaviour is byte-identical to before the extraction; `apply_block` is the new combined entry
the commitment layer, zkVM guest, and formal spec will all consume.

### Why this was an *extraction*, not a rewrite
Phase 0 verified at source that the transition (`runtime.rs:612-658`) was **already** pure: no clock
(only the derived cumulative-work counter), no rand (only seeded `splitmix64`, off this path), no
fs/net/DB/env, no mutable globals, and no `f64`. The q16 PoM path (`lib.rs:6514`) is integer-only and
the novelty SMT root is order-independent — both **read and confirmed**, not inferred. So Phase 1 moved
and reshaped code; it did not untangle I/O or de-float the transition.

## Replay-parity evidence (the gate)

`node/tests/apply_block_parity.rs` replays the `two_node.rs` convergence vectors (5 cells across 3
blocks, with provenance edges ⇒ non-trivial PoM) through **both** paths and diffs the full state tuple
after every block:

- **Old path:** `Node::apply` (now delegating to `apply_transition`).
- **New path:** fold `apply_block` from an empty `Ledger`.
- **Asserted equal each block:** `state_digest()` = `(cell-id sequence, novelty-index root, sorted PoM
  map, token-cell sequence, cumulative-work clock)` — the full consensus-comparison tuple.
- Plus: rejections surface as the correct typed `Violation` (`HeightMismatch`, `EmptyBlock`,
  `NonCanonicalOrder`).

```
$ cargo test -p noesis --test apply_block_parity
test apply_block_surfaces_typed_violations ... ok
test apply_block_matches_node_apply_over_vectors ... ok
test result: ok. 2 passed; 0 failed
```

Full-suite regression (proves the extraction changed nothing across the whole node):

```
$ cargo test -p noesis
lib: 285 · apply_block_parity: 2 · byzantine: 5 · ckb_vm_adversarial: 6 · commit_order: 6 ·
finalization: 10 · locksig: 12 · pom_typescript: 4 · proven_e2e: 8 · smoke: 1 · syscalls: 3 ·
core_drift_guard: 3 · gaming: 3 · two_node: 3
TOTAL: 351 passed / 0 failed   (was 349 + the 2 new parity tests)

$ cargo clippy -p noesis --tests   # exit 0; 0 new warnings in the changed code
```

**Replay parity PASSES. The Phase-1 gate is met.**

## Honest scope / what Phase 1 deliberately did NOT do

- **Still `std`, still in the `node` crate.** Phase 1 delivered *purity + parity*, not the `no_std`
  lift. The `HashMap`/`HashSet` → `BTreeMap`/sorted-`Vec` container swap and the move into
  `noesis-core` are **Phase-3 prerequisites** (a zkVM guest cannot link std hashers), staged next as
  their own parity-guarded step. The state is deterministic today; the containers just aren't
  no_std-linkable yet.
- **The O(chain) PoM recompute remains** (`apply_transition` re-folds `pom` over the whole chain each
  block). Correct and deterministic, but a zkVM guest would re-prove history every block — this must
  become a bounded per-block delta before Phase 3 (flagged in the map §8.5).
- **Token-path parity is covered transitively**, not by a bespoke vector: `Node::apply` and
  `apply_block` share `apply_transition`, and the existing suite (token tests, `gaming`, `two_node`)
  drives token apply through `Node::apply`. A dedicated token-movement parity vector is a cheap add.

## Boundaries (restated every report)

A validity receipt — the eventual Phase-3 artifact this rulebook feeds — proves **VALIDITY ONLY**. It
does **not** prove **canonicality** (a valid fork is still a fork; fork-choice is untouched) or **data
availability** (a node convinced of a state root still needs the data). No artifact in this engagement
claims "trustless full verification" without naming both gaps.

## Next: Phase 2 — Compact State (UTXO commitment, no ZK)

Goal: an audited-accumulator commitment to the UTXO (`token_cells`) set, updated per block, with KB
membership proofs and an assumeutxo-style checkpoint bootstrap.

**Decision to make first (crypto-honesty constraint):** the engagement rule is *"all accumulator/Merkle
code uses audited libraries; never hand-roll crypto."* The repo already contains a hand-rolled sparse
Merkle tree (`NoveltyIndex`, blake2b, order-independent root, membership + non-membership proofs) used
for the novelty index. Two honest options for the UTXO commitment:

- **(A) Reuse the in-repo SMT** for `token_cells` — zero new deps, consistent with the live novelty
  index, but it is *hand-rolled* (tension with the audited-library rule; would need disclosure).
- **(B) Introduce an audited external accumulator** (audited sparse-Merkle crate, or a Utreexo-style
  library) — satisfies the constraint cleanly, at the cost of a new dependency and a second Merkle
  construction to carry into the no_std/zkVM path.

This is a genuine trade-off (crypto-honesty vs. codebase consistency) and is surfaced for a decision
before Phase 2 builds. Recommendation pending that call.
