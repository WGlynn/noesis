# Noesis — architecture reference (canonical, code-grounded)

> **Why this file exists.** The load-bearing facts about Noesis's consensus, finality,
> tokens, and value measure are easy to misremember (the *overall consensus mix* vs the
> *finality mix* is a known trap — they are different). This is the single place that
> states them with a pointer to the **constant in code** that defines each one. Treat the
> code pointer as ground truth.
>
> **DISCIPLINE (load-bearing):** never assert a Noesis protocol number — consensus mix,
> finality weights, thresholds, anti-concentration floor, MIN_STAKE, tokenomics — from
> memory. Open the cited `file:line` and read the constant first. Numbers below are stamped
> from code on 2026-06-29; verify before quoting in any partner-facing artifact.

---

## Repo layout

- `node/src/lib.rs` — the reference node: novelty/PoM (`temporal_novelty`, `pom_scores`),
  consensus (`mod consensus`), the value stack (`mod value`: `value_v5..v8`), dispute/slash,
  outcome model, attribution/collusion detectors. Large; grep by symbol.
- `node/src/runtime.rs` — `Node`/`Ledger`/`Block`, the `Constitution`, token txs
  (`TokenTx`, `mod tokens`), the **finality gadget** (`mod finality`), lock-sig verifier.
- `node/src/tokens.rs` — ERC-analog conservation (Fungible/Nft/Multi).
- `onchain/noesis-core` — `no_std` verify-side core shared with the on-VM type-scripts
  (Q32.32 mirrors: `finalizes_pos_pom_fixed`, `lamport`, `tx`). Single-sourced with the node.
- `onchain/{pom,finalization,commit-order,locksig}-typescript` — RISC-V CKB-VM scripts
  (excluded from the workspace; built standalone for `riscv64imac`).
- `SECURITY.md` — the attack-class defense matrix (gameability / DoS / double-spend /
  rollback) with honest built-vs-designed status. **Read before any security claim.**
- `ROADMAP.md` — adversarial-loop log (newest first); `internal/CONTINUE.md` — handoff.
- `docs/` — canonical longform: `TOKENOMICS.md`, `POM-CONSENSUS.md`,
  `POM-FINALITY-TEMPORALITY.md`, `RESOURCE-DOS-BOUNDING.md`, `ISOMORPHISM-INVARIANCE-VS.md`,
  `ON-VM-FINALIZATION.md`, the whitepaper, accessible tier.

## Consensus vs. Finality — THE distinction (do not conflate)

Two different weightings. Quoting one for the other is the 2026-06-29 hallucination.

- **Overall consensus mix (NCI):** `pow 0.10 / pos 0.30 / pom 0.60`.
  → `consensus::NCI` at `node/src/lib.rs:4052`. Supermajority `TWO_THIRDS_BPS = 6667`
  (`lib.rs:4054`). This is the *whole-system* weight (production + ordering + finality inputs).
- **Finality mix — PoW EXCLUDED:** `pow 0.0 / pos 1/3 / pom 2/3`, renormalized so PoS+PoM
  sum to 1. → `finality::FINALITY_MIX` at `node/src/runtime.rs:1664`.
  - **Why PoW is out of finality:** PoW is probabilistic and reorgeable, so putting it on
    the *safety* path would itself be the hazard. PoW secures block production, ordering, and
    Sybil-cost (and is the money layer, JUL) — not safety. Finality routes through
    `finality::finalizes_pos_pom` (`runtime.rs:1730`).
  - **2/3 bar is over the PoS+PoM fast-final set**, not a global total.
  - **Each side is measured as a share of its own total before mixing** (2026-10-21,
    `finality::side_shares`, `runtime.rs:1697`; on-VM mirror `finalizes_pos_pom_fixed` in
    `onchain/noesis-core`). So capital holds exactly 1/3 of the finality vote once any contribution
    weight exists, and all of it at genesis when the contribution side is empty
    (`finality::capital_share`, `runtime.rs:1712`), at any denomination of stake or standing. With
    both sides present and no decay, a checkpoint finalizes when `a + 2b` clears `3 x 0.6667` and
    `a, b >= 1/2`, where `a` and `b` are the voting fractions of stake and of standing.
    Note (2026-10-21): before this, the mix weighed raw stake against raw novelty counts, so the
    balance between the sides depended on their units. With the test numbers (stake 1000.0 per genesis
    validator, standing in novelty counts) contribution swamps stake as data accumulates and capital's
    share of the 2/3 bar runs toward 0; with stake counted in a smallest unit it would flip and capital
    would own the bar. The per-side floor never depended on units (rescaling a side multiplies both
    terms of its comparison by the same factor). Found while answering a reviewer's (Boardy's) request
    to publish capital's share as a falsifier.
  - **Cost, stated plainly:** shares make the genesis handoff a step, not a slope. The first vested
    contribution weight, however small, carries the full 2/3 at once, so whoever holds a tiny early
    contribution side plus half of the stake can finalize (with raw units and stake dominant, that took
    about 2/3 of the stake). The per-side floor already gave a tiny contribution side its veto before
    this change. The intended fix is a ramp that phases the contribution side's weight in as it grows,
    keyed to something costly to fake (lineage diversity, not a date that can be front-run or a head
    count that extra accounts can pad). It is not designed yet.
