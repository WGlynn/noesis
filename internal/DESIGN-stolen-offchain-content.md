# DESIGN / THREAT — stolen off-chain content (first-ingestion authenticity)

> Attack vector raised by Will 2026-06-23. The purest form of the airgap at the content boundary
> (the oracle problem). Honest scope: NOT fully closable on-chain; the design shrinks the residual to
> "uncommitted off-chain content" and makes the attack negative-EV + recoverable. Design note, not yet
> a build; the commitment-priority defense rides existing temporal-order machinery.

## The attack
An attacker ingests content created by someone ELSE who never put it on-chain. The attacker is credited
as the contributor and captures the realized-downstream-flow value; the genuine off-chain creator gets
nothing. The victim is OFF-chain and casts no shadow on the provenance graph ⇒ outside the measurement
horizon `K` (topology-shadow test, cybernetics paper §6). The chain CANNOT, from on-chain data alone,
distinguish the real creator from a first-ingesting thief. This is the airgap; do not pretend otherwise.

## The sharp finding — a core defense pulls FOR the attacker
The novelty gate (SMT cross-cell similarity; first-commit-wins; duplicate damping) exists to defeat spam
and re-posting. But it anchors priority to FIRST INGESTION: if the thief ingests first and the real
creator later posts the same work, the CREATOR's post is flagged as the duplicate and damped to ~0. So
the anti-spam defense and the theft vector point the same way — first-ingestion wins — and the defense
entrenches the theft. This is the load-bearing danger in the vector, not mere lack of coverage.

## The structural fix — priority = earliest COMMITMENT, not first INGESTION
Re-anchor authorship priority to the earliest cheap CRYPTOGRAPHIC COMMITMENT (a 32-byte hash of the work
posted at CREATION), not to first full on-chain ingestion.
- Pulls the problem from OUTSIDE `K` to INSIDE it: the chain already measures temporal order objectively
  (consensus-sourced commit-order / first-commit-wins, `noesis_core::commit_order`,
  `TEMPORAL-ORDER-ONCHAIN.md`). "Who authored it" becomes ordering two commitments the chain already
  orders honestly — decided NOT by reading content but by commit-height.
- A creator who hash-commits AT CREATION is provably first: no one else had the content yet to commit it.
  The thief, who only sees the work at publication, can at best commit SECOND.
- Cost structure is right: commit one hash (near-free, automate on save), reveal only if challenged.
- **Also fixes the novelty-gate tension:** priority by earliest commitment ⇒ the creator's earlier
  commitment beats the thief's earlier full-ingestion ⇒ the duplicate-damp falls on the THIEF's instance,
  not the creator's. The two defenses re-align.

## Honest residual (does NOT close)
1. **Uncommitted content** — a creator who made NO on-chain commitment has no priority anchor ⇒ not
   parametrically protectable (the irreducible airgap). Recourse = bonded court with off-chain evidence
   (prior publication / signature / git history predating ingestion) + clawback over the contestability
   window. Judgment-tier, correctly (origin is not on-chain-objective).
2. **Content-bound vs identity-bound value** — identity-bound value partly SELF-HEALS (once the real
   creator is on-chain, downstream re-attributes to their identity); content-bound value (a theorem, a
   dataset) from an ABSENT creator is the exposed case.

## Scorecard
- creator commits-on-create ⇒ **structurally defeated** (priority on-chain-objective; novelty tension fixed).
- uncommitted creator ⇒ **not parametric** (airgap) ⇒ court + clawback; negative-EV over the window, real residual.
- design job = shrink residual + negative-EV + recoverable, NOT claim origin-omniscience.

## Build implications (future, fresh)
- An **authorship pre-commitment cell**: a creator posts `H(work ‖ salt)` at creation; reveal binds the
  work to the earliest commit-height. Priority in the novelty/attribution gate = earliest REVEALED
  commitment, not first ingestion. Rides `commit_order` temporal ordering.
- Threat-model row in the whitepaper: "stolen off-chain content" → `designed (commitment-priority); residual
  = uncommitted content → court + clawback`. Mark demonstrated-vs-designed honestly.
- 🔬 optional, NON-structural: an originality oracle (similarity vs an external corpus of the creator's prior
  works) — an ORACLE, not a structural close; name it as such if ever pursued.

## Related
`DESIGN-parametric-clawback.md` (the recovery/court tiers this residual uses) · `TEMPORAL-ORDER-ONCHAIN.md`
(the priority machinery) · `T7-CROSS-CELL-SIMILARITY.md` (the novelty gate this re-aligns) ·
[[honesty-as-structural-load-bearing-property]] · the cybernetics paper §6 (the `K` horizon).
