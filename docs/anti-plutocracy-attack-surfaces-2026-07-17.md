# Anti-plutocracy: the indirect-capture attack surfaces (longer brief)

> Companion to the one-paragraph explanation ("capital can buy exposure, never consensus standing").
> That paragraph asserts **"structurally free from plutocracy"** — a *theorem-level* claim, not a design
> description. A serious researcher will not attack the direct path (buying weight); the soulbound split
> closes that. They will attack the **indirect** paths: delegation, Sybil identities, reputation
> accumulation, and control of the value-measurement layer. This brief makes each explicit.
>
> Discipline (do not round up): ✅ built + tested · 🟡 designed-not-built · 🔬 open research. Constants are
> cited by name + file (`MIN_DIM_BPS`, `FINALITY_MIX`, `pos ≤ pom` in `check_mix`); exact line on request,
> never guessed. Grounded in `internal/STUDY-GUIDE-TOKEN-FLOW.md` (numbers verified 2026-07-14) + `node/src`.

---

## First, state the claim precisely (so the attack has a target)

"Structurally free from plutocracy" is precise for **one specific path** and should be scoped to it:

> **The direct path — money → consensus weight, or money → a say over what counts as contribution — is
> closed by construction, not by policy.** Consensus weight is soulbound PoM-standing (never a token, so
> never for sale). The anti-concentration floor (`MIN_DIM_BPS`) forces the capital dimension and the
> contribution dimension to *each* independently supply their share of finality, so capital cannot finalize
> a block without contribution's consent. Governance of the *measure* (θ_sim, vesting, mix direction) is
> Tier-1, gated to earned soulbound standing; `pos ≤ pom` (in `check_mix`) means governance can never tune
> the mix to capital-rule; and every amendment must pass the axiom-preservation gate regardless of who
> votes. **This half is ✅ built + tested.**

That is a real, defensible structural claim. It is *not* the same as "no wealthy actor can ever gain
influence." The honest work is naming the four indirect paths and saying, for each, exactly how far the
structure carries and where it hands off to an open problem.

---

## Surface 1 — Delegation (renting the franchise)

**The attack:** In ordinary PoS, plutocracy re-enters through delegation: I cannot *be* a big validator, but
I can rent the weight of many small ones. Even if PoM-standing is non-transferable, can a holder **delegate**
its consensus weight (or its Tier-1 vote) to a capital-aligned operator, reintroducing the capital→influence
channel through the back door?

**Honest position:**
- Soulbound-ness blocks **transfer**, and the governance model already reduces vote-Sybil to "can you Sybil
  PoM?" — a soulbound identity can't be split across wallets. So the *asset* cannot move. ✅
- But soulbound-ness does **not** by itself block **renting**. Delegation is the consensual form of
  vote-buying, and the study guide already marks the residual honestly: *"bribery / vote-buying (renting real
  identities) — hard everywhere, honestly unsolved."* 🔬
- The precise honest statement: **the protocol makes weight non-transferable; it does not make the franchise
  non-rentable.** Whether Noesis exposes a *delegation primitive* for PoM weight is a design decision, and the
  safe default for the anti-plutocracy claim is **no first-class weight-delegation** (make renting an
  out-of-protocol, uncoordinated, individually-priced act rather than a smooth on-chain market). That raises
  the cost of a rental cartel but does not dissolve it.

**Verdict:** transfer closed (✅); *renting/bribery* is the honestly-open residual that no franchise system —
including Bitcoin and every PoS chain — has dissolved. Don't claim it. Name it as shared open ground.

---

## Surface 2 — Sybil identities (manufacturing standing by splitting)

**The attack:** Split one actor into many identities so that fake or duplicated work reads as broad,
independent contribution — inflating the contribution dimension the floor is supposed to protect.

**Honest position:**
- **Structural Sybil of the same work is closed and tested.** Free-identity Sybils are priced to zero by a
  null-player rule; the value signal is **identity-blind** (it scores the identity-quotient graph), so a split
  is bit-identical to the honest form and buys exactly zero — the depth-split self-launder went from a +16.7
  gaming gain to 0 in code (closed 2026-07-03, tested). ✅
- **What stays open is Sybil at the *semantic* layer.** Many identities each submitting genuinely-distinct-
  looking-but-low-value or paraphrased work depends on the value measure refusing to credit it — and paraphrase
  detection on the deployed path, plus fabricated parent edges, are the two named-open vectors. The general
  guarantee (invariance under *any* structure-preserving relabeling) is graph-isomorphism-hard, and split/merge
  is a monoid, not a group, so it isn't cleanly invertible. 🔬

**Verdict:** identity-split of the same value = closed (✅); semantic Sybil reduces to the open value-measure
problem (Surface 4). Say which is which.

---

## Surface 3 — Reputation accumulation (farming a large franchise over time)

**The attack:** Even if each unit of standing must be *earned*, a well-funded actor can pay many contributors
(or agents) to produce genuine-enough contributions and **accumulate** a dominant franchise — then wield it as
capital-by-proxy, including a dominant say over the measure (Tier-1).

