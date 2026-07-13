---
title: "Contribution Provenance and Attribution"
subtitle: "Noesis patent family, Application 4 — DRAFT v1 for Rodney's-AI review"
date: "2 July 2026"
status: "INTERNAL DRAFT — not for filing. Sibling to the Proof-of-Contribution priority application."
---

# CONTRIBUTION PROVENANCE AND ATTRIBUTION

> **Family position.** This is Application 4 in the family mapped in the priority filing
> (`10_Patent_Family_Map.md`, candidate family #4, "Contribution Provenance and Attribution").
> The priority application (Proof of Contribution) claims the architectural invariant and the
> three implementation families (separation, valuation, finalisation) and *seeds* this
> application in a single cross-cutting dependent claim, priority claim 12: dividing a
> multi-contributor contribution's value by a graph-restricted cooperative-game value with
> value propagating backward along provenance edges under a damping factor strictly less than
> one. This application develops that seed into a standalone invention with two centres of
> gravity: (a) the graph-restricted cooperative-game division with damped backward propagation
> over provenance; and (b) per-identity collusion attribution keyed on a soulbound identity
> under a slash budget bounded by the manufactured value.
>
> **Organising principle (inherited, held as interchangeable terminology):** consensus
> authority and the value that produces it are protocol-managed state earned through
> contribution, independent of transferable economic ownership. The soulbound contributor
> identity, not a transferable holder, is the key on which value, provenance, and penalty all
> resolve; the claims capture that invariant.

## FIELD

Distributed-ledger contribution accounting; specifically, dividing the value of a
multi-contributor contribution among its contributors along a provenance graph, propagating
that value backward under damping, and attributing collusion to individual contributor
identities under a bounded slash budget.

## BACKGROUND

A system that mints consensus authority from measured contribution must answer a question that
a single-author reward scheme never faces: when a contribution has several contributors and is
built on prior contributions, who is credited, by how much, and what happens to that credit
when a later verdict finds the contribution — or a contribution beneath it — to have been
fabricated or colluded.

Three problems are specific to multi-contributor, provenance-linked contribution and are not
solved by prior art:

1. **Additive credit is gameable and ignores structure.** Splitting a contribution's value by
   a naive additive rule (equal shares, or shares proportional to raw output) rewards padding
   and duplication, and pools value across contributors who are not actually connected by
   provenance. A contributor can inflate its share by recombining or duplicating earlier
   content, or by asserting a coalition with contributors whose work it did not build on. A
   principled division must charge each contributor its marginal, synergy-aware contribution
   and must refuse to pool value across provenance-disconnected parties.

2. **Unbounded or unattributed backward propagation.** Value should flow backward to the
   contributions a later contribution was built on, so that pivotal upstream work is rewarded
   for what is built on it. But if that backward flow is undamped, a ring of self-attributing
   contributions can pump its own credit without bound; and if the flow is not keyed on a
   non-transferable identity, an attacker can certify the usefulness of its own work by
   building on it itself. Backward propagation must be a strict contraction and must run over
   the soulbound identity, not a transferable holder.

3. **Refutation and collusion penalties that double-count or escape attribution.** When a
   contribution is refuted, the unvested value it minted must be cancelled and the parties who
   certified it slashed; when a collusion ring is detected, each ring member must be named and
   charged its causal share. But a ring that manufactured the very value a refutation later
   cancels has committed one harm, not two; penalising it on both paths independently
   double-slashes it. Conversely, a detector that returns only a scalar ("collusion exists")
   names no one and cannot drive a per-identity slash. The system must attribute both harms to
   individual identities, compose them without double-counting on their overlap, and bound the
   total by the value actually manufactured.

Prior contribution-weighted consensus systems (for example Bittensor-style measured-value
weighting) do not address a graph-restricted, synergy-aware division with damped backward
propagation and per-identity collusion attribution under a manufactured-value bound. The
present invention does.

## SUMMARY

The value of a contribution having several contributors is divided among them by a
**graph-restricted cooperative-game (Myerson) value** over a submodular coverage game: only
provenance-connected sub-coalitions create value, so disconnected contributors cannot pool
credit, and a contributor that duplicates or recombines earlier content earns a low marginal
share. The same cooperative machinery runs at two levels — outcome to contributions, and
contribution to intra-contribution contributors.

Value **propagates backward along provenance edges** by a damped fixed-point iteration:
`flow(b) = own(b) + d · Σ flow(children of b)`, with damping `d < 1`. The damping factor is a
constitutional decay constant `ρ = 1/φ`, the reciprocal of the golden ratio. Because `d < 1`
the iteration is a contraction, so it converges and no self-referential ring can pump its own
flow without bound. Backward flow may be restricted to **external** edges — edges between
distinct soulbound identities — so a contributor cannot certify the usefulness of its own work.

Provenance is **preserved for enforcement**: the soulbound identities on a refuted target's
provenance lineage are recovered by walking the parent chain, keyed on the consensus-derived
soulbound identity and never on producer assertion. A refutation settlement cancels the
target's unvested value and slashes each certifier by a bounded share.

Collusion is **attributed per identity**: a topological detector returns, for each colluding
identity, its causal share (WHO and HOW-MUCH), summing an orthogonal directed (cyclic-residual)
component and a mutual (balanced) component, keyed on the soulbound identity. A cross-path slash
merges the collusion and refutation penalties so that an identity caught by both paths for the
**same** manufactured value is penalised once — the two are collapsed to their maximum on the
overlap and summed only when disjoint — and every identity's total is capped at its current
standing, so the aggregate slash never exceeds the value manufactured.

## DETAILED DESCRIPTION

All source pointers are to the public reference node (`WGlynn/noesis`, Rust on a RISC-V VM,
Cell state model). The mechanisms below are implemented and tested unless a status marker says
otherwise. Status discipline: ✅ built, 🟡 designed, 🔬 open — never rounded up.

### Graph-restricted cooperative-game division (✅ built)

The value of a contribution with several contributors is divided by a Myerson restricted game
over a submodular coverage value. The coalition value `v(S) = |union of coverage(cells in S)|`
is submodular, so a redundant contribution adds little (`node/src/lib.rs:2950`). The Myerson
restricted game `v^g(S)` sums `v` only over the **connected components** of `S` under parent
(provenance) edges, so disconnected coalitions cannot pool value (`:2951`, components at
`:2988`, restricted value at `:3007-3013`). Exact Shapley is exponential; the value is estimated
by deterministic Data-Shapley permutation sampling with a seeded PRNG so replicas converge
bit-for-bit (`:2953-2954`, `sampled_value(cells, samples, restricted)`, `:3017`). With
`restricted = true` the estimator is the Myerson graph game; with `restricted = false` it is the
plain submodular Shapley (`:3015-3016`). The cooperative machinery is load-bearing, not
cosmetic: the synergy Shapley provably diverges from an additive win-share
(`synergy_shapley_differs_from_additive_copeland`, `:3101`), a redundant contribution receives a
low Shapley marginal (`redundant_cell_gets_low_shapley_marginal`, `:3115`), and provenance edges
change the credit (`myerson_restricts_value_to_provenance`, `:3124`).

The same machinery runs one level down, splitting a single contribution's value among its
intra-contribution contributors: a closed-form two-player Shapley for the operator/model case
(`:3312`) and an N-contributor generalisation over the same submodular coverage game
(`recurse_shares`, `:3329-3333`). The economy is therefore two-level recursive: outcome →
contributions → contributors (`:3164-3168`).

### Damped backward propagation over provenance (✅ built)

A contribution earns credit not only for its own value but for the value of what is built on
it. Credit propagates backward along parent edges by a damped Jacobi fixed point
`flow(b) = own(b) + d · Σ_{c built on b} flow(c)` with `d < 1`
(`node/src/lib.rs:3158-3160`, `value_flow_with_own`, `:3223`; `value_flow`, `:3293`). Because
`d < 1` the map is a contraction: it converges, and a ring of self-attributing cells cannot
pump its own flow unboundedly — the circularity guard is made mechanical by the damping
(`:3160-3162`, `:3219-3220`).

The damping factor is a constitutional decay constant `RHO = 0.6180339887498949`, the
reciprocal of the golden ratio `1/φ` (`node/src/lib.rs:3255`). The consensus-path fixed-point
computation mirrors the same constant in Q32.32 fixed-point integer arithmetic as
`RHO_Q32 = round(2^32 / φ) = 2_654_435_769` (`node/src/lib.rs:7893`), so the floating-point and
on-VM integer sides agree within a drift-guard band. A single joint geometric tail (`ρ^j` over a
canonical flatten of a parent's external children) is applied rather than a per-axis product, so
the hybrid diagonal draws from the same geometric budget as a single axis (`:3250-3255`,
`:7886-7893`).

Backward flow may be restricted to **external** edges — a child whose soulbound identity
(`type_script.args`) equals its parent's is dropped — so a mind cannot certify the usefulness of
its own work by building on it itself (`children_of_external`, `node/src/lib.rs:3199-3216`;
`external_only` parameter, `:3220-3222`). The realised-flow gate seeds `own` with the
strategyproof floored novelty, so redundant children pump nothing (`:3221-3222`).

### Provenance preservation for enforcement (✅ built)

The soulbound identities on a refuted target's provenance lineage are recovered by
`refuted_lineage_identities(cells, target)`: the target cell plus every parent up its chain
present in the cell set, cycle-guarded (`node/src/lib.rs:4436-4445`). The identities are the
`type_script.args` soulbound identity, keyed on finalized cells and never producer-asserted, and
the walk reuses the same parent-chain connectivity that the Myerson value (`lineage_coalition`)
and the collusion detector already traverse — no new oracle (`:4427-4435`).

A closed dispute is settled by `resolve_refuted(...)` (`node/src/lib.rs:4221`): it cancels the
target's then-unvested value, slashes each certifier `λ × bounded_share + α`, returns bond plus
a `β`-bounty to the challenger, and burns the remainder to keep the mint↔sink balance
(`:4210-4211`, `:4227-4252`). The exposure set snapshots at the challenge's opened epoch so slow
resolution cannot vest value out from under a live dispute (`:4214-4216`); zero-share certifiers
are skipped, so a vested contributor whose edge minted nothing is never taxed (`:4217-4218`); and
`β` is clamped to `[0,1]` so the resolver can never become a mint (`:4219`). A guarded variant,
`resolve_refuted_guarded(...)`, judges each certifier on its own standing per-certifier, so a
mixed panel spares an honest certifier while the garbage certifier's slash lands
(`:4275-4290`, per-certifier rationale `:4263-4267`).

### Per-identity collusion attribution (✅ built)

`collusion_residual_by_identity(cells)` returns, for each colluding soulbound identity, its
causal share — the WHO and HOW-MUCH a bounded collusion slash spends (`node/src/lib.rs:361-379`).
For each unordered identity pair it sums two orthogonal components and attributes the total to
both incident identities (`:364-371`):

- a **directed (cyclic)** component: the per-edge Helmholtz–Hodge harmonic residual
  `|y − (s_i − s_j)|`, nonzero only on net-circulating directed-cycle edges; an honest acyclic
  gradient edge fits exactly and contributes zero (`:366-369`);
- a **mutual (balanced)** component `min(flow[i→j], flow[j→i])`, catching the balanced ring the
  residual is blind to (`:370-371`).

Honest provenance — acyclic, diverse certification — attributes zero to every identity, so there
is no false slash; a directed or mutual k-ring attributes equal nonzero shares to its members
(`:373-374`). The share is the causal weight a bounded topological-collusion slash spends
(`Σ ≤ manufactured value`), keyed on the consensus-derived soulbound identity (`:374-376`).

### Cross-path composition without double-slashing (target/share ✅ built; settlement wiring 🟡 designed)

Refutation and collusion measure orthogonal harms, and a ring member whose cells sit in the
refuted target's lineage manufactured the value the refutation then cancels — the same harm on
two paths (`node/src/lib.rs:4427-4435`, `:4399`). `unified_slash(collusion, refutation, overlap,
standing)` merges the two per-identity slash maps (`:4486-4515`):

- an identity in the overlap — on the refuted target's lineage per `refuted_lineage_identities` —
  is charged `max(collusion_i, refutation_i)`: one harm, one penalty (`:4466-4468`, `:4507`);
- a disjoint identity — a ring member who also certified an unrelated refuted target — is charged
  `collusion_i + refutation_i`: two distinct harms, two penalties (`:4469-4470`, `:4507`);
- every identity's total is then capped at its current standing, so a cross-path slash can never
  destroy more than the identity holds (`:4471-4472`, `:4508-4509`).

Because the overlap collapses to a maximum, the standing actually destroyed equals the sum of
this function's output, which is at most `collusion.burned + Σ refutation.slashes` and strictly
less whenever an overlap identity is double-listed (`:4474-4482`). Tests fix the behaviour:
`unified_slash_overlap_takes_max_disjoint_sums_spares_honest` (`:4984`),
`unified_slash_caps_total_at_standing` (`:5017`), and
`unified_slash_zero_standing_yields_no_slash` (`:5054`).

**Honest status.** The target/share computation (`collusion_residual_by_identity`,
`refuted_lineage_identities`, `resolve_refuted`) and the pure cross-path merge (`unified_slash`,
`unified_settlement`) are ✅ built and unit-tested. Wiring the merged settlement into the live
dispute-settlement application path — composing with the existing refutation slash in production
so no identity is double-slashed end to end — is the deploy-coupled step and is 🟡 designed, not
yet built (`:376-378`, ROADMAP (dd)/(kk); `unified_settlement` doc `:4517-4531`). Not rounded up.

### Inventive step (synergy, not aggregation)

The mechanisms are interdependent, and the safety property is destroyed by removing any one of
them. The graph-restricted division refuses to pool value across provenance-disconnected
contributors, but that restriction is only meaningful because backward flow runs over the
soulbound external identity, not a transferable holder: if flow ran over ownership, a single
party holding many addresses would appear as a connected, diverse coalition and the Myerson
restriction would protect nothing. The sub-unity geometric damping (a
tunable rate 0 < ρ < 1, default 1/φ; the inventive property is the contraction, not the specific
value) is what makes backward propagation a contraction, without which a self-attributing ring
pumps unbounded credit and the collusion detector faces an ill-posed, non-convergent flow field. The per-identity collusion attribution
is only actionable because it is keyed on the same soulbound identity the division and the
provenance walk use, so the ring named by the detector is the ring whose standing is slashed and
whose lineage the refutation cancels — which is exactly what lets the cross-path merge recognise
the overlap and collapse it to a single penalty. Remove the soulbound key, or the damping, or
the graph restriction, and the attribution either double-counts, fails to converge, or names the
wrong party. That interdependence is the signature of a synergistic combination, the recognised
lever for inventive step, and the opposite of an obvious aggregation of known parts.

## CLAIMS

**1.** A computer-implemented method of dividing a value of a contribution having a plurality of
contributors on a distributed ledger, comprising: representing contributions as units of ledger
state linked by provenance edges, each unit carrying a non-transferable contributor identifier;
defining a coalition value that is submodular in the units of a coalition, such that a unit
adding coverage already provided by the coalition increases the coalition value by less than a
unit adding new coverage; and dividing the value among the contributors by a graph-restricted
marginal-contribution value in which the coalition value is credited only over sub-coalitions
whose units are connected to one another under the provenance edges, such that contributors not
connected by provenance cannot pool value and a contribution that duplicates or recombines
earlier content receives a low marginal share.

**2.** The method of claim 1, wherein the graph-restricted marginal-contribution value is
estimated by permutation sampling driven by a deterministic pseudo-random generator, such that
the estimate is identical across replicas.

**2A.** The method of claim 1, wherein the coalition value is equal to a cardinality of a union
of coverage of the units in the coalition.

**2B.** The method of claim 1, wherein the graph-restricted marginal-contribution value is a
graph-restricted cooperative-game value in which the coalition value of a coalition is summed
over connected components of the coalition under the provenance edges, and the value credited to
a contributor is a Shapley value of the resulting restricted game.

**3.** The method of claim 1, further comprising propagating value backward along the provenance
edges by a damped fixed-point iteration in which a value flow of a unit equals an own value of
the unit plus a damping factor times a sum of value flows of units built on the unit, the
damping factor being strictly less than one so that the iteration is a contraction and no ring
of units can increase its own value flow without bound.

**4.** The method of claim 3, wherein the damping factor is a constitutional constant equal to a
reciprocal of a golden ratio, and the fixed-point iteration is evaluated in deterministic
fixed-point integer arithmetic on the ledger virtual machine.

**5.** The method of claim 3, wherein the backward propagation counts only edges between units
whose non-transferable contributor identifiers differ, such that a contributor cannot increase
its value flow by building further units on its own units.

**6.** The method of claim 1, wherein the dividing is performed recursively at two levels, a
first level dividing a value of an outcome among contributions and a second level dividing a
value of a contribution among intra-contribution contributors, by the same graph-restricted
cooperative-game value applied to a sub-game.

**7.** A computer-implemented method of attributing collusion among contributors on a
distributed ledger, comprising: for each pair of contributor identities linked by provenance
flow, computing a causal share as a sum of a first component measuring net circulation of flow
around directed cycles through the pair and a second component measuring balanced reciprocal
flow between the pair, and attributing the causal share to both identities of the pair, each
identity being a non-transferable identifier derived from consensus; and applying a slash to
each attributed identity in proportion to its causal share under a slash budget bounded such that
a sum of the slashes does not exceed a value manufactured by the collusion, wherein an acyclic
and diversely certified provenance attributes zero to every identity.

**7A.** The method of claim 7, wherein the first component is a harmonic residual of a net flow
relative to a least-squares gradient potential, being nonzero only on net-circulating
directed-cycle edges and zero on an acyclic gradient edge, and the second component is a minimum
of the bidirectional flows between the identities.

**8.** The method of claim 7, further comprising recovering, for a refuted target, a set of
non-transferable identities on a provenance lineage of the target by walking a parent chain of
the target under a cycle guard, and settling a refutation by cancelling an unvested value of the
target and slashing each certifier by a bounded share of the cancelled value.

**9.** The method of claim 8, further comprising merging the slashes of a collusion attribution
and a refutation settlement into a single applied slash per identity by, for an identity on the
provenance lineage of the refuted target, taking a greater of the collusion slash and the
refutation slash of the identity, and for an identity not on the lineage, taking a sum of the
two slashes, whereby an identity that manufactured the value a refutation cancels is penalised
once rather than twice.

**10.** The method of claim 9, further comprising capping the merged slash of each identity at a
current standing of that identity, such that a total slash applied to an identity never exceeds
the value the identity holds.

**11.** The method of claim 7, wherein the slash of a certifier in a refutation settlement is
determined per certifier on the certifier's own standing, such that in a mixed panel an honest
certifier is spared while a certifier of no value is slashed, a slash total under the per-
certifier determination never exceeding a slash total under an unguarded determination.

**12.** The method of claim 1 or claim 7, wherein the units of ledger state are cells each
governed by a lock script defining a transferable holder and a type script carrying the non-
transferable contributor identifier, and the dividing, the backward propagation, and the
collusion attribution are all keyed on the contributor identifier carried by the type script and
never on the holder carried by the lock script.

**13.** The method of claim 7, wherein the computing, attributing, and slashing steps are
performed by a deterministic validity program executed on a virtual machine, such that a slash
that would exceed the manufactured-value budget or a standing cap is unrepresentable as valid
ledger state.

---

*Claim-dependency note (for counsel, not for filing): Claims 1 and 7 are independent and are
drafted to the structural invariant rather than to any one embodiment, so that a design-around
altering a constant, a measure, or an enumerated component does not escape them; the specific
embodiments are held in reserve as dependent claims. Claim 1 is directed to the
graph-restricted marginal-contribution division over a submodular, provenance-connected coalition
value; claim 7 to per-identity collusion attribution keyed on a soulbound identity under a
manufactured-value-bounded slash budget. These are the two centres of gravity of this
application and are drawn to be separately defensible. Claims 2–6 depend on the division: claim
2A narrows the submodular coalition value to the cardinality-of-coverage measure; claim 2B
narrows the marginal-contribution value to a connected-component Myerson restricted game with
Shapley credit; claims 3–5 the backward-propagation embodiment (claim 4 fixing the damping
constant to the reciprocal of the golden ratio in fixed-point arithmetic); claim 6 the two-level
recursion. Claims 7A–11 depend on the attribution: claim 7A narrows the two components to the
Helmholtz–Hodge harmonic residual and the minimum-of-bidirectional-flows; claim 8
provenance-lineage recovery and refutation settlement; claims 9–10 the cross-path merge; claim
11 the per-certifier guard. Claims 12–13 are cross-cutting. Claim 9 (cross-path merge into
settlement application) reads on a 🟡-designed embodiment for its end-to-end wiring into live
dispute settlement; the merge function and target/share computation it recites are built, but the
internal status register should mark the deployment coupling. Multiple-dependency claims (12) are
drafted in the alternative to suit UK practice; counsel may convert to single dependencies for
jurisdictions that restrict multiple dependency. Counsel may renumber claims 2A, 2B, and 7A to
sequential integers on filing.*

## ABSTRACT

The value of a multi-contributor contribution on a distributed ledger is divided among its
contributors by a graph-restricted cooperative-game (Myerson) value over a submodular coverage
game, so that provenance-disconnected contributors cannot pool value and duplicated or
recombined content earns a low marginal share, and the same machinery divides an outcome among
contributions and a contribution among its contributors. Value propagates backward along
provenance edges by a damped fixed-point iteration whose damping factor, a constitutional
constant equal to the reciprocal of the golden ratio, makes the iteration a contraction and
bounds self-referential rings; propagation may be restricted to edges between distinct soulbound
identities so a contributor cannot certify its own work. Collusion is attributed per soulbound
identity by summing an orthogonal directed harmonic-residual component and a mutual balanced
component, with honest acyclic provenance attributing zero; the collusion and refutation slashes
are merged so that an identity that manufactured the value a refutation cancels is penalised
once, disjoint harms are summed, and every identity's slash is capped at its standing, bounding
the aggregate slash by the value manufactured.
