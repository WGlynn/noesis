# T7 — Cross-cell similarity floor ON-VM (design, PRIVATE)

> ROADMAP execution-tier T7. Design-first increment (2026-06-12 PM): the mechanism is
> chosen and adversarially walked here; code lands in the ordered increments at the end.
> Status: DESIGN, no code yet.

## The problem

Intake floors split by what they read:
- **Content-local** (semantic floor): pure function of the cell's bytes — ON-VM since T4.
- **History-dependent** (temporal novelty + similarity floor): need `seen` = the union of
  coverage shingles of every previously committed cell, in commit order. Global, monotone,
  unbounded. A type-script sees ONE transaction.

## Rejected approaches (and why)

1. **Serve the whole seen-set via syscall** — O(history) bytes per validation; fails both
   the cycle budget and the state-rent economics the chain is built on.
2. **Host computes novelty, script reads the answer** — the script stops validating and
   becomes a rubber stamp; authority silently moves off-VM. Violates the T-tier's purpose.
3. **Witness-supplied seen-set** — the prover lies by omission: drop the shingle that
   makes the cell redundant, collect fake novelty.

## The mechanism: committed index + complete per-shingle classification

CKB-native shape — this is what the cell model is for:

- **NOVELTY-INDEX CELL**: a consensus-maintained cell whose data is a sparse-Merkle-tree
  (SMT) root over the seen-shingle set (blake2b, fixed depth; shingle ids key into the
  tree). The index cell's OWN type-script validates every root transition.
- **Commit tx shape**: consumes the index cell + produces it with the updated root. The
  witness carries, for EVERY shingle in the committed cell's coverage, a proof against
  the consumed root — membership for overlap shingles, non-membership for novel ones.
- **PoM type-script** (extends T4-T6): computes the coverage list from the cell bytes
  ON-VM (deterministic — the script knows the complete shingle list and demands a proof
  per shingle), verifies each proof, and derives EXACT counts:
  `novelty = #valid-non-membership`, `overlap = #valid-membership`. Floors run on those
  verified counts — same Q16.16 comparisons as `value_fixed`.
- **Index-cell type-script**: verifies new root = old root with exactly the proven-novel
  shingles inserted (SMT insertion proofs compose), so the set can never be pruned,
  forked, or selectively forgotten.

## Adversarial walk (design-time tick)

- **Omission**: can't — the script derives the full coverage list itself; every shingle
  must carry a valid proof one way or the other against one root. Proving non-membership
  of a member fails; proving membership of a non-member fails. Classification is
  COMPLETE ⇒ counts are exact.
- **Stale root**: can't — the index cell is CONSUMED by the commit tx; UTXO liveness is
  root freshness. Two txs cannot both consume it.
  - Honest cost, pinned: the index cell serializes every commit (throughput bottleneck).
    Mitigation: shard by shingle-prefix into 2^k index cells; a tx touches only the
    shards its shingles land in. Calibration item, not a correctness item.
- **Novelty front-running** (see a pending tx's shingles, commit first): out of scope
  here BY COMPOSITION — commit-reveal provenance is a core pillar of the chain layer;
  reveals order novelty, not mempool observation.
- **Cycle cost**: per shingle one SMT path = O(depth) blake2b. Large cells ⇒ thousands
  of shingles ⇒ budget pressure. Honest answer = SMT MULTI-proofs (sorted keys share
  path prefixes; batch verification compresses heavily). Sampling is rejected — it
  breaks exactness, and exactness is what defeats omission. Pinned as calibration.

## Authority split (unchanged)

Flow/v5-v7 settlement stays OFF-VM (graph-global, T8's Q32.32 path). T7 moves only the
intake-time history floors on-VM. The learned model stays role-bounded (`evaluator`).

## Ordered code increments

1. **SMT in `node`** (blake2b, deterministic, no_std-compatible core): membership +
   non-membership + insertion-update proofs, tested off-VM. No new deps beyond a blake2b
   already proven in the vibeswap recipe (blake2b-ref).
2. **`novelty_with_proofs(cell_bytes, root, proofs) -> (novelty, overlap)`** — one shared
   verifier shape used by host tests and the script (mirrors the value_fixed pattern:
   same arithmetic both sides of the VM boundary).
3. **Index-cell type-script rule** — root-transition validation (consume/produce).
4. **PoM script extension + host syscalls** — witness-served proofs
   (`load_witness_args` path exists in ckb-std), index root served like cell data;
   end-to-end test = the T4-T6 pattern: same verdicts host-side and on-VM.
