# Sub-Block Security — Assumptions, Threats, Mitigations

> Status: security analysis / design-lens (ready-for-critique, NOT validated). Grounded in the shipped
> lean core `node/src/subblock.rs` (T9 slice 1) + `internal/RESEARCH-NETWORK-CONSENSUS.md` (T9) +
> `docs/research/role-separation-as-design-law.md`. Honest labels throughout; open questions flagged ⚑/🔬.
> Companion memory: `[[jul-elasticity-dissolves-deep-capital]]` (adjacent), `[[noesis-never-halt-chain]]`.

## 0. Why this document exists

Sub-blocks add a fast (~2 s), revertible transaction tier under the 120 s ordering blocks. Any new tier
is a new attack surface. This note states the security model explicitly, enumerates the threats, and
shows where the blast radius is bounded by construction versus where a real mitigation must be built.

## 1. The security-relevant model

- A **sub-block** is a fast, REVERTIBLE batch of VALUE transactions proposed between two ordering blocks
  by a contribution-weighted producer. It is soft-confirmation only.
- It is **NOT finalized state.** It touches no finalized cell set and never enters `state_digest`
  (grounded: `subblock.rs` is a consensus-isolated shadow — `validate_sub_block` reads a *provisional
  overlay*, mutates nothing). Settlement remains PoS+PoM finality on the **ordering block** that later
  ABSORBS the sub-block's txs.
- The producer gate is **PoM standing ≥ threshold** (Ergo's `H < T/64` weak-PoW re-derived to our
  substrate physics: contribution, not energy — `[[cross-port-fn-var-audit]]`).
- Sub-blocks carry **value txs only, never contributions** (a contribution's value is vesting-gated over
  `W`, so it has nothing to fast-confirm — role-separation, and it keeps the attribution/finality surface
  untouched by the fast tier).

## 2. The load-bearing property (bounded blast radius)

**Sub-block compromise degrades soft-confirmation UX; it CANNOT break settlement safety.** Two structural
reasons, neither of which is a trust assumption:

1. **Revertible by construction.** Sub-block validation mutates no finalized state and is excluded from
   `state_digest`. The worst a fully-malicious fast tier can do is make soft-confirmations unreliable.
2. **Finality-excluded.** Finality is PoS+PoM on ordering blocks (`FINALITY_MIX.pow == 0`); the fast tier
   never contributes finality weight. A totally captured sub-block tier leaves the finalized ledger
   exactly as safe as with no sub-blocks at all.

