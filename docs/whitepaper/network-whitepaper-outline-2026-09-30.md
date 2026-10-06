# Noesis Network Whitepaper — Layout (v0, 2026-09-30)

> **Scope.** This is the NETWORK / SYSTEM whitepaper. It is distinct from the existing
> `noesis-whitepaper.tex` ("Proof of Mind"), which is MECHANISM-focused (elicitation, Myerson
> synergy, temporal novelty, HodgeRank, the value function internals). This paper situates Noesis in
> the lineage of decentralized coordination and public-goods funding, and presents the whole network:
> a tamper-proof attribution ledger with a meta-consensus on value, powered by smart nodes. It
> POINTS AT the mechanism paper for the value-function math rather than re-deriving it.
>
> **Status discipline (carry into the prose, never round up):** built / designed / open. Noesis has
> no live network (deploy-axis at zero). The reference implementation is real and tested; the
> economics and the value meta-consensus are largely designed-or-open. The paper must say so.
>
> **Working title:** *Noesis: A Tamper-Proof Attribution Ledger with a Meta-Consensus on Value.*

---

## Part I — The problem and the lineage (context, ending in the logical conclusion)

The arc: each system below decentralized one layer of coordination and left the next layer open.
Noesis is the structural completion of the sequence, not a departure from it.

**1. Introduction: the missing ledger.**
We can decentralize agreement on ORDER (Bitcoin) and on PROGRAM (Ethereum), and we have bolted
funding mechanisms on top (Gitcoin, RetroPGF, DeepFunding). We cannot yet decentralize agreement on
VALUE / attribution with a tamper-proof record. Thesis in one paragraph + the honest floor: pricing
contribution is an open, genuinely hard problem (see Part III); Noesis is a structural bet plus a
partial build, not a solved system.

**2. What a blockchain is (consensus on order).**
Bitcoin [Nakamoto 2008]: the double-spend problem solved by consensus on transaction ORDER and
ownership. What it deliberately does NOT do: say anything about whether anything is worth anything.
Money is endogenous to the ledger, which is exactly why Bitcoin dodged the airgap. Set up the
contrast: value is not endogenous the same way.

**3. Programmable coordination (Ethereum and DAOs).**
Ethereum [Buterin 2014]: arbitrary programs over the ledger. DAOs: on-chain organizations, programmable
collective action and treasuries. The gap they leave: governance is token-weighted, i.e.
capital-weighted (plutocratic), and there is no native measure of CONTRIBUTION. One-token-one-vote is
not one-mind-one-vote.

**4. Democratic funding (Gitcoin / Quadratic Funding).**
Liberal Radicalism / QF [Buterin, Hitzig, Weyl 2018]. The formula (matching proportional to the
square of the sum of square-roots of contributions) weights breadth of support over size of donation;
single Nash equilibrium maximizes utilitarian welfare under the standard model. Gitcoin deploys it;
the Capital-Constrained variant (CQF) [Pasquini 2020, 2022] handles finite matching pools. Failure
modes to name honestly: collusion / Sybil rings, matching-pool exhaustion, and that it funds
PROMISES prospectively.

**5. Impact over promises (Optimism RetroPGF).**
Retroactive Public Goods Funding: reward demonstrated impact ex-post, not promised impact ex-ante
("impact = profit"). Judged by badgeholder juries in rounds. Failure modes to name: jury capture /
bribery, transparency, scalability, and that impact is still assessed by a small human panel (a
capturable oracle).

**6. Value as a graph (DeepFunding).**
Deep Funding [Buterin 2025]: the closest prior art, and the pivot of the whole paper. Two ideas:
VALUE AS A GRAPH ("how much credit for outcome Y belongs to dependency X", not "how much did X
contribute to humanity") and DISTILLED HUMAN JUDGMENT (an open market of AI models proposes edge
weights, a human jury spot-checks). Real dependency graph (~40k edges, Ethereum repos). What it gets
right: the graph reframing and AI-judgment-at-scale. What it leaves open (the seam Noesis fills):
it is OFF-CHAIN, one-shot per round, has no tamper-proof ledger or consensus, and its ground truth is
a human jury (a capturable oracle), with no structural answer to wash or to the consensus-vs-value
tension.

