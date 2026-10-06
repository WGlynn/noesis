# Session recap — 2026-07-14 (AM, autonomous): the owed inc-M3-3 adversarial review

Plain-English recap for Will. What happened, why it mattered, what's next.

## The one-sentence version

Last session shipped the N-way coinbase split (inc-M3-3) but committed it *before* running
the adversarial/Council review — that review was owed. I ran it, found one real bug, fixed it,
and shipped the fix. HEAD is now `81d244a`, pushed.

## What the review found

The coinbase split lets governance route the block reward to several recipients (say: 20% to an
infra fund, 5% to treasury, the rest to the miner). Each recipient's payout becomes its own "cell"
(coin), and every cell needs a unique id. The id was built from the block height plus an 8-bit
slot number for the recipient's position in the list.

8 bits only counts to 256. Nothing stopped the recipient list from being *longer* than 256. If it
were, recipient number 256 would wrap around and get the *same id* as recipient number 0 — two
different coins silently claiming to be the same coin. That corrupts a rule the system leans on
("every coin has one unique id").

Important honesty note: it could **not** print extra money. The total still added up exactly to
the block reward (there's a separate cap that guarantees that). So this was an id-collision bug,
not an inflation bug. And it only bites if someone *configures* a broken setup at genesis — it's
not something an attacker can trigger block-to-block, and Noesis isn't launched yet. So: a real
hardening fix, not a live exploit. I kept that framing honest in the commit and handoff.

## The fix

The system's rule is "never halt a running chain." A bad *genesis configuration*, though, should
be caught *before* the chain ever starts — that's not a halt, it's refusing to launch with a broken
setup (Bitcoin does the same). So:

- Added a named limit: `MAX_COINBASE_SPLIT = 256` (the 8-bit slot is the real cap).
- The chain now refuses to start (loud, clear error) if the recipient list is longer than 256.
- A second, cheaper check guards the internal path as a backstop.
- Two tests: one proves exactly 256 recipients all get distinct ids (the safe edge), one proves 257
  is rejected. If someone later deletes the guard, the second test goes red — so it can't rot.

All existing tests still pass (coinbase suite 13/13, the two-node parity check green, the full node
library 323/323), and no new lint warnings.

## Method note

I did the review as a solo analysis rather than spinning up a fleet of review agents — the surface
was small and had exactly one finding, so a swarm would have been wasted spend. Ran the real tests
to verify rather than estimating.

## What's next on the M3 map

1. ~~inc-M3-3 adversarial verify~~ — done this session.
2. **inc-M3-2 work-clock ceiling** — the next piece (clamp the clock only; a precondition for turning
   on real proof-of-work enforcement). Resume here.
3. inc-M3-1 ASERT difficulty controller.
4. Committee-attested wall-clock spec (for the public network phase).
5. Never-halt liveness mechanism (min-difficulty floor + stall detector) — still design-only.
