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
- **Validator-set provenance — now a SPECIFIED requirement, not just an open tension.** Today's
  reference pin (`validator_set_is_outcome_determining_so_must_be_consensus_bound`, node 196) proves
  the `all` set is outcome-determining: a producer who supplies a CURATED `all` omitting honest
  validators shrinks the basis (and the quorum floor's `base_total`) until a minority clears it. So
  the ELF MUST bind `all` to the canonical validator-registry — type-id singleton + identity, the
  SAME mechanism as `INDEX-DEP-CODEHASH-BINDING.md` — and RE-DERIVE it from that bound cell, never
  the witness. A caller-supplied `all` is rejected. (Same re-derive-and-reject rule as the temporal-
  order coords: "comes from consensus" is only real if the ELF refuses any input it can't reconstruct.)
- **Quorum floor** uses `base_weight` (un-decayed) — carry both base and effective sums; the
  floor must not itself be decayable or low-participation epochs become un-finalizable by design
  (which may be intended; flag for the consensus review).

## Build order (fresh session)
1. ✅ **DONE 2026-06-13** — `finalization_fixed` module: `finalizes_fixed` (Q32.32) + fixed-point
   `retention_q` / `effective_weight_q` / `base_weight_q`, drift-guarded vs `finalizes_hybrid`
   over a deterministic fixture sweep (liveness × decay-mode × voter-subset). The threshold and
   quorum floor are evaluated with a single ceil (`bps_of_ceil`) so rounding is AGAINST
   finalization; the sweep asserts (a) agreement away from the boundary band and (b) the
   conservative direction everywhere — `!(fixed && !float)`, the fixed rule NEVER finalizes a
   case the real-valued rule rejects — plus a constructed exact-2/3 tie that stays un-finalized.
   `retention_q` matches `consensus::retention` to <1e-9. node 197→202. STILL `now`/validator-set
   sourcing is the on-VM step (below), not yet wired — this is the arithmetic core only.
2. ✅ **DONE 2026-06-13** — On-VM program `onchain/finalization-typescript` (riscv64imac ELF,
   153KB, first-try compile). Reads the validator set + params from the finalization cell
   (GroupInput, wire format single-sourced in `noesis_core::finalization`), the vote index list
   from `witness[i]`, and **`now` from the block header** (`load_header` HeaderDep 0, the u64
   `RawHeader.timestamp` at byte offset 8 — NO witness/arg channel exists for it). Recomputes
   `finalizes_fixed` and exits: 0 finalizes · 30 below threshold · 31 malformed cell/empty group ·
   32 malformed votes · 33 header missing · 34 registry unbound (sentinel-gated inert). Iterates
   the whole GroupInput so a second cell can't smuggle a false claim (the intake-script lesson).
3. ✅ **DONE 2026-06-13** — `node/tests/ckb_vm_finalization.rs` (9 finalization cases, all green; node 203→212).
   Covers finalizes / below-threshold / **`now_is_header_sourced_not_tx_chosen`** (the SAME cell+
   votes flips 0→30 purely by changing the header timestamp, via the un-decayed quorum floor) /
   missing-header-33 / quorum-floor-vs-reference sweep (on-VM exit ≡ `finalizes_fixed` at every
   now) / malformed-cell-31 / malformed-votes-32 / empty-group / second-cell-smuggle. The harness
   gained header serving (`SYS_LOAD_HEADER` + `header_with_timestamp`).
   **STILL PENDING (honest, deploy-coupled):** the **curated-validator-set-rejected** fixture — the
   registry binding is `REGISTRY_BINDING_ACTIVE = false` (inert pre-deploy), exactly like the
   index-dep F1/F2/F3 activated path; it lands when the validator-registry cell deploys. Also a
   header-recency binding (assert the header-dep is the tip via `since`) is a follow-on; today the
   timestamp is consensus-bound but staleness of the chosen header isn't yet pinned.

## Composition
Consensus layer atop the value layer. This doc holds two of the SEVEN sites of the recurring
principle `[P·dont-let-attacker-choose-critical-input]` — finalization-`now` (header-sourced) and
the validator-set `all` (registry-bound) — joining index code_hash, temporal-order, index-dep, and
the two commit-order/coord sites in `TEMPORAL-ORDER-ONCHAIN.md`. The on-VM rule is uniform across all
seven: the ELF re-derives every security-critical input from consensus and rejects anything it cannot
reconstruct. Pairs with `CONSENSUS-REVIEW.md` (the NCI verification table) and the T8 settlement mirror.
