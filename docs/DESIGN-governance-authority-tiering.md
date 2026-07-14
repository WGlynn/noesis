# DESIGN — Governance authority tiering (who may amend what)

> Status: design note / ready-for-critique. **Built this pass (`node/src/amendment.rs`):** the
> `Authority { Contribution, Stewardship }` classification + `Amendment::authority()` / `GovField::authority()`
> (the inert attach-point metadata), and the **number-free anti-plutocracy mix bound `pos ≤ pom`** in
> `check_mix` (`ObligationBreach::CapitalOutweighsContribution`) — 3 RED-first tests, lib 328 green.
> **Still deferred (honestly):** the vote-*counting* layer (PoM-weighted for Contribution, VIBE for
> Stewardship) — it needs the VIBE token + a sybil-resistant vote curve (an MD-paper decision), and `VIBE`
> appears nowhere in `node/src` (verified 2026-07-14). The classification is its attach-point. This note
> records the decision Will greenlit: **tier amendment authority so the system stays anti-plutocracy at
> every layer** — and the anti-plutocracy property is carried *structurally* (soulbound weight + the
> `pos ≤ pom` bound + the axiom gate), so it does not wait on the vote layer.

## The invariant this exists to protect

> Money can buy the tokens (JUL, VIBE) and can rent storage (state-bytes) — but it can never buy
> **consensus weight** (soulbound PoM-standing) **nor a say in what counts as contribution.**

Today the first half is enforced (weight = soulbound PoM). The second half is *not*: the measurement
knobs `theta_sim_q16` and `vesting_w` are governance-amendable, and TOKENOMICS names VIBE as buyable —
so a buyable token governing the definition of contribution makes the invariant only *mostly* true. This
note closes that, and one subtler hole (`AmendMix`), without collapsing governance into a single
soulbound instrument (which would re-fuse consensus weight with rule-setting and create a
contributor-oligarchy — see §Rejected).

## The load-bearing point: the axiom gate is the primary backstop, not who-votes

Anti-plutocracy rests **first** on the axiom-preservation gate — `verify_amendment` (safety bounds:
2/3 threshold floor, mix normalized, θ_sim ≤ 1, no stale base) plus the Pragma coherence check — which
rejects any amendment that breaks the value/Shapley invariants **regardless of who voted for it**. The
authority tiering below is a **second** line (legitimacy + closing the measure-buyability hole). An
unbounded PoM-vote without the gate would still be capturable; a bounded VIBE-vote *with* the gate is
already mostly safe. Both lines ship; the gate is the one that carries the safety weight.

## The tiers — authority mapped to what is governed

| Tier | What is governed | Authority | Why |
|---|---|---|---|
| **0 — Physics** | conservation, base value axioms (I1–I4), no-double-spend | **none** (near-immutable) | the floor nobody amends (existing: `amendment.rs` `Layer::Physics`, always rejected) |
| **1 — The measure + the mix direction** | `ThetaSimQ16`, `VestingW` (what counts as contribution + how it clears); `AmendMix` (the pow/pos/pom weights) | **PoM-standing (soulbound), or near-immutable** — hard-bounded | a say over the definition of contribution must be *earned by contributing*, never bought; and shifting the mix toward PoS is itself a move toward capital-rule, so it is bounded (a PoM floor + PoS ceiling) |
| **2 — Operational dials** | `ThresholdBps`, `QuorumFloorBps`, `MaxMempool`, `Horizon`, `DecayPos` | **buyable VIBE stewardship** (exitable) | these tune the machine without redefining contribution or shifting power between capital and contribution; liquid, exitable stewardship is healthy here (price signal + exit) |
| **all** | every amendment, any tier | **must pass the axiom-preservation gate** | the real backstop — an amendment cannot break the invariants no matter who authorized it |

