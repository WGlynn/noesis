---
title: "Noesis Reference Node v0.1 — Status"
subtitle: "Prove-me-wrong audit, 2026-07-02"
date: "2 July 2026"
---

# Noesis Reference Node v0.1 — status

> Framed by the operating question: **not "is Noesis right?" but "how easy is it for someone to
> prove it wrong?"** This document reports what holds, what does not yet, and the exact gap to
> "impeccable." Status discipline: ✅ built · 🟡 designed · 🔬 open. Nothing rounded up.

## Verification run (2026-07-02, on this machine)
- **`cargo test --workspace`: 322 passed, 0 failed.** Compiles clean; suite runs in seconds.
- The number is measured this session, not quoted from memory.

## The invariants hold — each has a named, passing test

The Manifesto §3 invariants are not just designed; they are pinned by tests that fail if the
invariant breaks. This is the core of the prove-me-wrong claim.

| Invariant (Manifesto §3) | Named test | Where |
|---|---|---|
| Lifecycle: accrue/decay/slash accepted, reassignment not | `accrue_decay_slash_same_owner_accepted` | lib.rs:573 |
| Authority cannot be self-declared (earned, not minted at will) | `mint_authority_cannot_be_self_declared` | runtime.rs:1074 |
| Vote-decay ≠ balance-decay | `decay_touches_vote_weight_not_the_staked_balance` | lib.rs:3792 |
| Strategyproof: duplicate/padding/Sybil earns zero | `strategyproof_sybil_and_padding_earn_zero` | lib.rs:711 |
| Anti-concentration: no single dimension finalises alone | `single_dimension_cannot_finalize_under_two_thirds` · `pom_alone_cannot_finalize_anti_concentration` | lib.rs:3679 · runtime.rs:1523 |
| PoW excluded from the finality/safety path | `pow_is_excluded_from_finality` | runtime.rs:1539 |
| Fixed-point mirrors live rule (determinism) | `pos_pom_fixed_mirrors_the_live_finality_rule` | lib.rs:8366 |
| Two nodes converge | `two_nodes_converge_over_rounds` | onchain integration |

## Build & repo hygiene
- ✅ `Cargo.lock` committed · `Cargo.toml` · `Makefile` · `clippy.toml` · `rustfmt.toml`
- ✅ `ARCHITECTURE.md`, `README.md`, `SECURITY.md`, `CONTRIBUTING.md`, `STUDY-GUIDE.md`
- ✅ Deterministic core: a fixed-point (Q32.32) path mirrors the reference and is tested corner-for-corner.

## Gap to "impeccable" (the v0.1 → v0.2 work-list)
1. 🟡 **`rust-toolchain.toml`** — pin the compiler version for byte-reproducible builds (missing).
2. 🟡 **Architecture diagrams** — none in-repo; the manifesto/whitepaper need them (contribution → authority → consensus → finalisation flow).
3. 🟡 **Documented public APIs / doc-tests** — doc-test count is 0; the SDK surface needs runnable doc examples so someone can build without reading all of lib.rs.
4. 🟡 **Reproducible-build instructions** — a one-command `make verify` that a stranger can run to reproduce the 322-green result.
5. 🔬 **Deploy-coupled seams honestly open** — on-VM ordering-coordinate sourcing and the learned value model remain 🟡/🔬 per the patent honest-status; not claimed as done.
6. 🔬 **Independent review** — not yet. The next external milestone: smart protocol engineers trying to break the assumptions (not a paid audit yet).

## The one sentence to drive toward
*Can someone clone this repo, run one command, reproduce 322 green, read the docs, and rebuild
their mental model of Noesis from the patent + docs alone?* Today: tests reproduce ✅; one-command
verify + diagrams + API docs are the gap. Close those and the answer becomes "yes."