**7. The logical conclusion: Noesis.**
State the thesis as completion, not rupture. Bitcoin gave consensus on order; DeepFunding gave
value-as-a-graph judged by AI. Noesis puts the attribution itself ON a tamper-proof ledger and makes
the AI value judgment a CONTINUOUS, trust-minimized meta-consensus (commit-reveal + stake/slash over
many smart-node oracles) rather than a one-shot competition judged by a capturable jury, while being
explicit about the exogenous limit no such system escapes. This is the one-paragraph "where Noesis
differs" that Part III substantiates.
  - *7.1 Non-rivalrous by construction (convergence, not competition):* the systems Noesis descends
    from (QF, RetroPGF, Deep Funding) are upstream contributions recorded ON Noesis, not rivals; same
    recursion as co-authorship provenance. Belief: networks trend toward merging onto a shared
    substrate. Honest floor: a design stance and prediction, not a shipped cross-network integration.

---

## Part II — The Noesis network (the system)

**8. Architecture overview (the three-layer split).**
From `ADR-value-layer-placement.md`: L0 hard deterministic skeleton on-chain; L1 soft, revisable value
meta-consensus; one deterministic integer seam between them. Why this split is forced (consensus-vs-
value tension + the determinism wall converge). This is the spine of the network.

**9. The tamper-proof attribution ledger (L0) — BUILT.**
Provenance, commit-reveal ordering, soulbound PoM standing (`type_script.args`), Cell/UTXO
conservation, and the DETERMINISTIC value-flow skeleton: temporal novelty, flow propagation with
damping, Shapley/Myerson credit, Helmholtz-Hodge cycle detection. Point to `lib.rs` / `runtime.rs`.
This is the part that actually runs and is tested.

**10. The meta-consensus on value (L1) — DESIGNED / OPEN.**
CRPC (commit-reveal pairwise comparison) over the natural provenance shards as a third verification
axis (soft/graded), separate from and never overriding the hard rulebook. Revisable, never-final,
mean-reverting as realized use arrives. Honest: this is a sketch, not a spec.

**11. Smart nodes as value oracles ("powered by smart nodes").**
AI / small models run OFF-chain and emit pairwise judgments; the chain never runs the model, it
agrees on WHAT each model said and disciplines the claim with stake and slashing. Non-determinism is
quarantined at the input (committed fact), aggregation stays deterministic. zkML as the trustless
endgame. The learning-minds-as-nodes direction (turning a citation graph into a transfer graph of
measured behavioral change) goes here as a research direction, honestly gated on the learning working.
DISCRETION: keep external collaborators and their libraries unnamed in the public paper.

**12. Consensus on value (order finalizes fast, worth stays revisable).**
The consensus-vs-value tension and its resolution: finalize objective order and provenance fast and
irreversibly; keep value a slow overlay that never finalizes. The sharp point to state honestly,
because a reviewer will raise it: PoM-weighted finality means consensus DOES depend on a value-derived
quantity. Containment: only the deterministic, vested, dispute-gated proxy weights finality (never the
learned judgment); anti-concentration means no dimension finalizes alone. Honest: non-interference is
the load-bearing safety property and is not yet proved.

**13. The value function (pointer, not re-derivation).**
One-page summary of the endogenous value: elicitation from pairwise judgment, the additivity trap,
Myerson synergy, temporal-novelty strategyproofness, HodgeRank certification, the learned evaluator
bounded by construction (corruption harmless because it can only lower a seed). Cite the Proof of Mind
whitepaper for the full treatment.

---

## Part III — Theory, novelty, honesty

**14. Related work and where Noesis differs (the big references section).**
Organized by lineage, each with the honest differentiator:

- *Cooperative game theory / attribution math:* Shapley value [Shapley 1953], Myerson value on graphs
  [Myerson 1977], Banzhaf index; Data Shapley [Ghorbani & Zou 2019] and the ML data-valuation line.
  Differ: Noesis uses these as the ON-LEDGER deterministic credit skeleton with anti-Sybil damping,
  not as an offline analysis.
