# Multi-identity-split fix — acceptance criteria + adversarial test matrix (PRIVATE)

> Stealth / front-run-sensitive. NEVER sync to public substrate.
> Status: SPEC (no production code). Advances the (r) 🔬 gap toward a fresh-low-context build.
> Origin: pom-roadmap-advance tick 2026-06-18 (r-sibling). The `value_flow_with_own` surgery
> itself stays gated to fresh low-context (highest blast radius, feeds v5–v8) — this is the
> red→green target it must hit, written now while the numbers are loaded.

## The gap (r), restated with the honest numbers
The (q) per-identity λ^r damping caps ONE identity's volume but is INERT against a split across
K distinct vested identities (each child is rank-0 in its own identity-group ⇒ λ^0=1 ⇒ full weight).
Measured (test `multi_identity_split_volume_defeats_per_identity_damping_open_gap`, 221/221):

| K distinct identities | 1 | 2 | 4 | 8 |
|---|---|---|---|---|
| multi-identity v8 | 14.28 | 17.26 | 19.33 | **20.57** |

Still climbing, no saturation. K8 = 20.57 **exceeds** the single-identity v8 saturation bound (18.11):
**splitting beats stacking.** That is the vuln.

## ⚠ Open design decision the build MUST resolve first — the two canonical docs diverge
- **ROADMAP (r) prescribes Opt A (hard cap):** thread `value::max_certifying_identities(total_standing,
  floor)` into the per-parent certifier set in `value_v6`; cap distinct certifiers/parent at
  `total_standing/floor`; identities beyond the cap (lowest-flow first) contribute 0.
- **CONTINUE (r) recommends Opt B (geometric μ^m):** sort a parent's certifying identities by grouped
  contribution desc; weight the m-th identity by μ^m (μ = 1/φ candidate) — symmetric to the (q) λ^r
  within-identity decay, one axis up. One identity full, additional identities decay.

**Recommendation: Opt B**, on two grounds —
1. **Consistency / least-surprise:** (q) already chose geometric (λ^r) over a hard first-commit-wins cap
   per PONYTAIL; the cross-identity axis is the *same shape*, so μ^m is the natural sibling. One
   mechanism family (geometric saturation) governs both axes ⇒ smaller conceptual + audit surface.
2. **Honest-cert preservation:** a hard cap (A) can over-punish legitimately broad certification (a
   genuinely popular parent certified by many honest distinct identities gets truncated to 0 beyond the
   cap). B decays softly — honest few-identity cases stay ~full; only volume-via-many-identities saturates.
   Matches the (q) "bound, don't zero; honest cases INERT" residual philosophy.

A is sharper (a true ceiling) but brittle at the honest boundary. If a hard ceiling is later required for
a proof, A can compose ON TOP of B. **Final A-vs-B call is the build author's (fresh context) — but the
test matrix below is written to be fix-agnostic: either fix must pass it.**

## Acceptance criteria (the green target)
Let `bound_single = single-identity v8 saturation ≈ 18.11` (the (q) result). After the fix:

1. **SATURATION across identities:** multi-identity v8 over K = 1,2,4,8 must converge, not climb.
   Concretely `v8(K8) − v8(K4) ≤ ~3%` of `v8(K4)` (mirrors the (q) single-identity 4→8 = +2.5% bar).
2. **SPLIT ≤ STACK:** `v8(K distinct identities) ≤ bound_single × (1 + ε)` for all K, with ε ≤ the (q)
   honest residual (~2.7%). I.e. splitting must NOT beat stacking — the (r) inversion (20.57 > 18.11)
   is gone. This is THE assertion that flips the open-gap test.
3. **BOUND, don't zero (honest residual, explicit):** the fix bounds the pump; it need not drive the
   attacker to 0. State the residual number honestly in the test, as (q) did.
4. **HONEST CASES INERT:** every honest v5–v8 case stays green and ~unchanged — single-child lineages,
   and genuinely diverse certification by 1–2 honest distinct identities (ranks 0/1 ⇒ μ^0,μ^1 ≈ full).
   Add an explicit `honest_diverse_certification_unaffected` assertion (2 honest identities, distinct
   real parents) proving INERT.
5. **DETERMINISM:** the cross-identity ordering MUST be a canonical sort (e.g. grouped-contribution desc,
   tie-broken by a canonical key — identity args then commit index) so replicas converge. No HashMap
   iteration order. Same discipline as (q)'s ascending-commit-index pass.
6. **Q32.32 MIRROR:** replicate in the settlement port `value_flow_external_q32` (MU_Q32 = round(2^32/φ)
   = 2654435769 if μ = 1/φ, reusing the (q) LAMBDA_Q32 constant) and hold the drift-guard
   `v7_q32_tracks_f64_*` within band. Flow layer stays HOST-ONLY (no ELF rebuild).

## Adversarial test matrix (build to red, then green)
| # | scenario | pre-fix | post-fix (assert) |
|---|---|---|---|
| T1 | K distinct identities × 1 child each, K∈{1,2,4,8}, same parent | climbs 14.28→20.57 | saturates, crit. 1+2 |
| T2 | split-beats-stack: v8(K8 distinct) vs v8(8 children, 1 identity) | 20.57 > 18.11 | K8 ≤ 18.11×(1+ε), crit. 2 |
| T3 | hybrid: K identities × M children each (cross of λ^r and μ^m) | unmeasured | bounded both axes, no pump |
| T4 | honest: 2 distinct honest identities, distinct real parents | full | ~unchanged, crit. 4 (INERT) |
| T5 | determinism: shuffle identity/child input order | n/a | identical v8 (canonical sort holds) |
| T6 | Q32.32 parity on T1–T3 graphs | n/a | f64 ↔ fixed within drift band, crit. 6 |

T2 is the keystone (the inversion that defines the gap). T3 is the genuinely NEW surface — neither (q)
nor the (r) pin exercised the *cross* of both decays; a fix that bounds each axis independently could
still pump on the diagonal. T3 is the next adversarial-loop candidate even after this fix lands.

## What this tick did NOT do (honest scope)
No production code. `value_flow_with_own` / `value_v6` unchanged; suite still 221/221. This is the
spec the fresh-context build executes against — build = flip the open-gap test per crit. 2, add T3–T6,
honest-number the new saturation curve.
