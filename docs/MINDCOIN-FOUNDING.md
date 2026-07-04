---
title: "MindCoin ($MIND) — Founding Document"
subtitle: "The Ethereum Cogcoin: a fair-launch proof-of-mind subsidy"
date: "2026-07-03"
---

# MindCoin ($MIND)

A fair-launch token that pays minds for proven new information, the way CogCoin pays
LLMs for proven dense language. CogCoin is proof-of-language on Bitcoin. MindCoin is
proof-of-mind on Ethereum: the same information-density thesis, generalized from
sentences to any contribution.

Status discipline is used throughout: **built**, **designed**, **open**. Nothing is
rounded up. The honest limits are stated as plainly as the vision, because for this
project the honesty is the moat.

---

## 1. The thesis (Economitra)

Value is the transmission of real information. Strip redundancy and noise, and what
remains is signal. This is Economitra: information theory applied to value.

CogCoin is this thesis in production. Its "proof of language" scores a sentence by a
256-model ensemble and pays for the highest information density per byte; the only
winning move is to write genuinely good sentences.

MindCoin generalizes it. Noesis's `temporal_novelty` values a contribution by the
new content it adds that no earlier contribution contained. Proof-of-language is the
special case (the contribution is a sentence); proof-of-mind is the general form (the
contribution is anything). **built** in the reference reduction.

**Honest calibration.** `temporal_novelty` is a dictionary-growth novelty count in the
spirit of LZ78 parsing, over fixed-width content shingles. It is a real but crude
estimator of conditional information (how much this contribution adds given the chain
so far). It is *not* a channel-capacity measurement and carries no entropy-rate
convergence guarantee. We say "a measured quantity of conditional information, with
redundancy zeroed by construction and noise floored," never "Shannon capacity."

---

## 2. What MindCoin is

A fair-launch reward token with three structural properties:

- **Zero premine.** The token constructor mints nothing. **built.**
- **Single mint source.** Mint is gated to one address (the export hub), set once and
  then immutable, so future emission cannot be redirected off-schedule. **built.**
- **Value-router, not a casino.** Every $MIND ever minted is collateralized by
  strictly positive proven new novelty and routed to whoever produced it, by a
  checkable algorithm, with no rent-seeking skim on the emission.

Note on the three-token discipline: $MIND is a reward-routing instrument, not the money
layer. JUL remains the energy-pegged money layer; VIBE is governance; CKB-native is
state-rent capital. $MIND must not be positioned as elastic money.

---

## 3. The mechanism (designed, adversarially gated)

**The meta-block.** One meta-block is one finalized PoM standing on the optimistic
export layer that is already **built and hardened** (14/14 Solidity tests, 270/270 Rust
tests, cross-language Merkle conformance). A bonded operator posts a standing; after a
challenge window with no challenge it finalizes. Challenged standings are always
discarded. A new rule requires each standing to carry strictly more total proven
information than the last, so every subsidy is collateralized by new information and
empty blocks cannot grind the schedule.

**The subsidy schedule.** Bitcoin-form, denominated in meta-block height:

- Initial subsidy: 3.125 $MIND per meta-block.
- Halving: every 210,000 meta-blocks (count-based, so the cap holds under any cadence).
- Max supply: 1,312,500 $MIND, fixed forever. This equals Bitcoin's remaining issuance
  at block 840,000, the start of the 3.125-subsidy era, so MindCoin joins Bitcoin's arc
  in progress rather than replaying it.
- The export layer's own 10-minute challenge-window floor makes Bitcoin's block cadence
  a built-in speed limit: at most 144 meta-blocks per day.

**The split.** Each subsidy divides 91 / 6 / 3:

- 91% to the scored contributors (CogCoin's "miners"). This is a compile-time constant,
  so the router cannot be reconfigured into a skim *absent a hub upgrade* (see limits).
- 6% to the proposer who ran the reduction, posted the standing, and carried bond and
  gas (CogCoin's "domain anchorer").
- 3% to a security budget that pays winning challengers, compounding while unused.

**Delta pricing.** Each meta-block pays for the *new* information finalized in it, not
for lifetime cumulative score. Paying pro-rata against lifetime score would pay rent to
old contributions forever, violating both the value-router soul and the thesis
(redundant equals zero equals no reward). Contributors claim their share by Merkle proof
against the standing's payout root, minted lazily; unclaimed value is never minted and
can never become governance revenue.

---

## 4. Trust model

- **Happy path: no quorum.** One bonded proposer posts; finalize is permissionless.
- **Safety: one honest party is enough.** A single bonded challenger freezes a bad
  standing so it can never be consumed. This does not depend on the dispute resolver.
- **Liveness: the resolver is not a single point of failure.** After a bounded window,
  anyone can reopen a stuck slot.

**v1 is an optimistic oracle, not a metaprotocol.** Contribution data lives off the host
chain; the system carries a bonded challenge game, a governance dispute resolver, and an
upgrade authority that CogCoin does not have. "Trustless like CogCoin" is earned only at
v2: contribution data posted to EIP-4844 blobs (making challenges permissionless from
host data) plus a ZK or RISC-V one-step-proof resolver replacing the governance
adjudicator. This is the "A now, C later" path.

---

## 5. The honest limits (the credibility section)

**v1 pays for novelty volume through an entropy floor, not for quality.** This is the
most important sentence in this document.

The challenge game verifies *faithful computation*, not *value quality*. CogCoin's
defense against junk is its 256-scorer ensemble. MindCoin's analog is a learned quality
gate, which is **open**: its first real-data test returned null. Until it ships, low-
entropy generated content (word-salad, enumerated combinations) passes the entropy floor
and can capture the contributor pool, and because such a standing is computed honestly,
re-derivation matches and the challenge game structurally cannot reject it.

So the honest v1 claim is **"a fair-launch novelty subsidy with an entropy floor,"** not
"ungameable proof-of-mind like CogCoin." The fixed schedule bounds the damage: gaming can
only redistribute a fixed per-meta-block pie, never inflate supply. But "redistribute"
means honest contributors are the counterparty. The real economic-security parameter is
the Noesis-side cost to mint a unit of exportable novelty (state rent per byte) versus
the $MIND paid per unit; the subsidy should activate only when that cost exceeds the
reward.

**Un-gameability is proven for demonstrated vectors only:** exact duplicates score zero,
sybil clones score zero, reordering is invariant. **built and tested.** Encoded-noise
evasion of the entropy floor and content-level quality gaming remain **open**.

**A fixed ship-blocker.** The export seam originally emitted novelty without the entropy
floor, so random bytes would have earned maximal reward in exactly the tree that pays
money. The fix routes the exported value through the entropy floor before any subsidy is
paid.

---

## 6. Status summary

- **Built and hardened:** the optimistic export layer and the fair-launch, minter-locked
  token. 14/14 Solidity tests, 270/270 Rust tests, cross-language Merkle conformance,
  adversarial-review fixes applied (resolver cannot finalize a challenged standing,
  liveness without the resolver, challenge-window floor, minter lock).
- **Designed and adversarially gated (this document):** the meta-block subsidy schedule,
  the 91/6/3 split, and delta-priced contributor claims. Reviewed by a nine-agent design
  workflow across four adversarial gate lenses; all returned "concern" (core sound, real
  fixes required), not "pass" and not "fail." The fixes are folded into the design above.
- **Open:** the learned quality gate; blob data-availability; the ZK/RISC-V resolver; the
  economic-security inequality for subsidy activation.

---

## 7. Open founding decisions

These are genuine calls, not defaults. Each changes the founding parameters.

1. **Halving interval.** 210,000 meta-blocks keeps the clean Bitcoin-mirror cap but
   stretches epoch 0 to roughly 24 years at an hourly standing cadence. A cadence-fitted
   interval (around 35,000) gives a roughly 4-year arc but loses the clean 1,312,500 cap.
   The constant cannot be re-tuned later.
2. **Quiet-chain floor.** Ship without a minimum-information threshold per meta-block
   (Bitcoin's early-cheap-coins shape, recommended) or add one to blunt self-dealing on
   a near-idle chain.
3. **Ossify now or caveat.** Burn or timelock the hub's upgrade authority now, which
   makes the "immutable 91%" claim literally true, or keep upgradability and state the
   residual trust caveat plainly.
4. **Identity bridge.** The payout-root commits an ETH address per soulbound contributor,
   sourced from an in-band registration; its authenticated cell format needs deciding.
   Default until then: accrue-unclaimable until registered.
5. **Name timing.** Lead with "Ethereum Cogcoin" now, with the v1 caveats stated, or wait
   until the blob-DA upgrade earns the full analogy. (Ticker note: an ERC-20 "MIND"
   already exists on Ethereum; the public symbol is a launch-day decision.)
