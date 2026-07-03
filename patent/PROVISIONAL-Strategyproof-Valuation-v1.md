---
title: "Strategyproof Temporal Valuation of Contributions"
subtitle: "Noesis patent family, Application 3 — DRAFT v1 for Rodney's-AI review"
date: "2 July 2026"
status: "INTERNAL DRAFT — not for filing. Sibling to the Proof-of-Contribution priority application."
---

# STRATEGYPROOF TEMPORAL VALUATION OF CONTRIBUTIONS

> **Family position.** This is Application 3 in the family mapped in the priority filing
> ("Family B, strategyproof valuation by temporal novelty"). The priority application
> (Proof of Contribution) claims the architectural invariant and *seeds* this valuation
> invention in dependent claims 4 through 7. This application develops those seeds into a
> standalone invention: the deterministic, integer-only rule by which a contribution's
> consensus value is computed from temporal novelty in a consensus-fixed canonical order,
> such that duplicate, padding, recombined, or near-duplicate contributions earn zero, and
> such that the computation is bit-identical across replicas and therefore safe to place on
> the consensus path.
>
> **Organising principle (inherited, held as interchangeable terminology):** the quantity
> that drives consensus authority is *produced* by protocol rule and measured objectively
> over chain data, not assessed by subjective judgement. "Temporal novelty", "novel
> coverage", "strategyproof value" are interchangeable labels; the claims capture the
> invariant, which is that a later contribution adding no new coverage over the union of all
> earlier-ordered coverage is worth nothing, computed by a rule no producer can game.

## FIELD

Distributed-ledger consensus; specifically, deterministic and manipulation-resistant
valuation of contributions whose value determines a consensus authority, computed in
integer arithmetic that is reproducible across independent nodes.

## BACKGROUND

A consensus system that weights participation by *measured contribution* rather than by
transferable stake must define how a contribution is valued. That valuation function
becomes a direct attack surface: whoever can inflate their measured value inflates their
consensus authority. Three problems are specific to contribution valuation and are not
solved by prior art:

1. **Duplication and padding inflation.** If value is a function of a contribution's own
   content in isolation, a producer can manufacture authority by resubmitting content that
   is already on the ledger, by submitting a subset or padded variant of existing content,
   or by recombining existing content into apparently new units. Each such submission is
   worthless to the system yet, under a naive valuation, earns value. A valuation must make
   redundant contribution earn zero as a structural property, not by after-the-fact
   detection.

2. **Order manipulation.** A valuation that rewards being *first* to contribute given
   content invites a producer to arrange the apparent order of contributions so that a
   redundant unit appears earlier than the original it copies, stealing the novelty. If the
   valuation trusts a producer-arrangeable ordering — a submitted list order, or a
   self-asserted timestamp — the strategyproof property is only relocated, not achieved. The
   ordering that defines "earlier" must itself be sourced from consensus and be
   unpredictable to the producer at commit time.

3. **Non-reproducible or subjective scoring.** A valuation that depends on floating-point
   arithmetic, on iteration order of unordered collections, or on a subjective quality
   judgement cannot be placed on the consensus path: independent nodes will disagree on the
   value and therefore on the resulting authority, breaking determinism. Any component that
   is not bit-identical across replicas, or that is a subjective assessment, must be kept
   off the path that determines finality.

Prior contribution-weighted consensus systems (for example Bittensor-style measured-value
weighting) do not address a strategyproof valuation with a structural zero-for-duplicate
property, a consensus-sourced ordering that dissolves order manipulation, and an
integer-exact near-duplicate floor. The present invention does.

## SUMMARY

A contribution is valued by its *temporal novelty*: the number of content elements in the
contribution's coverage that are absent from the union of the coverage of all contributions
earlier in a canonical order. A contribution that duplicates, is contained within, or is
recombined from earlier content contributes no new elements to that union and therefore is
assigned a value of zero, by construction rather than by detection.

