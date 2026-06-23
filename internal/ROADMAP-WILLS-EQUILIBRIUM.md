# OFFICIAL ROADMAP — Will's Equilibrium on the Contribution Consensus Problem

> **Will 2026-06-23 (official, supersedes feature-priority as the strategic spine):**
> *"you cannot have noesis if you dont have wills equilibrium on the contribution problem."*
> This is THE load-bearing core. Feature/engineering builds (finalization twin-update, clawback,
> lock-sig go-live, deployment) continue as necessary infrastructure, but the SOLUTION-DEFINING work
> is the critical path below. Noesis ⊃ requires Will's Equilibrium.

## The problem (named here, name Will-gated)
The **Contribution Consensus Problem** (descriptor: *Consensus Without Ground Truth*): decentralized
agreement on contribution VALUE/attribution — not order — with NO ground-truth oracle, under
SELF-INTERESTED (strategic, ¬ Byzantine) adversaries. Solved when honest contribution AND honest
self-reporting is the equilibrium: **Will's Equilibrium** = (1) Nash + (2) coalition-proof +
(3) adaptive-stable. Spec: `DESIGN-wills-equilibrium.md`. Problem memory: `[[contribution-consensus-problem]]`.

## Honest status by property (✓ demonstrated · ◐ designed · ○ open/research)
### (1) Nash — no profitable UNILATERAL deviation — **essentially picked**
- ✓ honest contribution unilaterally rational — structural (novelty→0 padding, geometric saturation,
  standing-gating); demonstrated in `value` + the gaming suite.
- ✓ honest self-report IC — PROVEN (uu) `nash_honesty`: `p·b ≥ (1−p)·g`, computational proof.
- ◐ **GATING:** the IC proof is conditional on the catch-probability `p`. The peer-prediction layer
  that supplies a high `p` WITHOUT ground truth (truthful reporting as a BNE over peers' reports) is
  DESIGNED, not built. Until built, (1) for self-reporting is "proven modulo one component."
- ◐ formal theorem-grade statement of (1) for the PoM game — cold + `/critical-qa`.

### (2) Coalition-proof — no profitable JOINT deviation — **half-turned (hard half done)**
- ✓ cyclic collusion (rings / mutual-citation / manufactured flow) — the HodgeRank harmonic-energy
  certificate detects it on topology alone, wired to slash (`collusion_slash`, `unified_slash`).
  Demonstrated + tested.
- ○ **OPEN:** the self-report COLLUSION equilibrium (the classic peer-prediction weakness — everyone
  agrees on the same lie). Elimination via a bonded + Bayesian-Truth-Serum information-score backstop is
  DESIGNED, not proven. This is the (2) gate.
- ◐ formal coalition-proofness statement (collusion zeroed by the provenance geometry) — cold.

### (3) Adaptive-stability — honest stays the equilibrium vs an adversary who LEARNS — **THE FRONTIER**
- ◐ learned-`v(S)` retraining harness — wired (`load_prefs → train → v_outcome_floored → seed`),
  **DATA-BLOCKED**: needs real DeepFunding outcome labels at scale to show it closes the Goodhart gap.
- ○ **THE LINCHPIN — the adaptive-convergence theorem:** does the retraining loop converge to an
  un-gameable FIXED POINT rather than oscillate? Existence plausible via Brouwer (cf. iterated-Shapley
  fairness fixed point); **uniqueness OPEN in our own prior work**. This is the single item that is BOTH
  unproven AND the load-bearing claim of the whole un-gameability story ("fixed formula → gamed; adaptive
  measure → un-gameable"). Borrow performative-prediction convergence conditions (strong-convexity /
  smoothness of the retraining) explicitly.
- ○ empirical robustness — by its NATURE only fully confirmable IN DEPLOYMENT (adaptation to real,
  not-yet-arrived adversaries cannot be proven in a lab). Finishing (3) is inseparable from running the
  live network and surviving over time.

## The critical path (ordered by leverage, honest about kind-of-work)
- **M1 — Formalize.** State the Contribution Consensus Problem + the Will's Equilibrium definition,
  paper-grade, with the precise positioning vs coalition-proof-NE / performative-prediction / ESS (claim
  only the fusion). Companion paper to the cybernetics one. **[desk; cold + `/critical-qa`]**
- **M2 — The linchpin: adaptive-convergence theorem.** Existence + uniqueness conditions of the
  retraining fixed point. The highest-leverage RESEARCH item; the THEORY is workable NOW (the empirical
  half is teed to M5/M6). **[research; workable now]**
- **M3 — Peer-prediction self-report layer.** Build the mechanism that supplies `p` without ground truth
  (truthful reporting a BNE over peers). Makes (1) real, not conditional. **[build]**
- **M4 — Self-report collusion-eq elimination.** Bonded + BTS information-score backstop that kills the
  symmetric-lie equilibrium; with a proof obligation. Makes (2) real. **[build + proof]**
- **M5 — Data acquisition (DeepFunding labels at scale).** Unblocks (3) empirical — the moat. **[data]**
- **M6 — Empirical adaptive-robustness in deployment.** The only place (3) is finally confirmed. **[deploy]**

## The one-line distance
The STATIC problem (honest is the equilibrium against every known, non-adaptive deviation) is largely
solved in the reference layer — the hard structural foundation is real. The remaining distance is **M2
(adaptive convergence) + M5/M6 (its data and deployment)**: one deep open problem plus the empirical tail
that only deployment closes. M1 and the M2 theory are desk-work; M3/M4 are bounded builds. **Point the
research stack at M2 first** — it is the only claim that is both unproven and the linchpin of un-gameability.

## Not on this spine but required infrastructure (continue in parallel)
finalization PROGRAM twin-update ((tt)) · parametric clawback revocation predicate
(`DESIGN-parametric-clawback.md`) · lock-sig GO-LIVE flip · networking/deploy. Necessary for a live
chain; not the thing that defines whether the problem is solved.
