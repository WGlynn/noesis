# Session recap — 2026-07-14 (AM, autonomous): inc-M3-1 ASERT difficulty retarget

Plain-English recap. Third build increment of the session (after inc-M3-3 fix + inc-M3-2 ceiling).

## The one-sentence version

Built the algorithm that auto-adjusts mining difficulty to keep blocks arriving on a steady
schedule — the last missing piece of the proof-of-work math. HEAD is now `cc3254f`, pushed.

## What it does

Bitcoin-style difficulty adjustment: if blocks are coming too fast, make mining harder; too slow,
make it easier. The standard, decade-hardened algorithm for this is called ASERT (the version Bitcoin
Cash uses). I ported it in integer-only math (no floating point — the chain has to compute the exact
same number on every machine, and floats don't guarantee that).

The clean design trick, from our own spec: the adjustment splits into two halves.
- The **schedule** half ("how many blocks *should* have happened by now") is just a count times a
  constant. No clock needed.
- The **observed** half ("how much time *actually* passed") is the only part that needs a timestamp.

So the function takes the observed time as an *optional* input. When there's no timestamp yet (our
launch phase deliberately runs without one), it treats every block as exactly on schedule and leaves
difficulty untouched. That means the controller is fully built and tested *now*, while the decision
about where the timestamp comes from stays open for later. Same pattern we use everywhere: ship the
real mechanism switched off, wire it on when the inputs are decided.

## The honest boundary

I built the *algorithm*, not the *numbers*. The target block time, the adjustment speed (half-life),
and the difficulty floor are all inputs the caller supplies — the code hard-codes none of them. Those
are your calls, and they pair with the difficulty and genesis-time decisions that aren't made yet. And
it is **not wired into the chain** — it's a tested library function waiting for the Phase-2 timestamp
source. Nothing about difficulty adjustment is live on any running chain (there's no running chain).

## Verification

The strongest test: at exactly one "half-life" of extra time, difficulty should halve (target doubles);
at one half-life of deficit, it should double (target halves). Those come out as *exact* powers of two
in the tests — which is hard to fake and pins the math down. Plus direction (fast→harder, slow→easier),
monotonicity, the never-go-below-floor clamp, and graceful handling of bad inputs (returns nothing
instead of crashing). 15/15 in the proof-of-work test file, no new lint warnings, purely additive.

## Where this leaves M3

All four of the "just build it" proof-of-work pieces are now done: the two target encoders, this
retarget controller, the work-clock safety ceiling, and the reward-split (with its adversarial review).
The pure arithmetic and the safety gates are complete.

What's left in M3 is no longer pure mechanical work — it needs your input: wiring the retarget into the
chain (needs the timestamp-source decision + the actual numbers), the committee-attested-clock design,
the never-halt liveness mechanism, or drafting the queued whitepaper. Good seam to pause the autonomous
build and pick a direction together.
