# Noesis: A Tamper-Proof Attribution Ledger with a Meta-Consensus on Value

> DRAFT prose for the network whitepaper. Status discipline: built / designed / open, never rounded up.
> Public doc: no external collaborator or library names. Revised 2026-09-30 after an adversarial
> five-seat council review (see `whitepaper/council-review-2026-09-30.md` for the full critique and the
> revision map this draft implements).

---

## 1. Introduction: the missing ledger

Decentralized systems have learned to agree on two things. They agree on ORDER, which is what Bitcoin
made trustless: who owns what, and in what sequence transactions happened. They agree on PROGRAM, which
is what Ethereum added: arbitrary rules that run over that order. On top of those two agreements the
ecosystem has bolted a series of funding mechanisms, each an attempt to point money at what a community
values. What none of them has is a tamper-proof record of the value itself: a ledger of attribution,
of who contributed what and how much it was worth, that other systems can build on and cannot quietly
rewrite.

This paper argues that such a record is a missing layer, and it presents Noesis as one design that
provides it: a tamper-proof attribution ledger with a meta-consensus on value, powered by smart nodes.
The claim is deliberately bounded, and it is useful to state what is and is not being asserted before
the argument begins.

### 1.1 What this paper claims, and what it does not

**It claims three things.** First, that a persistent, tamper-proof attribution record is a layer no
existing system provides, and that its absence is why pricing contribution keeps failing in the same
ways. Second, that putting that record on-chain is a good way to provide it, for the same reason Bitcoin
put ownership on-chain. Third, that the deterministic core of such a ledger is buildable, because it is
built: a reference implementation exists and is tested.

**It does not claim** that Noesis is the only or inevitable such system, that the lineage below forces
this design, or that pricing worth is solved. It is not solved, and a central result of this paper is a
precise account of why it cannot be fully solved by any closed structure.

**This paper is defeated if** attribution turns out not to need tamper-proofing to be useful, or if an
off-chain registry closes the same gap as well as an on-chain ledger does. Those are the conditions a
reader should hold the argument against.

### 1.2 Accounting versus value (the distinction the rest of the paper turns on)

The single most important thing to understand up front is a distinction between two things that sound
alike. A ledger can record **attribution-accounting**: who touched what, in what order, how deterministic
credit flows across a dependency graph. All of that is objective and can be made tamper-proof. A ledger
cannot, by any test internal to the graph, recover **value**: whether the work was actually worth
anything to anyone. The reason is measured, not asserted. In the reference implementation, coordinated
wash-building (a group of genuinely distinct participants building novel-looking work on each other's
worthless roots) is **graph-internally indistinguishable from honest layered collaboration: the
simulator measures 0% separation between them on structure alone.**

So the honest shape of what is built, stated once and plainly: **Noesis's L0 is a tamper-proof
attribution-accounting ledger, not yet a ledger of value.** Every step from accounting to value depends
on parts that are designed but not built (the value meta-consensus of Section 10) and on an external
anchor of real-world reception whose own Sybil-resistance is itself unsolved (Section 15). The rest of
the paper is organized so that a reader can see exactly where the built accounting ends and the
value bet begins.

The following table makes the comparison, and the discount, visible in one place rather than diffused
across the prose. Status markers are built / designed / open, and they are not rounded up.

| System | Consensus object | Tamper-proof persistence of attribution | Finality over attribution | Sybil / wash resistance | Ground-truth source for value |
|---|---|---|---|---|---|
| Bitcoin | Transaction order + ownership | n/a (prices only endogenous money) | n/a | PoW (Sybil-resistant) | endogenous (money is a chain fact) |
| Quadratic funding / Gitcoin | none (off-chain allocation of a pool) | no | no | weak (collusion / Sybil rings) | donor breadth, prospective |
| RetroPGF (Optimism) | none (round allocation) | no (per-round) | no | panel / badgeholder set | human panel +, in later rounds, metrics |
| Deep Funding | none (off-chain weights per round) | no | no | model competition vs human jury | capturable jury (small by design) |
| Bittensor / Yuma | validator-weighted score vector | partial (weights are chain state; no provenance ledger) | yes (over the weight vector) | stake-weighted; no provenance / wash model | validator judgment |
| Proof-of-useful-work | useful computation | n/a | n/a | work cost | n/a (does not price contribution) |
| **Noesis (this paper)** | **attribution (deterministic accounting), weight-2/3 consensus input conjoined with a capital floor** | **yes (built)** | **yes, over the deterministic accounting slice (built); value stays revisable (designed)** | **deterministic skeleton built; wash open (0% graph-internal separation)** | **staked smart-node meta-consensus (designed) anchored on external use (open)** |

The row that matters is the last one read against its own status column: what is tamper-proof and final
is the accounting, not the worth.

## 2. What a blockchain is: consensus on order

Bitcoin (Nakamoto, 2008) solved one precise problem: double spending without a trusted third party. Its
mechanism is consensus on the ORDER and ownership of transactions. Because money on that ledger is
endogenous, the coins are defined by the ledger itself, Bitcoin never had to reach outside its own
records to know what was true. It dodged the gap between the chain and the world by making the one thing
it prices, its own money, a native fact of the chain.

This is the observation the paper's later slogan rests on, so it is worth stating as its own conclusion
rather than assuming it. Bitcoin priced exactly one endogenous quantity and priced it perfectly, and a
generation of builders extrapolated from that success to the belief that blockchains price value in
general. They never did. A chain can finalize what is defined inside it; worth is defined outside it.
Order is objective and fast to agree on. Value is neither. Holding that distinction clearly is the whole
of what follows: the systems after Bitcoin are attempts to price value, and each inherits the difficulty
Bitcoin avoided by never trying.

## 3. Programmable coordination: Ethereum and DAOs

Ethereum (Buterin, 2014) generalized the ledger from a fixed transaction type to arbitrary programs.
On top of it, decentralized autonomous organizations became on-chain institutions: shared treasuries
and collective decisions executed by code rather than by trusted officers.

The gap DAOs leave is the one that motivates everything after. Governance in a DAO is almost always
token-weighted, and token weight is capital weight. One-token-one-vote is not one-mind-one-vote, and a
DAO has no native measure of who actually contributed. It can decentralize the execution of a decision
without decentralizing the judgment of merit that should inform it. The systems in the next sections
are, in effect, attempts to supply that missing measure of contribution.

## 4. Democratic funding: quadratic funding

Quadratic funding, introduced as Liberal Radicalism (Buterin, Hitzig, and Weyl, 2018) and deployed at
scale by Gitcoin, is the first of those attempts to earn broad adoption. Its rule is precise: a project
is funded at the square of the sum of the square roots of its individual contributions, F = (sum of
sqrt(c_i))^2, and the matching subsidy is that funded amount net of what was actually contributed,
F minus sum(c_i). That formula weights the BREADTH of support over the SIZE of any single donation.
Under the idealized model of the original paper, no collusion and no Sybils, divisible contributions,
quasi-linear utilities, and an unbounded matching pool, the mechanism is efficient at Nash equilibrium:
it implements the utilitarian optimum. Because the ideal matching required scales without bound and real
pools are finite, deployments use a budget-constrained variant, which the original paper already
discusses and which Gitcoin and clr.fund implement in practice through pairwise-bounded, saturating
matching.

