# On-VM PoM-weighted finalization (PRIVATE) — design, ready to build

> Phase 3, the execution-tier step after T8 ("on-VM finalization next"). Ports the
> reference model `consensus::finalizes_hybrid` (node/src/lib.rs) to run inside the VM.
> Spec only; implementation deferred to a budgeted session (Q32.32 mirror + ELF + fixtures).

## The reference model (what we are porting)
`finalizes_hybrid(voters_for, all, mix, now, horizon, decay_pos, threshold_bps, quorum_floor_bps)`:
- `weight_for = Σ effective_weight(v)` over voters_for
- `eff_total  = Σ effective_weight(v)` over all; `base_total = Σ base_weight(v)`
- `floor = base_total · quorum_floor_bps / BPS`; `basis = max(eff_total, floor)`
- finalize ⇔ `basis > 0 ∧ weight_for ≥ basis · threshold_bps / BPS`

`effective_weight` mixes PoW/PoS/PoM per `Mix` and applies PoS retention-decay over
`horizon` relative to `now`. All f64 today.

## What the on-VM port must do
A finalization is decided by a type-script over a "finalization cell" whose data carries the
validator set and the vote set; the script recomputes the inequality on-VM and accepts/rejects.

1. **Fixed-point mirror (Q32.32), like T8 `settlement_fixed`.** `effective_weight` and the
   threshold comparison must be integer arithmetic, bit-identical to a node reference, no
   floats on-VM. Reuse the Q32.32 infra already built for the settlement mirror; add a
   fixed-point retention-decay (the only non-trivial term — a bounded geometric/linear decay
   in Q32.32, drift-guarded against the f64 reference within the same 1e-6 envelope T8 uses).
2. **Cell/witness layout.** Finalization cell data = the validator set, each as
   `(id, pow_q, pos_q, pom_q, last_heartbeat)` in fixed-point, plus params
   `(mix, threshold_bps, quorum_floor_bps, horizon)`. The vote set (`voters_for`) rides as a
   bitmap/index list in the witness. The program sums `weight_for` and `basis` and asserts the
   inequality; distinct exit code on "not finalized" vs "malformed input".
3. **`now` MUST be consensus-sourced, not tx-chosen (the adversarial point).** `effective_weight`
   depends on `now` through PoS decay. If `now` is a free witness/arg field, an attacker picks
   the `now` that maximizes their voters' weight (decays opponents, not themselves) and forges
   finalization. So `now` must come from a trusted on-chain source — the block/header timestamp
   (`load_header` → since/epoch), or the tx `since` field bound to the header — never a value
   the tx assembler chooses. This is the SAME lesson as the index-dep binding F1: never let the
   attacker choose a security-critical input. Pin it as the first finalization fixture
   (tx-chosen-now is rejected / ignored in favor of the header).
4. **Determinism + drift guard.** A node `finalizes_fixed` reference mirrors `finalizes_hybrid`
   in Q32.32; node carries a drift-guard test (`finalizes_fixed ≡ finalizes_hybrid` across a
   fixture sweep), exactly as `noesis-core` is guarded against the lib. The on-VM program calls
   the same fixed-point core.

## Honest tensions / open
- **PoS decay in fixed point** is the one place precision matters; calibrate the Q32.32 decay
  so a borderline 2/3 vote never flips between f64 and fixed (the band the T8 mirror already
  characterizes). If it can flip at the boundary, the threshold must be evaluated with a
  documented rounding direction (round-against-finalization = conservative: a tie does NOT
  finalize).
- **Validator-set provenance.** The finalization cell asserts a validator set; binding that the
  set IS the canonical one is the same class as the index-dep binding (type-id singleton +
  identity). Compose with `INDEX-DEP-CODEHASH-BINDING.md` rather than re-deriving.
- **Quorum floor** uses `base_weight` (un-decayed) — carry both base and effective sums; the
  floor must not itself be decayable or low-participation epochs become un-finalizable by design
  (which may be intended; flag for the consensus review).

## Build order (fresh session)
1. `finalizes_fixed` (Q32.32) in node + drift-guard vs `finalizes_hybrid` over a fixture sweep.
2. On-VM program: read validator set + votes + header `now`; recompute; exit codes.
3. Fixtures: finalizes / does-not-finalize / quorum-floor-binding / **tx-chosen-now-rejected**
   / fixed-vs-f64-boundary. ELF rebuild + recopy.

## Composition
Consensus layer atop the value layer; the recurring principle (do not let the attacker choose
the security-critical input — index code_hash there, `now`/validator-set here) is the same one.
Pairs with `CONSENSUS-REVIEW.md` (the NCI verification table) and the T8 settlement mirror.
