# Measuring what a contribution is worth: where we are, and a new idea

> Brought into the repo 2026-10-21 so `ARCHITECTURE.md` can cite it. Written 2026-10-16 (Will's design, plain version); still designed, not measured.
> Since then the EXP-62 series measured transfer on a brain that takes in content; current status lives in `ARCHITECTURE.md` (value measure) and the
> `WGlynn/cadence-teachers` ledger `VS-TRANSFER-PROGRAM.md`. The 'where the experiments stand' section below is as of 2026-10-16.

Plain version, 2026-10-16. Status: ideas and design only. Nothing here has been tested yet.

## The problem, in one paragraph

Noesis wants to pay people for what their contributions are actually worth. The hard part: if you only look at the record of who built on whose work, a fake can always be made to look exactly like the real thing, because writing records costs nothing. We confirmed this ourselves: on the record alone, real and fake contributions score the same. So something from outside the record has to come in. The plan names two outside ingredients: (1) proof that the people involved are really independent of each other, in a way that costs money to fake; and (2) a test of usefulness that a cheater doesn't control, a task whose answer they can't rig.

## Where the experiments stand (honest version)

- Our approach measures worth by teaching a copy of a small learning brain and checking whether it gets better at a task it hasn't seen. On a small, simple test world this works: real teachers help, fakes don't.
- On the first harder test world it got shaky. The answer depended on which brain did the measuring and what we compared against. One fake got paid when the comparison was a brain that had learned nothing at all.
- No milestone has moved since October 9. The next experiment tests the shaky part directly.

## Today's idea: stop treating it as one layer

The "fakes look identical" result is true for one layer: a record where every entry is free. Real systems, and nature, are layers on top of layers. This grew out of the same point made in today's messages about clawbacks: "immutable or not" is a false choice once you look at the whole stack. Three ideas follow.

**1. Stack layers that each cost something different.** Put a layer on top of the contribution record whose entries cost real money, energy or time. A cheating ring then has to look independent in every layer at once. If the costs of the layers are truly separate, the cost of faking multiplies, the way a password plus a phone code is much harder to beat than a password alone. Noesis already has half of this: its independence check looks at where the money behind each account came from.

**2. Use the learning brains as the measuring devices, and listen to their disagreements.** Several brains judging the same contributions should come up with prices that hang together. If they can't (one says A beats B, another says B beats C, a third says C beats A), that inconsistency is information, not noise. The paper *Value Disputes Are Missing Dimensions* (Glynn, 2026) says a stubborn disagreement points at something nobody is measuring yet. Here it points either at a missing dimension or at a contribution that is gaming one of the judges. A fake that fools one brain but not the others should leave exactly that kind of inconsistency behind. This is also close to Bernhard's work, where local views have to line up wherever they overlap, and a place where they can't is a signal.

**3. "Outside" just means "too big for the cheater to move."** There is no absolute outside. A test is outside enough if the attacker can't afford to influence it. Zooming out a layer, for example "does another system actually use this?", works for exactly that reason. The candidates:

- **Real use one layer up.** The strongest, because someone else spent effort relying on it. Weak spots: it shows up late, and a cheater could fake use across several cheap systems.
- **Provable energy burn (proof of work).** Truly external, but it proves that someone paid, not that the work is good. Good for proving independence; it can't judge usefulness.
- **Real-world money (revenue).** Expensive to fake, but reading it needs someone to report it honestly.

So the design: judge usefulness by real use one layer up, with cost-based proofs of independence underneath.

## What would make this fail

Everything above multiplies only if the layers are really separate. If a cheater can buy one thing that passes several layers at once (an old account that already has money and history, say), the layers collapse into one and the original problem comes back. So the open question is no longer "this is proven impossible". It's a practical one: do separate, costly layers exist that can't be bought together? That can be tested, but hasn't been.

## Tests we can run

1. **Free, in the next experiment.** Several brains already price the same teachers. Check whether fakes leave more disagreement between the brains than real teachers do. No extra computing.
2. **Price a cheating ring** that has to look independent across one, two and three layers, with separate versus shared costs. Does the cost multiply or level off?
3. **Later:** compare where the usefulness test comes from (real outside use versus our built-in test) against a cheater who gets to pick which tests to supply.