- **Anti-concentration floor:** each dimension (PoS and PoM) must *independently* supply
  ≥ **50%** of its own dimension's total to finalize. → `MIN_DIM_BPS = 5000` (`runtime.rs:1676`),
  `dim_ok` (`runtime.rs:1678`). This is what makes neither axis sufficient alone:
  - a gamed PoM (the 2/3 share) cannot finalize without the PoS dimension also clearing 50%;
  - capital holding *all* stake controls only the PoS dimension, and the unbuyable PoM
    dimension must independently clear 50% — **capital cannot finalize without contribution's
    consent.** This is the anti-plutocracy property; it is a per-dimension floor, not a cap.
- **WEIGHT ≠ EFFECTIVE POWER (do not conflate).** `60/30/10` (and finality's `1/3:2/3`) is how much
  each axis *weighs* in the combined vote (`base_weight`/`effective_weight`, `lib.rs:4070/4084`). It is
  NOT how much each axis *rules*: the anti-concentration floor + AND-composition make NCI a
  **separation-of-powers** (rock-paper-scissors), where each axis is *independently necessary* and none
  finalizes alone (`docs/CONSENSUS-REVIEW.md`, `docs/COHERENCE-LAWS.md`: "no dimension finalizes alone").
  So effective finalization power is far more balanced than the headline weights imply. `60/30/10` is the
  **voting-weight mix, not a reward distribution** — there is no separate `60/30/10` emission/reward
  constant; rewards come from PoM-standing-via-`v(S)`, JUL-via-PoW, and PoS returns.
- `MIN_STAKE = 100.0` bounds validator Sybils (`lib.rs:4160`).
- **Finality liveness under withholding: missed-finality-vote decay** (built in the reference, inert by
  default, not on-VM; pointers re-stamped 2026-10-21). A group holding more than a third of the combined
  weight that stays online and withholds stalls finality on the 2/3 bar; block production continues
  through any finality stall. The decay (`finality::VoteLeak`, `finalizes_pos_pom_leaking`,
  `runtime.rs:1860/1929`; `Node::checkpoint_round`, `runtime.rs:1101`; shares are taken against the
  unleaked totals, then leaked) runs only during a finality
  stall, decays the 2/3-bar weight of whoever does not vote for this chain's checkpoint (never stake or
  credit), and keeps the quorum floor on the registered base. Ruling (Will 2026-10-20, provisional
  pending peer feedback): it NEVER reaches the per-dimension floor, so a refusal by more than half of
  either dimension keeps its veto (the protocol can't tell an honest veto from a captor's stall); that
  case is answered by the social fork. Off while `Constitution.leak_horizon == 0` (`runtime.rs:167`).
  Design, partition risk, and the fork-as-exit remedy ladder: `SECURITY.md` section 5.
