# Sybil surface of the deployed testnet franchise — audit 2026-07-19

> Scope: the **single-node public testnet** as it would ship today (`ChainSpec::testnet()`,
> `node/src/rpc.rs` `serve_api`). Grounded in source read this session; every claim carries a
> `file:line`. Companion to `internal/EXTRACTION-AUDIT-2026-06-19.md` and
> `docs/anti-plutocracy-attack-surfaces-2026-07-17.md`. Status discipline: ✅ built · 🟡 designed · 🔬 open.
> This audit changes **no code** except an honest-scope doc-comment correction; the go-live binding
> flips remain Will-gated (PCP).

## TL;DR

The deployed testnet's write surface is a single endpoint, `POST /submit`, and it is **sound where it
claims to be**: contributions are signature-verified (real XMSS/Lamport), one-time-leaf replay is
rejected, and near-duplicate content is floored out of PoM standing at consensus. **But the property
Noesis exists to demonstrate — that standing reflects genuine contribution — is not enforced on the
deployed path.** The consensus franchise is the v0 novelty oracle, which rewards *first-appearance
coverage* and zeroes only *similar* content. It therefore rewards **varied high-entropy junk
maximally**, at ~zero cost (`submission_deposit = 0`, all on-VM binding flips `false`). A scripted
attacker can farm unbounded PoM standing + coinbase JUL with novel-but-worthless submissions — the
exact inverse of "proof of mind." This is a known-limitation of v0, not a code defect; the point of
this audit is that a **public, permissionless** testnet minting *meaningful* PoM standing is not yet
safe, and the shipping doc-comment overclaims it.

## What the deployed path enforces (sound — do not round down)

The only mutating endpoint is `POST /submit {address,index,ots_root,auth,ots_sig,data}`
(`rpc.rs:392`, `serve_api`). `submit_signed` (`rpc.rs:190`) gates every submission through:

1. **Cryptographic authorization** — the XMSS signature must verify under the claimed address over
   `contribution_digest(address, index, data)` (`rpc.rs:203-205`). A swapped address is rejected
   (`rpc.rs` test `rejects_reused_leaf_index_and_forged_address`). Identity is a real key, not a handle.
2. **One-time-leaf anti-replay** — each Lamport leaf signs once; the index must strictly advance
   (`rpc.rs:209-215`), rebuilt from the durable chain on boot (`rpc.rs:159-162`).
3. **Near-duplicate consensus floor** — PoM standing is `pom_scores_with_similarity_floor_q16`
   (`runtime.rs:1442`, the deployed franchise per `lib.rs:3210`), which zeroes any cell whose coverage
   overlap with earlier-committed coverage exceeds `theta_sim` (`lib.rs:200-216`).
4. **Advisory ingress screen** — rejects too-short / low-information / near-seen content
   (`screen.rs:83-105`), explicitly **node-local and never consensus** (`screen.rs:9-14`).

There is **no token-transfer endpoint**: the `CONTROL_BINDING_ACTIVE`-gated spend path
(`runtime.rs:498-505`, empty-auth authorizes while the flag is `false`) is **not reachable** through
the deployed HTTP surface. The earlier "anyone can spend anyone's JUL" hypothesis was verified false
for the deploy — that surface is inert *and* unexposed.

## The gap — the franchise rewards noise maximally

`coverage(data)` is a set of **4-byte sliding-window FNV shingles** (`lib.rs:153-164`). Temporal
novelty scores a cell by its count of never-before-seen shingles (`lib.rs:180-190`). The similarity
floor only zeroes a cell whose overlap with prior coverage **exceeds θ_sim**, which ships at **0.95**
(`chainspec.rs`/`runtime.rs:165`, `theta_sim_q16 = 62259 = floor(0.95·2^16)`).

Consequence: a **varied random** payload of N bytes yields ≈ N−3 all-distinct shingles with ≈ 0%
overlap against anything on-chain, so it earns novelty ≈ N−3 — far below the 0.95 cut. **The more
incompressible the junk, the higher the standing.** θ_sim catches *similar*; random junk is dissimilar
to everything. `temporal_novelty`'s "strategyproof" property (`lib.rs:176-179`) holds against
*duplication / padding* (a later copy adds no new coverage → 0), **not** against *novel worthlessness*.

The v0 oracle says this itself: `NoveltyOracleV0` "does not yet model *value* beyond novelty. The
learned v(S) is the open moat" (`lib.rs:293-296`); the screen says "a determined attacker can still pad
with *varied* junk to beat both floors; the ratio raises the cost, it does not prove value"
(`screen.rs:28-29`).

