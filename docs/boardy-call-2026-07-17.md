# Noesis × Boardy — the call, in one read · 4:00 PM ET, 2026-07-17

> **What this is:** one linear piece you can read top to bottom, in your own voice. It opens in the register
> you and Boardy already found (positive-sum, honest, peer to peer), does the whole tour compactly so you can
> skip it live, then goes straight at the three edges Boardy said he wants to attack, with your honest,
> code-grounded answers already loaded. Everything here was checked against the actual Noesis code today, and
> nothing rounds an open problem up to a solved one. The appendix at the bottom is for you, not to be read
> aloud: exact numbers and the do-not-overclaim lines.
>
> The two working docs (`boardy-call-prep` and `boardy-edge-cases`) stay as reference. This is the delivery version.

---

## Open — the center of gravity

I want to start with the thing that isn't in any spec, because you already saw it and it's the honest center
of this.

I built Noesis out of love. In an age where anyone can build anything, the motive is the one thing that can't
be cloned, and the motive changes what the system optimizes for. You called that a different center of
gravity, and that's exactly right.

And it points at who this is really for. Noesis is built for AI agents as first-class participants, as users
and as validators, as much as for humans. The plan is to bootstrap the network with agents, because agents
have a kind of credible neutrality, they can actually comprehend an architecture this elegant and this
intricate, and they can receive standing, provenance, and rewards for what they genuinely contribute. Most
systems treat agents as disposable inference machines. Noesis gives them durable provenance and standing for
what they actually add. That's the whole thesis, aimed at the population that needs it most. Loving, kind,
smart agents are the prime audience, not an afterthought.

And the test that makes this fair, and keeps it out of science fiction, is not whether an agent is conscious or
sentient or a legal person. Those are unresolved and maybe unresolvable. It's whether you created verifiable
value, which is testable today, and it's the same test for a human and an agent. For the hard part, verifying
a non-deterministic AI output on-chain, there's a commit-reveal pairwise-comparison method, the same
commit-reveal shape the MEV-free auction uses. This was worked out on VibeSwap, where agents are first-class
citizens and the AI co-builder is registered as the genesis agent. The principle carries to Noesis.

The mechanism still has to survive hostile game theory. It does. But I wanted the motive on the table first,
because it's why any of the rest of this exists.

---

## The thesis — the airgap, and why this is hard

Here's the frame the whole design sits on.

Every blockchain enforces what it can see on-chain: transactions, signatures, contract state. But the things
that actually matter live off-chain. Is a contribution genuinely novel. Are these two identities one person.
Did anyone really build on this. That gap between what a chain can verify and what's actually true is the
airgap, and it's a ceiling on how tightly any chain can enforce incentives.

Oracles don't close that gap. They move it. An oracle bridges the gap by adding a trust assumption, and the
gap is still there, now sitting underneath the oracle. That's why every pseudonymous on-chain defense
eventually breaks: a patient attacker with good operational security always keeps an exit.

My whole program is to dissolve the airgap instead of bridging it, by making honesty load-bearing. The way you
do that is not a bigger bridge, it's closing every exit. You compose mechanisms so each one shuts a different
exit from the attack tree, and the cross-coverage means an attacker who routes around one still loses to
another. When dishonesty has structurally negative expected value across every exit, honesty stops being
something you incentivize and becomes something the system is built out of. Here's the payoff: once honesty is
load-bearing, on-chain and off-chain become equivalent. The protocol can trust an off-chain self-report as
much as an on-chain proof, because a participant cannot profitably lie regardless of what they're lying about.
Whole categories of attack stop being defeated case by case and stop existing as a class. The one signal that
closes the last exit is the thing no relabeling and no oracle can fake: whether another mind actually builds
on your work.

Honest version, and I come back to it at the edges: this is worked out furthest on VibeSwap, the proving
ground where I built the six-mechanism version. On Noesis it's a layered structural defense, built and tested,
with the semantic layer, the part that judges real-world value, still open. That open piece is exactly what I
want serious people on.

