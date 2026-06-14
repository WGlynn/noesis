# On-chain temporal ordering (PRIVATE) — the consensus-sourced "earlier" relation

> The fix for the temporal-order attacker-choosable-input finding
> (`SECURITY-AUDIT-attacker-choosable-inputs.md`, applying `[P·dont-let-attacker-choose-critical-input]`).
> Reference model SHIPPED + tested (node 166/166); the on-VM enforcement is specified here and
> sentinel-gated inert pre-deploy, exactly like the index-dep binding and the finalization `now`.

## The finding (recap)
`temporal_novelty` and the index `valid_root_transition` assign shared novelty by ORDER: the
earlier-committed cell wins the contested coverage, a later redundant cell earns 0. That is
strategyproof ONLY if "earlier" is a relation the block producer cannot arrange. The rules trust
their caller's slice / step order. Correct for a reference model, but it RELOCATES the invariant
to the source of that order. The defensive audit verified `Cell.timestamp` is never read (so
backdating it is a no-op) and narrowed the real requirement: the ON-CHAIN path must source order
from consensus, never from a producer-arrangeable list or a self-set field.

## The fix — dissolve producer-favorable ordering at two scales
Ordering is made attacker-unreachable at both scales, so there is no commitment a rational
producer can pick to earn an earlier slot. This DISSOLVES the class (`[P·class-dissolution-vs-case-defeat]`)
rather than detecting instances.

- **INTER-block: commit-reveal BLOCK HEIGHT.** A later height can never precede an earlier one.
  The self-set `timestamp` is not the lever; the height the commitment landed in is.
- **INTRA-block (same-height ties): Fisher-Yates seeded by the XOR of EVERY revealing participant's
  secret** — the VibeSwap `DeterministicShuffle` primitive. A participant commits before any secret
  is revealed; their slot depends on the XOR of all secrets; so no party can predict, let alone
  choose, their own position. (Same MEV-resistance mechanism VibeSwap uses for commit-reveal batch
  auctions — cross-substrate coherence, `[P·substrate-geometry-match]`.)

## Reference model — SHIPPED (node 166/166)
`node/src/lib.rs`:
- `pub mod commit_order` — `Committed{height, secret}`; `block_shuffle` (canonical base order by
  secret, then XOR-seeded splitmix64 Fisher-Yates, presentation-INDEPENDENT by construction);
  `canonical_order` (height ascending, then in-block slot); `is_canonical_order` (the index-cell
  type-script ASSERTS this and REJECTS a non-canonical batch rather than silently re-sorting, so a
  producer learns nothing by probing).
- `novelty_in_commit_order(cells_presented, coords)` — the value-layer fix: sorts canonically,
  runs `temporal_novelty`, keys values back to presentation order.
- Tests (5, all green):
  - `cross_block_height_dominates_presentation` — redundant block presented FIRST still earns 0.
  - `canonical_order_is_invariant_to_presentation` — same set, any presentation, same sequence;
    heights non-decreasing.
  - `intra_block_slot_is_not_self_selectable` — a fixed attacker secret lands in >1 slot as the
    others' secrets vary ⇒ co-determined, not chosen. (The load-bearing dissolution property.)
  - `block_shuffle_is_deterministic_and_total` — consensus-replayable permutation.
  - `is_canonical_order_rejects_a_reordered_batch` — producer-favorable reorder rejected.

## On-VM enforcement — SPEC (deploy-coupled, sentinel-gated like index-dep / finalization)
The index-cell type-script (`valid_root_transition`, currently node-only per repo convention)
gains an ordering precondition on the batch it commits:

