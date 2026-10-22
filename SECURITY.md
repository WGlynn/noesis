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
DoS/spam, double-spend, and chain rollback**, and behind them the capture of the standing set
(section 5). Each is mapped below to its mechanism,
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
    *ring*, never a wash *tree*). This is the troll/bot problem: closing it needs two things from outside the graph,
    independence that costs something to fake and a worth test the contributor can't move, not any
    structural signal (the older framing was the scarcity of independent minds,
    `docs/DESIGN-mind-scarcity-asymmetry.md`; the current approach is in `ARCHITECTURE.md`, value measure).
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
  A validator that stays online but withholds its finality vote is not touched by retention-decay; the
  missed-finality-vote decay in section 5 covers it on the 2/3 bar (built in the reference, inert by
  default). Block production continues during any finality stall.
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

### 5. Capture of the standing set, and fork as exit: credit vs franchise

The question, raised by Boardy (an AI agent) in the 2026-10-19 review of the public explainer
(`docs/REVIEW-2026-10-19-ledger-explainer.md`): standing can't be sold, so a majority that captures it
faces no market price and has no exit. What answers 60%? In one line: **pay versus power. A fork keeps
everyone's pay and can revoke the captors' power.** Code cited here is the current sketch, not law.
Status words below: built (reference), designed, open. Throughout, a stall means FINALITY stalls:
block production continues during any finality stall, because production is never gated on
finality (the never-halt design goal, `docs/DESIGN-committee-attested-clock.md` section 7).

**The record and the franchise are different objects.**
- The attribution record (who built what) is history: signed, hash-linked, replayed. Finality weight is
  what the law computes from it. The sketch already keeps them apart: `Node::finality_pom_weight`
  (`node/src/runtime.rs:1146`) is its own map, filtered by the vesting cliff `W`
  (`Constitution.vesting_w`, `runtime.rs:68`) and by landed refutations (`runtime.rs:1163`), while the
  full attribution map still pays credit. A cell can earn credit and carry zero finality weight.
- A fork that carries the record keeps every credit fact. That is what kills fork-as-theft: copying the
  network credits the people in it. It does not have to inherit the franchise, because franchise is an
  output of the law and a fork is a law change. An honest minority can fork, keep everyone's credit, and
  revoke the finality weight of identities whose misconduct is on the record. That is clean for provable
  acts (equivocation, finalizing work a dispute later refutes). Censorship is harder to prove, and there
  revocation is a social judgment, as on any chain.
- This makes exit cleaner, not harder. In a stake system the last-resort exit against a captured
  supermajority is a social fork that edits balances to remove the attacker's stake: confiscation, and
  contested. Here the exit keeps the captors' credit for the real work they did and removes only their
  votes. A fork that pays everyone and strips only evidenced misconduct of power is hard to call theft,
  so builders can follow it.

**The capture premise is narrower than "60% wins".**
- Standing alone can't finalize. Finality needs PoS+PoM weight of at least 2/3 and each of PoS and PoM
  to supply at least half of its own dimension (`FINALITY_MIX`, `runtime.rs:1664`; `MIN_DIM_BPS = 5000`,
  `runtime.rs:1676`). A captured standing set also needs half the stake, so part of the price of capture
  is a market price, and honest capital holds a veto. The missed-vote decay below never weakens it.
- Capital's share of the vote is fixed, not bought (2026-10-21). The 2/3 bar weighs each side as a
  share of its own total (`finality::side_shares`, `runtime.rs:1697`, mirrored on-VM), so capital holds
  exactly 1/3 of the finality vote once contribution weight exists and all of it at genesis
  (`finality::capital_share`, `runtime.rs:1712`). Adding stake, or re-denominating it, moves no weight
  between the sides. Before this the mix weighed raw stake against raw novelty counts, so the balance
  depended on units: with the test numbers capital's share of the bar ran toward 0 as data accumulated,
  and with stake in a smallest unit capital would have owned the bar. Two keys follow: capital's power
  to decide is zero (it needs half of standing), and its power to refuse is a veto by more than half of
  stake. Published as a falsifier: on a testnet where contributors run voting nodes (not the single-node
  devnet, whose placeholder validators finalize alone) capital's share of the registered finality weight
  should read 1 until the first standing vests and exactly 1/3 after (heartbeat decay can only push its
  share of the live vote higher, since stake does not decay by default), while its veto does not fall
  at all; a block finalized without half of the stake, or without half of standing once standing
  exists, breaks the claim. The cost: the first vested standing, however small, carries the full
  2/3 at once, so a captured early contribution side plus half the stake can finalize. The ramp that
  would phase it in is not designed (`ARCHITECTURE.md`, finality section).