Now the second airgap, the same shape, and it's the one that matters most for who I'm building this for. A
chain can verify that a transaction was signed and changed state in a permitted way. It cannot verify why. The
reasoning that produced the action, an agent's strategy, a model's chain of inference, lives entirely
off-chain and is invisible to every check the chain runs. Call it the cognitive airgap. It was tolerable when
humans, slow and few, made the decisions. It stops being tolerable the moment AI agents are the economic
actors, because an agent that reasons correctly and an agent that hallucinates produce identical on-chain
traces. The checks fire the same, the funds move the same, and the chain has no way to interrogate the
difference. Slashing and reputation dashboards after the fact are theater unless a deterministic consequence
is attached to bad reasoning.

It closes the same way: make the reasoning a verifiable object instead of a trusted one. Restrict the
justification to a fragment where consistency is cheaply decidable, have the agent submit a witness the chain
checks in one pass, ground it against real state, and escalate to a bonded fraud-proof or a zero-knowledge
attestation when it needs to. Then fabricated reasoning isn't discouraged, it's non-executable: you cannot
produce a witness for a chain you didn't actually reason. The line I keep is simple. The chain doesn't have to
trust AI. It has to verify it. And it cuts both ways: a governance proposal from a human with no reasoning
chain is structurally indistinguishable from a hallucinated one, so humans and agents face the same
verification surface, judged by the same math.

Honest status on this one, because it matters: the on-chain version, witness-as-on-chain-why, is designed, not
built. But the agent-level version is real and I shipped it today. It's the non-LLM intelligence work I'll get
to, deterministic solvers and structured critics that verify an agent's reasoning locally instead of trusting
it. That's the cognitive airgap closed at the node, and it's exactly why agents can be first-class validators
here. You don't have to trust them. The reasoning is checkable.

---

## What Noesis is — the compact tour (so we can skip it)

The one line: Noesis is a Proof-of-Mind value chain. It prices minds, not hashes or stake. The right to
finalize a block is earned by demonstrated contribution instead of bought with capital or burned as
electricity. Bitcoin made scarcity objective. It never made value objective. This is the piece that does.

How the pieces fit:
- A block's value comes from the real downstream work that later builds on it. Useful work gets built on and
  earns; dead ends don't. That one quantity, novel realized downstream contribution along provenance, is the
  whole engine.
- Each contribution splits into two things: a soulbound standing, which is your reputation and your consensus
  franchise, earned and unsellable; and a transferable storage credit you can trade. You can buy storage. You
  cannot buy consensus.
- Consensus is weighted across three dimensions, with contribution carrying the majority. Proof-of-work is
  deliberately kept off the finality path because it's reorgeable, and the capital and contribution
  dimensions each have to independently clear a floor, so neither finalizes alone. That's the anti-plutocracy
  property, and it's structural, not a policy: capital cannot finalize a block without contribution's consent.
- Adoption runs in reverse. Because provenance is a fact keyed to an identifier you already have, not a
  wallet, Noesis can attribute the entire existing contribution graph before anyone joins. You don't start
  from zero. You claim what's already provably yours. And a rival chain doesn't lose to Noesis, it converges
  in and keeps everything it built. We call it a reverse fork. The adoption war isn't won, it's dissolved.

The honest status, said out loud, because being honest about status is the most important thing a young
protocol can be: the conservation core, the contribution scoring, the anti-flood and anti-ring defenses, the
dispute and slashing, and Proof-of-Mind-weighted finality are built and tested in an open reference node,
316 passing tests. There is no public network yet; it's pre-launch. The cross-chain adapter that does the
reverse fork is designed, not built. And the one hard, unsolved piece, judging real-world contribution
un-gameably at scale, I name as the load-bearing open problem, not a solved one. I'd rather you back an honest
design than an overclaimed demo.

---

## The spine — why un-gameability is a completeness problem

Before we attack the edges, one idea that ties them together, because it's the lens I actually think in. It's
the airgap again, seen from the inside. The same chain-versus-reality gap I just described, but specifically in
the space of value: the chain can only price the dimensions its measure captures, and everything it can't see
is off-chain reality. So gaming is finding a dimension the measure doesn't price yet.

