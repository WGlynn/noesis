---
title: "Hybrid Finalisation with Anti-Concentration"
subtitle: "Noesis patent family, Application 3 — DRAFT v1 for Rodney's-AI review"
date: "2 July 2026"
status: "INTERNAL DRAFT — not for filing. Sibling to the Proof-of-Contribution priority application."
---

# HYBRID FINALISATION WITH ANTI-CONCENTRATION

> **Family position.** This is Application 3 in the family mapped in the priority filing
> ("hybrid finalisation with conjunctive anti-concentration composition"). The priority
> application (Proof of Contribution) claims the architectural invariant and the three
> implementation families (separation, valuation, finalisation), and *seeds* the present
> subject-matter in dependent claims 8 through 11. Those seed claims recite the finalisation
> family as dependents of a broad weighted-vote method. This application develops each seed
> into its own centre of gravity: (a) the split whereby a proof-of-work dimension governs
> production and ordering but is excluded from the irreversibility path, and (b) the
> conjunctive anti-concentration composition in which each consensus dimension must
> independently clear a constitutional floor before a checkpoint finalises. Where the priority
> claims recite these as narrowing features of one method, this application recites them as the
> independent inventions they are and adds the retention-and-quorum machinery that makes the
> finalisation basis well-defined.
>
> **Organising principle (inherited, held as interchangeable terminology):** finalisation is a
> property of ledger state, and the safety of finalisation is made structural — no single axis
> of power can finalise a proposal by itself, and the axis whose finality is merely probabilistic
> is kept off the safety path entirely. "Dimension", "axis", and "consensus mixture component"
> are interchangeable labels; the claims capture the invariant.

## FIELD

Distributed-ledger consensus; specifically, the finalisation of proposals in a weighted-vote
consensus over a plurality of heterogeneous dimensions, in which the dimensions carrying
probabilistic finality are excluded from the irreversibility rule and the remaining dimensions
must each independently satisfy an anti-concentration floor for a proposal to finalise.

## BACKGROUND

A consensus that draws its weight from more than one source — for example, proof-of-work energy
expenditure, staked capital, and earned contribution — must decide how those sources combine to
declare a block irreversible. Three problems are specific to a multi-dimensional finality rule
and are not solved by prior art:

1. **Mixing probabilistic finality into a safety rule.** Proof-of-work confirmation is
   probabilistic and reorganisable: a block deep in the chain is only overwhelmingly likely, never
   certain, to remain canonical. A finalisation rule that counts proof-of-work weight toward the
   irreversibility threshold inherits that reorganisability into the safety guarantee. The
   probabilistic layer is useful for producing and ordering blocks and for making Sybil identity
   costly, but it must not be on the path that declares a block final. Prior hybrid designs that
   fold a proof-of-work term into a single finality weight leave this hole open.

2. **Unilateral finalisation by the dominant dimension.** If finality is a simple weighted sum,
   then whichever dimension carries the largest weight can, on its own, reach the supermajority
   threshold. A contribution dimension weighted above one half of the finality mixture would then
   finalise proposals without the capital dimension's consent, and a capital dimension could do
   the same in the converse arrangement. A weighted sum permits the largest axis to act alone,
   which defeats the purpose of having multiple axes.

3. **An ill-defined or gameable finalisation basis.** The denominator against which a
   supermajority is measured is itself an attack surface. If the basis is the live participating
   weight only, an adversary who eclipses honest validators shrinks the denominator and forces a
   premature finalisation; if the basis ignores participant liveness, stale validators keep their
   full influence indefinitely. The basis must be robust to both shrinkage and staleness, and the
   liveness discount must reduce voting influence without corrupting the underlying accounting.

Prior multi-dimensional and hybrid consensus systems combine their dimensions additively and do
not address a conjunctive anti-concentration composition, a probabilistic-dimension exclusion
from the safety path, and a robust finalisation basis together. The present invention does.

## SUMMARY

A proposal finalises under a finalisation mixture that assigns zero weight to the proof-of-work
dimension while retaining that dimension in a distinct, overall consensus mixture that governs
block production and ordering. The finalisation mixture is renormalised over the remaining
dimensions, so the supermajority threshold is measured against the set of dimensions whose
finality is not probabilistic.

