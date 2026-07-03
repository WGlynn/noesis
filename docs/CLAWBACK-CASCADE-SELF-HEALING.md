# Clawback cascade / self-healing — design synthesis (pre-implementation)

> Status: 🔬 DESIGN ONLY (2026-07-03). Nothing here is code; no code was modified.
> Clawback = value movement = consensus-adjacent ⇒ **COLD, Will-gated to build** (§4).
> Synthesized from 4 independent design candidates × 8 adversarial gate verdicts, every
> load-bearing claim re-verified against source at the line numbers below (lib.rs drifts;
> anchors re-pinned this session).
>
> Frame (Will, 2026-07-03): clawback cascades are the RETROACTIVE enforcement half —
> complement to the pre-emptive v(S) gates, which have a structural ceiling. Detect
> extraction post-hoc ⇒ claw back ⇒ cascade transitively to every position that depended
> on the clawed value ⇒ heal. "Semi-self-aware" = the network holds a model of its healthy
> state (honest, relabel-invariant value flow), detects deviation, and acts on the
> measurement — the cand-A invariance-probe residual is the awareness sensor.
>
> Parent docs: DISPUTE-SLASHING.md (point-slash, ✅ built), VS-AS-COMPLETION-PROCEDURE.md
> (completion loop), ISOMORPHISM-INVARIANCE-VS.md (cand-A / I-2),
> cybernetics-economic-layer.tex (recovery cascade + bona-fide protection, prose).

---

## 1. Honest current state: Noesis POINT-SLASHES; nothing cascades

**Slashing exists ✅. Transitive cascade: NO.** Verified surfaces, all point-scoped:

1. **`soulbound::Op::Slash`** (`node/src/lib.rs:475`, applied `:489` as
   `st.pom.saturating_sub(d)`) — decrements ONE identity's soulbound `Standing.pom`.
   Identity-preserving successor, no graph effect. Under an open dispute the decrease is
   clamped to the settlement-authorized amount (`valid_transition_under_dispute`
   `lib.rs:528-553`, clamp `:549`).
2. **Refutation settlement** (`dispute::resolve_refuted` `lib.rs:4400-4432`): cancellation
   filter is `e.cell_id == c.target && !is_vested(e, c.opened_epoch, p)` (`:4408`) —
   ONLY the challenged target's unvested `VestingEntry`s cancel, snapshotted at
   challenge-open (hardening doc `:4392-4395`). Certifiers are point-slashed
   `λ×bounded_share+α` (`:4413-4423`), zero-share certifiers skipped (`:4417`), Σ shares
   ≤ canceled via `bounded_shares` (`:4369-4376`), β-bounty to the challenger, remainder
   burned (`:4424-4431`). Per-certifier appeal guard: `resolve_refuted_guarded`
   (`:4454-4507`). No descendant/ancestor iteration exists anywhere on this path.
3. **`collusion_slash`** (`lib.rs:4583-4604`): ring standing burned per Hodge residual
   share (`collusion_residual_by_identity` `:379+`; honest acyclic identities attribute 0,
   doc `:373-374`), Σ ≤ manufactured_value, burn-only (no bounty, rationale `:4570-4572`).
   Wiring into the settlement path is the deferred deploy-coupled (dd) step (`:376-378`).
4. **Juror accountability** (`juror_slash_on_overturn` `lib.rs:4805-4811`) — point-scoped.
5. **Cross-path composition**: `unified_slash` (`lib.rs:4665-4694`) — lineage-overlapping
   harm counted once (max, `:4686`), per-identity total capped at current standing
   (`:4687-4688`); `unified_settlement` (`:4711-4726`) emits the corrected burn.

