# Something From Nothing: Oracle-Free Content Valuation as the Basis of Contribution Consensus

**Preprint — draft for critique, 2026-07-19.** Repository: `WGlynn/noesis` (public). Status discipline
used throughout: ✅ verified in the reference implementation · 🟡 designed, not implemented · 🔬 open
problem. Nothing below is claimed as a peer-reviewed theorem; "verified" means *checked in code and
tests*, and the central primitive is explicitly open. We do not claim an exhaustive survey.

## Abstract

A blockchain that rewards contribution must answer a question no monetary chain has to: *what is a
contribution worth?* Any open, permissionless system that attaches reward to contribution is exposed to
the **counterfeiter** — an agent who manufactures the appearance of value from worthless input (noise,
padding, self-dealing) and extracts real reward. This is the *something-from-nothing problem* for
mind-value, the analogue of counterfeiting for money. Bitcoin closed the monetary case by making a
block cost energy; it does not, and cannot, tell whether the *content* a block carries is worth
anything. We argue that closing the mind-value case requires a signal of **content value that is
oracle-free**: any external judge of value — a human panel, a model, a committee — is an authority, and
an authority is an airgap between protocol and reality that is capturable; an oracle does not *solve*
something-from-nothing, it *relocates* it to whoever controls the oracle. We formalize the problem,
argue the oracle-free requirement, and present an architecture in which an oracle-free content-value
function `v(S)` feeds soulbound standing, wrapped by three composable layers — peer-prediction to score
content, a self-assessed (Harberger-style) price to stake the claim, and a dispute market to adjudicate
the gap — under a load-bearing separation invariant: **payment must never buy standing**. We report
which parts are implemented and isolate the single open primitive — a learned, oracle-free `v(S)` — on
which the whole reduces. We position the work carefully against the useful-work, subjective-consensus,
and reputation-consensus lineages, several of which pre-empt weaker versions of our claims.

## 1. Introduction

Proof-of-Work gives a blockchain an objective, oracle-free fact: this block cost energy, so coins
cannot be minted from nothing [Nakamoto 2008]. That fact secures *money*. It says nothing about the
*content* a block carries. A system whose thesis is that **standing should reflect genuine
contribution** — that recognition is earned by minds, not bought by capital — cannot borrow Bitcoin's
answer, because the thing it must not allow to be forged is not coins but *value*.

Call this the **something-from-nothing problem** for contribution systems: in any open protocol that
converts contribution into reward or franchise, an adversary will try to convert *nothing* (worthless
content, cheaply produced at scale) into *something* (standing, tokens, voting weight). A Sybil farm
submitting varied gibberish is not a peripheral attack; it is the counterfeiter, and defeating it is
the whole security problem restated.

The obvious response is to *judge* the content — install an oracle that rules what is valuable. We
argue in §3 that this fails structurally: an oracle is an external authority, and in a system whose
purpose is to eliminate capturable authority over who is recognized, an oracle simply moves the
something-from-nothing hole to whoever controls it. The requirement is therefore stronger and stranger:
the signal of content value must be **oracle-free** — intrinsic, endogenous, reproducible by every
node, with no trusted judge. This is the dissolution move: do not *solve* the value-oracle problem;
make it *unnecessary*.

Our contributions: (1) we frame Sybil-resistance in contribution consensus as the something-from-
nothing problem and argue the oracle-free requirement (§2–§3); (2) we position this against the
adjacent literature honestly, noting where prior work pre-empts weaker claims (§4); (3) we present an
architecture — an oracle-free value seam, wrapped by peer-prediction, self-assessed pricing, and a
dispute market, under a payment-≠-standing invariant (§5); (4) we report what is implemented versus
open and reduce the residual security of the whole to a single open primitive (§6–§7).

## 2. The problem

