# M2 — The adaptive-convergence theorem (PRIVATE / stealth)

> M2 of `ROADMAP-WILLS-EQUILIBRIUM.md`: the single claim that is BOTH unproven AND load-bearing for
> un-gameability. Property (3) of the Honest-Contribution Equilibrium asks whether the retraining loop
> `v_{t+1} = T(v_t)` reaches a *unique* un-gameable fixed point rather than oscillating. This file
> states that as a **conditional theorem** borrowed from performative prediction, maps it to Noesis,
> and names exactly what must be discharged. The theory is workable now; the empirics are M5/M6.
> Companion: `DESIGN-wills-equilibrium.md` (M1). Run `/critical-qa` before whitepaper embedding.

## 1. The object
The measure `v` is retrained on the behavior it induces. Borrowing the performative-prediction frame
(Perdomo, Zrnic, Mendler-Dünner, Hardt 2020):

- `D(v)` = the distribution of contributions/reports participants play when the measure is `v` — their
  (game-theoretic) best response to being scored by `v`. The measure shifts the population it measures.
- `T(v) = argmin_{v'} 𝓛(v'; D(v))` = **repeated risk minimization**: retrain the measure on the
  realized outcomes generated under the previous measure. (Or a gradient-step variant `T_η`.)
- A **performatively stable measure** `v_PS` satisfies `v_PS = argmin_{v'} 𝓛(v'; D(v_PS))` — a measure
  that is already optimal for the very behavior it induces. This is precisely the un-gameable fixed
  point: no eventually-discovered exploit is profitable, because the measure has already adapted to it.

## 2. The conditional theorem (the deliverable)
**Theorem (conditional, borrowed).** Suppose:
- **(C1) strong convexity** — `𝓛(·; D)` is `γ`-strongly convex in the measure parameters, for every
  fixed `D`;
- **(C2) joint smoothness** — `𝓛(v; ·)` is `β`-jointly smooth (the loss gradient is `β`-Lipschitz in
  the measure argument);
- **(C3) bounded displacement** — the induced map `D(·)` is `ε`-sensitive: `W_1(D(v), D(v')) ≤ ε·‖v−v'‖`
  (participants' equilibrium behavior moves at most `ε`-Lipschitz in the measure — the
  strategic-classification displacement);
- **(C4) inner-equilibrium uniqueness** — for each `v`, the participants' sub-game has a *unique*
  equilibrium, so `D(v)` is single-valued and `T` is a function, not a correspondence.

Then `T` is a contraction with modulus `ε·β/γ`, and if **`ε·β/γ < 1`** it has a **unique**
performatively-stable measure `v*`, and `T^t(v_0) → v*` geometrically (Banach). At `v*`, honest
contribution + truthful reporting is the unique HCE.

This is the formal content of *"fixed formula → gamed; adaptive measure → un-gameable, and the
adaptation converges."* Existence alone needs far less (Brouwer on a compact convex measure set under
continuity of `T`); **uniqueness + convergence** is what `ε·β/γ < 1` buys.

## 3. Map to Noesis (what each condition means here)
- `𝓛` = the learned-`v(S)` objective (fit realized downstream value-flow on the provenance DAG).
- (C1) holds **by model choice**: a convex/linear `v(S)` head (or a strongly-convex-regularized one)
  satisfies it; a deep nonconvex head does not globally (only a local PL-region argument survives).
  → **design lever**: keep the learned head in the strongly-convex class to *buy the theorem*.
- (C2) `β` = smoothness of the value-fit in the measure — controllable by regularization / the
  retraining step size `η`.
- (C3) `ε` = how much participants' best-response moves when the measure changes — bounded by their
  manipulation-cost (large cost ⇒ small `ε`). The guards `𝒢` (novelty→0, saturation, Hodge-slash)
  *raise* manipulation cost ⇒ *lower* `ε`. So the static guards are not separate from (3): they shrink
  the very `ε` that the contraction needs `< γ/β`.
- (C4) is the **subtle one** — see §4.

## 4. What is OPEN (honest — do not claim discharged)
1. **(C4) inner-equilibrium uniqueness / the bilevel coupling.** At each `v`, participants play a game
   *among themselves* (their reports/contributions interact through the Hodge residual and the shared
   allocation), so `D(v)` is the outcome of an inner Nash *map*, not a single agent's best response. If
   the inner game has multiple equilibria, `T` is a correspondence and the clean contraction breaks.
   The real M2 work is showing the inner game has a unique equilibrium (or selecting one measurably) so
   the Stackelberg fixed point is well-posed. **This is the genuinely novel proof obligation** — it is
   our setting, not off-the-shelf performative prediction.
2. **`ε·β/γ < 1` is an assumption, not a theorem, until the three constants are bounded for the actual
   `v(S)` model class + the actual participant cost model.** Plausible (guards make `ε` small, convex
   head makes `γ` real), but it must be *derived*, with the regime where it fails stated.
3. **Uniqueness was OPEN in our own prior iterated-Shapley fixed-point work** — the Brouwer existence
   argument there never gave uniqueness. M2 must either supply the contraction (C1–C4 ⇒ `<1`) or
   honestly remain *existence-only* (a stable point exists; we do not claim it is unique/global).
4. **Empirical** — even with the theorem, that the *real* loop on *real* DeepFunding labels lands in
   the contraction regime is M5/M6 (data + deployment). By nature, adaptation to not-yet-arrived
   adversaries is confirmable only over time; the theorem bounds the *rate*, deployment confirms the
   *constants*.

## 5. The honest one-paragraph M2 result (whitepaper-ready, conditional)
> Under strong convexity and smoothness of the value-fit, bounded strategic displacement, and a unique
> inner equilibrium, the Noesis retraining loop is a contraction and converges geometrically to a
> unique performatively-stable measure `v*` at which honest contribution and reporting is the unique
> Honest-Contribution Equilibrium; the static guards (novelty, saturation, Hodge-slash) enter the proof
> by shrinking the displacement constant `ε` toward the contraction threshold. The open work is
> discharging the four conditions for our specific value model — in particular establishing uniqueness
> of the inner equilibrium so the loop is a well-posed map rather than a correspondence — and bounding
> `ε·β/γ < 1` empirically on real outcome data (M5/M6). We therefore state adaptive-stability as a
> *conditional* theorem with explicit hypotheses, not an unconditional result.

## 6. Next
- Discharge **(C4)** first — it is the load-bearing, genuinely-novel obligation (the others are
  borrow-and-bound). Try: monotone-game / potential-game structure on the inner sub-game to force a
  unique equilibrium (the Hodge potential is a candidate potential function — investigate).
- Then bound `ε` from the guard-induced manipulation cost; bound `γ, β` from the chosen `v(S)` head.
- `/critical-qa` this file (mechanism / edge / confidence / composition / honesty) before whitepaper.
