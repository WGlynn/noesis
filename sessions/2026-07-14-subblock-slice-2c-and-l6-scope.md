# 2026-07-14 — Sub-blocks finished, and we figured out exactly what's left before launch

Plain-English recap so the day-to-day progress doesn't get lost.

## What we did

**We finished the sub-blocks.** Sub-blocks are the fast "your payment showed up in ~2 seconds"
tier that sits under the slower 2-minute settlement blocks. The design was already done and
tested; the last structural piece was to actually put the sub-block's fingerprint *into the block
header* so the proof-of-work signs over it. We did that today.

Concretely: a block now carries a `subblock_root` — a single 32-byte fingerprint of all the fast
transactions it absorbed. It's bound into the part of the block the miner's work commits to. The
practical effect: once a block is mined, nobody can quietly swap out which fast-transactions it
promised — doing so changes the fingerprint, which changes the header, which invalidates the mining
work. It's the same "optional extra field, invisible until used" trick we already use for the
coinbase, the proof-of-work seal, and the timestamp, so old data still reads back perfectly.

One test, everything green (328 in the main suite, plus the sub-block, parity, wire, and sync
suites), no new warnings. Committed and pushed as `2c3d5a1`.

**We locked two remaining sub-block knobs as "tune when live."** There are two settings we
deliberately did NOT nail down: (1) how an ordering block mixes the fast-chain into its own list of
transactions, and (2) how much contribution-standing a producer needs to propose a fast block. The
honest call: these can't be calibrated on paper — they're the kind of number you set once real
traffic exists and adjust. Leaving them open is correct sequencing, not procrastination. The chain
is perfectly launchable at 2-minute blocks (Litecoin's cadence) with sub-blocks turned off entirely;
they're a user-experience upgrade, not a launch requirement.

## The honest launch answer

The consensus machinery — the actual rules of the chain — is essentially built and passing tests.
The "cold builds" (new mechanisms) are drained. What stands between us and a live network is now
almost entirely the **deploy pole**: porting a couple of enforcement checks to run *inside* the
on-chain VM, standing up a real public network (genesis config + peer-to-peer + a hosted seed node),
and getting a Linux environment for the one zero-knowledge piece this Windows box can't build.

So the honest sentence is: we're past "is it designed and built" and into "stand up the network."
That's deploy engineering measured in weeks, paced by the networking build and one environment fix —
not by more consensus work, and not a date we're going to fake.

## Next window

Build the **on-VM similarity floor.** Background: the chain rewards genuinely novel contributions
and rejects near-duplicates. That "is this too similar to something already seen?" check runs today
in the reference node but does NOT yet run inside the on-chain VM (the on-VM program's own comments
flag this as the missing piece). Everything around it — the soulbound identity check, the "is this
just noise?" check, the novelty proof — already runs on-VM. This is the clean, well-scoped next step,
and it reuses a pattern we've already built (serving a reference fingerprint to the VM as a
cell-dependency). We'll build it fresh next session, test-first, with the full review loop.
