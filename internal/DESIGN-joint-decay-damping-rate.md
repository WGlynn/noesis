# DESIGN — joint-decay damping rate (RHO / ρ): what is load-bearing, what is a tuning knob

Status: ✅ built (mechanism) · framing-corrected 2026-07-12 (RSAW finding: "RHO=1/φ is numerology here").

## Where it lives
- `node/src/lib.rs:3458` — `const RHO: f64` in `flow::value_flow_with_own`, applied as `RHO.powi(j) * contrib` (:3476).
- `node/src/lib.rs:8217` — `const RHO_Q32: u128` in `settlement_fixed::value_flow_external_q32`, applied as the iterated decay `w = mul(w, RHO_Q32)` (:8236).
- The λ^r within-identity and μ^m cross-identity dampings that preceded the single-joint tail used the same value; they are now folded into the one global-flattened ρ^j tail (ROADMAP.md 2026-06-18 (v)).

## The ONLY load-bearing property: 0 < ρ < 1
The joint decay closes the volume-gaming vectors (stack M under one identity / split across K identities / the K×M hybrid diagonal) because every layout lands in the SAME canonically-flattened sequence and draws from ONE geometric budget

    Σ_{j≥0} ρ^j = 1 / (1 - ρ),   finite iff 0 < ρ < 1.

Two facts follow, and BOTH need only `0 < ρ < 1`:
1. **Contraction** — the backward flow map is a contraction, so damped-Jacobi converges to a unique fixed point (no unbounded self-attribution pump).
2. **Saturation** — a parent's downstream flow is bounded by `flow / (1-ρ)`, so per-identity/per-parent volume saturates instead of amplifying.

At the default ρ = 1/φ the tail sum is 1/(1-0.618) ≈ 2.618. That number is NOT special: ρ=0.5 gives 2.0, ρ=0.7 gives 3.33, etc. The bound, the contraction, and the saturation hold identically for every ρ in the open interval.

## What is NOT load-bearing: φ specifically
Nothing in the mechanism exercises φ's defining identity φ² = φ + 1, any Fibonacci recurrence, or a low-discrepancy / equidistribution property. `RHO.powi(j)` and `w = mul(w, RHO_Q32)` are a pure geometric weight and a pure iterated decay. `round(2^32/φ)` (2_654_435_769) is used here ONLY as the Q32.32 image of the same default rate so the f64↔fixed drift-guard (`v7_q32_tracks_f64_v7_*`, `t6_*`) stays in band — the "Fibonacci-hashing constant" name is decorative and has been removed from the code.

Therefore: 1/φ is a **chosen default damping rate**, not a constant that falls out of the coverage/provenance geometry. Calling it a SubstrateGeometryMatch would require the macro fractal geometry to IMPLY the micro constant; it does not. This doc is the honest record of that.

## Evidence the value is a knob, not a derivation (break-on-purpose)
The team's own anti-theater tests certify `0 < ρ < 1`, never `ρ = 1/φ`:
- `ROADMAP.md:762` — ρ:=1.0 (removes contraction) reopens the K4×M4 diagonal and turns T3 RED. Load-bearing bound: ρ < 1.
- `internal/CONTINUE.md:303` — μ:=0.05 (a non-φ rate well below 1/φ) ALSO closes the pump (K2×M2 = 16.63 < 18.11). Load-bearing bound: ρ > 0.
- `patent/PROVISIONAL-Provenance-Attribution-v1.md:233` — the inventive step reduces to "makes backward propagation a contraction," a property of ANY 0 < ρ < 1.

## Why keep 1/φ as the default (not 0.5)
No mechanism reason — a mild aesthetic/consistency one only. 1/φ ≈ 0.618 gives a moderate contraction (tail ≈ 2.618): honest rank-0/1 children stay near full weight (ρ^0=1, ρ^1≈0.62) so diverse honest certification is barely touched, while the tail still saturates volume. Any ρ in roughly [0.4, 0.7] behaves comparably. Retune on real attribution data if the honest-vs-attacker separation warrants it — per [P·augmented-mechanism-design-paper], the calibration should follow measured labels, not a chosen constant.

## Framing rule going forward
- Code/patent/docs: describe ρ as a **tunable sub-unity damping rate (default 1/φ)**. Do NOT assert it as a derived structural constant, a Fibonacci-hashing constant, or a SubstrateGeometryMatch.
- The saturation/contraction claims are the honest, provable ones — state THOSE.
