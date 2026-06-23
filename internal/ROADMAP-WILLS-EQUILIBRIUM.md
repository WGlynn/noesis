# OFFICIAL ROADMAP — Honest-Contribution Equilibrium on the Contribution Consensus Problem

> **Will 2026-06-23 (official, supersedes feature-priority as the strategic spine):**
> *"you cannot have noesis if you dont have wills equilibrium on the contribution problem."*
> This is THE load-bearing core. Feature/engineering builds (finalization twin-update, clawback,
> lock-sig go-live, deployment) continue as necessary infrastructure, but the SOLUTION-DEFINING work
> is the critical path below. Noesis ⊃ requires Honest-Contribution Equilibrium.

## The problem (named here, name Will-gated)
The **Contribution Consensus Problem** (descriptor: *Consensus Without Ground Truth*): decentralized
agreement on contribution VALUE/attribution — not order — with NO ground-truth oracle, under
SELF-INTERESTED (strategic, ¬ Byzantine) adversaries. Solved when honest contribution AND honest
self-reporting is the equilibrium: **Honest-Contribution Equilibrium** = (1) Nash + (2) coalition-proof +
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
- **M1 — Formalize.** State the Contribution Consensus Problem + the Honest-Contribution Equilibrium definition,
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

## LEAN directive (PONYTAIL / Bitcoin-simplicity — anti over-engineering, (vv) self-audit)
Do NOT build the mechanism zoo. The SIMPLEST solution is **commitment-priority**: publish/commit a
32-byte hash the moment you have the work → consensus temporal-order (`commit_order`) decides priority →
theft becomes structurally impossible (the timestamp exists before any claim — prevention, not
punishment). Will's own 2025 trilogy already proved this (`vibeswap/docs/research/essays/THE_INVERSION_
PRINCIPLE.md`, `.../theorems/THE_PROVENANCE_THESIS.md`). **Sequence:** commitment-priority FIRST (it
handles the published-work majority on machinery that already exists); add bonded self-report + the
peer-prediction layer (M3) ONLY for the residual commitment-priority cannot cover (disputed / never-
committed work). Each added mechanism must earn its place against that residual or be dropped. Keep the
core statable in one sentence: *measure realized contribution; make honest reporting cost less than it
gains.*

## The one-line distance
The STATIC problem (honest is the equilibrium against every known, non-adaptive deviation) is largely
solved in the reference layer — the hard structural foundation is real. The remaining distance is **M2
(adaptive convergence) + M5/M6 (its data and deployment)**: one deep open problem plus the empirical tail
that only deployment closes. M1 and the M2 theory are desk-work; M3/M4 are bounded builds. **Point the
research stack at M2 first** — it is the only claim that is both unproven and the linchpin of un-gameability.

## CHARTER (Will 2026-06-23) — the real-time monitor / continuous immune-surveillance layer
> Will: *"i want to solve all these problems that surfaced from the 4 topics, also i need these all
> working in the future to catch it ALL in real-time. that way things can't escalate in a bad direction."*

The four problem-threads of the session are ONE runtime loop, not four fixes: a continuous per-block
health evaluation over the provenance/value graph (the cybernetic governor / immune surveillance from the
paper, made OPERATIONAL), with every effector firing PRE-finalization so deviations are corrected while
small (negative feedback ⇒ no compounding ⇒ "can't escalate").

| thread | real-time sensor | effector | status |
|---|---|---|---|
| theft / unauthorized spend / fraud | lock-sig gate + owner-published constraints | reject @ spend / parametric clawback | lock-sig ✓ · constraints+clawback ◐ |
| stolen off-chain content | commitment-priority (temporal order) + novelty/duplicate | priority @ commit; damp the later instance | ◐ (trilogy) |
| merge overlap / collusion ring | HodgeRank residual over the graph | `collusion_slash` | ✓ built+tested |
| dishonest self-report | bonded report + peer-prediction score | slash + redistribute, within window | ◐ |

**Unify**: ONE continuous per-block graph-health pass (one bloodstream, many threats), effectors
pre-finalization. Each thread ≡ "an honest equilibrium maintained in real time" ⇒ all four collapse into
the Contribution Consensus Problem.

**HONEST BOUNDARY (post-(vv), reputation-load-bearing):** ✗ catch literally ALL in real-time — structural,
not an engineering gap. Property (3): value reveals OVER TIME via realized downstream flow ⇒ adaptive /
slow-burn attacks (manufactured-value that only looks valueless LATER; the learning adversary) are caught
AS THEY REVEAL by the retraining loop, on a LAG, not instantaneously. So: real-time catches everything that
casts a graph-shadow THIS block; the delayed-value tail is caught lagged, BOUNDED, never zero. "Can't
escalate" holds HARD for observable deviations (homeostat keeps them small); for the unobservable tail it
is bounded escalation that self-corrects as the signal arrives — exactly M2 (how fast/surely the lagged
correction converges). ✗ claim instantaneous total coverage.

**Build path:** the per-thread sensors+effectors above are the pieces; the CHARTER deliverable is the
unified continuous monitor that runs them every block + fires pre-finalization. Gated by the same M1-M6
spine (esp. M2 for the lagged-correction guarantee). Sibling: `[[noesis-as-self-healing-immune-system]]`.

## Not on this spine but required infrastructure (continue in parallel)
finalization PROGRAM twin-update ((tt)) · parametric clawback revocation predicate
(`DESIGN-parametric-clawback.md`) · lock-sig GO-LIVE flip · networking/deploy. Necessary for a live
chain; not the thing that defines whether the problem is solved.
