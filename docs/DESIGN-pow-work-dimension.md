# Design note — the PoW work dimension (M2)

> **STATUS (2026-07-13):** M2a-1 ✅ built (arithmetic + data model, `23a90f0`), M2a-2 ✅ built
> (enforcement wiring, this changeset). The difficulty RETARGET rule and every numeric constant
> (genesis bits, block interval, Ergon retarget params, work-clock ceiling, emission rate) are
> ⚑ M3 (Will-gated) and deliberately absent. Honest labels: ✅ built · 🟡 designed · ⚑ Will-gated.
>
> Grounded against `node/src/runtime.rs`, `node/src/jul.rs`, `onchain/noesis-core/src/lib.rs`
> (`pub mod pow`). Every claim carries a code pointer; re-verify at source before quoting (line
> numbers drift).

## What M2 does

Makes `block_work` return a block's REAL, VALIDATED mined difficulty instead of the constant
`WORK_PER_BLOCK = 1`, so:
- JUL issuance becomes difficulty-proportional (`jul::reward_for_work(block_work(b), jul)`,
  `runtime.rs`) — **Lever A goes live**: energy → money.
- the cumulative work-clock (`state.work`, the canonical `now()` every temporal mechanism reads)
  advances by real work.

It ships **flag-gated** (`Constitution.pow_enforced`, default `false`): off ⇒ `block_work` returns
`WORK_PER_BLOCK` and behavior is byte-identical to pre-M2 (proven by the 323 lib + parity suites,
which run with the default). This is the same additive/inert-default discipline as `vesting_w`,
`quorum_floor_bps`, and the JUL shadow modules.

## The seal — work is a PROOF, not a claim

A block carries `PowSeal { bits: u32, nonce: u64 }` (`Block.pow: Option<PowSeal>`, `None` for every
pre-M2 block). `bits` is a Bitcoin-style compact target; the VALIDATED work is DERIVED from it:

- `noesis_core::pow::compact_to_target(bits) -> Option<[u8;32]>` — strict decode (rejects the sign
  bit, a zero mantissa, byte overflow, and an all-zero target).
- `noesis_core::pow::work_from_target(&target) -> u64` — chainwork `floor((2^256-1)/(target+1))`,
  saturating to `u64`, integer-only (no float/clock/rand — the transition-purity contract).

The block **never carries a work number** it could assert. Under enforcement `validate_block` rejects
any block whose `header_digest` does not meet `target(bits)` (`Violation::PowUnmet`) or that carries
no seal (`Violation::PowMissing`), and only THEN does `block_work` derive the work from `bits`. You
cannot claim target `T` without a nonce whose `header_digest ≤ target(T)` (~`T` expected hashes), so
difficulty is not attacker-choosable-for-free. This is the same don't-carry-what-you-can-derive
discipline as the constructed coinbase amount and `TokenTx`'s dropped `minter` field.

The compact-target decode and the chainwork formula were adversarially cross-checked (Council
2026-07-13) against Bitcoin `SetCompact` over 2816 cases and against a bigint reference over 4006
targets — 0 mismatches.

## The header commitment — and why NO prev-block-hash (Will 2026-07-13)

`header_digest(b)` binds the seal to this block's consensus content: `height`, the
canonically-ordered `cells` (full identity + data), the `coords`, the `token_txs`, the `coinbase`
recipient, and the seal's `bits` + `nonce`. It is length-prefixed + domain-separated
(`noesis-pow-hdr-v1` / `noesis-pow-v1`) via `noesis_core::pow` so the future on-VM PoW check is a
move, not a rewrite.