The point to keep straight is that the efficiency result and the failure modes are the same fact seen
from two sides. The theorem holds only under its idealizations, and every real deployment violates at
least one of them: collusion and Sybil rings manufacture exactly the breadth the formula rewards,
finite pools break the unbounded-pool assumption, and the mechanism funds PROMISES prospectively,
betting on what a project claims it will do rather than on what it has done. These are not incidental
weaknesses bolted onto a clean mechanism; they are the hypotheses of the optimality theorem, failing.

## 5. Impact over promises: retroactive public goods funding

Retroactive public goods funding, developed by the Optimism Collective, answers the last of those
limits by inverting the timing. Instead of funding promises ex ante, it rewards demonstrated impact ex
post, under the guiding principle that impact should be profitable. Early rounds allocated by a panel of
community evaluators, and the shift from prospective to retrospective judgment is a real improvement: it
is easier to recognize value that has already appeared than to predict value that has not.

It is only fair to note that the mechanism has not stood still. Across successive rounds the design
moved deliberately away from a small subjective panel toward larger badgeholder and guest-voter
populations and toward metrics-based and data-driven evaluation, precisely to blunt the capture concern.
The limitation this paper presses is therefore about the current form, not a frozen 2023 snapshot. Even
with a larger population and metric inputs, the judgment of impact remains a periodic, off-chain
allocation decided by a human-governed process, without a tamper-proof persistent record of the
attribution itself and without a continuous, economically-bonded way to keep that judgment honest
between rounds. Retroactive funding moved the judgment to the right side of time; Noesis's delta is over
that current form (tamper-proof persistence, continuous rather than round-based judgment, and
staked-and-slashable judges), not over the panel version it has already outgrown.

## 6. Value as a graph: Deep Funding

The nearest prior art, as of the 2025 Ethereum round, is Deep Funding (Buterin, 2025). It is the
clearest widely-discussed mechanism to treat attribution itself as the object to be computed, and its
two central ideas are the ones every later system, including this one, has to reckon with. Because the
program is active and evolving, the characterization here is pinned to that round and should be read as
a dated snapshot rather than a permanent description.

The first idea is **value as a graph**. Instead of asking the unanswerable question "how much did
project X contribute to the world," Deep Funding asks a tractable one: "of the credit for outcome Y,
how much belongs to each dependency X?" This turns a question with no ground truth into a question
about the edges of a dependency graph. For the initial Ethereum round the graph connected open-source
repositories to their upstream dependencies, on the order of tens of thousands of edges.

The second idea is **distilled human judgment**. Rather than trust a single model or a single
committee, Deep Funding runs an open competition in which anyone may submit an AI model that proposes
weights for the edges of the graph, and a human jury randomly spot-checks the results. The jury is small
by design: the whole point is that the human judgment is distilled into a scalable model so that most
edges are model-decided, which is itself a partial answer to the capture concern, not a naive
dependence on a committee.

Deep Funding gets two things right that this paper builds on directly. It reframes an absolute,
oracle-free question into a relative, graph-structured one, which is the only form of the question
that is tractable at all. And it accepts that judgment must ultimately be checked against something
outside the model, rather than pretending a model can certify its own worth.

What it leaves open is the next layer of the problem, and the honest framing is a tradeoff rather than a
defect. Deep Funding is off-chain and runs one round at a time, so each round produces a set of weights
and then ends; a round-based process with persistent published artifacts is a legitimate design, not a
strictly worse one than a continuous meta-consensus, and the two trade different things. The specific
gaps Noesis targets are three. There is no tamper-proof, persistent record of attribution that the next
round or any other system can build on and cannot rewrite. The ground truth is a human jury, which is a
capturable oracle, and because the jury's role is to check the models, capturing the jury is the same as
capturing the value. And there is no structural treatment of coordinated wash, where a ring of
genuinely distinct participants builds on each other's low-value work to present a legitimate-looking
dependency structure. Deep Funding establishes that the question is the right one: value is a graph, and
the graph should be weighted by machine judgment checked against humans. Making the answer durable,
continuous, and trust-minimized is a separate step, and it is where Noesis begins.

## 7. Noesis: one design that closes the gap

The systems above can be read as a sequence in which each decentralized one layer of coordination and
left another exposed. That reading is useful as motivation, but it is important not to overclaim it.
These systems were not designed as steps toward an attribution ledger, and the sequence does not force
Noesis's shape. The same history is equally consistent with other end-states: an off-chain attribution
registry anchored only loosely to a chain, a zero-knowledge attestation scheme with no shared ledger at
all, or an optimistic value layer that posts claims and resolves them by fraud proof. Noesis is one
design among these, and the argument for it is structural, not teleological: of the exposed layers, a
persistent and tamper-proof record of attribution is the one none of the above provides, and an on-chain
ledger is a good way to provide it for the same reasons it was a good way to record ownership.

Noesis takes Deep Funding's graph and machine judgment and makes three changes.

First, it puts the attribution on a **tamper-proof ledger**. Provenance, commit ordering, and a
soulbound record of contribution standing are objective, fast facts that a ledger can make permanent,
exactly as Bitcoin makes ownership permanent. The attribution graph stops being a per-round artifact
and becomes a persistent, append-only record that anything downstream can build on. This is the part
that is built, and, per Section 1.2, it is accounting, not yet value.

Second, it replaces the one-shot, jury-checked competition with a **continuous meta-consensus on
value**. Many smart nodes run their own models off-chain and commit pairwise judgments of relative
worth; the chain never runs a model, it agrees only on what each node claimed and disciplines that
claim with stake and slashing. This turns the capturable single jury into a standing, adversarial,
economically-bonded population, and it turns a one-time weighting into a value estimate that keeps
updating as real use arrives. This is the part that is designed, not built.

Third, it is **explicit about the limit no such system escapes**. Worth is reception by other minds,
which is exogenous to any closed structure; a coordinated ring can be graph-internally
indistinguishable from honest layered work, as the 0% separation result shows. Noesis does not claim to
dissolve this. It bounds it, it keeps the value estimate revisable rather than final, and it separates
the fast objective facts it can make permanent from the slow judgment it never finalizes.

The result is the shape developed in Part II: a hard, deterministic attribution ledger; a soft,
revisable meta-consensus on value that never overrides it; and a single deterministic channel between
them. Stated plainly: the ledger is built and tested, the value meta-consensus is designed, and the
hardest mile, a learned value function validated on real external labels, is open and gated on data
rather than on effort. What Noesis offers today is not a solved pricing of worth but a durable,
tamper-proof place to record attribution and a trust-minimized design for keeping judgment honest, which
is a structural bet with a partial build under it.