- **DECISION — PoM stays coupled to finality (ruled by Will, 2026-06-29; security-expert call).** The
  built design stands: PoM is load-bearing in finality, bounded by the anti-concentration floor.
  Rationale: full-decouple (PoS-only safety) would forfeit anti-plutocracy, which is the thesis. The
  floor converts the welding risk from "gaming OR capital" into "gaming AND capital" (a conjunction).
  - **The two real protections are the anti-concentration floor + the un-gameability moat — nothing else.**
  - **Standing-slash is NOT a finality-safety property (rejected as circular):** slashing gamed PoM
    standing is slashing collateral the attacker gamed into existence; and an honest staker who co-signs
    a gamed checkpoint committed no *objective* fault (unlike equivocation), so can't be fairly slashed.
    Slashing deters equivocation, not the gaming case. Do not claim accountable-safety-via-standing-slash.
  - Forward: keep PoW excluded from finality (done); consider whether `MIN_DIM_BPS` (50%) is conservative
    enough for the safety path specifically; the residual (severe undetected gaming + cleared floor) is
    exactly what the moat closes — coupling is the bet that the moat wins.

## Three tokens (orthogonal roles) — see `docs/TOKENOMICS.md`

| Role | Token | Axis | Status |
|---|---|---|---|
| Contribution / consensus franchise (soulbound standing) | PoM standing | PoM (60% overall, 2/3 of finality) | ✅ built (reference) |
| Bonded capital / stake | CKB-native (state-rent) | PoS (30% / 1/3 of finality) | ✅ built (reference) |
| Money / medium of exchange (energy-pegged, Ergon-style) | **JUL** | PoW (10%, excluded from finality) | issuance + coinbase built (reference; mints on the reference node, worthless at its test difficulty); stability reserve built but off and unwired; economics unset (`docs/DESIGN-jul-money-layer.md` section 4) |
| Governance (separate instrument, orthogonal to the capture cycle) | **VIBE** | — | per TOKENOMICS.md |

PoM standing is **soulbound** (the `type_script.args` contributor identity, never reassigned
on transfer); the transferable byte (`lock.args`) carries ownership. Consensus franchise rides
the soulbound side, never the transferable byte.

## Value measure `v(S)` — the moat (re-stamped from code and the research record, 2026-10-21)

> Read with `docs/research/WASH-PROBLEM-INDEX.md` (every approach, with status) and the recent recaps in
> `sessions/`. This section lagged the September-October research once and misled a reader; when the
> research moves, update it here. Code pointers are the current sketch, not law.

**1. What the sketch computes (built).** In `node/src/lib.rs`: `temporal_novelty` (`:187`) -> `pom_scores`
(`:260`). The value-oracle seam: trait `ValueOracle` (`:293`), the built scorer `NoveltyOracleV0` (`:303`),
the aggregator `pom_scores_with_oracle` (`:314`); `pom_scores_with_similarity_floor_q16` (`:341`) is the live
consensus PoM gate (near-duplicate floor `Constitution.theta_sim_q16`, default 0.95). The value stack is
`mod value` (`:1007`): `value_v5` (`:1189`) gates novelty by realized downstream use through `flow_gate`
(`:1157`), which counts use and so is an upper bound on dependence; `value_v6` (`:1239`) prices identity
through standing-gated seeds; `value_v7` (`:1306`) adds a semantic/entropy floor; `value_v8` (`:1391`) adds
a learned-outcome gate (Bradley-Terry over coalition features; its weights are passed in, and nothing yet
decides when it retrains). Interior anti-gaming: within-identity, cross-identity and joint damping,
`attribution_circulation` (`:373`) and `attribution_cycle_energy` (`:426`, Helmholtz-Hodge) feeding
`collusion_slash` (`:5010`); one invariant behind them in `docs/ISOMORPHISM-INVARIANCE-VS.md`.

**2. The fence: finality reads structural novelty only (built, 2026-10-05).** `Node::finality_pom_weight`
(`node/src/runtime.rs:1146`) reads novelty past the vesting window, not the value stack, and `value_v5..v8`
have no caller in the runtime or the RPC. The test
`standing_and_finality_are_structural_novelty_only_no_value_layer_leak` (`runtime.rs:3209`) fails if the
value layer reaches standing. So nothing that judges worth, including a learned or brain-based judge,
touches what gets finalized. Why split them: if finality and reward read one gameable signal, one wash
attack is a value, governance and consensus exploit at once (`docs/DESIGN-novelty-split-structural-vs-value.md`).

