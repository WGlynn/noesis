# DESIGN — inc-2b: the fork choice (heaviest PoS+PoM finality support) — ⚑ WILL REVIEW BEFORE BUILD

> Status: **ready-for-critique, NOT built.** Extends `DESIGN-multi-producer-nakamoto.md` (topology B,
> fork-choice option (ii) ratified 2026-07-19). inc-1 (reorgeable ledger, `node/src/reorg.rs`) and
> inc-2a (`Block.parent_hash`, the fork-tree link) shipped. This designs the piece inc-1 left a
> placeholder for: **what weight picks the canonical fork.** Consensus-critical ⇒ design-gate first.

## 0. The gap
inc-1's `ReorgTip::try_reorg` uses a placeholder comparator (`cand.work > tip.work`, heaviest PoW).
Topology (ii) says fork choice is **heaviest PoS+PoM finality support**, NOT PoW — keeping PoW
finality-excluded (`FINALITY_MIX`, `runtime.rs:1569`). But the finality gadget `finalizes_pos_pom` takes
a FLAT `voters_for` set — it has no notion of *which block* a validator is voting for. Multi-fork choice
needs per-fork vote attribution. That attribution model is what inc-2b adds.

## 1. Mechanism — LMD-GHOST weighted by finality support (not PoW)

Adapt Ethereum's LMD-GHOST, swapping attestation-weight for Noesis's PoS+PoM finality weight:

- **Attestation model (2b-i).** A validator broadcasts a **latest vote** = `(validator_id,
  target_block_hash)`. *Latest-message-driven* (LMD): only a validator's most recent vote counts, so a
  validator has exactly one live vote in the fork tree at a time. The vote's weight is the validator's
  PoS+PoM dimensions — the SAME weights `finalizes_pos_pom` already computes (reuse, don't reinvent).
  `target_block_hash` = `header_digest` (inc-2a already binds the parent, so the tree is well-formed).
- **GHOST traversal (2b-ii).** Canonical tip = start at the highest finalized checkpoint; at each fork,
  descend to the child whose **subtree** carries the greatest summed latest-vote weight (Greedy Heaviest
  Observed SubTree); repeat to a leaf. Weight is finality-support, not work.
- **Comparator swap.** `ReorgTip::try_reorg` compares `ghost_weight(cand_tip)` vs `ghost_weight(cur_tip)`
  instead of `.work`, still floor-bounded by the finalized checkpoint (reorg never crosses it).
- **Equivocation = slashable.** Two conflicting latest-votes from one validator (same height / sibling
  forks) is an equivocation — reuse the existing guard (`finalizes_with_equivocation_guard`,
  `epoch_equivocators`, `runtime.rs:1664`): slash-before-count, the offending votes don't weigh.

**Why this fits:** PoW still only secures production/ordering/Sybil-cost + mints JUL; it never selects
the chain. The SAME PoS+PoM weights drive both fork choice (GHOST) and finalization (the gadget), so the
two layers are consistent by construction — the fork the validators are building toward is the fork they
will finalize.

## 2. ⚑ Decisions for Will
1. **Vote weight = which dims exactly?** PoS+PoM per `FINALITY_MIX` (⅓:⅔)? Apply the anti-concentration
   floor (`MIN_DIM_BPS`) per-vote in GHOST, or only at the finalization checkpoint? (Lean: raw PoS+PoM
   weight for GHOST speed; anti-concentration only gates *finalization*, not fork choice.)
2. **Vote transport:** piggyback votes on block gossip (a producer's block implies its vote for its own
   parent) vs a separate attestation gossip channel? (Lean: separate lightweight attestation frames —
   validators must vote without producing.)
3. **Do non-producing validators vote?** LMD-GHOST needs the whole validator set attesting, not just
   producers. Confirms the attestation layer is new consensus data (the single-producer model had no
   standalone votes).
4. **Tie-break** when two subtrees weigh equally: lowest block hash (deterministic), matching inc-1's
   reorg tie rule.

## 3. Build order (once ⚑ answered) — each RED-first + Council pass, Will-reviewed
- **2b-i:** the attestation type (`Vote{validator, target_hash}`) + LMD store (latest-vote per validator)
  + equivocation slashing wired to the existing guard.
- **2b-ii:** GHOST traversal over the fork tree (from `reorg`'s block pool) → canonical tip; swap the
  `try_reorg` comparator to `ghost_weight`.
- Then inc-3 (wire `finalizes_pos_pom` as the checkpoint calling `reorg::finalize_to`) → inc-4 (gossip).

## 4. Honest scope
Consensus-critical and the second-biggest piece after inc-1. The attestation model is NEW consensus data
(the chain has never had standalone validator votes). NOT a launch blocker — the single-node testnet is
live without it. This is the path to a genuinely decentralized fork-choosing network.