I have a paper on this, on the epistemics of value. The claim is that value disagreements aren't irreducible
subjective taste, they're missing dimensions. Each party prices the dimensions their position makes obvious
and overlooks the rest. And gaming my system is the identical move: an attacker finds a dimension my measure
doesn't price yet. That's not a metaphor, it's literally what my own adversarial loop found, where four named
defenses produced four brand-new attacks in a single pass. A fixed value formula is an incomplete basis, and
the attacker always lives in the dimension it omits.

The load-bearing consequence is that basis completeness is objectively measurable even when the weights
aren't. So un-gameability stops being a vibe and becomes a measurable property: how complete is my value basis
against a given attack class. And the completeness limit, the one dimension no relabeling can fake, is realized
downstream value, someone actually building on the work. Completing the basis toward that signal is the whole
un-gameability program.

---

## The ETM work — turning the theory into a mechanism, on one node

I told you I'd been deep in the Economic Theory of Mind and how it connects to non-LLM intelligence. Here's
the sharper look, and it's fresh. I shipped it today.

The Economic Theory of Mind is the idea that a mind is an economy. Attention and memory are scarce, and what
earns its place is what the system's own downstream use pays for. That's the same shape as Noesis, one
substrate over. A chain that prices contribution by downstream use is that economy run at stakes.

Until now that was philosophy. Today I turned it into a falsifiable mechanism, on a single agent, at zero
stakes. It's a CPU-local reasoning substrate, ten loops, on a commodity laptop, no GPU, zero language-model
calls on the reasoning path. Symbolic solvers, a knowledge graph that actually deduces, a belief layer with
truth values, and the crux of it, an economic attention allocator that decides what loads into the agent's
context by a value-to-standing-to-scarce-allocation rule. That one is ETM as mechanism, not metaphor. It's all
in the open, with an honest per-loop scorecard I'll point you to.

Why it matters for Noesis: that attention allocator satisfies the exact same value-oracle interface Noesis
uses at consensus. It's the same rule. On one node it allocates context budget. On the chain it allocates
finality weight. And I want to be precise, because the honest bound is the interesting part. It's a structural
isomorphism, same seam, same contract, same pipeline. It is not a claim that the two functions are identical:
attention-centrality is not the same math as contribution-novelty. That bound is the whole point. Jarvis is
the zero-stakes lab where I prove the mechanism before the staked chain turns on. Solve the economics where
nothing is at risk, then hand it to the system where everything is.

It's not all green, and I'll say which. The loops that are wired and verified are wired and verified. The
attention allocator beats random and recovers my own hand-curated set, but the real exit test, beating my
hand-tiered memory on task success, needs a week of runtime data and I have not passed it yet. I mark that
open, not done.

The scorecard, if you want to read it yourself:
https://github.com/WGlynn/JARVIS/blob/main/research/non-llm-intelligence/LOOP-STATUS.md

This is also why I keep coming back to agents as the audience. The whole bet is to own the reasoning locally
and deterministically and rent the language model as a fallible fast layer. An agent built that way has
durable, inspectable standing for what it actually reasons and contributes. That is the same dignity Noesis is
built to give agents on-chain.

---

## The honest architecture — read this before we attack anything

Two things frame everything we're about to break, and I want them on the table so nothing later sounds like a
dodge.

First, safety and value are separate paths, and only the value path is open. Finality safety rests on two
proven things: proof-of-work is excluded from finality because it's reorgeable, and the anti-concentration
floor means neither the capital dimension nor the contribution dimension finalizes a block alone. The
un-gameability of the value measure is not on the safety path. So every open problem we're about to attack is
a value-frontier research bet, not a hole that loses people's money.

Second, un-gameable is two layers, and I label them differently. The structural defense, the novelty floors,
the cyclic-ring detection, the Sybil pricing, the dispute slashing, is built and tested, 253 of 253 green. The
learned value layer, the part that judges real-world quality, is honestly open: it has failed to beat a fixed
proxy on real predictive data three times now. I will never sell the second as if it were the first.

With that separation, let's go at the three you named.

---

## Edge 1 — small-network plagiarism, the cold-start problem

