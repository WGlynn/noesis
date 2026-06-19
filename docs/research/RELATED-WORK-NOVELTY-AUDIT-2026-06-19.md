# Related-Work & Novelty Audit — Proof of Mind vs the useful-PoW / contribution-consensus lineage

> Adversarial novelty audit, 2026-06-19. Deep-research harness: 5 angles, 23 sources fetched, 102
> claims extracted, 25 verified by 3-vote adversarial check (25/25 confirmed, 0 killed). Brief was
> "prefer finding prior art that subsumes our claims over flattering us." Feeds whitepaper §9 (Related
> Work) and the novelty claims. PRIVATE / stealth. Source of truth for what we may and may NOT claim.

## Net verdict

Across the entire surveyed corpus the consensus object is always something OTHER than an
endogenously-measured contribution-value: security comes from puzzle hardness (Primecoin, BRSV),
dedicated space (PoRep/Filecoin/Chia), honest execution (PoL, PoLe), or stake-weighted subjective
opinion (Bittensor Yuma). **No surveyed system makes the measured contribution-value the stake/finality
weight.** Claim (a) is the strong, defensible novelty.

## Per-claim verdicts

| Claim | Verdict | Closest prior art | Differentiator |
|---|---|---|---|
| **(a)** endogenous value = the consensus object | **NOVEL** (high) | Bittensor Yuma | Yuma aggregates subjective stake-weighted opinion; value never becomes the finality weight. Useful-PoW leaves value exogenous (external market / problem-provider). |
| **(b)** Myerson value over provenance DAG | NOVEL *in corpus*; **not cleared** vs its family | EF Deep Funding, Data-Shapley, Myerson itself | Deep Funding does pairwise-distilled judgment over a dependency graph — the closest cousin. NOT surveyed → needs a 2nd pass before asserting novel. |
| **(c)** temporal-novelty commit-reveal | NOVEL *in corpus* | none surveyed | No PoUW system uses commit-reveal first-to-cover novelty. Not cleared vs MEV/commit-reveal literature. |
| **(d)** soulbound standing vs transferable capacity | **NOVEL / PARTIAL** | Bittensor (TAO) | Direct inverse: TAO is purchasable and *buys the franchise*; we make standing soulbound ("buy storage, not consensus"). SBT/proof-of-personhood lit not surveyed. |
| **(e)** bounded deny-only v(S) + HodgeRank residual certificate | NOVEL *in corpus* (med) | none | Yuma's kappa-clip is median/quantile robustness, not a learned reward model; no residual-certificate construct. HodgeRank/robust-reward lit not surveyed. |
| **(f)** ToM → ETM → PoM framing | **UNASSESSED** | — | Narrative/positioning claim. Defend on the strength of (a)–(e), not as an independently-cleared novelty. |

## The most dangerous competitor: Bittensor (Yuma Consensus) — state it precisely

Yuma is a self-described "subjective utility consensus mechanism." Validators submit a weight matrix
`W_ij` ranking miner work; consensus is the stake-weighted aggregate `P_j = Σ S_i · W_ij` with a
median/kappa-clip; validators are rewarded for **agreeing with the stake-weighted majority**, not for
discovering truth (their docs concede the circularity). Empirically (arXiv:2507.02951, all 64 subnets,
~6.6M events): stake→reward r ≈ 0.50–0.95, miner performance→reward r ≈ 0.10–0.30 — rewards are
overwhelmingly stake-driven.

**Required framing (calibration risk):** say *"Yuma attempts to score contribution but aggregates
subjective stake-weighted opinion and lets exogenous stake dominate reward."* Do NOT say "Yuma makes no
attempt to measure value" — a proponent will rebut, and the empirical coupling is weak-not-zero
(miner r can reach ~0.8). Our differentiator is endogenous *measurement* vs subjective *stake-weighted
voting*, and soulbound standing vs purchasable franchise.

## Useful-PoW lineage is WEAKER prior art than it looks (usable in Related Work)

- **BRSV "Proof of Useful Work" (2017)** retracted its usefulness claims in the 2018 follow-up (honest
  prover needed poly-log task instances per proof, breaking efficiency). Usefulness was exogenous (an
  external "Problem Board"); security was worst-case hardness, not value.
- **Proof-of-Learning (Jia et al.)** is provably not robust — reproducible spoofing forges valid proofs
  without training; provable robustness reduces to open problems in DL optimization. Incentive-secure
  variants (Zhao et al.) secure only honest execution of an *externally supplied* task, not value.
- **Ofelimos** treats usefulness as an ex-post diagnostic; consensus security is proven separately via
  a moderate-hardness threshold.
- **PoRep / Filecoin / Chia**: consensus weight is dedicated space/storage cost; data value is priced
  by an external market.

These can be cited as a lineage that often fails even to deliver verifiable *useful work*, let alone
endogenous value *measurement*.

## SCOPE GAPS — a second research pass is required before the whitepaper asserts (b)/(d)/(e) novel

