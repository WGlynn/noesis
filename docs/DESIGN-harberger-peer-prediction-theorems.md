# The two peer-prediction theorems, worked honestly

> **Status: DESIGN / theory, ready-for-critique (2026-07-21).** Works Open Problem #3 of
> `docs/research/something-from-nothing-oracle-free-content-value.md` §5.3 / §7: *graph-generalization*
> and *inner-equilibrium uniqueness* for PEG/SD peer-prediction run over the contribution DAG as the
> oracle-free content signal `v(S)`. Grounded in the ratified composition (`[[augmented-harberger-honest-self-reporting]]`,
> Will 2026-07-19). **This does NOT claim to prove either theorem.** One of them is *provably false in
> isolation*; the honest results are (T1) conditional and (T2) resolved-only-by-composition. Status
> discipline: proven · conditional · open. Citations are author-level and marked `[verify]` — no arXiv/DOI
> is asserted from memory; resolve against sources before any external publication (guardrail: never a
> fabricated citation).

## 0. Why this matters and what "the theorems" actually are

The value seam consumes one deterministic integer per cell (`§5.1`, `ValueOracle`). The 🟡-designed
content signal behind it is peer-prediction: the network's own reports, scored so that truth-telling is
an equilibrium, with no privileged judge. The elicitation literature proves truthfulness for peers
answering *iid tasks under a common prior* (Miller–Resnick–Zeckhauser peer-prediction `[verify]`; Prelec
BTS `[verify]`; Dasgupta–Ghosh multi-task `[verify]`; Shnayder–Agarwal–Frongillo–Parkes Correlated
Agreement / "informed truthfulness" `[verify]`; Kong–Schoenebeck dominant truthfulness `[verify]`).
Noesis does not have that setting. It has a **provenance DAG**: the "peers" of a cell are its graph
neighbors, the "signals" are correlated by shared provenance, and the "tasks" (cells) are heterogeneous
and linked. The two named-open theorems are exactly the two gaps between the classical setting and the
DAG:

- **T1 (graph-generalization).** Does classical truthfulness survive when peers are DAG-neighbors with
  provenance-correlated signals rather than an iid panel?
- **T2 (inner-equilibrium uniqueness).** Is the truthful equilibrium the *unique* equilibrium of the
  inner reporting game, or do uninformative/collusive equilibria coexist and dominate?

## 1. Setup (precise, minimal)

A finalized cell `x` has a latent worth `ω_x ∈ ℝ_{≥0}`. Its **reference set** `R(x)` is a set of other
cells (candidate "peers"). Each identity `i` that reports on `x` emits a signal `s_i` and a report
`r_i`; a peer-prediction score `π(r_i, r_j)` pays `i` by agreement with a reference report `r_j`,
`j ∈ R(x)`, calibrated so that `E[π]` is maximized by truthful `r_i = s_i` **iff** the classical
conditions hold. Write the two classical conditions:

- **(SR) stochastic relevance:** `j`'s signal is informative about `i`'s — `Pr[s_j | s_i]` depends on
  `s_i`. Without it, `j`'s report carries no signal about `x` and the score is uninformative.
- **(CI) conditional independence:** `s_i ⊥ s_j | ω_x` — peers' signals are independent *given* the
  latent worth. This is what makes agreement evidence of truth rather than evidence of a shared cause.

`[[augmented-harberger-honest-self-reporting]]` composes two more legs onto this inner game: a Harberger
self-declared `V_x` carrying rent `ρ·V_x` and slash-at-risk `σ·V_x`, and a dispute market that pays a
challenger the adjudicated gap `V_x − (peer score)`. Call the inner game (peer-prediction alone) `G0`
and the composed game (peer-prediction ⊕ stake ⊕ dispute) `G+`.

## 2. T1 — graph-generalization: **conditional**; Layer A's predicate is a *necessary* part of the condition, not all of it

**Claim (T1).** Classical peer-prediction truthfulness carries over to `G0` on the DAG **iff** the
reference set `R(x)` can be chosen so that (SR) and (CI) hold for neighbor pairs. On a provenance DAG
the load-bearing failure of (CI) is **capital-correlated neighbors** — a wash ring's cells share a
hidden common cause (one actor), so `s_i` and `s_j` are dependent *even given* `ω_x`, and agreement
stops being evidence of worth. Therefore:

> **T1 (conditional).** Peer-prediction over the DAG is truthful on the sub-DAG where the reference set
> is drawn from **capital-independent** neighbors **and** the remaining signal-correlation channels are
> closed. Capital-independence (`node::vesting::independent_use_gate`,
> `docs/DESIGN-periphery-solution.md` §2A: distinct capital cluster) closes the **shared-controller**
> common cause — the wash-ring channel — which is **necessary but not sufficient** for (CI).

