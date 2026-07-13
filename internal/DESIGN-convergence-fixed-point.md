# DESIGN — structural ledger convergence, as a cited fixed-point result

> **Status: ✅ design-verified against source** (adversarial verification 2026-07-13, 3 independent
> referees + synthesis, line-by-line against `node/src/lib.rs`, `CLAWBACK-CASCADE-SELF-HEALING.md`,
> `noesis-pragma-overlap.md`). This is the STRUCTURAL convergence of the ledger/CLAWBACK re-fold. It is
> **distinct from** the *value-measure* convergence (learned v(S) → true measure), which remains 🔬 OPEN
> and data-blocked (whitepaper §HCE/§living). Do not conflate the two. Never round the OPH↔PoM
> single-fixed-point bridge up to a result — it is an OPEN conjecture (`noesis-pragma-overlap.md:38-42`).

## The result (exact, three layers)

**(i) OUTER — order-theoretic (standing↔value re-fold).** Carrier = the tombstone/taint set as a subset
of the finite cell set, ordered by inclusion (finite ⇒ complete lattice). The re-fold is ONE monotone
self-map `F`: `F(T)` = `T` plus every cell that falls below `standing_floor` after value is re-folded
under `T`. Monotone because more taint zeroes more non-negative seeds (`lib.rs:1108-1112`), lowering
downstream flow through the monotone gate `g=f/(f+half)` (`lib.rs:1011-1016`) and the AND-composed
factors ∈[0,1] (outcome factor `lib.rs:1226-1236`) — no cell's value can rise when taint grows. By
**Knaster–Tarski**, `F` has a complete lattice of fixed points (existence); the Kleene ascent from the
empty-taint bottom reaches the **least** fixed point. **Termination bound the order-theory actually
delivers = lattice HEIGHT ≤ number of cells.** The tighter "≤ provenance-depth" (CLAWBACK 3.4) is a
*strictly stronger*, structure-specific claim needing an unproved one-hop propagation lemma — flagged,
not asserted. Clean monotone story holds for the hysteresis-free v1 operator; the materiality/hysteresis
amendment the doc flags trades strict monotonicity for grief-resistance.

**(ii) INNER — the value-flow map.** `next[b] = own(b) + d · Σ_j ρ^j · flow(child_j)` over canonical
rank-ordered EXTERNAL children (`lib.rs:3437-3497`). Two damping knobs: caller-supplied outer `d∈(0,1)`,
and hard-coded inner per-child geometric rank-decay `ρ∈(0,1)`, default `1/φ≈0.618`. On a well-formed
provenance DAG (cells built on strictly-earlier cells, ids increasing) the child operator is
**NILPOTENT** — every path strictly descends the order — so the Neumann/Jacobi series terminates EXACTLY
in ≤ depth+1 for ANY `d`. **Convergence on the DAG is structural (finite path length), NOT a consequence
of the Lipschitz constant.** The damping is load-bearing for two *other* things: (a) bounding magnitude
— the joint tail `Σ_j ρ^j ≤ 1/(1−ρ) ≈ 2.618` closes the volume-gaming / hybrid-diagonal pump; (b) making
the map a genuine **Banach** contraction so a MALFORMED non-self cycle stays bounded — real and
non-vacuous because the code drops only self-loops (`p≠c.id`, `lib.rs:3392, 3420`), so a mutual-citation
A↔B is not structurally excluded; **acyclicity is a construction convention, not a runtime guard.**
`ρ=1/φ` is a tunable default; the only load-bearing property is `0<ρ<1` (`lib.rs:3469-3476`).

**(iii) SCHEDULE-INDEPENDENCE — a separate result.** Two honest nodes applying the same finalized blocks
reach an identical digest regardless of order. **Newman's lemma** (local confluence + termination ⇒ a
schedule-independent unique normal form) on the async apply/replica layer (a rewriting/patch-net object):
termination from finite blocks over a finite lattice; local confluence from fold-determinism
(`noesis-pragma-overlap.md:24-26`). A THIRD result about a DIFFERENT object — not the schedule-independence
of the (i)/(ii) re-fold.

