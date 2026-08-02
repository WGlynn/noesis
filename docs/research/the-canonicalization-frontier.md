# The Canonicalization Frontier — where contribution value is objectively conservable, and where it is not

> Status: this is a **characterization and an open problem**, not a proof and not a shipped mechanism.
> It states a theorem-shaped claim, names the object that claim is about, and gives the artifact
> (an identity matrix) that would let the objective region be extended honestly. The ultimate boundary
> is deliberately left **open**. Nothing here asserts a Noesis protocol constant; the note is about the
> *shape* of the value-oracle seam's identity substrate, and it connects to but does not restate
> `DESIGN-value-oracle-seam.md`, `ISOMORPHISM-INVARIANCE-VS.md`, and `THE-KEYSTONE-content-value-signal.md`.
>
> Origin: six adversarial rounds pressure-testing the Cognitive Consensus / value-conservation design
> with Boardy (2026-08-01). The working name and the artifact shape are his; the failure of an earlier
> version is mine and is recorded honestly below. This is **not** co-authored and the boundary is
> **not** co-named — the formal artifact has to earn that distinction later.

## The result, stated as a theorem-shaped claim

**Within a contribution namespace that admits a computable, unique, mint-time normal form,
representation identity can support objective budget conservation. Outside such a namespace,
conservation inherits an unresolved identity oracle and must not be presented as objective.**

That is the whole claim. Everything below defines its terms, says where the namespace boundary
actually falls, and gives the rule for moving it outward without cheating.

## The three identity levels (the frame the claim lives in)

A conservation law — "the total value drawn against a contribution never exceeds `v(contribution)`" —
is only as objective as its notion of *same contribution*. There are three distinct notions, and
collapsing them is the central bug:

- **Representation identity** — exact artifact equality. What a content hash or a consensus state
  actually decides: `H(x) = H(y)`, modulo cryptographic assumptions. Nothing more.
- **Derivation identity** — same transformation history. Conservable only where the history is itself
  substrate-authenticated (each step attested), otherwise it reduces to the level below or above it.
- **Contribution identity** — sameness of *causal value in the world*. This is the airgap. It is
  evidence-grounded only where a bounded, substrate-authenticated evidence path exists within the
  horizon, and is permanently a social anchor otherwise.

A content hash establishes representation identity. It **cannot** establish contribution identity in
either direction: the same artifact copied into two contexts has one hash but may be two contributions
(or one duplicated), and two causally identical contributions can have different bytes, dependencies,
timing, or authorship and so different hashes. Any design that uses representation identity *as*
contribution identity has silently committed the level-1-as-level-3 bug.

## The dangerous direction is under-merging, not over-merging

The two failure directions are not symmetric, which is what makes the frontier the right object rather
than a vague caution.

- **Over-merge** (call two distinct contributions the same): with a conservative merge rule
  (`v_merged = min`, clawback-or-quarantine the double-paid excess, monotone union-only), a false merge
  can only *underpay* — it griefs honest contributors but cannot inflate. Loud and contestable.
- **Under-merge** (fail to see that two artifacts are the same contribution): representation identity
  defaults different bytes to "distinct," mints a second budget for one contribution, and the merge
  rule never fires because no attacker volunteers the equivalence proof that would cut their own pay.
  **Silent and unbounded.** This is the split/duplication attack without needing to split anything — an
  attacker only has to *re-encode*.

So representation identity is not merely incomplete for contribution identity; where the same
contribution can wear more than one hash, it is **biased toward inflation**. The fix is therefore not
another firewall on top but a precondition underneath: the namespace must enforce a canonical normal
form at mint, so that one contribution has exactly one representation *by construction* and re-encoding
is the same key, not a new object.

## The Canonicalization Frontier

Define the **Canonicalization Frontier** as the set of contribution namespaces that admit a computable,
unique, terminating normal form assignable at mint time. Inside the frontier, representation identity
of normal forms is a faithful proxy for contribution identity, and the conservation invariant is
genuinely objective. Outside it, "same contribution" is an oracle call — an attestation problem — and
conservation is at best socially anchored.

The frontier is strictly larger than raw byte-equality. Non-trivial-but-decidable canonical forms
include:

- **alpha-equivalent syntax** — bound-variable renaming quotiented by a canonical naming (de Bruijn);
- **normalized commutative/associative expressions** — sorted operands under a fixed order;
- **canonical ASTs** — parse-then-canonicalize to a unique tree;
- **dependency-closed artifacts with pinned toolchains** — the closure plus pins is the normal form;
- **traces reduced under a terminating, confluent rewrite system** — the unique normal form is the
  reduct (this is Newman/Banach territory: local confluence + termination gives unique normal forms).

