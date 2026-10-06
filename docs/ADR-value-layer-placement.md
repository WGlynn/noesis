# ADR: Where the value-flow protocol lives (embedded vs separate vs stacked)

> Status: **DECIDED at the principle level; layer-2 build GATED on the open items in section 6.**
> Date: 2026-09-30. Supersedes nothing; it names the canonical decision that
> `DESIGN-value-oracle-seam.md` (built) and `research/crpc-second-meta-consensus-sketch.md` (open)
> already imply, and adds one new constraint (revisability) from the consensus-vs-value analysis.
>
> Grounding discipline: file:line references are relayed from the two design docs above; re-verify
> against `lib.rs` / `runtime.rs` / `amendment.rs` before any load-bearing external quote. Status
> markers: built / designed / open. Never round up.

## 1. The question

Should the value-flow protocol (the attribution / Proof-of-Mind value function `v(S)` and its flow)
be (a) separate from the chain entirely, (b) embedded into consensus, or (c) stacked on top?

## 2. The decision

**The value-flow protocol is not monolithic, so "embed vs separate" is the wrong axis.** It splits
along the determinism / finality seam into three layers:

- **L0 — embedded in consensus (hard, fast-final, on-chain): the deterministic skeleton.**
  Provenance, commit ordering, identity/standing keys, UTXO conservation invariants, AND the
  deterministic parts of value flow: novelty, similarity floor, flow propagation + damping, Shapley
  credit, cycle detection. These are pure integer arithmetic over the finalized graph, replicate
  bit-identically, and belong on-chain. **BUILT** (`value_v5..v8`, the Q32 settlement mirror,
  `NoveltyOracleV0` behind the `ValueOracle` seam; parity + swappability proven in
  `node/tests/value_oracle_seam.rs`).

- **L1 — stacked as an opt-in second meta-consensus (soft, graded, revisable): the worth judgment.**
  The subjective part: the learned / fuzzy "is this contribution actually valuable" seed, reputation,
  quality. Runs OFF the hard path via CRPC (commit-reveal pairwise comparison). AI / SLMs live here as
  node-level oracles: they run off-chain, emit pairwise judgments, and the chain disciplines their
  *claims* (commit-reveal + stake/slash) but never runs the model. **OPEN / sketch.**

- **The seam — the one narrow channel.** L1 reaches L0 ONLY through a single deterministic
  integer-value-per-cell that satisfies the `ValueOracle` contract (pure, bit-identical,
  shape-preserving, attribution-neutral). CRPC-settled today; zkML-attested at endgame. **Seam BUILT;
  CRPC / zkML feed OPEN.**

## 3. Why this is forced, not chosen

Two independent arguments converge on the same split:

1. **Consensus-vs-value tension.** Consensus requires fast agreement on a virtually irreversible
   state. Realized value is slow, fluid, revisable. Finalizing value at commit time finalizes a
   guess, which finality then makes irreversible = wrong by construction.
2. **The determinism wall.** Neural / SLM inference is not bit-identical across hardware (float order,
   GPU, library versions), so it cannot run inside the hard state transition regardless.

They are the same conclusion from two sides. **Fully separate is also rejected:** strip the chain and
the value judgment loses its only un-gameable anchor (commit-reveal, stake/slash, ordering, identity)
and the wash problem returns. The chain is precisely the harness that makes the fuzzy judgment
trustworthy. Stacked-through-a-seam is the unique surviving option.

## 3a. How L1 hosts non-deterministic inference (quarantine, not "non-deterministic consensus")

CRPC does not solve the inference problem by *being* non-deterministic. It **quarantines** the
non-determinism at the input boundary:

- Model outputs (the pairwise judgments) are non-deterministic. CRPC treats each as a **committed
  fact** ("node X claims A > B") via commit-reveal + stake. The chain never re-runs the model, so
  bit-identical replication of inference is never required. This is precisely how the determinism wall
  (section 3, argument 2) is sidestepped.
- The **aggregation stays deterministic**: tallying committed votes into a soft ordering, and the
  slash arithmetic, must be bit-identical across replicas from the same committed votes. If the
  aggregation were itself non-deterministic, CRPC could not converge either — the problem would just
  move down one level.

