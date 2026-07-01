# Noesis — living study guide (generated; do not hand-edit)

> Regenerated FROM the repo by `scripts/study-guide.py`, so it cannot lag the
> code. Tick the boxes as you internalize each piece. Re-run to refresh.
> Node test suite: **258 passing**.

## Read in this order

1. [ ] **WHITEPAPER-FOR-DAD.md** — *A version with no math and no jargon. — Will, with JARVIS*
2. [ ] **WHITEPAPER.md** — **Will Glynn, with JARVIS** · Draft v0.1 · 2026-06-11 · **PRIVATE — stealth, do not distribute**
3. [ ] **ROADMAP.md** — 1. **PoW #5 (does energy vote?)** — **No to the token, yes to mining-for-liveness.** The JUL energy-*money* holder has ZERO consensus weight ("energy circulates, does not vote"). The `pow=0.10` consensus weight belongs to the *act of mining...
4. [ ] **BLOCK-ECONOMY-SPEC.md** — A unified system that turns JARVIS's own session history into a verifiable, owned, valued contribution economy — and, through that, a realistic basis for decentralized consensus and for backwards-enforcing the model layer from the governanc...
5. [ ] **POM-CONSENSUS.md** — An agent's **PoM score** = its accumulated Myerson/Shapley credit across *verified, owned, provenance-complete* blocks (the block-economy value layer). It is a number that says: *this mind has provably contributed this much synergy-weighted...
6. [ ] **DISPUTE-SLASHING.md** — value_v6 priced identity: an all-fresh sybil ring earns 0 because unvested identities pump no flow. The surviving attack: a contributor with EARNED standing builds a novel-garbage child on a fresh-key garbage parent. The certifier clears th...
7. [ ] **OUTCOME-EVALUATOR.md** — The original Phase-1 plan was "replace the coverage proxy with a learned outcome-value and prove the learned v(S) preserves strategyproofness." Proving robustness properties of a learned model is the wrong shape — models drift, attackers pr...
8. [ ] **COHERENCE-LAWS.md** — Money, governance, and capital/franchise are *separable* functions; exactly **three** powers (cognition / compute / capital) form the minimal non-dominated cyclic equilibrium. 2 → binary capture; 4+ → coalitions without added non-domination...
9. [ ] **COORDINATION-SCHELLING.md** — The strong version is correct. State it precisely:
10. [ ] **CRYPTOECONOMICS.md** — **1 PoM = 1 byte of on-chain state.** Storage is the scarce resource (CKB's insight); PoM is the right to occupy it. Your accumulated PoM is your state budget.
11. [ ] **THRONE.md** — Noesis is not a product and not only a protocol. A throne is a seat built for an occupant — it does not rule; it holds the place for the one who does. Operationally: **the mechanism serves and never rules.** Final meaning, final judgment, f...

### Reference (not on the critical path)
- [ ] ARCHITECTURE.md — ---
- [ ] ATTACK-DEFENSE.md — Adversarial self-review, 2026-07-01. Every claim grounded to `file:line`; every residual named. Status discipline: ✅ built (reference layer, tested) · 🟡 designed-not-built · 🔬 open frontier.
- [ ] BUILD-NEXT-tx-digest.md — - `Script { code_hash: [u8;32], args: Vec<u8> }` (lib.rs:32). - `Cell { id: u64, lock: Script, type_script: Script, data: Vec<u8> }` (lib.rs:39, fields 43/49/55). - `TokenTx { standard: TokenStandard, code_hash: [u8;32], args: Vec<u8>, inpu...
- [ ] CKB-VM-PORT.md — - `ckb_vm::run::<R, M>(program: &Bytes, args: &[Bytes], memory_size: usize) -> Result<i8, Error>` — simplest entry; i8 exit code, 0 = success. (`src/lib.rs:41`) - Default machine recipe used by `run` itself (`src/lib.rs`):
- [ ] COMPETITIVE-POSITION.md — Every standard chain is a **possession chain**: it records *who holds which token*, orders blocks by an exogenous cost (burned energy in PoW, staked capital in PoS), and lets an off-chain market set worth. Bitcoin's "work" is hashing — deco...
- [ ] CONSENSUS-REVIEW.md — The load-bearing question was *"does NCI's 60/30/10 break the rock-paper-scissors / separation-of-powers claim?"* The answer turns entirely on **composition**, not the numbers:
- [ ] CONTINUE.md — Full-auto session while Will decides PoM↔finality (that surface untouched). Progress on the approved builds: 1. **Resource-DoS bounding — Bound A SHIPPED ✅** (`de59148`←`b80896e`). `Constitution.max_mempool` cap; `Node::submit`
- [ ] CONTRIBUTING.md — noesis is in a pre-release / stealth period: development is currently closed and external contributions are not yet open. This document describes how the codebase is built and the discipline it is held to, so the workflow is legible now and...
- [ ] CONVERGENCE-REVERSE-FORK.md — Noesis Proof of Mind must be not only **backwards-compatible** (continuous from genesis, prior blocks auditable) but **forwards-compatible** in a stronger sense: other useful-proof-of-work / proof-of- contribution chains should be able to *...
- [ ] DESIGN-elastic-pow-money.md — A stale frame was in play: the older `RESEARCH-NETWORK-CONSENSUS.md` (T3) treats PoW as a near-vestigial sybil-cost dimension "removed from finality." The **v3.2 whitepaper** is canonical and richer: PoW is a **proportional, Ergon-style ene...
- [ ] DESIGN-locksig-binding.md — `is_valid_in_ledger` (runtime.rs) proves a consumed input EXISTS as a finalized cell (identity `id + lock + type_script`), and (o) bound `data` so the amount can't be forged. It does NOT prove the spender CONTROLS the cell. `lock.args` carr...
- [ ] DESIGN-multi-identity-split-acceptance.md — The (q) per-identity λ^r damping caps ONE identity's volume but is INERT against a split across K distinct vested identities (each child is rank-0 in its own identity-group ⇒ λ^0=1 ⇒ full weight). Measured (test `multi_identity_split_volume...
- [ ] DESIGN-onvm-locksig-program.md — - **Verify arithmetic, single-sourced:** `noesis_core::lamport::verify(root, msg, sig)` ((pp)) — no_std, builds riscv64imac. The node's `runtime::lamport` re-exports it. - **Digest, single-sourced:** `noesis_core::tx::tx_digest(standard, co...
- [ ] DESIGN-parametric-clawback.md — Should clawback cascades be consensus-native (a justice-like presence at the wallet level that deals with theft / fraud / harm), or should the broad-harm case be governed by slashing?
- [ ] DESIGN-stolen-offchain-content.md — An attacker ingests content created by someone ELSE who never put it on-chain. The attacker is credited as the contributor and captures the realized-downstream-flow value; the genuine off-chain creator gets
- [ ] EXTRACTION-AUDIT-2026-06-19.md — **NO live extraction vector.** Noesis is GEV-aligned by construction. The classic MEV preconditions (fees, block rewards, producer-chosen ordering, transferable franchise, slash payouts, oracles, liquidations) are **structurally absent**, n...
- [ ] EXTRACTION-AUDIT-CHECKLIST.md — 1. **[Shapley invariant]** every value-paying path (`value_v5..v8`, `value_flow_with_own`) distributes strictly along the provenance DAG by Myerson share. FAIL if a cell can earn without realized external flow (`downstream_flow_external`).
- [ ] EXTRACTION-AUDIT-LOG.md — audit | 2026-06-21 | PASS(12/12) | collusion_slash (gg) burns no counterparty + griefing-resistant (hh) + cross-path residual tracked (ii); 1-10 grep-clean, 11-12 reasoning-clean (slash surface: no rent/order/oracle/platform extraction); su...
- [ ] FOUNDATION-grace-made-mechanical.md — *A record. Will Glynn, with JARVIS. 2026-06-12. Captured in-flight, the moment the moral substrate beneath the whole project became explicit.*
- [ ] HANDOFF.md — - **The thesis is set and honest.** Headline = the **non-zero-sum paradigm**: Noesis is the first blockchain whose competitive relationship to other chains is non-zero-sum. It absorbs rivals (reverse- fork = accretion) instead of competing;...
- [ ] INDEX-DEP-CODEHASH-BINDING.md — `onchain/pom-typescript/src/main.rs:164` reads the index root with: ```rust match load_cell_data(0, Source::CellDep) { Ok(rd) if rd.len() == 32 => { /* accept as root */ } _ => return 20, } ``` Any cell-dep at slot 0 whose data is 32 bytes ...
- [ ] ISOMORPHISM-INVARIANCE-VS.md — Every known `v(S)` gaming vector this repo has closed has the same shape: the attacker **relabels** the contribution graph to manufacture score without adding value.
- [ ] JARVIS-CORE-harness-as-coordination.md — *Draft. Will Glynn, with JARVIS. 2026-06-12. The competitive layer of an AI system is not the model's weights — it is the harness that coordinates models, and the harness wins by grounding their cross-check in structure that cannot lie. PRI...
- [ ] JARVIS-ON-NOESIS.md — ---
- [ ] LAUNCH-CHECKLIST.md — - [ ] 🟡→✅ **THE MOAT — un-gameable `v(S)` on REAL labels.** Seam is wired end-to-end (`load_prefs → train → v_outcome_floored → seed`); runs on SYNTHETIC labels today. Real closure = the DeepFunding-distill-over-sets outcome-label pull. **D...
- [ ] NOESIS-FAQ.md — *Plain, honest answers to the questions people actually ask. Where something isn't built or isn't solved, this says so.*
- [ ] NOESIS-FOR-DUMMIES.md — *Proof of Mind with no math and no jargon — the 5-minute version of the full paper, for the 99% who will never read it.*
- [ ] NOESIS-LITEPAPER.md — *The short version of the full whitepaper — the whole idea, none of the heavy math. For builders, partners, and anyone deciding whether to look closer.*
- [ ] NOESIS-ONEPAGER.md — **Proof of Mind: a value chain for verified contribution.**
- [ ] ON-VM-FINALIZATION.md — `finalizes_hybrid(voters_for, all, mix, now, horizon, decay_pos, threshold_bps, quorum_floor_bps)`: - `weight_for = Σ effective_weight(v)` over voters_for - `eff_total  = Σ effective_weight(v)` over all; `base_total = Σ base_weight(v)`
- [ ] PAPERS.md — Pick your depth. Each accessible doc is also available as PDF, TXT, and HTML in [`dist/`](dist/).
- [ ] POM-FINALITY-TEMPORALITY.md — **The hardest fair question about Noesis:** a contribution's value depends on *downstream* work that hasn't happened yet (who builds on it, what flow it generates), so a Proof-of-Mind score can change after the fact. Consensus weight in PoW...
- [ ] PRIOR-ART-contribution-dag.md — Noesis is not the first to model contribution as a graph and flow value along it. The honest move is to locate exactly where the lineage ends and the new work begins. Four prior-art clusters:
- [ ] README.md — Start at the top and read down by how deep you want to go. Every mechanism doc marks **built vs designed vs open** honestly; the single source of truth for status is [`../ROADMAP.md`](../ROADMAP.md) and [`../STUDY-GUIDE.md`](../STUDY-GUIDE....
- [ ] RELEASE-PLAN-VIBESWAP-ON-NOESIS.md — ---
- [ ] RESEARCH-NETWORK-CONSENSUS.md — **Recommendation: build on `rust-libp2p`, LEAN profile** — QUIC + GossipSub v1.2 + Identify + a **custom Bitcoin-style addr-gossip discovery** (replicate CKB RFC0012 as a `NetworkBehaviour`) INSTEAD of enabling the Kademlia DHT. Gets modern...
- [ ] RESOURCE-DOS-BOUNDING.md — `Node::submit(cell, coord)` (`node/src/runtime.rs`) pushes a proposal onto an unbounded `mempool: Vec<(Cell, Committed)>` at **zero admission cost**. A cell only has to be *well-formed* to be gossiped; it does not have to be *valuable*. So ...
- [ ] ROADMAP-WILLS-EQUILIBRIUM.md — The **Contribution Consensus Problem** (descriptor: *Consensus Without Ground Truth*): decentralized agreement on contribution VALUE/attribution — not order — with NO ground-truth oracle, under SELF-INTERESTED (strategic, ¬ Byzantine) adver...
- [ ] SECURITY-AUDIT-attacker-choosable-inputs.md — | Surface | Critical input | Source today | Attacker-choosable? | Status | |---|---|---|---|---| | Value gate | cell DATA (content) | tx-supplied | yes, BY DESIGN — content is the thing measured | ✅ OK: floors + flow + standing price the co...
- [ ] SECURITY.md — noesis is a pre-launch protocol; there is no public network and no funds at risk. If you find a flaw in the consensus, value, dispute, or execution layers, please report it privately rather than opening a public issue. Open a
- [ ] STATUS-LEDGER.md — | ID | claim | status | evidence | open obligation | |---|---|---|---|---| | **HCE-1-contrib** | (1) Nash: honest contribution is the no-regret unilateral action | demonstrated | novelty->0 padding/sybil, geometric saturation, standing-gati...
- [ ] T7-CROSS-CELL-SIMILARITY.md — Intake floors split by what they read: - **Content-local** (semantic floor): pure function of the cell's bytes — ON-VM since T4. - **History-dependent** (temporal novelty + similarity floor): need `seen` = the union of
- [ ] TEMPORAL-ORDER-ONCHAIN.md — `temporal_novelty` and the index `valid_root_transition` assign shared novelty by ORDER: the earlier-committed cell wins the contested coverage, a later redundant cell earns 0. That is strategyproof ONLY if "earlier" is a relation the block...
- [ ] TOKENOMICS.md — Noesis separates the three functions money systems usually fuse — **money, governance, capital** — into three distinct tokens, and keeps the one thing that must never be for sale, **consensus weight**, out of all of them. Consensus weight i...
- [ ] VISUALS.md — ---
- [ ] ZK-INTEGRATION.md — Design doc, 2026-07-01. Maps the four ZK fits onto the ACTUAL core, with tooling, build order, and an honest tractable-vs-research split. Status: all 🟡 designed / 🔬 to-build — nothing ZK ships today. Grounded to `file:line`; verify before c...

## Code map (`node/src/lib.rs`)

- [ ] `runtime` — Node runtime — the replicated state machine over the mechanism library (orchestration only; two nodes that finalize the same blocks converge...
- [ ] `tokens` — Starter Rust analogs of the ERC token standards in the cell model (fungible/ERC-20, nft/ERC-721, multi/ERC-1155)
- [ ] `soulbound` — SOULBOUND in the cell/UTXO model
- [ ] `ownership` — Bitcoin-shaped ownership (port of block-ownership.py): current owner = genesis folded over a signed transfer log
- [ ] `value` — Capability layer (port of value-v4.py + reward-model Bradley-Terry)
- [ ] `synergy` — Synergy aggregation (port of block-value-v2.py): a SUBMODULAR outcome-value with MYERSON credit, sampled Data-Shapley style
- [ ] `flow` — Eigenvector value-flow over the provenance DAG + two-level recursion (port of `value-flow.py`)
- [ ] `consensus` — PoM-weighted consensus — finalization, retention-decay, and AND-vs-OR composition made concrete and TESTED (build-don't-claim)
- [ ] `stability` — L9 — core / nucleolus stability (no profitable fork)
- [ ] `dispute` — Dispute-window endorsement-slashing (`DISPUTE-SLASHING.md`)
- [ ] `calibration` — Calibration harness (`DISPUTE-SLASHING.md` §8): the dispute stack's parameters (W, B, λ, α, β, γ) and the evaluator's (κ, μ) must satisfy th...
- [ ] `evaluator` — Role-bounded outcome evaluator (`OUTCOME-EVALUATOR.md`)
- [ ] `claims` — Concurrent claims on standing (`OUTCOME-EVALUATOR.md` §5): a contributor's standing is collateral for SEVERAL claimants — dispute restitutio...
- [ ] `value_fixed` — Fixed-point mirror of the INTAKE value rule — CKB-VM-PORT.md code increment #1
- [ ] `semantic` — Semantic / compressibility floor (ROADMAP Phase 1, Role-C — the garbage-novelty gap AT the gate)
- [ ] `outcome` — Learned OUTCOME model over coalitions (`OUTCOME-EVALUATOR.md` §4, Phase-1 frontier)
- [ ] `harness` — Harness checker-routing (the JARVIS core thesis, modeled and tested)
- [ ] `smt` — Sparse Merkle Tree over 64-bit shingle keys — T7 #1 (`T7-CROSS-CELL-SIMILARITY.md`)
- [ ] `settlement_fixed` — Q32.32 settlement mirror — ROADMAP T8 (`CKB-VM-PORT.md` fixed-point map, last entry)
- [ ] `finalization_fixed` — PoM-weighted finalization mirror in Q32.32 — `ON-VM-FINALIZATION.md` build-order step 1
- [ ] `proven` — T7 #2 — the shared proof-driven intake verifier (`T7-CROSS-CELL-SIMILARITY.md` §increments)
- [ ] `index_rule` — T7 #3 — the index-cell root-transition rule (`T7-CROSS-CELL-SIMILARITY.md` §QA R2: per-block batched update)
- [ ] `commit_order` — The fix for the temporal-order attacker-choosable-input finding ([P·dont-let-attacker-choose-critical-input], 2026-06-13)
- [ ] `index_binding` — Host-side reference model of the on-VM index cell-dep binding
- [ ] `nash_honesty` — The self-reporting sub-game of Honest-Contribution Equilibrium: a participant SELF-REPORTS a fact the chain cannot verify (the provenance of...

## Glossary (the load-bearing terms)

- [ ] **PoM (Proof of Mind)** — verified, synergy-weighted contribution as consensus weight, replacing Proof of Work.
- [ ] **Noeum** — the unit — 1 Noeum = 1 byte of state = 1 PoM unit.
- [ ] **temporal-novelty** — value = coverage novel vs earlier-committed blocks (commit-reveal order); strategyproof by construction.
- [ ] **floored novelty** — temporal-novelty after the similarity floor zeroes near-duplicates.
- [ ] **realized-flow gate (v5)** — value = floored_novelty x g(downstream_flow); quality is a realized GATE, not a predicted boost.
- [ ] **priced identity (v6)** — flow seeds count only from contributors whose soulbound standing clears a floor — identity costs earned standing.
- [ ] **soulbound standing** — earned, non-transferable franchise; valid_transition rejects reassignment (no simony).
- [ ] **dispute window (W)** — value vests W epochs after the flow that paid it; refutable while unvested.
- [ ] **causal-share slash** — a refuted certifier loses lambda x (their zero-seed marginal on the target's value) + alpha.
- [ ] **escalation court** — a round-1 PoM-only veto is appealed to the AND-composed full NCI mix; overturned jurors are slashed.
- [ ] **role-bounded evaluator** — the learned v(S) may advance timing + inform disputes, never mint; corrupt-evaluator bound is tested.
- [ ] **Myerson value** — graph-restricted Shapley — value flows only along provenance-connected coalitions.
- [ ] **core / nucleolus** — cooperative-game stability: an allocation no coalition can profitably defect from.
- [ ] **NCI mix** — Nakamoto-Consensus-Infinity weighting PoW 10 / PoS 30 / PoM 60 bps, 2/3 finalization bar.

## The one-sentence spine

Reward is paid only as others build on your work (service, structurally);
identity that certifies must be earned and is slashable when it certifies
garbage; and the learned judge can advance or inform but never mint — so the
measurement stays un-gameable without trusting any model.

