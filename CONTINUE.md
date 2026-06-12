# CONTINUE — Noesis handoff (PRIVATE, stealth)

## ▶ RESUME HERE (2026-06-11 night — AFK full-auto run)
Shipped this session (all pushed to `WGlynn/noesis`):
- **Visuals embedded INSIDE every doc** (not a central file): WHITEPAPER (4), BLOCK-ECONOMY-SPEC
  (2), POM-CONSENSUS (4), CRYPTOECONOMICS (3 + misconception callout), COORDINATION-SCHELLING
  (4), COHERENCE-LAWS (AND-vs-OR), ROADMAP (2), WHITEPAPER-FOR-DAD (2), node/README (1), README
  (system map + rewritten landing page), CONTINUE (status map). Mermaid; renders on GitHub.
- **COHERENCE-LAWS L12 + L1 amend — "composition before weighting (AND over OR)."** Resolves
  *"does 60/30/10 break RPS?"*: **verified vs `NakamotoConsensusInfinity.sol:19`** — NCI is
  OR-additive (`W = 0.10·PoW + 0.30·PoS + 0.60·PoM`), so Noesis declaring AND is a real
  divergence, not a relabel. One-liner (Will): *"60% PoM is only dangerous if it's a 60% vote."*
  Plus a 6-objection devil's-advocate hardening (liveness ≠ safety-AND; independence is
  load-bearing on L2 ∧ L5; per-dimension provisioning floor; no laundering of NCI's OR-risk;
  the <50% single-proof cap is insufficient under correlation; tie-break must be
  content-independent, never weight-proportional).
- **Rust port: `value-flow.py` → `node/` `flow` module** — eigenvector value-flow (damped,
  bounds self-reference = §8 guard mechanical) + two-level recursion (2-player closed form +
  N-contributor reusing the synergy game). **node 22/22 → 28/28.**
- **Rust consensus module + RSAW adversarial self-audit** (`consensus` mod) — PoM-weighted
  finalization, retention-decay, 2/3 bar (single dimension can't finalize alone), capital-drift
  + symmetric-decay fix, all TESTED. **Self-audit found the effective-weight liveness fix opens
  an ECLIPSE surface** (shrink the denominator → attacker finalizes alone); a **quorum-floor
  hybrid** closes it (both demonstrated as tests). POM-CONSENSUS resolution updated. **node
  28/28 → 39/39.** Audit gaps logged in-code (A2 log-scaling/saturation, A3 sybil econ, A4
  lifecycle, A5 slashability-under-decay) — open.

**NCI finalize path VERIFIED (this run):** `finalizeProposal` = **2/3 supermajority**
(`FINALIZATION_THRESHOLD_BPS = 6667`) of summed retention-adjusted combined `W` — so 60/30/10
IS a finalization vote-weight (OR-additive), **but threshold-hardened**: the 2/3 bar sits above
PoM's 60% ceiling ⇒ no single dimension finalizes alone (capture needs PoM + >6.67% of a second
dimension). L12 refined accordingly. Next: fold the L12 provisioning-floor into a machine
coherence check; type-script PoM (RISC-V) + PoM-weighted finalization. (Separately, the
ethresearch GEV Part 4 draft on Desktop was formatted — outside this repo.)

## ▶ RESUME HERE (2026-06-11 eve — chat rotated at 214k ctx)
Shipped this session (all pushed to `WGlynn/noesis`, head `7842e4e`):
- **VISUALS.md** (8 Mermaid figs) + Desktop render `noesis-figures.html`.
- **COORDINATION-SCHELLING.md** — Schelling/inward-outward synthesis + equi-dependence
  keystone + **meta-security** (LLM+DeFi coordinate through JARVIS on Noesis) + invariant.
  Folded into WHITEPAPER §5.2.
- **COHERENCE-LAWS.md** (L1–L11; L11 = coordination-layer integrity ≥ max spoke surface).
- **Fair launch RATIFIED = genesis-burn** (provable > asserted). WHITEPAPER §10.
- **WHITEPAPER-FOR-DAD.md** + Desktop PDF `Noesis-in-Plain-English.pdf`.
- **scripts/harvest-noesis.py** (code-only pick-list, 9 buckets) + daily cron `41445bfe`
  (⚠ 7-day expiry — needs self-perpetuation for permanence).