Precise statement: **CRPC converts non-deterministic inference into deterministic agreement about what
each model said.** Non-determinism becomes data; consensus over the data is deterministic. Calling
CRPC "non-deterministic consensus" is the wrong frame; it is deterministic consensus over
deliberately non-deterministic inputs.

- **What this buys:** the inference problem, cleanly. The model is never re-executed on the consensus
  path; its output is a committed, staked, disputable claim.
- **What it does NOT buy by itself:** honesty of a committed judgment, or independence of the judges.
  A node can commit a lying judgment; a colluding majority of judges settles a confident wrong answer
  (oracle-capture / 51% of the judging population). Those are handled by stake/slash, commit-reveal,
  enforced plurality, and the exogenous anchor — NOT by non-determinism. Non-determinism tolerance is
  necessary, not sufficient.
- **zkML (trustless endgame):** prove a committed model produced output X on input Y, so the output is
  deterministically *verifiable* without re-running the model. Non-deterministic to run, deterministic
  to check. Same quarantine, stronger mode.

## 4. The load-bearing safety property: NON-INTERFERENCE (proof obligation, not yet proved)

Let `H` be the hard state transition (UTXO invariants I1-I4 and finality). Let `S` be the soft layer
(CRPC value judgments, node-oracle outputs). Let `v` be the committed deterministic integer vector
the oracle seam exposes to `H`.

**NI (non-interference).** The finalized ledger state is a function only of (on-chain inputs, `v`),
and is independent of `S` except through `v`. Formally: for identical on-chain inputs and identical
committed `v`, any two soft-layer behaviors `S1, S2` (including adversarial ones) produce
**bit-identical** finalized state.

Corollaries that must hold:
- **No-mint.** `S` cannot cause issuance or raise any contributor's value except through `v`, and `v`
  is bounded by the aggregator. Partial support already exists: `value_v8`'s learned factor can only
  LOWER a seed, never raise it (`output_is_bounded_and_corruption_is_harmless_by_construction`), so a
  corrupt model is harmless by construction. NI generalizes this to the whole soft layer.
- **No-finalize.** `S` cannot cause a cell to finalize; finalization depends only on the hard
  invariants plus PoM weight derived from the deterministic oracle.
- **Degradation is liveness-only.** A compromised `S` may worsen value estimates (coordination
  quality) but can never violate safety.

**This property is NOT yet proved. It is THE gate before any L1 build.** The proof target is: exhibit
the `H` input surface, show `v` is the sole soft-to-hard channel, and show `v` is produced only by the
deterministic oracle contract, never by `S` writing hard state directly.

## 5. Revisability + vesting (partly already built — corrected 2026-09-30)

- **Never-finality + mean-reversion.** L1 value is a slow overlay that never reaches finality and
  updates (can revert) as realized use arrives. STILL OPEN for the economic-value overlay.
- **Delayed, slashable vesting.** CORRECTION: this already exists for the FINALITY INPUT.
  `Node::finality_pom_weight` (runtime.rs:1092-1113) only counts a cell toward finality weight after
  its finalization-work-time is at least `W` old (`vesting_w`), and a cell refuted during `W` is
  stripped forward-only. So the finality input is already vested + dispute-gated. What remains open is
  (a) never-finality/mean-reversion of the economic-value overlay, and (b) the explicit invariant in
  section 8 that the learned/CRPC judgment never reaches the finality path.

## 6. Open gates before building L1 (in priority order)

1. **Prove NON-INTERFERENCE (section 4).** Highest priority. Nothing in L1 is safe to build first.
2. **Make revisability + delayed vesting explicit (section 5).** Second. Design property, then encode.
3. **Spec the CRPC claim object + sharding.** What is a fuzzy-claim cell (a graded-score attestation
   cell)? Per-provenance-subgraph or sampled global panel? Aggregation rule?

## 7. Consequences

- The AI / learned model is never on-chain; it is a node-oracle disciplined by the chain. This is
  permanent, not transitional.
- L0 can ship and harden independently of L1 (it already has).
- Swapping the deterministic oracle is a governance amendment, not a per-node plugin
  (`DESIGN-value-oracle-seam.md` section "How the swap happens"); swapping/soft-updating L1 is
  continuous and opt-in.
- Reciprocity: any graded-agreement primitive built here that is useful to CKB/Nervos is offered
  upstream.

## 8. Does PoM-weighted finality violate the separation? (the sharpest question)