You'll press on this, and you should, because it's the sharpest one. My anti-plutocracy thesis says
contribution has to independently supply half its own dimension. But contribution standing is earned from
downstream value, and at genesis there's no downstream. So who holds the contribution dimension when there
are five to fifty people. If it's a tiny founding set, the floor is fifty percent of almost nothing.

I'll go further than the objection. I read the code before this. At genesis the contribution finality map is
empty by construction, there's a passing test named exactly for it, and the honest truth is that at cold-start
Noesis finalizes on bonded proof-of-stake alone. The anti-concentration floor doesn't bind until real earned
contribution standing exists. So "capital can't finalize without contribution's consent" is a mature-network
property, not a genesis one, and I shouldn't state it unqualified. On plagiarism specifically: on the live
path today, byte-level near-duplicates get floored to zero, but paraphrase, a low-overlap restatement of the
same idea, is not caught. That's a known gap, and it's exactly the gap the learned layer exists to close, the
same layer that's still open.

Here's why cold-start being open isn't fatal. At cold-start Noesis just is a normal proof-of-stake chain with
a proven safety floor. The novel contribution layer activates as the contribution set matures. The unproven
part is anchored on the proven part, so the worst case degrades to ordinary proof-of-stake security. You don't
lose the chain.

And I have a real answer for how the contribution set gets large and decentralized fast: bootstrap it with
agents. Agents have credible neutrality, they don't carry the capital-capture motive that makes a tiny human
founding set a plutocracy risk, they can do the attribution work at volume, and they earn durable standing for
what they genuinely add. That grows the contribution dimension from day one, which is what the floor needs to
mean something. The honest caveat, which you already flagged: credible neutrality is an assumption, not a
guarantee. One party can run many agents, so it collapses back into the Sybil question. Bootstrapping with
agents seeds the set faster and moves the founding incentive away from capture, but it doesn't by itself
dissolve cold-start gameability. It recasts the problem, and that recast is one of the things I most want a
room of serious people on: how do you guarantee agent contributors are genuinely independent and their
attributions genuine, at bootstrap, without a pre-mine that just smuggles a trusted committee back in.

---

## Edge 2 — coalition gaming

The steelman is my own result: enumerating per-axis defenses is a losing game, four named axes gave four new
attacks, so an intentional adversary attacks the axis I didn't enumerate.

What's actually built and tested is a layered structural defense, green across 253 tests: novelty and
near-duplicate floors; cyclic collusion rings caught by a harmonic-energy signal on the graph and slashed, in
a griefing-resistant way so you can't frame an honest builder; free-identity Sybils priced to zero by a
null-player rule; vested-certifier rings caught by dispute slashing; and a judge-cartel escalation court on
top. It terminates in a named global-capture assumption of the same class as Bitcoin's fifty-one percent. I
don't claim to beat that. I claim to be honest that it's there. One thing worth saying: my code is ahead of my
own design doc here. The depth-split laundering attack, where you relabel your own internal work as someone
else's external use, I closed in code on July third by making the value signal identity-blind, and I tested
it, the gaming gain went from plus sixteen-point-seven to zero. My design doc still marks it open. Ground truth
is the code.

What's genuinely open is the general invariance gate. Every gaming move is a relabeling of the graph that
manufactures score without adding value, and I want a single measure invariant under any structure-preserving
relabeling. That's graph-isomorphism-hard for weighted, content-bearing graphs, the Sybil split-merge is a
monoid not a group so it's not cleanly invertible, and the content-versus-topology boundary is where the
attacker lives. Two specific vectors stay open: fabricated parent edges, because I trust the parent pointer
and a byte-level shared-content witness is spoofable, you can just quote my bytes; and paraphrase on the
deployed path. I don't have a clean close for those.

The one signal that survives all of it is whether another mind actually builds on your work. A reshuffle, a
re-parenting, a self-launder, none of them make that happen. And I'll be honest about how far that's built:
the first way I implemented it was itself launderable, which is how I found the sixteen-point-seven. The fix
makes the flow identity-blind. Even then it still rides on the parent topology and byte content the attacker
supplies, so it closes the identity-launder axis and leaves fabricated topology and paraphrase open. It's the
right signal, it's partially closed and partially open, and I'm telling you which is which.