The canonical order that defines "earlier" is not the order in which contributions are
presented. It is sourced from consensus at two scales: between blocks by the commit-reveal
block height at which each contribution's commitment landed, and within a block by a
Fisher-Yates permutation seeded by the exclusive-or of every revealing participant's
secret. Because a participant commits a hash of its contribution and secret before any
secret is revealed, and because its within-block slot depends on the combined secrets of
all participants, no producer can predict, let alone choose, its own position. Order
manipulation is dissolved rather than detected.

The novelty count is refined by a near-duplicate similarity floor: a contribution whose
coverage overlap with the union of earlier coverage exceeds a threshold is assigned zero,
closing the residual leak by which a paraphrase of existing content earns a small non-zero
value. Both the novelty count and the similarity floor are computed in integer arithmetic —
the overlap comparison is performed by exact cross-multiplication rather than division — so
the result is bit-identical across replicas and safe to drive the consensus authority.

An optional learned quality score may multiply the value, but as a bounded multiplier
applied to novelty, so that a zero-novelty contribution remains valued at zero at any
quality. The learned model is deliberately kept off the consensus path.

## DETAILED DESCRIPTION

All source pointers are to the public reference node (`WGlynn/noesis`, Rust on a RISC-V VM,
Cell state model). The mechanisms below are implemented and tested unless a status marker
says otherwise. Status discipline: ✅ built, 🟡 designed, 🔬 open — never rounded up.

### Coverage of a contribution (✅ built)

Each contribution is a cell carrying `data`. Its *coverage* is the set of content shingles
of that data, each identified by an integer `CovId` (`node/src/lib.rs:58`, `coverage`
`:62`). Coverage is a deterministic function of the bytes and is the proxy over which
novelty is measured. The union of coverage across contributions is the set of content
elements the ledger has already received.

### Temporal-novelty valuation with the zero-for-duplicate property (✅ built)

`temporal_novelty(cells_in_commit_order)` walks the contributions in canonical order,
maintaining the running union `seen` of all coverage from earlier contributions, and
assigns to each contribution the count of its coverage elements not in `seen`
(`node/src/lib.rs:89-99`; the novel-count filter at `:94`, the union update at `:96`). A
contribution whose coverage is entirely contained in the union of earlier coverage — an
exact duplicate, a subset (padding), or a recombination of already-seen elements — adds no
new element and is therefore assigned zero. This is the strategyproof floor: a later sybil,
padding, or collusion cell earns nothing by construction (verified in the adversarial test
suite, which appends attacker cells after the honest set and asserts they earn zero,
`node/src/lib.rs:7291-7294`).

### Consensus-sourced canonical order (✅ built)

Strategyproofness holds only if "earlier" is a relation the producer cannot arrange. The
valuation therefore does not trust the order in which cells are presented. It sorts cells by
a consensus-sourced coordinate before running the novelty walk
(`novelty_in_commit_order`, `node/src/lib.rs:135-152`, canonical sort at `:144`), and keys
the returned values back to presentation order (`:147-150`). The canonical order is defined
in the shared on-VM core (`commit_order`, re-exported at `node/src/lib.rs:8980`;
implementation `onchain/noesis-core/src/lib.rs`):

- **Between blocks**, order is by the commit-reveal block height at which the commitment
  landed (`canonical_order`, `onchain/noesis-core/src/lib.rs:279-300`, height sort at
  `:283-287`). A later height can never precede an earlier one; a self-set timestamp field
  is never consulted, so backdating it is a no-op (pinned by the test
  `temporal_order_is_consensus_critical_and_timestamp_is_not_the_lever`,
  `node/src/lib.rs:759-785`).
