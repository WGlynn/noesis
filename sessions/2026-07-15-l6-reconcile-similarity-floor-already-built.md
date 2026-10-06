# 2026-07-15 — L6 reconcile: the "on-VM similarity floor" was already built; the real gap is the index-cell transition twin

## What happened (plain English)

Last session teed up "build the on-VM similarity floor" as the confirmed next task. Before
building it, I checked it against the actual code and tests (the standing lesson: Noesis docs
keep understating progress, so verify HEAD before building). **It's already built and tested.**

The similarity floor is the rule that stops someone farming standing by submitting near-copies
of existing work: if a new contribution overlaps too much with what the chain has already seen,
its value is zeroed. On-VM, `pom-typescript` already does this — it reads the "seen" set as a
committed root (a cell dependency), counts how much of the new cell overlaps it, and zeroes the
whole cell past the threshold. Three end-to-end tests already prove it inside the VM (an exact
duplicate, a near-twin, and a two-outputs-in-one-transaction trick — all rejected with exit 22).

So the task as written was done. The file's own header comment and the handoff both still said
"not built yet" — stale docs, the third time this pattern has shown up. **I fixed both.**

## The genuinely missing piece (corrected scope)

The "seen" set has two halves:
- **Read side (done on-VM):** given the committed root, floor the new cell. ✅
- **Write side (NOT on-VM):** prove the root itself is honest — that it only ever grows by
  inserting genuinely-new shingles, in the real commit order (so nobody can hand the floor a
  doctored "seen" set). That logic exists in the node reference (`index_rule::valid_root_transition`
  and `valid_ordered_root_transition`) but has no on-VM program. There's no `index-typescript`
  in `onchain/`.

Right now the on-VM floor *trusts* whoever serves the root. The missing program is what makes
that trust unnecessary.

## What's committed

- `75eab46` — comment-only correction to the `pom-typescript` header (was actively wrong in
  shipped code). HEAD == origin/master. Nothing else touched (Will's dirty do-not-sweep set left
  alone; CONTINUE.md top block corrected in place, uncommitted per convention).

## Next session (fresh, low-context, RED-first — the on-VM crypto note applies)

Port `index_rule` to an on-VM ELF, mirroring `finalization` → `finalization-typescript`:
1. Single-source `index_rule` (+ `InsertStep`/`CellBatch` + a wire codec) into `noesis-core`.
   Its `leaf`/`root_from`/`verify_insert` and `commit_order` are already there; node re-exports
   the moved module (`pub use noesis_core::index_rule::…`) and keeps a drift-guard test.
2. Scaffold `onchain/index-typescript`, link the core rule, serve old-root/new-root/steps via
   syscalls, RED-first host harness (the T4–T6 / e2e pattern).
3. THE LOOP: 2 planners → build → Council (read-only Explore agents) → Pragma-confluence.
