# Noesis documentation — index

Start at the top and read down by how deep you want to go. Every mechanism doc marks
**built vs designed vs open** honestly; the single source of truth for status is
[`../ROADMAP.md`](../ROADMAP.md) and [`../STUDY-GUIDE.md`](../STUDY-GUIDE.md).

> New here? Read **[NOESIS-FOR-DUMMIES](NOESIS-FOR-DUMMIES.md)** → **[NOESIS-ONEPAGER](NOESIS-ONEPAGER.md)** → **[NOESIS-FAQ](NOESIS-FAQ.md)**, in that order.
> Here to attack it? Jump to **[Security & adversarial](#security--adversarial)**.

---

## Start here (plain-language, shortest first)
| Doc | What it is |
|---|---|
| [NOESIS-FOR-DUMMIES](NOESIS-FOR-DUMMIES.md) | The whole idea with no jargon. The Bitcoin analogy. |
| [NOESIS-ONEPAGER](NOESIS-ONEPAGER.md) | One page: what it is, why it's different. |
| [NOESIS-FAQ](NOESIS-FAQ.md) | The common objections, answered (incl. "can't people farm points?"). |
| [NOESIS-LITEPAPER](NOESIS-LITEPAPER.md) | The thesis in a few pages, between the one-pager and the whitepaper. |

## The full thesis
| Doc | What it is |
|---|---|
| [WHITEPAPER](WHITEPAPER.md) | The complete design. Demonstrated-vs-designed marked throughout. |
| [POM-CONSENSUS](POM-CONSENSUS.md) | Proof-of-Mind: contribution as the consensus weight. |
| [BLOCK-ECONOMY-SPEC](BLOCK-ECONOMY-SPEC.md) | The per-block ownership + value model. |

## Security & adversarial
**This is the section a critic should read — execution, not the idea, is the attack surface.**
| Doc | What it is |
|---|---|
| [../SECURITY.md](../SECURITY.md) | **Start here.** The attack-class defense matrix: gameability · DoS/spam · double-spend · rollback, each with mechanism + honest status + where to verify. |
| [SECURITY-AUDIT-attacker-choosable-inputs](SECURITY-AUDIT-attacker-choosable-inputs.md) | The "never let the attacker choose a security-critical input" audit, surface by surface. |
| [CONSENSUS-REVIEW](CONSENSUS-REVIEW.md) | The adversarial fix-chain (each fix reveals the next attack) + the NCI verification table. |
| [DISPUTE-SLASHING](DISPUTE-SLASHING.md) | Challenge/verdict, causal-share slash, escalation court, anti-griefing bonds. |

## Mechanism deep-dives
| Doc | What it is |
|---|---|
| [TEMPORAL-ORDER-ONCHAIN](TEMPORAL-ORDER-ONCHAIN.md) | Canonical commit ordering recomputed inside the VM; reorder-to-steal-novelty defeated. |
| [T7-CROSS-CELL-SIMILARITY](T7-CROSS-CELL-SIMILARITY.md) | The cross-cell similarity floor (near-duplicate defense). |
| [OUTCOME-EVALUATOR](OUTCOME-EVALUATOR.md) | The learned value model (and its honest null result on real data). |
| [COORDINATION-SCHELLING](COORDINATION-SCHELLING.md) | Coordination / Schelling-point structure. |
| [COHERENCE-LAWS](COHERENCE-LAWS.md) | The conservation/coherence invariants the chain enforces. |
| [CONVERGENCE-REVERSE-FORK](CONVERGENCE-REVERSE-FORK.md) | The reverse-fork: rival chains converge in instead of forking off. |

## Economics
| Doc | What it is |
|---|---|
| [TOKENOMICS](TOKENOMICS.md) | The three tokens + soulbound PoM standing. |
| [CRYPTOECONOMICS](CRYPTOECONOMICS.md) | Incentive structure and the anti-plutocracy properties. |

## On-VM / execution (CKB-VM)
| Doc | What it is |
|---|---|
| [CKB-VM-PORT](CKB-VM-PORT.md) | How the reference rules become on-VM type-scripts. |
| [ON-VM-FINALIZATION](ON-VM-FINALIZATION.md) | The finalization rule recomputed inside the VM. |
| [INDEX-DEP-CODEHASH-BINDING](INDEX-DEP-CODEHASH-BINDING.md) | Cell-dep bound by full script identity, not by shape. |

## Status, meta & reference
| Doc | What it is |
|---|---|
| [../ROADMAP.md](../ROADMAP.md) | **Source of truth for status.** Adversarial-loop log (newest first) + phase plan. |
| [../STUDY-GUIDE.md](../STUDY-GUIDE.md) | Auto-generated map of the codebase ↔ docs. |
| [COMPETITIVE-POSITION](COMPETITIVE-POSITION.md) | Where Noesis sits vs the ecosystem. |
| [PAPERS](PAPERS.md) | The standalone papers (cybernetics, etc.). |
| [VISUALS](VISUALS.md) | Diagrams and figures. |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | How to contribute. |
| [../LAUNCH-CHECKLIST.md](../LAUNCH-CHECKLIST.md) | Pre-launch gate. |

---
*Status legend used across docs: ✅ built & tested · 🟡 designed, not built · 🔬 open problem. If a doc and the ROADMAP disagree, the ROADMAP wins.*
