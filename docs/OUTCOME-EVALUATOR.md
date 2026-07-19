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

## 3b. The learned model — first increment (node/, `outcome` module, 2026-06-12)

Built: a coalition-level outcome model. `coalition_features(S)` extracts SET-level
structural features the per-block coverage proxy cannot see — breadth, synergy
(union/Σindividual), internal connectedness (fraction of S whose parent is also in S),
and lineage depth. `train()` is Bradley-Terry over PAIRWISE coalition preferences (the
outcome labels — the outside signal); `v_outcome(S) ∈ [0,1]`. Tested: the features
separate a connected/synergistic coalition from orphaned high-entropy garbage (which the
content proxy ranks similarly); the model learns a label ordering the proxy cannot
express; it generalizes to an unseen connected coalition; and — the load-bearing test —
an absurd/corrupt weight vector routed through `evaluator::intake_advance` on a fresh
identity still yields 0. The learned signal is safe BECAUSE of the authority boundary,
not a robustness proof.

Honest scope: this informs the bounded evaluator (advance + dispute evidence); it does
NOT replace the strategyproof gate and does NOT auto-close
`garbage_novelty_is_the_documented_open_gap` at the v_4/v_5 gate (that pin is about the
coverage proxy inside the gate). Garbage-novelty is now ADDRESSED out-of-band — a
labelled outcome model can decline to advance it and can testify against it in a dispute
— without ever being able to mint it. Closing the gap AT the gate (AND-composing a
learned semantic floor, Role C) remains research.

## 4. Honest open items

- **Held-out generalization harness — SHIPPED (2026-06-13, `outcome` module).** The
  measurement plumbing now exists: `proxy_value` (the coverage-only baseline the per-block rule
  sees) + `pairwise_accuracy` (the held-out ranking metric) + the test
  `learned_v_s_beats_coverage_proxy_on_held_out_coalitions`. It trains Bradley-Terry on one split
  and measures ranking accuracy on coalitions NEVER seen in training: at identical coverage, a
  connected work-built-on-work coalition vs the same cells dumped as orphans. On this **structural
  lineage stand-in** the learned `v(S)` ranks the unseen pairs ≥ 0.9 while the coverage proxy ties
  at 0.5. **Honest scope (load-bearing):** this is the harness exercising a SYNTHETIC lineage
  signal — it shows the plumbing separates connected from orphaned sets, NOT that the learned
  measure beats a fixed proxy on real outcomes. It is not the moat demonstrated.
- **First real-data test — NULL (DeepFunding).** When the same learned `v(S)` was run against
  real DeepFunding jury labels, it did **not** reliably beat the best fixed structural proxy
  (mean delta +0.0021 over 20 seeds, wins 11/20, both ~0.56 vs a 0.50 floor). The "learned measure
  beats a fixed proxy / closes the Goodhart gap" claim is therefore **unsupported, NOT refuted**:
  the test used single-repo PROXY features over a DEPENDENCY graph, not the set-level features over
  a PROVENANCE DAG this harness scores; the `load_prefs` data seam IS validated end-to-end.
  **Update (`data/deepfunding/RESULTS-RICH-JUDGE.md`):** a *rich-feature* judge (repo
  popularity/age/funding, not structural features) predicts the same jury prefs at **0.68** held-out,
  so the null is specific to *structural* features — but that is popularity-heavy prediction on
  *honest* labels, still not the *adversarial* un-gameability the moat claims. No doc
  may state the moat / un-gameability / Goodhart-closure as demonstrated. Adaptive-stability
  (HCE property 3) is DESIGNED / CONJECTURE, with this note attached. See the status ledger.
- Outcome-set LABELS at scale, faithfully ported (the model + the held-out harness are built; the
  remaining mile is real preference data — DeepFunding-distill-over-sets — feeding the SET-level
  features over a true provenance DAG, replacing both the structural lineage stand-in AND the
  single-repo dependency-graph proxy the null test used). The faithful feature port is the open
  real test; the harness runs unchanged the moment those labels land.
- κ, μ calibration (with W/B/α/β — one calibration harness for the whole dispute stack).
- Role C semantic floor — research; the AND-composition rule is fixed in advance so the
  research cannot drift into a rescue path.
- ~~Advance default-risk under concurrent claims~~ RESOLVED (2026-06-12, `claims`
  module): claim PRIORITY = restitution-to-others > advance-shortfall > decay
  (seniority is the enum order, settlement input-order-independent); EXPOSURE FREEZES
  BORROWING (the `standing_exit_blocked` predicate gates new advances — collateral a
  live dispute may claim cannot be double-pledged); DEFICITS land on the advance pool
  (Role A's risk), never on honest third parties; standing never negative. Tested.
