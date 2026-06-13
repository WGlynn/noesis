# Defensive audit — attacker-choosable inputs (PRIVATE)

> Applies the invariant `[P·dont-let-attacker-choose-critical-input]` (2026-06-13) as a
> sweep across the noesis input surfaces: for each security-critical input, where does it
> come from, and can the tx-assembler pick it to their advantage? Honest status per row;
> CANDIDATEs are "verify against code", not confirmed holes.

## Surface-by-surface

| Surface | Critical input | Source today | Attacker-choosable? | Status |
|---|---|---|---|---|
| Value gate | cell DATA (content) | tx-supplied | yes, BY DESIGN — content is the thing measured | ✅ OK: floors + flow + standing price the content; choosing it gains nothing |
| Index root | novelty-index root | cell-dep 0 | was SHAPE-only ⇒ choosable | ✅ ADDRESSED: identity binding (code_hash+type-id), F1/F2/F3, on-VM ported |
| Mint proofs | witness sibling paths | tx-supplied witness | yes — but VERIFIED vs root (`classify`) | ✅ OK: attacker-supplied-but-verified (forged path ⇒ exit 21). correct pattern (verify, don't trust) |
| Identity | contributor standing | soulbound, earned | no — cannot buy/pool/transfer | ✅ OK ([[value_v6]] standing gate) |
| Dispute verdict | who refutes | 2/3 PoM bloc | only a >2/3 cross-dim cartel | ✅ converged to global assumption (escalation court; not a new hole) |
| Intra-tx dup | claimed-novel set | COMPUTED across outputs | no — derived, never claimed | ✅ OK (double-mint fix) |
| Finalization time | `now` for PoS decay | (port design) | would be choosable if witness-sourced | 🟡 DESIGNED: must be header-sourced (`ON-VM-FINALIZATION.md`) — pin not yet built |
| **Temporal order** | **which block is "earlier"** | **SLICE position; `timestamp` field NOT consulted (verified)** | **only via the on-chain path's slice assembly** | ✅ rule-side / 🟡 integration requirement (below) |

## Temporal-novelty's ordering source — VERIFIED 2026-06-13

Temporal-novelty (the strategyproof core of `v(S)`) values a block by coverage **novel vs
earlier-committed blocks**, so its strategyproofness rests on "earlier" being a relation the
attacker cannot choose. Checked the code:

`temporal_novelty(cells_in_commit_order: &[Cell])` iterates the slice **in the order given**
and never reads `Cell.timestamp`. So the hypothesized "backdate a `timestamp` to rank a
redundant block earlier and flip its overlap into novelty" attack is **moot** — the field is
not the ordering source.

What the critical input actually is: **the slice order itself.** The rule trusts its caller
to supply cells in true commit order. That is correct for a reference model, and it relocates
the invariant to the **on-chain path**: whatever feeds cells to the rule on-chain MUST source
their order from the **commit-reveal block height / consensus ordering**, never from a
producer-arrangeable list or a self-set field. This is an integration property, not a
rule-side bug.

**Action:** when the on-chain temporal path is built, source the commit order from block
height (the commit-reveal block the hash landed in) — the same invariant as the index-dep
identity binding and the finalization `now`-from-header. Pin a fixture there: a tx that
presents cells in a producer-favorable order (vs their true commit-block order) must not let
a redundant block earn novelty. Until then: rule-side is clean (timestamp un-spoofable
because unused); the requirement is carried as a build note, not an open hole in `v(S)`.

## Takeaway
Five surfaces are clean or addressed, one (`now`) is designed-pending-build, and one
(temporal ordering source) is a genuine audit candidate the invariant surfaced. The invariant
earns its keep as a standing audit lens: run it against every new input the design adds.
