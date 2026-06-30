# Isomorphism-invariance gate for `v(S)` — design pass (OPEN RESEARCH)

> ROADMAP cand-A / CONTINUE.md priority #2. This is a **design pass, not a build**:
> the strongest known hardening of the contribution measure, and an open research
> problem for coalitional measures. The honest contribution of this doc is (a) to
> state the invariant precisely, (b) to show that several already-built defenses are
> point-instances of it, and (c) to specify the smallest measurement-first grain that
> can be built next without claiming the general problem is solved.
>
> Scope note: this is a `v(S)` / gameability surface. It is **not** finality — it
> changes how value/standing is *scored*, never the safety path. Independent of the
> open PoM↔finality decision.

## 1. The threat, stated as a symmetry

Every known `v(S)` gaming vector this repo has closed has the same shape: the attacker
**relabels** the contribution graph to manufacture score without adding value.

- **Sybil identity split** (closed by cross-identity μ^m damping, ROADMAP (r)): take one
  body of work under identity `A`, re-attribute it across `K` fresh identities
  `A₁…A_K`. The *content and topology are unchanged*; only the identity labels on the
  soulbound `type_script.args` changed.
- **Volume padding** (closed by within-identity λ^r, (q)): the same identity re-posts
  `M` children of one parent — a relabeling of one contribution into `M` near-copies.
- **Citation ring** (detected by `attribution_cycle_energy` / `attribution_circulation`,
  (aa)/(bb)/(dd)): `K` identities cross-cite each other's roots — a relabeling that
  manufactures a *cyclic* provenance topology no honest "builds-upon" order produces.

The common structure: an honest contribution's value is a property of **what was
contributed and how it composes** — its position in the provenance DAG up to
isomorphism — **not** of which identity-labels sit on the nodes. So:

> **Invariant (target).** Let `σ` be a structure-preserving relabeling of the soulbound
> identities of a contribution set `S` (a permutation of `type_script.args` that
> preserves the content of every cell and the parent-edge topology). Then a sound
> contribution measure should satisfy `v(σ·S) = v(S)`. A **gaming maneuver is a
> relabeling-class action that changes `v`** — and the magnitude `|v(σ·S) − v(S)|` is
> the *gaming energy* the gate must cap.

A gaming vector breaks invariance; honest work doesn't. That is the whole gate in one
line — and it is exactly the controlled result from the loop-engineering literature
sweep (Helff et al., 2604.15149: proxy verifiers are gamed, the demonstrated fix is
**invariance-based verification**). This doc applies that frame to a *coalitional*
measure, which is where it stops being a clean port.

## 2. Why this is a monoid, not a group (the honest hard part)

Strict isomorphism-invariance assumes a **group** action: relabelings are permutations,
invertible, and `v` is constant on each orbit. Two of our three vectors are **not**
group actions:

- **Split / merge is a monoid.** Splitting `A → A₁…A_K` has no inverse the attacker is
  forced to take; merging `A₁…A_K → A` is a *different* map. The relevant algebraic
  object is the **sybil monoid** of split/merge maps on the identity set, and the
  property we actually want is **monotone quasi-invariance**: splitting must never
  *increase* `v` (`v(split·S) ≤ v(S)`), and the honest baseline (no split) is the
  supremum. This is strictly weaker than `v(σ·S) = v(S)` and is the right target —
  μ^m damping already realizes it for the cross-identity axis (it makes K split
  identities *saturate at* the single-identity bound, not exceed it).
- **Content-vs-topology boundary is not clean.** `v` depends on coverage (content), not
  only on graph shape. A relabeling that preserves content AND topology is a genuine
  symmetry; one that preserves topology but *changes content* is not — and the attacker
  lives in the gap (paraphrase-padding changes content slightly to dodge an
  exact-content invariant, which is why the near-dup similarity floor θ_sim, loop (xx),
  is the *content-metric* companion to the *topology* invariant). The gate is therefore
  **two-sided**: invariance under identity-relabeling (topology) AND a content metric
  that quotients near-duplicates (the θ_sim floor). Neither alone suffices.

Stating these honestly is the point: for general coalitional/Shapley measures,
isomorphism-invariance is **unsolved** (graph-iso has no known practical canonical form
for our weighted, content-bearing DAGs; the sybil monoid's merge direction is
adversary-unconstrained). We do **not** claim a general proof. We claim a *frame* that
unifies the point-defenses and a *measurable* sub-invariant we can harden toward.

## 3. HodgeRank is the natural carrier of the invariant we can compute

