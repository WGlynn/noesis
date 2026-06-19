# Noesis Extraction Self-Assessment Checklist (cron-runnable)

> Standing GEV/extraction rubric. Run by the `noesis-extraction-audit` cron each tick.
> Test for every check = the paper's: **received value ≤ Shapley/Myerson contribution; no precondition
> for position-rent introduced.** A tick is GEV-clean iff ALL pass. PRIVATE — findings never go public.
> Source: "Extraction is Conserved: From MEV to GEV", ethresear.ch/t/...24953. Baseline audit:
> `EXTRACTION-AUDIT-2026-06-19.md` (verdict: clean, residuals V1/V2 bounded).

## Fast checks (1–10): grep / test-presence, deterministic, Haiku-tier
1. **[Shapley invariant]** every value-paying path (`value_v5..v8`, `value_flow_with_own`) distributes strictly along the provenance DAG by Myerson share. FAIL if a cell can earn without realized external flow (`downstream_flow_external`).
2. **[No fee / no reward]** `grep -i 'fee|reward|tip|gas|proposer|seigniorage|block_reward'` over `node/` + `onchain/` returns no PAYOUT path. FAIL on any new validator/certifier/proposer payout.
3. **[Slash burns, never transfers]** every `slash`/`Op::Slash`/`resolve_refuted*` path reduces standing and credits NO counterparty. FAIL if a refuter/juror balance increases from a slash.
4. **[Ordering consensus-sourced]** all order-determining inputs (commit slot, height, `now`, validator set) are header/consensus-derived, never producer-asserted; `valid_ordered_root_transition` rejects non-canonical order before root math. FAIL on any producer-choosable ordering input.
5. **[Soulbound standing]** `valid_transition` rejects contributor reassignment (no simony); `value_v6+` seeds gated on earned standing ≥ floor. FAIL if standing becomes transferable/poolable/purchasable.
6. **[Sybil bound]** `max_certifying_identities = total_standing/floor` holds; joint ρ^j damping intact (single tail). FAIL if split/stack/diagonal can exceed Σρ^j ≈ 2.618.
7. **[Token conservation]** `token_txs_conserve_and_single_use` + issuer-only mint; `mint_authority_cannot_be_self_declared` green. FAIL on any non-issuer issuance or non-conserving tx.
8. **[v(S) can only lower]** learned outcome factor ∈ [0,1] multiplied into seeds — corrupt model reduces v8→v7, never mints above. FAIL if v(S) can raise a seed.
9. **[Appeal cannot grief]** `total_slash(guarded) ≤ total_slash(pre_appeal)`; guard flag derived by counterfactual, not asserted. FAIL on any producer-set guard channel.
10. **[Decay is a sink]** PoM/state-byte decay reclaims capacity, pays no one. FAIL if decay credits any party.

## Reasoning checks (11–12): only when diff touches `value`/`dispute`/`flow`/`runtime`/token OR adds a payout/ordering surface
11. **[Residual register current]** V1 (common-atom front-run) + V2 (per-certifier clamp) still tracked with status; new residuals added in ROADMAP adversarial format (named → DECIDED → DEMONSTRATED → CLOSED). FAIL if a known residual silently dropped.
12. **[New-surface gate]** any mechanism added this tick was tested against all 7 paper channels (ordering / governance / token-rent / capital-formation / oracle / platform / liquidation). FAIL if a new surface ships without a channel-by-channel note.

## Tick procedure
- run 1–10 always (cheap). Run 11–12 iff the touched-paths condition holds.
- append result to `EXTRACTION-AUDIT-LOG.md`: `audit | YYYY-MM-DD | PASS(N/12) | <any FAIL detail>`.
- commit + push private. **Ping Will ONLY on a FAIL** (a found extraction vector = the one thing worth interrupting for). All-pass ⇒ silent log line.
- a FAIL = name it in the ROADMAP adversarial register (named → … → CLOSED), same discipline as a gaming vector.
