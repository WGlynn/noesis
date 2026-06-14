# Noesis HANDOFF — 2026-06-12 (PRIVATE, stealth)

Resume point for a fresh chat. Detail lives in `CONTINUE.md` (top block) and `ROADMAP.md`;
this is the fast orientation. Repo: `WGlynn/noesis` (private remote). Node: `node/`, Rust.

## Current state
- **node: 203/203 green** (`cd node && cargo test`; lib 178 + integration suites).
- **RSAW tick on the above (2026-06-13):** adversarial edge probe of `finalizes_fixed`
  (horizon=0 no-decay, 100% threshold, zero-weight padding, empty voters, all-zero basis) — the
  conservative direction `!(fixed && !float)` holds at every corner; NO break found, edges pinned
  (`adversarial_edges_hold_conservative_direction`). node 202→203.
- **Latest increment (2026-06-13, full-auto loop):** Phase 3 build-order step 1 — `finalization_fixed`
  module: `consensus::finalizes_hybrid` recomputed in pure Q32.32 (fixed-point retention-decay +
  effective/base weight + max(eff,floor) basis + 2/3 threshold, ceil-rounded against finalization),
  drift-guarded vs the f64 reference over a deterministic liveness×decay×subset sweep — conservative
  direction proven everywhere (`!(fixed && !float)`), agreement off the boundary band, exact-2/3 tie
  stays un-finalized. The 3rd/last on-VM arithmetic surface after value_fixed + settlement_fixed.
  node 197→202. Remaining: the on-VM program + header-`now`/validator-set sourcing.
- **Last increments (2026-06-13, full-auto story-loop):** PM-17 index-dep binding, both
  layers. (1) `index_binding` reference model F2-COMPLETED — dep identity grew a `hash_type`
  field (`HashType{Data,Type,Data1}` + `DepScript`); a forged dep reusing code_hash+type-id
  under a different Data/Type/Data1 is REJECTED (`bound_wrong_hash_type_rejects`, node 196→197).
  (2) on-VM mirror — `main.rs` `index_dep_bound` compares `r.hash_type()` (QA-port-1) and the
  overloaded `[0;32]` sentinel is now an explicit `const BINDING_ACTIVE: bool` (QA-port-2);
  ELF rebuilt, 22 on-VM fixtures green (binding inert). Reference ↔ on-VM now F2-parity. Only
  the activated-path fixture (real deployed script-hash) remains deploy-coupled.
- **Prior increment (pom-roadmap tick `a905048`):** encoding-evasion of the
  semantic seed floor is CLASS-DISSOLVED economically. Hex/zero-dilute noise slips
  `semantic_floor` AND re-opens the v7 seed-gate pump (on encoded bytes v7≡v6), but the
  v6 standing price is byte-blind (fresh-key ring earns 0) and the dispute slash is
  content-agnostic (vested certifier = negative-EV, identically to raw garbage). Chasing
  it at the content layer = case-detection vs the airgap; the economic layer dissolves the
  class. Added three regression cases; ROADMAP + README tier marks updated. Next 🟡: bind
  index-dep by code_hash (CONTINUE.md PM-17).
- Value layer is comprehensively built + adversarially hardened (suite grew from 5 at the
  start of the continuous run to the count above, via the adversarial-layering method: each
  layer's surviving attack named the next, until the survivor was the consensus layer's own
  ≥2/3 cross-dimension capture ceiling — pinned as a never-flips test).

## The layer stack (all in `node/src/lib.rs`, all tested)
- `value` v4/v5/v6/v7 — realized-flow gate (v5) → priced identity via standing-gated seeds
  (v6) → semantic-floored seeds (v7: noise certifies nothing, even vested; own-value airgap
  backstop preserved by flooring the seed, not the cell's own gated value).
- `dispute` — endorsement-slashing: windowed vesting + deterministic causal-share clawback
  + PoM-only 2/3 verdict (reuses `consensus::finalizes_hybrid`) + escalation court (full-mix
  tribunal + juror accountability) closing the judge-cartel veto.
- `soulbound::valid_transition_under_dispute` — exit-block so you can't burn standing to dodge a slash.
- `evaluator` — role-bounded learned-signal consumer (advance timing + dispute evidence,
  never mint; corrupt-evaluator-can't-mint is the load-bearing test).
- `outcome` — learned coalition v(S), Bradley-Terry over set-level structural features.
- `claims` — concurrent-claims settlement (seniority, exposure-freezes-borrowing, pool-eats-deficit).
- `calibration` — feasible-region sweep + soundness guard (refuses to certify p_min<1/2).
- `semantic` — compressibility floor: closes the incompressible-NOISE subclass of garbage-novelty
  at the gate (entropy ≥ θ ⇒ 0), AND-composed, airgap false-positive honestly pinned.

## Open frontier (post story-loop 10/10, 2026-06-12 evening)
1. ✅ DONE 2026-06-12 PM-8/PM-9 — group-input iteration AND T6 mint-side validation
   shipped; both smuggling pins FLIPPED (input index-1 ⇒ 13, minted noise ⇒ 14).
   Execution tier now open at T7 (cross-cell similarity state) + T8 (Q32.32 settlement).
2. **Cross-cell similarity floor on-VM** — needs the seen-shingle state served via a
   Noesis syscall (host exists, tests/common/mod.rs).
3. **Q32.32 settlement mirror** — flow/v5-v7 in fixed point (design in CKB-VM-PORT.md).
4. **Real outcome-LABEL data** (DeepFunding-distill-over-sets) — external dependency.
5. **Structured-but-valueless novelty** — out-of-band (labels/flow); encoding-evasion
   pin (`encoded_noise_evades_the_entropy_floor_open_gap`) folds into this class.

## Infra that now runs itself
- `scripts/doc-coherence.py` (--check/--stamp) + `scripts/study-guide.py` (--check, generates
  STUDY-GUIDE.md from the repo) + `scripts/pre-commit` (installed via `scripts/install-hooks.sh`)
  auto-enforce doc + study-guide freshness on every commit. Caught its own first drift.
- Auto-continue cron `3b8e2f47` (pom-roadmap-advance, every 3h) advances this roadmap one
  increment per fire; keep it private (leak-gate enforces).

## Method (saved as a memory primitive)
`adversarial-layering-self-names-next-layer`: run the adversary against every new v(S) the
moment it lands; the surviving attack is the spec for the next layer; pin each gap as a
passing test; converge when the survivor is the system's irreducible global assumption.
