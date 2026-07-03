---
title: "Authority Lifecycle and Enforcement"
subtitle: "Noesis patent family, Application 2 — DRAFT v1 for Rodney's-AI review"
date: "2 July 2026"
status: "INTERNAL DRAFT — not for filing. Sibling to the Proof-of-Contribution priority application."
---

# AUTHORITY LIFECYCLE AND ENFORCEMENT

> **Family position.** This is Application 2 in the family mapped in the priority filing
> (`10_Patent_Family_Map.md`, candidate family #5, "Authority Lifecycle and Enforcement").
> The priority application (Proof of Contribution) claims the architectural invariant and
> the three implementation families (separation, valuation, finalisation). It *seeds* this
> application in dependent claims 3, 13 and 15. This application develops those seeds into a
> standalone invention: the state machine by which non-transferable consensus authority is
> minted, decays, is slashed, and is destroyed over time, and the enforcement rule that makes
> that lifecycle resistant to evasion.
>
> **Organising principle (inherited, held as interchangeable terminology):** consensus
> authority is protocol-managed state, earned through contribution, independent of
> transferable economic ownership. "Standing", "authority state", "consensus franchise" are
> interchangeable labels; the claims capture the invariant.

## FIELD

Distributed-ledger consensus; specifically, the lifecycle management and enforcement of a
non-transferable, contribution-earned consensus authority represented as ledger state.

## BACKGROUND

Systems that weight consensus by a quantity other than transferable stake must answer a
question that pure proof-of-stake never faces: once authority is *earned* rather than
*bought*, how does it change over time, and what stops a holder from gaming the transitions?

Three problems are specific to earned authority and are not solved by prior art:

1. **Reassignment leakage.** If earned authority can be reassigned, transferred, or have its
   holder rewritten, it collapses back into a tradeable asset and the earned property is lost.
   Forbidding reassignment by policy is weaker than making it *unrepresentable*.

2. **Exit-before-verdict (slash evasion).** Authority earned by contribution is subject to
   revocation when a contribution is later refuted. A holder who anticipates an adverse
   verdict has an incentive to voluntarily destroy or drain the authority before the slash
   lands, escaping the penalty while keeping any prior benefit. A lifecycle that permits
   unconditional voluntary destruction leaves this hole open.

3. **Conflating vote-decay with balance-decay.** Liveness mechanisms decay an inactive
   participant's *influence*. If that decay is implemented by reducing the participant's
   underlying earned quantity, it corrupts the accounting and can be abused. The decay of
   voting weight and the decay of the earned quantity are distinct and must not be conflated.

Prior contribution-weighted consensus systems (for example Bittensor-style measured-value
weighting) do not address a full earned-authority lifecycle with structural anti-evasion. The
present invention does.

## SUMMARY

A consensus authority ("standing") is recorded as a unit of ledger state carrying a soulbound
contributor identity and an accumulated value magnitude. The unit's validity rule admits only
a closed set of identity-preserving operations — accrue, decay, slash, destroy — and the
transition function is constructed so that reassignment of the holder or of the contributor
identity *cannot be expressed*, not merely rejected.

A second, dispute-aware validity rule governs the same lifecycle while a challenge is open.
When an open challenge names a target reachable by a provenance edge from the contributor, the
destroy operation is rejected and the value magnitude may decrease by at most an
authorised-slash amount (zero while the verdict is pending). Voluntary drain disguised as decay
or destruction therefore cannot pre-empt a pending slash. The slash itself is always able to
land. This closes exit-before-verdict structurally rather than by monitoring.

Separately, an effective *voting weight* decays with time since a participant's last heartbeat
by a retention factor applied symmetrically, without reducing the participant's earned
magnitude or staked balance, keeping influence-decay and accounting-decay distinct.

## DETAILED DESCRIPTION

All source pointers are to the public reference node (`WGlynn/noesis`, Rust on a RISC-V VM,
Cell state model). The mechanisms below are implemented and tested unless a status marker says
otherwise. Status discipline: ✅ built, 🟡 designed, 🔬 open — never rounded up.

### The standing unit (✅ built)

A standing cell carries `Standing { contributor: [u8;32], pom: u64 }`
(`node/src/lib.rs:459`). `contributor` is the soulbound identity, invariant across every valid
transition (`:460`). `pom` is the accumulated novelty-value credit, append-only except via
decay or slash (`:462`). Consensus reads `Standing.contributor`, never byte-ownership
(`:452`).

### The closed operation set (✅ built)

The only operations the type script permits are `Op::Accrue(u64)`, `Op::Decay(u64)`,
`Op::Slash(u64)`, and `Op::Burn` (`node/src/lib.rs:469-477`). Anything outside this set is, by
construction, a reassignment, and is rejected (`:466-467`).

- **Accrue** (mint) adds newly-finalised novelty value; the accrual proof is checked by the
  value layer (`:470-471`).
- **Decay** reduces the magnitude per the rent/decay schedule — the supply sink (`:472-473`).
- **Slash** revokes on a proven refutation, in the dispute window (`:474-475`).
- **Burn** (destroy) voluntarily destroys the holder's own standing, producing no successor
  (`:476-477`).

### Reassignment is unrepresentable (✅ built)

`apply(input, st, op)` produces the required successor `(cell, standing)` or `None` for burn
(`node/src/lib.rs:483-491`). The successor is byte-identical in lock and type script; only the
carried `pom` moves; the contributor is copied unchanged. There is no code path by which
`apply` can emit a successor with a different holder or contributor: reassignment "cannot be
expressed" (`:482`). Magnitude changes use saturating arithmetic (`saturating_add`,
`saturating_sub`, `:487-489`), so no transition can underflow or overflow.

