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
| Money / medium of exchange (energy-pegged, Ergon-style) | **JUL** | PoW (10%, excluded from finality) | 🟡 designed, NOT built |
| Governance (separate instrument, orthogonal to the capture cycle) | **VIBE** | — | per TOKENOMICS.md |

PoM standing is **soulbound** (the `type_script.args` contributor identity, never reassigned
on transfer); the transferable byte (`lock.args`) carries ownership. Consensus franchise rides
the soulbound side, never the transferable byte.

## Value measure `v(S)` — the moat

Pipeline (all `node/src/lib.rs`): `temporal_novelty` (`:89`) → `pom_scores` (`:162`) /
`pom_scores_with_similarity_floor_q16` (`:182`, the live consensus PoM gate, near-dup floor
`Constitution.theta_sim_q16` default 0.95) → `value_v5..v8` (`:1026/1076/1143/1226`).
- `v5` realized-flow gate (counts use, an upper bound on dependence; `docs/REVIEW-2026-10-19-ledger-explainer.md`) · `v6` standing-gated seeds (prices identity, not participation) ·
  `v7` semantic/entropy floor · `v8` learned-outcome gate (Bradley-Terry over coalition features).
- Anti-gaming defenses are projections of one invariant (see `docs/ISOMORPHISM-INVARIANCE-VS.md`):
  within-identity λ^r, cross-identity μ^m, joint ρ^j damping; `attribution_circulation` +
  `attribution_cycle_energy` (Helmholtz–Hodge) → `collusion_slash`; θ_sim near-dup floor.
- **🔬 open / THE moat:** learned `v(S)` on real DeepFunding labels — graph-topology features
  returned NULL (both proxy + faithful set-level port, ~0.54; RESULTS-FAITHFUL), but a **rich-feature**
  judge (stars/age/funding) beats the null — on the honest **repo-disjoint** split it generalizes at
  **~0.60** (the 0.68 pair-split was partly repo-overlap inflation; `data/deepfunding/RESULTS-RICH-JUDGE.md`)
  ⇒ a *modest, popularity-driven* real signal on honest labels. NB: honest-label prediction ≠ the
  *adversarial* un-gameability the moat claims (that rests on the structural defense, 253/253). General
  isomorphism-invariance gate (cand-A) still open. Un-gameability claimed for *demonstrated* vectors
  only, never as a finished proof.
- **🔬 open / discernment (HCE-4) — the deepest open item.** Relabel-invariance is closed for the
  demonstrated axes (`node/examples/adaptive_sim.rs`, `g=0`; paraphrase the one live content-proxy gap)
  but does **not** cover **wash-building** — cheap, genuinely-distinct minds building on each other's
  worthless content. Measured: **0% graph-internal separation** of wash vs genuine
  (`node/examples/wash_sim.rs`; cyclic defenses catch only a wash *ring*). Base case = the scarcity of
  independent minds (`docs/DESIGN-mind-scarcity-asymmetry.md`). Solution shape (🟡 designed) = manufacture
  a **periphery**: value vests on realized use by *capital-independent* minds + rent/slash prunes patient
  wash (asymmetry = *rent*, not time) + a `v(S)` grounded on external-use labels
  (`docs/DESIGN-periphery-solution.md`), which prices a wash-ring negative-EV to a Bitcoin-51%-class
  residual (`node/examples/periphery_sim.rs`, harvest measured, Layers A+B design params).
- **🔬 open (review 2026-10-19):** genesis prior (a grown judge keeps its first rounds' taste), composition
  (year-one funding shapes the corpus the judge trains on), the patience filter (paying on use pays late),
  frame underprovision (pay capped at own novelty), capital buying variance, survival-as-lock-in, and
  counterfactuals that don't climb to roots. Agreed principles and contested proposals: `docs/REVIEW-2026-10-19-ledger-explainer.md`.

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