### Why `AmendMix` is Tier 1, not Tier 2 (the correction the anti-plutocracy bar forces)
`AmendMix` reweights the overall NCI consensus mix (`NCI = pow 0.10 / pos 0.30 / pom 0.60`,
`lib.rs:3820`). A governance move that shifts weight toward PoS shifts the system toward *capital*
deciding outcomes — a plutocracy vector even though no single amendment "breaks an axiom." The finality
path is already structurally fenced (`FINALITY_MIX` is LOCKED at `pos 1/3 : pom 2/3`, PoW excluded,
`runtime.rs:1195`; and `MIN_DIM_BPS = 5000` forces each of PoS and PoM to independently supply ≥50% of
its own dimension, `runtime.rs:1432`). But the *overall* NCI mix has no such governance bound today.
Tier-1 treatment = a PoM-floor / PoS-ceiling on `AmendMix` so governance can never tune the chain toward
capital-rule, whoever holds the votes.

## Why not "governance = PoM" wholesale (the rejected over-correction)

Folding *all* governance into soulbound PoM was considered and rejected on mechanism grounds:
1. **Separation of powers / Tinbergen** — one instrument per function; PoM already carries consensus
   weight + right-to-mint, so adding rule-setting re-fuses three roles into one instrument.
2. **Contributor-oligarchy** — top-PoM holders controlling both finalization *and* the rulebook could
   amend rules to entrench their own contribution style (lower θ_sim to favor their output, set W to
   disadvantage newcomers). Soulbound changes *who* captures (the early/prolific), not *whether*.
3. **No exit** — a buyable governance token has a price and an exit; soulbound governance has neither,
   so a bad governing class is unremovable by market pressure.

The measure layer (Tier 1) genuinely *should* be contribution-gated; the operational layer (Tier 2)
genuinely *should* be liquid/exitable. They are two roles with conflicting requirements — so they get
different authorities. (This is the role-separation design law again — see
`role-separation-as-design-law.md` — applied to governance itself.)

## Anti-plutocracy check (the property Will set as the bar)

- **Consensus weight**: soulbound PoM, unbuyable. ✓ (existing)
- **Say over the measure** (θ_sim, W): Tier 1, contribution-gated ⇒ money cannot buy a say in what
  counts as contribution. ✓ (the TOKENOMICS invariant becomes literally true)
- **Mix direction**: Tier 1 + hard-bounded ⇒ governance cannot tune the chain toward capital. ✓
- **Operational dials**: Tier 2, buyable but cannot redefine contribution, shift power axes, or break
  axioms. ✓
- **Backstop**: the axiom gate holds regardless of voter. ✓

No layer lets capital buy weight, buy the definition of contribution, or tilt the mix toward capital.

## Sybil resistance — governance inherits it from PoM, adds no new open problem

The naive anti-whale voting rule (concave per-holder weight — quadratic/log) is sybil-gameable on any
*transferable* token: split holdings across N wallets and `√`-concavity turns back into linear (100 ×
`√1` > `√100`). The fix everywhere is a sybil-resistant identity layer — and Noesis *has* one:
**PoM-standing is soulbound and earned, so it cannot be split across wallets.** Anchoring vote weight to
it reduces "can I sybil the vote?" to "can I sybil PoM-standing itself?" — which is exactly the
un-gameability the chain already stakes everything on (the moat). So governance opens **no new sybil
surface**; it inherits the core defense. If the moat holds, governance-sybil-resistance holds for free.

**Better still, the tiering makes concave voting *optional*, not required:**
- Tier 1 (the measure) is on **soulbound PoM** — unsplittable, so the wallet-split attack does not exist
  there in the first place.
- Tier 2 (buyable VIBE) governs only **bounded, axiom-gated** dials, so even a whale who dominates the
  vote cannot reach a plutocratic state — the outcome bound does the work.