`valid_transition(...)` is the on-chain check: a transition is valid iff the output is an
identity-preserving successor — owner unchanged, contributor (type args) unchanged, and the
soulbound identity invariant holds (`node/src/lib.rs:500-515`, predicate `:509-511`). That
rejection *is* the soulbound guarantee (`:494-495`).

### Dispute-aware enforcement — closing exit-before-verdict (✅ built)

`valid_transition_under_dispute(...)` is the canonical check once the dispute layer is live
(`node/src/lib.rs:528-553`). It consults `dispute::standing_exit_blocked(...)` on the
contributor's type-script args (`:537-541`). With no exposure, it defers to `valid_transition`
(`:542-543`). While blocked — i.e. an open challenge names a target this contributor has a
provenance edge into:

- **Burn is rejected** (`(None, None) => false`, `:546`): no torching standing ahead of a
  verdict.
- **Magnitude may decrease by at most `authorized_slash`**: the successor must satisfy
  `out.pom + authorized_slash >= in.pom` (`:547-550`), where `authorized_slash` is the amount
  a closed settlement authorises and is zero while the dispute is pending (`:522-524`).

The slash itself can always land; voluntary drain dressed as decay cannot (`:523-524`). This
is enforcement by construction, not by monitoring: the invalid transitions are unrepresentable
as valid state, so the evasion path does not exist rather than being detected after the fact.

### Vote-weight decay is distinct from magnitude decay (✅ built)

An inactive validator's *effective weight* is reduced by a retention factor that decreases
with elapsed time over a horizon (`retention(elapsed, horizon)`, `node/src/lib.rs:3507`;
Q32.32 mirror `:7893`). This retention reduces the vote weight without reducing the staked
balance, and the validator remains slashable regardless of retention (priority claim 11; test
`decay_touches_vote_weight_not_the_staked_balance`, `node/src/lib.rs:3792`). The rent/decay of
the earned `pom` magnitude (`Op::Decay`) and the retention-decay of voting influence are two
different sinks acting on two different quantities.

### Slash settlement wiring (partially built)

Refutation settlement computes slash targets and shares over the refuted target's provenance
lineage, keyed on the consensus-derived soulbound identity
(`refuted_lineage_identities`, `node/src/lib.rs:4436`; `resolve_refuted`, `:4221`). The
lifecycle op that consumes a settled amount is `Op::Slash` bounded by `authorized_slash`.
**Honest status:** target/share computation and the exit-block are ✅ built; wiring the settled
amount into the dispute path without double-slashing is the deploy-coupled step, 🟡 designed.

### Launch-height neutralisation (🟡 designed)

At a designated launch height, the consensus authority and state value of all units recorded
before the launch height are programmatically reduced to zero while the units are preserved as
auditable history (priority claim 13). This removes any pre-launch head start from the earned
dimension without discarding the audit trail. **Status:** specified in the genesis
constitution, not yet a built transition in the reference node.

### Contributions as training signal (🔬 open)

Contributions valued above a threshold serve as positive training signal and refuted
contributions as negative training signal, to constrain or update the model that produces the
contributions (priority claim 15). **Status:** the learned-value retraining on real
downstream-value labels is the open research mile; not built, not rounded up.

## CLAIMS

**1.** A computer-implemented method of managing a consensus authority on a distributed
ledger, comprising: recording the consensus authority as a unit of ledger state carrying a
contributor identifier and a value magnitude; and constraining transitions of the unit to
identity-preserving operations that cannot reassign a holder or the contributor identifier, by
a transition function that copies the contributor identifier unchanged into any successor unit
and provides no path to emit a successor whose holder or contributor identifier differs from
the input, such that reassignment of the consensus authority is not merely rejected but
unrepresentable.

**1A.** The method of claim 1, wherein the identity-preserving operations comprise operations
that accrue to the value magnitude, reduce the value magnitude, and destroy the unit without
producing a successor.

**1B.** The method of claim 1A, wherein the identity-preserving operations comprise, as the
sole admitted operations, an accrual operation, a decay operation, a slashing operation, and a
destruction operation, any transition outside those operations being, by construction, a
reassignment and rejected.

