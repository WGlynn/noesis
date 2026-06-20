# Will's Equilibrium — design note (PRIVATE / stealth)

> Will, 2026-06-19: "want my own mathematical economics game theory on this called Will's Equilibrium."
> Captured in-flight. This is the grounded definition sketch + the proof obligations + the honest
> positioning vs existing equilibrium concepts. NOT yet in the whitepaper — embedding a named
> equilibrium requires the formal definition + a proposition that the PoM game induces it, done in a
> fresh low-context pass and run through /critical-qa, or a hostile reader uses a vacuous label to
> discredit the whole paper. Feeds whitepaper §"The economic frame" and possibly a new short section.

## The one-line claim

Proof of Mind does not just reach a Nash equilibrium where honest contribution is individually
rational. It reaches one that **also survives coalitions and survives an adaptive adversary**, because
the value measure itself adapts. That third property is the novel part, and it is the formal statement
of "any fixed formula gets gamed the moment it is public; only a measure that adapts is un-gameable."

## The game (to be formalized)

Players are participants on the chain. Each chooses a strategy over: what to contribute, whether to
pad / sybil / collude, whether to honestly build on others' work. Payoff = PoM standing + state-capacity
earned, as scored by the mechanism: cooperative-game value `v(S)` over the provenance DAG, with
temporal novelty (sybil/padding -> 0), geometric saturation (volume cannot pump), the HodgeRank
harmonic residual (collusion circulation flagged and slashed), and a **learned `v(S)` retrained on
realized downstream outcomes** (the adaptive measure / AMD control loop).

## Definition (sketch) — Will's Equilibrium

A strategy profile `s*` is a **Will's Equilibrium** of the contribution game under mechanism `M` if:

1. **(Nash)** no participant has a profitable unilateral deviation;
2. **(Coalition-proof)** no coalition of any size has a profitable joint deviation — collusion rings,
   mutual-citation, and sybil pools are structurally zeroed (temporal novelty + saturation + HodgeRank
   harmonic residual), not merely deterred by cost;
3. **(Adaptive stability / Goodhart-robust)** `s*` remains an equilibrium under the mechanism's
   measure-update dynamic against an adversary who learns: when an adversary discovers an exploit, the
   retraining of `v(S)` on realized outcomes moves the best-response surface to close it, so no
   *eventually-discovered* deviation is profitable, not only no *currently-known* one.

(1)+(2) alone is close to a coalition-proof / strong Nash equilibrium. **(3) is the contribution**:
an equilibrium of a game whose payoff function is itself a learned object that co-adapts with the
adversary.

## Why this is NOT just Nash (honest positioning vs prior art)

- **Nash equilibrium** — unilateral deviations only, fixed payoffs. WE adds (2) and (3).
- **Strong Nash (Aumann) / Coalition-proof NE (Bernheim–Peleg)** — handle coalitions, but assume a
  FIXED payoff function. WE adds the adaptive-payoff dimension (3).
- **Evolutionarily stable strategy (ESS)** — dynamic, but a fixed fitness landscape. In WE the
  landscape itself moves adversarially (a principal's control loop), which ESS does not model.
- **Closest real literature to cite, not pretend we invented:** performative prediction / strategic
  classification (the measure adapts to those being measured), Goodhart-robust and online/adversarial
  mechanism design, Stackelberg learning. WE is, honestly, **the equilibrium of a performative
  value-measurement game on a value chain, with coalition-proofness from the provenance geometry.**
  That fusion (performative/Goodhart-robust + coalition-proof + made the consensus object) is the
  defensible novelty. The name is Will's; the lineage must be cited.

## Proof obligations (what makes it grounded, not a label)

- **Existence / convergence.** Does a WE exist, and does the retraining loop converge rather than
  oscillate? Map to a fixed-point argument (cf. iterated-Shapley fairness fixed point — existence via
  Brouwer is plausible; **uniqueness is OPEN** in our own prior work, state that honestly). Performative
  prediction gives convergence results under conditions (e.g. strong convexity / smoothness of the
  retraining) — borrow those conditions explicitly.
- **The three properties are each enforced by a named mechanism component** (claim-needs-structural-
  enforcer): (1) standing-gating in `value_v6`; (2) temporal novelty + geometric saturation + HodgeRank
  residual (the adversarial-loop log in ROADMAP.md (q)–(bb)); (3) the learned-`v(S)`-retrained-on-
  outcomes loop — **which is data-blocked / DESIGNED-not-DEMONSTRATED today (the moat)**.
- Therefore in the paper WE must be marked **demonstrated for (1)+(2), designed for (3)** — the same
  honesty register as the rest of the paper. Do not claim (3) is proven.

## Honest risk

If we cannot state at least a conditional existence result and the precise distinction from coalition-
proof NE + performative-prediction equilibria, then "Will's Equilibrium" is a naming, not a theorem,
and should stay a *named conjecture* ("we conjecture the PoM game admits an equilibrium that is
coalition-proof and Goodhart-robust; we call it Will's Equilibrium") rather than a claimed result.
A named conjecture, openly labeled, is defensible. A named theorem without the theorem is not.

## Next (fresh low-context, run through /critical-qa)

1. Formalize the game (players, strategy space, payoff = `v(S)` with the four guards + retraining op).
2. State WE as Definition; state existence as Proposition (with conditions) or as labeled Conjecture.
3. Cite performative prediction / coalition-proof NE / ESS explicitly; claim only the fusion.
4. Decide placement: a short subsection under §"The economic frame", or its own §.
5. Only then embed in the whitepaper, marked demonstrated-vs-designed per component.
