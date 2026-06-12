# Coherence Laws (PRIVATE, stealth)

> The cryptoeconomic invariants Noesis must satisfy to be *coherent* — the anchor
> doc the others reference. Will: "set laws/rules/standards of cryptoeconomic
> coherence." Each law is structural (must be enforced by construction, not asserted).
> A design change that violates one is incoherent until the law is restored or
> explicitly amended here. Cross-refs: `WHITEPAPER.md`, `CRYPTOECONOMICS.md`,
> `POM-CONSENSUS.md`, `COORDINATION-SCHELLING.md`.

## L1 — Separation of powers (RPS minimality)
Money, governance, and capital/franchise are *separable* functions; exactly **three**
powers (cognition / compute / capital) form the minimal non-dominated cyclic
equilibrium. 2 → binary capture; 4+ → coalitions without added non-domination. One
instrument per function (Tinbergen).

## L2 — Soulbound franchise: no capital → consensus
Consensus weight (the franchise) is **soulbound / non-transferable**. Anything tradable
(state-bytes, money) MUST NOT buy consensus weight. Enforced as a type-script invariant
on the consume→produce transition (no owner/contributor reassignment), and consensus
reads the **contributor** key, never the owner lock. Violation ⇒ collapse to PoS.

## L3 — Conservation of proof (GEV)
Proofs are **conserved, relocated** — never eliminated by fiat. PoW is not deleted; it
moves to the money layer (JUL). Removing a proof requires showing its job is either
unnecessary or done by another proof, not just dropping it.

## L4 — Mint ↔ sink balance
Endogenous minting (novel contribution → PoM/state) MUST be matched by a sink (decay /
state-rent) that bounds total live supply and forces ongoing contribution to retain
state. No unbounded mint without a conserving burn.

## L5 — Strategyproof minting
Minting is gated on **temporal-novelty**: sybil, padding, and collusion-ring strategies
earn **0** by construction. Any new value rule MUST preserve this floor (the learned
`v(S)` included) or it is incoherent.

## L6 — Closed value-provenance
Value flows only along the provenance DAG (Myerson / graph-restricted). A coalition
disconnected in provenance creates no value. No value may be conjured from outside the
recorded contribution graph.

## L7 — Append-only, slashable
The chain is append-only and tamper-evident (signed, owned, Merkle-committed). Refuted
contributions are **slashable** within a dispute window, not silently deleted. History
is preserved; correction is an explicit, auditable event.

## L8 — Contributor floor
A genuine contributor MUST NOT be zeroed by a quiet period alone: a decay floor /
minimum-capacity grant per active contributor (anti-starvation). Prevents the decay sink
(L4) from eating honest participants.

## L9 — Core / nucleolus stability
Consensus is defection-proof: no validator coalition profits by deviating. Enforced by a
core/nucleolus stability constraint over the PoM-weighted coalition game. Required for
consensus specifically (not kitchen-sinked onto pure attribution).

## L10 — Two-axis robustness
Every hard-to-defend parameter/choice carries ≥2 independent justifications, so no single
objection collapses it. Single-pillar designs are incoherent under adversarial review.

## L11 — Coordination-layer integrity ≥ max coordinated attack surface
When two systems (e.g. the LLM and the DeFi protocol) coordinate **through** the chain,
the chain becomes a shared dependency: its compromise is correlated failure of both.
This nets positive **iff** the coordination layer's own integrity guarantees exceed the
**weaker** of the systems it coordinates. The hub must be harder to break than its
spokes, or it concentrates risk instead of dissolving it. (Derives the meta-security
result in `COORDINATION-SCHELLING.md`; pairs with the openness/cheap-exit condition that
keeps the hub a keystone, not a hostage.)

---

### Amendment log
- 2026-06-11 — L1–L10 drafted from the in-context invariant set; **L11 added** (meta-
  security / coordination-layer integrity bound). Genesis-burn fair launch ratified
  (see WHITEPAPER §10) — candidate L12 if it needs a standing invariant ("no creator
  pre-launch advantage; neutralization must be on-chain-provable, not asserted").