The section 2 separation is NOT "consensus must not depend on any value quantity" — that is
impossible for a PoM chain, where the Sybil-resistance franchise IS contribution. The correct seam is
WITHIN the value stack:

- Finality MAY depend on a value-derived franchise weight, but ONLY on the slice that is
  **deterministic, vested, and dispute-gated**.
- The subjective / learned / CRPC judgment must NEVER feed finality; it stays advisory (economic
  reward, reputation), strictly in L1.

**Grounded in code (verified 2026-09-30):**
- `Node::finality_pom_weight` (runtime.rs:1092-1113) computes finality weight via
  `pom_scores_with_similarity_floor_q16`, which delegates to the deterministic `NoveltyOracleV0`
  (`DESIGN-value-oracle-seam.md`). The learned outcome model (`value_v8`) and any CRPC judgment do NOT
  feed finality. No determinism violation on the finality path today.
- It is vested + dispute-gated: a cell contributes finality weight only after its finalization-work-
  time is at least `W` old (`vesting_w`), and a cell refuted during `W` is stripped forward-only
  (runtime.rs:1089-1113, 1116+). Consensus finalizes on standing that survived a vesting window plus a
  dispute window, not on a fresh value guess.

**Residual risk (bounded, not closed):** the deterministic proxy (novelty) is itself game-able — wash
is the open moat (0% graph-internal separation, `wash_sim.rs`). A ring that games novelty and survives
the vesting + dispute window inflates standing, which weights finality. Contained, not eliminated, by:
1. Vesting `W` — time for a refutation to strip the gamed cell before it ages into weight.
2. Refutation / slash — a landed dispute removes the cell from future finality weight.
3. Anti-concentration — PoM must independently supply >=50% of its dimension and cannot finalize
   alone; PoS must also consent (`MIN_DIM_BPS`, `FINALITY_MIX`). A value-layer attack cannot
   unilaterally break finality.

This is inherent to PoM consensus: because Sybil-resistance IS contribution, contribution-gaming is a
consensus-attack surface. The design goal is to BOUND it (above), never to claim it is separate.

**THE INVARIANT TO PROTECT:** never wire the learned / CRPC subjective value into
`finality_pom_weight`. It is not wired in today. This is the concrete, checkable form of
non-interference (section 4) specialized to a PoM chain, and it is what NI's proof must establish for
the finality path.

## 8.1 Adversarial correction (2026-09-30): determinism is not objectivity

A review pressed on section 8 and it holds; recording the correction rather than softening it.

**Determinism is not objectivity.** The seam exposes one deterministic integer per item, but a staked,
committed, attacker-shaped integer is deterministic and wrong, and once the hard layer consumes and
finalizes it, worth has been finalized. "Settle order, not worth" fails at the seam, because order now
includes how much credit flowed. This is the reputation-weighting subtlety in its purest form, not a
separate issue.

**The three containments in section 8 are weaker than stated:**
- Vested proxy is a DEFINITIONAL move (we define finality to read the proxy), not a dissolution.
- Slow vesting is a RATE / COST bound, not an unprofitability proof; whether it deters turns on the
  discount rate against standing value, so slow is not the same as unprofitable.
- Anti-concentration assumes PoS and PoM fail INDEPENDENTLY, which is the correlated-failure
  assumption we refuse to grant elsewhere (a capitalized ring can hold both dimensions).
The weight therefore rests entirely on the one unproven property, non-interference. Say so plainly.

**The novelty equivocation (must fix in the value docs).** Novelty is currently used BOTH as the
finality-weighting proxy AND as a reward seed: `pom_scores` (novelty) feeds `finality_pom_weight`, and
`value_v5..v8` seed reward from `temporal_novelty`. But the binding-sufficiency lemma already named
novelty-only scoring as a value seam that permits wash-building, and the defect audit flagged novelty
as a cheap denial attack. A quantity cannot be both objective and paying. Split it:
- STRUCTURAL novelty (canonical-form distinctness) may be objective and may gate finality, but must
  NOT pay reward.
- VALUE novelty pays, but is the subjective open seam, and must not sit on the finality path.
Reconcile against the binding-sufficiency lemma (`ISOMORPHISM-INVARIANCE-VS.md` / the burn-down defect
audit) before building further on novelty. This is an open correction, not a resolved one.