1. **Per-cell commit coords come from consensus, not the witness.**
   - `height` = the commit-reveal block height, from the header (`load_header` → since/epoch), NOT a
     tx-assembler field. This is the SAME `now`-from-header rule as `ON-VM-FINALIZATION.md` §3.
   - `secret` = each participant's revealed commit-reveal secret for this block, served from the
     block's reveal set. The XOR seed is over the WHOLE block's reveals, so it cannot be finalized
     until reveal closes — which is exactly why no committer can pre-compute their slot.
   - **RE-DERIVE, don't trust (the load-bearing closure).** The ELF must RECONSTRUCT each cell's
     `(height, secret)` from consensus — the header the cell's commitment landed in, plus the reveal
     keyed to that commitment hash — and REJECT any cell whose CLAIMED coord differs from the
     re-derived one. Without this, sourcing is decorative: the host-side `valid_ordered_root_transition`
     trusts the coords AS GIVEN, so a redundant cell that claims a falsely-earlier `height` sorts
     first and banks the contested novelty (pinned: `ordered_rule_trusts_coords_so_they_must_be_consensus_sourced`,
     node 196). "Comes from consensus" is only true if the ELF refuses any coord it cannot itself derive.
2. **Assert canonical order before applying the transition.** The batch's per-cell steps must be
   grouped in `canonical_order` of their coords; `is_canonical_order` false ⇒ reject (new exit
   code, distinct from the transition-validity exits). First-commit-wins then cannot be gamed by
   batch arrangement, because the arrangement is consensus-fixed.
3. **Sentinel-gated inert pre-deploy.** Until the commit-reveal/header wiring deploys, an all-zero
   sentinel selects the legacy shape path (caller-ordered), so existing fixtures stay green — the
   same pattern as `EXPECTED_INDEX_CODE_HASH == [0;32]` in the index-dep binding.

## Composition
- **Index-dep F3 binding** (`INDEX-DEP-CODEHASH-BINDING.md`) already pins the cell to the CANONICAL
  head root, so a redundant block cannot prove against a stale pre-original root across blocks. This
  doc adds the INTRA-block ordering source the F3 binding does not constrain.
- **Batched root transition** (`index_rule::valid_root_transition`) already makes first-commit-wins
  executable via evolving intermediate roots; this doc fixes WHICH order those steps must be in.
- Recurring principle, NOW SEVEN SITES (`[P·dont-let-attacker-choose-critical-input]`): do not let
  the attacker choose the security-critical input — `code_hash` (index identity) / finalization-`now`
  / temporal-order / index-dep binding / finalization validator-set / commit-ORDER / commit-COORD
  derivation (here). The on-VM rule is uniform: the ELF re-derives every one of these from consensus
  and rejects anything it cannot reconstruct. This doc is the commit-order + commit-coord sites.

## Honest status
- DEMONSTRATED (host reference model, node 196): presentation-invariance; the co-determination
  (non-self-selectable slot) property; the ordered index rule (`valid_ordered_root_transition` —
  producer reorder rejected at the order gate); AND the forged-coord pin
  (`ordered_rule_trusts_coords_so_they_must_be_consensus_sourced` — a falsely-earlier claimed height
  steals novelty, proving the coords must be consensus-derived). The permutation core is also ported
  to `noesis-core` (no_std, builds riscv64imac) ready for the ELF to link.
- DEMONSTRATED (on-VM, 2026-06-13): the **ORDERING RULE now runs inside the VM** —
  `onchain/commit-order-typescript` (riscv64imac ELF) reads the presented batch, runs
  `is_canonical_order` (single-sourced from `noesis_core::commit_order`), and exits 0 canonical /
  40 non-canonical / 41 malformed. 6 e2e cases (`node/tests/ckb_vm_commit_order.rs`): canonical
  accepted, reversed/cross-block-descending rejected, on-VM ≡ reference across presentations.
- NOT YET (deploy-coupled): COORD PROVENANCE — re-deriving `height` from each cell's commit-block
  header and `secret` from the block's reveals, and rejecting any claimed coord. Gated INERT behind
  `COORDS_BOUND` pre-deploy (needs the commit-reveal block plumbing live), honestly deferred like the
  index-dep activated path and the finalization registry binding. The order rule is enforced on-VM;
  the provenance of the coords it orders is the remaining activated path.