So the `bounded` lever (can't reach a bad state) and the `soulbound Tier-1` lever (can't split the
identity) together deliver what vote-concavity was for. Concavity becomes a Tier-2 *refinement*, not a
load-bearing requirement. **The one residual this does NOT solve is bribery / vote-buying** (renting a
real identity's vote) — a hard problem across all voting systems, costlier than wallet-splitting, and
honestly open.

**Corollary (the reusable primitive):** soulbound earned standing is a general-purpose sybil-resistance
substrate — plural funding, reputation, fair airdrops, rate-limiting, one-person-one-vote can all anchor
to PoM instead of building fragile identity layers. PoM as a *primitive the ecosystem builds on*
(PoM:Noesis ∷ PoW:Bitcoin), conditional on the moat.

## VIBE issuance — earned by validation (proposal, not yet decided)

VIBE's issuance was unspecified (TOKENOMICS.md names only its *function*, "voting + validating," status
"designed"; no mint/distribution mechanism in docs or code — verified 2026-07-14). The thesis-consistent
resolution: **VIBE is earned by validating — you earn a say over how the machine is tuned by running the
machine.** Stewardship earned by stewardship. It cannot be earned by contribution (that is PoM — would
collapse the separation) and should not be a pure genesis sale (governance-plutocracy at the origin).
This lands exactly on the tiering: **operators earn governance over the operational dials (Tier 2);
contributors (PoM) govern the measure (Tier 1)** — each authority governs the layer it has skin in. It
also completes an earn-by-activity table where every instrument is earned by a distinct productive act:

| Instrument | Earned by | Governs |
|---|---|---|
| PoM-standing (soulbound) | **contribution** (novel value) | the measure (Tier 1) |
| state-bytes (capital) | **minted by PoM-standing**, then trades | — (capital) |
| JUL (money) | **work** (PoW / energy) | — (money) |
| **VIBE (governance)** | **validation** (operating consensus) | the dials (Tier 2) |

**Double payoff:** this same choice fills the **node-operator incentive vacuum** — validating now pays
in a valuable, governance-bearing, tradeable asset, so it is no longer uncompensated externality-work.
**Scoped deliberately to validators, not all full nodes:** self-interested full nodes (exchanges,
wallets, power users) already provision themselves out of existing need; validation is the positive-
externality role that under-provisions without explicit reward, so that is where the incentive belongs.
Broader public-good infra that is *not* self-solving (archival, public RPC, bootstrap) is the separate,
optional L-INFRA slice (`DESIGN-infrastructure-incentives.md`), not bundled here.

**Security note that makes the tiering load-bearing:** VIBE-by-validation means capital-that-operates can
earn governance (buy state-bytes → stake → validate → earn VIBE → vote). That is *acceptable only because
Tier 2 is bounded + axiom-gated* — and it is one more reason **Tier 1 (the measure) must stay soulbound
PoM**, the one layer a capital→governance path would be dangerous and the one place VIBE cannot reach.

## Structure vs parameters vs moat (the closing discipline)

The anti-plutocracy guarantee is now **structural**: soulbound weight (unbuyable), the `pos ≤ pom` mix
bound (can't tilt to capital), the axiom gate (can't break invariants), the tiering (measure is
soulbound-governed). A wrong *parameter* degrades quality (cadence, participation), it cannot break the
property. So what remains for governance is a **derived parameter pass inside a safe envelope** — the
vote curve, thresholds, δ — *derived* (augmented-mechanism-design paper + testnet) and, for
consensus-touching ones, Will-ratified, **never invented**. And the **measurement moat** (un-gameable
`v(S)`) is *not* a parameter — it is open research; "just parameters" must never quietly absorb it.

## Honest status + open questions (do not invent)

- **Built (this pass)**: `Authority` classification (inert attach-point) + the `pos ≤ pom` mix bound.
- **Deferred**: the vote-*counting* layer (needs VIBE + the vote curve — MD-paper). The `√`-vs-log-vs-
  conviction curve is that pass's call; the classification is its attach-point.
- **⚑ Open considerations**: (a) whether the mix also needs a `pom ≥ pow` bound — `pos ≤ pom` guards
  *capital vs contribution* (the plutocracy axis); a PoW ceiling is a *separate* energy-security choice
  (PoW is finality-excluded, so a high `pow` is not a capital-plutocracy vector) — flagged, not built.
  (b) the exact VIBE-by-validation mechanism (per-block? stake-weighted? decay? trade-after?).
- **Designed-not-built**: the VIBE token itself; the constitutional dimension-set surface
  (`ConstitutionalPending` upstream).
- **Owed**: update Will's token-flow study guide (`internal/STUDY-GUIDE-TOKEN-FLOW.md`) to cover the four
  instruments, their earn→use→govern flows, and this tiering — so token-flow / impact / coherence
  questions are answerable from one page.