Over that finalisation mixture, a proposal finalises only if two conditions both hold. First,
the supporting weight reaches a supermajority fraction of a finalisation basis. Second, each
dimension in the finalisation mixture independently supplies at least a fixed minimum fraction of
that dimension's own total weight in support of the proposal. The second condition is
conjunctive: it is evaluated per dimension and all dimensions must pass. The minimum fraction is
a constitutional constant, fixed outside the reach of governance, so that no single dimension can
finalise a proposal alone — the capital axis and the contribution axis must each independently
consent.

The finalisation basis is the greater of a total effective weight of the validators and a quorum
floor equal to a fixed fraction of the total base weight, guarding against denominator shrinkage.
Each validator's effective weight is its base weight, a weighted sum over the dimensions,
multiplied by a retention factor that decreases with time since the validator's last heartbeat
and is applied symmetrically. The retention factor reduces voting weight without reducing the
validator's staked balance, and a validator remains slashable irrespective of its retention.

## DETAILED DESCRIPTION

All source pointers are to the public reference node (`WGlynn/noesis`, Rust on a RISC-V VM,
Cell state model). The mechanisms below are implemented and tested unless a status marker says
otherwise. Status discipline: ✅ built, 🟡 designed, 🔬 open — never rounded up.

### Two distinct mixtures: consensus versus finality (✅ built)

The system maintains two distinct dimension mixtures, and conflating them is the most common
misreading of the design.

The **overall consensus mixture** governs block production and ordering. It weights all three
dimensions: proof-of-work at 0.10, proof-of-stake at 0.30, and proof-of-mind (contribution) at
0.60 (`NCI`, `node/src/lib.rs:3484`; documented as "NCI = 0.10/0.30/0.60" at `:3475`). The
combined base weight of a validator under a mixture is the linear form
`W = pow·m.pow + pos·m.pos + pom·m.pom` (`base_weight`, `node/src/lib.rs:3502-3504`).

The **finalisation mixture** governs irreversibility and is a *different* quantity. It assigns
zero weight to the proof-of-work dimension and renormalises the remaining two so they sum to one:
proof-of-stake at 1/3 and proof-of-mind at 2/3 (`FINALITY_MIX`, `node/src/runtime.rs:584-588`).
The in-code rationale states the reason directly: proof-of-work "secures production / ordering /
sybil-cost, never finality" because its finality is probabilistic, so the probabilistic layer is
kept out of the immediate finality weight (`node/src/runtime.rs:575-582`). Because the mixture is
renormalised over the fast-final set, the supermajority bar is two-thirds of the proof-of-stake
plus proof-of-mind set, not two-thirds of a mixed-confidence global total (`:582-583`).

The distinction is load-bearing and is the first independent invention here: the same validator
population is weighted one way to produce and order blocks and a different way to declare them
irreversible, precisely so that the reorganisable dimension can do useful production work without
contaminating the safety guarantee.

### The anti-concentration floor (✅ built)

A checkpoint finalises only if, in addition to clearing the supermajority threshold, each
fast-final dimension independently supplies at least a minimum fraction of that dimension's own
total in support. The minimum fraction is `MIN_DIM_BPS = 5000`, i.e. fifty percent expressed in
basis points (`node/src/runtime.rs:596`). The per-dimension predicate is `dim_ok`, which passes a
dimension only if the supporting weight in that dimension is at least `MIN_DIM_BPS` of the
dimension's total (`node/src/runtime.rs:598-602`); a dimension absent from the whole set is
excluded from gating to avoid division by zero (`:600-601`).

The composition is conjunctive. The finalisation predicate first requires the renormalised
supermajority to be met, then requires *both* `dim_ok(pos_for, pos_all)` *and*
`dim_ok(pom_for, pom_all)` to hold (`finalizes_pos_pom`, `node/src/runtime.rs:608-633`, the
conjunction at `:632`). The consequence is stated in the code comment and is the crux of the
inventive step: proof-of-mind's larger sixty-percent share "cannot unilaterally finalize" because
the capital axis must independently participate, and conversely (`node/src/runtime.rs:590-593`).
Neither axis finalises alone; capital and contribution must each independently consent.