**The asymmetry:** accrual IS transitive — "certification is TRANSITIVE through unvested
intermediaries" (`lib.rs:1066-1068`; flow engine `value_flow_with_own` `:3402-3468`,
`downstream_flow_external` `:3482-3485`) — and liability attribution reaches transitively
UPSTREAM via the zero-seed counterfactual (`causal_share` `lib.rs:4349-4365`: re-run
`value_v6` with one certifier's standing removed, diff at ONE index). There is **no
slash-side mirror**: children's vesting entries, third-party accruals, and standing
accrued from since-refuted flow are never touched. Every dependent position requires an
independent bonded challenge (`challenge_admissible` `:4310-4312`).

**Vested = untouchable** by construction (`lib.rs:4300-4301`; DISPUTE-SLASHING.md:44-45 —
"the price of finality"). Only downstream effect of a slash is PROSPECTIVE: standing below
`standing_floor` stops seeding future `value_v6` gates (`lib.rs:1090-1096`).

**Runtime is not even wired:** `runtime.rs` contains ZERO references to
dispute/Settlement/Slash/VestingEntry/tombstone (grep-verified). `Ledger.pom` is
recomputed as a **pure fold over all cells every `Node::apply`**
(`runtime.rs:524-525` → `pom_scores_with_similarity_floor_q16` `lib.rs:182-193`) — a
slash applied to the replicated pom map would be **erased by the next block's fold**.
Slashing today lives entirely in the reference-spec modules, bound "by caller convention"
(DISPUTE-SLASHING.md:134-136).

**What is clawed today** (all from named earned identities, none from dependents): the
refuted target's still-unvested value; λ×share+α of each culpable certifier's soulbound
standing; collusion-ring standing (burned); overturned jurors' vote-weight. Never:
capacity/token cells of third parties, vested value, or any dependent position.

---

## 2. The gap: what's missing for self-healing

Six pieces, cross-confirmed by two independent current-state reads:

- **G1 — No value-dependency ledger.** `VestingEntry = {cell_id, amount, realized_epoch}`
  (`lib.rs:4274-4278`) carries no back-link to the flows/seeds that justified it;
  `Op::Accrue` is fire-and-forget `saturating_add` (`lib.rs:487`). There is no index of
  "positions whose value derived from X" to unwind.
- **G2 — No tombstone.** A refuted cell stays in `cells` and keeps participating in future
  scoring and flow. No refuted-set exists in `Ledger` (grep-verified).
- **G3 — No settlement→runtime bridge.** Zero dispute wiring in `Node::apply`; the pure
  fold would erase any naive map-mutation slash (§1).
- **G4 — Hard healing horizon + vest-race.** W bounds healing depth
  (DISPUTE-SLASHING.md:106-107 "W is the price of clawback-ability"); a depth-k laundering
  chain lets each hop vest before a root refutation reaches it; exit-blocking is one-hop
  only (`standing_exit_blocked` `lib.rs:4530-4540`, `child.parent == Some(c.target)`).
- **G5 — Bona-fide stop rule unbuilt.** The holder-in-due-course / gradient-vs-circulation
  walk exists only as prose (cybernetics-economic-layer.tex:245-251; status "not yet
  built" tex:303-304) — and §3.5-B below finds it UNSOUND at the value layer anyway.
- **G6 — No cascade anti-extraction bounds; detection unwired.** The point-slash
  invariants (Σ≤harm, per-identity cap, zero-share-spared, no-mint clamps) have no cascade
  equivalents; the cand-A probe is explicitly "a test harness, NOT a consensus gate"
  (`lib.rs:2861-2862`) and never opens disputes.

**Load-bearing deployment caveat (⚠ do not round up):** the DEPLOYED franchise fold is
**flow-free** `pom_scores` — per-cell temporal novelty summed by identity, no external-edge
flow, provably split-immune (`lib.rs:2977-3007`). On the deployed path there IS no
value-dependency between identities to cascade over: the dependency-unwind half of this
design only becomes live when `value_v6+/v8` drives the franchise (the same
v8-before-franchise ordering the pinned +16.7 laundering gap already imposes,
`lib.rs:2939-2975`, scope note `:2954-2956`). What works on the deployed fold today:
tombstone-exclusion of the refuted cell's own standing, nothing transitive.

---

## 3. Best design: TOMBSTONE-MASK COUNTERFACTUAL DELTA SETTLEMENT

**Gate outcome first, honestly: no candidate fully passed.** 4 designs × 2 adversarial
verdicts each = 8 verdicts, all `overall: partial`. But the verdicts CONVERGE: the same
core architecture survived every re-derivation, and the same defect set was found
independently by every reviewer. What follows is that convergent core with the
verdict-mandated amendments folded in. One-line mechanism:

> On a finalized 2/3 refutation, write the refuted cell into a consensus-carried
> refuted-set consumed by the (already-pure) ledger fold; heal = re-derive the world with
> the wound **masked** — one global counterfactual re-run of the live value function; claw
> `min(Δ_i, unvested_i)` per position where `Δ_i = v_i(world) − v_i(world∖X)`; burn every
> cascaded cancellation; bounty at the root only.

### 3.1 The operator (already exists in kind ✅)

`causal_share` (`lib.rs:4349-4365`) IS the un-pay operator run at one index: re-run
`value_v6` with a party removed, diff, clamp at 0. The cascade is the SAME deterministic,
oracle-free computation run at EVERY index, in the downstream/restitution direction
instead of the upstream/liability direction. No dependency ledger is needed — the
counterfactual replaces journaling (dissolves G1). "Recurse on cancellations" dissolves:
the damped fixed point in `downstream_flow_external` already contains every order of
dependency, so dependents-of-dependents are priced in the same pass.

**Flow-direction theorem (verdict-derived, verified against `lib.rs:3402-3468`):** payment
flows CHILD→PARENT (`flow(b) = own(b) + d·Σ_children`, `:3470-3471`). Therefore the
Δ>0 set for refuted X = **X's ancestors whose gates X's seed pumped, plus
standing-laundering chains** — honest DESCENDANTS of X have Δ ≡ 0 STRUCTURALLY (their
value = own novelty × g(own subtree flow), containing nothing of X). Honest strangers are
unreachable, not merely spared by rule. Note this corrects the naive "forward cone"
intuition: the cascade heals the direction laundering value actually accumulates.

### 3.2 Tombstone semantics — MASK, never REMOVE (the unanimous kill-shot fix ⚠)

All 8 verdicts independently found the same fatal bug in "re-derive as if X never
existed": `temporal_novelty` is a first-witness prefix fold over a `seen` coverage set
(`lib.rs:89-99`). Filtering X OUT of the fold's input hands X's coverage to the
next-earliest near-copy — a pre-planted duplicate under a clean key **inherits ~X's
standing at refutation time**. The heal becomes a mint; self-refuting your own garbage
becomes standing-laundering to a fresh identity; the extraction the cascade exists to
reverse is resurrected by the cascade itself (P-001 violation as written).

**Pinned semantics:** tombstone = **attribution-zero + coverage-retained**. The refuted
cell stays in the fold sequence (append-only preserved, provenance/history intact); its
payout, seed, and own-novelty attribution are zeroed; its coverage STAYS in `seen` so
near-copies still floor to 0. Burned, not un-happened. This matches the `causal_share`
precedent (standing removed from the map; cells never removed from the sequence).

**Consequence accepted openly:** automatic un-flooring restitution (a plagiarism victim's
suppressed novelty restored by removing the thief's coverage) is **given up in v1** — the
same fold operation is the copy-resurrection mint, and the mint risk dominates.
Victim restitution = 🔬 open, as a separate bonded-claim channel (inheritance gated to
cells committed BEFORE the challenge opened, or routed through the court), not automatic.

**Required RED-as-designed tests before any build:**
`tombstone_does_not_reassign_novelty_to_a_near_copy`,
`pre_planted_copy_cannot_collapse_the_cascade_claw`,
`refuted_cell_coverage_still_floors_later_duplicates`.

### 3.3 Standing propagation and the runtime bridge (dissolves G2/G3)

The current-state defect inverts into the mechanism: because `Ledger.pom` is a pure fold
(`runtime.rs:524-525`), the durable slash is a **correction of the fold's INPUT**, not a
mutation of its output. The refuted-set rides finalized blocks as consensus content;
`Node::apply` folds over (cells, refuted-set). Replicas converge by the same determinism
argument as today (`runtime.rs:488-491`). Reversal on appeal-overturn is near-free
(remove from set ⇒ re-fold restores) — for standing; see 3.5-E for value burns.

Two honesty corrections from the verdicts:
- **A durable settlement journal is still required** (G1 not fully dissolved): the
  certifier deterrence slash `λ×share+α` is NOT derivable from (cells, refuted-set) — a
  fold-only ledger would resurrect a slashed certifier's standing every apply. The journal
  is a second fold input. "No journal needed" was an overclaim; scope it.
- **On the v6+ franchise, the delta needs a prefix/epoch-indexed evaluation** — entries
  realized at different epochs need the diff at their realization prefix, and the
  epoch-aware value fold **exists nowhere** (every `value_vN` takes one standing map over
  all cells). New machinery, named as such: 🔬.

### 3.4 Termination + frontiers

- **Fixpoint, not walk.** The one place iteration is real: the `standing_floor` seed gate
  (`lib.rs:1090-1096`) is discontinuous — a re-fold that drops a certifier below floor
  zeroes their OTHER seeds, a second-order effect one pass misses. Iterate
  (standing → value → standing) to the fixed point: the tombstone/taint set only grows,
  the value vector only decreases (every gate is an AND-composed floor that can only
  lower: outcome factor ∈[0,1] `lib.rs:1208-1217`), bounded below by v' ≥ 0 ⇒ monotone
  fixpoint on a finite lattice, reached in ≤ provenance-depth iterations. Inner
  convergence: damped Jacobi contraction d<1 + ρ=1/φ rank decay (`lib.rs:3434`, break
  `:3463`). ⚠ The floor cliff needs **materiality/hysteresis** (zero a seed only when the
  shortfall attributable to refuted value exceeds a bound) — otherwise an attacker who
  fake-inflates a rival's standing just above floor pre-certification weaponizes the
  discontinuity as grief (blast radius > the fraud's cone).
- **Frontier F1 — zero-delta contour:** Δ_i = 0 ⇒ untouched; nothing downstream of a
  zero-delta node is tainted through it.
- **Frontier F2 — vested firewall:** vested stays untouchable (`lib.rs:4300-4301`).
  Depth-k vest-racing is closed NOT by freezing clocks but by **cone-scoped snapshot
  extension**: at challenge-open, entries in the Δ>0 cone are marked
  cascade-cancelable-even-if-they-vest-during-the-dispute — extending the existing
  snapshot idiom (`lib.rs:4392-4395`) without any liquidity freeze. This preserves
  DISPUTE-SLASHING.md:59 ("vesting clock is NOT reset — no griefing-by-delay"), which a
  subtree clock-pause would directly violate. Named residual ⚠: pre-detection hop-by-hop
  vesting (laundering fully vested before any challenge opens) stays behind the finality
  firewall — W_s calibration, not cascade reach, is the lever there.
- **Frontier F3 — bona-fide stop: DROPPED at the value layer (verdict-mandated).** The
  proposed test (zero Hodge harmonic residual + independently-positive v') is satisfied by
  exactly the cheap attack: one-way feed-forward laundering scores 0 residual **by the
  same theorem that protects honest DAGs** (`lib.rs:373-374`; the code's own pinned blind
  spot: the vested-certifier-endorsing-garbage shape is topology-invisible,
  `lib.rs:1070-1074`), and v'>0 is purchasable with a little real work — F3 would exempt
  precisely the arms-length self-pump windfall. The counterfactual floor (keep exactly
  v'_i) already IS the holder-in-due-course rule for measurable value. Bona-fide
  protection is re-scoped to the TOKEN layer (state-bytes/JUL spent onward for
  consideration, tex:245-251's actual domain) — where it is genuinely needed and where
  **no persisted hop-graph exists today** (`runtime.rs:505-516` retires/persists a flat
  `token_cells` Vec; `collusion_residual_by_identity` reads citation edges, not
  transfers). Token-layer recovery = 🔬 open, honestly re-statused from
  "composition-of-built" to new work.

### 3.5 Anti-extraction bounds on the cascade itself (G6 closed by design)

- **Proportional-by-construction:** claw = `min(Δ_i, unvested_i)` per position — never
  whole honest entries; an honest ancestor keeps aggregate ≥ v'_i (exactly the value of
  their contribution in the world where X never existed). Inherits the v8
  authority-boundary discipline verbatim: gates lower what a cell certifies upward, never
  a cell's own earned value (`lib.rs:1219-1224`).
- **No global conservation rescale.** The proposed clamp Σ claw ≤ v_tot − v'_tot is
  WRONG-SIGNED when mixed-sign deltas exist and attacker-collapsible (a pre-planted copy
  pushes v'_tot ~ v_tot, zeroing the whole claw). Per-position Δ is already exact; drop
  the clamp. (Verdict-mandated deletion.)
- **Bounty at the ROOT only; every cascaded cancellation 100% BURNED** (the
  `collusion_slash` burn discipline, `lib.rs:4570-4572`): challenger payoff is independent
  of the dependent tree's size ⇒ no bounty-harvest across hops, GEV-elim preserved,
  challenge economics stay pointed at extraction roots.
- **α root-only, causal-only** (zero-share-skip generalized to zero-delta-skip); deterrence
  never cascades.
- **Per-identity total capped at current standing**, `unified_slash` extended THREE-way
  (refutation + collusion + cascade-refold delta counted once on lineage overlap) —
  without the extension, double-jeopardy returns.
- **Mint↔sink:** burned = Σ actually destroyed (the `unified_settlement` corrected-burn
  discipline, `lib.rs:4696-4726`).
- **Determinism ⚠ build-prerequisite:** the delta run as drafted rides f64
  `value_v6/v8` (Jacobi, sigmoid, 1e-9 break) — a replica-fork surface. The repo's own
  discipline is Q16.16 fixed-point for anything consensus-carried (`lib.rs:178-179`,
  `value_fixed`). A Q16 port of the counterfactual pipeline is a named prerequisite, not
  an implementation detail.
- **Sequencing ⚠:** cascade burns are irreversible; execution must wait out the appeal
  ladder (bounded, bonds double per round `lib.rs:4797-4799`) or hop-cancellations are
  escrowed-not-burned until appeal-final. Juror slashing punishes but does not restitute.

### 3.6 The cand-A trigger (detection → cascade, sensor ≠ court)

Two admission channels, one settlement path, conviction authority never leaving the
existing machinery:

1. **Bonded challenge (✅ built):** `Challenge(X, B)` while unvested value exists
   (`lib.rs:4310-4312`); 2/3 vested-PoM verdict (`verdict_refutes` `:4326-4343`) with the
   full §7 stack — escalation court, PoM-recused appeal mix (`DISPUTE_APPEAL`
   `:4323-4324`), asymmetric acquittal ratchet (`appeal_refutes_asymmetric` `:4867+`),
   juror accountability (`:4805-4811`), doubling bonds — authorizes the cascading
   settlement instead of the point settlement.
2. **cand-A invariance probe promoted from harness to standing diagnostic (🔬):** the
   probe computes g(σ) = v(σ·S) − v(S) over structure-preserving maneuvers on the live
   v8 path (`lib.rs:2855-3007`); identity permutations are bit-exact zero by contract
   (`:2917-2937`), so a nonzero residual on a zero-baseline class over finalized cells is
   a deterministic, replayable **incompleteness certificate** (the pinned +16.7
   self-flow-laundering split `:2939-2975` is the live example; cand-A as
   dispute-GENERATOR per VS-AS-COMPLETION-PROCEDURE.md:75). A firing probe
   **publishes a machine-readable challenge lead**; the β-bounty makes conversion into a
   bonded challenge positive-EV (p ≥ 1/2 is purchased by β, DISPUTE-SLASHING.md:192-204).
   Per the objectivity dial (tex:285-290): the probe NEVER slashes and NEVER freezes
   bond-free — high-confidence residuals get at most the cone-scoped snapshot extension
   (3.4-F2, no liquidity effect); the verdict remains the only conviction authority
   ("the evaluator is EVIDENCE, never the verdict", DISPUTE-SLASHING.md:100-105). The
   probe certifies "a missing dimension was pumped"; only the court certifies "this
   instance is garbage."

**v1 scope pin ⚠:** Tombstone(verdict) is the ONLY admissible cascade trigger.
BasisUpgrade (retro re-scoring under a shipped v_{N+1}) and Canonicalize (I-2-style
relabel-energy subtraction) retro-claws are NOT admissible via the cand-B corpus alone —
a 3-fixture test corpus is Goodhart-able and tribunal-less; per the objectivity dial,
learned/model repricing routes through the verdict machinery or stays forward-only.

### 3.7 Honest-work safety (theorem + policy, stacked)

1. Counterfactual floor: every honest position keeps exactly v'_i — good-faith building on
   partly-real work loses only the inflation, never the work (own novelty never touched;
   descendants Δ≡0 structurally per 3.1).
2. Vested firewall absolute; no retroactive reach into finalized honest positions.
3. No hunting incentive: burned hops, root-only bounty, α never cascades.
4. cand-B as the cascade's own regression gate: any cascade rule that cancels a
   `must_earn` corpus cell on an unrelated refutation is an inadmissible RED
   (`legit_regression_guard` `lib.rs:2746-2853`) — but **fail-toward-escalation-court,
   not fail-closed** (verdict amendment: a fail-closed precondition lets an attacker
   entangle fraud with a corpus-legit cell to manufacture un-refutable fraud).
5. Court before cascade, always; failed challenges burn the bond minus γ
   (`resolve_upheld` `lib.rs:4512-4521`).

### 3.8 Open risks (the precise residue — why this is `partial`, not `pass`)

- **⚠ TOP RISK — freeze/liquidity griefing.** Every reviewer's sharpest finding. Any
  design where a challenge (or worse, a bond-free probe) pauses vesting across a subtree
  converts anti-laundering into a liquidity-DoS: one flat bond freezes an honest cone,
  serial re-challenges sustain it (no cross-dispute doubling ladder exists), γ compensates
  only the challenged author, never frozen third parties. Mitigation adopted here: NO
  clock-pause at all (cone-scoped snapshot extension instead, 3.4-F2). If any real freeze
  is ever added: bond ∝ frozen exposure, γ ∝ frozen-value×duration paid to the cone,
  per-target doubling ladder, probes escalate-only. Residual even with the mitigation:
  challenge-open forces a global counterfactual recompute on every validating node —
  compute-DoS priced only by B_min (RESOURCE-DOS-BOUNDING pin required; bound
  settlements-per-block; cascade compute off the block-production critical path).
- **⚠ Capture-prize amplification.** A captured 2/3 verdict previously moved one cell's
  unvested entries; the cascade moves an unvested cone. Capture COST is unchanged (§7
  court, the priced global assumption, DISPUTE-SLASHING.md:8-10) but the prize is bigger;
  and burns execute-then-appeal is irreversible (hence 3.5 sequencing). §4/§8 calibration
  must be re-run with cascade-sized prizes; add magnitude-triggered auto-escalation to the
  full-mix court before effect.
- **⚠ Copy-resurrection** if mask semantics slip in implementation (3.2) — one spec line,
  but it is THE kill-shot; hence the pinned RED tests.
- **⚠ Floor-cliff grief** absent materiality/hysteresis (3.4).
- **🔬 Post-W extraction is permanently unclawable** on the value layer (the priced
  finality tradeoff); token-layer bona-fide recovery unbuilt (3.4-F3).

---

## 4. Build classification

**COLD. Consensus-adjacent. Will-gated to build.** The cascade touches earned standing,
which feeds finality at PoM = 2/3 of the fast-final mix (`FINALITY_MIX`
`runtime.rs:584-588`, `MIN_DIM_BPS` `:596`), and turns `Node::apply`'s fold into a
fold-with-consensus-inputs. Same class as I-2, which is already flagged "consensus-affecting
⇒ build COLD, finality decision first" (ISOMORPHISM-INVARIANCE-VS.md:146, 197-199).

**Warm-designable NOW (no finality decision needed, no code):**
- The tombstone-mask spec + the RED-as-designed test fixtures (3.2) as written test cases.
- cand-B corpus extension to cascade semantics (cascade edition of `must_earn`).
- Q16 fixed-point port SPEC for the counterfactual pipeline (3.5).
- Settlement-journal schema (second fold input) + three-way `unified_slash` extension spec.
- Bond-pricing formulas (exposure-scaled) + §4/§8 calibration re-run with cascade prizes.
- The per-epoch/prefix taint definition (3.3).

**Needs the finality/franchise decision FIRST:**
- Refuted-set + journal as `Node::apply` fold inputs (consensus content).
- Anything on the v6/v8-drives-franchise path — the dependency cascade only has an object
  to act on there (§2 caveat); interacts with I-2 and with the `Standing.pom →
  Validator.pom` wiring, itself 🟡 designed-not-built
  (VS-AS-COMPLETION-PROCEDURE.md:257-261, 291).
- Provisional/lagged-standing window (W_s) for finality weight — this doubles as the named
  fix for the circular-PoM-floor finding ("challenge-window discount on freshly-accrued
  standing", VS-AS-COMPLETION-PROCEDURE.md:265) and must be set with the finality design,
  not before it. ⚠ W_s strictly > W + D, and any standing that MATERIALIZES at settlement
  time gets a full fresh provisional window (no zero-length-window landing in the vested
  fold).

Status ledger: point-slash + dispute court ✅ built (reference spec, tests green) ·
recovery-cascade prose 🟡 designed (tex:200-201, 303-304) · this synthesis 🔬 design ·
runtime bridge / epoch-fold / Q16 delta port / token-layer bona-fide 🔬 unbuilt ·
risks ⚠ as marked. No round-up.

---

## 5. Composition: the completion loop, run in both directions

cand-A is the SHARED sensor; the two feeds are the two enforcement halves:

- **FORWARD (pre-emptive gate, exists as tests ✅):** probe certifies a missing dimension
  ⇒ a completion Δv is admissible iff it drives the residual → 0 AND cand-B stays green
  (VS-AS-COMPLETION-PROCEDURE.md:279-280, 321-323) ⇒ ships as the next v(S) floor. This
  half has a structural ceiling: gates can only refuse what they can already see.
- **BACKWARD (retroactive heal, this design 🔬):** the SAME residual, fired over finalized
  cells, arms a bonded challenge ⇒ 2/3 verdict ⇒ tombstone-mask ⇒ counterfactual delta
  settlement claws the instances that exploited the gap before it closed. Detect once,
  immunize prospectively AND heal retroactively.
- **cand-B gates BOTH directions** (`lib.rs:2746-2853`): forward, a completion that stops
  paying honest work is RED; backward, a cascade that cancels corpus-legit value is
  inadmissible (fail-toward-court).

Semi-self-awareness made operational, with its boundary stated: the pure fold IS the
network's model of its healthy (relabel-invariant, honest-flow) self; the probe residual
is the deviation sensor; healing = re-deriving state from the model with the wound masked.
Awareness is exact on the on-graph shadow K and blind outside it (tex:204-233) — theft
that never casts a graph shadow, and extraction that fully vests before any residual
fires, are outside the healing horizon. Knowing that edge is what keeps the immune system
from becoming the autoimmune disease (tex:265-278): no sensor convicts, no cascade pays
its trigger, and no claw exceeds the counterfactual.
