# NOESIS MVP SCOPE — JULY 2026

**Status:** synthesis of 4 code-grounded status maps + anti-extraction ledger + 3 scope/sequencing views, 2026-07-03.
**Test baseline:** 324 passed / 0 failed — measured this session via `cargo test --workspace` on the working tree (HEAD afbd4b9 + uncommitted +109-line cand-B guard in `node/src/lib.rs`). Trajectory 318 (ROADMAP yy) → 321 → 322 (REFERENCE-NODE-STATUS 2026-07-02) → 324 now. Do not quote 318 or 322 going forward.
**Status legend (NO ROUND-UP):** ✅ built+tested · 🟡 designed-not-built · 🔬 open research · ⚠ risk/stale · ⚑ Will-gated decision. A 🟡 called ✅ in this doc would poison the launch.

---

## 0. MVP DEFINITION — the honest one

**"MVP" is defined nowhere in-repo** (grep MVP over `*.md` = 0 hits). This document declares it, in two senses that must never be conflated:

> **MVP-as-release (Track A, July-achievable):** the reference node at 324/324 + whitepaper + honest demonstrated-vs-designed framing, opened to collaborators. Per LAUNCH-CHECKLIST.md's own split — Track B build-out items are "NOT launch blockers."
>
> **MVP-as-launchable-network (not July):** a **strategyproof-floor contribution ledger with capital-orthogonal PoS+PoM finality, soulbound franchise, zero fees, and burn-only slashing — presented as the floor, never the moat.**

Grounding for the floor framing: BLOCK-ECONOMY-SPEC.md — "fall back to the floor and it is a strategyproof reputation system, not a gamed one." This definition dissolves the apparent contradiction across the status maps: the learned-v(S) moat (NULL twice on real labels) and HCE-full (M2/C4 open) block the **thesis claim**, not the **launch** — provided launch copy never claims more than the floor. Where status maps marked them `blocks_mvp: true`, that is true only for MVP-as-thesis.

**What can never ship under any MVP definition:** a fee, a transferable franchise, a slash that pays anyone's standing, or launch copy that claims the moat.

### Foundation already in hand (✅, zero work owed)

| Component | Evidence |
|---|---|
| Reference node v0.1, 324/324 green | measured 2026-07-03, `cargo test --workspace` |
| NCI overall mix pow .10/pos .30/pom .60 + 2/3 bar | `node/src/lib.rs:3552`, `:3554` (⚠ ARCHITECTURE.md stamps drifted — see §1.E) |
| FINALITY_MIX — PoW excluded, PoS 1/3 : PoM 2/3 | `node/src/runtime.rs:584-588`; pin `pow_is_excluded_from_finality` runtime.rs:1539 |
| Live routing `finalizes` → `finalizes_pos_pom` | runtime.rs:567→608; anti-theater pin runtime.rs:1491 |
| Anti-concentration floor MIN_DIM_BPS=5000 | runtime.rs:596, `dim_ok` :598-602, enforced :628-632 (buys **capital-orthogonality only** — see circularity, §1.A) |
| PoM↔finality coupling **RESOLVED — COUPLED** | Will ruled 2026-06-29, ARCHITECTURE.md §Consensus, commit 9aa93e3. No longer gated. |
| MIN_STAKE=100 sybil floor | lib.rs:3660 |
| Soulbound PoM franchise (reference layer) | lib.rs:453 `pub mod soulbound`; strategyproof pin lib.rs:711 |
| State-bytes conservation / single-use / mint authority | runtime.rs:466, :1074 |
| Bound A mempool cap | runtime.rs:425, default 10_000 |
| θ_sim near-dup floor **code** on live PoM path | runtime.rs:77 (62259 = 0.95), routed :525; gaming.rs green (default unratified — §1.A) |
| Fixed-point mirror `finalizes_pos_pom_fixed` (Q32.32) | onchain/noesis-core/src/lib.rs:481; drift guard node lib.rs:8578 |
| CKB-VM harness + syscalls + pom-typescript semantic-floor ELF | ckb_vm_* suites: 6 targets green this run |
| PQ Lamport lock-sig + locksig ELF | ckb_vm_locksig.rs 12 pass (deploy-inert by design) |
| nash_honesty — unilateral IC, property-(1) scope marked in-code | lib.rs:9501, 4 tests |
| All extraction invariants preserved-at-reference | EXTRACTION-AUDIT-2026-06-19.md, PASS(12/12) through 2026-06-22 (⚠ log stale — §3) |