- **Within a block**, same-height ties are ordered by a Fisher-Yates permutation seeded by
  the exclusive-or of every revealing participant's secret (`block_shuffle`,
  `onchain/noesis-core/src/lib.rs:260-275`, XOR seed at `:263-268`). A participant commits a
  hash of its contribution and secret before any secret is revealed, and its slot depends on
  the combined secrets of all participants (`Committed { height, secret }`,
  `onchain/noesis-core/src/lib.rs:239-241`), so no producer can predict or choose its
  position. This dissolves producer-favourable ordering rather than detecting it case by
  case: there is no secret a rational producer can pick that guarantees an earlier slot.

The type script asserts the batch is already in canonical order and rejects a non-canonical
batch rather than silently re-sorting it (`is_canonical_order`,
`onchain/noesis-core/src/lib.rs:304-309`).

**Honest status of the ordering.** The permutation itself — the height sort and the
secret-seeded intra-block shuffle — is ✅ built and consensus-replayable, drift-guarded
bit-identical between the node and the on-VM core. The remaining seam is the on-VM
*sourcing* of the ordering coordinates: binding `height` to the commitment's block header
and `secret` to the block's reveals (rather than to a producer's claim) is the
deploy-coupled, sentinel-gated step (`onchain/noesis-core/src/lib.rs:229-231`). That
sourcing is 🟡 designed. The claims below recite the ordering mechanism, which is supported
for claiming; the internal status register should note that the on-chain coordinate sourcing
is the deploy-coupled part, not rounded up to shipped.

### Deterministic-integer valuation with a near-duplicate similarity floor (✅ built)

The plain novelty rule zeroes only exact subsets and duplicates. A near-duplicate — a few
tokens flipped — leaks a small residual novelty from the change-spanning shingles, so a
paraphrase-padding ring of K near-copies banks roughly K contributions of standing. The
similarity floor closes this: a contribution whose coverage overlap with the union of
earlier coverage exceeds a threshold `theta_sim` is assigned zero
(`temporal_novelty_with_similarity_floor`, reference form
`node/src/lib.rs:109-125`, floor test `overlap > theta` at `:121`).

The consensus-path form of this computation is integer-only. Overlap is `|cov ∩ earlier| /
|cov|`; rather than dividing, the rule cross-multiplies and compares
`overlap · 2^Q > theta_q16 · |cov|` in wide integer arithmetic, which is exact
(`temporal_novelty_with_similarity_floor_q16`, `node/src/lib.rs:6285-6302`, the
cross-multiplied comparison at `:6296`). This is the crux of reproducibility: no
floating-point, no division, so the value is bit-identical across independent replicas and
therefore safe to determine consensus authority. The per-contributor attribution that the
runtime consumes is `pom_scores_with_similarity_floor_q16`, which sums this floored novelty
per soulbound contributor identity (`node/src/lib.rs:182-192`; the runtime invokes it at
`node/src/runtime.rs:525`). The plain, unfloored attribution `pom_scores`
(`node/src/lib.rs:162-169`) is the reference variant; the floored integer form is the
deployed one.

### The similarity threshold is a constitutional measurement parameter (✅ built)

`theta_sim_q16` is carried by the genesis Constitution, which governs how value is measured
(`Constitution.theta_sim_q16`, `node/src/runtime.rs:56`). Its default is `62259`, that is
`floor(0.95 · 2^16)`, so only near-identical contributions (overlap above ~0.95) are cut and
honest novel work at low overlap is untouched (`node/src/runtime.rs:77`). Placing the
threshold in the constitution rather than in ordinary governance keeps the measurement rule
in the amendment frame rather than tunable per block.

### Quality multiplier, kept off the consensus path (🔬 open)

An optional learned quality score may boost value as `value = novelty · (1 + quality)` with
`quality` bounded to `[0, 1]` (reference form `value_v4`, `node/src/lib.rs:949-954`;
documented composition `node/src/lib.rs:842-850`; integer mirror `production_value_q16`,
`node/src/lib.rs:6306-6320`). Because novelty *multiplies*, a zero-novelty contribution
earns zero at any quality — the strategyproof floor stays dominant regardless of the learned
component. **Honest status:** the integer composition is built, but the learned quality
model trained on real downstream-value labels is deliberately **not** on the consensus path
and is research-stage; it is a scoring layer, not a determinant of finality. It is claimed
here only in the bounded-multiplier, floor-preserving form, and the learned model itself is
marked 🔬 open — not rounded up.