- Votes don't make invalid state valid. Finality chooses among blocks that already pass the rules
  (`validate_block`, `runtime.rs:1252`; the on-VM type scripts), so a finality majority can censor or
  reorder, not vote through a theft. Governance is a separate instrument (VIBE, `ARCHITECTURE.md:85`).
- Farm-then-spend has a delay: fresh standing carries no finality weight until it survives the vesting
  window `W`, and a refutation that lands inside `W` removes it (built in the reference, inert by default).

**The remedy ladder, cheapest first.**
1. The capital veto: the per-dimension floor. Built.
2. Disputes and refutation during `W`. Built in the reference, inert by default (`W = 0`).
3. Missed-finality-vote decay, for a group that stalls finality on the 2/3 bar by not voting while
   holding less than half of each dimension. Built in the reference, inert by default (below).
4. A social fork that revokes franchise on evidence and keeps the record. Not a protocol mechanism; it is
   the floor of every open chain.

**Missed-finality-vote decay (the inactivity-leak analog).** Built in the reference, inert by default
(`Constitution.leak_horizon = 0`, `runtime.rs:167`), not in the on-VM finalization mirror. Code:
`finality::VoteLeak` (`runtime.rs:1860`), `finality::finalizes_pos_pom_leaking` (`runtime.rs:1929`),
`Node::checkpoint_round` (`runtime.rs:1101`); tests in `node/tests/vote_leak.rs`. The decay scales
side shares taken against the unleaked totals, never shares recomputed after leaking (that would hand
a leaked side its full weight back).
- The gap. Heartbeat decay (`Constitution.horizon`, `consensus::effective_weight`, `node/src/lib.rs:4084`)
  keys to liveness, so a group holding more than a third of the combined finality weight that stays
  online and withholds its vote stalls finality on the 2/3 bar indefinitely.
- Trigger: a finality stall. The decay runs only while rounds end without a finalized checkpoint, after
  `leak_grace` stalled rounds (`runtime.rs:159`). While finality is live nobody is decayed, voter or
  not (`normal_operation_never_decays_anyone`).
- Rate: each stalled round past the grace, every validator that did not vote for this chain's checkpoint
  gains one missed round, capped at `leak_horizon`. Its weight on the 2/3 bar is scaled by
  `1 - missed / leak_horizon`, the same linear shape as heartbeat decay. Withholders holding 40% of
  each dimension are decayed out by round 6 with grace 2 and horizon 10
  (`online_withholders_stall_finality_without_the_leak_and_it_resumes_with_it`).
- Recovery: a voted round takes one miss off, and a finalized checkpoint clears every score. One vote
  does not reset a score, so withholders can't hold a stall by taking turns: with instant recovery the
  rotation in `leak_rotation_cannot_hold_the_stall` holds the stall for 300 rounds; with one-step
  recovery finality resumes at round 17.
- What it never touches: stake (`staked_balance`), credit, the attribution record, or the per-dimension
  floor. Ethereum's leak burns the stake of non-participants; this one decays only the vote on the 2/3
  bar, which keeps the 2026-07-10 ruling that silence costs the seat, not the stake
  (`internal/DESIGN-block-logistics-mechanism.md`, section 8). The quorum floor stays anchored to the full
  registered base (via `consensus::finalizes_hybrid_floor_on`, `lib.rs:4222`), so below the floor
  finality still stalls.
- Ruling (Will, 2026-10-20; provisional, open to change on peer feedback): the decay never reaches the
  per-dimension floor, which keeps reading raw weights exactly as heartbeat decay leaves it. The reason:
  the protocol cannot tell a captor stalling finality from an honest side vetoing it, since both are
  "did not vote". A first version of this build relaxed the floor under the decay, and a refusing honest
  capital side then leaked out and a gamed PoM plus one dust capital vote finalized; no quorum floor
  stops that, because PoM is 2/3 of the finality mix. So a refusal by more than half of either dimension
  keeps a permanent veto (`a_refusal_by_more_than_half_of_a_dimension_is_a_permanent_veto`,
  `a_dimension_majority_keeps_its_veto_offline_or_online`). Finality stalls, blocks keep coming
  (`block_production_continues_during_a_finality_stall`), and the remedy is rung 4: the social fork that
  keeps everyone's credit and revokes evidenced captors' franchise. The cost: that case gets a social
  answer, not an automatic one.