- *Public-goods funding mechanisms:* Quadratic Funding / Liberal Radicalism [BHW 2018], CQF [Pasquini
  2020/2022], RetroPGF [Optimism], Deep Funding [Buterin 2025]. Differ: these are allocation
  mechanisms bolted on top of a chain or run off-chain; Noesis makes attribution the consensus object
  itself, continuous and tamper-proof, with a trust-minimized value meta-consensus instead of a
  capturable jury or a one-shot competition.
- *Consensus / useful-work:* Nakamoto [2008], PoS, proof-of-useful-work lineage. Differ (from the
  existing novelty audit, `RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md`): across the surveyed corpus the
  consensus object is always something OTHER than an attribution; in Noesis, attribution IS the
  consensus object.
- *AI-in-governance:* Buterin's "AI is the engine, humans the steering wheel." Differ: Noesis
  disciplines the AI's CLAIMS via the chain rather than trusting a jury to check them.

**15. Open problems and honest limitations (the well-defined-hard section).**
- The exogeneity terminus: worth is exogenous to any closed structure; wash-building is graph-internal-
  invisible (0% separation, `wash_sim.rs`). You cannot extract external-world judgment from a closed
  system.
- Consensus vs realized value: structurally at odds; the design bounds it, does not dissolve it.
- Non-interference: stated, not proved (the gate before building L1).
- CRPC: unspecified. Learned v(S): data-gated.
This section is a feature, not an apology: the problems are hard but now well defined.

**16. Conclusion.**
Noesis as the value chain Bitcoin is mistaken for. Honest close: the shape is decided, the open
problems are precise, the ledger is built, the value meta-consensus is designed, and the hardest
mile (un-gameable learned value on real labels) is open and data-gated.

**Appendices.** Glossary; the value-layer ADR; the consensus-vs-value + transfer-graph note; reference
implementation pointers.

---

## References (seed — REAL, verified this session; expand before draft)

- Nakamoto, S. (2008). *Bitcoin: A Peer-to-Peer Electronic Cash System.*
- Buterin, V. (2014). *Ethereum White Paper.*
- Shapley, L. (1953). *A Value for n-Person Games.*
- Myerson, R. (1977). *Graphs and Cooperation in Games.*
- Buterin, Hitzig, Weyl (2018). *Liberal Radicalism: A Flexible Design For Philanthropic Matching Funds.* https://gwern.net/doc/economics/mechanism-design/quadratic-voting/2018-buterin.pdf
- Pasquini, R. (2020). *Quadratic Funding and Matching Funds Requirements.* arXiv:2010.01193
- Pasquini, R. (2022). *Optimal Allocation of Limited Funds in Quadratic Funding.* arXiv:2207.14775
- Ghorbani, A., Zou, J. (2019). *Data Shapley: Equitable Valuation of Data for Machine Learning.* ICML / arXiv:1904.02868. https://proceedings.mlr.press/v97/ghorbani19c/ghorbani19c.pdf
- Optimism Collective. *Retroactive Public Goods Funding (RetroPGF).* + *A Social Choice Analysis of Optimism's Retroactive Project Funding*, arXiv:2508.16285.
- Buterin, V. (2025). *Deep Funding.* https://gitcoin.co/mechanisms/deep-funding ; deepfunding.org
- *Evaluating Voting Design Vulnerabilities for Retroactive Funding.* arXiv:2505.16068

### To gather before drafting (not yet surveyed — do not fake these)
- Banzhaf power index (primary), and axiomatic-uniqueness treatments of Shapley for credit assignment.
- Proof-of-useful-work / proof-of-contribution consensus survey (to ground the "attribution as
  consensus object" novelty claim precisely).
- Peer-prediction / Bayesian truth serum (Prelec) for the elicitation-honesty lineage behind CRPC.
- Token-curated registries and reputation-system literature (for L1 prior art).
- zkML references (for the trustless-oracle endgame in section 11).
- CKB / Cell model + RISC-V VM (substrate citations).
