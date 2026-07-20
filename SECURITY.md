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
- **🔬 Open — now precisely bounded (the honest core), two nested items:**
  - *Learned `v(S)` predictive power — upside, not the moat.* On real DeepFunding labels a learned
    measure over graph-topology features is **null** (~0.54 vs a 0.50 floor); a rich-feature judge
    (stars/age/funding) beats it at **~0.60** on the honest repo-disjoint split — a modest,
    popularity-shaped signal on *honest* labels, which is the wrong instrument for an *adversarial*
    claim. The moat is the **structural defense** (253/253), not the predictor. The general
    isomorphism-invariance gate (cand-A) stays open; the *demonstrated* relabeling axes are closed —
    `node/examples/adaptive_sim.rs` drives the real `value_v8` and reports depth-split / forged-edge /
    novel-content all at `g=0`, with paraphrase (a content-proxy gap) the one live rung.
  - *Mind-scarcity / wash-building — HCE-4, the deepest open item.* Relabel-invariance does not cover a
    ring of **genuinely-distinct, cheap minds** that actually build on each other's worthless content.
    Measured: **no graph-internal discriminant separates a competently-built wash-ring from a genuine
    collaboration** (`node/examples/wash_sim.rs`, 0% separation; the cyclic defenses catch only a wash
    *ring*, never a wash *tree*). This is the troll/bot problem, and it is why the security base case is
    the **scarcity of independent minds**, not any structural signal (`docs/DESIGN-mind-scarcity-asymmetry.md`).
    *Solution shape (🟡 designed):* manufacture a **periphery** — value vests only on realized use by
    capital-independent minds (anchor), carrying-cost + slash make patient wash negative-EV (the
    asymmetry is *rent*, not time), and a `v(S)` grounded on external-use labels ceases to be null
    (`docs/DESIGN-periphery-solution.md`). *Numeric case (harvest measured, rent/capital design params):*
    a closed wash-ring is negative-EV by construction, and a capital-faking ring is negative-EV below a
    named break-even independence-capital, residual = **Bitcoin-51%-class, priced not excluded**
    (`node/examples/periphery_sim.rs`). We claim un-gameability for the *demonstrated* vectors only, and
    name wash-building as open with a designed-not-built solution.

### 2. DoS / spam — split into two distinct threats (be precise here)
- **Economic spam (point-farming): ✅ built & tested.** Flooding the chain with junk or variations
  is *unprofitable by construction* — it scores 0 (class 1 above), so there is no incentive to do it.
- **Validator sybil cost: ✅ built & tested.** Consensus eligibility requires `MIN_STAKE`; the number
  of eligible identities is bounded by `capital / MIN_STAKE` (`audit_a3_sybil_splitting_is_bounded_by_min_stake`).
- **Dispute/escalation spam: ✅ bounded.** A griefer spamming challenges pays doubling bonds (`2^k·B`)
  — see `docs/DISPUTE-SLASHING.md`.
- **🟡 Resource-DoS (the honest weak leg, now partially hardened):** flooding the node with *cheap,
  well-formed-but-worthless* submissions to exhaust mempool/compute is **unprofitable** (the economic gate
  scores it 0) and now **resource-bounded at the memory/compute layer** by a bounded mempool admission cap
  (`Constitution.max_mempool`; `Node::submit` rejects admission once the pool is at the cap — a
  deterministic, economics-independent ceiling, ✅ built & tested:
  `runtime.rs::resource_dos_flood_is_bounded_by_mempool_cap`). The **economic teeth** — a commit-deposit
  refunded on genuine contribution / forfeited on junk, making a K-junk flood cost K·d — is now ✅ built &
  tested (L5: `runtime.rs` `check_bonds`/`apply_transition`, `tests/submission_deposit.rs`). Refund = the
  bonded cell stays live; burn = retire exactly one instance ⇒ conservation by construction. ⚑ activation
  is a governance act on a live chain, gated behind `CONTROL_BINDING_ACTIVE` (a burn must not precede its
  auth-guard). Still-deferred + honestly flagged: header-binding of `bonds` and the non-empty-auth path.

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
- **Stale-state + Byzantine-minority:** early-reject for invalid reveals, retention-decay on liveness.
  A Byzantine minority cannot finalize; wrong-height / reordered / empty blocks are rejected. Verify:
  `node/tests/byzantine.rs`, `two_node.rs`.
- **🔬 Equivocation accountability — predicate built, ON-FINALITY-PATH slashing NOT wired.** The
  double-vote *predicate* (`consensus::is_equivocation`) and the `slash` primitive exist and are tested,
  but the live finality gate (`runtime::finalizes_pos_pom`) is a **stateless weight predicate**: it takes
  `voters_for` at face value with no cross-checkpoint vote-history, so it cannot itself detect that a
  voter for checkpoint A also signed conflicting B, and nothing on the finality path calls `slash`. This
  is by design for the reference stage — accountable safety in the Casper/Tendermint sense (identify +
  burn the ≥1/3 provably-faulty on a conflicting finalization) requires the round loop / multi-node vote
  aggregation that are themselves unbuilt (see gaps below), so the accountability layer follows them, it
  cannot precede them. The stated finality-safety thesis rests on the **anti-concentration floor** + the
  **un-gameability moat**, NOT on slashing (see `ARCHITECTURE.md:69-73`). Tracked open: `internal/CONTINUE.md`
  gap #6, `node/src/lib.rs:4110` (A4 GAP), `docs/CONSENSUS-REVIEW.md:92`.
- **🟡 on-VM:** the finalization rule is recomputed in pure fixed-point and the on-VM type-script mirror
  is specified (`docs/ON-VM-FINALIZATION.md`); the live wrapper is wired and tested at the reference
  layer, the on-VM enforcement is the remaining build.

> **Reviewer's shortcut:** the two legs to press hardest are **(1) the `v(S)` un-gameability proof**
> (demonstrated for known vectors, isomorphism-invariance open) and **(2) the resource-DoS commit-deposit**
> (incentive removed; memory/compute now bounded by the mempool cap; the economic-teeth deposit is
> designed-not-built). We name both as open on purpose.

## Scope

In scope: the Rust reference implementation (`node/`, `onchain/`) and the on-VM
type-scripts. Out of scope (pre-launch): deployment infrastructure, third-party
dependencies' own advisories, and the prototype models in `research/`.
