# DESIGN — corroboration: staked peer-attestation of contributions — ⚑ WILL REVIEW BEFORE BUILD

> Status: **ready-for-critique, NOT built.** From Will 2026-07-19 ("a frontend UI connection so
> contributions can be corroborated"). Corroboration is Sybil-sensitive (it touches the known
> vested-certifier / judge-cartel open gaps), so this is design-gate-first, never a blind build. The
> read-only half is already live: `GET /contributions` + the DAG view (provenance + downstream) shipped
> (`6109393`, `f0b1df6`). This designs the *action*.

## 0. What corroboration is (and why it matters)
Downstream-build already connects a contribution to an event ("someone built on it"). Corroboration is
the *lighter, faster* connection: a second party **stakes their standing to attest** a contribution is
real/valuable, without having to build on it. It is the peer-prediction value signal — the human
judgment layer that the something-from-nothing / oracle-free-value work needs, made honest by structure.

## 1. Grounding — this is ~half-built already (do NOT reinvent)
Noesis already has the machinery:
- **Endorsement mints value + is slashable.** `endorsement_slashing_makes_the_vested_certifier_ring_negative_ev`
  and `encoded_noise_endorsement_is_negative_ev_slashing_is_content_agnostic` (lib.rs ~5526+): an
  endorsement that vouches for later-refuted / garbage content triggers a λ=1 clawback ⇒ negative EV.
- **The dispute layer** (`lib.rs::dispute`, `resolve_refuted`, appeals) is the refutation path a bad
  corroboration is slashed against.
- **Collusion detection** (`attribution_circulation` + `attribution_cycle_energy` Hodge residual →
  `collusion_slash`) catches mutual-attestation *rings* on graph topology.

So corroboration ≈ a first-class **endorsement cell** wired to the live API, riding the existing
mint-then-slash-on-refutation + Hodge-ring-detection rails.

## 2. Mechanism (proposed)
- **Action:** `POST /corroborate { address, index, ots_*, target_id }` — a signed cell (same XMSS auth as
  `/submit`) naming a finalized `target_id`, that **stakes** the corroborator's standing.
- **Reward (peer-prediction):** the corroborator earns iff the target later accrues realized value
  (downstream flow) OR is corroborated by others whose judgment proves out — rewarding *predicting what
  the network will value*, not mere agreement. The target also gains (its value is corroborated).
- **Slash:** if the target is refuted (dispute settles against it), every corroborator is clawed back
  (endorsement-slashing, existing) ⇒ vouching for garbage is negative-EV.
- **Anti-ring:** the corroboration graph is fed through the SAME Hodge residual → `collusion_slash`, so a
  mutual back-scratching ring shows circulation and is slashed; an honest one-way attestation shows ~0.

## 3. The Sybil surface — the load-bearing risk (Boardy edge #2, and OUR open gaps)
Corroboration that feeds value is a new gaming surface. The KNOWN open gaps it touches (named in
`RESULTS-FAITHFUL.md`, all pedagogical-pinned tests): `vested_certifier_endorsing_garbage_open_gap`,
`judge_cartel_protects_its_own_garbage_open_gap`, `sybil_identity_ring_pumps_the_flow_gate_open_gap`.
**The design is not done until it shows corroboration-gaming is unprofitable** — the existing
endorsement-slash + Hodge-ring closers must be shown to cover the corroboration-specific ring, MEASURED
(a sim), not asserted. This is exactly the adversarial test the moat work says is the right instrument.

## 4. ⚑ Decisions for Will
1. **Who does corroboration pay — the TARGET (credit the ancestor), the CORROBORATOR (peer-prediction
   reward), or both?** (Lean: both, but the corroborator's reward is peer-prediction-scored + staked, so
   it is not free agreement.)
2. **Stake size:** how much standing must a corroborator lock, and for how long (vesting/dispute window)?
3. **Peer-prediction scoring rule** — the exact function rewarding "corroborated something that proved
   valuable." (This is a mechanism-design choice; candidates: Bayesian truth serum / output-agreement.)
4. **Does corroboration WIDEN or CLOSE the vested-certifier gap?** The load-bearing question — must be
   answered by an adversarial sim before build.

## 5. Build order (once ⚑ answered + the anti-gaming sim passes)
- Reference: the corroboration/endorsement cell + peer-prediction scoring + slash wiring (RED-first,
  reuse dispute + Hodge rails).
- **Adversarial sim FIRST** (like `sybil_sim.rs` / `moat_sim.rs`): a corroboration ring must be priced to
  ≤0; an honest corroborator must be paid. Ship the mechanism only if the sim passes.
- Then API (`POST /corroborate`) + a corroborate button in the DAG view.

## 6. Honest scope
NOT a launch blocker; the live testnet works without it. It is the highest-value *value-layer* build —
the human peer-prediction signal — but it is precisely the Sybil-sensitive surface, so it earns its place
only behind an adversarial sim, never on assertion. Design-gate, then sim-gate, then build.