---

## 1. IRREDUCIBLE MUST-HAVE SET (blocks a launchable network)

### A. The Will-ratification package — ⚑ one sitting, four consensus-affecting decisions. Cheap in hours, expensive in consequence. **THE TOP BLOCKER.**

The coupling ruling itself is DONE (coupled, 2026-06-29). What remains Will-gated is the **PoM→finality parameter surface** that makes coupled-with-open-moat safe:

1. **⚑🟡 Vesting window W + v(S)-independent PoM-finality input.** POM-FINALITY-TEMPORALITY.md:48-52: "the one load-bearing parameter... Designed, not yet set." This is the fix for the ⚠ CRITICAL floor circularity (VS-AS-COMPLETION-PROCEDURE.md:222-245: the anti-concentration floor "consents over the same quantity v(S)-gaming inflates"; trace runtime.rs:524/630). **W is the moat's stand-in at launch** — fresh standing cannot vote finality until disputes can catch it. Without W, coupled PoM-finality + open moat = the floor is circular on the PoM axis and the second protection does not exist yet. Fix shape: dispute-window discount on fresh standing (VS-AS:265-273). "CRITICAL but consensus-affecting ⇒ build COLD, Will-gated" (VS-AS:326-327).
2. **⚑🟡 Standing.pom → Validator.pom production bridge.** VS-AS:256-263: "designed, not built... No production map→weight bridge exists" — `Validator.pom` is set only in test constructors (lib.rs:3739-3740) + slash path. HOW it is wired decides whether the finality PoM input is a raw v(S) pass-through (baking in the circularity) or W-discounted. **Build with W — they are one surface.**
3. **⚑✅-code/unratified-default: θ_sim_q16 = 0.95 consensus default.** CONTINUE.md:48-51 ⚑ WILL DECISION PENDING: "honest work with >95% coverage overlap would also be floored... Is 0.95 the right consensus default, or hold the floor until the quality model is in-path?" Ratify-or-hold before genesis. The cand-B legitimacy-regression guard (built, **uncommitted**) is the safety net for "ratify."
4. **⚑ Rider while Will has the pen:** MIN_DIM_BPS raise for the safety path — decide or explicitly defer (CONTINUE.md:37, ARCHITECTURE.md:74). One constant if taken.
5. **⚑🟡 I-2 depth-split close: go/no-go + shape** (gates the Week-4 stretch slot only; hard gate is pre-v8-franchise, not July — see §2).

### B. On-VM enforcement parity (🟡 designed, decision-unblocked, pure build)

6. **🟡⚠ Finalization ELF twin-update to `finalizes_pos_pom_fixed` — the one live landmine.** `onchain/finalization-typescript/src/main.rs:36,110` still calls `finalizes_fixed` (the OLD PoW-inclusive hybrid rule) — the ELF enforces a **different finality rule than the live node**. Pre-deploy nothing breaks; a deploy with this ELF ships the wrong rule. DECIDED, build fresh, "NOT a 1-line swap" (ROADMAP (tt): `now_is_header_sourced` redesign via differential validator decay + anti-concentration fixture; forward-parity constraint runtime.rs:564-566).
7. **🟡 Validator-registry + params consensus-binding.** `REGISTRY_BINDING_ACTIVE = false` (finalization main.rs:48); curated-set-rejected fixture + registry-bound threshold/floor/horizon/mix (ON-VM-FINALIZATION.md:58-63, 96-100). Header-`now` sourcing itself is ✅ (10 tests green).
8. **🟡 Lock-sig GO-LIVE flip.** `CONTROL_BINDING_ACTIVE = false` (runtime.rs:335) + `CONTROL_ENFORCED = false` (locksig main.rs:53) + populate auths + real-entropy keygen. **Until flipped, spend authorization is not enforced anywhere.** Deploy step — flipping early "breaks every empty-auth test until flows carry sigs" (CONTINUE.md).
9. **🟡 On-VM soulbound-transition validation + cross-cell similarity-floor syscall surface.** pom-typescript self-disclaims: "the similarity floor still needs cross-cell state... the next piece, not claimed here." On a live network, "you cannot buy consensus weight" must be VM-rejected, not reference-node politeness.
10. **🟡 Deploy-coupled double-spend crypto.** Cryptographic nullifier set + on-VM UTXO retirement (SECURITY.md §3 names this the deploy layer; reference layer ✅).