**Token movements are committed by their consensus-consumed bytes, NOT by `TokenTx::digest`**
(Council 2026-07-13). `TokenTx::digest` is a *canonicalized* (input/output-sorted), *auth-free*
signing view; but `apply_transition` appends outputs in **presented slice order** and per-input
validity depends on **`auths`**. Committing only `tx.digest()` would let a solved PoW replay onto a
block with reordered outputs (state-divergent) or swapped auths (differently-authorized once
`CONTROL_BINDING_ACTIVE` flips at deploy). So the header commits, per tx, `standard` + `code_hash` +
`args` + inputs + outputs (in slice order, full-cell framing) + `auths` (positional, length-prefixed).

It **deliberately does NOT commit a prev-block-hash.** This is a considered departure from Bitcoin,
not an oversight:

- **Bitcoin:** PoW + the prev-hash chain *is* the whole consensus. Cumulative work over a hash-linked
  chain secures ordering/history (reorg-resistance). The prev-hash is load-bearing there.
- **Noesis:** PoW is one of three NCI axes (`pow 0.10 / pos 0.30 / pom 0.60`, `lib.rs`) and is
  **excluded from finality** (`finality::FINALITY_MIX = pow 0.0 / pos 1/3 / pom 2/3`, `runtime.rs`,
  because PoW is reorgeable). Safety and ordering are carried by **PoS+PoM finality + coord-sourced
  canonical order** (`is_canonical_order`, the commit-reveal coords), not by a work-weighted hash
  chain. So PoW's actual jobs here are **issuance (JUL), liveness/production, and per-block
  Sybil-cost** — *not* securing chain history. A prev-hash would be **vestigial to PoW's real role**
  and would falsely imply PoW secures ordering.

We decomposed PoW into its separable functions and wired only the ones PoW is actually performing.
That is why removing the prev-hash is principled: it narrows PoW to exactly its comparative advantage.

### Why the "PoW is for issuance" claim carries real weight here (JUL elasticity)

Most PoW coins issue a **scarce** asset that becomes a hoarded speculative store-of-value, so "PoW
turns energy into money" is a weak claim there — the "money" behaves like a commodity, not money.
Noesis JUL is **elastic, energy-pegged, and spend-designed** (mint↔circulate; the literal opposite of
scarce, inelastic, soulbound standing — see `docs/TOKENOMICS.md`). So "PoW is *for issuance of
money*" is literally true: the issued thing behaves like money. The elasticity is the receipt behind
narrowing PoW to issuance + liveness + Sybil-cost and handing ordering/finality to PoS+PoM.

## What is ⚑ M3 (do NOT infer these from the code)

- The difficulty **retarget rule** + its constants (block-interval target, Ergon-style retarget
  window/params). M2a-2 accepts any valid compact target and derives its work; there is no retarget
  controller yet, so issuance is self-limiting per block (claiming more difficulty costs more energy)
  but the network-wide difficulty is not yet regulated.
- **Genesis bits** (the starting difficulty) — coupled to the genesis bootstrap (PoW starts genesis,
  bonded PoS finalizes; `[[project_noesis-genesis-bootstrap-decision]]`).
- A **work-clock ceiling** (defense-in-depth against a validly-mined-but-enormous difficulty inflating
  `state.work`). Today `work_from_target` saturates `u64` safely (no wrap, Council-verified) and
  pushing the clock requires actually doing the work, so this is a future governable clamp, not a live
  hole.
- **Emission numbers** — `JulParams` defaults are v0 unit-definitions (1 JUL per unit of work), NOT
  pinned economics (`jul.rs`, `docs/DESIGN-jul-money-layer.md` §5).

## Forward constraints (deferred, not open)

- **On-VM mirror:** the target math already lives in `noesis_core` (no_std); the on-VM PoW check
  reuses it. `header_digest`'s preimage layout is node-side (Block-specific) and will be mirrored from
  cell syscalls at deploy — the same boundary as the lock-sig / finalization on-VM ports.
- **Live mining / difficulty regulation:** a node only VERIFIES a seal; production-side mining and the
  retarget controller land with M2a-2's ⚑ M3 numbers and the genesis/P2P work (L7).
