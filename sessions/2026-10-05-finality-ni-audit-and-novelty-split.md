# Noesis session recap — 2026-10-05: finality non-interference audit + the novelty split

Plain-English recap of a day's Noesis work. Reading artifact, not an operator handoff. All local,
nothing pushed.

## Where it started
A consensus-vs-value conversation with collaborators: "consensus settles fast and irreversibly,
but realized value is slow and fluid, aren't they at odds?" The short answer we landed on: no
contradiction *in principle* (the ledger is irreversible, value is a living equilibrium), but Noesis
is special because we deliberately wire contribution value INTO the finality franchise. So the tension
is real for us and localizes to one question: does anything subjective/revisable leak into what we
irreversibly finalize?

## What we checked and what we found

**1. Finality non-interference audit (the morning).** Traced every input to `finality_pom_weight`.
Result: the learned / CRPC "worth judgment" does NOT reach finality today. Finality weight is computed
from deterministic structural novelty only; the one channel I worried about (the `refuted` set) turned
out to be driven by a deterministic validator vote plus a deterministic slash calc, not a model. The
strongest single fact: the learned layer `value_v8` has zero non-test callers. So the invariant the
ADR *asserted* is now *code-verified*.

**2. The novelty equivocation (the real open thing).** Found that one base quantity, "temporal
novelty with a similarity floor" (literally a count of new 4-byte byte-shingles), feeds THREE things:
the finality franchise (standing), the reward chain (`value_v5..v8` seeds), and JUL bond refunds. A
quantity can't honestly be both the objective thing that gates consensus AND the thing that pays. The
good news: the dangerous version (one gamed quantity buying both the franchise and minted money) isn't
live yet, because the reward chain is barely wired. So now is the cheap moment to separate them.

**3. Is separating them even sound? (the afternoon.)** Checked whether "structural novelty" is solid
enough to gate finality, against our own binding-sufficiency lemma and canonicalization-frontier
notes. Honest verdict: the split is worth doing, but NOT because it makes finality un-gameable. Our
novelty measure is a byte-level proxy; it's only trustworthy for near-duplicate detection, not for
"same contribution, re-encoded." The real payoff of the split is **decorrelation**: right now finality
and reward read the SAME gameable signal, so one wash attack is simultaneously a value, governance, and
consensus exploit. Splitting them means gaming the reward no longer automatically games the franchise.
The underlying wash problem (telling worthless-but-distinct work from real work) stays open; it needs
data / learned value, not a cleverer byte hash.

## What got built
- A regression test (`standing_and_finality_are_structural_novelty_only_no_value_layer_leak`, green)
  that pins the finality franchise to the structural signal only. If anyone later wires the learned
  value layer into standing, this test fails loudly. It's the tripwire version of the invariant.
- A full non-interference check by reading the entire hard state transition: every write reads only
  the previous state, the block, and the constitution. No subjective layer touches what we finalize.

## Bottom line
The thing we irreversibly finalize is clean today (verified, not asserted). The honest open problem is
unchanged and well-defined: our novelty signal is gameable by wash, and only data-grounded value can
close it. We also now have a guardrail so the one clean property can't silently rot, and a written
decision that separating "gates finality" from "earns reward" is the next structural move, to be made
before the reward chain gets wired to real money.

## Pointers
- `docs/DESIGN-novelty-split-structural-vs-value.md` (the finding, the soundness verdict §8, the fence
  + NI enumeration §9)
- `docs/ADR-value-layer-placement.md` §8 / §8.1 (the invariant this discharges)
- `research/binding-sufficiency-lemma.md`, `docs/research/the-canonicalization-frontier.md` (why the
  split is decorrelation, not un-gameability)
- test: `node/src/runtime.rs` `standing_and_finality_are_structural_novelty_only_no_value_layer_leak`