The corpus has ZERO verified sources on the families most likely to pre-empt us:
1. **EF Deep Funding** (pairwise distilled judgment over a dependency graph) — the most likely
   collision with (b). Both operate over a contribution graph. MUST check before claiming (b) novel.
2. **Data-Shapley (Ghorbani–Zou)** + Myerson's graph-restricted Shapley primitive itself.
3. **Soulbound tokens (Weyl–Ohlhaver), proof-of-personhood, non-transferable-reputation consensus** —
   for (d).
4. **HodgeRank (Jiang–Lim–Yao–Ye) + robust reward-model literature** — for (e), specifically whether
   anyone already pairs a learned value model with a residual manipulation certificate.
5. **Fast-moving decentralized-training / proof-of-inference** (Gensyn, Prime Intellect, Nous, Ritual)
   — named in the brief, zero verified findings; the most likely place (a) could be pre-empted.

Open question flagged as highest-risk: *does Deep Funding's pairwise-distilled judgment over a
dependency graph subsume claim (b)?*

## SECOND PASS (RI-002, 2026-06-19) — the flagged claims cleared adversarially

Deep-research pass 2 (105 agents, 23 sources, 24/25 verified, 1 killed) targeted exactly the families pass 1 left uncleared.

- **(b) NOVEL — confirmed against the most dangerous collision.** Deep Funding (Vitalik/EF) computes a distilled **scalar per-edge credit weight** (fraction of a dependent's credit assigned to a dependency), regressed by a model (Bradley-Terry/GBM/LLM) to match jury pairwise preferences — **no characteristic function, no coalition marginal, no connected-coalition restriction, no Shapley/Myerson**. It is an **off-chain funding-allocation oracle** (splits a fixed pot, e.g. $170k), **never a consensus/finality/stake object**. Differentiated on TWO independent axes (edge-weight-regression vs graph-restricted-Shapley synergy; funding-oracle vs consensus-object) so a single "isn't this Deep Funding?" objection can't collapse it. Data-Shapley = flat (non-graph) sibling, offline metric. Myerson (1977) = known primitive; novelty = its application AS the consensus object. All on-chain Shapley found (MEV-redistribution arXiv 2502.15420; PoS-reward) = post-hoc reward redistribution layered on existing consensus.
- **(d) NOVEL as consensus-franchise; PARTIAL on the split PATTERN.** SourceCred (Cred non-transferable PageRank-earned / Grain transferable) + DeSoc soulbound tokens establish the non-transferable-reputation-vs-transferable-token split — but **neither makes non-transferable standing the CONSENSUS WEIGHT** with a separate transferable state-capacity token. SourceCred's "consensus" is social town halls; DeSoc is correlation-discounted DAO voting. Cite both as closest; differentiate on consensus-weight + state-capacity.
- **(e) NOT adjudicated · (a)-vs-decentralized-training NOT adjudicated.** The pass-2 synthesis concentrated on (b)/(d); no verified findings on HodgeRank-residual-as-manipulation-certificate, reward-overoptimization (Gao), or Gensyn/Prime Intellect/Nous(Psyche/DisTrO)/Ritual/proof-of-inference making OUTPUT VALUE the consensus weight. **A THIRD pass is required** before the WP asserts (e) novel or claims (a) cleared against decentralized-training. The WP §9 hedges both honestly ("to our knowledge", "we do not claim an exhaustive survey").
- **Caveat:** Deep Funding is an active 2024-26 program; a future round promoting its edge-weight oracle to an on-chain issuance/consensus role would narrow the (b) differentiator from two axes to one — re-check periodically.
- **Pass-2 sources:** Vitalik X (1867886974058520820) · github deepfunding/dependency-graph · deepfunding.org · ethereum-magicians #24200 · Data-Shapley arXiv 1904.02868 · Myerson 1977 (Math.OR 2(3):225) · SourceCred FAQ + protocol.ai intro · DeSoc SSRN 4105763 · MEV-Shapley arXiv 2502.15420.

## Key sources (verified, primary unless noted)
- BRSV Proof of Useful Work: eprint.iacr.org/2017/203 + 2018 follow-up eprint.iacr.org/2018/678, /2018/559
- Ofelimos: link.springer.com/chapter/10.1007/978-3-031-15979-4_12 (+ IOHK blog)
- Proof-of-Learning spoofing: arxiv.org/abs/2208.03567 ; incentive-secure PoL: arxiv.org/abs/2404.09005 ; PoLe: arxiv.org/pdf/2007.15145
- Bittensor Yuma: docs.learnbittensor.org/learn/yuma-consensus ; github opentensor/subtensor docs/consensus.md ; empirical: arxiv.org/html/2507.02951v1 (FLock.io authors — a competitor, but corroborated by Bittensor's own docs)
- Deep Funding: deepfunding.org ; HodgeRank: jiang-lim-yao-ye (Math.Prog.B 2010)
