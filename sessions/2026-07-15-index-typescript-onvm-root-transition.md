# 2026-07-15 — The index cell learns to police itself (on-VM root-transition ELF)

Plain-English recap. One shipped unit. HEAD == origin == `34ee644`.

## What we built, in one breath

Every contribution to Noesis is checked for novelty against a running list of everything the
network has already seen — the "seen-set," stored as one 32-byte fingerprint (a Merkle root) inside
a special cell. Last session we confirmed the *reading* half already works on-chain: when you submit
a contribution, the on-VM intake script reads that fingerprint and zeroes out anything too similar to
what's been seen.

But there was a hole: **nobody on-chain was checking that the fingerprint itself was updated
honestly.** The intake script trusted the seen-set root. A dishonest block producer could hand out a
doctored root — quietly dropping a rival's contribution from the "seen" list so they could re-mint
it, or padding it to censor someone.

This session closed that hole's *reference-to-on-VM* gap. We wrote a new tiny on-chain program
(`onchain/index-typescript`, ~70 lines) that guards the seen-set cell and refuses to let its
fingerprint change unless the change is provably honest: the root may only move from old to new as an
exact, correctly-ordered chain of single insertions, each one mathematically proven against the
running fingerprint at that moment. Drop a key, sneak one in, duplicate one, reorder to favor
yourself, or forge a proof — the program rejects it. It shares the *exact same* rule code the node
already uses (single-sourced, so the two can never drift apart).

## Why it's trustworthy (not just my say-so)

- It runs the identical `valid_ordered_root_transition` the node has been drift-guarding for weeks
  (8 reference tests still green).
- 9 new end-to-end tests actually execute the compiled RISC-V program inside the VM and check every
  attack — honest transition accepted, dup/omit/smuggle/forge/reorder all rejected — and confirm the
  on-VM verdict matches the node reference every time.
- We ran the adversarial pass **before** shipping and it caught a real bug: the byte-decoder trusted
  a length field from attacker-supplied data and would have tried to reserve terabytes of memory
  (crashing the VM) on a malformed input. Fixed so it now rejects cleanly, plus a regression test.

## The honest limit (say it plainly)

Today the program proves the *math* of an honest update, but it still reads the "old" and "new"
fingerprints from the data it's handed rather than from the real cells on-chain. Wiring it to the
actual input/output cells (and to consensus-sourced timing) is deploy-coupled — it needs the live
chain plumbing. That's fenced behind an inert `PROVENANCE_BOUND` switch and honestly labeled
"pinned, not yet enforced," exactly like the sibling ordering and finalization scripts. Same posture,
same honesty. This is a real brick in the wall, not the finished wall.

## What's next

The seen-set is now on-VM on both halves (read: done; write: this). The remaining L6 fronts are
deploy-coupled: flip the provenance bindings, the registry binding, and single-use/nullifier crypto —
then the L7 genesis/P2P deploy pole to a public testnet. The north star is unchanged: get online.