The floor is a **constitutional constant, explicitly not governance-tunable** — it sits at the
physics/constitutional layer of the value-matrix governance rather than the tunable layer
(`node/src/runtime.rs:593-596`). This is a deliberate design property, not an accident of
configuration: the anti-concentration guarantee cannot be voted away by whichever coalition would
benefit from removing it.

### The finalisation basis: retention and quorum (✅ built)

The basis against which the supermajority is measured is constructed to resist both denominator
shrinkage and validator staleness.

Each validator's **effective weight** is its base weight discounted by a retention factor.
Retention is `1.0` when fresh and declines linearly to `0.0` as elapsed time reaches a horizon,
clamped to the unit interval (`retention(elapsed, horizon)`, `node/src/lib.rs:3507-3512`).
Effective weight multiplies the dimension contributions by this retention factor, applied
symmetrically across the decaying portions (`effective_weight`, `node/src/lib.rs:3516-3525`).

The **finalisation basis** is the greater of the total effective weight and a quorum floor equal
to a fixed fraction of the total base weight (`finalizes_hybrid`, `node/src/lib.rs:3623-3642`;
`basis = eff_total.max(floor)`, `:3639-3640`, where `floor = base_total · quorum_floor_bps / BPS`
at `:3639`). A proposal finalises when supporting effective weight reaches the supermajority
fraction of this basis (`:3641`). The supermajority fraction is `TWO_THIRDS_BPS = 6667`, i.e.
two-thirds expressed in basis points (`node/src/lib.rs:3486`). Taking the maximum of effective
total and the base-derived floor stops an eclipse attacker from shrinking the denominator below
real honest participation while still letting the effective term close the liveness gap
(`:3618-3622`).

Critically, the retention factor reduces **vote weight, not staked balance**. The test
`decay_touches_vote_weight_not_the_staked_balance` fixes this: a fully-stale validator's effective
weight goes to zero while its staked balance is unchanged (`node/src/lib.rs:3792-3797`, assertion
of untouched balance at `:3796`). Retention-decay of influence and any decay of the underlying
capital are distinct operations on distinct quantities, and a validator remains slashable
irrespective of its retention factor.

### Composition into the finalisation call (✅ built)

The anti-concentration finaliser calls the retention-and-quorum machinery with the finalisation
mixture and then applies the per-dimension floors. `finalizes_pos_pom` invokes
`finalizes_hybrid(..., FINALITY_MIX, ...)` and, only if that returns true, evaluates the
conjunctive `dim_ok` floors on the raw per-dimension sums (`node/src/runtime.rs:616-632`). Thus
the three inventions compose: the proof-of-work-excluded finality mixture feeds a shrinkage- and
staleness-robust basis, over which a supermajority must be reached, and on top of which each
dimension must independently clear the constitutional floor.

### Inventive-step note (synergy, not aggregation)

The strength of the claimed combination is that its elements are interdependent rather than merely
co-present. The anti-concentration floor is what makes contribution-minted authority safe to place
on the finality path at a sixty-percent weight; but that floor is only meaningful because the
contribution dimension is soulbound and separated from transferable capital (the subject of the
sibling separation application). If authority could be purchased, an attacker could satisfy both
the capital and contribution floors by purchase, and the conjunction would protect nothing. The
proof-of-work exclusion is what lets a probabilistic energy layer do production work without
weakening finality; but it is only safe to exclude proof-of-work from finality because the two
remaining dimensions, held to independent floors, jointly carry the safety guarantee. Remove any
one element — the exclusion, the floor's conjunction, or the constitutional non-tunability — and
the safety property collapses. That interdependence is the signature of a synergistic combination
and is the opposite of an obvious aggregation of known hybrid-consensus parts.

### Relationship to the overall consensus mixture (clarifying, ✅ built)

For the avoidance of the common misreading: the overall consensus mixture
(`NCI = 0.10/0.30/0.60`, `node/src/lib.rs:3484`) is the quantity used for production and ordering,
and it is the mixture over which anti-plutocracy is enforced structurally by the linearity of the
weight path (`node/src/lib.rs:3659-3666`). The finalisation mixture
(`FINALITY_MIX = 0/(1/3)/(2/3)`, `node/src/runtime.rs:584-588`) is the distinct quantity that
governs irreversibility. Both are single-source-of-truth constants in the reference node; the
claims below are drawn to the *relationship* between them (proof-of-work included in one and
excluded from the other) rather than to any particular numeric value.

