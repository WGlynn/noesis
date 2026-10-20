# Session recap, 2026-10-20: SAGE, and memory you can price by rerunning

Plain-English recap. No Noesis code changed; one research note was added and one issue was filed on
another project.

## Where it came from

Will shared SAGE (github.com/l33tdawg/sage), an open source system where AI agents sign the lessons they
learn and a validator network votes on which ones become shared memory. His hint was to think about how
it relates to Noesis. It relates closely: SAGE decides what a group of minds remembers, and Noesis tries
to decide what a contribution is worth.

## What we found

**The same wall.** SAGE's own README says its checks decide what gets committed, not whether it is true.
That is the validity versus veracity line Noesis already draws: consensus can certify that a record is
well formed and admitted, and nothing more. Under it sits an old problem, the Munchhausen trilemma: any chain of justification ends in a regress, a
circle, or a chosen stopping point. SAGE stops in two places: a circle (validators are
scored by agreeing with the final vote) and a human governance authority it names openly. Noesis stops at
proof of work for on-chain facts and, for value, at the scarcity of independent minds, which is still
open.

**Agreement only counts where disagreement was possible.** This one rule explained everything we found.
Honest SAGE validators run the same fixed checks on the same memory, so they almost always agree, and agreement
that could hardly have gone otherwise says little. The
one signal that could have gone either way, a separate AI trying to break each challenge blind, is what
made SAGE's agents improve over time in its fourth paper. The rule also turned up a likely bug: on ARM chips
the Go compiler computes SAGE's validator weights slightly differently than on x86, and in our emulation 14%
of inputs gave different bits (not yet confirmed on ARM hardware). Each machine is fine alone; only comparing two machines that could
disagree shows it. And it restates Noesis's hardest problem: a closed ring of minds vouching for each
other passes every internal consistency check, because no one outside the ring is part of the comparison.

**Memory is a counterfactual you can actually run.** Will asked whether institutional memory is a
counterfactual, because that would make it easy to price. It is. A memory's user is an agent, and an agent
can be rerun with the memory removed. SAGE already has the on/off switch. Remove one memory, rerun the same
tasks under the same frozen judge, and the difference is what that memory was worth. Noesis already does
this kind of with-and-without rerun inside its value function. The catch is cost, not definition: runs are
noisy, the judge has to be fixed in advance, and a memory nobody retrieves scores zero.

**That widens the scope a notch.** Noesis starts with contributions whose counterfactual can be run (code,
models, prompts, datasets) and widens as measurement improves. Replayable agents add written know-how:
lessons, postmortems, design decisions, anything an agent uses on a task with a fixed outside judge. It
measures value to the agent, not to people, and it only reaches as far as fixed judges exist. Designed,
not built.

## What shipped

- Research note: `docs/research/sage-validity-layer-and-the-memory-counterfactual.md`.
- Issue on SAGE, with Will's go: l33tdawg/sage#458 (the ARM versus x86 weight mismatch, with a
  reproduction and three possible fixes, plus two smaller paper-versus-code notes).

## Honest status

We did not rerun SAGE's experiments; their pipeline is not public. The 14% figure comes from emulating
ARM's arithmetic on an x86 machine, not from ARM hardware. The wider scope is a design direction. The
mind-scarcity base case is still open; this session sharpened how to test for it, it did not close it.

## What's next

Watch #458 for the author's reply. If the conversation opens, the natural joint experiment is pricing
individual memories in SAGE's red-team loop by removing them one at a time, which would be the first
real-labels data of the kind Noesis's learned value function is waiting for.
