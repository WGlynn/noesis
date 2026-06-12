# Noesis HANDOFF — 2026-06-12 (PRIVATE, stealth)

Resume point for a fresh chat. Detail lives in `CONTINUE.md` (top block) and `ROADMAP.md`;
this is the fast orientation. Repo: `WGlynn/noesis` (private remote). Node: `node/`, Rust.

## Current state
- **node: 133/133 tests green** (`cd node && cargo test`).
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

## Open frontier (honest — these are why the autonomous queue drained)
1. **Real outcome-LABEL data** (DeepFunding-distill-over-sets) — `outcome` model is built;
   real preference data is the unbuilt input. External dependency.
2. **On-VM type-script (ckb-vm)** — the RISC-V validation program. Needs the ckb-vm crate
   APIs verified against the Nervos source; do NOT assume them. Port-time requirement
   (critical-qa 2026-06-12): the value layer is f64 throughout — on-VM consensus code needs
   fixed-point/deterministic arithmetic, plan the conversion as part of this item.
3. **Structured-but-valueless novelty** — the `semantic` floor catches noise, not structured
   pointless content. That genuinely needs labels/flow (out-of-band), not a content gate.
4. ✅ DONE (2026-06-12 PM-6) — `semantic` wired into `production_value`: AND-composed after
   the similarity floor, before quality (`entropy_theta` param). Noise zeroed at the canonical
   rule even at max quality (contrast in-test: similarity floor alone still pays it); airgap
   pin propagated. Follow-up SHIPPED same day: `value_v7` semantic-floored seeds (flips the
   noise-child pump pin; v7 ≡ v6 on content graphs; survivor = structured-but-valueless,
   which is frontier #3 — the layering converged onto the already-named out-of-band gap).

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