### Inventive-step note (synergy, not aggregation)

The three components are interdependent, which is the signature of a synergistic combination
rather than an aggregation of known parts. The zero-for-duplicate novelty rule is
strategyproof only because "earlier" is sourced from a consensus order the producer cannot
arrange; a producer who could reorder would present a redundant copy first and steal the
novelty, so the ordering mechanism is load-bearing for the valuation, not merely adjacent to
it. The near-duplicate similarity floor is meaningful only because it is computed in exact
integer arithmetic; a floating-point floor would itself break the replica-determinism that
lets the value drive consensus, so the integer computation is not an implementation detail
but a precondition of putting the valuation on the finality path at all. And the learned
quality multiplier is safe to admit only because it multiplies a strategyproof floor that is
zero for redundant work; without that floor, a learned score would be an unbounded new attack
surface. Remove any one component and either the strategyproofness or the determinism
collapses. That interdependence is the inventive step.

## CLAIMS

**1.** A computer-implemented method of valuing contributions to a distributed ledger,
comprising: establishing a canonical order of the contributions; and assigning to each
contribution a value determined by a measure of coverage of the contribution that is absent
from a union of coverage of all contributions earlier in the canonical order; whereby a
contribution that adds no coverage absent from said union is assigned a value of zero as a
property of the assignment rule.

**2.** The method of claim 1, wherein the canonical order is sourced from consensus and is
not an order in which the contributions are presented, such that a producer cannot cause a
redundant contribution to be ordered earlier than a contribution whose content it
duplicates.

**3.** The method of claim 2, wherein establishing the canonical order comprises ordering
the contributions between blocks by a commit-reveal block height at which a commitment of
each contribution landed, a self-asserted timestamp of a contribution not being consulted,
such that a contribution committed at a later height cannot precede a contribution committed
at an earlier height.

**4.** The method of claim 2, wherein establishing the canonical order comprises ordering
contributions committed at a same block height by a permutation seeded by a combination of
secrets each revealed by a respective participant after committing a hash of a contribution
and the secret, such that no participant can predict or select its own position in the
canonical order.

**5.** The method of claim 4, wherein the combination of secrets is an exclusive-or of the
secrets of all revealing participants and the permutation is a Fisher-Yates permutation
seeded from the exclusive-or, and wherein a batch of contributions presented in an order
other than the canonical order is rejected rather than re-sorted.

**6.** A computer-implemented method of valuing contributions to a distributed ledger,
comprising: for each contribution in a canonical order, deriving a novelty measure of
coverage of the contribution absent from a union of coverage of earlier contributions, and
deriving an overlap of the contribution's coverage that is present in said union; and
assigning the contribution a value of zero when the overlap satisfies a threshold relation
and a value determined by the novelty measure otherwise; wherein the overlap is evaluated
against the threshold by a division-free exact-integer operation, such that the assigned
value is bit-identical across independent nodes computing it, whereby the value is admissible
as a determinant of a consensus authority.

**7.** The method of claim 6, wherein the threshold is carried by a genesis constitution as a
fixed-point integer parameter governing measurement and is not tunable by ordinary
governance.

**8.** The method of claim 1 or claim 6, further comprising accumulating the assigned values
per contributor by keying an attribution on a soulbound contributor identifier of each
contribution rather than on a transferable holder, such that the accumulated value tracks the
contributor that produced the content.

**9.** The method of claim 6, wherein the coverage of a contribution is a set of content
shingles of data of the contribution, each identified by an integer, and the novelty count
and overlap fraction are computed over integer set operations that do not depend on an
iteration order of an unordered collection.