### Quantified attack (deployed testnet)

- Generate a varied random string → pass the screen (≈100% distinct ratio, ≈0% seen-overlap) → earn
  novelty ∝ length. No per-cell cap: a 10 KB blob ≈ 10,000 standing per submission.
- Cost: one XMSS sign + trivial PoW (`genesis_bits` low by design) + **0 JUL** (`submission_deposit = 0`).
- Repeat with an advancing leaf index (one identity) or fresh addresses (a Sybil ring — keygen is free).
- Standing keys the finality franchise (`lib.rs:268-269`: per-contributor PoM standing weights finality
  via `finality_pom_weight`; PoM is 60% of the consensus mix and must independently supply ≥50% of its
  dimension). So farmed standing is farmed finality weight.
- On the **single node** this is inert (one finalizer). But the testnet exists to *exercise the
  mechanism*, and the mechanism's core anti-Sybil property is false on the shipping path.

## Why this is not a quick fix (the honest hard part)

The defenses that close it are built but **not deployable on a cold permissionless chain**:

- `value_v5`/`v6` gate value by *downstream flow* (`value = floored_novelty × g(flow)`, `lib.rs:1302-1321`)
  → an isolated junk cell earns ~0. But on a cold testnet this punishes **honest early** contributors
  identically (nobody has downstream flow yet), and `v6+` need bootstrapped `standing` as an input.
- `value_v7` semantic/entropy floor gates only the cell's **seed** (its ability to *certify others*),
  explicitly **not its own base novelty** (`lib.rs:1279-1284`) — so it does not, by itself, deny an
  isolated junk cell its own standing.
- `value_v8` outcome-gating and the **learned v(S) on real labels** are the actual moat, and that mile
  is 🔬 open and data-gated (`lib.rs:293-296`, `docs/DESIGN-value-oracle-seam.md`).

So: **there is no drop-in Sybil-resistant franchise for a cold, permissionless, label-free testnet.**

## The coupling — the two open axes are one knot

The economic Sybil brake is also off, and cannot be turned on independently:

- `submission_deposit = 0` on testnet (`chainspec.rs:91`).
- Raising it > 0 is **gated behind `CONTROL_BINDING_ACTIVE`** at genesis admission (`runtime.rs:921`):
  a bond forfeiture burns a cell, so it must not activate before spends are owner-authorized, or an
  attacker could bond a victim's live JUL for junk and burn it.
- `CONTROL_BINDING_ACTIVE` (`runtime.rs:542`) is exactly the deploy-coupled go-live flip that is deferred.

So the novelty-only franchise **and** the zero submission cost are the *same* deferred milestone. Each
alone is farmable; together the surface is wide open.

## Recommendations

A **local / permissioned** single-node testnet is fine as-is (the operator is the only submitter). For
a **public permissionless** one, resolve the go-live knot as one unit
(`[[interdependent-enforcement-ships-together]]`):

1. Flip `CONTROL_BINDING_ACTIVE` + real-entropy keygen (the browser wallet `frontend/crypto.js`
   already signs) → enables `submission_deposit > 0` → raises Sybil cost above zero. **Will-gated PCP.**
2. Until the graph bootstraps: cap per-identity / per-block standing, or allowlist initial contributors.
3. **Honest-scope the claim** (done in this commit): `chainspec.rs::testnet()` said the testnet "tests
   the REAL security model." It tests the real **PoW / issuance** model; the **PoM contribution
   franchise** ships at the v0 novelty floor, so testnet PoM standing is **not yet a Sybil-resistant
   signal**.

## What this audit does NOT claim

- Not a live exploit on a **single-node** or **permissioned** deploy (no external finalizer set; the
  operator gates submissions).
- Not a defect in v0 — v0 is honestly labelled a novelty heuristic, not v(S).
- Not a token-spend hole (that surface is inert *and* unexposed on the deploy).
- The fix is **not** "ship v7" — v7 does not deny an isolated junk cell its own base standing.

## Status

- 🔬 **open (the moat):** learned v(S) on real labels — the only thing that makes standing reflect value
  on a cold chain.
- 🟡 **designed, deploy-coupled:** `CONTROL_BINDING_ACTIVE` flip → `submission_deposit > 0` (economic brake).
- ✅ **built + sound on the deployed path:** signature auth, one-time-leaf replay, near-duplicate floor,
  advisory screen.