### C. Liveness + extraction hygiene

11. **🟡 Bound B commit-deposit.** Zero-fee + Bound A alone = an attacker keeps the 10k mempool full for free forever (junk earns 0 novelty but occupies admission). SECURITY.md: "a serious reviewer should still press on the deposit leg." **Condition: refund-on-contribution / forfeit-BURNS-only, passed through the 12-item extraction rubric BEFORE merge** — forfeiture = value movement, the named canonical fee-reintroduction surface (audit rec #4).
12. **⚠ Pin the invariants launch copy will claim** (cheap; AA#2 claim-needs-structural-enforcer):
    - (a) explicit "slash burns, never transfers standing" test, incl. fencing the β-bounty from ever crediting standing (audit rec #3 — unbuilt; the single place a careless future wiring could convert dispute into an extraction market);
    - (b) a zero-fee pin (today true-by-absence, grep-clean, unpinned);
    - (c) a gate that **value_v8 never drives the franchise until I-2 closes** — flip the lib.rs:2940 pin to g≤0 + 12-item pass first (deployed `pom_scores` path verified split-immune, lib.rs:2978).

### D. Network substrate

13. **🔬 Genesis / chain-spec / P2P.** LAUNCH-CHECKLIST.md:49 (today: reference runtime + tested 2-node convergence, **not a network**); SECURITY.md:5 "no public network and no funds at risk." LAUNCH-CHECKLIST calls deploy-crypto+genesis "bounded, known engineering" — the literal launch substrate, and the long pole after the cold builds.

### E. Honesty hygiene (trivial cost, load-bearing for "honest" — collaborators read the doc chain)

14. **⚠ Doc coherence:**
    - POM-FINALITY-TEMPORALITY.md:76-86 is **STALE vs the 2026-06-29 ruling** — still says "the audit recommends decoupling." Contradicts ARCHITECTURE.md §Consensus + commit 9aa93e3. Anyone reading the chain gets the wrong current state.
    - ARCHITECTURE.md file:line stamps drifted: NCI lib.rs:3289→**3552**, TWO_THIRDS_BPS 3291→**3554**, MIN_STAKE 3397→**3660**; REFERENCE-NODE-STATUS pos_pom drift test 8366→**8578**. The anti-hallucination canon points at wrong lines. Re-stamp at next commit (working tree has uncommitted lib.rs edits, so stamp AFTER committing).
    - Extraction-audit log stale since 2026-06-22 (checklist item 11 technically failing — see §3).
    - Commit the uncommitted cand-B guard.
    - NCI doc↔code reconciliation (TOKENOMICS open item) if launch copy uses the capital/compute/cognition framing; confirm token naming (JUL/VIBE vs NOE) before any public copy.
    - rust-toolchain.toml pin + one-command `make verify` (an honest release invites reproduction; both named REFERENCE-NODE-STATUS "gap to impeccable," ~hours).

---

## 2. DEFERRABLE — launches later, each with the guard that makes deferral honest

| Deferred | Status | Guard condition |
|---|---|---|
| **JUL PoW money layer** | 🟡 designed, zero code (grep hits = NATO-phonetic test strings only) | Deferred **by design**: TOKENOMICS.md — the minimal core "needs no Proof of Work to secure consensus or mint state." Will decisions LOCKED 2026-06-20 ("energy circulates, does not vote"; Ergon constants kept) — build is decision-unblocked, just not July. |
| **VIBE governance token** | 🟡 designed, no code | Orthogonal to the capture-resistant cycle; only naming-confirm outstanding. |
| **Learned v(S) moat** | 🔬 open — NULL twice (round 1 + faithful port: 95/115 ancestor coalitions singleton ⇒ "untestable here for lack of ANCESTRY, not labels"); adversarial instrument ✅ 2026-07-02 (constructed fixture, not real-outcome data) | Launch copy = **floor only**, never "un-gameable value chain." W substitutes temporally for the moat on the finality path. Data hunt (deep-ancestry outcome-labelled dataset) runs all month in background — calendar-bound, not effort-bound. |
| **HCE full three properties (M2/C4)** | 🔬 open conjecture; Will's OFFICIAL TOP PRIORITY ("you cannot have noesis if you dont have wills equilibrium on the contribution problem") | Blocks the **thesis**, not the floor launch. Claim IC for demonstrated vectors only — the property-(1) scope marker is already in-code at nash_honesty. Research runs parallel, unscheduled. |
| **value_v5..v8 driving the franchise + I-2 depth-split close** | v8 ✅ built as moat-TARGET path; I-2 🟡 ⚑ build COLD | Deployed franchise = flow-free `pom_scores`, verified split-immune (lib.rs:2978, regression-guarded). Hard gate pinned per §1.C.12(c): +16.7 pin at lib.rs:2940 flips to g≤0 + 12-item pass BEFORE v8 ever drives the franchise. |
| **General isomorphism-invariance gate for v(S)** | 🔬 graph-iso-hard | I-1 probe (✅, d689ef9) + cand-B legit-guard pin the known grains; §6.1 loop invariant + §6.2 probe-diversity remain open. |
| **A3 paraphrase/reshingle on the live path** | 🔬 open | **Disclosure item, not silence**: the named moat gap ("byte-proxy ≠ semantic"), mitigated at launch by θ floor + W + dispute layer. Rosetta canonicalization lands in the learned-v(S) layer, off-chain. |
| **zk-finalize (RISC Zero PoC)** | 🟡 PoC — parity harness + guest exist, NOT in the 324 suite (not counted built) | Optional hardening. |
| **Nucleolus/iterated-LP over the real PoM-weighted Myerson game; temporal Shapley fixed-point** | 🟡/🔬 | Value-layer research, not consensus. |

---

## 3. ANTI-EXTRACTION LEDGER — per invariant, with the moat-change guardrail

**THE GUARDRAIL (constitutional for this scope doc):** *no moat change may open an extraction surface.* Operationally: every consensus-affecting change (anything touching finality franchise, standing→weight, or params) builds **COLD** (fresh low-context, one at a time, RED-first), passes the **12-item extraction checklist BEFORE merge** (EXTRACTION-AUDIT-CHECKLIST.md), and is Will-reviewed. Any future fee-shaped proposal — including Bound B forfeiture — is the canonical MEV-precondition reintroduction and audits against the rubric before merge (audit rec #4). value_v8 (or any flow-valued v(S)) never drives the franchise until its pinned gaming pin flips to g≤0 under the same rubric.

| Invariant | Verdict | Evidence (file:line) | Gap / action |
|---|---|---|---|
| **P-001 No-Extraction-Ever** | **PRESERVED** (reference level) | EXTRACTION-AUDIT-2026-06-19.md:5-12 "NO live extraction vector... GEV-aligned by construction"; 8/8 mechanisms aligned; PASS(12/12) 2026-06-21 + 06-22 (EXTRACTION-AUDIT-LOG.md:1-2). Scope honest: SECURITY.md:4 — pre-launch, no funds at risk. | ⚠ **Log stale since 06-22**: I-1 depth-split (+16.7) and A1-A4 vectors documented (ISOMORPHISM-INVARIANCE-VS.md, CONTINUE.md) but NOT in the log or ROADMAP's adversarial register — checklist item 11 technically failing. Run the tick, register residuals (Week 0). 🔬 P-001's machine side (protocol-autonomous self-correction) rides M2/C4 — open. |
| **GEV-resistance (preconditions absent, not patched)** | **PRESERVED** | Four channels structurally ABSENT: capital-formation (bytes earned not sold), oracle (outcome factor ∈[0,1] can only LOWER, lib.rs:1209-1217), platform (grep-clean of fee/tip/gas/block_reward/seigniorage, re-verified 2026-07-03), liquidation (no leverage primitive). Ordering consensus-sourced XOR-seeded. Soulbound kills token-rent/simony (TOKENOMICS.md:113-119). | Residual register: V1 common-atom front-run (Low-Med, closes with learned-v(S)) + V2 slash-clamp coarseness (Low, grief-not-payout, DECIDED) + NEW post-audit I-1/A1/A3/A4 — named and pinned, not yet in the GEV register. Register hygiene (Week 0); architecture intact. |
| **Zero-fee principle** | **PRESERVED — ⚠ unpinned** | Strongest form: fees absent by construction, grep returns nothing (audit row 3). Decay = supply SINK paying no one (TOKENOMICS.md:76-78). Slash accounting: burned ≥ canceled, β clamped [0,1] (lib.rs:4327-4361). | ⚠ True-by-absence, not by-assertion: no test pins no-fee-ever (audit rec #3 partially discharged). Pin it (Week 0). Watch surface: Bound B forfeiture (🟡) must pass the rubric before merge. |
| **HonestyStructural (dishonesty unprofitable)** | **AT-RISK** (honestly split) | Built half: dispute-layer negative-EV swept (attacker_ev<0 ∧ griefer_ev<0 ∧ challenger_ev>0, lib.rs:5990-6008); strategyproof floor (lib.rs:711); paraphrase ring closed on live path (ROADMAP (xx)). Counter-evidence: dishonesty IS profitable on the moat-target path — depth-split pumps v8 +16.7 (lib.rs:2940, RED-as-designed); A2 killed the economic close (standing floor is self-funded, "real attacker outlay ≈ 0"). | Preserved on the DEPLOYED franchise (flow-free `pom_scores`, split-immune, guard lib.rs:2978); violated on v8. Actions: (1) I-2 structural close (⚑ COLD); (2) do NOT rely on the economic close; (3) M2/C4 = the only claim both unproven AND load-bearing for the full property. **Never claim "rational attacks don't exist as a class" until I-2 + the moat land** — demonstrated vectors only. |
| **Shapley 5-axiom Σφ=v(N) (no MLM, null-player)** | **AT-RISK on v8 path / PRESERVED on deployed path** | Myerson over provenance DAG (lib.rs:3116-3126); nucleolus constrained to the efficiency hyperplane (lib.rs:4064-4067); restitution ≤ harm (lib.rs:4298-4306); no compounding channel (provenance-forgery earns negligible credit, lib.rs:7580-7591); checklist item 1 = standing FAIL condition. | (1) Depth-split violates efficiency/symmetry ON v8 (+16.7 with v(N) fixed); deployed path bit-exact identity-invariant (lib.rs:2918, :2978). (2) 🔬 temporal iterated-Shapley fixed-point open; LP solver over real PoM-weighted game 🟡. **Do not claim "5-axiom verified" for the v8/franchise path until I-2 closes.** |
| **FLAG — depth-split self-flow-laundering (v8)** | **AT-RISK, pinned, fenced** | sybil_split lib.rs:2902-2916; pin lib.rs:2940 (g≈+16.7, RED-as-designed); deployed-path immunity guard lib.rs:2978 (goes RED if split-escapable damping ever added to the live path). All breadth dampers + ring detectors structurally miss it (one child/parent ⇒ μ⁰=1; not a ring). | Close = I-2 (subtract relabel-variant flow energy at scoring time; ⚑ COLD, VS-AS:141-147). A4 (depth×breadth) rides the same close. Hard gate per guardrail: pin→g≤0 + 12-item pass BEFORE v8 drives the franchise. Register in audit log (Week 0). |
| **FLAG — moat/finality coupling as extraction-reintroduction channel** | **AT-RISK by design, fenced by W** | Chain: gamed v(S) → pumped soulbound standing → pumped finality weight → consensus capture (governance-shaped, not fee-shaped — no position-rent exists, grep-clean). Coupling ruled COUPLED 2026-06-29; "the two real protections = anti-concentration floor + un-gameability moat — nothing else" (ARCHITECTURE.md §Consensus). But the floor is CIRCULAR against v(S)-gaming on the PoM axis (VS-AS:222-245) and the moat is 🔬 open ⇒ **until W lands, the PoM axis leans on an open protection.** | This conjunction is the sharpest reason a live-network July MVP would be premature, and exactly what §1.A exists to break. Watch: (a) I-2 before v8-franchise; (b) MIN_DIM_BPS knob open; (c) moat NULL-twice ("unsupported, not refuted") — interim fence: v(S) is "a gameability surface only — not finality, never the safety path" (VS-AS:6) + outcome factor can only LOWER (lib.rs:1209-1217). |
| **FLAG — dispute β-bounty (the one designed transfer-from-slash)** | **PRESERVED — owed a pin** | challenger_payout = bond + β·total_slashed (lib.rs:4358), β clamped (lib.rs:4354), burned ≥ canceled always. Levied only on proven-dishonest standing; purchases detection p≥1/2 (DISPUTE-SLASHING.md:40,82,200) — deterrence spend, not rent. Self-challenge farming negative-EV (lib.rs:5349-5362); no path credits a challenger's STANDING (true by absence). | ⚠ Audit rec #3 unbuilt: pin "slash burns, never transfers standing" + fence β-bounty from ever crediting standing; reconcile checklist-item-3 wording (bounty = value-denominated, never a standing credit). One cheap assertion test (Week 0). The single spot where careless future wiring could convert dispute into an extraction market. |

---

## 4. CRITICAL PATH

**#1 blocker: the ⚑ Will-gated PoM↔finality decision package (§1.A).** Honest framing: the coupling ruling itself is RESOLVED (coupled, Will 2026-06-29 — do not reopen); what blocks everything downstream is the remaining Will-gated surface ON that coupling: **W (vesting window) + the v(S)-independent PoM-finality input + the bridge wiring shape + θ_sim ratification**. All four are decisions-not-builds, one sitting, and every consensus-affecting build queues behind them by explicit repo discipline ("consensus-affecting ⇒ build COLD, Will-gated"). Deciding them late forces a rebuild of the bridge (§1.A.2 must be born W-discounted, never raw pass-through).

```
A (⚑ Will sitting: W + circularity-fix shape + bridge wiring + θ_sim ± MIN_DIM_BPS, I-2 go/no-go)
  → B6 finalization ELF twin-update          [decision-unblocked — starts BEFORE A lands]
  → B/C cold builds in order: circularity fix (W) → Standing.pom→Validator.pom bridge
  → C12 invariant pins + E14 doc tick        [days, warm]
  → C11 Bound B (rubric-gated)
  → D13 genesis / chain-spec / P2P           [the long engineering pole]
  → launch as the FLOOR (copy never claims the moat)
Parallel, unscheduled: moat data hunt + M2/C4 research  [Tier-0, cannot be dated]
```

**Load-bearing reframe:** the moat is not on the launch path — **W is the moat's stand-in at launch**. Everything in the must-have set except §1.A and D13 is bounded, decided engineering. The two unschedulable risks (moat, HCE) are demoted to honestly-labeled open items by the demonstrated-vs-designed release framing — which is also the collaboration mechanism ("come join us" wants you public before it's finished).

---

## 5. JULY BUILD ORDER — week-by-week

**Track discipline (hard rule, VS-AS:326-327 / ISOMORPHISM-INVARIANCE-VS §7):**
- **COLD** = consensus-affecting (finality franchise, standing→weight, params): fresh low-context session, ONE cold build landed per week max (design→RED→green→extraction-audit→Will-review is a full week at single-dev throughput; never parallelize cold builds).
- **WARM** = scoring-side, docs, pins, instruments, data hunt: 2-3 threads alongside.

### WEEK 0 (Jul 3-5) — hygiene + decision packet. All WARM.
1. Ship Will the ⚑ D-packet (one page: W, circularity-fix shape, bridge wiring, θ_sim ratify-or-hold, MIN_DIM_BPS, I-2 go/no-go — evidence pointers from §1.A). **Due back before Week 2 cold work.**
2. Commit the uncommitted cand-B guard; re-stamp ARCHITECTURE.md drift (3289→3552, 3291→3554, 3397→3660; REFERENCE-NODE-STATUS 8366→8578) at that commit.
3. Fix POM-FINALITY-TEMPORALITY.md:76-86 staleness (align to the coupled ruling).
4. Extraction-audit tick: append I-1 (+16.7) + A1-A4 to EXTRACTION-AUDIT-LOG.md + ROADMAP adversarial register (checklist item 11 currently failing).
5. Pin audit rec #3: "slash burns, never transfers standing" test + β-bounty standing-fence + zero-fee grep-pin; reconcile checklist-item-3 wording. Non-consensus test work, cheap, closes the two true-by-absence gaps.

### WEEK 1 (Jul 6-12) — COLD #1: finalization ELF twin-update. Decision-unblocked, starts immediately.
- **Cold:** twin `onchain/finalization-typescript` to `finalizes_pos_pom_fixed` — kills the wrong-rule-at-deploy landmine (main.rs:36/110). NOT a 1-line swap: redesign `now_is_header_sourced` (differential validator decay), re-derive tests, ADD anti-concentration fixture (ROADMAP (tt); parity constraint runtime.rs:564-566). No ⚑ gate — which is exactly why it runs while the D-packet is out.
- **Warm:** moat data hunt STARTS (deep-ancestry outcome-labelled dataset — calendar-bound, runs all month). TOKENOMICS NCI doc reconciliation. Week-0 spillover.

### WEEK 2 (Jul 13-19) — COLD #2: circularity fix / vesting window W. **Needs D-packet by end of Week 1.**
- **Cold:** dispute-window discount on fresh standing (VS-AS:265-273). RED-first: inflated fresh v(S) does NOT move finality weight inside the window. Fold in MIN_DIM_BPS if Will takes it. Extraction checklist before merge.
- **Warm:** design (not build) the on-VM cross-cell syscall surface + soulbound-transition validation — Week 4's build contract.

### WEEK 3 (Jul 20-26) — COLD #3: Standing.pom → Validator.pom production bridge.
- **Cold:** the franchise wiring — must consume Week 2's discounted/vested standing, never raw v(S) pass-through (that ordering is why this cannot precede Week 2). RED-first: raw-pass-through-rejected fixture. The week the finality PoM input becomes real instead of test-constructed.
- **Warm:** registry-binding fixture scaffolding (curated-set-rejected, armed-at-flip; REGISTRY_BINDING_ACTIVE stays false). Moat experiment if the data hunt landed anything.

### WEEK 4 (Jul 27-31) — on-VM enforcement seams + month-end verify.
- **Build:** on-VM cross-cell similarity floor syscalls + soulbound-transition validation on-VM (fresh-context, not ⚑-gated).
- **Stretch (only if I-2=go AND Weeks 1-3 landed):** I-2 depth-split close — flip lib.rs:2940 to g≤0; A4 rides the same close. Not July-blocking (deployed path immune); hard gate is pre-v8-franchise.
- **Verify:** full `cargo test --workspace` measured (not quoted); re-stamp all doc file:line; extraction-audit PASS tick with current residual register; add `make verify` + rust-toolchain.toml pin.
- **Bound B:** extraction-rubric **design review only** — build punts to August unless the month ran clean.

### Throughput honesty + slip policy
Queue = 4 cold-class builds + 1 on-VM build in 4 weeks = **zero slack** at the honest rate of one cold build/week. If a week slips: **drop from the tail, never reorder the safety spine** — drop order: Bound B build → I-2 stretch → on-VM similarity floor → registry fixtures. Weeks 1-3 (wrong-rule ELF, circularity/W, bridge) are the non-negotiable core: the three genuinely-buildable-in-July blockers. **Will's D-packet latency is the single biggest schedule risk** — if it slips past Jul 12, Weeks 2-3 shift right one week and Week 4 becomes cold-build overflow.

### July does NOT deliver (no round-up, ever)
- 🔬 Learned-v(S) moat on real labels (data-blocked; the hunt runs, the result is not scheduled).
- 🔬 HCE M2/C4 (open research; Will's own "solution-defining" bar — blocks the thesis, not the floor).
- 🔬 Genesis / chain-spec / P2P (Track B; a July "launch" claim would be false).
- 🟡 Lock-sig GO-LIVE flip (deploy step — do at deploy, not before).
- 🟡 JUL / VIBE builds (decision-unblocked, deferred by design — the core needs no PoW to secure consensus or mint state).

---

*Do not push this file to any public remote. Noesis is PRIVATE (leak-gate enforced).*
