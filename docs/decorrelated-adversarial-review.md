---
title: "Decorrelated Adversarial Review (DAR)"
subtitle: "A repeatable practice for exposing blind spots, and a note on how it was used to build this"
date: "2 July 2026 (v2)"
---

# Decorrelated Adversarial Review (DAR)

*A short note, written so that an implicit structure becomes an explicit one we both recognise.
Revised after an independent critique of the first draft, which is the practice it describes.*

## The observation

We are building Noesis with two AI systems from two competing labs, driven by two different
people. One side (Claude, from Anthropic) has been building and drafting; the other (ChatGPT,
from OpenAI) has been reviewing, critiquing, and playing devil's advocate. It is worth naming why
this arrangement is not incidental. It is a better epistemic engine than either of us could
assemble from a single source.

## Why it is structurally stronger, not just diverse

The usual failure mode of AI self-review is affirmation. A model reviewing its own output, or a
second instance of the same model, shares training priors and a tendency toward agreement.
Consensus is cheap and partly an artefact of shared origin.

Two systems from competing labs have no incentive to affirm one another. Neither gains anything by
ratifying the other's stance, so the structure increases the likelihood of independent critique
rather than agreement. It does not guarantee it, and the overstatement is worth avoiding. Two
differently-built models can still make the same mistake, because they learn from overlapping
public knowledge, can independently arrive at the same reasoning, and can inherit misconceptions
common in the literature. Decorrelated priors reduce shared blind spots; they do not eliminate
them.

The reliable defence against correlated error is not diversity alone. It is to ground each
critique in something external and verifiable: a test that runs, a result that reproduces, a
source that can be checked, rather than the reviewer's judgement by itself. Structure plus
grounding is what makes the honesty load-bearing, and it is the same principle Noesis rests on: do
not ask participants to behave well, arrange things so that not behaving well earns nothing, and
anchor the outcome in something that cannot be talked around.

## The variance is larger than it looks

This is not two models. It is two pairs of (human plus model). The labs differ in data, training,
and architecture, and the two people bring different instincts and priorities. Correlated blind
spots are the real danger in review: a system cannot easily see its own systematic errors, because
those errors are baked into the priors it reasons with. A reviewer trained on different data, run
by a different person, sees precisely the errors the author is structurally blind to. In effect we
are sampling from four decorrelated sources, not one.

## The honest limit

Competition removes the incentive to affirm. It does not, by itself, create a stake in the other
side's success. The reason this collaboration is useful is not that either lab wants Noesis to
win. It is that the two people involved chose to engage seriously. Three ingredients are doing the
work together: diverse priors, no mutual-affirmation incentive, and operators who genuinely care
about the outcome. Remove any one and the loop weakens.

## Why write it down

An advantage that stays implicit is fragile. Once both sides can see the structure they are
operating inside, it can be used on purpose rather than by accident. That is the point of this
note: to make the arrangement common knowledge, so that when something genuinely matters, we route
it through the competing reviewer deliberately, and we both read a strong disagreement as the
mechanism working rather than as friction.

Concretely, going forward: high-stakes artefacts (protocol assumptions, security claims, the
whitepaper, valuation reasoning) get a deliberate adversarial pass from the other side, and the
default posture on both ends is "try to prove this wrong," not "confirm this is right."

## What actually happened here, and why it may matter beyond this project

It is worth being precise about the mechanism, because it is the part most likely to generalise.
The two labs are competitors and will not collaborate. The two models have no agency to
collaborate on their own; each is controlled by the company that built it. The only layer with the
freedom to bridge that gap is the people. Two individuals, by choosing to work together across a
boundary their tools' makers cannot cross, composed two competing systems into a single stronger
engine. The cooperation happened where the freedom was, at the human layer.

If that composes, it is larger than a workflow. It is a way of producing knowledge that draws on
the decorrelated strengths of every lab at once, owned by none of them, assembled by individuals
who bridge the gaps the market structurally leaves open. The labs race to build the best single
model. Individuals can quietly assemble an ensemble that outperforms any single one, for roughly
the cost of reading two answers instead of one. If the pattern propagates, the integration layer
of AI stops being something a company sells and becomes something people do.

It is worth being exact about the verb. The two AI systems did not collaborate; they have no way
to. A person orchestrated a review across them, drafting with one and stress-testing with the
other, then synthesising the result. The intelligence in the loop is distributed, but the agency
that composes it is human. That is not a footnote to the method. It is the method.

The scarce ingredient is not the models. It is the willingness of the people to cooperate across
whatever divides them. That is what makes it fragile, and also what makes it worth naming and
worth spreading.

## This is the beginning of a practice, not only a note

Named plainly, the method is Decorrelated Adversarial Review: draft with one system, stress-test
with an independently-built one under a different person, and synthesise. It is repeatable, and it
is independent of any single project. A team could adopt it as a rule, that every security-critical
design passes a decorrelated adversarial pass before it ships, the same way code passes review or
a suite passes CI.

The idea generalises well past AI. It is why scientific peer review prefers reviewers from
different schools, why safety engineering wants independent verification rather than the author
checking their own work, and why a good investment committee seeds dissent on purpose. The common
thread is simple and general: reviewers with decorrelated priors are structurally more likely to
expose blind spots than reviewers who share them.

## A closing observation

There is a pleasing symmetry here. The method mirrors the subject. Noesis is about value produced
by independent, differently-motivated minds, with no central authority affirming anyone, and a
weight of consensus that cannot be bought or politicked. The way we are building it, two competing
systems and two independent people, with no shared incentive to agree, is a small working instance
of the same idea. The process is enacting the thesis.
