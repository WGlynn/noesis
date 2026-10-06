# 2026-07-14 (evening) — the clock, the ramp we deleted, and sub-blocks

Plain-English recap. Six things shipped, all pushed (HEAD `ee9d4f3`).

## What we set out to do
Finish the "ordering tier" of the chain — the 120-second blocks that carry finality — and then start
sub-blocks, the fast lane on top. You ratified the timing numbers up front (2-minute blocks, and the
tolerances that hang off that), and we went.

## 1. The clock now actually enforces itself (CLK-2)
Before today the block timestamp was just carried along, ignored. Now, when enabled, the chain checks
that each block's timestamp doesn't run backwards and is baked into the proof-of-work (so a miner can't
solve a block and then quietly change its time). The "is this timestamp too far in the future?" check is
kept on each node's own wall-clock and deliberately kept *out* of the replay path — that's how Bitcoin
splits it, and it's the safe way. A nice catch mid-build: I first wrote the rule as "strictly increasing,"
but the existing kernel deliberately allows two blocks in the same second — I caught the contradiction by
reading the kernel and matched it.

## 2. We deleted the emission ramp — your call, and it was the right one
The design doc said JUL should start miners at a low reward and ramp up over the first week, to avoid an
early "land grab." **You pointed out this is solving a Bitcoin problem that JUL doesn't have.** Bitcoin's
early coins are mined nearly free (fixed reward, low difficulty) and become hugely valuable later — that's
"deep capital," and it concentrates wealth. JUL is *elastic*: reward is proportional to work, so one JUL
costs the same energy in block 1 as in block one-million. No cheap early coins, no land grab. Worse, a ramp
would have *broken* the one property that makes JUL honest money — that identical work always mints
identical JUL. So the ramp is gone, the flat reward we already had is correct and final, and I wrote the
reasoning into memory so nobody re-derives the mistake. (This is the kind of tokenomics error that has no
undo button, so catching it in design mattered.)

M2 (genesis JUL issuance) turned out to be essentially *done already* once the ramp was dropped — the
issuance machinery was all built; the only leftovers need a GPU benchmark at build-time or a live chain.

## 3. Sub-blocks — the fast lane (this is the big new thing)
The 2-minute cadence is a gift to small, cheap nodes (low bandwidth, low storage — good for
decentralization). But 2 minutes is slow for a user waiting on a payment. **Sub-blocks pay for that
tradeoff:** every ~2 seconds, a batch of value transactions gets a fast, *revertible* "soft confirmation,"
while real settlement still happens on the 2-minute block. You asked whether sub-blocks should be
transactions-only (not contributions) — yes, and I'd already built it that way: a contribution's value
matures slowly anyway, so there's nothing to fast-confirm.

The security story is the reassuring part: because sub-blocks never touch finalized state and the fast lane
carries no finality weight, **even a completely hacked fast lane can only make soft-confirmations
unreliable — it cannot break settlement.** Same shape as "a broken clock slows the chain but never makes it
unsafe." I wrote that up as a proper security paper (7 threats, honest open questions).

Then, while you were away, I built the **absorption** step: how the 2-minute block "absorbs" the fast-lane
transactions and turns them from "soft" to "final." The key property — that two honest nodes absorb in the
exact same order — is now built and tested end-to-end (a payment goes soft → gets absorbed → goes final).

## What's waiting on you
- **One real design fork:** when the 2-minute block absorbs the fast lane, does it *replace* its own
  pending transactions with the fast-lane ones, *merge* them, or *prioritise*? I deliberately did NOT guess
  this one — it's the kind of choice you've been steering all session.
- A couple of numbers/knobs the security paper flagged (the standing threshold to propose sub-blocks; the
  "wait N blocks for large payments" guidance).

## What I could do next without you
Wire sub-block gossip (how they spread between nodes) + Ergo's compact 6-byte transaction IDs for
bandwidth. Decision-unblocked, but I stopped here rather than push further into networking on my own.