**Why this is a real result, but a bounded one (corrected 2026-07-21, `CALIBRATION-ci-argument-2026-07-21.md`).**
The periphery work introduced capital-independence to solve *vesting* (deny a closed ring free downstream
use). The **same predicate is the necessary filter against the shared-controller common cause**, and it
is reused for both scoring and vesting — that reuse is real (one oracle serves two mechanisms). But it is
**not** the *whole* of (CI): the earlier draft's "identically the same set / buys the oracle once" was an
overclaim (necessary rounded up to sufficient). Distinct capital clusters can still share a **public
prior**, **herd on public information**, **semantically copy**, or be **sybils of a distinct third party**
— each an ω-external common cause that `independent_use_gate` (a pure cluster-id compare) does not touch.
So the honest statement: capital-independence is the *necessary* shared-controller filter, common to both
layers; full (CI) needs *additionally* a common-prior / detail-free-CA condition that Layer A does not
supply. Reuse, not identity — [[filter-coincidence-as-structural-edge]] on **one** channel, not all.

**Proof sketch (of the conditional).** Restrict `R(x)` to neighbors in distinct capital clusters and
assume the residual common-prior/herding channels are controlled (the detail-free-CA condition, §4-open).
(SR) holds because a genuine downstream builder's report is informative about `x`'s worth (they built on
it). Under those assumptions the only remaining dependence between `s_i, s_j` is through `ω_x` — the
CA/informed-truthfulness precondition (Shnayder et al. `[verify]`) — so the Correlated-Agreement score
makes truthful reporting a best response, and `G0` restricted to the independent sub-DAG inherits
classical truthfulness. ∎(sketch, under the stated assumptions). The sketch is honest that
capital-independence alone does **not** discharge the ω-external prior; it discharges the controller.

**Honest boundary.** (i) This is a *proof sketch under an assumed independence oracle*, not a machine-
checked proof; the capital-independence oracle itself is 🟡 (`DESIGN-value-oracle-seam.md`). (ii) It
degrades gracefully but does not hold on the capital-*correlated* sub-DAG — which is the funded-cartel
residual, priced not excluded (`periphery-solution` §6, Bitcoin-51%-class). (iii) Heterogeneous worth
across cells means the common-prior assumption is per-neighborhood, not global; multi-task CA tolerates
this but the DAG's overlapping neighborhoods need the detail-free (prior-free) CA variant to avoid a
global-prior assumption — flagged open in §4.

## 3. T2 — inner-equilibrium uniqueness: **false in isolation, resolved by composition**

**The honest fact first.** Uniqueness is **false for `G0`**. Every self-financed peer-prediction
mechanism admits uninformative equilibria: all reporters emit a constant/uncorrelated report, collect
the agreement payment, and no unilateral deviation helps. This is not a gap in our proof — it is a known
impossibility for the standalone inner game (the "uninformative equilibrium" line: Waggoner–Chen `[verify]`,
Gao–Mao–Chen–Zeckhauser `[verify]`; even the strongest positive results — CA informed-truthfulness,
Kong–Schoenebeck dominant truthfulness — make truth *maximal/focal*, they do **not** eliminate the
collusive equilibrium). **So T2 as literally stated (unique truthful equilibrium of `G0`) should be
struck; asserting it would be false.**

**What is true, and it is enough.** The collusive equilibrium of `G0` survives only because reporters
are *indifferent* — the uninformative report is costless. `G+` removes the indifference:

> **T2 (resolved-by-composition).** In `G+`, the uninformative/collusive equilibrium is **not an
> equilibrium**: a reporter who suppresses their signal still (a) carries Harberger rent `ρ·V_x` on
> whatever `V_x` stands, and (b) leaves an unclosed gap `V_x − (peer score)` that any single honest
> challenger can convert to profit via the dispute market. Truthful reporting is therefore the unique
> equilibrium **that survives the stake**, i.e. the unique equilibrium of `G+` in the class where at
> least one challenger is live and bonded.

**Argument.** In a candidate collusive profile, peer scores are uninformative, so declared `V_x` is
unconstrained by content. Two cases. If the ring declares `V_x` high to bank standing, rent + expected
slash on challenge make it negative-EV (`periphery_sim`: closed-wash EV = −36 at honest params; the
challenge is the slash channel). If it declares `V_x` low to dodge rent, it banks ~0 standing — the
collusion earns nothing. Either way a live challenger profits from the gap between the declared value
and the (uninformative ⇒ unsupported) content signal. So no collusive profile is a mutual best response
once the stake and dispute legs are present: the indifference that made it an equilibrium in `G0` is
gone. Truthful reporting, which carries proportional rent and survives challenge, is the surviving
equilibrium. ∎(sketch)