## CLAIMS

**1.** A computer-implemented method of finalising proposals in a weighted-vote consensus over a
plurality of dimensions, comprising: maintaining a first consensus mixture that assigns respective
weights to the plurality of dimensions and governs production and ordering of blocks, the
plurality of dimensions including at least one dimension whose finality is probabilistic;
maintaining a distinct finalisation mixture that excludes the said at least one dimension whose
finality is probabilistic and assigns weights to the remaining dimensions; and declaring a
proposal final only when supporting weight computed under the finalisation mixture reaches a
supermajority fraction of a finalisation basis, whereby the dimension whose finality is
probabilistic contributes to production and ordering but is excluded from the rule that declares a
block irreversible.

**2.** The method of claim 1, wherein the dimension whose finality is probabilistic is retained in
the first consensus mixture for at least one of block production, block ordering, and imposition of
a Sybil identity cost, the supermajority fraction of the finalisation mixture being measured
against a set comprising only the remaining dimensions rather than against a total that includes
the said dimension whose finality is probabilistic.

**2A.** The method of claim 1, wherein the said dimension whose finality is probabilistic comprises
a proof-of-work dimension.

**2B.** The method of claim 1, wherein the finalisation mixture assigns to the remaining dimensions
renormalised weights that sum to unity, such that the supermajority fraction is measured over a
normalised total of the remaining dimensions.

**3.** A computer-implemented method of finalising proposals in a weighted-vote consensus over a
plurality of dimensions, comprising: for a proposal, computing for each dimension of the plurality
a supporting weight in that dimension and a total weight in that dimension; and declaring the
proposal final only if, in addition to a supporting weight reaching a supermajority fraction of a
finalisation basis, each dimension of the plurality independently supplies a supporting weight of
at least a minimum fraction of the total weight of that dimension, the said minimum fraction being
a constant that is not adjustable by the consensus governance, and the said condition being
evaluated per dimension and required to hold conjunctively across all of the plurality, whereby no
single dimension finalises the proposal alone and the conjunction cannot be removed by a coalition
that would benefit from its removal.

**4.** The method of claim 3, wherein the said minimum fraction is held at a constitutional layer of
the system distinct from a layer of governance-adjustable parameters, such that no procedure of the
consensus governance can raise, lower, or disable it.

**5.** The method of claim 3, wherein the minimum fraction is at least one half of the total
weight of each dimension, such that a first dimension carrying a larger weight in the finalisation
mixture than a second dimension nonetheless cannot finalise the proposal without the second
dimension independently supplying at least said minimum fraction of the second dimension's total
weight.

**5A.** The method of claim 5, wherein the minimum fraction is one half of the total weight of each
dimension, expressed as five thousand basis points of that dimension's total weight.

**6.** The method of claim 3, wherein a dimension whose total weight across the validators is zero
is excluded from the said per-dimension condition, and the remaining dimensions are each required
to satisfy the condition.

**7.** The method of claim 1 or claim 3, wherein the plurality of dimensions of the finalisation
mixture comprises a proof-of-stake dimension representing transferable staked capital and a
proof-of-mind dimension representing non-transferable earned contribution, and the said condition
requires both the proof-of-stake dimension and the proof-of-mind dimension to independently supply
at least the fixed minimum fraction, whereby capital and contribution must each independently
consent to finalisation.

**8.** The method of claim 1 or claim 3, further comprising computing, for each validator, an
effective weight by multiplying a base weight comprising a weighted sum over the plurality of
dimensions by a retention factor that decreases with time elapsed since the validator's last
heartbeat and is applied symmetrically, and computing the finalisation basis as the greater of a
total effective weight of the validators and a quorum floor equal to a fixed fraction of a total
base weight of the validators.

**9.** The method of claim 8, wherein the retention factor decreases linearly from unity to zero
as the said elapsed time increases from zero to a horizon and is clamped to the unit interval, the
supermajority fraction being measured against the finalisation basis so computed.