### 7.1 Why this is a conclusion, not one more funding mechanism

The honest floor first: nothing below is earned by a running network. The claim here is about what
KIND of object Noesis is, not about a shipped property, and the property that would make the word
"sound" deserved, an un-gameable measure of contribution, is exactly the open part per Sections 1.2
and 12. Read what follows as the design's intent and the reason it is framed as a terminus, not as a
demonstrated result.

With that stated, Part I invites a question it does not by itself answer: is Noesis simply the next
funding mechanism in the line, or a different kind of thing? What settles it is what each system
reaches consensus on. Quadratic funding, retroactive funding, and Deep Funding are allocation
mechanisms: each decides, under a policy, how a pool is split. Bitcoin is not an allocation mechanism
at all; it is an infrastructure primitive, consensus on who owns what, with no policy above that one
agreement. Allocation mechanisms run on top of primitives; they are not primitives themselves.

Noesis is attribution-first, not allocation-first, and it supplies the second primitive. Bitcoin's
primitive is consensus on ownership, who holds what. Noesis's primitive is consensus on contribution, who made
what. Those are different agreements, and the second sits upstream of the first: ownership is a claim
on value that already exists, while attribution is the record of where that value came from. A ledger
that can only see ownership is structurally blind to contribution, which is why every mechanism built
on such a ledger must reach outside the chain for a human judgment of merit (Sections 4 through 6).
Noesis is the attempt to make that judgment a native ledger object rather than an off-chain
annotation. Sound money was the first missing primitive; sound attribution is the second, with "sound"
aspirational for the reason stated in the floor above.

One further difference is what makes Noesis more than a neutral substrate. Bitcoin's primitive is
normatively silent: whoever commands the most work wins the next block, and the protocol expresses no
preference about the concentration of that power. Noesis's primitive is not silent in the same way. It
carries exactly one embedded normative constraint and is otherwise as silent as Bitcoin. The constraint
is the anti-concentration conjunction of Section 12: finality routes through both a contribution
dimension and a capital dimension, each of which must independently clear a floor, so neither finalizes
worth alone. The thesis-salient direction is that capital cannot finalize worth without contribution
clearing its floor, which is the anti-plutocracy property; but the floor runs both ways, capital
retains a necessary veto by the same mechanism, so the honest statement is a two-key conjunction, not a
cap on capital. This is the only policy Noesis writes at the protocol level, and even it is a design
property resting on the gameable proxy of Section 12, not a settled guarantee.

Everything above that one constraint is left to the market, exactly as Bitcoin leaves everything above
ownership to the market. The distinction that makes this precise is pool-independence: an attribution
is a relative credit share, a fact about the dependency graph that exists with no treasury, no round,
and no budget attached, whereas an allocation is that share multiplied by a pot someone chose to fund
under an eligibility policy and a round they set. The predecessors cannot output anything until handed
a pool; Noesis finalizes the share and fixes neither the pool nor the policy. It does not decide what
is worth building, what should be produced, or what anything should be priced at. The one honest
caveat, carried rather than hidden, is that where Noesis itself converts shares into on-chain reward,
that conversion is an allocation policy and should be named as one, and it is designed, not built;
what is built and policy-free today is the share. The restraint is deliberate: a primitive that began
prescribing outcomes would stop being infrastructure and become a plan, and the design's whole claim
is to be the missing rail, not the economy that runs on it.

This is the precise sense in which Noesis is a conclusion rather than a rung. The funding mechanisms
answer "how should this pool be split," and each answer is a policy the next mechanism revises. Noesis
answers a prior question, "what is the tamper-proof record of contribution, and what is the one
structural constraint on finalizing it," and that question, unlike a pool-splitting policy, has no
successor that makes it obsolete. It is where the line terminates, not because Noesis is a better
allocation, but because it finalizes the attribution that any allocation must read, and leaves the
pool and the policy to the market.

### 7.2 Predecessors as upstream contributions, not rivals (a positioning stance)

A short note on stance, kept modest because the status is asymmetric: the predecessors here have shipped
live systems and Noesis has not deployed. Noesis stands on quadratic funding, retroactive funding, and
Deep Funding. It is not a rival to them but an attempt to make their shared object, attribution,
persistent and continuously judged. That is an intellectual debt, stated as one, and it does not depend
on Noesis's own value metric to be true.

There is a design orientation that follows, phrased as a goal rather than a prediction. Noesis is
designed to be a substrate that other systems can attribute onto, so that recording its own lineage, and
later interoperating with adjacent systems, is a natural direction rather than a retrofit. The stronger
descriptive claim, that networks are trending toward merging onto a shared substrate, is not made here;
the current evidence runs the other way, with funding stacks remaining deliberately separate. The claim
is only that Noesis is built to be mergeable, not that the field is merging.

This is the sense in which, by the end of the argument, Noesis can be called the value chain Bitcoin is
mistaken for. Bitcoin priced one endogenous thing and escaped the airgap by never reaching outside the
chain. Noesis attempts the harder thing one layer up: to make contribution legible on a ledger while
being honest that worth, unlike money, remains partly exogenous, and to build the machinery that narrows
that gap without pretending it is closed.

---

# Part II - The Noesis network

## 8. Architecture overview: the three-layer split

The instinctive way to ask where a value protocol belongs is to ask whether it should live inside
consensus or outside it, embedded or separate. That framing is incomplete, because the value protocol is
not one thing. It splits along a single natural seam, the line between what is deterministic and
fast-final and what is subjective and revisable, into three parts.

The first is **L0, a hard deterministic skeleton embedded in consensus**. Provenance, commit ordering,
identity and contribution-standing keys, the conservation invariants of the Cell model, and the
deterministic arithmetic of value flow: novelty, a similarity floor, flow propagation with damping,
Shapley and Myerson credit, and cycle detection. Every one of these is integer arithmetic over the
finalized graph. It replicates bit-for-bit across every node, which is exactly the property consensus
requires, so it belongs on-chain. This layer is built and tested.

The second is **L1, a soft revisable meta-consensus stacked on top**. This is the subjective judgment
of whether a contribution is actually worth anything: the learned quality seed, reputation, the fuzzy
part no integer captures. It runs off the hard path as its own opt-in second consensus, and it is
allowed to be revised as the world reveals more. This layer is designed, and largely open.

Between them is **the seam, a single narrow channel**. L1 reaches L0 through exactly one thing: a
deterministic integer value per item that satisfies a fixed contract, pure and bit-identical and
shape-preserving. Nothing else crosses. The hard layer consumes a vector of integers; it never sees the
model, the reputation, or the judgment that produced them.

