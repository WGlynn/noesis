---
title: "Noesis Patent Family, Applications 2-5 — Batch for Review"
subtitle: "Four v1 provisional drafts, sent together for parallel review"
date: "2 July 2026"
---

# Batch cover, Noesis patent family

Four v1 provisional drafts, sent as a batch so they can be reviewed in parallel rather than
one at a time. All are internal drafts, not for filing. Foreign-filing-license gate still
governs any actual filing (US-origin invention).

## The family

Application 1, **Proof of Contribution** (already drafted, UKIPO package built), claims the
architectural invariant and the three implementation families together. It is the priority
filing and the centre of gravity. These four develop the candidate families mapped in its
`10_Patent_Family_Map.md` into standalone siblings:

| App | Title | Centre of gravity |
|---|---|---|
| 2 | **Authority Lifecycle and Enforcement** | Reassignment made unrepresentable; dispute-aware closure of exit-before-verdict. |
| 3 | **Hybrid Finalisation with Anti-Concentration** | PoW-excluded finality vs included consensus; each dimension independently clears a constitutional floor. |
| 4 | **Contribution Provenance and Attribution** | Graph-restricted cooperative-game division with damped backward propagation; collusion attribution keyed on soulbound identity. |
| 5 | **Strategyproof Temporal Valuation** | Zero-for-duplicate temporal-novelty value; deterministic division-free integer core. |

## Drafting posture (what to expect when reviewing)

1. **Claims recite the invariant, not the implementation.** Independent claims are broad and
   open ("comprising"); specific constants and enumerations (dimension floor fraction,
   supermajority fraction, damping factor, the exact operation set) are held in dependent
   claims as reserve narrowing. The intent is that a small design tweak cannot escape the
   independent claim while keeping the invention.

2. **Every mechanism in the detailed description carries a source pointer** (`file:line` into
   the public reference node), so any claim term can be anchored to something built and
   testable rather than to a description.

3. **Honest status markers throughout:** ✅ built, 🟡 designed, 🔬 open. Nothing is rounded up.
   Where a mechanism's on-chain sourcing is deploy-coupled, or a learned model is
   research-stage, it is marked, not asserted as complete.

## What would help most from the review

- Whether each independent claim is at the right altitude, broad enough to resist design-around
  but still supported by the reference implementation.
- Which single application, if any, should lead the family after the priority filing.
- The synergy / inventive-step posture (interdependence of the mechanisms) versus an
  element-by-element reading.
