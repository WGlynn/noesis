# `v(S)` as a completion procedure — the unifying frame for the moat

> **Status: FRAME / design synthesis (2026-07-02).** Not a build. This doc unifies two
> already-teed moat candidates (ROADMAP cand-A isomorphism-invariance, cand-B legitimacy
> guard) under one theory, and states precisely what that theory does and does not buy.
> It is a `v(S)` / gameability surface only — **not finality**, never the safety path.
>
> **Theory anchor (public):** W. T. Glynn, *Differential Incompleteness: Value Disputes
> Are Missing Dimensions*, Zenodo 2026, DOI 10.5281/zenodo.21150665. The paper is
> mechanism-free and names its "concrete instantiation in a valuation and consensus
> setting" as deliberately out of scope. **That instantiation is this: `v(S)`.** The
> paper is the epistemology of this exact moat, published without naming it.
>
> **Grounding:** this doc asserts nothing about `v(S)` internals beyond what is stated in
> `ISOMORPHISM-INVARIANCE-VS.md` and `ROADMAP.md` (2026-06-29 research input). Read those
> for the primary claims; this doc is the frame over them.

## 0. The resolution, stated first (Will, 2026-07-02)

The whole frame reduces to one already-named idea, and it should be read before anything
below it: **objectivity is a cardinal direction, not a destination.** The paper (§4, §11)
does not promise a finished, un-gameable `v(S)`; it names a compass. `v(S)` is an estimator
reaching for the one cardinal target — real contribution value, social utility, objective
truth (paper §12) — and the completion loop's only job is to keep the needle calibrated so
`v(S)` gets **progressively less wrong over time.**

This retires the wrong question. The moat was never "prove `v(S)` complete / un-gameable"
(a destination — the god trick, unreachable by construction). It is "keep walking the
cardinal direction with what we already have." So the un-gameable moat is **not a matter of
*if*, but *when*** — conditional only on continuing to work toward it, because the method
*is* the progress. The residue below is not a hole in the moat; it is the length of road not
yet walked. A limit is not a defeat (§11). Everything after this section is *how to walk*,
not *whether we arrive*.

The only checks that survive this framing are **compass-calibration** checks (is the needle
pointing at truth, is the backstop real), not completeness checks. They are marked as such,
and were run 2026-07-02 — **results in §5.1.** Headline: the compass is 🔬 magnetized on the
cyclic sub-problem only (the general certificate is road-not-yet-walked, not a defeat), and
the consensus floor is ⚠ **circular** against v(S)-gaming on the PoM axis (it buys
capital-orthogonality, not a scoring backstop) — the §4 and §5(b) claims were overstated and
are corrected below.

## 1. The moat problem is a differential-incompleteness problem

The open 🔬 item is an un-gameable learned `v(S)`. Every closed gaming vector has had the
same shape (`ISOMORPHISM-INVARIANCE-VS.md` §1): the attacker finds an axis the current
scoring basis cannot see and pumps score along it — sybil relabeling, volume padding,
citation rings, paraphrase-padding. The first real-data test of a learned proxy came back
inconclusive for the same reason: a semantic paraphrase slipped past a byte-level proxy.

Stated in the paper's terms, this is not a bug in a particular `v(S)`. It is the **god
trick** in engineering form (paper §10): *any fixed reward/score is incomplete along the
axes its authors' position could not see, and a capable optimizer will find them and eat
them, because an unweighted axis is free energy.* The failed test was not a failure of
effort. It was aimed at an impossible object: a **frozen** `v(S)` that is un-gameable.

The consequence is liberating, not discouraging: **stop trying to build an un-gameable
fixed `v(S)`.** There is no such thing, for the same reason there is no complete fixed
reward function and no view from nowhere. The achievable target is different in kind.

## 2. The achievable target: `v(S)` as a self-completing basis

Paper §9 (the mechanism-design warning, written into the theory on purpose): *any system
that scores contributions on a fixed set of dimensions is biased by construction and
gameable along every axis it cannot see. The fix is not a better fixed basis. It is a way
to discover the dimensions it is missing, driven by the disputes its own outputs provoke.*