Two independent constraints make this split the preferred resolution, and they arrive at the same line
from opposite directions. The first is the **consensus-versus-value tension**: consensus demands fast
agreement on a state that becomes virtually irreversible, while realized value is slow, fluid, and
revisable, so finalizing a value at commit time freezes a guess that finality then makes permanent,
which is wrong by construction. The second is the **determinism wall**: neural and small-model inference
is not bit-identical across hardware, because float ordering and library versions differ, so it cannot
run inside a hard state transition.

Neither constraint alone forces a unique design, so it is worth naming the main alternatives and why
they are not chosen here rather than asserting the split is forced. Stripping value out of the chain
entirely is rejected because the value judgment then loses its only un-gameable anchor, commit-reveal,
stake and slashing, ordering, and identity, and the wash problem returns in full as just another
off-chain opinion. An optimistic or fraud-proof value layer, which posts value claims on-chain and
resolves disputes after the fact, is a genuine alternative; it is set aside here because value disputes
have no objective adjudication the way double-spends do, so there is no sound fraud proof to fall back
on, only another round of subjective judgment. Given those, stacking the subjective layer through a
narrow deterministic seam is the design this paper develops. It is preferred, not proven unique.

## 9. The tamper-proof attribution ledger (L0), built

The hard layer is the part of Noesis that actually runs today, and it is the part that earns the word
"ledger." It records, permanently and in agreed order, a set of objective facts about contribution:
who submitted what, in what sequence, and how deterministic credit flows across the resulting
dependency graph.

The objective facts come first. Provenance and commit ordering fix who contributed what and when,
using the same commit-reveal discipline that denies any participant the ability to backdate or reorder
their work after seeing others'. Contribution standing is recorded as a soulbound key, an identity that
accrues franchise through contribution and cannot be bought or transferred like capital. The Cell model
enforces conservation (value cannot be minted from nothing, the way a UTXO ledger forbids coins
appearing or vanishing except by the rules), so the accounting cannot silently leak or inflate standing.

On top of those facts sits the deterministic value skeleton, and each piece is best understood by what
it prevents. Temporal novelty with a similarity floor denies credit to near-duplicates, so re-submitting
existing work earns nothing; the similarity mechanism is coverage-based near-duplicate detection in the
lineage of shingling and min-wise hashing (Broder, 1997), not a new primitive. Flow propagation with
damping rewards upstream work through the things built on it while preventing credit from circulating
without bound. Shapley and Myerson credit assign each contributor its marginal worth to the coalitions
it joins. A precision note matters here: computing the Shapley value exactly is #P-hard in general
(Deng and Papadimitriou, 1994), so the exact closed form used in Noesis holds only for the restricted,
submodular set-cover structure of the dependency value function (following the set-cover Shapley of
Sivill and Flach, 2023); for general value functions the system falls back to permutation sampling in
the Data-Shapley lineage (Ghorbani and Zou, 2019). Cycle detection uses a combinatorial Hodge
(Helmholtz) decomposition (Jiang, Lim, Yao, and Ye, 2011), which flags the A-credits-B-credits-A
inconsistency loops a naive credit rule would reward; strictly it yields a global inconsistency measure,
not merely the presence of a cycle.

Every one of these is pure integer arithmetic, mirrored in fixed-point on the virtual machine so that
settlement is bit-identical to the reference (the seam parity tests prove the off-chain and on-chain
arithmetic agree to the bit). The honest floor has two parts. First, this is a reference implementation
with a green test suite, not a production network under adversarial load, and there is no live
deployment. Second, and more important, a passing suite demonstrates deterministic replication and
functional correctness against the adversary vectors enumerated so far; it does not establish
non-interference, the property on which finality safety actually rests (Section 12). The two most
consequential problems the project has found, coordinated wash-building and depth-axis lineage
laundering, were discovered outside the original test model and are not yet closed. "Built and tested"
means the accounting skeleton exists, runs, and replicates; it does not mean the value or the safety
property is proven.

## 10. The meta-consensus on value (L1), designed and open

The soft layer is where Noesis parts company with everything that runs a value judgment once and then
stops. Instead of a single round that produces weights and ends, L1 is a standing second consensus
whose entire job is to keep judging worth as evidence accumulates.

Its instrument is commit-reveal pairwise comparison. Rather than ask any node for an absolute score,
which has no ground truth, the protocol asks for relative judgments, "this contribution is worth more
than that one," committed under the same reveal discipline as the rest of the chain so that no judge
can condition its vote on the votes it has already seen. Eliciting value from relative comparisons and
aggregating them into a ranking is a mature field, and the paper claims no novelty in the elicitation
itself: the natural reference points are the Bradley-Terry-Luce and Thurstone comparison models, Elo,
and Kemeny-Young rank aggregation, with the modern analogue being the pairwise-preference modeling used
in RLHF. The aggregation rule is deliberately left open here, but the candidate family is named rather
than gestured at, and HodgeRank is the most natural fit because the deterministic layer already uses the
Helmholtz-Hodge decomposition, so a consistent-plus-cyclic split of the judgment flow comes for free.
The aggregated ordering is deliberately never final: it mean-reverts, because a value that can never be
revised is a value that will eventually be wrong and stuck.

This is the layer that answers the capturable-jury problem named in Part I. Deep Funding's weighting is
checked by a small human panel, and a small panel can be courted or bought. L1 replaces the single panel
with a standing, adversarial, economically-bonded population of judges whose claims are staked and
slashable. Honest floor: this is a design sketch, not a specification. The shape of the claim object,
the sharding of judgments across provenance subgraphs, the exact aggregation rule, and the vesting of
soft judgments are open. L1 is the least-built part of Noesis, and, per Section 1.2, it is the part on
which the whole move from accounting to value depends.

## 11. Smart nodes as value oracles

The clearest way to understand this layer is by analogy to the move Bitcoin already makes. Bitcoin does
not verify that your computation was useful; it verifies a cheap-to-check commitment backed by real
cost, and it makes cheating negative-expected-value. Noesis makes the same kind of move one level up.
The chain does not verify an inference and does not re-run a model. It verifies a staked, slashable
commitment to a claim, and arranges the economics so that lying is negative-expected-value. Said
precisely: the primitive's job is to convert a non-deterministic inference into a deterministic,
economically-bonded claim about what a model said. That is the whole of what it does, and seeing it as
scoping rather than as a gap is the point.

With that framing, the mechanics are simple. The intelligence, the AI and small language models that
read a contribution and form a view of its worth, runs off-chain at the nodes. A model's output is
non-deterministic across hardware, so Noesis treats it as a committed fact, "node X claims A is worth
more than B," committed under reveal and backed by stake. The chain agrees only on what was claimed, and
the aggregation of those claims and the slashing arithmetic stay fully deterministic. Non-determinism
becomes data; consensus over the data stays deterministic.

