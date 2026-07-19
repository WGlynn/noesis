# DESIGN — multi-producer Nakamoto (the reorgeable tip beneath the finality gadget)

> Status: **ready-for-critique, NOT built.** Will chose topology (B) over (A) on 2026-07-19 (see
> `DESIGN-live-gossip.md`). This is the design for his sanity-check BEFORE any consensus-critical code.
> It is the largest single build in the node's history and touches flow + standing semantics, so it
> ships increment-by-increment, each RED-first + Council-passed + Will-reviewed. Nothing autonomous-blind.

## 0. The one-sentence design

Activate the **reorgeable PoW tip** that Noesis's finality gadget was always designed to sit on top of:
many producers mine concurrently, a heaviest-cumulative-work fork choice picks the canonical tip, and
the **existing** PoS+PoM finality gadget checkpoints a prefix as irreversible — reorg may only touch the
suffix ABOVE the last checkpoint.

## 1. Why this is activation, not reinvention (grounded)

The architecture already separates the two layers by construction:

- **Overall consensus mix (NCI)** = `pow 0.10 / pos 0.30 / pom 0.60` (`node/src/lib.rs:3856`; BPS
  1000/3000/6000). PoW's 10% secures **production / ordering / Sybil-cost**, never safety.
- **Finality mix EXCLUDES PoW** = `Mix { pow: 0.0, pos: 1/3, pom: 2/3 }` (`node/src/runtime.rs:1569`),
  with anti-concentration `MIN_DIM_BPS = 5000` (`:1581`) forcing PoS AND PoM to each independently
  supply ≥50% of their own dimension. The gadget `finalizes_pos_pom` (`:1586+`) is already built and
  covered by the 235-test core.
- The code's own rationale (`runtime.rs:1560`): "PoW finality is probabilistic/reorgeable... the
  production pattern (Casper-FFG / GRANDPA / Babylon / Decred) keeps the probabilistic layer OUT of the
  immediate finality weight."

So the finality gadget is the **checkpoint layer** of a Casper-FFG / GRANDPA-shaped design. What is
missing is the **probabilistic Nakamoto layer underneath it** — because single-producer instant
finality (`chainspec.rs:177`, "single ... proposer") let us skip building reorg. B builds that layer.

## 2. What exists vs what is missing

**Exists (reuse wholesale):**
- PoS+PoM finality gadget `finalizes_pos_pom` (tested) — becomes the checkpoint rule.
- PoW mining + ASERT difficulty retarget (`noesis_core::pow::next_target`) — targets the block interval
  so orphan rate is bounded like Bitcoin.
- Cumulative-work clock (`block_work`, work-clock ceiling) — the natural fork-choice weight.
- Per-block producer/coinbase (`chainspec.rs:161`), durable length-framed store (`store`), framed TCP +
  `sync` replay + `gossip` dedup/broadcast primitive.

**Missing (the build):**
1. **Reorgeable ledger.** `Node::apply` is forward-only. Need snapshot-at-checkpoint + rollback +
   re-apply along a new tip.
2. **Multi-tip tracking + heaviest-work fork choice**, bounded below by the last finalized checkpoint.
3. **Finality-over-reorg wiring** — advance a `finalized_prefix` marker via `finalizes_pos_pom`; reorg
   is forbidden to cross it.
4. **Live gossip of competing blocks** — broadcast mined block, peer validates + attempts extend/reorg
   + relays (dedup). (This is the easy part; it only becomes meaningful once 1-3 exist.)

## 3. Mechanism

### 3.1 Fork choice
Canonical tip = the valid tip with the **greatest cumulative PoW work** (sum of `block_work`) that
descends from the highest finalized checkpoint. Ties broken by lowest block hash (deterministic). This
is Bitcoin's heaviest-chain rule with the finality floor bolted on.

