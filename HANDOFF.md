# Noesis HANDOFF — 2026-06-12 (PRIVATE, stealth)

Resume point for a fresh chat. Detail lives in `CONTINUE.md` (top block) and `ROADMAP.md`;
this is the fast orientation. Repo: `WGlynn/noesis` (private remote). Node: `node/`, Rust.

## Current state
- **node: 146/146 tests green (133 lib + 13 ckb-vm integration)** (`cd node && cargo test`).
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
1. ✅ DONE 2026-06-12 PM-8 — group-input iteration shipped; smuggling pin FLIPPED.
   New survivor (ROADMAP T6): group OUTPUTS not validated — mint-side noise passes;
   host must serve Source::GroupOutput, program iterates both directions.
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
