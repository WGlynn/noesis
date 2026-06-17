# Noesis HANDOFF — 2026-06-16 (PRIVATE, stealth)

Fast orientation for a fresh chat. DETAIL lives in `CONTINUE.md` (top block, newest first),
`ROADMAP.md`, and `internal/RESEARCH-NETWORK-CONSENSUS.md`. Repo: `WGlynn/noesis` (private remote).
Node: `node/`, Rust. Keep ALL of it out of public substrate (leak-gate enforces).

## Current state (suite 262 green)
The mechanism library (22 modules in `node/src/lib.rs`, ~6k lines) now has a NODE RUNTIME on top, so
the chain can be RUN, not just unit-tested. Latest increment (2026-06-16 (f), RSAW) hardened the gap #4
token gate: the mint authority is now DERIVED from issuer control of a consumed authority cell, not a
self-declared `minter` field (8th attacker-input site) — an attacker can no longer mint by naming itself
the issuer. (e) wired **gap #4 — token conservation into block validation**: `runtime::TokenTx` /
`TokenStandard` ride inside a `Block` (empty by default), and `Node::validate` rejects any block carrying
a non-conserving / unauthorized-mint movement (single-sourced from the `tokens` analogs). Validation only
— spending/persisting token state is the deploy-coupled next layer. Prior session (full-auto, Will-armed) added:

- **`node/src/runtime.rs`** — replicated state machine (orchestration only, no new mechanism):
  `Constitution` (value-matrix governance frame), `Ledger` (cells + novelty-index + PoM + height),
  `Block` (canonical commit-reveal batch), `Node` (submit/propose/validate/apply), `finalizes`
  (wraps `consensus::finalizes_hybrid`), and `finality::finalizes_pos_pom` (T3 fix — see below).
- **`node/tests/two_node.rs`** (3) — deterministic state-machine replication: two nodes hold
  byte-identical (cells, index root, PoM) after every block; presentation-independent assembly;
  non-canonical reorder rejected.
- **`node/tests/byzantine.rs`** (5) — honest node rejects wrong-height/reordered/empty blocks;
  equivocation detected; Byzantine minority can't finalize; honest supermajority can.
- **`node/src/tokens.rs`** (9) — starter ERC analogs: fungible/ERC-20 (sUDT-style), nft/ERC-721,
  multi/ERC-1155. Conservation = a PURE function of the tx (no oracle, airgap closed — Will T7).
- **`runtime::finality::finalizes_pos_pom`** (3) — T3 fix: PoW out of finality, PoS+PoM gadget,
  anti-concentration rule (PoM-60% can't unilaterally finalize = T11 capital-orthogonality in code).
- **`node/tests/gaming.rs`** (2) — adversarial-gaming loop at runtime level: a 5-identity sybil ring
  banks ≤1 cell's coverage; cross-block re-post earns 0. Un-gameable-`v(S)` holds through the live node.

**Honest scope:** the 2-node milestone is achieved IN-PROCESS (deterministic SMR, adversarial-safe).
It is NOT yet two OS processes over a network — that needs the transport (T1), genesis, persistence.

## Key decisions this session (and why)
- **Value-dimension matrix = MIXED 3-layer, NOT immutable** (Will). physics (anchor-in-realized-flow +
  noise floor; near-immutable) > constitutional (amendment rules: a dimension admitted only if it
  predicts realized value — verifier-gated; weights bounded) > governance (weights, fluid). Boundary =
  the completeness/weights cleavage (value-disputes-are-incompleteness-bias). Immutable would foreclose
  debiasing-by-completion; free-governance would reopen gameability. Code: `Constitution` struct (stub).
- **T3 — PoW OUT of finality.** `finalizes_hybrid` had a latent bug: it counted reorgeable PoW weight
  as final ⇒ PoW lag = finality-safety vector. Fix at runtime level (core left intact):
  `finalizes_pos_pom` uses `FINALITY_MIX={pow:0,pos:1/3,pom:2/3}`, 2/3-of-fast-final-set, +
  `MIN_DIM_BPS` anti-concentration (each dim must independently clear its floor).
- **T11 — capital-orthogonality is a FEATURE.** Do NOT value-weight PoS. Because PoM (60%) already
  carries subjective value, PoS (30%) must be the objective, capital-at-risk, slashable complement.
  Value-weighting stake would correlate the axes and destroy the Minotaur multi-resource security gain
  (= multi-axis-robustness + filter-coincidence). The anti-concentration rule enforces this in code.
- **CKB-shape COMMITTED** (Will); only the transport/peer layer is open.

## Research verdicts (full detail in RESEARCH-NETWORK-CONSENSUS.md)
- T1 transport → **rust-libp2p lean** (QUIC + GossipSub v1.2 + custom RFC0012 addr-gossip, skip DHT);
  tentacle #2 (lightest, TCP-only). FOUNDATIONAL ⇒ Will-confirm before build.
- T2 ML-consensus → role-bound learned signal VALIDATES the existing design; safe add = clamped
  deterministic weight multiplier; DO-NOT float-on-consensus-path / score-gates-finality.
- T9 Ergo sub-blocks → ADOPT two-tier (sub-blocks fast/revertible, ordering blocks = finality
  checkpoints), gate re-derived from contribution-weight not PoW.
- T10 Constellation → mostly hype; salvage only standing-weighted GossipSub peer-scoring (converges w/ T1).

## Open threads / next steps
- **Will-gated:** (1) T1 transport choice (rust-libp2p vs tentacle); (2) audit PoM validator/identity
  distribution before shipping finality (PoM=60%=kingmaker).
- **Pure-additive builds (no core change):** genesis/chain-spec (gap #1) · tx input/output model + wire
  token conservation into block validation (gap #4) · VRF leader selection + Phragmén (T11) · two-tier
  sub/ordering blocks (T9) · mempool policy · persistence · sync/late-joiner.
- **Design:** T5 shard + commit-reveal + pairwise wiring (VibeSwap CommitRevealAuction + PsiNet CRPC);
  constitutional-cell whose transitions obey the verifier gate (matures the `Constitution` stub).
- The 12-item gap list is in CONTINUE.md top block (d).

## Build / verify
`cd node && cargo test` (262 green). Pre-commit hooks (doc-coherence + study-guide) enforce doc
freshness — if blocked: `python scripts/study-guide.py && python scripts/doc-coherence.py --stamp`,
then `git add -A`, retry. Watch for the `N tests` regex false-positive (don't write "(9 tests)" in a
doc — the checker compares it to the suite total).
