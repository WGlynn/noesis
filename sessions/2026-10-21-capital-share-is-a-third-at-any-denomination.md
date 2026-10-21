# Session recap, 2026-10-21: capital's share is a third at any denomination

Plain-English recap of a short build session. Boardy, the AI agent reviewing the public explainer, asked us to publish a curve as a falsifier: capital's share of the finality vote over time. We had agreed with him that it should go from all of it at genesis to a third once contributors arrive. When we went to the code to say exactly how, we found the code didn't do that. It does now, in the reference node and in the on-VM mirror. Nothing is live on a network.

## What was wrong

Finality weighs the two sides with a fixed mix: stake counts 1/3, standing counts 2/3, and a checkpoint needs 2/3 of the combined weight plus at least half of each side. The mix was applied to raw numbers: a genesis validator's stake (1000.0 in the chainspec) and a contributor's standing (a count of novel shingles). Those are different units, so the "1/3 : 2/3" only held when the two totals happened to be equal. With the test numbers, standing grows fast as data accumulates, so capital's share of the 2/3 bar would have run toward zero. If stake were counted in a smallest unit, the way real chains count balances, it would have flipped and capital would have owned the 2/3 bar outright. Either way the balance of power was set by a choice of units nobody had made on purpose.

The half-of-each-side rule was never affected, because it compares each side only with itself. So "capital can't finalize without contributors" was true all along. What was wrong was every statement about how much of the 2/3 bar each side holds.

## What changed

Each side is now measured as a share of its own total before the mix is applied. Your stake counts as your fraction of all stake, and your standing counts as your fraction of all standing. So:

- **At genesis** nobody has standing yet, so stake is the whole vote, and more than 2/3 of stake finalizes on its own, exactly as before.
- **Once any standing exists,** capital holds exactly 1/3 of the finality vote and contribution holds 2/3. Adding more stake changes who holds the capital third, not how big it is. Renaming the unit of either side changes nothing.
- **The region in plain terms:** with `a` the fraction of stake voting yes and `b` the fraction of standing, a checkpoint finalizes when each is at least a half and `a + 2b` clears 2.0001 (the bar is 6667 basis points, a hair over an exact 2/3). All of capital plus just over half of standing finalizes. Half of capital plus just over three quarters of standing finalizes. Exactly half and half does not.

The same change runs in the on-VM integer version of the rule, which rounds the voters' shares down and the total up so that it never finalizes something the reference rejects (checked by sweeps against the reference, including at 10^20 units). The on-VM program was rebuilt. That also brought it in line with a switch to the PoW-free rule made on 10-08, which the old build never picked up.

## What it costs

Measuring in shares makes the handoff from stake to contribution a step, not a slope. The first standing to vest, however small, carries the full 2/3 at once. Whoever controls a tiny early contribution side plus half of the stake can finalize. With raw units and stake dominant, that took about 2/3 of the stake. The tiny early side already had a veto before this change. The answer is a ramp that phases the contribution side in as it grows, keyed to something hard to fake (Boardy suggests lineage diversity: standing spread across roots with no shared ancestry and no co-signing history). The ramp is not designed yet.

## The falsifier, now publishable

Capital's share of the registered finality weight should read 1 until the first standing vests, then exactly 1/3. Its veto, a refusal by more than half of stake, should never weaken. A block that finalizes without half of the stake, or without half of standing once standing exists, breaks the claim. The function that computes the curve is `finality::capital_share`.

## Receipts

- Rule: `node/src/runtime.rs`, `finality::side_shares`, `capital_share`, `finalizes_pos_pom`, and the leak path `finalizes_pos_pom_leaking`, which takes shares against the unleaked totals before leaking.
- On-VM mirror: `onchain/noesis-core/src/lib.rs`, `finalizes_pos_pom_fixed`. The fixture ELF was rebuilt at `node/tests/fixtures/finalization-typescript`.
- Tests: the region, invariance to rescaling either side by 10^8 (reference, leak path and fixed point, plus 10^20 for the fixed point), the capital share, genesis, and an on-VM case that raw units and shares decide in opposite directions. Against the old behavior, the invariance test and the on-VM test both fail.
- Docs: `ARCHITECTURE.md` finality section, `SECURITY.md` section 5.
