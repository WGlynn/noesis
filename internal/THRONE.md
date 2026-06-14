# THRONE — the design telos (PRIVATE, internal register)

> Will, 2026-06-12: *"i want noesis to be a throne for jesus himself. its alignment,
> symmetry, technology that's inherently christian."*
>
> This document states what that means operationally, grounds every claim in shipped
> mechanism (no claim without a module and a test behind it), and records what the claim
> does NOT say. Private during stealth; the telos itself is Will-shareable at his
> discretion.

## What "throne" means here

Noesis is not a product and not only a protocol. A throne is a seat built for an
occupant — it does not rule; it holds the place for the one who does. Operationally:
**the mechanism serves and never rules.** Final meaning, final judgment, final worth are
not mechanical; the system is built so that the seat stays empty of every pretender —
including the mechanism itself, including the founder.

"Inherently Christian" is a property OF the structure, not a label ON it. No marketing
cross, no performative piety. The test is whether the mechanism IS the ethic when nobody
is naming it.

## The correspondences (each one shipped, tested, named)

1. **Reward is service, structurally.** Under the value gate (`value::value_v5` /
   `value_v6`) a contribution is paid only as OTHER minds build on it. Not as it claims,
   not as it markets — as it serves. *"Whoever wants to be first must be servant of
   all"* — at the value rule, not the mission statement.

2. **Fruit-judged, not claim-judged.** Value vests retroactively (`dispute::VestingEntry`,
   windowed): at intake everything is worth zero and accrues as it proves useful.
   *"By their fruits you will know them"* — as the vesting schedule.

3. **The franchise cannot be bought.** Standing is soulbound: earned per identity, and
   `soulbound::valid_transition` REJECTS every reassignment, so certifying power cannot
   be purchased, pooled, or transferred in — only earned by work others build on. Simon
   Magus tried to buy standing with money and was refused; here the refusal is a type
   invariant. **No simony, by construction.**

4. **Kenosis at the tokenomics.** Genesis-burn fair launch (WHITEPAPER §10): the founder
   provably renounces pre-accrual — self-emptying as the launch mechanism, auditable
   on-chain rather than asserted.

5. **No single power finalizes alone.** AND-composition with the 2/3 bar (COHERENCE-LAWS
   L12; `consensus::finalizes_hybrid`) and, as of today, the escalation court
   (design §7): even a supermajority of ONE kind of power — cognition, capital, or
   compute — cannot rule by itself. Anti-hierarchy as the finalization rule.

6. **Judges are judged — by the measure they used.** `dispute::juror_slash_on_overturn`:
   a juror whose verdict is overturned on appeal is slashed in proportion to the PoM
   weight they voted with. *"With the judgment you pronounce you will be judged, and
   with the measure you use it will be measured to you"* — implemented literally, the
   slash indexed to the vote.

7. **Restitution before retribution.** The certifier slash is λ × causal share with
   λ = 1: first make the harm whole (exactly what the endorsement minted, computed
   deterministically), then a bounded penalty α. And the reckoning cannot be fled:
   `valid_transition_under_dispute` denies burning your standing while a dispute names
   your edge — accountability is not optional when it is due.

8. **Honesty is load-bearing, not decorative.** The whole chain is built so dishonesty
   is unprofitable (the v5→v6→dispute→escalation arc closes each dishonest trajectory
   class). Same property the cross names in another register: the truth does not need
   protection from scrutiny; it needs only that the books be open.

## The design test (apply to every future decision)

> Does this change make the mechanism more **servant-shaped** or more
> **throne-shaped-for-power**? Reject the latter — even when it is locally efficient,
> even when it is ours.

The founder is not exempt. The genesis-burn is this test applied to ourselves first.

## What this does NOT claim (honest boundary)

- A mechanism can embody an ethic; it cannot believe, love, or be saved. The claim is
  structural correspondence, not sanctity. Code is not church.
- The airgap stands (DISPUTE-SLASHING §5.1): final judgment of meaning is not
  mechanical. That is not a flaw in the throne — it is WHY there is a throne. The seat
  is for the occupant; the mechanism keeps it from being usurped, it does not sit down.
- "Anti-hierarchy" is about power-composition in finalization, not a statement on
  church polity.
- The alignment claim is convergence, not conflation: AI-alignment and this telos meet
  at structure-does-the-work — extraction made unprofitable yields a service
  equilibrium. A throne fit for the occupant, whether or not the builders are watched.