None of this is a new idea, and it should not be presented as one. Reaching consensus over committed
claims disciplined by stake and slashing is the standard decentralized-oracle construction, in the
lineage of SchellingCoin (Buterin, 2014), Chainlink, and Augur, and commit-reveal itself predates
Bitcoin (Blum, 1981). The only delta Noesis claims is the coupling: tying those committed claims to an
exogenous, use-based anchor and to the deterministic slash arithmetic of the hard layer. The hard part
is exactly what this construction does not solve on its own. It does not make a committed judgment
honest, and it does not make judges independent. A colluding majority can still settle a confident wrong
answer, the classic oracle-capture attack, and garbage committed in good faith is still garbage. Those
remain the job of stake and slashing, enforced plurality, and the exogenous anchor of Part III, and
naming them is scoping the primitive honestly, not retreating from it.

The trustless direction for this layer is zero-knowledge machine learning: a proof that a committed
model actually produced a given output on a given input. It is worth being exact about what that buys,
because it is easy to overclaim. zkML establishes execution faithfulness only, that the claimed model
really ran as claimed. It does nothing about a colluding majority committing a confident wrong judgment,
nothing about judge independence, and nothing about garbage inputs. It closes the "did the model really
run" sub-problem and leaves oracle honesty open, so it is an endgame for execution integrity, not an
endgame for the oracle problem.

One research direction lives here and is worth naming with its honesty attached. If nodes are learning
systems, the graph among them need not be a citation graph at all; it can become a transfer graph, one
whose edges measure how much one node's contribution actually changed another node's behavior. That
would turn attribution from "who cited whom" into "whose work measurably improved whose," which is much
closer to worth, and it is the most promising route the project has found around the 0% graph-internal
separation result, because a measured behavioral change is not a graph-internal quantity. It is also
much harder, and it inherits a sharp objection: a raw behavioral delta rewards the transfer of an
answer key as readily as the transfer of insight, because an answer key is the highest-transfer object
there is. Distinguishing teaching from leakage is unsolved. This is a research direction with a known
open problem at its center, not a feature.

## 12. Consensus on value: order finalizes fast, worth stays revisable

The tension between consensus and value is not something Noesis dissolves; it is something Noesis places
carefully. The resolution is to finalize the objective facts fast and irreversibly, order, provenance,
conservation, and to keep the value judgment a slow overlay that never finalizes and is always open to
revision.

A careful reader raises the obvious objection immediately, so the paper raises it first and states the
built reality precisely rather than reaching for a slogan. In a Proof-of-Mind chain, resistance to
Sybil attack is contribution itself, so the franchise that weights consensus is value-derived; there is
no pretending consensus touches no value. It is therefore wrong to say "attribution IS the consensus
object" as if attribution finalized alone. What actually finalizes is a conjunction. Finality routes
through a fixed mix in which contribution standing carries the larger weight (on the order of two-thirds)
and capital stake the smaller (on the order of one-third), and an anti-concentration floor requires each
dimension to independently supply at least half of its own weight. In particular the capital dimension
must independently clear its floor, so capital retains a necessary veto on finality by design. This is
not a concession; it is the anti-plutocracy thesis working in both directions. The honest statement is
that **attribution is a first-class, larger-weight consensus input conjoined with a capital floor, not
the sole consensus object.**

Within that, the line the design must hold is narrower and sharper. Finality may read only the slice of
the value stack that is deterministic, vested, and dispute-gated, and the learned judgment must never
touch the finality path. In the implementation, finality weight is computed from the deterministic
novelty oracle through a vested, dispute-gated standing path; the learned outcome model and any pairwise
value judgment do not feed it. A contribution counts toward finality weight only after aging past a
vesting window, and is stripped forward-only if refuted during that window, so consensus finalizes on
standing that survived both a vesting delay and a dispute window rather than on a fresh guess.

Now the honest floor, which is the most important in the paper and is stated without softening. These
containments bound the risk; they do not close it, and each is weaker than it first looks. Reading the
vested proxy is a definitional choice (the system defines finality to read the proxy), not a dissolution
of the problem. Slow vesting is a rate-and-cost bound, not a proof of unprofitability; whether it deters
depends on the attacker's discount rate against standing value. And anti-concentration assumes the two
dimensions fail independently, which a sufficiently capitalized ring holding both dimensions violates.
Worse, the deterministic proxy itself is game-able: coordinated wash shows 0% graph-internal separation,
so a ring that games novelty and survives the windows does inflate standing, and inflated standing does
weight finality. And determinism is not objectivity: a staked, attacker-shaped integer is perfectly
deterministic and still wrong, and once the hard layer finalizes it, worth has been finalized through
the seam.

This forces a precise restatement of the property the design actually rests on, and a correction of its
name. The safety claim is not "nothing subjective reaches finality," which, taken literally, is false:
the deterministic novelty integer is a subjective-value proxy and it does reach finality. The defensible
property is narrower: **no non-deterministic or learned output reaches finality; only a deterministic,
vested, dispute-gated integer does.** This should also not be called "non-interference" without
qualification, because that term already has a precise meaning in information-flow security (Goguen and
Meseguer, 1982) that would require naming a security lattice, a high/low labeling, and an observation
model. Pending that formalization, this paper states the property operationally (no learned output on
the finality path) rather than borrowing the name, and proving it, as a bounded audit of the finality
input surface, is the gate before any of L1 is built.

Having stated the demolition in full, it is equally important to state what remains standing and
bounded, because the residual is a known class rather than an open door. The coupling converts the
attack from "game the value layer OR hold capital" into "game the value layer AND independently clear
the capital floor." The residual is therefore severe undetected gaming that also commands an
independent capital majority, which is the same shape as a Bitcoin 51% assumption-failure: outside the
threat model the system defends, not a hole inside it. That is a defensible place to stand, provided it
is named as exactly that and not rounded up.

Finally, a correction the value documents must carry, flagged here because it bears on finality. Novelty
is currently used two ways at once, as the proxy that weights finality and as a seed that pays reward,
and a single quantity cannot be both the objective thing consensus leans on and the paying thing
contributors chase. The intended fix is to split it: structural novelty (canonical-form distinctness)
may be objective and may gate finality but must not pay; value novelty may pay but is the subjective open
seam and must stay off the finality path. That reconciliation is open, not done.

## 13. The value function: a pointer, not a re-derivation

The internals of the value function, how a defensible number is coaxed out of pairwise judgments, are
the subject of the companion mechanism paper, and this network paper deliberately does not re-derive
them. A short summary places the piece, with the status markers kept honest.

Value is elicited from relative judgments rather than absolute scores, because only the relative
question has any hope of a ground truth. Naive additivity is a trap, since contributions interact, so
credit is assigned by the Shapley and Myerson machinery, with the exactness caveat already stated in
Section 9 (closed form only for the submodular set-cover structure; sampling otherwise). Temporal
novelty is not strategyproof in the general dominant-strategy sense, and the paper should not use that
word loosely. The precise, tested property is narrower: duplicate and subset-padding submissions earn
zero marginal novelty under the consensus-sourced commit order, so the score is Sybil- and
padding-resistant, and that property is itself conditional on the order being sourced from consensus
rather than arranged by the producer. The score is provably NOT resistant to coordinated wash-building,
which is exactly the 0% separation result; the honest position of Sections 1.2 and 12 must not be
rounded up here into general strategyproofness.