**Setup.** A contribution `c` is a cell carrying content `data(c)`, authored by a soulbound identity
`id(c)`, admitted in a commit-reveal order. A *value function* `v` assigns each contribution a
non-negative integer; per-identity aggregate `V(id) = Σ_{c: id(c)=id} v(c)` is that identity's
**standing**, which weights finality. (In the reference chain, standing weights the contribution
dimension of a bonded-validator finality mix, under a per-dimension anti-concentration floor; §5.4.)

**Adversary.** A counterfeiter controls arbitrarily many fresh identities (Sybils) and can produce
arbitrarily much content at low unit cost, choosing content adversarially. The counterfeiter succeeds
if it obtains standing (or reward) disproportionate to the genuine value it contributed — in the limit,
positive standing for zero genuine value.

**Why novelty is not value.** A natural, oracle-free proxy is *novelty*: reward content for the new
coverage it adds over everything seen. Novelty is genuinely strategyproof against the *duplication*
family — a later copy, paraphrase, or padding adds no new coverage and earns nothing (✅ verified for
exact/near duplicates in the reference implementation, `node/src/lib.rs` `temporal_novelty` +
`temporal_novelty_with_similarity_floor`). But novelty rewards *first-appearance*, not *worth*, and the
two diverge exactly where the adversary lives: **high-entropy random content is maximally novel.** A
varied random payload overlaps nothing, so a novelty-only score rewards it maximally. Thus a
novelty-only franchise does not close something-from-nothing; it *inverts* it, paying most for content
that is by construction meaningless. (This is not hypothetical: it is the audited gap in the reference
testnet's shipped v0 franchise; `docs/SYBIL-SURFACE-deployed-franchise-2026-07-19.md`.)

The counterfeiter therefore reduces to a single question the protocol cannot yet answer: **is this
content worth anything?** Everything else — identity, ordering, energy-cost, economic penalties — is
machinery around that question, and is comparatively well-understood. The value signal is the keystone.

## 3. Why an oracle cannot be the answer

Let an *oracle* be any external process that pronounces content value: a human panel, a trained model
run by a designated party, a permissioned committee. Installing an oracle appears to answer §2's
question. It does not, for the same reason a central bank does not solve counterfeiting by decree: it
relocates the trust.

- **An oracle is an airgap.** The protocol's guarantees hold on-chain; the oracle's judgment is formed
  off-chain and imported. The binding between them is a trust assumption, and any trust assumption over
  a *valuable* judgment is a target. Whoever captures the oracle captures the ability to mint standing —
  i.e., they can make something from nothing at will. The counterfeiter is not defeated; it is promoted
  to *oracle operator*.
- **It contradicts the thesis.** A contribution chain exists to remove capturable authority over who is
  recognized. An oracle *is* that authority. A design that reintroduces it to defend recognition has
  conceded the point it was built to make.

Hence the requirement: the content-value signal must be **oracle-free** — computable deterministically
and reproducibly by every node from on-chain data, with no privileged judge, such that all honest
replicas agree bit-for-bit. This is a hard constraint (it forbids, e.g., a floating-point model run by
one party on the consensus path), and it is the constraint that makes the problem a *research* problem
rather than an engineering one. The rest of this paper is an architecture that satisfies the constraint
for its wrapper and isolates the open core.

## 4. Related work (honest positioning)

We survey the lineages closest to "measured value as the consensus object" and state differentiators
precisely; several pre-empt weaker versions of our claims. This section deliberately prefers prior art
that subsumes us over prior art that flatters us.

**Useful-PoW.** BRSV Proof-of-Useful-Work [eprint 2017/203; 2018/678 retraction], Ofelimos, and
Proof-of-Learning [Jia et al.; spoofing arXiv:2208.03567] tie security to puzzle hardness or honest
execution of an *externally supplied* task; usefulness, where present, is exogenous (an external
problem board / market) and often not even robustly verifiable. None make *measured value* the security
object. PoRep/Filecoin/Chia weight consensus by dedicated space; data value is priced by an external
market.

**Subjective-utility consensus.** Bittensor's Yuma [learnbittensor docs] is the nearest live system: it
*attempts* to score contribution, but aggregates **subjective, stake-weighted validator opinion**
(weight matrix `W_ij`, consensus `P_j = Σ S_i W_ij` with median/κ-clip), rewarding agreement with the
stake-weighted majority. Empirically rewards are stake-dominated (stake→reward r≈0.5–0.95 vs
performance→reward r≈0.1–0.3, arXiv:2507.02951). We differentiate on *endogenous measurement vs
subjective stake-weighted voting*, and *soulbound standing vs purchasable franchise* — and we do not
claim Yuma "makes no attempt to measure value" (it does; the coupling is weak, not zero).

**Decentralized inference.** Fortytwo [arXiv:2510.24801] makes endogenous *output quality* the
consensus object via 3N pseudo-random pairwise comparisons aggregated with a Bradley–Terry model — very
close to our elicitation layer. This pre-empts the *bare* claim "output-value-as-consensus." Our precise
mechanism differs on three axes: a cooperative-game *synergy* value over a provenance DAG rather than
peer-ranked quality; a learned value-*model* bound to a manipulation certificate; and measured value as
the *finality object itself*. We narrow, not drop, the claim. (Decentralized *training* systems —
Gensyn/Verde, Prime Intellect/TOPLOC, Nous/Psyche — secure honest *execution*, not value, and do not
pre-empt it.)

**Reputation / graph credit.** EF Deep Funding regresses a scalar per-edge credit weight over a
dependency graph to match jury pairwise preferences — an off-chain *funding-allocation oracle* that
splits a fixed pot; it is never a consensus/finality object and uses no coalition value. SourceCred
(non-transferable Cred vs transferable Grain) and DeSoc soulbound tokens establish the
non-transferable-reputation split but make it *social/DAO* weight, not consensus weight. DPoR
[arXiv:1912.04065] proposes a Hodge-decomposition filter that confiscates stake on flows in the
harmonic/curl subspaces — a real partial pre-emption of a residual-certificate defense — but as a
proposal without algorithm, over raw token flows rather than a learned value scalar.

**Net position.** To our knowledge, no surveyed system makes an *endogenously measured, oracle-free
contribution value* the finality weight. That is the defensible core; the wrapper mechanisms below
(peer-prediction, self-assessed pricing, dispute adjudication) are adapted from known families and are
not claimed novel individually.

## 5. Architecture

### 5.1 The value seam (oracle-free by construction) ✅ interface built

Consensus consumes exactly one thing from the value layer: a deterministic integer value per finalized
cell. We isolate this behind a single interface, `ValueOracle::cell_values(cells, θ) -> Vec<u64>`
(`node/src/lib.rs`; tested `node/tests/value_oracle_seam.rs`), whose contract enforces the oracle-free
requirement of §3 as a *type-level obligation*: any implementation must be pure and deterministic
(bit-identical on every replica; no floats on the consensus path, no wall-clock, no map-order
dependence), shape-preserving (one value per cell, in commit order), and attributed to the soulbound
identity. Dispatch is static (no `dyn`), for an eventual RISC-V/CKB-VM port. Swapping the value
function is a governance-gated version bump, never a runtime plugin: the whole network agrees on one
canonical `v(S)`.

### 5.2 The v0 floor and its ceiling ✅ built / its limit stated

The shipped implementation, `NoveltyOracleV0`, is temporal novelty with a deterministic Q16.16
near-duplicate similarity floor. It is honestly a *novelty heuristic*, not a value model: it closes the
duplication family (§2) but not the novel-worthlessness family. It is the floor a cold chain can run
today; it is not the moat.

### 5.3 The wrapper: content-signal ⊕ claim-price ⊕ adjudication 🟡 designed

The open primitive is a *content-value* signal; around it we compose two mechanisms that make honest
self-report the profitable strategy. These are **composable and complementary**, not alternatives:

- **Peer-prediction scores the content.** Peer-elicitation games (PEG, SD-peer-prediction) elicit
  truthful peer signals of what content is worth, with truth-telling as an equilibrium — an oracle-free
  *content* signal (the peers are the network, not a privileged judge). This is the plausible substrate
  of `v(S)` at the seam. 🔬 two open theorems (graph-generalization, inner-equilibrium uniqueness).
- **A self-assessed (Harberger) price stakes the claim.** In a Harberger/COST scheme an owner
  self-declares a value `V`, pays a carrying cost proportional to `V`, and anyone may buy at `V`,
  forcing honest valuation from both sides. Standing is *soulbound*, so the "buy at `V`" leg is void;
  we replace it with **challenge at `V`** — `V` sets both the carrying cost (state-rent / vesting bond,
  ✅ present) and the *slash-at-risk*. Declaring junk as high-value incurs high rent and high slash on a
  successful challenge; declaring it low earns little; declaring true value honestly is proportional and
  survives challenge. Honesty is the profitable report.
- **A dispute market adjudicates the gap.** The reference chain already implements dispute-window
  challenge-and-slash: any vested-standing holder posts `Challenge(X, bond B)` while X's value is
  unvested, resolved by a PoM-weighted verdict reusing the consensus machinery, with an escalation
  court and juror-accountability slashing (✅ `docs/DISPUTE-SLASHING.md`, `node/src/` `dispute`). The
  adjudicated quantity is precisely the gap between the self-declared `V` and the peer-scored content
  value; that gap is the slashable overclaim.

The composition is closed: peer-prediction answers *what is it worth* (content), Harberger answers
*what did you claim and stake* (claim), the dispute market is *where they meet*. Neither closes the
honest-self-report obligation alone — Harberger without a truth signal has no principled definition of
"wrong"; peer-prediction without stake has no teeth.

### 5.4 The load-bearing invariant: payment ≠ standing

The augmentation is safe only under one invariant: **paying the Harberger carrying cost must never buy
standing.** Standing is earned by a contribution *surviving* challenge, never by the payment. Otherwise
capital purchases franchise weight and the system's anti-plutocracy property collapses — soulbound
contribution-standing and transferable capital must remain orthogonal (the reference chain enforces a
per-dimension anti-concentration floor so neither capital nor contribution finalizes alone,
`node/src/runtime.rs` `MIN_DIM_BPS`). The self-assessed price makes *dishonest reporting costly*; it
does not convert capital into recognition. That separation is the entire content of the word
"augmented."

## 6. What is built, what is open

| Component | Role | Status |
|---|---|---|
| Soulbound identity + one-time-leaf signature | who, and once | ✅ built (`node/src/rpc.rs`) |
| Commit-reveal ordering | strategyproof value order | ✅ built |
| PoW / issuance | objective ordering, Sybil-cost, money (JUL) | ✅ built |
| Value seam (`ValueOracle`) | oracle-free obligation, swap point | ✅ built (interface + tests) |
| v0 novelty + similarity floor | duplication defense (the deployed testnet floor) | ✅ built + **deployed** |
| v5–v8 value factors + structural defense (semantic/entropy floor, Hodge-residual slash, v6 identity pricing) | multi-factor value + anti-relabel defenses | ✅ built + tested (behind the seam; **not** the deployed testnet franchise) |
| Dispute market (challenge/verdict/slash, escalation court) | adjudicate the claim gap | ✅ built |
| State-rent / vesting | carrying cost substrate for a self-assessed price | ✅ present |
| Harberger self-assessed claim price | stake the self-report | 🟡 designed |
| Peer-prediction content signal (PEG/SD) | oracle-free content value | 🟡 designed · 🔬 2 open theorems |
| **Learned, oracle-free `v(S)`** | **the keystone** | **🔬 open, data-gated** |

The security of the whole against the counterfeiter reduces to the last row. Every other component is
built or designed; the residual open problem is a single primitive. Note the built/deployed distinction:
the multi-factor value functions and the structural anti-relabel defenses are implemented and tested,
but the *deployed testnet* intake franchise runs only the v0 floor — so a public permissionless launch
needs an interim economic/admission brake, whose adversarial failure envelope is measured (driving the
real scorer) in `v0-sybil-failure-envelope-2026-07-19.md`: with a per-identity cap, captured standing is
≈ F/(N+F), so a per-identity cap alone loses to costless keygen and an allowlist / proof-of-personhood
bounding identity count is the load-bearing bootstrap brake.

## 7. Open problems

1. **The keystone: a learned, oracle-free `v(S)`.** A value model trained on real contribution outcomes
   that (i) satisfies the seam's determinism contract, (ii) is not gameable by the vectors v0 leaves
   open (novel-worthlessness, structured-but-valueless, self-report rings), and (iii) can be run or
   verified oracle-free (e.g. via the peer-prediction substrate or succinct verification). This is the
   moat and it is open.
2. **Anti-plutocratic pricing.** The `V →` standing-weight relation must not let a self-priced claim
   scale franchise weight, or §5.4 is violated. Formalizing the safe class of pricing maps is open.
3. **Peer-prediction on a provenance graph.** Graph-generalization and inner-equilibrium uniqueness for
   PEG/SD over the contribution DAG (open theorems inherited from the elicitation literature).
4. **Manipulation certificate vs DPoR.** Any residual/Hodge-style certificate must be differentiated
   from DPoR [arXiv:1912.04065]; the honest claim is the *pairing* of a learned value model with such a
   certificate, not the certificate alone.
5. **The isomorphism-invariance gate.** Un-gameability is claimed only for demonstrated vectors; a
   general invariance argument (value stable under content-preserving transformations, unstable under
   value-destroying ones) is open.

## 8. Conclusion

The security question for a contribution chain is not "did this cost energy" but "is this worth
anything," and the counterfeiter attacks exactly there — making standing from noise, something from
nothing. An oracle cannot answer it without becoming the new thing to capture; the answer must be
oracle-free. We have framed the problem, argued the constraint, positioned it honestly against the
useful-work, subjective-consensus, and reputation lineages, and given an architecture that satisfies the
oracle-free constraint for its wrapper and reduces the residual to one open primitive — a learned,
oracle-free content-value signal, stake-wrapped by a self-assessed price and adjudicated by a dispute
market, under the invariant that payment never buys standing. The keystone is the moat: close the
content-value signal and the claim that standing reflects genuine contribution becomes true by
construction rather than by trust.

## References (verified in the novelty audit; not exhaustive)

- Nakamoto, *Bitcoin: A Peer-to-Peer Electronic Cash System*, 2008.
- Ball, Rosen, Sabin, Vasudevan, *Proof of Useful Work*, eprint.iacr.org/2017/203; follow-up 2018/678.
- Jia et al., *Proof-of-Learning*; spoofing: arXiv:2208.03567; incentive-secure PoL: arXiv:2404.09005.
- Bittensor *Yuma Consensus*, docs.learnbittensor.org; empirical: arXiv:2507.02951.
- *Fortytwo*, arXiv:2510.24801.
- EF *Deep Funding*, deepfunding.org; Myerson, *Graphs and cooperation in games*, Math. OR 2(3), 1977.
- Weyl, Ohlhaver, Buterin, *Decentralized Society (DeSoc)*, SSRN 4105763; SourceCred protocol docs.
- *Delegated Proof of Reputation (DPoR)*, arXiv:1912.04065; Jiang, Lim, Yao, Ye, *Statistical ranking
  and combinatorial Hodge theory*, arXiv:0811.1067.
- Data-Shapley: Ghorbani, Zou, arXiv:1904.02868.

*Companion internal docs: `docs/THE-KEYSTONE-content-value-signal.md`,
`docs/SYBIL-SURFACE-deployed-franchise-2026-07-19.md`, `docs/DESIGN-value-oracle-seam.md`,
`docs/DISPUTE-SLASHING.md`, `docs/research/RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md`.*
