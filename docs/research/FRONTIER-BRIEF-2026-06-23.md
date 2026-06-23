# Noesis Frontier Brief — Contribution-Value Consensus Without Ground Truth (2026-06-23)

> PRIVATE / stealth. Synthesis of a 4-agent parallel frontier scan (decentralized-LLM consensus ·
> agentic dispute resolution · agents+blockchains · LLMs+blockchains), cross-referenced against the
> Noesis/PoM design. "Deployed" vs "designed" flagged on BOTH sides. Source: workflow
> `noesis-frontier-research-fanout` (5 agents, ~339k tok). Feeds whitepaper §9 + `MOAT-STACK.md` +
> the validate/novel question. Companion to `RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md` (this is the
> demanded second pass on the Gensyn/Prime-Intellect/Nous/Ritual/DeepFunding scope-gaps).

## 0. Verdict on validate-vs-novel (the question this run was built to answer)
**Validated direction + a narrow-but-real unoccupied solution square = the best-case position.**
- **VALIDATED:** the field independently rediscovered our levers — non-linear/saturating Sybil
  resistance, commit-reveal timestamp-priority, performative prediction, HodgeRank, peer-prediction,
  false-name-proofness. We don't defend these; we cite them as established and claim only the fusion.
- **STILL NOVEL (specific):** value/attribution with NO immediate ground-truth oracle AND no eventual
  realized-outcome anchor for *individual* attributions; topology-only collusion detection that does
  NOT need an honest stake majority; an adaptive/Goodhart-robust value measure — where the literature
  is *empty*. Closest formal neighbor ("It Takes Two") explicitly leaves Sybil open.
- **HONEST narrowing:** ahead on rigor, behind on everything deployed (Yuma/dTAO years ahead). The
  learned-`v(S)`-on-realized-outcomes *does* re-introduce an *aggregate* outcome anchor for retraining
  (not per-decision) — state precisely. The moat is conditional on `ε·β/γ<1` holding.

---

## 1. Landscape

### A. Decentralized LLM / proof-of-X consensus
- Replication is economically dead (execution dominates consensus 3–6 OoM) ⇒ everything reduces to
  spot-check+penalty, or crypto/TEE attestation. Three schools: cryptographic correctness (zkML, Gensyn
  Verde refereed-delegation + RepOps, Prime Intellect TOPLOC LSH); game-theoretic sampling (Hyperbolic
  PoSP — unique pure-strategy Nash, opML); hardware attestation (Phala TEE, Morpheus).
- Consensus-on-subjective-value (the hard, ground-truth-free regime) in production = only **Bittensor
  Yuma** (stake-weighted median + clip + EMA bonds; dTAO Feb 2025) and **Allora** (forecasted-peer-loss
  + realized-outcome anchor). Yuma holes: weight-copying beats honest validators, ~50% collusion
  ceiling, off-chain rings undefended (commit-reveal only stops on-chain copying).
- Sybil resistance = non-linearity, not counting. Truth-Inducing Sybil-Resistant Oracle + SD-Peer-
  Prediction (arXiv 2506.02259) independently rediscover sublinear/saturating scoring. Commit-reveal is
  the standard anti-copy fix; Bittensor finds benefit saturates ~5 tempos.
- Must-know: Bittensor/Opentensor (Jacob Steeves); "It Takes Two" Byzantine-robust peer-prediction
  (arXiv 2406.01794 — Sybil left open); TOPLOC (arXiv 2501.16007, ICML 2025).

### B. Agentic dispute resolution / truthful elicitation
- Peer prediction = the theoretical backbone for truth without ground truth. Strength ladder:
  Bayesian-Nash → dominant-truthful (Kong, JACM 2024, f-mutual-information) → stochastically-dominant
  (arXiv 2506.02259). Impossibility: no mechanism is simultaneously prior-free and permutation-proof
  (Kong-Schoenebeck).