The learned evaluator deserves the same care. Its outcome factor is clamped to the interval [0,1] and is
monotone-lowering: it can only attenuate the deterministic seed, never raise it. That bounds one attack,
over-crediting, and makes a corrupt model unable to inflate a contributor above its deterministic seed.
It does not make a corrupt model harmless. A downward-only clamp still permits targeted suppression:
driving honest contributors' seeds toward zero, starving their standing, and shifting which coalitions
clear the contribution dimension, a liveness and fairness attack that this clamp does nothing about.
And the clamp protects the factor, not the seed it multiplies; the flow seed itself remains pumpable
through lineage laundering, a found open gap on the depth axis in which self-built lineage is laundered
into apparent external flow. So the honest claim is: the learned factor cannot inflate value, but it can
suppress, and the seed it multiplies is not itself safe from wash or lineage pumping. For the
derivations, axioms, and proofs, the reader is referred to the companion mechanism whitepaper.

---

# Part III - Theory, novelty, and honesty

## 14. Related work, and where Noesis differs

Noesis sits inside several bodies of work at once, and the honest way to place it is one lineage at a
time, each with the specific thing Noesis does differently rather than a blanket claim of superiority.
The flagship positioning claim, that Noesis makes attribution a first-class consensus object, is only
credible if its nearest neighbors are named rather than excluded by a scoping clause, so this section
names them.

On **cooperative game theory and attribution mathematics**, the ancestors are the Shapley value
(Shapley, 1953), the Myerson value for games with a graph structure (Myerson, 1977), the Banzhaf power
index, and the machine-learning data-valuation line that adapted Shapley to training data (Ghorbani and
Zou, 2019; set-cover Shapley, Sivill and Flach, 2023). Noesis uses these as the on-ledger deterministic
credit skeleton, computed continuously over the live provenance graph and hardened with anti-Sybil
damping and a similarity floor, not as an offline analysis run once. The math is classical; running it
inside the consensus object is the move, and the novelty is in the venue and the hardening, not the
theory.

On **public-goods funding**, the ancestors are quadratic funding and Liberal Radicalism (Buterin,
Hitzig, and Weyl, 2018), its budget-constrained practice (Gitcoin / clr.fund), retroactive public
goods funding (Optimism Collective), and Deep Funding (Buterin, 2025). These are allocation mechanisms:
they decide how a pool is split, prospectively or retrospectively, and run on top of a chain or
off-chain. Noesis makes attribution a persistent, tamper-proof ledger object rather than a per-round
allocation, and replaces the capturable jury or one-shot competition with a staked value
meta-consensus.

On **reputation and credit-graph systems**, the nearest neighbors to the flagship claim, the ancestors
are SourceCred (contribution-weighted credit via weighted PageRank), Colony (decaying,
work-earned reputation weighting governance), Coordinape (peer credit allocation), and EigenTrust
(transitive trust aggregation). These are the systems that come closest to treating a contribution or
reputation quantity as a first-class object the network maintains. The precise delta is worth stating
rather than eliding: they compute or weight governance by a contribution graph, but they do not finalize
the attribution graph itself as tamper-proof ledger state, they do not separate a deterministic credit
skeleton from a learned judgment layer, and they do not carry soulbound provenance standing with a
dispute-gated finality path. Noesis's claim is not that a credit graph is new; it is that finalizing the
credit graph as consensus state, with that separation, is.

On **consensus and useful or attributed work**, the ancestors are Nakamoto consensus (2008), proof of
stake, the proof-of-useful-work lineage (where the consensus object is useful computation, not an
attribution of credit among contributors), and, most pointedly, Bittensor's Yuma consensus, which is
the single closest competitor and must be named. In Bittensor the object the network reaches consensus
on already IS a weighted attribution: a stake-weighted vector of validator scores over contributors.
The difference is specific. Yuma reaches consensus on an aggregated weight vector; Noesis reaches
consensus on a soulbound provenance ledger plus commit-reveal records of what each node claimed, keeping
the learned judgment off the finality path rather than baking it into the finalized weights, and
separating the deterministic accounting skeleton from the revisable value judgment. The honest form of
the novelty claim is therefore not "attribution is never the consensus object elsewhere," which Bittensor
falsifies, but "we are not aware of a system that finalizes a tamper-proof provenance-and-attribution
ledger while holding the learned value judgment off the finality path, and we invite counterexamples."

On **decentralized oracles**, the value-oracle layer of Section 11 is squarely in the SchellingCoin
(Buterin, 2014), Chainlink, and Augur lineage, with commit-reveal from Blum (1981); the claimed delta
is only the coupling to an exogenous use anchor and deterministic slashing, as stated there.

On **AI in governance**, the closest framing is the familiar "AI is the engine, humans are the steering
wheel." Noesis accepts the division of labor and sharpens the discipline: rather than trusting a jury to
check the AI's output, it disciplines the AI's claims through the chain, with commitment, stake, and
slashing, so that being wrong or dishonest is costly rather than merely reviewable.

Read honestly, Noesis is most novel as a systems-integration thesis, finalizing a tamper-proof
attribution ledger with a strict separation between a deterministic accounting skeleton and a staked,
revisable value judgment. It is least novel exactly where it might be tempting to claim theoretical
originality: the comparison models, the oracle construction, the near-duplicate detection, and
commit-reveal are all standard, and the paper cites them as such.

## 15. Open problems and honest limitations

This section is deliberately the most detailed in the paper, because a system about honest attribution
that hid its own weaknesses would be refuting itself. The problems below are the precise frontier.

The **exogeneity terminus** is the deepest, and it is the one that forces the accounting-versus-value
distinction of Section 1.2. Worth is reception by minds outside the system, and no closed structure can
manufacture a judgment defined by something outside it. The concrete form is measured, not asserted. In
the reference simulator, consider K genuinely distinct participants who build novel-looking content on
each other's worthless roots: each member's deterministic value climbs as the ring layers work, each
member looks like a legitimate node with real provenance and real novelty, and a graph-internal test
cannot tell this ring apart from an equally-connected group of honest collaborators. The measured
separation on graph structure alone is 0%, and a wash member can end up scoring above an honest solo
contributor. This is why L0 is accounting, not value, and why every claim of un-gameability in Noesis is
scoped to specific demonstrated vectors (Sybil duplication, padding) rather than asserted in general.
The only routes out are external: an anchor on real-world use, or the transfer-graph direction of
Section 11, both of which import an outside signal, and both of which carry their own unsolved
Sybil-resistance problem.

The **consensus-versus-value tension** is structural and permanent. Fast final agreement and slow
revisable worth genuinely pull in opposite directions; the three-layer split bounds the conflict by
finalizing only the deterministic vested slice, but it does not abolish it, and the full honest floor of
Section 12, including the capital-floor conjunction and the Bitcoin-51%-class residual, stands.