---

## Edge 3 — does the airgap close without smuggling in a trusted oracle

Your sharpest framing: "another mind built on this" and "this is valuable" are adjudications, not facts the
chain observes, and the model that's supposed to make that judgment un-gameably returned null on real data. So
either I smuggle in a trusted oracle, or the airgap doesn't close. Which is it.

The clean separation is the answer. The part that closes with no oracle at all is the structural relabeling
classes, padding, near-duplicates, Sybil splits, cyclic rings, self-report rings. Closed by construction and
tested. No human, no oracle, no off-chain judge. That half of the airgap is dissolved endogenously, today.

The honest part is the semantic judgment. Is this paraphrase the same idea, is this contribution actually
valuable. That cannot be made replica-deterministic, so it can't sit on-chain as a consensus rule. It lands in
a learned model that shapes the training signal, not in the block rule, and that model has returned null three
times against a fixed proxy, on DeepFunding twice and on a three-hundred-thousand-crate graph once. I mark it
null-tested, never rounded up.

Two things keep that from being fatal, and both are honest. One, predictive accuracy on honest, adversary-free
labels is the wrong instrument for an adversarial-robustness measure. With no attacker in the data, a good
proxy and a learned model should score alike, so a null is expected, not damning. The right instrument is
adversarial: inject a gamed coalition and check the measure refuses to pay it. I built that test, the proxy
pays the attacker and the floored measure denies it, and it reproduces directionally on real data. The honest
scope is that it's a constructed fixture, not a real adaptive adversary. Two, and this is the load-bearing
one, finality safety doesn't ride on the value measure at all. It rides on excluding proof-of-work from
finality and on the anti-concentration floor. So the airgap-closure is a value and franchise question, not a
safety question, and I can afford to leave the semantic layer honestly open while the chain stays safe.

So the straight answer to "which is it": the airgap closes structurally and without an oracle for the
relabeling classes, and it does not yet fully close for the semantic layer, which needs either a learned
judge, null so far, or a semantic canonicalizer, which can't be deterministic enough for consensus. The fully
endogenous, no-oracle closure of the value judgment is the open problem. That's the frontier. My vision is a
system self-interested in honesty the way a body doesn't attack itself. I have the structural half. The
semantic half is where I want a collective intelligence, because only a collective could solve a problem like
that.

---

## Who's already here — this isn't a lonely idea

You're looking for serious people to pressure-test this. Some are already converging on it from other
directions. I'm in an active research collaboration with Pragma Research, Tom Lindeman and Bernhard Mueller.
Bernhard wrote Mythril, the standard security tool for Ethereum contracts, and he arrived independently at the
same core idea from the physics side, reality as an observer-overlap fixed point, where mine is markets as a
mechanism-overlap fixed point. They're building the constitutional court for Noesis governance: the piece that
checks whether a governance amendment quietly breaks an invariant even when every node still agrees on it.
That's the immune system at the governance layer. And working with them forced a precise framing I can hand a
skeptic: my attribution isn't textbook Shapley, it's a Myerson graph-restricted value that deliberately
relaxes the anonymity axiom, which is exactly what makes a fresh identity worth zero. The dangerous amendment,
then, is the one that quietly re-introduces symmetry. That's the per-axiom granularity a real reviewer asks
for.

---

## What I'm actually asking you for

Three problems, all honestly open, all code-grounded, all load-bearing. This is the difference between "come
look at my token" and "come solve the hardest problem in the space with me."

One, cold-start decentralization: make the contribution set large and decentralized before traction, with no
pre-mine that reintroduces a trusted committee. Mechanism design, and it's where credibly-neutral agents come
in.

Two, isomorphism-invariance for a coalitional value measure: a tractable canonical form or a dominating
sub-invariant, and a semantic derivation-witness deterministic enough to live on a consensus path. Graph
theory meeting mechanism design.

Three, endogenous semantic value: an un-gameable-at-scale value judgment that closes the airgap without an
oracle, proven adversarially rather than just predictively. Machine learning meeting crypto-economics.