### 3.2 The finality floor (the safety/liveness split)
`finalizes_pos_pom` periodically finalizes a checkpoint (cadence = ⚑, §5). Everything at or below the
highest finalized height is **immutable** — fork choice may only reorganize the suffix above it. Final
means final: a reorg attempting to rewrite a finalized block is rejected, not applied. This is exactly
what `FINALITY_MIX` (PoW-excluded) buys — the reorgeable layer can never endanger a finalized prefix.

### 3.3 Block production
Many producers mine concurrently on their view of the heaviest tip. ASERT difficulty keeps the interval
near target so concurrent-mine collisions (orphans) stay rare and shallow, resolved by fork choice —
standard Nakamoto. Testnet difficulty stays low (worthless JUL by construction), so orphan handling,
not hashpower, is what we are actually testing.

### 3.4 Standing + novelty under reorg — THE load-bearing correctness requirement
This is the deepest part and the biggest ⚑. PoM standing (`pom_scores`) and the novelty index
(temporal-novelty seen-set / SMT root) are **path-dependent**: a contribution's value depends on what
came before it. Therefore a reorg must roll back **standing and the novelty index**, not just token
UTXOs. A contribution in an orphaned block must lose its standing and release its novelty-index entry,
or the same content re-submitted on the winning tip would be wrongly rejected as duplicate (and standing
would double-count across forks). Concretely: the reorgeable ledger must snapshot `(cells, novelty
root, pom_scores, token set, work clock)` at each finalized checkpoint and restore the full tuple on
rollback. Anything less is a consensus split in value-space. This requirement is non-negotiable for
correctness and is where the build's real risk lives.

## 4. Build sequence (decision-unblocked first)
- **inc-1 — reorgeable ledger.** Snapshot-at-checkpoint + rollback + re-apply, over the FULL state
  tuple incl. novelty index + `pom_scores` (§3.4). Foundational, consensus-critical. RED-first on
  "orphaned contribution loses standing and frees its novelty slot."
- **inc-2 — multi-tip + heaviest-work fork choice**, floor-bounded by the finalized checkpoint.
- **inc-3 — finality-over-reorg wiring.** Advance `finalized_prefix` via `finalizes_pos_pom`; reject
  any reorg crossing it. (Gadget exists; wire it to the tip.)
- **inc-4 — live gossip + multi-producer driver + `--peers`.** Broadcast, reorg-on-receive, dedup.

## 5. Decisions — RATIFIED (Will 2026-07-19; do NOT re-litigate)
1. **Two-tier model** ✅ (reorgeable PoW tip + PoS/PoM-finalized prefix bounding reorg). The whole
   design rests here.
2. **Checkpoint cadence = a configurable `checkpoint_interval` PARAMETER**, not a hard-coded number.
   The mechanism is cadence-agnostic (works for any N≥1: N=1 → near-instant finality, N large →
   Bitcoin-like reorg window). Testnet-pinned, tune-when-live (the sub-block-knob + ASERT "no economic
   number hard-coded" precedent). Build parameterized; default a sane testnet N.
3. **Standing + novelty roll back with the chain** ✅ — the mandatory §3.4 invariant. The reorgeable
   ledger snapshots and restores the FULL state tuple.
4. **Open CPU mining on testnet** ✅ (worthless JUL makes it safe; tests real orphan dynamics).
5. **Finality floor is the reorg bound; NO separate max-reorg cap in v0** ✅ (floor +
   `checkpoint_interval` already bound depth). Revisit only if a concrete need appears.

## 6. Honest risk
This is the node's biggest build and it is consensus-critical. §3.4 (reorg-safe standing + novelty) is
the part most likely to hide a subtle split. Mitigation: strict increment order, RED-first tests per
increment, a Council/critical-qa adversarial pass on inc-1 and inc-3 specifically, and your review at
each seam. It is NOT a launch blocker — the single-node `--serve-api` testnet ships without any of it;
this is the path to a genuinely decentralized chain, built after (or alongside) go-live.