**10.** The method of claim 1 or claim 6, further comprising multiplying the assigned value
by a factor comprising one plus a learned quality score bounded to a unit interval, such
that a contribution of zero said cardinality or zero said novelty count is assigned zero
value at any quality score, and wherein the learned quality score is excluded from a
computation that determines finality.

**11.** The method of claim 6, wherein the computing and assigning steps are performed by a
deterministic validity program executed on a virtual machine, and a contribution whose
value is asserted at a value other than the value the program computes causes the program to
return failure and the transaction to be invalid.

**12.** The method of claim 1, wherein assigning a value of zero to a duplicate, contained,
or recombined contribution is a property of the assignment rule itself rather than of a
detection step applied after assignment, such that no evasion of the zero valuation exists to
be detected.

**13.** The method of claim 1, wherein the coverage is a set of content elements each
identified by an integer, and the value assigned to each contribution is a cardinality of
those content elements of the contribution that are absent from the union.

**14.** The method of claim 1, wherein the contribution that adds no coverage absent from the
union is one of a duplicate of earlier content, a subset contained within earlier content,
and a recombination of earlier content.

**15.** The method of claim 6, wherein the novelty measure is a cardinality of coverage
elements of the contribution absent from the union, and the overlap is a fraction of the
contribution's coverage that is present in the union.

**16.** The method of claim 6, wherein the division-free exact-integer operation is a
cross-multiplication comparing the overlap scaled by a fixed-point factor against the
threshold multiplied by a cardinality of the contribution's coverage.

---

*Claim-dependency note (for counsel, not for filing): Claims 1 and 6 are independent and are
the two centres of gravity of this application. Claim 1 is directed to the temporal-novelty
valuation with the zero-for-duplicate property; claim 6 to the deterministic-integer
computation with the near-duplicate similarity floor, which is the strategyproof core that
makes the value admissible on the consensus path. They are drawn to be separately defensible:
claim 1 stands even without the integer-floor refinement, and claim 6 stands even if the
novelty count is defined differently. Claims 1 and 6 are drafted to the structural invariant
(a measure of coverage absent from the earlier-ordered union; a division-free exact-integer
threshold evaluation) so that a design-around by substituting the specific measure, set
representation, or comparison arithmetic still reads on the independent claim; the demoted
specifics are held in reserve as narrowing fallbacks in dependent claims 13 through 16 (claim
13, cardinality-of-integer-elements form of claim 1; claim 14, the duplicate/contained/
recombined enumeration; claim 15, the novelty-count and overlap-fraction form of claim 6;
claim 16, the cross-multiplication form of the integer comparison). Claims 2 through 5 develop
the consensus-sourced ordering and depend on the valuation; claims 7 and 9 depend on the
integer computation; claims 8, 10, 11, 12 are cross-cutting. Claim 10 recites the learned quality multiplier in a
floor-preserving, off-finality-path form; the learned model itself reads on a 🔬-open
embodiment and should be marked in the internal status register. Multiple-dependency claims
("claim 1 or claim 6") are drafted in the alternative to suit UK practice; counsel may convert
these to single dependencies for jurisdictions that restrict multiple dependency.*

## ABSTRACT

A contribution to a distributed ledger is valued by its temporal novelty: the number of
content elements in its coverage that are absent from the union of coverage of all
contributions earlier in a canonical order, so that a duplicate, contained, or recombined
contribution is assigned zero by construction. The canonical order is sourced from consensus,
between blocks by commit-reveal height and within a block by a permutation seeded from the
exclusive-or of participants' revealed secrets, so that no producer can order a redundant copy
ahead of the original it duplicates. A near-duplicate similarity floor assigns zero to a
contribution whose coverage overlap with earlier coverage exceeds a threshold, computed by an
exact integer cross-multiplication rather than division, so the value is bit-identical across
nodes and admissible as a determinant of consensus authority. An optional learned quality
score may multiply the value as a bounded factor applied to novelty, so a zero-novelty
contribution stays valued at zero, and the learned model is kept off the finality path.
