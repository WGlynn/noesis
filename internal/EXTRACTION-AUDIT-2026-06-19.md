# Noesis Extraction Audit — 2026-06-19

> Self-assessment vs **"Extraction is Conserved: From MEV to GEV"** (schelling @ ethresear.ch,
> https://ethresear.ch/t/extraction-is-conserved-from-mev-to-gev/24953 — Will's airgap series, Part 3).
> Directive: P-001 No Extraction Ever. PRIVATE — never sync public.

## Verdict
**NO live extraction vector.** Noesis is GEV-aligned by construction. The classic MEV preconditions
(fees, block rewards, producer-chosen ordering, transferable franchise, slash payouts, oracles,
liquidations) are **structurally absent**, not patched. Two known, already-logged residuals (V1, V2)
remain, both bounded with precondition-removing fixes scheduled.

## Paper's yardstick (verified, citation-level)
- **GEV** = total value received by participants **in excess of their Shapley value**, across ALL
  channels. MEV (ordering extraction) is just one channel.
- **Conservation:** extraction is conserved over **preconditions** (privileged position / ordering
  authority / information asymmetry), NOT over surfaces. Closing a surface without removing its
  precondition relocates extraction; removing the precondition kills all dependent surfaces at once
  (= [[class-dissolution-vs-case-defeat]]).
- **Distinguishing test:** "Does the intervention remove the precondition, or only hide symptoms?"
- **7 channels:** ordering(MEV) · governance · token-rent · capital-formation · oracle · platform · liquidation.

## Per-mechanism table (received value ≤ Myerson/Shapley contribution?)
| # | Mechanism | Verdict | Evidence |
|---|---|---|---|
| 1 | Value dist v5–v8 / `value_flow_with_own` | **Aligned** | value = floored_novelty × g(downstream_flow) along provenance DAG = Myerson value; nobody-builds-on-it ⇒ 0 (lib.rs ~2484) |
| 2 | Block/commit ordering | **Aligned (V1 residual)** | order = XOR-seeded slot/height, consensus-sourced ¬ producer-arrangeable; `is_canonical_order` rejects reorder before root math (lib.rs ~7381) |
| 3 | Position rent (validators/certifiers) | **Aligned (by absence)** | NO block reward / tip / gas / fee — grep returns nothing; certifiers earn only own Myerson share |
| 4 | Token issuance / seigniorage | **Aligned** | pure conservation + issuer-only mint; bytes PoM-minted by earned contribution; "rent" = decay = supply SINK, pays no one |
| 5 | Slash / dispute economics | **Aligned** | slash BURNS, never transfers ⇒ no payout target; bounty β·(λV+α), β≤1, from deterrence margin ¬ honest standing |
| 6 | Appeal grief (gov-extraction analog) | **Aligned (V2 refinement)** | guard flag derived by counterfactual on defendant's own standing ¬ producer-asserted; total_slash(guarded) ≤ pre_appeal |
| 7 | Sybil / identity | **Aligned** | seeds count only soulbound earned standing ≥ floor; non-transferable (no simony); all-fresh ring ⇒ 0 |
| 8 | Volume gaming | **Aligned** | single joint ρ^j tail (ρ=1/φ): stack/split/diagonal share one budget Σρ^j ≤ 2.618 (the (u) fix) |

**Channels structurally absent** (strongest GEV form): capital-formation (bytes earned, not sold) · oracle (v(S) role-bounded, can only LOWER a seed) · platform (no fee on activity) · liquidation (no leverage primitive).

## Residuals (known, logged — not new)
- **V1 — public/common-atom novelty front-run** (channel 1). Front-run common boilerplate banks novelty before honest reveals. DEFEATED in-proxy by rarity/value-weighted novelty (attacker 16.5 vs honest 560); full closure rides the learned-v(S)-on-real-labels mile (removes the precondition: common atoms ≈ 0 to a value-by-outcome v(S)). Severity Low-Med, bounded.
- **V2 — per-certifier slash-clamp coarseness** (channel 2 analog). Mixed panel (honest + garbage certifier) ⇒ whole-settlement clamp is all-or-nothing. GRIEF surface, NOT payout (slash still burns). DECIDED (per-share gate via `certifier_keys` join), build deferred. Severity Low.

## Recommendations
1. V1 ⇒ ship learned-v(S)-on-real-labels (the precondition-removal; confirmed correct GEV move ¬ patch).
2. V2 ⇒ build per-certifier asymmetric clamp (DECIDED). Hold YAGNI: no `RECUSED_DIM` abstraction until a 2nd court exists.
3. **Make "slash burns, never transfers" an EXPLICIT invariant test** (today true by absence ⇒ make true by assertion). The moment a slash pays a refuter, dispute becomes extractive.
4. **Keep "no fee, ever" structural.** Any future fee proposal = canonical MEV-precondition reintroduction ⇒ audit against this rubric BEFORE merge.

Full checklist ⇒ `internal/EXTRACTION-AUDIT-CHECKLIST.md` (cron-runnable). Source: ethresear.ch/t/...24953.