This is precisely §5.3's "the composition is closed": *peer-prediction without stake has no teeth;
Harberger without a truth signal has no principled definition of wrong.* T2's real content is that the
teeth are what collapse the multiplicity the truth-signal alone cannot.

**Honest boundary.** (i) "Unique surviving equilibrium" is conditional on **challenge liveness** — ≥1
honest, bonded challenger. If the *entire* eligible-challenger set is in the ring (global capture), the
gap goes unchallenged; this is again the 51%-class residual, not a new hole. (ii) It assumes the dispute
verdict tracks the content signal well enough that the gap is adjudicable — the `V`-vs-peer-score gap
function is itself listed open (`§5.3` residual (c) / primitive residual (c)). (iii) The
**recognition-lag residual stands unchanged**: a visionary true report ahead of consensus can be scored
low within the finite settlement window `W` and slashed pre-vindication ([[will-paradigm-break-creativity]];
mitigated but not removed by Differential-Incompleteness = diagnose-missing-dimension-then-complete
rather than slash).

## 4. What is proven, conditional, open (no rounding up)

| Piece | State |
|---|---|
| T1 truthfulness on the capital-independent sub-DAG (under an independence oracle) | **conditional** — proof sketch; oracle 🟡; needs detail-free CA to drop the per-neighborhood common-prior |
| T1 capital-independence = Layer A's predicate, reused for scoring | **necessary-not-sufficient** (corrected 2026-07-21) — closes only the shared-controller channel; shared-prior/herding/semantic-copy/3rd-party-sybil remain; NOT "same set / bought once" |
| T2 uniqueness for `G0` (standalone) | **false** — struck; uninformative equilibria are a known impossibility |
| T2 truthful = unique *stake-surviving* equilibrium of `G+` | **conditional** — argued under challenge-liveness; the honest replacement for "uniqueness" |
| Global-capture residual (both theorems bottom out here) | **open/priced** — Bitcoin-51%-class, not structurally excluded |
| Recognition-lag (visionary false-negative within `W`) | **open** — inherent to finite settlement |

## 5. The one honest headline

Neither theorem is "proven as stated." **T1 holds on the sub-DAG where Layer A's capital-independence
predicate holds *and* the residual signal-correlation channels are closed — capital-independence is the
*necessary* shared-controller filter that both scoring and vesting reuse, but it is not *sufficient* for
(CI) on its own** (shared-prior/herding/semantic-copy/3rd-party-sybil remain; detail-free CA still open —
corrected against `CALIBRATION-ci-argument-2026-07-21.md`). **T2 is false in isolation — peer-prediction
alone has collusive equilibria — and that is precisely why the Harberger stake and dispute market are not
optional garnish but the load-bearing legs that make truth the unique surviving equilibrium.** The
composition was ratified before this analysis; this analysis shows the composition is *forced*: remove
the stake and multiplicity returns, remove the shared-controller filter and the wash-ring channel of (CI)
breaks. That is a more honest claim than a standalone QED — bounded exactly, not rounded up — and it
terminates where the whole stack terminates: at global capital capture, priced not excluded.

## 6. What this unblocks / next (measurement-first, all deploy-independent)

1. A `peer_prediction_sim.rs` companion to `periphery_sim`: instantiate a small DAG, a genuine reporter
   set and a colluding ring, score with a CA payment on capital-independent reference pairs, and
   **measure** that (a) the collusive profile is negative-EV once rent+challenge are priced (numeric T2),
   and (b) restricting `R(x)` to independent clusters restores truthful best-response (numeric T1). Same
   discipline as `discernment.rs`: pin the finding so a regression surfaces.
2. Resolve the `[verify]` citations against sources before any external write-up (BTS, CA, dominant
   truthfulness, uninformative-equilibrium impossibility).
3. The detail-free (prior-free) CA variant for overlapping DAG neighborhoods (drops the per-neighborhood
   common-prior assumption in §2 boundary (iii)). **Partially done + measured** (`peer_prediction_sim.rs`
   T1-RESIDUAL section): detail-free CA (cross-task subtraction) **closes** a task-CONSTANT common bias —
   it attenuates toward 0 and never lifts a junk cell above genuine — but does **not** close a
   task-SPECIFIC ω-external correlation (herding on a per-cell public signal, semantic copying,
   coordinated sybil reports). Measured crossover: past **γ\*≈0.70** task-specific coordination a junk
   cell's CA score beats genuine's. So the sharpened residual is *task-specific ω-external correlation*
   (plus the 3rd-party-sybil gap in the capital proxy) — not the four flat channels the calibration
   listed, but a named, measured one. Closing it is genuinely open (the "correlated signals" problem;
   Kong–Schoenebeck-style assumptions `[verify]`).
