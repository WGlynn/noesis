# NOESIS — CLAIM SET (DRAFT, PRIVATE — for Rodney review, NOT legal advice)

> Companion to `UKIPO-FILING.md` (the to-file description). Every limitation below traces to a
> passage in that description — claims added within 12 months must be supported by it.
>
> **Order: C → B → A** (Will's call, 2026-06-27). Independent Claim 1 = hybrid eclipse-resistant
> finalisation (the strongest, least pre-empted, most clearly *technical* claim — Rodney's read
> and ours). Then strategyproof valuation. Then non-transferable standing. Combination claim last.
>
> **Prior-art narrowing applied** (`docs/research/RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md`):
> - Bare "measured value = the consensus object" is **pre-empted** (Bittensor Yuma; Fortytwo
>   arXiv:2510.24801). Independent claims are narrowed to the *specific* differentiated mechanisms,
>   never the bare version.
> - HodgeRank-residual-as-manipulation-certificate is **partially pre-empted** (DPoR
>   arXiv:1912.04065) AND is not in the filed description → **no claim drafted to it.**
> - Temporal-novelty commit-reveal kept specific (first-to-cover coverage count over a commit-reveal
>   canonical order), not a broad "commit-reveal for ordering" claim.

---

## INDEPENDENT CLAIM 1 — Hybrid eclipse-resistant weighted finalisation (Family C)

1. A computer-implemented method of finalising a proposed block in a distributed ledger
   maintained by a plurality of validators, the method comprising, at a validating node:

   (a) computing, for each validator, a **base weight** as a weighted sum across a plurality of
       independent consensus dimensions;

   (b) computing, for each validator, an **effective weight** by multiplying the base weight by a
       retention factor that decreases with the elapsed time since that validator's most recent
       heartbeat, the retention factor being **applied symmetrically across all of said
       dimensions** such that the relative mixture of dimensions is invariant to said elapsed time;

   (c) computing a **finalisation basis** as the greater of (i) the sum of the effective weights of
       the validators and (ii) a **quorum floor** equal to a fixed fraction of the sum of the base
       weights of the validators; and

   (d) finalising the proposed block only when the sum of the effective weights of the validators
       supporting it is at least a supermajority fraction of said finalisation basis;

   whereby the symmetric retention factor prevents any single non-decaying dimension from coming to
   dominate the consensus weight under validator inactivity, the use of effective weight in the
   basis prevents the ledger from halting under low participation, and the quorum floor prevents a
   party that reduces the apparent set of present validators from finalising the block with a
   sub-floor coalition.

### Dependent claims on Claim 1

2. The method of claim 1, wherein the retention factor reduces the validator's vote weight but not
   an underlying staked balance, and the validator remains subject to slashing irrespective of its
   retention factor.

3. The method of claim 1, wherein said elapsed time is measured against a time value sourced from a
   consensus-bound block header, and a time value supplied by the assembler of the finalising
   transaction is rejected.

4. The method of claim 1, wherein the set of validators and the finalisation parameters, including
   the supermajority fraction, the quorum-floor fraction, the dimension mixture, and the staleness
   horizon, are re-derived from a consensus-bound validator registry, and any such set or parameter
   supplied by the assembler of the finalising transaction is rejected.

5. The method of claim 1, wherein a repeated vote from a single validator is rejected such that
   that validator's effective weight is counted at most once.

6. The method of claim 1, wherein the plurality of dimensions is **composed conjunctively** such
   that finalisation requires the supporting weight to reach the supermajority fraction in each said
   dimension independently, and no single dimension whose supporting weight is below the
   supermajority fraction finalises the block alone.

7. The method of claim 1, wherein the allocation of weight across validators is constrained to lie
   in the core of the validator coalition game, or, when the core is empty, to minimise the maximum
   coalition excess, such that no coalition of validators increases its payoff by deviating.

---

## INDEPENDENT CLAIM 8 — Strategyproof valuation by temporal novelty (Family B)

8. A computer-implemented method of assigning a manipulation-resistant value to a contribution in a
   distributed contribution ledger, the method comprising:

   (a) binding authorship and ordering of contributions by a **commit-reveal** procedure in which a
       producer publishes a signed, timestamped cryptographic hash of a contribution concatenated
       with a secret before revealing the contribution, failure to reveal a valid contribution
       matching a published commitment being a slashable event, the procedure supplying a
       **canonical order** of contributions;

   (b) mapping a contribution payload to a set of **content elements** by a coverage function; and

   (c) assigning to the contribution a value equal to the count of content elements in its coverage
       that are **absent from the union of the coverage of all contributions earlier in the
       canonical order**;

   whereby a contribution that duplicates, is a subset of, or recombines content from earlier
   contributions across one or more identities contributes no new content elements and is assigned
   zero value, while genuinely novel content retains its value.

### Dependent claims on Claim 8

9. The method of claim 8, wherein the coverage function maps the payload to a set of fixed-length
   shingles of the payload.

10. The method of claim 8, wherein the assigned value is said count multiplied by one plus a learned
    quality score bounded to a unit interval, such that a contribution of zero novelty is assigned
    zero value at any quality score.

11. The method of claim 8, further comprising assigning a value of zero to a contribution whose
    coverage overlaps the union of the coverage of earlier-ordered contributions above a similarity
    threshold, such that a near-duplicate that alters only a few content elements to escape
    exact-subset detection is assigned no value.

12. The method of claim 8, wherein the contribution records the inputs from which its payload was
    produced such that production is reproducible, and a derived deployment either preserves the
    resulting provenance graph or, on removing it, causes the valuation to cease to function.

---

## INDEPENDENT CLAIM 13 — Earned, non-transferable consensus standing by a two-unit mint (Family A)

13. A computer-implemented method of maintaining consensus standing in a distributed ledger whose
    state is partitioned into independent units, the method comprising:

    (a) responsive to a verified contribution, minting **two units of state**: a **capacity unit**
        representing storage and a **standing unit** representing consensus franchise;

    (b) treating the capacity unit as **transferable**, its current holder being derivable by folding
        a signed transfer log over an origin holder such that there is no mutable ownership table;

    (c) enforcing on the standing unit a **non-transferability invariant** by a state-transition
        validity rule that admits only a closed set of identity-preserving successor transitions —
        being accrual of newly finalised value, decay under a rent schedule, slashing on a proven
        refutation, and voluntary destruction — and rejects any successor transition that reassigns
        the holder or the contributor identifier of the standing unit; and

    (d) deriving a validator's consensus weight from the contributor identifier of standing units and
        never from the holder of any unit;

    whereby storage capacity is liquid and transferable while consensus standing is earned and can
    neither be transferred, sold, nor bought.

### Dependent claims on Claim 13

14. The method of claim 13, wherein each unit of state is a cell comprising a lock script defining a
    current transferable holder and a type script encoding the state-transition validity rule and
    holding the contributor identifier and a program hash, and a transition is valid only if the
    type script returns success.

15. The method of claim 13, wherein the value accruing to a standing unit, where the corresponding
    contribution has a plurality of contributors, is divided among them by a **graph-restricted
    cooperative-game value** computed over the contributors, with value propagating backward along
    provenance edges under a damping factor strictly less than one that both converges and bounds
    self-referential attribution loops.

16. The method of claim 13, further comprising, at a designated launch height, programmatically
    reducing to zero the consensus standing and state value of all units recorded before the launch
    height while preserving those units as auditable history.

17. The method of claim 13, wherein the value-weighted, signed record of contributions is used as a
    training signal — contributions of value above a threshold as positive signal and refuted
    contributions as negative signal — to constrain or update a model that produces contributions.

---

## CATEGORY CLAIMS (system + medium)

18. A distributed-ledger system comprising a plurality of validating nodes, each comprising one or
    more processors and memory storing instructions that, when executed, cause the node to perform
    the method of any of claims 1, 8, or 13.

19. A non-transitory computer-readable medium storing instructions that, when executed by one or
    more processors of a validating node, cause the node to perform the method of any of claims 1,
    8, or 13.

20. The system of claim 18, wherein the method of claim 1, the method of claim 8, and the method of
    claim 13 are performed in combination — the consensus weight of claim 1 being derived from the
    standing units of claim 13, and the value accrued to said standing units being the value
    assigned by claim 8 — such that temporal-novelty valuation, earned non-transferable standing,
    and hybrid eclipse-resistant finalisation operate within a single unit-and-script ledger
    architecture.

> **Claim 20 is the inventive-concept combination claim.** Our novelty audit names exactly this
> as the defensible position: even if an examiner narrows 1/8/13 individually against prior art,
> the *specific combination* within one unit-and-script ledger is held by none of the surveyed
> prior art.

---

## INVENTORSHIP, OWNERSHIP, AND FILING ORDER

- **Inventorship:** Will Glynn, sole inventor. This is a statement of fact (who conceived the
  invention), not a property right. It is recorded so the patent is valid; it confers no ownership.
- **Ownership:** assigned in full to Rodney. Will retains no rights, no royalty, no interest.
  Papered by a one-page inventor-to-applicant assignment executed before filing. Will named as
  inventor + owning nothing is a normal, clean configuration.
- **Why Will is still named:** naming the true inventor, and only the true inventor, is what keeps
  the patent enforceable. Omitting the actual inventor, or naming a non-inventor, is a validity
  defect a challenger can exploit — and that loss would fall on Rodney, the owner. Naming Will is
  therefore protection of Rodney's asset, not a claim against it.
- **⚠ FILING-ORDER GATE — independent of ownership, do not skip:** the foreign-filing-license
  requirement keys off the invention being **US-made by a US inventor**, NOT off who owns it.
  Assigning everything to Rodney does **not** remove it. Filing at UKIPO first, with a US inventor
  and no license, (i) bars the eventual US patent — Rodney's to lose — and (ii) is itself a
  violation for a US-made invention. **Hold UK submission until a US provisional is filed (this
  auto-triggers FFL review and secures the same priority date) OR a standalone FFL is granted.**
  Petition drafted: `US-FFL-PETITION.md` / `FFL-petition-package.md`.

## ADVERSARIAL QA — see `CLAIMS-QA-2026-06-27.md` (run before sending to Rodney)
