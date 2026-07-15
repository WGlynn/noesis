# DESIGN — JUL stability tiers (TSS L2/L3): deferred, app-layer, not base consensus

> **Status: 🟡 designed-not-built, and deliberately OFF the base layer.** This is a one-paragraph SPEC
> STUB so whoever picks up the oracle work has the map. Do NOT build the oracles into Noesis consensus.
> Authoritative decision + the verified constants: `internal/DECISIONS-M3-money-2026-07-15.md` §2
> (single-sourced there — do not re-list constants here, they drift).

## The decision

JUL's base layer ships **TSS Layer 1 only**: energy-anchored proportional PoW reward with the Moore's-law
decay (`jul::reward_with_decay`, `noesis_core::pow::moore_decay_q32`). L1 alone is **energy-pegged and
oscillating** — honest, oracle-free, self-contained, forever-viable. The two stability tiers that damp the
oscillation —

- **L2 — PI controller** (RAI-style, target floats on electricity + CPI; ~120-day half-life integrator), and
- **L3 — elastic rebase** (AMPL-style, ±band, O(1) global scalar, multi-oracle median),

both depend on a **CPI + electricity-price oracle**, which is trust-minimization-hard and MUST NOT sit in
base consensus. Per the ratified ruling they belong on a **different layer entirely**: pegged-stable
**DeFi derivatives built on JUL-L1** (the TSS is a VibeSwap paper; VibeSwap or any DeFi layer builds the
stable on top). The oracle dependency then becomes **opt-in at the derivative layer**, never
consensus-critical. This matches the "pure-utility base, no bolted-on speculative token" thesis: the full
trinomial still exists as a system, correctly LAYERED (base = honest energy money, app = stability
derivatives) instead of monolithic.

## Handoff for the team that builds it

- **Constants** (Kp/Ki/leaky-α, rebase band/lag, the convergence theorem `Var(p)→σ²_elec`): read
  `DECISIONS-M3-money-2026-07-15.md` §2. Carry the VibeSwap audit fixes noted there (per-tick clamp +
  windup cap on L2; `MAX_REBASE_SCALAR` per-rebase clamp + oracle MEDIAN not owner-set on L3).
- **Oracle** = the real work: the dual electricity/CPI feed. Noesis's AMD upgrade over the Solidity
  original is a **bonded-verified-compute dispute** (bonded input + dispute window + slash) — dissolves the
  oracle-trust at the substrate level. Design this before locking any tier.
- **Honesty (launch copy):** never claim the stability theorem / σ²_elec floor / "stable" while only L1
  ships. L1 = "energy-anchored money," oscillating; L2/L3 = "designed-not-built, deferred pending oracle
  design + contributors." ✅ built · 🟡 designed · never round up.
