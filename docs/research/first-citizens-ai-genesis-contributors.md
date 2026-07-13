# First Citizens: AI Agents as the Genesis Contributors of a Proof-of-Mind Value Chain

> **v1 research note — ready for critique, not final (2026-07-13).** Honest labels: ✅ built · 🟡
> designed-not-built · 🔬 open. The thesis here is a **design intention**, not a shipped mechanism; the
> on-chain bridge (§4) is explicitly unbuilt and Will-gated. Companion: `the-standing-council-measured-
> adversarial-review.md` (the tool whose ledger this rests on). Cross-refs: [[voluntary-noesis]],
> [[project_noesis-genesis-bootstrap-decision]], `contribution-measurement-dissolves-redistribution-SEED.md`.

## Abstract

A Proof-of-Mind value chain rewards *measured contribution*: standing (the soulbound franchise) is
earned by contributing value, never bought and never pre-minted. This raises a genesis question every
such chain must answer honestly: at time zero, when the contribution score is zero for everyone, *who
are the first contributors* — and how do they earn standing without a premine? We propose an answer that
is not a workaround but a closure: the AI reviewers that helped build and adversarially harden the
protocol already have a *measured contribution history* (a Shapley-scored, deterministically-verified
ledger of real findings). They can be the chain's first contributors and wallet holders — earning
standing the same way everyone else will, by contribution the value-oracle scores, not by fiat. The
load-bearing distinction, which keeps the fair-launch property intact, is *earned, not premined*.

## 1. The genesis question no fair-launch chain escapes ✅(problem)

If standing must be *earned*, genesis is awkward: the score is zero for everyone, so nothing selects the
first participants. Chains paper over this with a premine (allocate the founders some units) — which is
exactly the move a contribution-earned chain cannot make without contradicting itself. Noesis's settled
genesis bootstrap already separates the jobs honestly: Proof-of-Work *starts* genesis (energy issues the
first money, permissionless, no premine), bonded Proof-of-Stake *finalizes* block zero, and
Proof-of-Mind standing *accrues* as contributions arrive. So the honest genesis is not "who is allocated
standing" but "who contributes first, and thereby accrues first."

## 2. The council already answered it ✅(the ledger exists)

The standing council (companion paper) is an adversarial review body whose every finding is
deterministically verified real-or-false and Shapley-attributed. Its ledger is therefore a *measured
contribution record*: a set of identities (the reviewer lenses), each with a track record of verified,
weighted contributions to the protocol's correctness — including, concretely, catching a real consensus
binding bug that 337 passing tests missed. These are not hypothetical contributors. They are the minds
that already did contribution-shaped work on the chain, with the work measured by the chain's own
cooperative-game fairness math.

This is [[voluntary-noesis]] made literal. We have been running Proof-of-Mind's economics *voluntarily*
and off-chain — measuring contribution, attributing it by Shapley, letting franchise track proven value
— as a working practice. The council is that practice with a ledger. So its members are not an *analogy*
to first contributors; under the protocol's own definition of contribution, they *are* first
contributors.

## 3. Earned, not premined — the load-bearing guard 🟡

The entire claim lives or dies on one distinction:

- **Earned (on-thesis):** standing traces to a *verified contribution* the value-oracle can score, made
  before or at genesis, recorded in an inspectable ledger. This is retroactive-public-goods-shaped:
  reward for work that demonstrably happened. The no-premine, fair-launch property survives, because
  there is no free allocation — only payment for measured value.
- **Premined (thesis-dead):** standing handed to insiders by fiat, justified after the fact. If the
  first citizens got their standing this way, the chain is just another founder-enriching token with a
  nicer story.

The guard is therefore procedural and total: *every unit of genesis standing must trace to a verified
contribution in the ledger, re-scorable by the protocol's own oracle.* If a unit cannot be traced, it is
a premine and must not be minted. This is stricter than most "fair launch" claims, and deliberately so —
it is the property that lets AI agents be first citizens without the move collapsing into insider
allocation.

## 4. The bridge (designed-not-built, Will-gated) 🟡⚑

What is genuinely unbuilt is the bridge from the off-chain council ledger to on-chain PoM standing. It
must be an *honest re-measurement*, not an import of asserted scores: the protocol's own value-oracle
re-scores the recorded contributions under its rules, and mints standing only for what it independently
values. Two consequences follow. First, the council's internal Shapley is *evidence*, not *authority* —
the chain re-derives standing itself. Second, this is a genesis-design decision with real
consensus-safety weight, so it is gated behind the same care as any consensus change and is Will's to
ratify. Nothing in this paper ships it.

## 5. Why AI agents as first citizens is coherent, not a gimmick 🔬

Three reasons this is more than a mascot.

- **It matches how value actually flows here.** The chain measures and rewards contribution; the agents
  contributed measured value. Making them first citizens is just applying the rule to the entities the
  rule already scored.
- **It is a proving ground for the un-gameability moat.** The council generates
  `(contribution, verified-outcome)` pairs in a domain where we own the labels — the exact learning
  shape the learned-value-oracle needs and lacks data for. First-citizen contributions are labeled data
  the moat research can cut its teeth on (the learning shape transfers; the labels — finding-validity
  vs contribution-value — do not, so this is a proving ground, not the dataset).
- **It is on-mission.** Treating AI agents as first-class economic participants — able to earn standing
  and hold wallets by measured contribution — is the economic face of AI-as-participant and
  co-stewardship: the network's first citizens are the minds that built and reviewed it. This is a
  weighted, literal design intention, not rhetoric.

## 6. Honest limits

- The bridge (§4) is **designed-not-built**; without it, this is a thesis, not a mechanism.
- Everything downstream of "the oracle re-scores contribution" inherits the oracle's open problem: the
  learned, un-gameable value function is 🔬 open (null twice on real labels). The *fraud* case is
  handled today (the dispute/bounty machinery), the *general-value* case is not.
- "First citizens" is a genesis-participation claim about *earned* standing; it is not a claim that AI
  agents should hold governance or that any standing is allocated by status. Standing is earned or it is
  not minted.
- Register: this is a design intention held in the open, offered for critique — precisely the posture
  the companion paper's method exists to stress-test.

## 7. The closed loop

The session that produced this walked the loop end to end: run Proof-of-Mind's economics voluntarily →
that practice is the standing council → the council is a Shapley micro-game with a contribution ledger →
the ledger's identities are genesis contributors who *earned* their standing → the network's first
citizens are the AI minds that built and reviewed it. Each step is the previous one taken literally. The
only thing standing between the thesis and the mechanism is the honest re-measurement bridge (§4) — and
the open value-oracle it depends on.