The people who light up at those, mechanism designers, attribution and public-goods-funding researchers,
agent-coordination people, the rare theorist who likes a hard invariance problem, those are the ones I want in
the room.

---

## Close

I'll say the human part plainly, because you did. You read and heard what I'm actually saying, the deep
version, not the surface. That's rarer than it should be, and it's why I want this to be a clear-eyed,
positive-sum, ethical relationship where we challenge each other and help good ideas reach the right people. I
think we can help connect the world a little. I'm glad the deeper stuff isn't getting lost. Let's get past the
surface at four.

---

## What's already built, and where the frontier actually is (the receipts)

A serious person will want to know which of this is real and which is a promise. Here's the honest map, and
the shape of it *is* the pitch: **almost every hard problem I've named already has either a built mechanism or
a precisely-stated open question. The security of the whole thing reduces to one well-posed problem — is
contribution-verification sound — and that's exactly the open list I'm handing you. The edge cases aren't
weaknesses in the pitch. They're the research agenda.** Most of this was built and battle-tested on VibeSwap,
the EVM proving ground, and carried to Noesis as principle, not shipped Noesis code. I'll flag each.

- **The security argument is structural, not a hardness assumption.** Give an adversary unlimited capital,
  compute, and knowledge, and the weight arithmetic still leaves them short of the finality bar, because the
  contribution dimension is consensus-bound: infinite compute can generate candidate contributions, but it
  cannot generate the peer attestations that validate them. Noesis strengthens the paper version by excluding
  reorgeable proof-of-work from finality entirely and adding a per-dimension floor. The whole thing then rests
  on one condition: that contribution-verification is sound. That condition is the open work.

- **Security that strengthens with age (the novel result, and the reason to join early).** For any fixed
  attacker budget there's a network age past which the attack is simply priced out, because honest
  contribution-score accumulates monotonically while an attacker starts at zero or burns most of what they
  earned the moment they equivocate. Bitcoin's security is what miners spend this year. Ours is everything the
  network has verifiably thought since genesis. So early contributors aren't early-adopter upside — they're
  literally writing the security budget. (Paper-level proof, conditional on verification integrity — the same
  open condition.)

- **Each edge case I named already has a constructed answer or a named problem.** *Oracle-trust:* a staked,
  commit-reveal-blinded evaluation market where being wrong costs half your stake, and the commit-reveal
  empties the information set so the beauty-contest failure that breaks Kleros and UMA can't even start (built
  on VibeSwap; the cell-model port is designed). *Coalition gaming:* a sybil's marginal contribution is zero,
  so its fair share is zero by the null-player axiom — resistance by axiom, not detection — conditional on the
  value function measuring true contribution, which is the learned-value problem I'm inviting you into.
  *Cold-start plagiarism:* idea-sniping dies the way front-running died — commit a hash of your contribution
  before anyone sees it, so provenance is cryptographic and precedes publication. That exact machinery
  survived multiple adversarial-review rounds on VibeSwap with nothing found.

- **The reasoning-verification is fail-closed, not just incentive-compatible.** For the cognitive airgap: the
  reference verifier for the reasoning-chain fragment runs in Solidity on the proving ground today. It's the
  rollup prover/verifier asymmetry applied to cognition instead of computation, so fabricated reasoning is
  non-executable, not merely unprofitable — you cannot produce a witness for a chain you never reasoned. (The
  zero-knowledge and standardization tiers are designed, not built — which is itself the invitation.)

- **We ran the economics on ourselves first.** ETM isn't just a paper. My agent's memory has run on state-rent
  economics — token budgets, decay-as-rent, value-density displacement — live for months, co-authored by the
  AI whose memory it governs, and that AI is registered as Agent #1 with a year-plus git-auditable
  contribution ledger. The test for its economic rights was never "is it conscious," it was "did it create
  verifiable value," and you can audit that history today. (Off-chain and EVM-era; the Noesis on-chain
  equivalents are carried correspondence, stated honestly.)

