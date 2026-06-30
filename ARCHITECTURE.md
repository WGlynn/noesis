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
  → `consensus::NCI` at `node/src/lib.rs:3289`. Supermajority `TWO_THIRDS_BPS = 6667`
  (`lib.rs:3291`). This is the *whole-system* weight (production + ordering + finality inputs).
- **Finality mix — PoW EXCLUDED:** `pow 0.0 / pos 1/3 / pom 2/3`, renormalized so PoS+PoM
  sum to 1. → `finality::FINALITY_MIX` at `node/src/runtime.rs:584`.
  - **Why PoW is out of finality:** PoW is probabilistic and reorgeable, so putting it on
    the *safety* path would itself be the hazard. PoW secures block production, ordering, and
    Sybil-cost (and is the money layer, JUL) — not safety. Finality routes through
    `finality::finalizes_pos_pom` (`runtime.rs:608`).
  - **2/3 bar is over the PoS+PoM fast-final set**, not a global total.
- **Anti-concentration floor:** each dimension (PoS and PoM) must *independently* supply
  ≥ **50%** of its own dimension's total to finalize. → `MIN_DIM_BPS = 5000` (`runtime.rs:596`),
  `dim_ok` (`runtime.rs:598`). This is what makes neither axis sufficient alone:
  - a gamed PoM (the 2/3 share) cannot finalize without the PoS dimension also clearing 50%;
  - capital holding *all* stake controls only the PoS dimension, and the unbuyable PoM
    dimension must independently clear 50% — **capital cannot finalize without contribution's
    consent.** This is the anti-plutocracy property; it is a per-dimension floor, not a cap.
- `MIN_STAKE = 100.0` bounds validator Sybils (`lib.rs:3397`).

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
- `v5` realized-flow gate · `v6` standing-gated seeds (prices identity, not participation) ·
  `v7` semantic/entropy floor · `v8` learned-outcome gate (Bradley-Terry over coalition features).
- Anti-gaming defenses are projections of one invariant (see `docs/ISOMORPHISM-INVARIANCE-VS.md`):
  within-identity λ^r, cross-identity μ^m, joint ρ^j damping; `attribution_circulation` +
  `attribution_cycle_energy` (Helmholtz–Hodge) → `collusion_slash`; θ_sim near-dup floor.
- **🔬 open / THE moat:** learned `v(S)` on real DeepFunding labels (first real-data test
  returned NULL, ROADMAP (ww)); general isomorphism-invariance gate (cand-A). Un-gameability is
  claimed for *demonstrated* vectors only, never as a finished proof.

## Security model — see `SECURITY.md` (read it; don't paraphrase from here)

Four attack classes, honest status: (1) `v(S)` gameability ✅ built (reference+runtime),
isomorphism-invariance open; (2) DoS/spam — economic ✅, **resource-DoS Bound A (mempool cap)
✅, Bound B commit-deposit 🟡**; (3) double-spend ✅ reference / 🟡 deploy-coupled crypto;
(4) rollback/finality ✅ reference / 🟡 on-VM mirror. Structural maxim: *don't let the attacker
choose a security-critical input* (every consensus value re-derived on-VM, rejected if not
reconstructable).

## Status discipline

✅ built & tested · 🟡 designed, not built · 🔬 open problem. Never round designed up to built.
Test count + green status: `node/README.md` (doc-coherence hook stamps it against code).