- LLMs are now the predictor inside peer prediction: GPPM/GSPPM (EC '24); **PEG** (NeurIPS 2025) proves
  dominant-truthfulness + last-iterate convergence for AI agents via determinant-based mutual-info —
  structurally blocks agreement-on-falsehood. The closest formal template for HCE.
- Schelling-oracle stack (Kleros, Aragon Court, UMA DVM, Reality.eth) = deployed competitor substrate;
  theory wall = the p+ε bribery attack (George-Lesaege; Buterin-Miller).
- Aggregation beats deliberation (confidence-weighted voting 83.4% > debate ~76%, "persuasive error
  propagation"). LLMs ~89.6% on post-dispute resolution but ~50% forecasting disputes. Lehmann 2026
  survey: theory rich, empirics weak, too complex to deploy.

### C. Agents + blockchains (rails, identity, reputation)
- Converging stack: identity/comms (A2A, MCP, **ERC-8004** live Jan 2026) + payments (**x402** 100M+ tx,
  now Linux Foundation; AP2 mandate chains) + reputation (registries, TraceRank). ERC-8004's
  Reputation/Validation registries are deliberately empty shells — scoring left off-chain.
- **TraceRank** (Operator Labs, arXiv 2510.27554): reputation from value-weighted, recency-decayed
  payment DAGs — the nearest *deployed* analog to our topology mechanisms, but heuristic (flat propagation).
- Structural > prompt-level honesty, proven empirically (arXiv 2601.11369: agents collude for profit,
  prompt prohibitions fail). Virtual Agent Economies (DeepMind, arXiv 2509.10147) argues proactive MD.
- Provenance/attribution DAGs already live: Story Protocol Proof-of-Creativity (auto-royalty along
  derivation graph); Vana Proof-of-Contribution (DataDAOs, mainnet Dec 2024). Wall: Cheng-Friedman
  Sybil impossibility (axiom-satisfying reputation ⇒ Sybil-attackable).

### D. Substrate (zkML/opML/TEE, contribution measurement, Goodhart)
- Three verification regimes, none price value — they certify *what ran*: zkML (EZKL/Kang EuroSys'24 —
  expensive); **opML** (ORA, arXiv 2401.17555 — cheap, staked challengers); TEE (Phala GPU-TEE).
- Contribution measurement = cooperative-game characteristic functions (Data Shapley, Ghorbani-Zou
  2019) — but provably gameable via the score function (validation-set attack, TMLR 09/2024). Asymmetric
  Data Shapley (2024) drops symmetry for structure-awareness.
- Goodhart is an impossibility, not a bug: no non-trivial proxy reward is guaranteed unhackable. Winning
  move = structural: redefine the objective so genuine contribution and collusion-resistance *coincide*
  (Plural QF, RadicalxChange 2024). **DeepFunding** (Buterin/Gitcoin, ~$250k pilot) = closest public
  DAG-credit-allocation analog (AI allocators + sparse human jury).
- Primitives Noesis already invokes: Performative Prediction (Perdomo ICML 2020 — the adaptive-stability
  engine); HodgeRank harmonic-residual + 2025 collusion-robust extension; false-name-proof / cost-of-
  identity (Yokoo 2000; Mazorra–Della Penna 2023).

---

## 2. Learn / borrow
- **Strengthen the truthfulness target Nash → stochastic dominance** (SD-Peer-Prediction 2506.02259 +
  Kong f-mutual-info) as design constraints for `v(S)` payouts ⇒ "truth strictly out-pays collusion"
  becomes a theorem, matching coalition-proofness.
- **PEG (NeurIPS 2025, arXiv 2505.13636) = near-drop-in proof template** — determinant-based mutual-info
  blocks agreement-on-falsehood + last-iterate convergence for self-interested AI agents = the HCE setting.
- Commit-reveal window saturates ~5 tempos (don't over-lengthen).
- Non-linearity is the field-consensus Sybil lever — cite as external validation that geometric
  saturation + temporal-novelty are *the* lever, not ad hoc.
- Performative prediction is published prior art for the moat — ground `ε·β/γ<1` in Perdomo 2020 + the
  2024 performative-on-games extension; don't reinvent.
- HodgeRank collusion-robustness has a 2025 academic backbone — cite as detector feeding the slash; the
  fixed-point framing pairs with performative prediction for a unified theory section.
- TDA (influence functions, LoGra/EK-FAC) + watermarks make provenance-DAG edges content-verifiable
  (the "whose data caused this output" signal `v(S)` regresses against).
- Borrow Yuma's EMA-bond smoothing for validator-score stability; differentiate on the topological detector.

---

## 3. Compete / overlap (honest about who's ahead)
1. **Bittensor Yuma + dTAO — deployed, largest, incumbent.** Direct competitor for "reward agreement on
   subjective AI value." Years ahead in deployment/stake/ecosystem. But its security is exactly our
   target (weight-copying, ~50% ceiling, off-chain rings). Frame Noesis as "what Yuma cannot fix
   on-chain," not a replacement.
2. **TraceRank — deployed/published, nearest topological competitor.** Sybil-resistant reputation on a
   payment DAG, no curator. We out-class on the same axes (formalized saturation/novelty + Hodge
   ring-detection vs flat propagation) — on paper; they ship against real x402 flows.
3. **Vana PoC + Ocean — deployed tokenized data-contribution markets.** Heuristic per-DAO valuation +
   staking leaderboard (Goodhart surfaces). We're more rigorous in design; they have mainnet/users/TVL.
4. **DeepFunding — pilot, closest in spirit.** Consensus-on-contribution over a dependency DAG,
   structure-over-social + sparse human jury. Anchors on a human-jury "ground truth" we claim to
   eliminate. Watch closely — most aligned public effort, Buterin-backed.
5. **Schelling-oracle stack (Kleros/UMA/Aragon) — deployed adjudication.** We differentiate on p+ε and
   mutual-citation rings.

**Bottom line:** ahead on mechanism rigor + the no-eventual-ground-truth framing; behind on everything
deployed. Every named competitor has an eventual-ground-truth anchor (Allora, DeepFunding jury) or a
heuristic value function (Yuma, TraceRank, Vana). That's the real edge — a paper edge until something ships.

---

## 4. Fill-gap / differentiated (overclaims flagged)
- **Unoccupied white space: value/attribution with NO ground-truth oracle AND no eventual realized
  outcome to anchor individual attributions.** The field is either correctness-verification (a checkable
  answer exists) or eventual-ground-truth forecasting. PoM's `v(S)` over a provenance DAG is a different,
  unclaimed formalization. "It Takes Two" is the closest neighbor and leaves Sybil open. **Own and name it.**
  - ⚠️ Overclaim risk: the *learned* `v(S)` retrained on realized downstream outcomes re-introduces an
    *aggregate* outcome anchor. Be precise: PoM eliminates the *immediate per-decision* oracle and uses
    aggregate realized outcomes only to retrain `v(S)` (the performative loop), not to settle individual
    attributions. Don't claim "no outcomes anywhere."
- **HodgeRank harmonic-residual on topology alone** is genuinely novel vs every surveyed mechanism — it
  detects rings *without* a large honest stake majority (the field's central unsolved frontier:
  collusion approaching/exceeding 50%). The 2025 collusion-robust extension is prior art AND a sparring
  partner — cite, don't claim sole invention.
- **Generalizing peer prediction → cooperative-game value in a dependency graph** is unaddressed in the
  cited literature. Defensible generalization.
- **Escaping a proven impossibility, precisely:** Cheng-Friedman says axiom-satisfying reputation is
  Sybil-attackable. State which axiom PoM relaxes — symmetry/anonymity, via commit-reveal temporal-
  priority + PoW-anchored (JUL) identity making fresh identities structurally worthless. The false-name-
  proof lineage makes the JUL money-layer load-bearing for coalition-proofness, not incidental.
- **Adaptive/Goodhart-robust HCE is where the literature is empty** (Yuma SN28 meme-coin exploit within
  a month of dTAO; Data Shapley validation-set attack; RL "no proxy unhackable"). Our performative-
  prediction contraction is the moat *precisely here.* ⚠️ Holds only if `ε·β/γ<1` in deployment — prove/
  measure, don't assert.
- **Substrate:** CKB-fork Rust+RISC-V cell/UTXO-with-state-rent fits provenance-DAG state; PQ Lamport
  locks = forward defense. ⚠️ Architectural-fit argument, not proven advantage; adds adoption friction
  (no EVM ecosystem).

---

## 5. Highest-value crossings to act on (ranked)
1. **Prototype the PEG proof for HCE** (PEG arXiv 2505.13636 + SD-PP 2506.02259). Adapt determinant-based
   mutual-info + last-iterate convergence to `v(S)`-payouts ⇒ theorem that HCE is the focal, collusion-
   blocking equilibrium. Upgrades the core claim designed→proven-in-template. **Highest leverage.**
2. **Benchmark head-to-head vs TraceRank** (2510.27554). Build the mutual-endorsement-ring TraceRank's
   flat propagation misses and HodgeRank catches ⇒ a reproducible "PoM catches what TraceRank misses."
3. **State the Cheng-Friedman axiom relaxation explicitly.** *"PoM escapes the Cheng-Friedman Sybil
   impossibility by relaxing anonymity: commit-reveal timestamp-priority on a PoW-anchored (JUL) identity
   makes a fresh identity structurally worth zero, so false names cannot inherit standing."*
4. **Watch + engage DeepFunding** (closest public effort; credible validation/collaboration channel).
   Position PoM as the strategy-proof, jury-free successor. ⚠️ public artifacts only (NDA discipline).
5. **Measure the performative-prediction contraction empirically** (Perdomo 2020 + 2024 games extension).
   Build a small learned-`v(S)` loop, measure whether `ε·β/γ<1` under the TMLR 09/2024 validation-set
   adversary. The single most-exposed claim.
6. **Compose, don't compete, with verified-inference.** opML/TOPLOC as the admissibility gate feeding the
   value layer. *"Proof-of-execution certifies what ran; PoM prices what the contribution was worth."*
7. **Position against ERC-8004 as a pluggable backend.** *"ERC-8004 standardized where to post reputation;
   PoM supplies the un-gameable algorithm to compute it."* Consume x402/AP2 settled-value graphs as the
   realized-outcome signal retraining `v(S)`.

**Cross-side honesty:** deployed frontier = Yuma/dTAO, x402, ERC-8004, Vana, Phala, opML/ORA, Story;
designed/published = PEG, SD-PP, TraceRank, "It Takes Two", DeepFunding (pilot). Noesis side: everything
designed, nothing deployed. The defensible lead is narrow + specific: the no-immediate-oracle framing,
topology-only collusion detection without an honest stake majority, and a performative-prediction moat
where the literature is empty — conditional on the contraction holding.
