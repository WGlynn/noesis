# Session recap — 2026-10-19 — keeping the judge out of the contributor's reach

Plain-English recap of a long research day: three experiment threads (Cadence, a private trading project, and Bernhard's OPH group) that all ended up saying something about how Noesis should measure worth.

## The through-line

Bernhard confirmed something in the OPH group that turned into a design principle. In OPH, a higher layer can never rewrite the laws of the layer below it; animals move rocks but cannot change gravity. Will's question was what that means for cheating: a cheater can buy state (tokens, an old account with history) but not law. Applied to Noesis, the "law" is whatever judges a contribution: the judging model, the test it is scored on, the baseline it is compared against, and the rules. Most of today's ideas come down to one move: fix each of those before the work it judges arrives, so the contributor cannot reach it.

The best moment of the day was a single pushback. A draft reply to Bernhard said Noesis is "weaker than nature" because we pick the measuring brain while nature's laws grew from the layer below. Will asked why we were accepting the weakness instead of closing the gap. The answer was a design: grow each round's judge from work that has already settled, and freeze it before the round opens. That question is now generated mechanically on every draft (an inner-dialogue hook with a boot canary), because Will should not have to ask it every time.

## What the experiments showed

**The measure prices information, continuously, down to small amounts (Cadence EXP-62e, 216 fresh students).** Teachers who know 1, 2, 3 or 4 stages of a task are worth +.069, +.146, +.254 and +.416 to a learning brain, a fourth replication on students the measure had never seen. The surprise was in the "fakes": one of them, a teacher that only ever says "not the thing you just did", turned out to carry real information. Labels arrive only after mistakes, so excluding the action just tried is right a quarter of the time instead of a fifth. The measure priced that at about 38% of the weakest real teacher, the same share as its share of the information. Four earlier runs had "failed" because we had filed an informative teacher under fakes. For Noesis: worth is not genuine-or-fake. It is continuous and roughly proportional to information carried, so payouts should price small honest help instead of treating it as fraud.

**The judge sets the exchange rate, not the order (Cadence EXP-62b, 48 fresh students).** Three differently built brains scored teachers who know the same amount but different parts of the task. The two brains that take in content ranked them identically. They disagreed on the rate: knowing the late stages was worth about a quarter of knowing the early stages to one brain, and about a ninth to the other. The third brain barely took in content at all. For Noesis: even when judges agree on which contribution is worth more, the choice of judge decides how much more. That is the strongest reason yet for idea 1 (freeze the judge before the round). It also means idea 2 (pay only what judges agree on) should pay on order, where judges agreed, more than on exact price, where they did not. Caveat: the two content-taking brains are close relatives, so the order result needs a brain from a different family before it generalizes.

**A cost on failure is a signal, not a teacher (Cadence EXP-66).** A brain cannot tell a hidden success from a miss when both cost nothing (EXP-65). We charged .05 for every miss that did not end the attempt. The brain's own learning signal could then see the misses. A memory gate keyed on that cost added a clear gain on top of what the cost taught: 18 discordant passes to 3. It never once held a correct move (58,563 held, all misses). The catch: the gain appeared only when a teacher was also present. With reward alone, the cost made the brain timid, and how often it succeeded fell on every chain. Posted to Bernhard on the Cadence tracker (#202).

For Noesis this bears on design idea 6, a refundable deposit whose forfeits become failure labels for the judge. The deposit tells the judge something it cannot learn otherwise. On its own it may also make honest contributors cautious. So it should sit next to a signal that shows what good work looks like, not replace one. In Noesis that signal is realized use by independent minds (the periphery design); the deposit adds the failure side. Realized use reports failure only as absence, which cannot tell useless work from work nobody has found yet.

**Measured edges decay (separate market research).** Every edge we found there was strongest a few years ago and close to gone today. A measurement that worked once does not stay calibrated as the world adapts. For Noesis: standing should vest against continued use and be re-scored as new evidence arrives. The re-opening trigger and clawback design from 2026-10-06 is the machinery for that.

## Design ideas that came out of it

Full list with tests: Desktop `noesis-design-brainstorm-2026-10-19.md` (designed, not built). The two cheapest with the most reach:

1. **Grow the judge, freeze it before the round.** v8 is already a learned judge. Train it only on settled work, publish a hash of its weights before the next round opens, and score that round with exactly that judge. Work submitted in a round has no way to shape the judge scoring it. Open attack: plant work early to bend a later judge.
2. **Commit-reveal the test.** Contributors commit first; the evaluation task is then drawn from a public random beacon. Nobody can train toward a test that did not exist when they committed.

Not yet checked: whether the Constitution already fixes when v8 retrains, and whether rule changes already wait for round boundaries.

## Open questions

- For Bernhard: in OPH, does a layer's law finish settling before the next layer starts to form? If layers overlap in time, early work could steer the law that later judges it, which is the main attack on idea 1.
- Does an independent set of judges exist for Noesis, or do all practical judges share features and fail together?
