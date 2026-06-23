# A1 — PEG / SD-PP proof template for the Honest-Contribution Equilibrium (PRIVATE / stealth)

> The highest-leverage move in `ROADMAP-LEAN-INTO-EDGE.md` (Track A1). Adapts two *verified* published
> results into a proof template that elevates HCE's self-report properties from *designed* to *designed;
> proof-templated, with two named open theorems* — NOT "proven". The remaining gaps are named precisely
> below. Status authority: `STATUS-LEDGER.md`. Companions: `DESIGN-wills-equilibrium.md`
> (M1), `DESIGN-adaptive-convergence-theorem.md` (M2). Run `/critical-qa` before whitepaper embedding.
> Papers fetched + verified 2026-06-23 (arXiv abstracts), not taken on the research agent's word.

## 1. The two source results (verified, cite precisely)
- **PEG — Peer Elicitation Games** (Chen, Zhu, Han, B. Li, G. Li, Dai; arXiv:2505.13636, May→Oct 2025).
  A training-free game: one generator + multiple discriminators instantiated from *distinct* base models;
  utilities computed from a **determinant-based mutual-information score**. Proves, with NO ground-truth
  labels: (i) the MI score incentivizes truthful reporting; (ii) each agent attains sublinear regret via
  online learning; (iii) **last-iterate convergence to a truthful Nash equilibrium** — actual policies
  converge to stable truthful behavior, not just time-average.
- **SD-PP — Stochastically Dominant Peer Prediction** (Zhang, Xu, Pennock, Schoenebeck; cs.GT). Strengthens
  the truthfulness target: truth-telling's *score distribution* **stochastically dominates** every other
  strategy ⇒ honesty is optimal under *any monotone utility*, not only in expectation. Constructions:
  binary-lottery rounding; the **Enforced Agreement (EA)** mechanism (SD-truthful in binary-signal,
  mild assumptions).

## 2. Why these are the right template (the mapping PEG → Noesis)
| PEG / SD-PP | Noesis / HCE |
|---|---|
| generator emits a report; multiple discriminators score it | a contributor's self-report on contribution provenance/quality; multiple validators/peers score it |
| determinant-based MI score, no ground truth | the score that supplies the catch-probability `p` for the `nash_honesty` IC — **without an oracle** (the M3 gap) |
| truthful reporting = Nash equilibrium | HCE property (1), self-report half |
| last-iterate convergence to the truthful NE | the M2 adaptive-convergence claim (the retraining loop lands on the truthful fixed point, not oscillation) |
| SD-truthfulness under any monotone utility | robustness of (1)+(2) to risk-seeking / non-linear-payoff colluders — hardens property (2) self-report collusion (M4) |

## 3. The conditional theorem we can now state (claim elevation)
**Proposition (template, conditional).** Instantiate the PoM self-report layer as a Peer Elicitation Game:
contributors report, ≥2 independent validators score via a determinant-based mutual-information score over
the reports. Then, by PEG (i)–(iii):
- truthful self-reporting is a **Nash equilibrium with no ground-truth oracle** — this *supplies the
  catch-probability `p`* that `nash_honesty` (`p·b ≥ (1−p)·g`) was conditional on, closing the M3 gating
  of property (1) **in template**;
- the agents' learning dynamic exhibits **last-iterate convergence** to that truthful equilibrium — a
  concrete realization of the M2 convergence claim on the self-report sub-game;
- replacing the expected-value target with SD-PP's **SD-truthfulness** makes truth **payoff-dominant**
  under *any monotone utility*. This removes the **risk-attitude loophole** (a risk-seeking reporter
  cannot prefer to lie) — a property of a **unilateral** deviation. It does NOT by itself eliminate the
  **symmetric-lie co-equilibrium**, where a coalition jointly agrees on the same falsehood; that JOINT
  deviation is the separate M4 obligation (bonded BTS backstop). SD-truthfulness hardens the unilateral
  half of property (2), not the joint half.

This moves HCE's self-report properties from **designed** to **designed; proof-templated by PEG/SD-PP,
with two named open theorems (graph-generalization + C4 inner-uniqueness)** — NOT "proven". The components
PEG/SD-PP prove are (1)-self-report `p` and a self-report-layer instance of the M2 convergence; for
(2)-self-report-collusion they give only the UNILATERAL robustness (SD-truthfulness removes the
risk-attitude loophole), which does NOT eliminate the symmetric-lie co-equilibrium (a JOINT deviation —
see §4). Status authority: `STATUS-LEDGER.md` HCE-1-report, HCE-2-selfreport.

## 4. The remaining obligations (honest — what PEG does NOT give us)
PEG/SD-PP prove their results for **single-fact reporting among symmetric discriminators**. HCE needs two
generalizations they do not cover; these are the real, novel work:
1. **Graph-generalization of the MI score.** PEG scores a single report; HCE scores **cooperative-game
   value `v(S)` over a provenance DAG**. The determinant-based mutual-information must be defined over
   *graph-structured value reports* (reports about edges/attribution in the DAG), and the truthfulness/
   convergence proofs re-derived for that object. Open. (HodgeRank's harmonic structure is a candidate
   coordinate system for the determinant — investigate whether the MI determinant and the Hodge residual
   share an operator.)
2. **Inner-equilibrium uniqueness (M2 condition C4).** PEG's last-iterate convergence is for its specific
   game; transporting it to the *Noesis retraining map* `T` requires `D(v)` single-valued — the inner
   game among reporters must have a unique equilibrium (else `T` is a correspondence). This is the same
   C4 obligation flagged in M2; PEG narrows it (its game converges) but does not discharge it for the
   graph-valued generalization. Candidate tool: monotone / potential-game structure on the inner game,
   with the Hodge potential as the potential function.

**Net honest status:** with A1, HCE self-report = *designed; proof-templated by PEG/SD-PP, with two named
open theorems remaining — (1) the graph-generalization of the determinant-MI score and (2) C4 inner-
uniqueness for the graph game* — plus, for the (2)-self-report-collusion half specifically, the joint
symmetric-lie deviation that SD-truthfulness does not cover (M4). This is NOT "proven": it is a sharp,
defensible elevation that NAMES the theorems left to prove rather than papering over them. Status
authority: `STATUS-LEDGER.md`.

## 5. Next
- Draft the graph-generalized determinant-MI score over the provenance DAG; test the HodgeRank-determinant
  operator conjecture (4.1).
- Discharge C4 via a potential-game argument (4.2) — shared obligation with M2 §6.
- `/critical-qa` this file (mechanism / composition / honesty) before it enters whitepaper §9 / the M1 paper.
- Fold the elevated status into `DESIGN-wills-equilibrium.md` §4 table (self-report rows: ◐ → designed; proof-templated, two named open theorems). Done — both cite `STATUS-LEDGER.md`.