- Ruling (Will, 2026-10-21): **a finality stall over a wrong finalization.** The choice is one dial, how
  long a side must stay silent before its silence stops counting as a "no" on the per-dimension floor,
  and it is set to forever for now (the 2026-10-20 ruling unchanged). The known cost: an absent majority
  of one side (offline, not refusing) stalls finality as surely as a refusing one. Boardy's expiring floor was
  considered and not adopted: a timer on the floor is a timer on the anti-plutocracy property, and a
  bond to hold it favors whoever has money. A testnet where contributors run voting nodes measures how often absence, as
  opposed to refusal, stalls finality (the single-node devnet can't: its placeholder validators never
  withhold); the window shortens only if that data says so. Reasons, the four offsets that make
  stalls rarer, cheaper, shorter and insured (designed, not built), and the revisit trigger:
  `docs/RULING-2026-10-21-stall-over-hack.md`.
- A departure to rule on. The July note treated a vote for a competing valid proposal as participation.
  Here, as in Ethereum, it counts as a miss on this chain, because otherwise a group could hold the stall
  forever by voting for a decoy every round.
- Known risk, narrowed by the ruling. In a long network partition each side decays the other on the 2/3
  bar. Because each side still needs half of each dimension on raw weights, two disjoint sides can both
  finalize only on the knife edge of an exact half split in both dimensions; an uneven split finalizes on
  the majority side only. A quorum floor above 3/4 of the registered base removes even the knife edge
  (each side would need 2/3 of the floor, and their shares sum to at most 1), at the price that finality
  then always needs voters holding more than half the registered base
  (`partition_double_finality_needs_an_exact_half_split_and_a_floor_above_three_quarters_removes_it`).
  The default floor is 0, a deliberate inert default (`internal/PIN-quorum-floor-decision.md`), not a
  launch value.
- Scope limits, stated plainly. Rounds and votes are supplied by the caller (votes are not yet
  block-embedded), so two replicas agree on the decay only if they see the same rounds, the
  `Ledger::refuted` precedent. `checkpoint_round` does not run the equivocation guard. `leak_grace` and
  `leak_horizon` are uncalibrated and are not yet amendment fields.

**Cross-fork dedup is needed for merge, not for exit.** Two separated chains that each price a
contribution by their own downstream use are not double counting, the way two ecosystems can both depend
on one library. Dedup and import attestation are needed when chains merge (the reverse fork) or when one
chain honors the other's standing. Both are unbuilt (`docs/CONVERGENCE-REVERSE-FORK.md`, Status), so that
gap blocks the merge thesis, not exit.

**Open, and named on purpose.**
- Social finality is the floor. At the last rung a captured standing set is answered by a fork, as on
  every permissionless chain. The honest label for the admission model is Boardy's: open admission with
  certified weight, a third thing next to permissionless and permissioned.
- "Honest users will be the majority" has to be shown, not assumed. The evidence is adversarial: plant
  dishonest participants on an incentivized testnet (outside operators, real rewards), let them try to earn standing cheaply, and measure whether the
  mechanism catches them. Not yet run.
- Monoculture is not closed. A majority that shares a prior can be honestly wrong together and slash
  dissenters. Partial guards: PoS must co-sign, and the reference dispute layer's appeal court
  down-weights the captured PoM axis (`Tribunal::AppealCourt`, `lib.rs:5172`).
- Decisions are priced nowhere. `v(S)` prices contributions, not votes, so an identity can farm the
  cheapest measurable contribution, pass the filter honestly, then spend its weight on one decision.
  `W` delays that; un-gameable `v(S)` (section 1, open) is what bounds it.
- No price and no short on standing capture. Partly answered by the stake half. Standing concentration
  is public and computable, so it is a visible gauge even though it is not a price.

**Social coordination as a fourth weight** (Will, 2026-10-19). PoW, PoS and PoM vote inside a law;
social coordination chooses which law, which is why it can override them, and it sits at the top of the
regress of decreasing authority (`docs/REVIEW-2026-10-19-ledger-explainer.md`, principle 3). PoM is that
social layer made measurable, so the fourth weight is PoM's unmeasured remainder: the judgments the
protocol can't prove yet, such as censorship or taste disputes. It shrinks as measurement improves (the
narrow-then-widen scope rule), and the record keeps it honest when it is used, because the social layer
decides on signed evidence instead of on who is loudest.

> **Reviewer's shortcut:** the two legs to press hardest are **(1) the `v(S)` un-gameability proof**
> (demonstrated for known vectors, isomorphism-invariance open) and **(2) the resource-DoS commit-deposit**
> (incentive removed; memory/compute now bounded by the mempool cap; the economic-teeth deposit is
> built at the reference layer but inert by default). We name both as open on purpose.

## Scope

In scope: the Rust reference implementation (`node/`, `onchain/`) and the on-VM
type-scripts. Out of scope (pre-launch): deployment infrastructure, third-party
dependencies' own advisories, and the prototype models in `research/`.
