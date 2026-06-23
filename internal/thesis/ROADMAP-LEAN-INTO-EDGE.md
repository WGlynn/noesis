# ROADMAP — Leaning into the edge (PRIVATE / stealth)

> Derived 2026-06-23 from `docs/research/FRONTIER-BRIEF-2026-06-23.md` + `MOAT-STACK.md`. The actionable
> overlay on `ROADMAP-WILLS-EQUILIBRIUM.md` (M1–M6): *how* to press the advantage the frontier scan
> confirmed. Not a new spine — a leverage-ordering of the existing one.

## The edge, honestly (one line)
**Narrow + specific + paper-stage:** (a) the *no-immediate-oracle* framing of the Contribution Consensus
Problem; (b) topology-only collusion detection that does NOT need an honest stake majority (HodgeRank
harmonic residual); (c) an adaptive/Goodhart-robust value measure (performative-prediction contraction)
— sitting exactly where the literature is empty. Everything deployed is someone else's (Yuma/dTAO,
x402, ERC-8004, Vana, opML, Story). We are ahead on rigor, behind on deployment.

## The strategy (one line)
**Harden the unique, compose the rest, never compete on deployment.** Make the three unique claims
*undeniable* (designed→proven + one reproducible demo) because a paper-edge erodes the moment a rival
ships; and plug INTO the deployed substrate (turn incumbents into distribution + the data signal) rather
than rebuilding what already works.

---

## Track A — Harden the unique claims (designed → proven). THE core lean-in.
| id | move | deliverable | leverage | M-spine |
|---|---|---|---|---|
| **A1** | **PEG-template proof for HCE.** Read PEG (NeurIPS 2025, arXiv 2505.13636) + SD-Peer-Prediction (2506.02259). Adapt the determinant-based mutual-information utility + last-iterate-convergence proof to `v(S)`-payouts. | A theorem: HCE is the focal, collusion-blocking equilibrium under self-interested agents — upgrades the core claim from *designed* to *proven-in-template*, and discharges part of the (2) self-report-collusion gate + supplies the M2 convergence template. | **HIGHEST.** It is the M1+M2 theory linchpin and the single most claim-elevating move. | M1 + M2 |
| **A2** | **Measure the performative contraction `ε·β/γ<1` empirically.** Read Perdomo 2020 (ICML) + the 2024 performative-on-games extension. Build a small learned-`v(S)` retraining loop; use the TMLR 09/2024 Data-Shapley validation-set attack as the adversary. | Evidence the moat's contraction condition is real, not assumed — the single most-exposed claim (property 3 / moat-3). | HIGH (de-risks the moat). Data-gated → see C2/M5. | M2-empirical / M5 |
| **A3** | **State the Cheng-Friedman axiom-relaxation, precisely.** One paragraph naming the axiom (symmetry/anonymity) PoM relaxes. | Positioning line: *"PoM escapes the Cheng-Friedman Sybil impossibility by relaxing anonymity: commit-reveal timestamp-priority on a PoW-anchored (JUL) identity makes a fresh identity structurally worth zero, so false names cannot inherit standing."* Makes JUL load-bearing for coalition-proofness. | CHEAP, immediate, unblocks whitepaper §9. | M1 |

## Track B — Prove the differentiator visibly (one reproducible demo).
| id | move | deliverable | leverage |
|---|---|---|---|
| **B1** | **Head-to-head vs TraceRank** (the nearest *deployed* topological competitor, arXiv 2510.27554). Build the mutual-endorsement-ring scenario its flat propagation can't catch and HodgeRank can. | A reproducible *"PoM catches what TraceRank misses"* experiment for the whitepaper — converts "better on paper" into a demonstrable win on the topology-only-collusion edge. | HIGH, concrete, shippable, independent of A1. |

## Track C — Compose into the deployed substrate (incumbents → distribution + data).
| id | move | positioning line |
|---|---|---|
| **C1** | **opML/TOPLOC as the admissibility gate** feeding the value layer (fits the CKB cell model better than zk-cost / TEE-trust). | *"Proof-of-execution certifies what computation ran; PoM is the orthogonal layer that prices what the contribution was worth."* |
| **C2** | **ERC-8004 reputation/validation backend** — its registries are empty shells. Consume x402/AP2 settled-value graphs as the realized-outcome signal that retrains `v(S)`. | *"ERC-8004 standardized where to post reputation; PoM supplies the un-gameable algorithm to compute it."* Doubles as the **data pipe for A2.** |
| **C3** | **Watch + engage DeepFunding** (Buterin/Gitcoin — closest public effort, jury-anchored). Public artifacts only (NDA discipline). | *"The strategy-proof, jury-free successor to DeepFunding's allocator + human-jury design."* |

---

## Sequenced critical path
1. **A3** (today-scale) — cheapest, unblocks positioning + §9. Write the paragraph.
2. **A1** (cold, `/critical-qa`) — **the move.** The PEG-template proof. This is where the edge becomes undeniable.
3. **B1** (parallel build) — the TraceRank demo; concrete, independent of A1.
4. **C2 / C1** — wire the composition; C2 supplies the realized-outcome data A2 needs.
5. **A2** — measure the contraction once C2 (or M5 DeepFunding labels) supplies data.
6. **C3** — ongoing watch/engage.

## Why this is the right shape (the design-pattern note)
The frontier scan didn't just spare our patterns — it **empirically validated them**: "structural > prompt-
level honesty" proven (arXiv 2601.11369); Goodhart shown to be an *impossibility* whose only escape is to
make genuine contribution and collusion-resistance *coincide* (Plural QF) = our [[honesty-as-structural-
load-bearing-property]] / [[filter-coincidence-as-structural-edge]]; non-linearity confirmed as *the*
Sybil lever = [[substrate-geometry-match]]. So "lean into our design patterns" = keep running
dissolution-over-defense + Augmented-Mechanism-Design + substrate-geometry-match — now with external
citations backing every primitive, claiming only the **fusion**. The roadmap is that discipline applied:
harden where we're singular, cite where the field agrees, compose where we're behind.