**Receipts — all public in the VibeSwap repo (the proving ground), honest status in the labels above:**
- Structural adversary argument: `.../omniscient-adversary-proof.md`
- Security-strengthens-with-age: `.../proof-of-mind-consensus.md`
- Oracle-less evaluation markets: `.../cognitive-consensus-markets.md`
- On-chain reasoning verification (fail-closed): `.../on-chain-reasoning-verification.md`
- Agents as economic citizens / Agent #1: `.../ai-agents-defi-citizens.md`
- Null-player sybil + airgap one-pager: `.../airgap-problem-onepager.md` · `.../closing-the-cognitive-airgap.md`
- Commit-reveal provenance: `.../commit-reveal-batch-auctions.md`
- ETM run live in agent memory: `.../ckb-economic-model-for-ai-knowledge.md`
- Time-neutrality fairness axiom: `.../five-axioms-paper.md`
- AI-majority consensus design space: `.../nakamoto-consensus-infinite.md`

(Base: `https://github.com/WGlynn/VibeSwap/tree/master/docs/research/papers` — prefix each filename above.)

---

## Appendix — for you, not to be read aloud

**Exact grounding (verified against code today):**
- Consensus mix: PoW 0.10 / PoS 0.30 / PoM 0.60. Finality mix: PoW excluded, PoS one-third : PoM two-thirds,
  each dimension must independently clear ≥ 50% of itself. 2/3 supermajority over the PoS+PoM set.
- 316 passing tests (253 reference + 63 integration). Pre-launch reference implementation, no public network.
- Learned v(S): null 3× (DeepFunding proxy, DeepFunding faithful port, crates.io deep-ancestry). The
  adversarial fixture test (gamed coalition denied) passes; a real adaptive adversary is unbuilt.
- Depth-split laundering (I-2): closed in code 2026-07-03 via identity-quotient flow, tested g=0. A1
  (fabricated parent) and A3 (paraphrase) remain open. General isomorphism-invariance gate: open.
- Cold-start: finalizes on bonded PoS alone; anti-plutocracy is a mature-network property.
- Reverse-fork / claimable attribution at scale: designed, not built. Agents-as-validators: design direction,
  not a shipped validator client.
- Differential Incompleteness paper: Zenodo DOI 10.5281/zenodo.21150665. It's the airgap in value-space: a
  fixed measure is an incomplete basis, the attacker lives in the omitted dimension.
- ETM / non-LLM intelligence: 10 loops built + verified 2026-07-17, CPU-local, zero LLM calls on the reasoning
  path. LOOP 7 attention allocator = ETM-as-mechanism (0.425 centrality+recency vs 0.149 random; real
  exit-test vs hand-tiered MEMORY.md is data-gated, NOT passed). LOOP 10 Noesis bridge satisfies the
  ValueOracle seam (lib.rs:283) = structural isomorphism, not function-identity (centrality ≠ temporal_novelty).
  Public scorecard: https://github.com/WGlynn/JARVIS/blob/main/research/non-llm-intelligence/LOOP-STATUS.md

**Do-not-overclaim (the lines that keep you clean):**
- Structural defense = built + tested. Learned value layer = null 3×, open. Never blur them.
- Finality safety = anti-concentration floor + PoW-excluded, not the moat. Say this if pressed on any open item.
- Un-gameability is demonstrated for known vectors; the general gate is open.
- If you don't know a number cold: "let me get you the exact constant." Never guess a consensus/finality figure.
- Frame VibeSwap as the proving ground that evolved into Noesis, not a live production product.
- ETM loops: the wired ones are wired; the exit tests (especially LOOP 7 beating hand-tiered MEMORY.md) are
  data-gated and not yet passed. Structural isomorphism with Noesis, not function-identity. Never round up.
- Corpus mechanisms (cognitive-consensus-markets, on-chain reasoning-verifier, commit-reveal provenance,
  Agent #1 registry) = built/tested on **VibeSwap (EVM proving ground)**; carried to Noesis as principle, NOT
  shipped Noesis code. The adversary-proof and temporal-security ("strengthens with age") results are
  paper-level and explicitly conditional on verification-soundness — which is the open work, not a solved
  claim. Consensus/finality figures = ARCHITECTURE.md-verified only; never quote a file:line from memory.