The built `attribution_cycle_energy` (§ lib.rs) is already a **relabel-invariant
functional** in the exact sense we want, for the cyclic sub-problem:

- It is defined on the **net cross-identity flow graph** via the graph Laplacian. A
  permutation `σ` of identities conjugates the Laplacian `L → PᵀLP` (a symmetric
  reindexing); the divergence `b` permutes the same way; so the harmonic energy
  `‖Y‖² − b·s` is **invariant under `σ`** — it is a function of the *graph up to
  isomorphism*, not of the labels. This is a property, not a coincidence: the
  Helmholtz–Hodge decomposition splits the flow into
  - a **gradient** part `grad s` = the honest global "builds-upon" potential — the
    component that *is* the isomorphism-meaningful ranking (invariant, and zero-energy:
    honest work sits here), and
  - a **harmonic residual** = the divergence-free cyclic component — precisely the
    subspace a relabeling-attack (citation ring) injects energy into.

So the harmonic residual **is** the gaming-energy of the cyclic relabeling class, and it
is already computed, replica-deterministically, with no real-label data. The
isomorphism-invariance gate is the **generalization of `attribution_cycle_energy` from
the cycle class to all relabeling classes**: cycles are the harmonic subspace; sybil
splits are the monoid-quasi-invariance μ^m already enforces; volume is λ^r. The gate
names the union.

## 4. The unifying view — the point-defenses are projections of one invariant

| Relabeling class | Invariant required | Built mechanism | Status |
|---|---|---|---|
| Identity permutation (pure rename) | `v(σ·S) = v(S)` | inherent — `v` keys on consensus identity, never label order | ✅ (by construction) |
| Sybil split (1→K identities) | `v(split·S) ≤ v(S)` (monotone quasi-inv.) | cross-identity μ^m damping | ✅ built (r), saturates |
| Volume pad (1→M children) | `v(pad·S) ≤ v(S)` | within-identity λ^r damping | ✅ built (q) |
| Hybrid split×pad (diagonal) | joint bound, not product of tails | single joint ρ^j geometric decay | ✅ built (u) |
| Cyclic re-attribution (citation ring) | harmonic energy = 0 for honest | `attribution_cycle_energy` + circulation → `collusion_slash` | ✅ built (aa/bb/dd) |
| Near-duplicate content relabel | content-metric quotient | θ_sim similarity floor | ✅ built (xx) |
| **Linear self-flow laundering (DEPTH split)** | **`v(σ·S) ≤ v(S)` for a vertical-edge relabel** | **— breadth dampers blind (μ⁰=1, one child/parent)** | **🔬 open — measured by I-1, see below** |
| **General structure-preserving relabel** | **`v(σ·S) = v(S)` over the full orbit** | **— (this gate)** | **🔬 open** |

The contribution of the frame: each built defense is a *projection* of one invariant
onto one relabeling axis. The open gate is the **completeness critic** — a single check
that asks "is there *any* structure-preserving relabeling that moves `v`?" rather than
enumerating known axes. Its value is catching the *next* vector before it is named (the
axes above were each found and patched reactively; an invariance probe is the proactive
form).

## 5. Smallest buildable grain (measurement-first, teed fresh)

Do **not** attempt the general gate first. The repo's pattern is measure-the-gap, pin it
RED-as-designed, then close (the T3 matrix, the (aa) ring probe). The same here:

**Grain I-1 — relabel-invariance PROBE (a test harness, not a consensus gate).**
1. Fix an honest reference contribution set `S` (a small provenance DAG with real
   coverage, the kind already used in the `value_v*` tests).
2. Generate a canonical family of **structure-preserving relabelings** `{σ}`: identity
   permutations (must be exactly invariant) and sybil splits (must be ≤ baseline).
3. Compute `g(σ) = v(σ·S) − v(S)` for each, over the live `value_v8` path.
4. **Assert** the invariance contract per class: permutations ⇒ `g = 0` exactly;
   splits ⇒ `g ≤ 0` (monotone). Any class that violates is a *named, measured*
   invariance gap, pinned RED-as-designed like (aa) — it becomes the next close.
5. Anti-theater: a deliberately label-sensitive `v` (e.g. credit by label order) makes
   the permutation assertion go RED; the real `v` keeps it green.

This is deploy-independent, touches no consensus path, needs no real-label data, and
turns "isomorphism-invariance" from a slogan into a number the suite tracks. It is the
honest first step; the general orbit-search gate (and its cost — checking invariance
over a relabeling orbit is potentially graph-iso-hard) stays explicitly open.

