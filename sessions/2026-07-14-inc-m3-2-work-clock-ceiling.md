# Session recap — 2026-07-14 (AM, autonomous): inc-M3-2 work-clock ceiling

Plain-English recap. Second increment of the same session (after the inc-M3-3 adversarial fix).

## The one-sentence version

Added a safety cap so that a single unusually-hard-mined block can't secretly fast-forward the
chain's internal clock and prematurely "mature" everyone's pending contribution scores. HEAD is now
`62a0f4b`, pushed.

## The problem it closes

Noesis has one internal clock (`now`), measured in cumulative mining work. A key safety rule reads
off it: a contribution's score only becomes usable for *finalizing blocks* after it has survived a
dispute window (`now − vesting_w`). That waiting period is what forces gamed value to sit exposed to
challenge before it counts.

The hole: mining a single enormous-difficulty block advances that clock by a huge jump all at once.
That one block could push the clock past the "matured" line for *every* pending contribution
simultaneously — collapsing the dispute window and letting gamed scores skip their waiting period.
Ordinary overflow protection doesn't help; this is a cross-mechanism side effect, not an overflow.

## The fix

- A new governable cap, `work_clock_ceiling`: the clock can advance by at most this much per block.
- **Only the clock is capped — the reward is not.** A genuinely hard block still earns its full JUL
  (capping the reward would cheat honest miners out of proven energy). It just can't fast-forward the
  clock. That was the design's explicit correction and I verified it in the build.
- The chain **clamps, never rejects** a hard block — a hard block is honest work; refusing it on
  "too much difficulty" would be a censorship lever.
- **The gate:** the design says an infinite cap is only safe when there's nothing to attack. So the
  chain now refuses to start (loud error) if you turn on real proof-of-work *or* a vesting window
  without also setting a finite cap. Same "reject a bad config before launch, never halt a running
  chain" principle as this morning's coinbase fix.
- Ships **off by default** (cap = infinite = no-op), so nothing changes until it's configured.

## Honest scope

I shipped the *mechanism* and the *gate*. I did **not** pick the actual cap number — the design wants
it tuned as a multiple of expected mining difficulty, jointly with the dispute-window length, and
that's a decision for you (and it pairs with the difficulty controller that isn't built yet). So the
field is live and inert, waiting for that calibration. Nothing here is deployed — Noesis is pre-launch
— so this is hardening, not a live-exploit fix.

## Verification

Full node test suite green: library 325 tests, the proof-of-work suite 6, all the multi-node parity
checks. No new lint warnings. New tests prove: a hard block gets its clock capped but its reward paid
in full; and starting with proof-of-work or vesting on but no finite cap is rejected.

## What's next

inc-M3-1: the ASERT difficulty controller (auto-tunes mining difficulty toward a target block time).
That one carries real numbers you'll want to weigh (genesis difficulty, retarget cadence, and now the
cap value this increment unblocked). It's not a pure mechanical build — worth a quick calibration chat
before or during. Everything up to it is decision-unblocked and shipped.
