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
- **A5 — Sub-block DATA AVAILABILITY holds (new, from the commit-by-root absorption choice).** The ordering
  block commits only a Merkle root over the absorbed txs (`subblock::subblock_txs_root`), NOT the tx bytes —
  a light block. This trades away self-containment: validators + syncing nodes must be able to RETRIEVE the
  sub-block txs (from the gossip layer / an archive) to reconstruct state and to let a user prove their own
  coin exists. **Validity ≠ availability** — neither the root nor a future ZK proof-of-absorption removes
  this duty (a root or proof with withheld data is a *validium*, with the classic frozen-funds risk). DA is
  the gossip layer's job (`[[validity-not-availability]]`).

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
| **T8** | **Data withholding (validium freeze)**: an ordering block commits a root over txs whose data is not made available, so nodes cannot reconstruct state / users cannot prove their coins to spend them. | DA is required (A5): sub-block txs are gossiped + archived; a root committing UNAVAILABLE data must be treated as invalid at absorption (a validator that cannot obtain + verify the committed txs does not accept the ordering block). | The enforcement (reject-on-unavailable + archival/serving rules) is the gossip slice (2c). 🟡 — this is the security cost of the lean commit-by-root choice, entered knowingly. |

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