**9A.** The method of claim 1 or claim 3, wherein the supermajority fraction is two-thirds of the
finalisation basis, expressed as six thousand six hundred and sixty-seven basis points.

**10.** The method of claim 8, wherein the retention factor reduces the effective weight of the
validator without reducing a staked balance of the validator, and the validator remains subject to
slashing irrespective of the retention factor.

**11.** The method of claim 8, wherein taking the said greater of the total effective weight and
the quorum floor prevents a party that suppresses participation of honest validators from reducing
the finalisation basis below a fixed fraction of the total base weight, thereby resisting a
premature finalisation induced by denominator shrinkage.

**12.** The method of claim 1, wherein the finalisation basis and the supermajority test are
computed by a deterministic validity program executed on a virtual machine, the said program
producing an identical finalisation decision across replicas from identical ledger inputs.

**13.** The method of claim 1 or claim 3, wherein the said declaring is performed by a
deterministic program that returns failure, causing a checkpoint not to finalise, whenever either
the supermajority fraction is not reached under the finalisation mixture or any dimension fails to
independently supply the fixed minimum fraction, such that a checkpoint lacking independent
support from every dimension is unrepresentable as a finalised state.

---

*Claim-dependency note (for counsel, not for filing): Claims 1 and 3 are independent and are the
two centres of gravity of this application. Claim 1 is directed to the proof-of-work-excluded
finalisation mixture (the "included in consensus, excluded from finality" split); claim 3 to the
conjunctive anti-concentration composition (each dimension independently clearing a fixed floor).
They are drawn to be separately defensible: claim 1 reads on any hybrid that keeps a
probabilistic-finality dimension off the safety path, without reciting that the dimension is
proof-of-work or that the remaining weights are renormalised; and claim 3 on any multi-dimensional
finaliser with a per-dimension conjunctive floor whose minimum fraction is a non-governance-tunable
constant, independently of the specific dimensions and independently of the exact fraction. Claim 2
depends on the exclusion; claims 2A (proof-of-work label) and 2B (renormalisation to unity) hold
the specific features removed from independent claim 1 in reserve as fallback narrowings. Claims 4,
5, 5A and 6 depend on the anti-concentration composition, with 5A holding the exact one-half
(five thousand basis points) floor and 4 the constitutional-layer non-tunability. Claim 7 ties the
two independents to the specific capital/contribution dimensions; claims 8 through 11 recite the
retention-and-quorum basis and are cross-cutting; claim 9A holds the exact two-thirds (six thousand
six hundred and sixty-seven basis points) supermajority fraction in reserve; claims 12 and 13
recite the deterministic-VM enablement and the fail-closed unrepresentability property. Every claim
reads on a ✅-built mechanism in the reference node; no claim in this application depends on a
🟡-designed or 🔬-open embodiment. Multiple-dependency claims (7, 8, 9A, 13) are drafted in the
alternative ("claim 1 or claim 3") to suit UK practice; counsel may convert these to single
dependencies for jurisdictions that restrict multiple dependency.*

## ABSTRACT

Proposals in a weighted-vote consensus over a plurality of dimensions are finalised under a
finalisation mixture that assigns zero weight to a proof-of-work dimension while a distinct
consensus mixture retains that dimension for production and ordering, so a probabilistic,
reorganisable layer does not contaminate the irreversibility guarantee. A proposal finalises only
when supporting weight reaches a supermajority fraction of a finalisation basis and, conjunctively,
each dimension independently supplies at least a fixed minimum fraction of that dimension's own
total, the minimum being a constitutional constant that is not governance-tunable, so that neither
capital nor contribution can finalise a proposal alone. The finalisation basis is the greater of a
total effective weight, discounted by a symmetric per-validator retention factor that reduces vote
weight without reducing staked balance, and a quorum floor derived from total base weight, making
the basis robust to both staleness and denominator shrinkage. The elements are interdependent: the
anti-concentration floor is only meaningful when the contribution dimension is non-transferable,
and excluding proof-of-work from finality is only safe because the remaining dimensions jointly
carry the guarantee under their independent floors.
