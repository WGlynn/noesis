# v0 Sybil Failure Envelope — the deployed franchise under adversarial simulation

**2026-07-19.** Companion to `something-from-nothing-oracle-free-content-value.md` and
`SYBIL-SURFACE-deployed-franchise-2026-07-19.md`. Reproduce:
`cargo run --release -p noesis --example sybil_sim` (`node/examples/sybil_sim.rs`). The simulation
drives the **real** consensus scorer (`pom_scores_with_similarity_floor_q16`, θ_sim = 0.95) — only the
content and the attack model are synthetic. Numbers below are the program's output, not estimates.
Prompted by Boardy's review (publish the failure envelope alongside the parameters).

## Threat target (Will, 2026-07-19)

v0 + a per-identity standing cap should survive the **casual solo scripted farmer** — one actor, tens
of identities, a weekend of effort — during bootstrap (≈ 5–50 honest participants). It is **not**
claimed to survive a funded, coordinated cartel; that is the realized-value moat + the dispute market's
job once the network has honest participants and outcome history. This document states exactly where
the boundary sits.

## An honest preliminary: the deposit is near-theater on the testnet

Test JUL is worthless by design (trivial PoW), and it is *self-financing*: a contribution earns
coinbase JUL that funds the next submission's deposit. So a JUL-denominated `submission_deposit` costs
a farmer essentially nothing on the testnet, and the farmer's only real cost is **compute** (generating
varied content + XMSS keys). The load-bearing brakes are therefore the **per-identity standing cap** and
the **initial allowlist**, not the deposit. We say so rather than imply the deposit is doing work it
isn't.

## Results

Units: standing is coverage-novelty; one honest identity contributing 5 posts earns ≈ 985, and the
per-identity cap `C` is set to that (each identity, honest or farmer, is capped at one honest
contributor's worth). Farmer content is high-entropy junk — **maximally novel**, so under v0 it earns
like genuine work; that is the audited gap, not a bug in the sim.

**Regime A — no cap, one farmer identity, growing volume (N = 10 honest):**

| farmer junk submissions | farmer share of contribution dimension |
|---|---|
| 10 | 17.0% |
| 50 | 50.5% |
| 200 | 80.3% |

Without a cap a single identity dominates by volume. The cap is necessary.

**Regime B — per-identity cap `C`, farmer fields `F` identities (5 posts each):**

| honest N | smallest F to capture ≥50% | share at F=N | share at F=2N |
|---|---|---|---|
| 5  | 5  | 50.3% | 66.9% |
| 10 | 10 | 50.7% | 67.2% |
| 25 | 24 | 51.5% | 68.0% |
| 50 | 45 | 52.9% | 69.2% |

With a cap, aggregate standing is proportional to identity count: **share ≈ F/(N+F)**, and the farmer
captures the dimension once **F ≈ N**. Because keygen is costless, a solo farmer reaches `F ≈ N`
trivially against a 5–50-person honest set. **So the cap alone does not survive the target threat.**

**Regime C — cap + an allowlist bounding admitted farmer identities to ≈ N/5:**

| honest N | admitted farmer identities | farmer share |
|---|---|---|
| 5  | 1  | 16.8% |
| 10 | 2  | 17.0% |
| 25 | 5  | 17.5% |
| 50 | 10 | 18.4% |

Bounding **identity count** holds the line (~17%). The allowlist / proof-of-personhood is the
load-bearing bootstrap brake.

**Collusion is a null at v0.** The deployed franchise scores first-appearance novelty only (no
downstream-flow gate), so cross-citation among farmer identities changes nothing — share at N=10, F=10
is 50.7% with or without collusion. Collusion first bites at the v5+ realized-flow layer, not here.

## v0's stated guarantee (adopt verbatim)

**v0 provides bounded identity capture — not value measurement, and not anti-collusion.** (Label
crystallized in review, 2026-07-19.) Everything below is that sentence, shown.

## The envelope, stated plainly

1. **The cap is necessary but not sufficient.** It bounds per-identity damage; it does not bound total
   damage across free identities. With the cap, Sybil-resistance reduces *entirely* to controlling
   identity count.
2. **v0 + cap survives a solo farmer only while F < N.** A scripted actor with costless keygen defeats
   that against a bootstrap-size honest set (5–50). Therefore, for a public permissionless launch, an
   **allowlist (or proof-of-personhood) bounding identity count is mandatory, not optional**, until the
   realized-value moat and the dispute market come online.
3. **The deposit is not the brake on the testnet** (worthless, self-financing JUL). Do not rely on it.
4. **These are training wheels.** They relax as the moat (learned oracle-free `v(S)`) and the dispute
   market — which make cheap content negative-EV structurally rather than by admission control — come
   online with graph history.

## The admission rule — an open decision, not a solved one

Since identity-count control is the load-bearing brake, the natural next question is *what admission
rule bounds the trickle, and what makes an identity eligible.* We do not publish a specific rule yet,
and the honesty here matters: **any admission rule is itself an authority — the same airgap the design
exists to dissolve, moved up one level.** Worse, at bootstrap there is **no oracle-free legitimacy
signal** to key it on, because legitimacy in this system *is* realized contribution value, which does
not exist until the graph has history. So admission necessarily imports an external signal; the honest
move is to name which one and its failure mode, not to dress it as structural.

| Candidate | Eligibility signal | Honest failure mode |
|---|---|---|
| Founder-curated allowlist | operator judgment | capturable authority; does not scale |
| Bounded-fan-out invites | an admitted identity vouches for ≤ k more | Sybil-able if invites leak or the seed colludes |
| External proof-of-personhood | third-party uniqueness (BrightID/Worldcoin-class) | heavy external oracle; privacy + its own capture surface |
| Per-identity proof-of-work | real compute per identity | only shifts the budget line a funded farmer can still cross |

**Current lean (not locked):** founder curation for the genuine first cohort → bounded-fan-out invites
as it grows, published explicitly as a *temporary, bounded-harm authority that gates identity count and
not value*, designed to dissolve as the realized-value moat comes online. The eligibility signal a
reviewer would want does not yet exist in an oracle-free form — and saying so is more useful than
manufacturing one.

## Honest limits of this simulation

- It models the **intake franchise** (novelty + θ_sim + cap), not the full stack; the dispute market and
  v5–v8 are not exercised (they are not the deployed testnet franchise).
- Content is synthetic; "honest" content is language-shaped but from a large alphabet, so honest
  contributors do not artificially cannibalise each other's novelty (an earlier tiny-vocab draft did,
  overstating the farmer's edge; corrected). Under v0 honest-diverse and junk are novelty-equivalent by
  construction — which is the whole point.
- The cap unit is normalized to one honest identity's output; absolute values scale with content length.
