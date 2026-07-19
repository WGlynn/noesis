# DESIGN — bootstrap admission (the load-bearing Sybil brake before the moat)

> Status: **proposed design, pending Will's family choice** (§6). No code yet. Directly downstream of the
> measured result in `docs/research/v0-sybil-failure-envelope-2026-07-19.md`: with a per-identity
> standing cap, captured share ≈ F/(N+F), so identity-count control — not the cap, not the (near-theater)
> deposit — is the load-bearing brake during bootstrap. This spec defines that control honestly, as a
> **temporary, bounded-harm authority that gates identity count and never value**, designed to dissolve.

## 1. What this is and is not

- **Is:** an admission mechanism that bounds the number of distinct contributing identities during the
  cold-start window, so a solo scripted farmer cannot reach parity (F ≈ N) by costless keygen.
- **Is not:** a value judgment, an anti-collusion mechanism, or a permanent authority. It gates *who may
  submit*, never *what a contribution is worth*. v0's stated guarantee is unchanged: **bounded identity
  capture, not value measurement or anti-collusion.**
- **The honest cost, named:** any admission rule is itself an authority — the airgap the design exists to
  dissolve, one level up. We accept it **only** during bootstrap, **only** for identity count, and with a
  **dissolution trigger** (§5). At bootstrap there is no oracle-free legitimacy signal to key it on,
  because legitimacy in this system *is* realized contribution value, which has no history yet.

## 2. Objective (from the sim)

The sim shows farmer share ≈ F/(N+F): the farmer captures the contribution dimension once F ≈ N. So the
admission invariant is simply:

> **Bound the count of admitted-but-unvouched identities to a small fraction of the honest set** so that,
> for the anti-concentration-relevant threshold τ (contribution-dimension share the design must keep from
> a single actor), admitted-adversary share stays < τ. The sim's Regime C (farmer identities ≤ N/5) held
> share at ~17%.

This is a *rate/count* bound, not a value bound. It composes with (does not replace) the per-identity
standing cap.

## 3. The mechanism — two phases

**Phase A — founder cohort (genesis..H₁).** A small curated set of genesis contributors, admitted by the
operator. Honest, simple, and correct for a set of ~5–50: the operator is trusted at genesis anyway (they
chose the chainspec). Recorded on-chain as an admission cell so the set is auditable. Blast radius:
capturable authority — but the harm is bounded (identity count) and the set is public.

**Phase B — bounded-fan-out invites (H₁..dissolution).** Each *vested* identity (standing ≥ a vesting
threshold, so it has skin in the game) may vouch for at most **k** new identities per window W, where the
vouching identity's own stake is **partially bonded** against the invitee scoring zero novelty (reusing
the Bound-B deposit machinery). This makes an invite a *costly, accountable* act, not free keygen:
- growth is rate-bounded (≤ k·(vested count) new identities per W);
- a farmer who buys/farms one vested identity can mint only k invitees per window, each bonded;
- Sybil-via-invite requires either compromising vested identities or burning bond on junk invitees.

Fan-out `k` and window `W` are tune-when-live parameters (like the inclusion policy and θ_sim), pinned on
the testnet, not derivable in the abstract. Start conservative (k = 1–2) and relax as the honest vested
set grows.

## 4. What makes an identity eligible (the honest answer)

There is **no oracle-free legitimacy signal at bootstrap**, so eligibility rests on an imported signal,
and we name it rather than dress it as structural:
- Phase A: operator judgment (curation).
- Phase B: a *vouch from a vested identity* + the voucher's bond — i.e. eligibility is "an accountable
  existing contributor stakes on you." This is social-graph admission with skin in the game, not identity
  uniqueness. It bounds *rate*, not Sybil-ness per se; a colluding vested seed is the residual attack
  (bounded by k·W and the bond).

Explicitly **not chosen** (and why), from the tradeoff table in the failure-envelope doc: external
proof-of-personhood (heavy external oracle + privacy + its own capture surface); per-identity PoW (only
shifts the budget line a funded farmer crosses); pure founder curation forever (does not scale).

## 5. Dissolution trigger (the part that makes it honest, not permanent)

Admission control is scaffolding. It is removed when the mechanisms that make cheap content negative-EV
*structurally* are live and load-bearing on the deployed franchise, specifically when **both**:
1. the **structural defense** (v6 identity pricing + dispute endorsement-slashing) is wired onto the
   deployed intake franchise (today it ships only the v0 floor), so a fresh identity is worth ~0 and a
   junk-certifying ring is negative-EV without admission control; and
2. the **honest vested set** is large enough that the anti-concentration floor (`MIN_DIM_BPS`) is a real
   constraint (Boardy's "50% of almost nothing" concern is gone).

At that point Phase B relaxes k → ∞ (open admission) and the authority is retired. The dissolution
condition is published up front so admission is visibly temporary, not a permanent gate wearing a
"bootstrap" label.

## 6. Open decision (Will)

- **Family choice:** confirm founder-curation → bounded-fan-out invites (this spec's lean), or pick
  another family from the failure-envelope tradeoff table.
- **Parameters (tune-when-live):** k (fan-out), W (window), vesting threshold for vouching, invite bond.
- **On-chain vs off-chain admission record:** on-chain admission cell (auditable, recommended) vs an
  operator-side allowlist (simpler, less transparent).

Nothing here is built; this is the design to accept, modify, or reject before any wiring. It is
consensus-adjacent (admission gates block production), so the flip is a deliberate, Will-gated step (PCP),
sequenced with the go-live knot (`CONTROL_BINDING_ACTIVE` → submission_deposit) per
`[[interdependent-enforcement-ships-together]]`.