**Honest position — this is the sharpest of the four, concede its strong form:**
- Legitimate accumulation is **by design**: if you fund work that genuinely adds value the network keeps, you
  earned standing — that is contribution, not plutocracy. The claim is "capital can't *buy* weight," not
  "wealthy actors can't *contribute*." The novelty floor + vesting window raise the per-unit cost of farming.
- **The un-floored gap (state it plainly):** `MIN_DIM_BPS` is a **per-dimension** balance (PoS vs PoM each
  supply their share), **not a per-identity cap *within* a dimension.** So the floor guarantees capital can't
  finalize without contribution; it does **not** guarantee that the contribution dimension is itself
  *decentralized*. A single dominant contributor concentrating the PoM dimension is a distinct surface the
  floor does not speak to — and because Tier-1 governance of the measure is weighted by standing, concentration
  **compounds** into a dominant say over what counts as contribution. *(Verified in code against `dim_ok`
  (`runtime.rs`, `MIN_DIM_BPS = 5000`): the rule is `weight_for >= weight_all * 0.5` applied **per
  dimension** — the voters finalizing a block must hold ≥50% of the PoS dimension's total weight AND ≥50%
  of the PoM dimension's. `weight_for` / `weight_all` are *sums*, so it is a within-dimension agreement
  threshold, **not** a per-identity cap: a single identity holding >50% of the PoM dimension satisfies the
  PoM floor alone, so "contribution's consent" can degenerate to one dominant contributor's consent.
  Residual #2 is confirmed in code, not conjectured.)*
- **Two honest design questions this raises:** (a) does PoM-standing **decay**, or is accumulation monotonic
  and permanent? (state-bytes decay as state-rent; whether the *franchise* decays is the relevant knob for
  concentration). (b) Should there be an explicit **intra-dimension** anti-concentration measure, or is the
  cold-start "large, decentralized contribution set" the only defense? Both are 🔬/⚑, not settled.

**Verdict:** buying-standing-directly = closed (soulbound + earned); **concentration of earned standing** is a
real, partly-un-floored surface that compounds with Surface 4 — the honest theorem-gap behind the phrase.

---

## Surface 4 — Control of the value-measurement layer (capturing "what counts")

**The attack:** If you control how contribution value is *measured*, you control who gets standing — a subtler
capture than buying weight. Capture can come two ways: (a) **govern** the measure, or (b) **game** the measure.

**Honest position — split the two, because their status is opposite:**
- **Governing the measure is structurally defended (✅).** Amending θ_sim / vesting / mix direction is Tier-1,
  gated to **earned soulbound standing** — not a bought token. `pos ≤ pom` means no amendment can tilt the mix
  to capital-rule. And the **axiom-preservation gate** (`verify_amendment` + the Pragma coherence socket) is
  the real backstop: every amendment must preserve the base axioms *regardless of who voted*, so even a
  governance majority can't legislate a plutocratic measure into place.
- **The soundness of the measure itself is the open moat (🔬).** The structural relabeling classes (padding,
  near-dupes, Sybil splits, cyclic rings, self-report rings) are closed with **no oracle**, by construction and
  tested. The **semantic** judgment — is this paraphrase the same idea, is this contribution actually valuable —
  cannot be made replica-deterministic, so it can't sit on the consensus path; it lands in a learned model that
  shapes the training signal. **That learned `v(S)` has returned null against a fixed proxy three times** (two
  DeepFunding, one 300k-crate deep-ancestry graph). It is marked null-tested, never rounded up.
- **Why this is not a safety hole:** finality safety rides on PoW-exclusion + the anti-concentration floor,
  **not** on the value measure. So the measure can be honestly open while the chain stays safe — the airgap
  closes structurally for the relabeling classes and remains open for the semantic/quality layer.

**Verdict:** governance-capture of the measure = closed (✅); **soundness** of the measure = the open research
moat (🔬). The reviewer's real target lives here — and it is exactly the problem to hand a research collective.

---

## Synthesis — keep the phrase, scope it, and name the residuals

"Structurally free from plutocracy" is **true and precise for the direct path** (money → weight, money → a say
over the measure): closed by the soulbound split, the anti-concentration floor, `pos ≤ pom`, and the axiom
gate — built and tested. The four indirect surfaces resolve to **three named residuals**, and honesty about
them is what earns a serious researcher's trust:

1. **Franchise renting / bribery (delegation)** — non-transferable ≠ non-rentable. Honestly unsolved
   *everywhere*; shared open ground, not a Noesis-specific hole. 🔬
2. **Intra-dimension concentration (reputation accumulation)** — the floor is per-dimension, not per-identity;
   concentration of earned standing is un-floored and compounds into measure-governance. Design questions
   (standing decay? intra-dimension cap?) are open. 🔬 / ⚑
3. **Soundness of the value measure (Sybil-semantic + measurement-capture)** — governance of the measure is
   soulbound-gated (✅); the *un-gameability* of the learned `v(S)` is the open moat (🔬, null 3×).

The one-sentence version for the brief: **capital cannot buy consensus weight or a say over the measure — that
is structural and tested. What remains open is not a way to *buy* the franchise, but three ways to *bend* it:
rent it, concentrate it, or fool the measure that grants it — and the second and third both reduce to the one
open research bet, the un-gameable value measure.** That is the honest shape of the theorem: a proven structural
core with a precisely-bounded open frontier, not a solved problem.
