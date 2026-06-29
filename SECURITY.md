# Security Policy

## Reporting a vulnerability

noesis is a pre-launch protocol; there is no public network and no funds at risk.
If you find a flaw in the consensus, value, dispute, or execution layers, please
report it privately rather than opening a public issue. Open a
[GitHub security advisory](https://github.com/WGlynn/noesis/security/advisories/new)
or contact the maintainers directly. We aim to acknowledge within 72 hours.

Please include: the component (`node` module or on-VM type-script), a minimal
reproduction (ideally a failing test against the suite), and the impact you believe
it has on a deployed network.

## Security model

noesis is designed so that the cheapest path to influence is to contribute, not to
attack — the attack surface is dissolved structurally rather than patched. Two
invariants carry most of the weight:

- **Don't let the attacker choose a security-critical input.** Every value the chain
  acts on — index identity, commit ordering, the finalization clock `now`, the
  validator set — is re-derived from consensus on-VM and rejected if it cannot be
  reconstructed. A free, transaction-chosen value is self-assertion, not a check.
- **History is verified, not trusted.** Novelty, similarity, and finalization are
  proven against committed state (a sparse-merkle novelty index, header-sourced
  time, the bonded validator registry), with classification that makes omission and
  stale-state attacks structurally impossible rather than merely detectable.

A standing internal audit of these surfaces lives in the design docs (see the
attacker-choosable-input audit and the per-mechanism critical-QA notes in `docs/`).

## Attack-class defense matrix

The idea (contribution as a conserved, measured quantity) is not the thing critics
attack — the execution is. The four classes anyone will probe are **gameability,
DoS/spam, double-spend, and chain rollback.** Each is mapped below to its mechanism,
its honest status (✅ built & tested · 🟡 designed, not built · 🔬 open problem), and
where to verify it. Status is the truth line; we do not round designed up to built.

### 1. Gameability of the contribution measure `v(S)` — ✅ built & tested (reference + runtime)
The whole protocol rests on `v(S)` being un-gameable: the cheapest path to score must be
to genuinely contribute.
- **Copy / padding / near-duplicate spam → 0.** Temporal-novelty scores a cell only for
  coverage novel vs all earlier-committed cells; the similarity floor zeroes near-duplicates
  (a few bytes flipped). Now enforced on the **live consensus path**, not just in the lib:
  `pom_scores_with_similarity_floor_q16` (`Constitution.theta_sim_q16`, default 0.95).
  Verify: `node/tests/gaming.rs` (identical-content ring, cross-block re-post, paraphrase-padding
  ring all bank ≤1 cell), commit `9dba5dd`, ROADMAP `(xx)`.
- **Reorder-to-steal-novelty → fails.** Novelty is assigned in canonical commit order
  (height, then XOR-seeded in-block slot), invariant to presentation order. `novelty_in_commit_order`.
- **Collusion / citation rings → detected + slashed.** Mutual cross-citation = circulation
  (`attribution_circulation`); directed k-cycles = Helmholtz–Hodge harmonic energy
  (`attribution_cycle_energy`); both feed a bounded `collusion_slash` that burns ring standing
  while sparing honest builders (griefing-resistant — inbound-only edges can't frame a victim).
- **🔬 Open:** the learned `v(S)` moat had its first real-data test return **NULL** (did not
  beat a fixed proxy on real DeepFunding labels — ROADMAP `(ww)`); the faithful provenance-feature
  port is the open test. And the strongest known hardening, an **isomorphism-invariance gate**
  (score `v(S)` invariant under structure-preserving relabeling), is an open research problem for
  coalitional measures (ROADMAP cand-A). We claim un-gameability for the *demonstrated* vectors,
  not as a finished proof.

### 2. DoS / spam — split into two distinct threats (be precise here)
- **Economic spam (point-farming): ✅ built & tested.** Flooding the chain with junk or variations
  is *unprofitable by construction* — it scores 0 (class 1 above), so there is no incentive to do it.
- **Validator sybil cost: ✅ built & tested.** Consensus eligibility requires `MIN_STAKE`; the number
  of eligible identities is bounded by `capital / MIN_STAKE` (`audit_a3_sybil_splitting_is_bounded_by_min_stake`).
- **Dispute/escalation spam: ✅ bounded.** A griefer spamming challenges pays doubling bonds (`2^k·B`)
  — see `docs/DISPUTE-SLASHING.md`.
- **🟡 Resource-DoS (the honest weak leg):** flooding the node with *cheap, well-formed-but-worthless*
  submissions to exhaust mempool/compute is **unprofitable but not yet resource-bounded.** The economic
  gate removes the *incentive*; a submission bond / rate-limit / commit-deposit that bounds the *resource
  cost* of evaluating junk is **designed, not built.** This is the leg to harden next and the one a
  serious reviewer should press on — we flag it rather than overclaim.

### 3. Double-spend — ✅ built & tested (reference) / 🟡 deploy-coupled crypto
- Single-use UTXO retirement: a consumed authority/value cell is retired on apply, so a later block's
  existence check fails for an already-spent cell. **Within-block** and **cross-block** double-spend
  both rejected. Verify: `runtime.rs::token_txs_conserve_and_single_use`, tests at
  `runtime.rs:1099/1153/1221`, `intra_tx_double_mint_is_denied` (`ckb_vm_proven_e2e.rs`).
- Mint authority is *derived* from control of a consumed authority cell, never a self-declared field.
- **🟡 deploy-coupled:** the cryptographic nullifier set + on-VM UTXO-set retirement are the deploy
  layer; the reference model proves the rule, the on-chain enforcement ships at deploy.

### 4. Chain rollback / finality safety — ✅ built (reference) / 🟡 on-VM mirror designed
- **PoW is excluded from finality.** PoW is probabilistic and reorgeable — a finality-safety hazard —
  so finality routes through `finalizes_pos_pom` (PoS + PoM only). PoW still secures production/ordering.
- **2/3 supermajority + anti-concentration:** both the PoS and PoM axes must clear a floor; no single
  axis (e.g. a PoM whale) can finalize unilaterally.
- **Equivocation + stale-state:** double-votes detected (`is_equivocation`), early-reject for invalid
  reveals, retention-decay on liveness. A Byzantine minority cannot finalize; wrong-height / reordered /
  empty blocks are rejected. Verify: `node/tests/byzantine.rs`, `two_node.rs`.
- **🟡 on-VM:** the finalization rule is recomputed in pure fixed-point and the on-VM type-script mirror
  is specified (`docs/ON-VM-FINALIZATION.md`); the live wrapper is wired and tested at the reference
  layer, the on-VM enforcement is the remaining build.

> **Reviewer's shortcut:** the two legs to press hardest are **(1) the `v(S)` un-gameability proof**
> (demonstrated for known vectors, isomorphism-invariance open) and **(2) resource-DoS bounding**
> (incentive removed, resource cost not yet bounded). We name both as open on purpose.

## Scope

In scope: the Rust reference implementation (`node/`, `onchain/`) and the on-VM
type-scripts. Out of scope (pre-launch): deployment infrastructure, third-party
dependencies' own advisories, and the prototype models in `research/`.