**3. The open problem, stated precisely.** Interior signals are worth-blind. A competent acyclic wash
(distinct keys, novel content, honest topology) is structurally identical to genuine collaboration: 0.0%
separation in `node/examples/wash_sim.rs`. Wash (fake value from real, distinct actors) is not Sybil (fake
identity), so identity proofs don't reach it. Closing it needs two things from outside the graph:
independence that costs something to fake, and a worth test the contributor cannot move. Already handled:
cyclic self-citation is caught by built interior defenses, and so is relabeling on the demonstrated axes
(`node/examples/adaptive_sim.rs`, `g = 0`; paraphrase is the one live content-proxy gap). On real crates.io
data the independence gate's logic zeroes closed self-owned dependency rings (78.8% of reuse crates; they vest 0.00
against 69.36 for genuine ones), so the machinery works given a clean independence signal
(`docs/research/WASH-PROBLEM-INDEX.md`, sections 0 and 6).

**4. The current approach (designed; the measuring part partly measured).** Four parts, from the
September-October 2026 research:

- **Measured transfer instead of declared citation.** Make the graph's dependencies learners (Cadence
  brains): an edge a -> b means b took in a and b's measured behavior changed, so the graph records who
  measurably learned from whom instead of who cited whom. The receiving brain is the instrument and stands in
  for a user: worth is how much a copy fed the contribution improves over its untaught twin (a byte-identical
  snapshot) on a held-out task the contributor doesn't control. Contributions come from anyone; the
  receiving brains hold no standing, cast no votes and vouch for nothing. They are costly to fool (a fake
  that moves an honest task had to carry real task information, so faking collapses into contributing) and
  cheap to copy, which is what makes the twin comparison and many-judge checks affordable. Measured in the
  public lab `WGlynn/cadence-teachers` (ledger `VS-TRANSFER-PROGRAM.md`):
  - On a bandit task: separation from surface-matched fakes (M1); it survives a fake matched on every
    interior feature, additively and for two forms of synergy (M2); collusion among peers buys nothing given
    an honest task pool, by construction, since the measure never reads peer agreement (M4). Isomorphism
    invariance is empirical only (M3).
  - On a richer chain task: transfer prices skill on a brain that takes in content (a know-1..4 ladder,
    ordered, replicated on four disjoint student sets, fakes at most about 4% of pay; EXP-62 to EXP-62e) and
    is blind on a brain that doesn't (M1b). The zero point is the unsettled part.
  - The price depends on three outside ingredients: the task, the instrument (which brain) and the null
    (what "no contribution" means). On every brain tested the order of worth held; the exchange rate between
    kinds of contribution moved with the brain, the zero and how often lessons reached late stages
    (EXP-62b, EXP-62h, EXP-62i).
  - Boundary (M5, `M5-EXOGENOUS-ANCHOR.md`): whoever supplies a majority of the scoring tasks wins (the
    crossover is half the task pool; about a third against an adversary who knows the task, derived, not
    measured). Transfer moves the honest-majority assumption from peers to tasks. That is progress, because a
    task has an answer that can be checked and a peer report doesn't, but it is not an escape.
- **Judges out of the contributor's reach** (`sessions/2026-10-19-judges-out-of-reach.md`). Law below state:
  a cheater can buy state (tokens, an aged account) but not law, so fix everything that judges (model, task,
  baseline, rules) before the work it judges arrives. Adopted 2026-10-21: a frozen judge pins its whole setup
  (library commit, every setting, its zero, its curriculum; `docs/DESIGN-value-oracle-seam.md`). Designed:
  grow each round's judge from settled work and freeze it before the round opens; commit-reveal the test from
  a public random beacon. Not built: no judge-version field in `Constitution`, no retrain schedule, no
  amendment activation point (`node/src/amendment.rs` rejects the class as not built).