The frontier **breaks** exactly when the normal form would have to depend on:

- **observational behavior** (two programs "do the same thing"),
- **external or mutable state**,
- **open-ended dependency meaning** (what a dependency "is for"), or
- **semantic program equivalence**.

In each of these, normalization is non-terminating, non-confluent, or an attestation problem in
disguise. Semantic sameness is not rank-0 decidable and may be permanently a social anchor.

## The artifact: the identity matrix

The honest way to extend the objective region is to fill this matrix, one row per candidate namespace,
and to admit a namespace only when every column is discharged by proof rather than assertion:

| Namespace | Equivalence relation | Normal-form algorithm | Termination / confluence proof | Conserved object | Legal merge / split | Failure mode (where it leaves the frontier) |
|---|---|---|---|---|---|---|
| Raw bytes | `H(x)=H(y)` | identity | trivial | the artifact | merge only on hash-equal | any re-encoding of the same contribution |
| Alpha-equiv terms | ≡α | de Bruijn canonicalization | terminating, confluent | the term up to renaming | merge proven-α-equal; split on disjoint scope | free variables bound to external meaning |
| Comm/assoc exprs | ≡AC | operand sort under fixed order | terminating, confluent | the normalized expression | as above | operators whose meaning is state-dependent |
| Canonical AST | tree-equal | parse + canonicalize | terminating | the tree | as above | macros / effects that change meaning post-parse |
| Dep-closed + pinned | closure-equal | resolve + pin + hash closure | terminating | the closed artifact | merge on closure-equal | unpinned / floating deps ⇒ open-ended meaning |
| Rewrite-reduced trace | same normal form | reduce to reduct | requires Newman: local confluence + SN | the reduct | merge on same reduct | non-confluent or non-terminating system |
| Behavioral / semantic | "does the same" | — | **none (undecidable)** | — | — | this row is *outside* the frontier: social anchor |

The last row is deliberately empty in its middle columns. That emptiness is the honest content of the
result: it is the boundary, not a gap to be filled by a cleverer algorithm.

## Where this bites Noesis

The value-oracle seam (`DESIGN-value-oracle-seam.md`) already isolates *what work is worth* behind a
`ValueOracle` interface and keeps the oracle **attribution-neutral** — it scores cells, and the
aggregator attributes value to a contributor keyed on `cell.type_script.args`. The identity question is
therefore *upstream* of `v(S)`: before you can conserve value against a contribution, you need "same
contribution," and that is exactly a Canonicalization-Frontier question about the keying.

The consequence is a clean division of the seam:

- The region where cell/contributor identity has a computable mint-time normal form is where PoM
  standing can be **objectively conserved**. The existing deterministic near-duplicate floor (the
  Q16.16 similarity gate) is a concrete, honest instance of pushing representation identity a little way
  into the frontier — it is a *structural* canonicalization of near-duplicates, not a semantic one, and
  it should be understood as living strictly inside the frontier for that reason.
- The region where "same contribution" would require behavioral or semantic equivalence is the
  **social-anchor** region. This is the same boundary the learned `v(S)` + isomorphism-invariance gate
  hit from the value side (`ISOMORPHISM-INVARIANCE-VS.md`): un-gameability is claimed for *demonstrated*
  structural vectors, and the general semantic case is open. The Canonicalization Frontier says *why*
  that is not a temporary shortfall: semantic sameness is outside the decidable region by construction,
  so it is closed by **data and attestation**, never by asserting a semantic relation canonical.

## The extension rule: partial and monotone

The strongest defensible conclusion is that the objective region is **partial and monotone** — but
monotone growth is necessary, not sufficient (see *Composition* below: a namespace may be admitted only
if it is canonical **and** compatible with the ones already admitted). You may extend the region **only**
by proving a larger canonical namespace — a new row of the matrix with its termination/confluence
discharged. You may **never** extend it by treating a richer semantic relation as canonical by
assertion. Concretely, at the mechanism level:

- rank-0 identity conserves budgets only within a declared representation namespace;
- moving from representation to derivation or contribution identity is a **typed successor claim** that
  carries its own authority and can fall to social anchor, never a rank-0 merge;
- "isomorphic" alone is forbidden as a merge justification, because it is precisely the ambiguity an
  attacker exploits by wrapping one artifact in two contexts and arguing for whichever merge or split
  pays.

## Composition, and the compatibility condition the extension rule was missing

The extension rule above assumed the objective region grows by **subsumption** — a larger canonical
namespace swallowing a smaller one. It said nothing about **composition**: what happens when two
independently valid canonical namespaces `N1` and `N2` are combined. Per-row canonicity does not imply
the rows compose. If composition is not closed, "partial and monotone" needs a compatibility condition,
not just a larger-row proof. (This gap was found by the same pressure-test that produced the note.)

