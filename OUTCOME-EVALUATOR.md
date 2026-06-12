# Outcome evaluator — role-bounded by construction (Phase-1 core bet, design)

> Status: DESIGN + first increment IMPLEMENTED (2026-06-12): `node/` `evaluator` module
> (bounded intake-advance + reconciliation + corrupt-evaluator bound, tested).
> The learned model itself (training, features, data) is future work; this doc fixes the
> AUTHORITY BOUNDARY first, because that is what makes any learned v(S) safe to deploy.

## 1. The reframe (what v5 taught)

The original Phase-1 plan was "replace the coverage proxy with a learned outcome-value
and prove the learned v(S) preserves strategyproofness." Proving robustness properties
of a learned model is the wrong shape — models drift, attackers probe, proofs rot.

v5 already made the decisive move: quality is not a predicted boost, it is a realized
gate. The composition `floored_novelty × g(realized_flow)` — with v6's standing-gated
seeds and the dispute layer's clawback — is the strategyproof skeleton. So the learned
evaluator is NOT the gate. Its authority is bounded to two roles that cannot mint:

- **Role A — advance timing.** Vesting is slow (W epochs). The evaluator may offer an
  intake ADVANCE against expected vesting — liquidity for honest contributors, repaid
  from realized vesting, shortfall slashed from the contributor's standing at window
  close. The evaluator takes risk, never authority.
- **Role B — inform judgment.** The evaluator's score enters a dispute as EVIDENCE the
  jurors see (DISPUTE-SLASHING §5.1). It never decides; the verdict machinery is
  unchanged.
- **Role C (research) — semantic floor.** A learned semantic-novelty may eventually
  AND-compose with the coverage floor (`min(coverage_novelty, semantic_novelty)`) so
  high-entropy garbage stops being "novel" at intake. AND only — it can zero, never
  rescue. Not implemented; listed for honesty.

## 2. The invariant (why a corrupt evaluator cannot hurt the chain)

For ANY evaluator output — including an adversarial +∞ on garbage:

1. `advance = min(κ · score · floored_novelty, μ · standing)`.
   - Floored novelty multiplies: redundancy (floor 0) gets no advance at any score.
   - Standing caps: a fresh identity gets NO advance at any score (consistent with v6 —
     you can earn from day one, you can't be fronted until you've earned).
2. At window close, advance is repaid from realized vesting; shortfall is slashed from
   standing. With μ ≤ 1 the shortfall is always coverable: the evaluator's worst case is
   a TIMING shift plus a standing transfer, never minted value.
3. Total value paid for a cell never exceeds the v6 skeleton's verdict-surviving vesting.
   The evaluator modulates WHEN, never HOW MUCH.

So the Phase-1 proof obligation collapses from "the learned model is un-gameable" to
"the bounds hold" — three small functions with tests, not a property of a neural net.
Same checker-routing thesis as the harness module: structure where verifiable, learned
judgment only where its failures are bounded.

## 3. Implemented increment (node/, `evaluator` module)

- `intake_advance(score, floored_novelty, standing, κ, μ)` — the double bound.
- `reconcile(advance, vested)` → `(paid_total, standing_slash)` — conservation at close.
- Tests: corrupt evaluator mints nothing (fresh identity 0 at any score; redundancy 0 at
  any score; vested identity leak ≤ μ·standing and recovered by the slash); honest path
  gets early liquidity and no slash; conservation `paid_total ≤ realized vesting` both
  branches.

## 4. Honest open items

- The actual learned model (Bradley-Terry exists in `value::quality_scores`; outcome-set
  labels per the DeepFunding-distill plan) — unbuilt at the evaluator role.
- κ, μ calibration (with W/B/α/β — one calibration harness for the whole dispute stack).
- Role C semantic floor — research; the AND-composition rule is fixed in advance so the
  research cannot drift into a rescue path.
- Advance default-risk when standing is concurrently slashed by another dispute —
  ordering/priority of claims on standing; design before implementing concurrent claims.