**Framing.** The converged ledger is a **DERIVED attractor** of a disagreement-lowering dynamic (the zero
set of an inconsistency potential), NOT an underived base case. (See `internal/CALIBRATION-base-case-is-god-2026-07-13.md`.)

## Citations
- **Knaster–Tarski** (existence of the fixed-point lattice): B. Knaster (1928); A. Tarski, *A
  lattice-theoretical fixpoint theorem and its applications*, Pacific J. Math. 5 (1955) 285–309.
- **Kleene iteration** (reachability of the least fixed point from ⊥; finite lattice ⇒ monotonicity
  suffices, terminates in ≤ lattice height): S. C. Kleene, *Introduction to Metamathematics* (1952).
- **Newman's lemma** (terminating + locally confluent ⇒ confluent): M. H. A. Newman, Ann. of Math. 43
  (1942) 223–243.
- **Banach fixed-point** (contraction ⇒ unique fixed point): S. Banach (1922).
- Source anchors: `lib.rs:1011-1016, 1092-1122, 1226-1236, 3388-3396, 3413-3430, 3432-3510`;
  `CLAWBACK-CASCADE-SELF-HEALING.md §3.4`; `noesis-pragma-overlap.md:24-26, 38-42`.

## Required hedges (carry every time this is stated)
1. OUTER step-count is lattice-height (≤ #cells); "≤ provenance-depth" for the outer re-fold is an
   unproved conjecture. (The INNER map genuinely IS depth-bounded, by nilpotency — do not conflate.)
2. INNER converges by nilpotency for ANY `d`; do NOT attribute inner convergence to `d<1`.
3. `1/φ` is a tunable default, not a derived constant; load-bearing property is `0<ρ<1`.
4. Keep the two damping knobs distinct (outer `d`, inner `ρ^j`); keep the `ρ^j` anti-gaming weighting.
5. Acyclicity is a construction convention, not a runtime guard (only self-loops dropped).
6. Knaster–Tarski (existence) vs Kleene (least-fixed-point reachability) are distinct attributions.
7. The clean monotone story is the hysteresis-free v1 operator; hysteresis breaks strict monotonicity.
8. Newman's lemma is the apply/replica layer — a third, distinct result; name its two premises.
9. OPH↔PoM single-fixed-point bridge = OPEN conjecture, never a result.

---

## Ready-to-drop whitepaper paragraph (Will places it; scope it AWAY from the open value-measure convergence)

> **Structural convergence of the ledger (distinct from the open value-measure convergence of Section~\ref{sec:hce}).**
> The self-healing re-fold has a fixed point by construction. Over the finite lattice of taint sets
> (ordered by inclusion) the standing→value→standing map is monotone, so by the Knaster–Tarski theorem
> its fixed points form a complete lattice and Kleene iteration from the empty-taint bottom reaches the
> least one; termination is bounded by the lattice height. The inner value-flow propagation converges on
> the acyclic provenance graph by nilpotency — every path strictly descends the provenance order, so the
> series terminates in at most one step per provenance layer for any damping factor; the damping
> `0<\rho<1` (default `1/\varphi`, a tunable choice, not a derived constant) is what bounds a parent's
> total child pull by the geometric tail `\sum_j \rho^j \le 1/(1-\rho)`, closing the volume-gaming vector,
> and what keeps any malformed off-graph cycle bounded (a Banach contraction). Schedule-independence — two
> honest nodes reaching an identical digest regardless of apply order — is Newman's lemma on the replica
> layer (local confluence from fold-determinism, termination from finite blocks). None of this closes the
> separate, open question of whether the learned value measure converges to the true one; that remains
> data-blocked (Section~\ref{sec:living}).