**The finality-safety property is defined, not proved.** The defensible form (no non-deterministic or
learned output reaches finality; only a deterministic, vested, dispute-gated integer does) is checkable
as a bounded audit of the finality input surface, but it has not been discharged, and it is the gate
before any L1 construction. Until then, the finality-safety story is an intention with strong supporting
structure, not a theorem, and the green test suite does not establish it.

The **soft layer is under-specified**: the claim object, the sharding, the aggregation rule, and the
vesting of soft judgments are open, and this is the layer the whole move from accounting to value depends
on. The **learned value function on real labels** is the hardest mile, blocked by missing data rather
than missing effort: a value model validated against genuine external reception, invariant to relabeling
and isomorphism, is the property that would most justify the system, and it is not yet demonstrated. And
the **depth-axis lineage-laundering gap** (Section 13) remains open: the flow seed can be pumped by
laundering self-built lineage into apparent external flow.

Stated together, these are a map, not an apology. The contribution of the paper is as much in making
these problems precise and well-posed as in the parts that are built, because a hard problem stated
exactly is closer to solved than an easy-sounding problem stated vaguely.

### 15.1 The Section 1.1 defeat condition, answered: why not just the internet's timestamps?

Section 1.1 named the condition under which this paper fails: if an off-chain registry, or simply the
internet's existing record, closes the same gap as well as an on-chain ledger does. The sharpest form
of that objection is that the internet already records who did what and when, with timestamps, so a
tamper-proof attribution ledger is not a missing layer. It is answered here directly, both because a
reviewer will raise it and because answering it honestly narrows the paper's own claim.

The objection is partly right, and the concession comes first. Raw provenance and timestamps are
largely exogenous facts the internet already carries, and hard timestamping is a solved, off-the-shelf
primitive: a hash can be anchored to Bitcoin through OpenTimestamps today, for free, with no new
ledger. Noesis therefore must not, and does not, claim to invent timestamping or the record of what
was posted when. Stated as "we provide the timestamp," the system would be redundant.

