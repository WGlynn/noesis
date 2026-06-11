# The Block Economy — PRIVATE (2026-06-11)

> Will: *"keep this all in a private repo for now, I'm tired of being front-run. I want a
> couple weeks to myself on this idea, then release it when it's mature and I have a head
> start for once."* This repo has NO public remote. Do not sync. Do not publish.

A unified system that turns JARVIS's own session history into a verifiable, owned,
valued contribution economy — and, through that, a realistic basis for decentralized
consensus and for backwards-enforcing the model layer from the governance layer.

## The stack (each layer demonstrated, not asserted)

1. **Block + provenance.** A block is the unit a session produces:
   `{id, parent, timestamp, prompt(input), response(output), checkpoints, hash}`. The
   inputs are present so *how the output came about is provable* (Will's requirement).
   Hardening: **commit-reveal** authorship — a producer commits `hash(block‖secret)` +
   signature + timestamp before revealing content, so authorship and ordering are
   un-front-runnable (VibeSwap's MEV-elimination applied to JARVIS's own provenance; the
   50%-invalid-reveal slash maps to commit-without-valid-reveal).

2. **Ownership — Bitcoin-shaped** (`block-ownership.py`, proven). Each block is locked to
   an owner public key; only the current owner's key produces a valid attestation, and
   ownership is transferable — the current owner signs a reassignment to a new key, like
   spending a UTXO. Current ownership is *derived* by folding a signed transfer log over a
   genesis owner (no mutable table to forge). Transfer voids the prior attestation; the new
   owner must re-sign. Start: a single genesis owner (Will); model is multi-owner.

3. **Value — multiplicative, pairwise → Shapley** (`block-value.py`, proven). Credit is a
   *share of a whole* (sums to 1), not an additive count. Pairwise comparison (DeepFunding's
   elicitation) feeds a cooperative game; Shapley aggregates it (VibeSwap's fairness anchor).
   **Honest finding from building it:** pairwise wins alone form an *additive* game, so
   Shapley reduces to the normalized Copeland win-share — there is no coalition *synergy* in
   pairwise data. True multiplicativity (synergy) needs an **outcome-value** `v(S)` measuring
   what a coalition of blocks achieves together. So elicitation (pairwise) and synergy
   (outcome) are separate layers — the synergy layer is the next build. The additive
   fair-share ships now; the gap is named, not hidden.

4. **Proof of Mind (PoM) score.** An agent's PoM = its accumulated Shapley credit across
   *verified, owned, provenance-complete* blocks. This is what makes decentralized consensus
   realistic: weight validators by **proof of verified mental contribution** rather than PoW
   (energy) or PoS (capital). Sybil-resistance is structural — credit requires owned blocks
   whose value was pairwise-judged and whose provenance is signed; you cannot fabricate PoM
   without actually producing verifiable, valued work. (Connects the existing MessagingPoM /
   proof-of-mind line to a concrete, computable score.)

5. **Backwards-enforcement of the model.** The block economy *is a training signal*. Each
   block is provenance-complete, owner-authenticated, and Shapley-valued — i.e. a clean,
   value-weighted dataset. With **open weights** (the sovereignty direction), fine-tune on
   high-PoM verified blocks (positive) vs caught-hallucinations (negative): the governance
   layer's accumulated truth shapes the weights. With closed weights, the same truth enforces
   the model in-context (gates block, the signed chain contradicts hallucination, correction
   is forced). Governance → training signal → model → governance verifies → **compounds**.
   The cage doesn't only constrain the mind; it teaches it. That is the maximally-moral-agent
   loop made mechanical.

## Why this unifies everything (the reason it's worth a head start)

| Borrowed from | Used as |
|---|---|
| Bitcoin | block ownership + transferable rights (UTXO fold) |
| VibeSwap commit-reveal | un-front-runnable block authorship |
| VibeSwap Shapley | value aggregation (multiplicative shares) |
| DeepFunding pairwise distillation | value elicitation |
| Contribution-graph | assignment of credit to owners |
| Merkle + Ed25519 signing | tamper-resistance of the whole |
| PoM / proof-of-mind | consensus weight from verified contribution |

It is JARVIS applying VibeSwap's own mechanisms to itself (internalize-own-protocols), and
it closes the airgap on the agent's own provenance and value.

## Status (honest)

- **Demonstrated:** ownership + transfer (Bitcoin-shaped), per-block signing, tamper-
  resistance (Ed25519-signed merkle root, keyless re-baseline caught), pairwise→Shapley value
  (differentiated shares, efficiency axiom holds).
- **Designed, not built:** commit-reveal authorship for new blocks; outcome-value synergy
  layer (true multiplicativity); PoM aggregation across blocks; open-weight fine-tune loop;
  multi-owner operation; full input-context capture per block.
- **Exposure note:** tamper-resistance + Bitcoin-shaped ownership concepts and the hooks were
  already pushed to the public WGlynn/JARVIS substrate earlier today, before this stealth
  decision. The crown jewels — `block-value.py` (pairwise→Shapley), PoM derivation, and this
  spec — are NOT public. Decision pending: make the public repo private / scrub today's
  commits / accept and keep only new work private.