**Grain I-2 (later, harder) — fold the harmonic residual into the score, not just the
slash.** Today `attribution_cycle_energy` feeds a *slash* (`collusion_slash`). The
invariance gate's stronger form *subtracts the relabel-variant energy from `v` at
scoring time*, so a ring earns less rather than being caught-then-punished. This is
consensus-affecting (it changes earned standing, which drives the franchise) ⇒ build
cold, with the finality decision resolved first (it interacts with how much PoM weight a
contribution carries). Flagged, not scheduled.

## 6. Honest status line

| Item | Status |
|---|---|
| Invariant stated for the coalitional setting (monoid quasi-invariance + content metric) | ✅ this doc |
| Point-defenses unified as projections of the invariant | ✅ this doc (§4) |
| HodgeRank residual shown to be a relabel-invariant functional | ✅ argued (§3); the energy is built |
| General isomorphism-invariance gate for `v(S)` | 🔬 **open research** — graph-iso-hard, merge-monoid unconstrained, content/topology boundary |
| Relabel-invariance probe (grain I-1) | ✅ **built** — `node/src/lib.rs` `value::tests::relabel_invariance_*` (3 tests: exact-permutation, sybil-split, anti-theater teeth) |
| Linear self-flow-laundering vector (DEPTH split) | 🔬 **open — FOUND by I-1 on first run.** `g = +16.7` measured (split pumps `value_v8`); pinned RED-as-designed |

### 6.1 What the I-1 probe found (2026-06-29)

The probe did its job on the first run: it surfaced a **new, un-named invariance gap** the
reactive point-defenses do not cover.

- **Permutation invariance — exact.** A consistent relabeling of identities (cells + standing)
  leaves the set total `value_v8` **bit-identical** (`g = 0.0`). Identity enters `value_v8` only
  through the standing-floor lookup, so a label bijection is a true symmetry. ✅
- **Anti-theater — has teeth.** The same σ that is exact for the real `v` is detected as
  *variant* by a deliberately label-sensitive `v` (credit-by-label-byte). The `g = 0` assertion
  is a real test, not vacuously green. ✅
- **Sybil split — VIOLATED (`g = +16.7`).** Peeling a self-built linear lineage's child onto a
  fresh identity **increases** the set's earned value. Mechanism: downstream flow counts only
  **cross-identity** edges (`flow::children_of_external`, the un-spoofable-by-content self-pump
  defense). A vertical edge `id 1 → id 1` is internal (pays nothing); relabeling the child to a
  fresh identity makes the *same edge* external, so intra-mind self-flow is **laundered** into
  apparently-external use.

**Why the built dampers miss it.** λ^r / μ^m / ρ^j all damp a *parent's children* (breadth /
fan-out). This is a **depth-axis** relabel: one child per parent ⇒ `μ⁰ = 1`, the cross-identity
damper never engages; it is not a ring ⇒ cycle-energy / `collusion_slash` miss it. The only live
barrier is the per-identity standing **FLOOR (MIN_STAKE)** cost — which a sufficiently valuable
self-chain can out-earn.

**Severity (calibrated).** This is on the **`value_v8` moat-target path**, the value function the
franchise is being built *toward*. The **currently-deployed runtime** franchise is `pom_scores`
(temporal_novelty + θ_sim), which is **flow-free** and therefore **not exposed today**. It is a
forward-design gap to **close before `value_v8` drives the franchise**, not a live exploit on the
current node. **Verified, not asserted (I-1b):**
`value::tests::relabel_invariance_deployed_pom_scores_path_is_split_immune` runs the *same* split
through `pom_scores_with_similarity_floor_q16` (the live runtime path) and confirms total standing
is **exactly invariant** — per-cell temporal-novelty is identity-independent and summed by
identity, with no per-identity damping to escape. The test is **regression-guarded**: if a future
change adds split-escapable damping to the runtime path, it goes RED and surfaces the now-live
exposure.

**Close path.** (a) Price the split via the standing cost (make `N` sybils cost `> ` laundered
flow — economic); or (b) **I-2**: subtract the relabel-variant flow energy at scoring time so the
*same edge* earns the same whether it is labeled internal or external — the structural close. I-2
is consensus-affecting (changes earned standing) ⇒ build cold.

We claim invariance for the *demonstrated* relabeling axes (the built table in §4), and
a *frame + measurable probe* for the rest — not a finished proof. That is the same
honesty line the whitepaper and SECURITY.md hold: demonstrated is not rounded up to
solved.