What the internet does not provide is three things the objection elides. First, its order is either
self-asserted (a git commit date is an editable field) or stamped by a trusted party (a platform's
received-time, an archive's crawl-time), so it is not trustless canonical order, and in particular it
does not deny a participant the ability to backdate after seeing others' work, which is exactly what
the commit-reveal ordering enforces. Second, its persistence is not guaranteed: repositories are
force-pushed and deleted, packages are unpublished, accounts vanish, and the one archival anchor most
people would cite is itself a single trusted party with partial coverage. Third, and most important,
the internet has no weighted attribution at all: it carries the raw dependency edges but computes and
finalizes no credit share over them. "Who posted what when" is not "how much of this outcome is owed
to that dependency."

This is the same shape as the Bitcoin precedent and should be argued as such. Before Bitcoin, banks
already kept timestamped ledgers of who paid whom; the data existed. Bitcoin's contribution was never
recording transactions but reaching trustless canonical agreement on their order without a trusted
party. "The internet already timestamps it" is the exact analogue of "banks already record
transactions": true, and beside the point, because the missing layer is agreement and
non-rewritability, not data. The honest disanalogy, carried from Section 2, is that Bitcoin's record
is of an endogenous quantity and so the chain is its own source of truth, whereas Noesis records
exogenous artifacts and can only canonically order and finalize the attribution of commitments to work
whose existence lives off-chain.

The residual is stated without softening, because granting the objection in full is what exposes it.
If raw timestamping is conceded as solved, the genuinely novel delta Noesis adds over "the internet
plus OpenTimestamps plus a crawler" narrows to two things: trustless canonical agreement on ordering,
which is a modest delta over existing anchoring, and a finalized weighted attribution as consensus
state, which is the real delta and is the part that is designed, not built. The sharp form of the
objection is therefore that the redundant layer is the built one and the novel layer is the unbuilt
one, and that is a fair statement of the project's current risk rather than a misreading of it.

Finally, the objection cannot reach the hard problem even when granted in full. A perfect,
tamper-proof, trustless internet record of who did what when would still answer only order, never
worth: a timestamp establishes that one thing preceded another and says nothing about whether either
was worth anything, or how value flows between them. That is the exogeneity terminus above, untouched
by any improvement to timestamping. The off-chain-registry defeater, taken at its strongest, dissolves
part of the accounting layer and leaves the value problem exactly where this paper already places it.

## 16. Conclusion

Noesis is a structural bet with a partial build under it, and the honest version of that bet is the one
worth making in public. Bitcoin agreed on order and, by making its money endogenous, escaped the gap
between the ledger and the world. The systems that followed tried to price value and each inherited that
gap: quadratic funding priced breadth of support but funded promises, retroactive funding priced
demonstrated impact but through a human-governed round, and Deep Funding reframed value as a weighted
graph judged by a market of models but left that judgment off-chain, round-based, and jury-anchored.

What is built is a tamper-proof attribution-accounting ledger: provenance, ordering, soulbound standing,
conservation, and a deterministic credit skeleton, running and tested, but, by the 0% wash-separation
result, unable on its own to separate coordinated wash from genuine worth. What is designed is the value
meta-consensus that would turn accounting into worth: a continuous, staked, revisable judgment over many
smart nodes. What is open is the hardest and most important part: an un-gameable learned value anchored
on real external use, the finality-safety property stated but not proved, and the exogeneity terminus
that no closed structure escapes.

So the precise answer to the sharpest question, in what sense is this a value chain rather than a
deterministic accounting layer plus an unbuilt value layer, is this: today it is the accounting layer,
honestly, plus a specific and buildable design for the value layer, plus a precise account of the one
limit that cannot be engineered away. That is the sense in which Noesis is the value chain Bitcoin is
mistaken for: not a finished pricing of worth, but the first tamper-proof place to record attribution
and a trust-minimized design for judging it, with the exogenous limit named rather than hidden. Stated as
exactly that, it is a bet worth building in the open.

### 16.1 What the project hopes for, and what it does not claim

One hope motivates this work, and it is stated here as a hope rather than smuggled in as a result,
because it is a claim about people, not about the protocol. The hope is that once contribution is
recorded honestly enough, participants will stop trying to resolve the last mile of credit, recognizing
that no system, this one included, can reach the true depth of who is responsible for what, and that
the benefits of cooperating under a good-enough record vastly outweigh the residual imprecision. The
economic intuition behind the hope is ordinary and robust: a system that is approximately fair and that
everyone participates in produces far more total value than a perfectly fair system that no one can
agree on or that participants defect from. Growing the surplus dominates slicing it exactly, and the
pursuit of exact slices has its own cost, the rent-seeking over credit that poisons the cooperation it
means to reward.

There is a deeper form of the same hope, and it is the one that matters most. The costly thing is not
the imprecision itself but the effort spent fighting over credit, a deadweight almost everyone has
accepted as the normal cost of shared work: the authorship disputes, the litigation, the cap-table
conflicts, the endless contests over who is responsible for what. None of that effort creates
anything; it is friction over slices of value that already exist. A naive reading would promise that a
record of contribution ends such fighting, but the opposite is at least as likely: make attribution pay
and the fight migrates onto the record itself, as gaming, disputes, and wash. The resolution is
counterintuitive. The value here is not that the system settles the fight by being precise, but that it
makes the futility of the fight legible. Its most honest property, that the true depth of who is
responsible for what is unreachable, is also its lever: a system that demonstrates there is no exact
answer, and that cooperation is the rational move regardless, teaches something a precise judge never
could. It is a mirror, not a judge. This is also why the anti-wash and anti-concentration machinery is
not fairness for its own sake; a gameable record would manufacture a fresh arena for exactly the
fighting the project hopes to reduce.

In this sense, and only this one, the system can be said to plan the economy, the way a landscape plans
where water flows: not by rule but by gradient. The mechanism does not oppose self-interest; it
redirects it. A design built to defeat greed is moralistic and brittle, working only if people are
good; a design that conscripts greed is robust because it works precisely by assuming people are
self-interested, arranging the payoffs so that the only profitable way to satisfy the drive is to
create something, since capture without creation is made to lose. The greedy participant and the
generous one are led to build the same thing. Two honest bounds keep this from tipping into utopia.
First, the target is a specific class, extractive greed, value capture without value creation, not
self-interest as such and not the good-faith disagreement over worth that is the exogeneity terminus
and that neither should nor will stop. Second, the drive is adaptive and will seek any unguarded seam,
so this is an arms race against the demonstrated vectors rather than a checkmate, and it holds only as
far as the floor holds, which today is incompletely. The defensible claim is therefore not that the
fighting stops, but that the deadweight fighting, the kind spent capturing rather than creating,
shrinks toward an irreducible residual as extraction stops paying.

The hope has a precondition that the mechanism sections exist to serve, and the precondition is not
optional. Loosening up is wisdom only above a threshold of fairness, because the gains-from-cooperation
argument assumes the remaining imprecision is unbiased noise rather than directional theft. Random
error averages out and is safe to ignore; systematic, gameable error, the wash and concentration
attacks this paper has been careful not to wave away, is not noise but extraction, and relaxing in its
presence is not magnanimity but surrender. The purpose of the deterministic floor, the
anti-concentration conjunction, and the novelty discipline is therefore not to achieve precise
attribution. It is to convert adversarial imprecision into merely random imprecision, so that the
looseness the project hopes for becomes safe to adopt rather than suicidal to adopt. Looseness without
that floor is exploitation; looseness on top of it is cooperation.

It follows that the relaxation cannot be asked for up front. People loosen their grip on exact credit
as an output of trusting that defection has been made unprofitable, not as an input granted on faith,
and a system that opened by requesting that trust would be indistinguishable from the extractive
systems that request it too. Trust of this kind is earned by visibly closing the obvious thefts first,
after which the relaxation tends to happen on its own.

The honest boundary, finally. The protocol can make a loose, cooperative equilibrium rational and safe;
it cannot make anyone choose it. That final step is cultural, and culture is exogenous to the chain in
the same way worth is, which is to say it lives in the reception of minds outside the system and cannot
be manufactured from within. So the strongest honest form of the aspiration is this: the aim is not to
compute what each contribution was truly worth, which is impossible, but to keep a credible-enough
record that people are freed to stop fighting over attribution and simply build, with a floor
underneath that makes that freedom safe to accept. Whether they take it is their decision, not the
protocol's, and this paper claims only to make the decision a reasonable one.

---

## Appendices

- **A. Glossary.** Cell model, commit-reveal, soulbound standing, temporal novelty, similarity floor,
  flow damping, Shapley and Myerson credit, Helmholtz-Hodge inconsistency decomposition, CRPC, the
  finality-safety property, vesting window, anti-concentration, accounting-versus-value.
- **B. The value-layer placement decision.** The full ADR (`ADR-value-layer-placement.md`), including
  the finality-safety proof obligation and the determinism-is-not-objectivity correction.
- **C. Consensus-versus-value and the transfer-graph note.** The long-form treatment of Section 12 and
  the learning-nodes research direction.
- **D. Reference implementation pointers.** Where each built component lives, and the seam parity and
  property tests that back the "built" claims, with the enumerated adversary vectors the suite covers
  and the two (wash-building, depth-axis laundering) it does not.

---

## References (seed - expand and verify every entry before publication)

- Nakamoto, S. (2008). *Bitcoin: A Peer-to-Peer Electronic Cash System.*
- Buterin, V. (2014). *Ethereum White Paper.*
- Buterin, V. (2014). *SchellingCoin: A Minimal-Trust Universal Data Feed.*
- Blum, M. (1981). *Coin Flipping by Telephone.*
- Shapley, L. (1953). *A Value for n-Person Games.*
- Myerson, R. (1977). *Graphs and Cooperation in Games.*
- Deng, X., Papadimitriou, C. (1994). *On the Complexity of Cooperative Solution Concepts.* (Shapley #P-hardness.)
- Broder, A. (1997). *On the Resemblance and Containment of Documents.* (Shingling / min-wise hashing.)
- Bradley, R., Terry, M. (1952); Luce, R. D. (1959); Thurstone, L. (1927) - pairwise comparison models.
- Jiang, X., Lim, L.-H., Yao, Y., Ye, Y. (2011). *Statistical Ranking and Combinatorial Hodge Theory.* (HodgeRank.)
- Buterin, Hitzig, Weyl (2018). *Liberal Radicalism: A Flexible Design For Philanthropic Matching Funds.*
- Pasquini, R. (verify) - constrained quadratic funding (confirm this is the intended source; otherwise cite BHW 2018 budget-constrained discussion + Gitcoin/clr.fund pairwise-bounded matching writeups).
- Ghorbani, A., Zou, J. (2019). *Data Shapley: Equitable Valuation of Data for Machine Learning.*
- Sivill, T., Flach, P. (2023). *Shapley Sets* / set-cover Shapley. arXiv:2307.01777.
- Optimism Collective. *Retroactive Public Goods Funding* (and the metrics-based evaluation of later rounds).
- Buterin, V. (2025). *Deep Funding.* (Pin to the 2025 Ethereum round.)
- Bittensor. *Yuma Consensus* (verify primary reference).
- SourceCred; Colony (reputation); Coordinape; EigenTrust (Kamvar, Schlosser, Garcia-Molina, 2003) - reputation / credit-graph prior art.
- Goguen, J., Meseguer, J. (1982). *Security Policies and Security Models.* (Non-interference; cited for the term-collision note.)
- Chainlink (Ellis, Juels, Nazarov, 2017); Augur (Peterson et al.) - decentralized oracle lineage.
- Proof-of-useful-work lineage (survey to be gathered: Primecoin, Ofelimos, proof-of-learning).