- **Rust:** `node/src/lib.rs` `synergy` module (Myerson, Data-Shapley sampling) — **20/20 tests.**
- Memory: `primitive_meta-security-coordination-hub.md` (local, discretion:internal; NOT yet MEMORY.md-indexed).

**Top next steps:** (1) port `value-flow.py` (eigenvector + 2-level recursion) → Rust;
(2) make harvest cron self-perpetuating; (3) Phase-1 open: prove LEARNED v(S) preserves
strategyproofness. See OPEN THREADS below for the full list.

Public-side this session (separate, codeword-free): integrity root **re-attested + signed**
(drift was benign +2 files); leak-gate hardened (4 codeword sites scrubbed + self-skip
removed so the scanner polices its own public mirror).


> Read this first on a fresh session to continue the Proof-of-Mind value chain.
> Repo: `~/jarvis-private` (private remote `github.com/WGlynn/jarvis-private`).
> The roadmap-advance cron loop (`3b8e2f47`, every 3h) auto-continues this.

## What this is
**Noesis** (provisional name) — the value chain Bitcoin is mistaken for. Proof of Mind
(verified, synergy-weighted contribution) replaces Proof of Work for consensus. CKB-shaped:
Rust + RISC-V (CKB-VM) + Cell model + state-rent. **Core inspiration = Nervos CKB
(github.com/nervosnetwork/ckb), keep that lineage.** Full context: `WHITEPAPER.md`.

```mermaid
flowchart LR
  subgraph DONE["✅ Demonstrated (tested)"]
    d1["Bitcoin-shaped ownership (UTXO fold)"]
    d2["Ed25519 signing + tamper-resistance"]
    d3["Synergy value v2 + Myerson (sampled)"]
    d4["temporal-novelty (strategyproof)"]
    d5["Rust node: ownership · soulbound · synergy (39/39)"]
    d6["value-flow: eigenvector + two-level recursion (ported)"]
    d7["consensus: 2/3 finalization + retention-decay + A4 (equivocation/early-reject) + A2 log-scaling + L9 stability w/ nucleolus least-core solver + RSAW self-audit (eclipse→quorum-floor, sybil, slashability) + value-layer adversary (provenance-forgery, quality-bound) (54/54)"]
  end
  subgraph TODO["🟡 Designed / next"]
    t1["reward-model outcome-evaluator (close garbage-novelty gap)"]
    t2["learned v(S) preserves strategyproofness"]
    t3["type-script PoM program (RISC-V)"]
    t4["PoM-weighted finalization + core/nucleolus"]
  end
  DONE -->|"critical path"| TODO
```

## Built + TESTED (demonstrated, not claimed)
- Python prototype: `block-ownership.py` (UTXO transfer-fold), `block-value-v2.py`
  (Myerson synergy), `value-v3.py` (temporal-novelty, strategyproof), `value-v4.py`
  (novelty × quality), `pom-score.py` (PoM = consensus weight), `value-flow.py`
  (eigenvector + 2-level recursion), `adversarial-game.py` (sybil/padding/collusion all
  → 0 under temporal-novelty), `reward-model.py` (Bradley-Terry learned v(S)).
- **Rust node** (`node/`, `cargo test` = 5/5): Cell model, lock script (ownership) +
  type script (encapsulates PoM), temporal-novelty value, pom_scores, shardability,
  ownership transfer-fold. CKB-attribution README.

## Key results / decisions
- **Value rule = temporal-novelty** (commit-reveal order): strategyproof by construction
  (sybil/padding/collusion earn 0). Inter-block = temporal-novelty; intra-block co-authors
  = Myerson (synergy). Composed with learned quality: value = novelty × (1+quality).
- **Cryptoeconomics** (`CRYPTOECONOMICS.md`): 1 PoM = 1 byte of state (CKB direct-port);
  issuance reinterpret (PoM minted by contribution, earned not bought); rent augment
  (PoM decay). **PoM soulbound** (non-transferable) → consensus/franchise; **state-bytes
  transferable** → medium of exchange; buy storage, not consensus.
- **3-token = RPS equilibrium** (capital/compute/cognition = state-stake / PoW-JUL / PoM).
  3 is minimal for non-dominated capture-resistance. PoW relocates to the **money layer
  (JUL)**, orthogonal to PoM. JUL NOT yet integrated (honest open item).
