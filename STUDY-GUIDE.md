# Noesis — living study guide (generated; do not hand-edit)

> Regenerated FROM the repo by `scripts/study-guide.py`, so it cannot lag the
> code. Tick the boxes as you internalize each piece. Re-run to refresh.
> Node test suite: **151 passing**.

## Read in this order

1. [ ] **WHITEPAPER-FOR-DAD.md** — *A version with no math and no jargon. — Will, with JARVIS*
2. [ ] **WHITEPAPER.md** — **Will Glynn, with JARVIS** · Draft v0.1 · 2026-06-11 · **PRIVATE — stealth, do not distribute**
3. [ ] **ROADMAP.md** — - ✅ **demonstrated** — runs, tested on real blocks this session - 🟡 **designed** — specified, not yet built - 🔬 **research** — open problem, no settled approach
4. [ ] **BLOCK-ECONOMY-SPEC.md** — A unified system that turns JARVIS's own session history into a verifiable, owned, valued contribution economy — and, through that, a realistic basis for decentralized consensus and for backwards-enforcing the model layer from the governanc...
5. [ ] **POM-CONSENSUS.md** — An agent's **PoM score** = its accumulated Myerson/Shapley credit across *verified, owned, provenance-complete* blocks (the block-economy value layer). It is a number that says: *this mind has provably contributed this much synergy-weighted...
6. [ ] **DISPUTE-SLASHING.md** — value_v6 priced identity: an all-fresh sybil ring earns 0 because unvested identities pump no flow. The surviving attack: a contributor with EARNED standing builds a novel-garbage child on a fresh-key garbage parent. The certifier clears th...
7. [ ] **OUTCOME-EVALUATOR.md** — The original Phase-1 plan was "replace the coverage proxy with a learned outcome-value and prove the learned v(S) preserves strategyproofness." Proving robustness properties of a learned model is the wrong shape — models drift, attackers pr...
8. [ ] **COHERENCE-LAWS.md** — Money, governance, and capital/franchise are *separable* functions; exactly **three** powers (cognition / compute / capital) form the minimal non-dominated cyclic equilibrium. 2 → binary capture; 4+ → coalitions without added non-domination...
9. [ ] **COORDINATION-SCHELLING.md** — The strong version is correct. State it precisely:
10. [ ] **CRYPTOECONOMICS.md** — **1 PoM = 1 byte of on-chain state.** Storage is the scarce resource (CKB's insight); PoM is the right to occupy it. Your accumulated PoM is your state budget.
11. [ ] **THRONE.md** — Noesis is not a product and not only a protocol. A throne is a seat built for an occupant — it does not rule; it holds the place for the one who does. Operationally: **the mechanism serves and never rules.** Final meaning, final judgment, f...

### Reference (not on the critical path)
- [ ] CKB-VM-PORT.md — - `ckb_vm::run::<R, M>(program: &Bytes, args: &[Bytes], memory_size: usize) -> Result<i8, Error>` — simplest entry; i8 exit code, 0 = success. (`src/lib.rs:41`) - Default machine recipe used by `run` itself (`src/lib.rs`):
- [ ] CONSENSUS-REVIEW.md — The load-bearing question was *"does NCI's 60/30/10 break the rock-paper-scissors / separation-of-powers claim?"* The answer turns entirely on **composition**, not the numbers:
- [ ] CONTINUE.md — - **T7 DONE — the execution tier (T1-T8) is COMPLETE.** Mint side now requires PROOF: every group output must prove its novelty against the live index root (cell-dep 0, 32 raw bytes) via the canonical witness blob (concatenated 64×32B sibli...
- [ ] FOUNDATION-grace-made-mechanical.md — *A record. Will Glynn, with JARVIS. 2026-06-12. Captured in-flight, the moment the moral substrate beneath the whole project became explicit.*
- [ ] HANDOFF.md — Resume point for a fresh chat. Detail lives in `CONTINUE.md` (top block) and `ROADMAP.md`; this is the fast orientation. Repo: `WGlynn/noesis` (private remote). Node: `node/`, Rust.
- [ ] JARVIS-CORE-harness-as-coordination.md — *Draft. Will Glynn, with JARVIS. 2026-06-12. The competitive layer of an AI system is not the model's weights — it is the harness that coordinates models, and the harness wins by grounding their cross-check in structure that cannot lie. PRI...
- [ ] README.md — ```mermaid flowchart TD CONTRIB["Block of thought (contribution)"] --> VAL["Value: temporal-novelty × learned quality<br/>strategyproof — sybil / padding / collusion → 0"] VAL --> POM["Proof of Mind score<br/>(accumulated Myerson value)"]
- [ ] T7-CROSS-CELL-SIMILARITY.md — Intake floors split by what they read: - **Content-local** (semantic floor): pure function of the cell's bytes — ON-VM since T4. - **History-dependent** (temporal novelty + similarity floor): need `seen` = the union of
- [ ] VISUALS.md — ---

## Code map (`node/src/lib.rs`)

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
- [ ] `proven` — T7 #2 — the shared proof-driven intake verifier (`T7-CROSS-CELL-SIMILARITY.md` §increments)
- [ ] `index_rule` — T7 #3 — the index-cell root-transition rule (`T7-CROSS-CELL-SIMILARITY.md` §QA R2: per-block batched update)

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