Applied to `v(S)`: a gaming attempt that scores high but the network disputes (a probe
fires, a relabeling changes the score, a reviewer flags "this is a copy, it added
nothing") is **not noise to suppress. It is the diagnostic signal that `v(S)`'s basis is
missing a dimension.** The attack is the sensor. `v(S)` becomes aligned-by-completion, not
specified — a loop, not a frozen function.

This reframes cand-A from "solve the general invariance gate (graph-iso-hard, 🔬 unsolved)"
to "cand-A is **one dispute-generator** feeding a completion loop." The generality that was
unsolvable as a fixed object becomes unnecessary: you do not need to enumerate all
invariants a priori. You need the loop to close each gap faster than gamers open the next.

## 3. cand-A and cand-B are the two halves of ONE procedure

The roadmap lists them as separate items. Under the frame they are the detect-half and the
preserve-half of a single completion loop:

- **cand-A (invariance probe) = the detect-half.** It certifies a missing dimension exists
  without naming it. `|v(σ·S) − v(S)|` over structure-preserving relabelings is the
  gaming-energy signal (`ISOMORPHISM-INVARIANCE-VS.md` §1). When it is non-zero on a
  maneuver honest work would leave at zero, the basis is provably incomplete along that
  axis, in the open — the paper's "completeness is a status a basis loses the instant an
  articulable dimension reorganizes the judgment" (§4), instantiated on `v(S)`.

- **cand-B (legitimacy regression guard) = the preserve-half.** Every `v(S)` patch must
  keep a regression set of KNOWN-LEGIT contributions scoring (ROADMAP cand-B). This is the
  paper's §11 manipulation guard and recognition test made operational: completing the
  basis (adding a dimension / patching the gate) must not introduce a NEW blind spot that
  cuts honest work. It is what stops the loop from being gamed *through its own completion*
  — a manufactured "dimension" that flips scores an attacker's way fails because it also
  breaks the legit regression set.

Detect a missing dimension (cand-A), complete the basis, verify honest work survives the
completion (cand-B), repeat. That is `v(S)` as a completion procedure. Neither half is the
moat alone; the loop is.

## 4. The correspondence: HodgeRank's harmonic residual is ONE built instance of the incompleteness certificate

This is the load-bearing correspondence. It is a true isomorphism — but only on the
**cyclic-relabeling sub-problem**, and the earlier framing ("it is not an analogy, it is the
same math ... these are the same object") **overstated it**. Calibrated statement below;
full reconciliation in §5.1.

- Paper §3: *the residual, the structured part of the disagreement that no weighting of the
  current axes explains, is a computable object. It does not name the missing dimension, but
  it certifies that one exists and characterizes where it must lie.*
- `ISOMORPHISM-INVARIANCE-VS.md` §3: the Helmholtz–Hodge decomposition of the net
  cross-identity flow splits into a **gradient** part (the honest global builds-upon
  potential — invariant, zero-energy, honest work sits here) and a **harmonic residual**
  (the divergence-free cyclic component — precisely the subspace a relabeling attack injects
  energy into). The harmonic energy `‖Y‖² − b·s` is invariant under identity permutation:
  a function of the graph up to isomorphism, not of the labels
  (`node/src/lib.rs:270-293` input = net cross-identity citation-flow graph, self-edges
  skipped at `:288`; `:321-323` energy = `norm_sq − proj`; provably `0` iff acyclic, `= k`
  for a `k`-cycle, `:264-265`).

**What is actually proven (honest altitude).** Both objects share ONE schema: *the residual
left after projecting the observed data onto the subspace the current basis can explain.*
That is a real isomorphism at the **genus** level (the BECAUSE test passes). But the Hodge
object is **one species** of it: its input is fixed (the cross-identity citation-flow graph),
its basis is fixed (a single scalar "builds-upon" potential `s`), so its residual certifies
exactly ONE kind of incompleteness — **cyclic citation topology**. It always points *"there
is a cycle"*, never *"there is a missing value dimension of type X"*. The paper's residual,
by contrast, ranges over **value-judgment** disagreement, and the missing thing is a latent
**value axis** — and `value_v8`'s actual basis is a fixed 4-vector
`coalition_features = [breadth, synergy, connected, depth]` (`lib.rs:6794-6835`). A dimension
missing from THAT basis is invisible to a Hodge residual computed on citation flow: a
different object entirely.

The companion doc says exactly this against itself: the isomorphism-invariance gate is *"the
**generalization** of `attribution_cycle_energy` from the cycle class to all relabeling
classes"* (`ISOMORPHISM-INVARIANCE-VS.md` §3) — i.e. cycle-energy is the strict **special
case**, not the general certificate. So:

- ✅ **built:** the HodgeRank harmonic residual is a computed instance of the
  incompleteness-certificate construction, on the **cyclic-relabeling axis**
  (`attribution_cycle_energy`).
- 🔬 **open:** the general certificate (all relabeling classes / missing value dimensions) is
  the generalization of it — the road not yet walked, per §0.

Honest phrasing for the frame: *the Hodge residual is one projection of the abstract
certificate onto the cyclic axis — not the certificate.* Do not read §0's cardinal-direction
framing as licensing a "same object / same math" identity claim; §0 is a loop-framing choice,
not a proof that the built carrier already spans the general case.

## 5. The honest boundary — what this frame does NOT buy

The frame must not be oversold; overselling the moat is the exact dishonesty the whole stack
is built against.

- It does **not** crack graph-iso, and does **not** solve general coalitional
  isomorphism-invariance. `ISOMORPHISM-INVARIANCE-VS.md` §2 stands unchanged: the sybil
  action is a **monoid, not a group** (split/merge asymmetric; target is monotone
  quasi-invariance `v(split·S) ≤ v(S)`, not `v(σ·S) = v(S)`), and the content-vs-topology
  boundary is not clean (paraphrase-padding lives in the gap; θ_sim is the content-metric
  companion). The general gate stays 🔬 open. The frame reorganizes the problem; it does not
  dissolve the hard math.

- **The residue is real and now precisely located.** A gaming vector that respects
  relabel-invariance AND dodges the θ_sim content floor AND fires no other probe stays
  invisible — no dispute fires, `v(S)` stays fooled. This is the paper's **correlated-
  blindness** residue (paper §8/§11), the same shape as the CRPC silent-false-consensus
  failure mode: the mechanism kills the *adversarial* version of the problem, not the case
  where the whole network is honestly blind to a vector at once.

- **Two structural mitigations, neither a proof.** (a) **Probe diversity:** the residue
  shrinks as independent probes are added, each blind to different vectors — the
  perspective-diverse-verification pattern applied to `v(S)`. (b) **The consensus floor as
  backstop — CIRCULAR in the PoM direction; see §5.1.** The earlier claim that "a
  `v(S)`-gaming success that fools scoring still cannot buy finalization weight" does **not**
  follow from the code and is retracted. The floor buys a **capital-orthogonality** property
  (a `v(S)`-gamer must ALSO independently command ≥50% of the orthogonal PoS/capital axis),
  which is a real cost multiplier — but it does **not** detect or discount gamed contribution,
  because the PoM dimension the floor consents on IS the (possibly-gamed) `v(S)` output. Status
  discipline: the floor bounds a *capital-only* or *PoM-only* capture; it does not bound
  `v(S)`-gaming on the PoM axis itself.

## 5.1 Calibration results (2026-07-02)

Two compass-calibration targets were read primary-plus-two-independent-cross-checks against
the live code (`node/src/lib.rs`, `node/src/runtime.rs`) and the companion doc. Per §0 these
are calibration checks (is the needle on truth, is the backstop real), **not** completeness
checks. Verdicts are reconciled, not averaged; where the readings disagreed the disagreement
is surfaced as its own finding.

### Target 1 — the compass (§4 "HodgeRank harmonic residual IS the incompleteness certificate")

**Reconciled verdict: 🔬 PARTIALLY MAGNETIZED — true isomorphism on the cyclic sub-problem
only; §4's "same object / same math" identity claim overstated (now fixed above).** The three
readings agreed on substance (partial / sound-with-caveat / partial); the only spread was a
one-notch label difference over how localized the defect is, not over the mechanism. Not a
defeat — the general axis is road-not-yet-walked (§0), and the built carrier is genuinely one
instance of the construction.

- **BECAUSE passes (schema-level isomorphism is real):** both objects are *the residual after
  projecting observed data onto the explainable subspace* — Hodge `norm_sq − proj`
  (`lib.rs:322-323`) vs paper §3 (quoted §4 above).
- **DIRECTION fails (it points one way only):** the Hodge object has a fixed input (net
  cross-identity citation flow, `lib.rs:283-293`) and a fixed single-scalar basis
  (`lib.rs:321`), so it certifies only cyclic topology (energy `= k` for a `k`-cycle,
  `lib.rs:264-265`) — never a missing value axis.
- **REMOVAL fails, and the code proves it against itself:** `value_v8`'s real basis is the
  fixed 4-vector `coalition_features = [breadth, synergy, connected, depth]`
  (`lib.rs:6794-6835`). The I-1 probe FOUND a linear self-flow-laundering **DEPTH-split** that
  pumps `value_v8` by `g ≈ +16.7` (`lib.rs:2899-2934`, pinned RED-as-designed), and the code
  states verbatim it is *"Not a ring either, so cycle-energy / collusion_slash miss it"*
  (`lib.rs:2911-2912`). The A1/A3/A4 vectors (re-parenting, paraphrase, depth×breadth,
  `ISOMORPHISM-INVARIANCE-VS.md` §7) are likewise invisible to the Hodge residual.
- **Self-admission in the companion:** `ISOMORPHISM-INVARIANCE-VS.md` §3 calls the general
  gate *"the generalization of `attribution_cycle_energy` from the cycle class to all
  relabeling classes"* — the general object is NOT the cycle-energy; cycle-energy is the
  special case. §7: *"Enumeration is incomplete: 4 named axes, 4 new attacks."*

### Target 2 — the floor (§5(b) "a v(S)-gaming success ... cannot buy finalization weight")

**Reconciled verdict: ⚠ CRITICAL — CIRCULAR against v(S)-gaming in the PoM direction. The
§5(b) sentence is false as written and has been retracted above.** The finality floor's PoM
dimension consents over the **same quantity** v(S)-gaming inflates, so it cannot be the
backstop against v(S)-gaming that the sentence claimed.

**Disagreement surfaced (not averaged):** primary read **circular**; both cross-checks read
**sound-with-caveat**. The substance was identical across all three — the dispute was only the
*label*. The cross-checks argue "circular" reads as "buys nothing" and over-labels, because the
floor does buy a genuine non-zero property (below). **Reconciliation:** the floor is **circular
in the PoM direction specifically** (it cannot bound PoM-axis gaming — that is the critical
finding), and **non-circular cross-axis** (it forces a gamer to also acquire the orthogonal
capital axis — that is the real, smaller property). Both statements are true; the §5(b)
sentence conflated them and claimed the strong one.

The trace (all real):
1. `Ledger.pom = pom_scores_with_similarity_floor_q16(...)` — the v(S)/attribution output
   (`runtime.rs:524-525`).
2. `Standing.pom` = *"accumulated novelty-value credit"* accrued via `Op::Accrue` =
   *"add newly-finalized novelty value"* — the same v(S) output banked, soulbound
   (`lib.rs:462-463, 471`; *"Consensus reads `Standing.contributor`"* `lib.rs:452`).
3. The finality floor's PoM term is `pom_for/pom_all = Σ v.pom` gated by `dim_ok` requiring
   `pom_for ≥ pom_all · MIN_DIM_BPS/10000` (`MIN_DIM_BPS = 5000`, `runtime.rs:596-602,
   630-632`). So inflating v(S) inflates that contributor's own PoM finality weight — the
   gaming propagates **into** the floored dimension; it is not bounded by it.

**What the floor genuinely DOES buy (non-circular, code-grounded):** the PoS/capital axis is
orthogonal to v(S) (`staked_balance`-backed, *"franchise decay NEVER reduces this"*,
`lib.rs:3563-3566`), and `dim_ok(pos_for, pos_all)` forces capital to independently supply
≥50% (`runtime.rs:596, 632`). This backstops a **PoM-only** capture — PoM's 60% cannot
unilaterally finalize (T11 capital-orthogonality, the property the code comment at
`runtime.rs:559-560` actually claims). A v(S)-gamer must therefore ALSO command ≥50% of real
staked capital. That is a genuine capital-cost multiplier — but it is a **capital-cost floor
on the gamer, NOT a scoring-correctness backstop.**

**🟡 designed-not-built caveat (honest-number discipline):** the `Standing.pom` →
`Validator.pom` franchise wiring that this whole property assumes is **designed, not built**.
`Validator.pom` is an `f64` finality weight (`lib.rs:3548, 3563`) set only in test constructors
(`lib.rs:3739-3740, 5397, 7698, 8391`; `runtime.rs:645`) and the slash-to-zero path
(`lib.rs:3683, 4811`); `Ledger.pom` is a `HashMap<Vec<u8>,u64>` (`runtime.rs:94`). No
production map→weight bridge exists. So the circularity holds under the **intended** design
(which is what §5(b) invokes), and even the cross-axis property it does buy is a 🟡 design
claim pending the franchise-wiring build — not a ✅ deployed guarantee.

**The fix (what would make the backstop non-circular in the PoM direction):** introduce a
genuinely v(S)-independent input to PoM finality weight, so the finality PoM term is NOT a
pure pass-through of raw `pom_scores`. Concrete candidate: a **dispute/challenge-window
discount on freshly-accrued standing** before it counts toward finality weight (the
`Op::Slash` / dispute path already exists, `lib.rs:474-475, 489`). Until such a separation is
wired, the only defenses that actually reduce v(S)-gaming are the **scoring-side** ones (probe
diversity §6.2; the realized-downstream-value moat, `ISOMORPHISM-INVARIANCE-VS.md` §7:241-246;
the cand-B legit-regression guard §6.4) — the floor prices the attack in capital, it does not
filter the gaming.

## 6. Next grains (measurement-first, honest priority)

1. 🔬 **Formalize the loop invariant.** State the completion procedure as: probe set `P`
   generates residuals `{r_i}`; a non-zero `r_i` on a maneuver with honest baseline zero is
   an incompleteness certificate; a patch `Δv` is admissible iff it drives `r_i → 0` AND the
   cand-B legit-regression set is preserved. Pin the admissibility condition as a test
   (RED-as-designed on a patch that fixes the probe but breaks a legit case).
2. 🔬 **Probe-diversity metric.** Define coverage of the probe set over known gaming vectors
   and surface the residue explicitly (which vectors no current probe catches). Silent
   coverage gaps read as "solved" when they are not — log them.
3. 🟡 **Wire the floor as the named backstop — corrected scope (per §5.1).** Document, in the
   consensus spec, that the anti-concentration floor buys **capital-orthogonality** (a
   PoM-only capture cannot finalize; a v(S)-gamer must also command ≥50% of the orthogonal
   PoS axis), NOT a bound on v(S)-gaming blast-radius into finality. The earlier "scoring
   residue is bounded in blast radius by the floor" wording is retracted — the floor's PoM
   half is downstream of the same possibly-gamed v(S) (`runtime.rs:524, 630`). Also note the
   `Standing.pom → Validator.pom` wiring is 🟡 designed-not-built.
4. Build cand-B first (cheap, ROADMAP-flagged): the legit-regression guard is the preserve-
   half and gates every future completion; it should exist before the loop runs.

## 7. One-line statement

`v(S)` is not a frozen function to be proven un-gameable (impossible: the god trick). It is
a **completion procedure**: gaming attempts are the sensor, the HodgeRank harmonic residual
is one built instance of the incompleteness certificate (the cyclic axis; the general
certificate is 🔬 open, §5.1), the legit-regression guard keeps completion honest, and the
anti-concentration consensus floor bounds a *capital-only or PoM-only* capture — **not**
v(S)-gaming on the PoM axis, which is downstream of the same score (⚠ §5.1). The moat is the
scoring-side loop; consensus floors cross-axis capture, it does not filter gaming.

## 8. THE single next build grain (honest priority, 2026-07-02)

The findings, not the roadmap default, decide this. Two grains compete:

- **Fixing the compass** is a doc + probe-diversity job (§6.1/§6.2) plus the eventual second
  residual instance — real, but the compass is already correctly located as 🔬 road-not-walked
  and is *self-correcting* (the I-1 probe found its own gap).
- **The floor is a ⚠ CRITICAL live mis-statement** about the safety-adjacent finality layer:
  the frame claimed a v(S)-gaming backstop that the code does not provide, and §6-grain-3 was
  about to hard-wire that false claim into the consensus spec. That is the higher-leverage
  correction (full-leverage: it prevents a false property being cited downstream).

**Next grain: build cand-B (the legit-regression guard) FIRST — unchanged from ROADMAP, and
the findings confirm it.** Rationale: §5.1 shows the floor does NOT backstop v(S)-gaming, so
the *only* real defenses are scoring-side, and cand-B is the preserve-half that gates every
one of them — no basis-completion (closing I-1/A1/A4, or the I-2 relabel-variant-energy
subtraction, `ISOMORPHISM-INVARIANCE-VS.md` §5) is admissible without it. Concretely: pin a
KNOWN-LEGIT contribution set as a regression fixture asserting `value_v8` keeps it above
threshold, so any future patch that closes a gaming gap is admissible **iff** it drives the
probe residual → 0 **and** leaves the fixture green. It is cheap, measurement-first,
deploy-independent, touches no consensus path, and is the prerequisite for the loop to run at
all. The floor fix (a v(S)-independent PoM-finality input, §5.1) is CRITICAL but
consensus-affecting ⇒ build COLD, Will-gated, after the finality decision — not autonomously.