- **Consensus**: PoM-weighted + Nakamoto-Infinity fallback (`POM-CONSENSUS.md`). Stability
  = core/nucleolus. Slashing = invalid-reveal + refuted-value dispute window.

## NEXT increments (critical path, do-it-right + test each)
1. **Port Python → Rust** (continue): value-v4 (novelty×quality) + adversarial tests +
   reward-model (Bradley-Terry) into the `node/` crate. Idiomatic Rust.
2. **Phase 1 still-open** (🔬): prove the LEARNED v(S) preserves the novelty/strategyproof
   property; attribution-ring under the learned model; decay + reviewer-diversity.
3. **Type-script PoM program** (RISC-V) — the actual on-VM validation; integrate `ckb-vm`
   crate (verify APIs against the CKB source, don't assume).
4. **Cryptoeconomics open**: decay rate/half-life, contributor floor, JUL integration.
5. **Consensus**: PoM-weighted finalization + core/nucleolus stability in code.

## Honest load-bearing risk
The whole thing rests on **un-gameable `v(S)`**. The coverage proxy is strategyproof;
the *learned* reward model must preserve that. The adversarial-gaming loop is the moat —
keep running it against every new `v(S)`.

## Naming (LOCKED 2026-06-11)
- **Noēsis** = the network (the act of mind). Crate name; private repo `WGlynn/noesis`.
- **Noeum** = the unit / token (1 Noeum = 1 byte of state = 1 PoM unit; Ethereum/Ether shape).
- **Web-checked 2026-06-11:** no established crypto token named Noesis or Noeum (only a
  Solana NFT-game "Quantum Noesis" using the $SNS token, and a one-off Noesis NFT) →
  appears available. Trademark/domain check still TODO before any public reveal.
- Both names are in the leak-gate (`~/.claude/state/private-leak-patterns.txt`) — keep
  out of public during stealth.

## SESSION 2026-06-11 PM — shipped (repo renamed jarvis-private → noesis)
- **value-v4 ported to Rust** (`node/`, novelty × (1+quality), Bradley-Terry quality, normalized 0..1). Multiplication keeps novelty floor dominant: redundant cell = 0 even at max quality (tested).
- **3-attack adversarial moat ported to Rust** (sybil / padding / collusion-ring all earn 0; honest keep novelty).
- **SOULBOUND resolved in code** — `soulbound` module: soulbound is NOT a data flag (UTXO has no account to freeze); it is a TYPE-SCRIPT INVARIANT on the consume→produce transition. `valid_transition` admits only identity-preserving successors (accrue/decay/slash/burn), REJECTS any owner/contributor reassignment. Two-cell mint: transferable **capacity cell** (money) rides the ownership fold; soulbound **standing cell** (franchise) cannot move. `pom_scores` now keys by contributor (`type_script.args`), NOT owner lock. **node tests 5/5 → 16/16.**
- **doc-coherence gate built** (`scripts/doc-coherence.py`) — closes the docs-lag-code information asymmetry. code content-hash; docs stamped with the hash they were reconciled against; `--check` fails if code moved past stamp; machine-checks (no `jarvis-private` refs, doc test-counts == cargo). NOT yet `--stamp`ed, NOT yet wired as pre-commit hook.
- **Boot-bind**: `~/.claude/session-chain/private-handoff-loader.py` (registered in settings.json SessionStart, after session-state-loader) now surfaces this handoff at every boot — fixes the "reboot drifts to public task" class. Generic globs only, no private nouns in source (sync-safe).

## SESSION 2026-06-11 (eve) — visuals + Schelling synthesis
- **`VISUALS.md`** — 8 Mermaid figures (value pipeline, two-cell mint, 3-power RPS,
  consensus stack, inward/outward Schelling fold, fair-launch decision, ToM→ETM→PoM,
  mint↔sink). Renders on the private GitHub remote.
- **`COORDINATION-SCHELLING.md`** — deployment thesis: JARVIS-as-Schelling-point →
  same reconciliation fold at two radii (inward = coherent self, outward = network).
  Two load-bearing edges: protocol-not-platform; openness-is-what-makes-it-focal.
  Whitepaper §5.2 added.
- **Fair launch DECIDED (recommend): genesis-burn > chain-reset** — provable fair launch
  (pre-launch blocks auditable, PoM/value burned to 0 on-chain at launch height) beats a
  reset (asserted, trust-me). In WHITEPAPER §10 + COORDINATION-SCHELLING. Will to ratify.
- TODO queued: (a) **whitepaper-for-dads** (plain-language explainer); (b) **simple cron**
  that greps Will's own repos for items that DIRECTLY serve the noesis roadmap (indirect later).
- **DONE (a)** `WHITEPAPER-FOR-DAD.md` + Desktop PDF (`Noesis-in-Plain-English.pdf`).
- **DONE (b)** `scripts/harvest-noesis.py` (code-only, 9 mechanism buckets, ~614 candidates)
  + daily cron `41445bfe` (durable; 7-day auto-expire — add self-perpetuation for permanence).
  Output `NOESIS-HARVEST.md` (gitignored, regenerable).
- **DONE — Rust port continued:** `synergy` module = block-value-v2.py (submodular coverage
  value + **Myerson** graph-restricted Shapley, Data-Shapley sampling, deterministic SplitMix64
  PRNG, no `rand` dep). Tests prove cooperative game is load-bearing: synergy-Shapley ≠ additive
  Copeland (L1>0.02), Myerson restricts value to provenance, redundant→low marginal, sampling
  deterministic. **node tests 16/16 → 20/20.** Next un-ported: value-flow.py (eigenvector + 2-level recursion).

## OPEN THREADS — do next session
1. **Finish doc reconciliation** (Will: "they're all outdated… docs never lag code"). Systematic fixes across WHITEPAPER / BLOCK-ECONOMY-SPEC / POM-CONSENSUS / CRYPTOECONOMICS / ROADMAP / node/README / CONTINUE: (a) kill the owner-vs-contributor / transferable-PoM conflation (now resolved = soulbound two-cell, consensus reads contributor); (b) make temporal-novelty × quality the canonical value rule everywhere; (c) names (noesis) + test counts (16). Then `python scripts/doc-coherence.py --stamp` and install it as a git pre-commit hook (`.git/hooks/pre-commit`).
2. **README.md still says `jarvis-private` + "NO public remote"** — rewrite (repo=noesis, private remote exists, push freely). Write was started; not applied.
3. **COHERENCE-LAWS.md** — Will: "set laws/rules/standards of cryptoeconomic coherence." ~10 invariants drafted in-context (separation-of-powers/RPS, soulbound-franchise/no-capital→consensus, conservation-of-proof/GEV, mint↔sink balance, strategyproof-minting, closed-value-provenance, contributor-floor, append-only-slashable, core/nucleolus stability, two-axis robustness). Write it as the anchor doc the others reference.
4. **token↔proof mapping** — now RESOLVED by the two-cell split: PoM-byte = tradable **capacity** (state, money-ish); franchise = soulbound **standing** + VIBE validation; JUL = PoW/money. Buy storage, not consensus, ENFORCED (pom keys by contributor). Still verify vs NCI contracts (`a442fc5b`) before reusing labels (tokenomics-zero-tolerance).
5. Roadmap next code increment: port `reward-model.py` (Bradley-Terry learned v(S)) into `node/` and prove it preserves the novelty/strategyproof property.
6. **Living study guide** (Will: "a living breathing study guide locally that updates with its contents, so I can study and internalize it all over time"). Build `scripts/study-guide.py` → generates `STUDY-GUIDE.md` FROM the repo (so it can't go stale, same philosophy as the coherence gate): read-in-order path, per-doc one-line synopsis, module/file map, glossary of key terms (PoM / Noeum / temporal-novelty / Myerson / soulbound two-cell / core-nucleolus), the key decisions + WHY, test inventory from `cargo test`, and progress checkboxes Will ticks as he internalizes each piece. Wire it to regenerate alongside the doc-coherence stamp (and optionally the pre-commit hook) so it tracks contents automatically. Pairs with [F·will-learning-goals].

## Language decisions (general, saved to memory)
Saved `memory/primitive_language-decision-router.md` (domain→language router; substrate-fit ¬ popularity; full-stack map; **strengths-lens** = find a language's strength in what others call its weakness, same as treating any mind by strengths). MEMORY.md index line still PENDING (deferred under context-rotation — add it in the fresh chat).