This is the same role-separation dividend as the committee clock ("total clock compromise degrades cadence,
never safety"). **Consequence:** the security question for sub-blocks is *not* "can they break settlement"
(they structurally cannot) but "how reliable is soft-confirmation, and is revertibility honestly enforced."

## 3. Security assumptions (explicit)

- **A1 — Honest UX contract enforced.** Wallets display a soft-confirmed tx as REVERTIBLE (`ConfirmationTier`
  is the protocol-level signal); high-value flows wait for `Final` (ordering-block absorption + PoS+PoM
  finality). A client that treats soft as final assumes the user's own risk — but the protocol MUST make
  the tier unmistakable. Revertibility is never hidden (Ergo's honest contract, verbatim intent).
- **A2 — The standing threshold is calibrated.** High enough that fast-tier Sybil/equivocation costs real
  PoM standing; low enough that liveness holds (enough qualified producers). Value = ⚑/testnet-pinned.
- **A3 — The ordering block is authoritative.** On any conflict between a sub-block tx and the absorbing
  ordering block, the ordering block wins. Sub-block state is provisional.
- **A4 — One ordering interval bounds the reversion window (~120 s).** A soft-confirmed tx is either
  absorbed by the next ordering block (→ path to Final) or reverts.
- **A5 — Absorption is RE-INCLUDE (self-contained blocks), so there is NO settlement/history DA assumption.**
  Resolved 2026-07-14 (Will): the ordering block CARRIES the absorbed txs in its body (`token_txs`) and
  commits their Merkle root (`subblock::subblock_txs_root`) in the HEADER — Bitcoin-standard (data in body,
  root in header for PoW binding + light-client inclusion proofs). The block is fully self-contained and
  replayable; a bare-root/validium data-availability requirement was CONSIDERED and REJECTED because it buys
  little (a no-sub-block block carries those same txs anyway — re-include is not heavier than baseline) at the
  cost of a real DA risk. The only residual availability need is EPHEMERAL and non-safety: a sub-block's txs
  must reach peers for the ~2 s soft-confirm; if they don't, the tx simply isn't soft-confirmed (it still
  settles normally in the ordering block). Propagation efficiency (not safety) is recovered with BIP152-style
  compact blocks over the 6-byte `weak_tx_id`s, so a node that followed the fast tier reconstructs the
  ordering block from cache without re-download. `[[validity-not-availability]]` (the distinction that made
  the tradeoff legible; the conclusion here is to keep the data on-chain).

## 4. Threats and mitigations

| # | Threat | Mitigation | Residual / status |
|---|---|---|---|
| **T1** | **Soft-confirm double-spend** (fast-payment attack): producer soft-confirms a payment, then a conflicting spend is what the ordering block absorbs. | Revertibility contract (A1): a merchant releasing goods on soft-confirm takes an informed risk; high value waits for `Final`. Damage bounded to soft-confirmed-**and-acted-upon** value within one interval. | Need a principled "wait N" number: quantify double-spend success vs wait-time (Bitcoin/Ergo 0-conf style). 🔬 |
| **T2** | **Producer equivocation**: a qualified producer proposes two conflicting soft-chains to different peers. | Only ONE chain is absorbed by the ordering block (canonical resolution at settlement, A3); equivocation is *attributable* (producer signed both) ⇒ slashable against PoM standing — the gate provides the stake to slash. | Slashing wiring is a later slice (design hook, not built). 🟡 |
| **T3** | **Fast-tier Sybil / spam**: flood the tier with sub-blocks. | The contribution-weight gate (must hold PoM standing ≥ threshold — grounded: `InsufficientStanding`) + sequential `seq` + the per-interval cadence. A no-contribution actor is gated out entirely. | Threshold calibration (A2). ⚑ |
| **T4** | **Withholding / censorship**: soft-confirm a tx, then withhold it from the ordering block. | Any qualified producer proposes the ordering block; a withheld tx simply is not absorbed ⇒ reverts (soft was never a promise). | No safety impact by construction. ✅ |
| **T5** | **Soft-chain reorg**: sub-blocks reordered/dropped before absorption. | By design the soft-chain is provisional; intra-interval reorg is expected and bounded by A4. | Expected behaviour, not a bug. ✅ |
| **T6** | **Cross-tier inconsistency**: soft-confirm a spend of a cell not live in finalized state. | `validate_sub_block` validates against the provisional overlay built ON TOP of the finalized token set (grounded: `provisional_live` starts from `ledger.token_cells`), so it cannot soft-confirm a phantom/retired input. At absorption, re-validation vs the (possibly advanced) finalized state is authoritative. | ✅ (re-validation-at-absorption is part of the absorption slice). |
| **T7** | **Standing-key compromise**: a high-standing contributor's key is stolen. | Damage confined to the fast tier (revertible); does NOT touch finality — the key's PoM finality weight is a separate, vesting-gated path. | Bounded; thresholds + eventual slashing are the mitigation surface. 🟡 |
| **T8** | **Data withholding (validium freeze)** — *DISSOLVED 2026-07-14 by the re-include choice.* | The ordering block CARRIES its txs (A5), so there is no committed-but-unavailable data and no validium freeze: every block is self-validating/replayable. The only residual is ephemeral + non-safety — a withheld sub-block just doesn't soft-confirm (the tx still settles in the ordering block). | ✅ eliminated at the settlement/history layer; the ~2 s soft-confirm reliability residual is a UX matter (T1/T4), not a fund-safety one. |

## 5. What sub-blocks deliberately do NOT secure

- **No new settlement-safety assumption.** Finality is unchanged: PoS+PoM on ordering blocks, PoW-excluded.
- **No fast-confirmation of contributions.** Value txs only. A contribution's value is vesting-gated (`W`);
  fast-confirming it is meaningless, and excluding contributions keeps the attribution/finality surface
  untouched by the fast tier (role-separation). This is a *security* choice, not only a scope choice.

## 6. Open questions (honest)

- **⚑ Threshold calibration (A2):** the contribution-weight gate value — Sybil-cost vs liveness. Testnet-pinned.
- **🟡 Equivocation slashing (T2):** the attributable-equivocation → PoM-standing-slash path is a design hook,
  not built. Needs the persistent validator/standing registry (the T1 network layer).
- **🟡 The absorption rule (soft → final):** the exact ordering-block absorption + conflict-resolution
  algorithm is the next slice; its DETERMINISM is itself a security property to prove (two honest absorbers
  must resolve an interval's soft-chain identically).
- **🔬 Merchant guidance (T1):** quantify double-spend success probability vs wait-time to justify a
  principled "wait N ordering blocks" for a given value.
- **Honest boundary:** Ergo's Matrix (the structural inspiration) is R&D / never-mainnet with known sync
  edge cases. We adapt the STRUCTURE and must prove our OWN gate and absorption rule — do not inherit its
  security by assumption.

## 7. References

- `node/src/subblock.rs` — the shipped lean core (data model + gate + provisional-overlay conservation +
  `ConfirmationTier`).
- `internal/RESEARCH-NETWORK-CONSENSUS.md` — T9 (Ergo sub-block adoption plan, "adapt don't copy").
- `docs/research/role-separation-as-design-law.md` — the fast-soft / slow-final separation as a design law.
- The finality mix (PoW-excluded) — the reason the fast tier cannot touch settlement safety.