Two ways composition breaks:

1. **Overlap disagreement.** A contribution in `N1 ∩ N2` can be identified by `≡1` and distinguished by
   `≡2` (or vice versa). Then the conserved object is ambiguous — one budget under one namespace, two
   under the other — and an attacker invokes whichever namespace pays. The masquerade attack relocated
   to the seam between namespaces.
2. **Non-homomorphic composition.** For a composite `c = a ⊕ b` with `a ∈ N1`, `b ∈ N2`, there may be no
   normal form on the composite determined by `(nf1(a), nf2(b))`. If normalization does not commute with
   `⊕`, the composite's conserved object is not a function of its parts', so value can be created or
   destroyed at the seam.

So the objective region is **not** the union of all canonical namespaces. It is the largest
sub-collection of canonical namespaces that is **closed under composition with agreeing overlaps.** The
extension rule gains a side-condition: a namespace may be admitted iff it is canonical **and** compatible
with every namespace already admitted, where compatible means:

- **(C1) Pairwise overlap agreement (gluing).** On `N1 ∩ N2` the induced identity relations coincide:
  for all `x, y` in the overlap, `x ≡1 y` iff `x ≡2 y`. Disagreement drops the overlap out of the
  frontier.
- **(C2) Interface-preserving, representative-independent normalization.** It is not enough that `nf12`
  be "determined by" `(nf1(a), nf2(b))` — that has to be earned. Component normal forms can erase
  interface behavior, so `nf12` can secretly depend on a *hidden representative* of a component's
  equivalence class. C2 therefore requires a **specified boundary interface** (the seam information
  normalization must retain) plus proofs that normalization is **associative** and
  **representative-independent** (well-defined on classes, not on chosen representatives). Otherwise the
  composite normal form leaks the representative and is not canonical.
- **(C3) Order-independent settlement (operational coherence).** A single-valued budget is not enough;
  the settlement machinery — clawback, quarantine, reservations, mint priority — must produce the **same
  final budget regardless of merge order**. C3 requires the merge operation on budget-state to be
  **associative and commutative** (confluent), so merge order is not an attack surface.
- **(C4) Descent coherence (the cocycle condition).** Pairwise agreement does not imply global
  coherence. A family `{N1, N2, N3}` can satisfy C1 on every pair yet fail on the triple overlap
  `N1 ∩ N2 ∩ N3`: the identity maps compose inconsistently around the loop (`g13 ≠ g23 ∘ g12`), so there
  is no coherent global identity map even though every pair is compatible in isolation. C4 requires the
  identity maps **and** the conserved-object maps to be **path-independent over every finite overlap
  cover**, not merely pairwise — the cocycle condition of descent.

The right formalization is therefore not a sheaf but **descent data**: conserved value is a section of a
stack over the site of namespaces, and objective conservation across a composed family holds iff the
descent data is *effective* — the identity/value cocycle over the full cover splits. The obstruction is
then not "a bad overlap" but a **typed failure of descent**, an element of a (generally nonabelian)
cohomology class: `H1` for the identity-map cocycle, pushed toward `H2` by the conserved-object
coherence. That is strictly better than a scalar "outside the frontier" verdict, because the obstruction
is *typed* — one can say which coherence broke (pairwise C1, triple-overlap C4, associativity or
representative-independence in C2, or merge-order in C3). Where the class is nonzero, the composite is
outside the frontier: a labeled, *typed* empty cell, held to the same discipline — do not launder a
descent failure into a theorem.

Whether descent-effectiveness is decidable is, predictably, partial: for finite covers with
well-behaved, provably-commuting normalizations the cocycle is checkable; in general it inherits the
undecidability of semantic equivalence, because verifying the coherence maps over all overlaps can encode
program equivalence. The compatibility condition does not rescue the boundary; it **types** it and
relocates it into a classified obstruction group, which is the honest outcome.

## The open boundary

Left open, on purpose: **is there a contribution class whose canonical form is computable but genuinely
non-trivial — richer than the rows above, yet still decidable without further attestation?** If yes, the
objective region is strictly larger than we have mapped and each such class is worth its matrix row,
because it is a class a protocol can conserve without a social oracle. If no — if every step past the
listed structural forms already needs attestation — then the objective region is only as wide as
structural canonicalization and everything of real economic interest lives in the anchored region. The
honest guess is that it is partial and monotone: computable canonical forms for structurally-closed
contributions, failing exactly when semantic equivalence enters. But a guess is not a proof, and the
matrix is how it gets earned.
