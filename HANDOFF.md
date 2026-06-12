# Noesis HANDOFF — 2026-06-12 (PRIVATE, stealth)

Resume point for a fresh chat. Detail lives in `CONTINUE.md` (top block) and `ROADMAP.md`;
this is the fast orientation. Repo: `WGlynn/noesis` (private remote). Node: `node/`, Rust.

## Current state
- **node: 119/119 tests green** (`cd node && cargo test`). Last commit `d635dbf`.
- Value layer is comprehensively built + adversarially hardened this session (5 → 119 tests
  in one continuous run via the adversarial-layering method: each layer's surviving attack
  named the next, until the survivor was the consensus layer's own ≥2/3 cross-dimension
  capture ceiling — pinned as a never-flips test).

## The layer stack (all in `node/src/lib.rs`, all tested)
- `value` v4/v5/v6 — realized-flow gate (v5) → priced identity via standing-gated seeds (v6).
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

## Open frontier (honest — these are why the autonomous queue drained)
1. **Real outcome-LABEL data** (DeepFunding-distill-over-sets) — `outcome` model is built;
   real preference data is the unbuilt input. External dependency.
2. **On-VM type-script (ckb-vm)** — the RISC-V validation program. Needs the ckb-vm crate
   APIs verified against the Nervos source; do NOT assume them.
3. **Structured-but-valueless novelty** — the `semantic` floor catches noise, not structured
   pointless content. That genuinely needs labels/flow (out-of-band), not a content gate.
4. **Wire `semantic` into `production_value`** — currently standalone; composing it into the
   canonical value rule is a clean next increment.

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