## 7. Adversarial loop (2026-06-29) — 4 independent attacks, and the simplest close

Method (Tom Lindeman's framing): Claude proposes the answer; *independent* adversaries
attack it ("why is this wrong against an intentional adversary?"); then synthesize
solutions. Run here with 4 in-repo adversary agents (stronger than an external model:
they read the actual `value_v8`/`flow`/`coverage` code). The loop found **4 new vectors
beyond I-1 in one pass** — which is itself the load-bearing result.

**The attacks:**
- **A1 — re-parenting fabrication (value_v8).** A pure re-parenting σ (change only
  `Cell.parent`, content + identity unchanged) raises every cell's outcome seed:
  `coalition_features` trusts the attacker-supplied parent pointer as proof of
  derivation. `connected`/`depth` rise → the trained outcome model (built to reward
  lineage structure) pays more for fabricated topology. Single-identity, so every
  cyclic/flow defense (cross-identity-keyed) is blind. Fix proposed: an edge counts only
  if `coverage(child) ∩ coverage(parent) ≠ ∅` — derivation witnessed by shared content,
  not asserted by a pointer.
- **A2 — the economic FLOOR barrier is illusory.** The I-1 "per-identity standing FLOOR
  cost" is **not** a posted/burned stake — it is *earned soulbound PoM* (`Op::Accrue`),
  which a valuable chain **self-funds by accruing**. Real attacker outlay ≈ 0 for exactly
  the chains worth laundering; the gain is per-edge with depth cancelling. **This kills
  the proposed economic close (solution 3).**
- **A3 — content reshingle/paraphrase multiplication (DEPLOYED path).** `coverage()` is
  order-exact 4-byte FNV shingling; the θ_sim floor (`temporal_novelty_with_similarity_
  floor_q16`, lib.rs:6296) floors only on `overlap/len > θ` (high overlap). A reshuffle/
  paraphrase has **low** overlap → not floored → banks full novelty → K× standing for one
  body of work, on `pom_scores_with_similarity_floor_q16` (the live path). **Honest
  calibration: this is not a new bug — it is the *known* gap the learned-v(S) moat exists
  to close (byte-proxy ≠ semantic). A new floor/canonicalization layer would not catch
  synonym-paraphrase anyway.**
- **A4 — depth×breadth composition (value_v8).** Chain the I-1 depth split with a
  one-child-per-parent breadth so the ρ^j/μ^m/λ^r breadth dampers never engage (rank 0),
  while the flow recursion's own `d^depth` tail compounds the laundered flow. Linear sybil
  cost, geometric gain.

**The meta-lesson (the real answer).** Enumeration is incomplete: 4 named axes, 4 new
attacks. An intentional adversary attacks the axis you did not enumerate. Therefore the
only *complete* close is not another per-axis damper — it is the **one structural signal
no relabeling can fake: realized downstream value (the moat).** A reshuffle, a
re-parenting, a self-flow-launder — none of them make *another mind actually build on the
work*. That single signal dissolves the whole class.

**Design constraint (Will, 2026-06-29): simplest is best — we are already one layer into
complexity; do NOT add another.** This *rules out* the agents' convergent ρ^j-on-depth
proposal (a new damper layer) and any new content-floor layer. The simplest close is
*fewer* moving parts: lean on the realized-value moat, stop shipping axis-detectors.

**Rosetta convergence (Will, 2026-06-29).** The Rosetta cross-domain semantic compiler is
the natural close to the **content axis (A3)**: canonicalize content to a semantic normal
form, *then* shingle, so reshuffle/paraphrase of the same meaning collapses to the same
coverage and is floored as a duplicate. This is **reuse of an existing primitive, not a
new layer** — fitting the simplicity constraint. Caveat: consensus needs replica-identical
canonicalization, and an LLM-Rosetta is non-deterministic ⇒ it lands in the **learned-v(S)
/ training-signal layer** (or a distilled deterministic normal form), not directly
on-chain. Shape is correct: Rosetta canonicalizes meaning, PoM prices it — two of Will's
primitives solving one bottleneck from two sides.

**Scope discipline (calibrated).** A1 + A4 are **value_v8-only** (the moat-target path, not
deployed). A3 hits the **deployed** `pom_scores` surface but is the *known* moat gap, not a
surprise. The deployed path stays **identity-relabel-immune** (I-1b verified,
flow-free/coalition-free). The converged structural close (de-identify the value signal /
the moat) is **consensus-affecting ⇒ build COLD in fresh low-context, Will-gated** — not
shipped autonomously.
