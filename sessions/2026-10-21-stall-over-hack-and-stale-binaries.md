# 2026-10-21: a stall over a hack, a judge that pins its whole setup, and binaries that were quietly out of date

Plain-English recap of the afternoon. The morning's normalization fix has its own note:
`sessions/2026-10-21-capital-share-is-a-third-at-any-denomination.md`.

## What we decided

**A finality stall beats a wrong finalization.** Boardy asked how a stall ends if one side refuses to vote.
His answer was an expiring floor: after a deadline, silence stops counting, unless someone posts money to
object. Will looked hard at it and said no for now. A timer on the half-of-each-side rule is a timer on
the promise that money can't buy finality, and an objection that needs money to count favors whoever has
money. No design gets both safety and liveness when enough people go quiet, so you pick which failure to
eat. A stall can be paid back; a wrong final block can't. The choice is really one dial, how long silence
has to last before it stops counting as a no, and it's set to forever until a testnet shows how often
stalls come from people simply being offline. The price of forever: a side that's mostly offline stalls
finality just as a side that refuses does. The full reasoning, the four ways to make stalls rarer,
cheaper, shorter and insured (none built), and when to revisit: `docs/RULING-2026-10-21-stall-over-hack.md`.

**A frozen judge has to pin its whole setup, not just its name.** If a learned model ever judges
contributions, the amendment that adopts it must write down the exact library version, every setting,
what worth is measured against, and the practice conditions. Our own experiments showed those choices
alone move the price, and the library we use is about to change a default (Cadence #220). Recorded in
`docs/DESIGN-value-oracle-seam.md`, which also had an outdated claim from before EXP-62i; that's fixed.

## What we found and fixed

**Four of the five on-chain test binaries were out of date.** The tests that run our RISC-V scripts use
compiled binaries checked into the repo. Nothing checked they matched the source. This morning's work
found the finalization binary two weeks behind. Rebuilding everything showed four of the five differed
from a fresh build. All 60 on-chain tests passed on the fresh binaries, so nothing broke that the tests
cover. Now a check (`scripts/fixture-freshness.py`) runs in CI and in the pre-commit hook (in any clone where
`scripts/install-hooks.sh` has been run), and fails if a script's source changes without a rebuild. Two lockfiles were stale the same way and are updated.

## Still open

- The genesis ramp. With each side now counted as a share of its own total, the first contributors to
  vest carry the whole contribution side at once, so a small captured early group plus half the stake
  could finalize. A ramp, triggered by how many independent lineages hold standing (Boardy's idea),
  would phase that in. Next up.
- Whether a vote for a competing proposal counts as a miss (held for peer feedback since 10-20).