- **Layered anchors; "outside" means too costly for this attacker to move**
  (`docs/research/layered-anchors-2026-10-16.md`; designed, not measured). Stack anchors rooted in different
  scarce resources (capital, energy, time) so a ring has to look independent in every layer at once; the
  faking cost multiplies only if the roots can't be bought together. Task labels come from realized use one
  layer up; proof of work anchors independence and bootstrap, not worth; revenue needs an oracle. Brains
  that disagree on prices give a signal, not noise (OPH-style local consistency; Glynn 2026, *Value Disputes
  Are Missing Dimensions*): a fake that fools one judge but not the others should
  leave a price inconsistency (proposed test, not run). Built half: the independence gate's logic; the
  capital-cluster oracle that would feed it is not built. Honest floor: for an attacker with capital, locked
  money, bought energy and bought time largely convert into one another, so the moat is the conversion cost,
  and an aged, funded account buys several layers at once (`docs/REVIEW-2026-10-19-ledger-explainer.md`,
  principle 4).
- **Pay the honest outcome instead of detecting the fake.** Value vests on realized use by capital-independent
  minds, and a carrying cost plus slash-at-risk makes patient wash negative-EV, which prices a wash ring to a
  51%-class residual rather than excluding it (the periphery, `docs/DESIGN-periphery-solution.md`; simulated
  in `node/examples/periphery_sim.rs` under design parameters). Credit has no clock (built for `value_v5`; test
  `credit_has_no_clock_late_use_pays_early_work_the_same`, `lib.rs:1634`). A refundable submission deposit is
  built and inert by default (`submission_deposit`, `check_bonds`); it refunds on novelty today, and the
  designed change is to refund on realized use and feed kept deposits to the judge as failure labels. Rule
  for any such cost (Will, 2026-08-05): it lands on the fakeness, not on entry. If a funded ring can afford
  it and a poor honest newcomer can't, it filters poverty, not fakes.

**5. Scope.** The measured scope starts where the counterfactual can be run: code first, then models,
prompts and datasets, and written know-how an agent consumes under a frozen judge
(`docs/research/sage-validity-layer-and-the-memory-counterfactual.md`). It widens as AI makes
counterfactuals cheaper. Less measurable value earns under stricter terms (larger deposit, longer vesting,
lower cap), never zero (Will, 2026-10-19). Designed, not built: `submission_deposit` and `vesting_w` are
single global settings in `Constitution`.

**6. A learned judge on real labels (open).** A rich-feature judge on 1,965 usable Deep Funding jury pairs
generalizes at about 0.60 on a repo-disjoint split, mostly from popularity; graph-topology features alone
were null at about 0.54 (`data/deepfunding/RESULTS-RICH-JUDGE.md`). A modest real signal, and a live risk of
learning attention instead of dependence. Prediction on honest labels is not adversarial un-gameability.

**7. Still open.** The capital-cluster independence oracle (designed, not built); an honest task source
(M5); isomorphism invariance as a proof (M3; `WASH-PROBLEM-INDEX.md` row 9); transfer beyond small brains
on toy tasks (no instrument yet takes in ordinary code or prose); and the review's list: genesis prior,
composition, the patience filter, frame underprovision, capital buying variance, survival as lock-in,
counterfactuals that don't climb to roots (`docs/REVIEW-2026-10-19-ledger-explainer.md`). Un-gameability is
claimed for demonstrated vectors only, never as a finished proof.

## Security model — see `SECURITY.md` (read it; don't paraphrase from here)

Five attack classes, honest status: (1) `v(S)` gameability ✅ built (reference+runtime),
isomorphism-invariance open; (2) DoS/spam — economic ✅, **resource-DoS Bound A (mempool cap)
✅, Bound B commit-deposit ✅ (reference layer, inert by default)**; (3) double-spend ✅ reference / 🟡 deploy-coupled crypto;
(4) rollback/finality ✅ reference / 🟡 on-VM mirror; (5) capture of the standing set and fork as exit
(credit vs franchise), with the missed-finality-vote decay (2/3 bar only) built in the reference and inert by default. Structural maxim: *don't let the attacker
choose a security-critical input* (every consensus value re-derived on-VM, rejected if not
reconstructable).

## Status discipline

✅ built & tested · 🟡 designed, not built · 🔬 open problem. Never round designed up to built.
Test count + green status: `node/README.md` (doc-coherence hook stamps it against code).