**2.** The method of claim 1, wherein the transition function computes each successor value
magnitude by saturating integer arithmetic that neither underflows nor overflows, an accrual
operation adding to the magnitude, a decay operation and a slashing operation subtracting from
it, and a destruction operation producing no successor.

**3.** The method of claim 1, further comprising validating a proposed transition only if the
output is an identity-preserving successor of the input in which a holder field and the
contributor identifier are both unchanged, any transition altering either field being
rejected.

**4.** A computer-implemented method of enforcing a consensus-authority lifecycle against
evasion of a pending penalty, comprising: determining whether an open challenge names a target
reachable by a provenance edge from a contributor of a unit of consensus-authority state; and
when such a challenge is open, rejecting any transition that destroys the unit, and admitting a
transition that reduces a value magnitude of the unit only if the magnitude decreases by at
most an authorised amount fixed by a settlement of the challenge, said authorised amount being
zero while the challenge is unresolved; whereby a penalty transition can always be applied but
voluntary destruction or drain cannot pre-empt an unresolved challenge.

**5.** The method of claim 4, wherein, absent any open challenge naming such a target, the
transition is validated by the identity-preserving successor rule of claim 3.

**6.** The method of claim 4, wherein the authorised amount is derived from a settlement that
computes penalty shares over identities on a provenance lineage of a refuted target, each
identity keyed on a soulbound contributor identifier rather than a transferable holder.

**7.** The method of claim 1 or claim 4, further comprising reducing an effective voting weight
of an inactive participant by a retention factor that decreases with elapsed time since a last
heartbeat and is applied symmetrically, without reducing the value magnitude of the consensus
authority and without reducing a staked balance, the participant remaining subject to slashing
irrespective of the retention factor.

**8.** The method of claim 1, wherein the value magnitude decays over time by a decay operation
acting as a supply sink, the decay of the value magnitude being a distinct operation from the
retention-factor decay of voting weight of claim 7.

**9.** The method of claim 1, wherein the unit of ledger state is a cell governed by a lock
script defining a transferable holder and a type script encoding the closed operation set and
the identity-preserving successor rule, the consensus authority being read from the contributor
identifier carried by the type script and never from the holder carried by the lock script.

**10.** The method of claim 1, further comprising, at a designated launch height,
programmatically reducing to zero the value magnitude of every unit recorded before the launch
height while preserving those units as auditable history.

**11.** The method of claim 1 or claim 4, further comprising using contributions of value above
a threshold as positive training signal and refuted contributions as negative training signal
to constrain or update a model that produces the contributions.

**12.** The method of claim 4, wherein the rejecting and admitting steps are performed by a
deterministic validity program executed on a virtual machine, a proposed transition that
violates either step causing the program to return failure and the transaction to be invalid,
such that the evasion path is unrepresentable as valid ledger state rather than detected after
occurrence.

---

*Claim-dependency note (for counsel, not for filing): Claims 1 and 4 are independent and are
each drawn to the structural invariant rather than to any labelled operation set or numeric
constant, so that a design-around adding a fifth operation, renaming a field, or retuning a
constant does not escape the independent claim while practising the invention. Claim 1 is
directed to the unrepresentable-reassignment lifecycle state machine (invariant:
identity-preserving operations that cannot reassign holder or contributor); claim 4 to the
dispute-aware anti-evasion enforcement (invariant: destruction blocked and magnitude bounded by
a settlement-authorised amount that is zero while unresolved). These are the two centres of
gravity of this application and are drawn to be separately defensible. The specific enumerated
four-operation set (accrue, decay, slash, destroy) is held in reserve at dependent claim 1B, with
claim 1A an intermediate narrowing to the three magnitude-and-destruction operation classes.
Claims 1A, 1B, 2, 3, 8, 9, 10 depend on the lifecycle; claims 5, 6, 12 depend on the
enforcement; claims 7 and 11 are cross-cutting. Claim 10 (launch-height) reads on a 🟡-designed
embodiment and claim 11 (training signal) on a 🔬-open embodiment; both are supported for
claiming but should be marked in the internal status register. Multiple-dependency claims are in
the alternative to suit UK practice.*

## ABSTRACT

A consensus authority earned by contribution is recorded as ledger state carrying a soulbound
contributor identifier and a value magnitude, and is confined to a closed set of
identity-preserving transitions — accrual, decay, slashing, destruction — by a transition
function that cannot express reassignment of the holder or contributor, making the earned
property structural rather than policy-enforced. A dispute-aware validity rule closes
exit-before-verdict: while a challenge naming a provenance-connected target is open, voluntary
destruction is rejected and the magnitude may fall only by an amount a settlement authorises
(zero while pending), so a penalty always lands but cannot be evaded. Voting-weight decay by an
inactivity retention factor is kept distinct from decay of the earned magnitude and of staked
balance.
