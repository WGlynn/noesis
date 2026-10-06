# DESIGN — the re-opening trigger: biological priors × un-gameable game

> Status: 🔬 DESIGN ONLY (2026-10-06). No code. Warm-designable now (no finality decision
> needed for the design itself); the BUILD is COLD / consensus-adjacent / Will-gated, same
> class as the CLAWBACK cascade it triggers.
>
> Frame (Will, 2026-10-06): nature already solved "commit state hard, but keep a bounded,
> non-exploitable trigger to re-open a wrong commitment" many times. Copy the shape from
> biology; biology's triggers are themselves gameable (molecular mimicry, immune evasion),
> so the limits each one leaves open are closed by game theory — not by a better sensor but
> by a game with no free move ([[boundary-measurement-ungameable-game]]).
>
> Origin: the admission-invariant / re-opening-trigger convergence with Bernhard Mueller and
> Jay in the Pragma / OPH consensus group (loop-holonomy thread, 2026-10). "What triggers
> system-2 intervention on the world model" (Bernhard) is this exact object on the cognitive
> substrate.
>
> Parent docs: CLAWBACK-CASCADE-SELF-HEALING.md (the re-fold this arms; 🔬 design),
> ISOMORPHISM-INVARIANCE-VS.md (the interior sensor; I-1 probe ✅ built, I-2 🔬 open),
> boundary-measurement-ungameable-game.md (the boundary game; accountant built, oracle
> unbuilt), DISPUTE-SLASHING.md (the court; ✅ built), DESIGN-novelty-split-structural-vs-value.md
> (the forward fence; test ✅ green at runtime.rs:2800).

---

## 1. Honest current state (what already exists)

The re-opening problem is NOT greenfield. What is in hand:

- **The re-opening MECHANISM is designed.** Tombstone-mask + counterfactual-delta settlement,
  with a Knaster–Tarski/Kleene convergence result (CLAWBACK §3, DESIGN-convergence-fixed-point.md).
  This is the "what happens once a cell is tainted" half. Status: 🔬 design, COLD.
- **The conviction authority is built.** Bonded challenge → 2/3 vested-PoM verdict, escalation
  court, doubling-bond appeal ladder, juror accountability (DISPUTE-SLASHING.md, ✅ built).
- **An interior invariance sensor is built, as a probe.** The relabel-invariance residual
  `g(σ) = v(σ·S) − v(S)` (I-1, ISOMORPHISM-INVARIANCE-VS.md §6: `value::tests::relabel_invariance_*`,
  ✅ built), computed replica-deterministically with no real-label data. The cyclic slice is
  `attribution_circulation` (verified this session, `lib.rs:358-401` — the bidirectional 2-cycle
  sum, honestly scoped as the KERNEL below the full Helmholtz–Hodge harmonic, which is designed-not-built).
- **The specific gap.** CLAWBACK §3.6 ("v1 scope pin") declares exactly our case inadmissible:
  "Canonicalize (I-2-style relabel-energy subtraction) retro-claws are NOT admissible via the
  cand-B corpus alone — a 3-fixture test corpus is Goodhart-able and tribunal-less." So wash-across-
  semantically-equivalent-re-encodings is currently forced to "route through the verdict machinery
  or stay forward-only." **That routing is the thing this doc designs.**

The task is therefore narrow and well-posed: **make the canonicalize/wash re-opening trigger
admissible without trusting an unaccountable equivalence-oracle.**

---

## 2. The reframe: a two-signal, two-tier trigger (biology gives the shape, the game prices it)

Two internal results constrain any honest answer:

1. **Interior measurement is worth-blind** (boundary-measurement-ungameable-game.md §2):
   genuine and a topology-matched wash score *identically* (0.0% separation). An interior sensor
   can only ever catch wash that casts an on-graph shadow.
2. **Enumeration is incomplete** (ISOMORPHISM-INVARIANCE-VS.md §7): 4 named relabel axes produced
   4 new attacks in one adversarial pass. An intentional adversary attacks the axis you did not
   enumerate. The only *complete* signal no relabeling can fake is **realized downstream value by
   independent minds** (the moat / boundary).

So the trigger cannot be one sensor. It is two tiers, which is precisely biology's layered answer
and precisely Bernhard's system-1 / system-2 split:

- **Tier 1 (system 1) — interior prediction-error reflex.** Fast, deterministic, replayable,
  model-free. Fires on the shadow-casting relabel classes. Structurally incomplete by §2/§7.
- **Tier 2 (system 2) — boundary deliberation.** Slow, expensive, economic. Fires only when a
  bonded challenger stakes that a finalized cell's *independence* signal was forged. The only
  complete signal, but priced, recursive, and partly unbuilt.

The two-signal (costimulation) rule from immunology threads through both: a sensor firing ALONE
is anergic (evidence, never verdict); re-opening always requires an independent, costly second
signal (a posted bond) plus conviction by the court. "The evaluator is EVIDENCE, never the
verdict" (DISPUTE-SLASHING.md) is the immune two-signal rule, independently arrived at.

---

## 3. Tier 1 — the interior prediction-error trigger (shadow-casting wash)

**Biological shape.** Memory reconsolidation: a consolidated (finalized) memory is returned to a
labile, editable state ONLY by a prediction-error — a mismatch between the model's prediction and
what was encountered. No mismatch, no destabilization. The system does not spontaneously re-open
anything; it re-opens on objective surprise.

**Mechanism.** The ledger's healthy-state model predicts relabel-invariance: a structure-preserving
relabeling σ should not move value, `v(σ·S) = v(S)`. A wash re-encoding that casts an on-graph
shadow violates this, and `g(σ) = v(σ·S) − v(S) ≠ 0` IS the prediction error. The residual arms a
bonded challenge; it never convicts (anergy / two-signal).

**Why it is admissible where cand-B was not.** CLAWBACK §3.6 rejected the canonicalize trigger
"via the cand-B corpus alone" — a 3-fixture, Goodhart-able, tribunal-less corpus. The invariance
residual is a different object: replica-deterministic, replayable by every node to the same bit
(I-2 §6: permutation invariance is bit-identical, anti-theater teeth confirmed). A *forged*
certificate fails replay, so forgery is free to detect; a false challenge burns its bond
(resolve_upheld). This is admissible on the SAME footing cand-A already is as a dispute generator
(CLAWBACK §3.6 channel 2) — deterministic evidence, court conviction.

**Game theory overcoming biology's gameability.** Biology's failure here is *molecular mimicry*:
a pathogen forges a self-signal and triggers autoimmune attack on healthy tissue — biology's
defense is imperfect, which is literally how autoimmune disease happens. The game-theoretic close
is the thing biology lacks: make the signal objectively replayable (so a forged mismatch is caught
on replay) AND bond the assertion (so a griefing challenge is negative-EV). Replayability +
bond = the autoimmune-safe version of the immune trigger.

**Honest limits (carry every time).**
- Incomplete by construction (§2/§7). Tier 1 catches named relabel classes only; a competent
  wash-tree with distinct content and honest topology produces `g = 0` and is invisible. This is
  the immune analogue of cancer evasion (downregulate the signal so the sensor never fires).
- The DEPLOYED franchise (`pom_scores`, flow-free) is already relabel-immune (I-1b verified:
  `relabel_invariance_deployed_pom_scores_path_is_split_immune`). Tier 1's *gaming surface* only
  becomes live if/when `value_v8` drives the franchise — same ordering as the pinned +16.7
  self-flow-laundering gap (I-2 §6.1). Until then Tier 1 is a probe, not a live trigger.
- The content axis (paraphrase / reshingle, I-2 A3) is not a topology relabel; its natural close
  is Rosetta semantic canonicalization, which is non-deterministic (LLM) and therefore lands in
  the learned-v(S) layer or a distilled deterministic normal form, NOT directly on-chain (I-2 §7).
  So paraphrase-wash is NOT a Tier-1 case today.

---

## 4. Tier 2 — the boundary trigger (shadow-free wash: the complete signal)

**Biological shape.** The immune two-signal rule at full strength, plus germinal-center selection:
a proposed revision (system 2 / the LLM) does not stick because it was proposed; it sticks only if
it survives selection against *realized* external binding. "Experience decides which stick" (Jay)
is germinal-center affinity selection.

**Mechanism.** For wash that casts no interior shadow, the only non-fakeable evidence is the
boundary: a finalized cell X was credited because the ledger believed an independent-capital mind
realized use of it. The re-opening trigger is a **boundary prediction-error**: a bonded challenger
proves the independence predicate was FALSE — the "independent" downstream builder shared a capital
cluster with X's author. The ledger's belief ("validated by independent use") mismatched reality;
re-open and re-fold with X masked.

**Game theory (this is where the complete close lives, and where it bottoms out honestly).**
- No free move, not un-gameable (boundary-measurement-ungameable-game.md §1, §9): forging the
  original independence signal already cost the attacker a per-cell capital toll (rent one distinct
  cluster per vested cell). Re-opening is a bonded challenge; being wrong burns the bond.
- The numeraire is standing itself (§5 of that doc): the whole EV model (rent ρ, slash σ,
  break-even capital) is denominated in standing, an economic filter, not a truthful-revelation
  mechanism. We claim only the filter.

**Honest limits (these are the real open frontier — do not round up).**
- **Unbuilt oracle.** The capital-cluster *source* that establishes "distinct origin" on-chain is
  NOT built (that doc §3, `lib.rs:7146` verbatim "the source ... is itself unbuilt"). The vesting
  gate that *consumes* it is built but wired to a single `a != b` cluster-id compare
  (`lib.rs:7191-7192`), not into consensus. Tier 2 cannot fire for real until this lands.
- **Adjudicator circularity.** The EV sign gap presupposes a challenge process that already tells
  junk from genuine (`p_slash` wash 0.5 vs genuine 0.02, that doc §6) — the exact discernment §2
  proves impossible on-graph. The judge re-enters through the dispute door; whether the court is
  itself boundary-anchored is open.
- **Recursion, not terminus** (§4a): "another mind built on this" is evidence only if that mind is
  independent, which is the same question one hop down. The boundary relocates Goodhart one level
  out and prices it; it does not end it.
- **Samuelson free-riding dual** (§5): genuine value realized by external parties who never post a
  capital-independent on-chain edge UNDER-vests. Tier 2 fixes over-crediting wash, not
  under-crediting honest off-chain use.
- **Semi-funded-ring leak** (§7): renting ONE distinct cluster per cell vests it in full, banking
  standing "well below any 51% capture." The frontier attack is not the majority cartel.

---

## 5. All actionables — the biological priors × their game-theoretic close

Each row: biological mechanism → how biology's version is gamed → the game-theoretic structure that
overcomes it → honest limit / status.

**A1. Prediction-error gate (reconsolidation).**
- Bio: re-open only on model-vs-reality mismatch.
- Gamed by: a model *asserting* mismatch (forgeable; mimicry).
- Game close: residual must be replica-deterministic + replayable (I-1 ✅ built), + bonded
  assertion. Forged mismatch fails replay; false challenge burns bond.
- Limit: Tier-1 only; shadow-casting classes only. Incomplete by §7.

**A2. Two-signal / costimulation → anergy (T-cell licensing).**
- Bio: recognition (signal 1) without an independent license (signal 2) silences the cell.
- Gamed by: costimulation hijack (pathogen fakes signal 2).
- Game close: signal 2 = a real posted bond (unforgeable cost) + licensed challenger; doubling-bond
  appeal ladder (DISPUTE-SLASHING.md). Court is the only conviction authority.
- Limit: none new — this is corroboration of "evidence never verdict," now with a reason.

**A3. Graded resistance / hysteresis (stronger memories need bigger prediction error).**
- Bio: destabilization threshold scales with consolidation strength; bistable switches resist noise.
- Gamed by: an attacker who parks a rival just above a sharp threshold weaponizes the discontinuity
  (the CLAWBACK §3.4 floor-cliff grief).
- Game close: make the re-open threshold a BOND magnitude ∝ exposure + materiality/hysteresis on the
  floor cliff (CLAWBACK §3.4 already calls for this), not a flat constant.
- Limit / HONEST TENSION: a graded re-open threshold ∝ vesting depth conflicts with the hard
  vested-untouchable firewall (lib.rs vested = "the price of finality"). The hard wall buys a clean
  finality guarantee a graded version gives up. This is a whiteboard trade with Bernhard, not a free
  win — it is exactly his finality-vs-correctability dial.

**A4. Antagonistic-regulator triple (clotting / fibrinolysis / antiplasmin; Treg damping).**
- Bio: every commit mechanism ships with a localized re-opener AND an inhibitor that keeps the
  re-opener from running away. Three layers, never two.
- Gamed by: runaway fibrinolysis = bleeding; absent = thrombosis (over-rigid).
- Game close: pair the trigger with its localizer in one stroke — bond ∝ frozen exposure, bounty ∝
  frozen-value × duration paid to the cone, per-target doubling ladder, probes escalate-only
  (CLAWBACK §3.8 specifies exactly this as the freeze-grief mitigation). Biology says treat it as
  mandatory-paired, not a bolt-on.
- Limit: freeze/liquidity griefing is the reviewers' top risk; the adopted mitigation is NO
  clock-pause at all (cone-scoped snapshot extension, CLAWBACK §3.4-F2).

**A5. Prion / amyloid + post-W (the cautionary prior).**
- Bio: a misfolded protein that is stable AND catalyzes the same wrong fold — a rigid incorrect
  equilibrium biology largely cannot cheaply reverse.
- Not "gamed" — it is the honest horizon.
- Game form: price the finality tradeoff explicitly. Post-W extraction is permanently unclawable
  (CLAWBACK §3.8); the moat is blind outside the on-graph shadow K. Name the cost, do not engineer
  a false promise over it.
- Limit: this IS the limit — Bernhard's flat-earther, confirmed expensive even for evolution.

**A6. Germinal-center selection → the complete close (both internal docs converge here).**
- Bio: proposed revisions stick only if they survive selection against realized external binding.
- Gamed by: cancer immune evasion (no shadow) = shadow-free wash.
- Game close: Tier 2 boundary trigger — realized downstream value by capital-independent minds, the
  one signal no relabeling fakes (I-2 §7, boundary doc §9). Priced per-cell toll, no free move.
- Limit: unbuilt oracle + adjudicator circularity + recursion + Samuelson (see §4).

---

## 6. System-1 / system-2 mapping (Bernhard's frame, made precise)

- **System 1 = Tier 1.** The fast, deterministic, model-free interior invariance reflex. Catches
  cheap named wash instantly and replayably. This is the Cadence-brain "fast settle."
- **System 2 = Tier 2.** The slow, expensive boundary + court deliberation, invoked only when Tier 1
  is blind and the stakes justify the boundary cost. This is the LLM "propose a revision; experience
  decides."
- **"What triggers system-2 intervention" (Bernhard's exact open question)** has an economic, not a
  sensory, answer: system 2 fires when a challenger posts a bond asserting the boundary was gamed.
  You do not need a perfect detector for *when to spend the expensive re-evaluation* — you make the
  challenger pay, and pay them if right. The bounty exceeding the challenge cost is what keeps the
  gate from being permanently stuck shut (the anti-flat-earth / anti-ossification property); the
  point where even that stops (post-W, off-shadow) is the honest irreversibility floor.
- **The flat-earth answer, both directions.** The ledger never spontaneously re-opens (stability /
  no autoimmune chatter). But it can ALWAYS be forced open by someone willing to pay and be right
  (no permanent ossification, as long as bounty > cost). That dial — bond, bounty, W, W_s — IS the
  finality-vs-correctability knob, and it is the thing to co-design with the Pragma group.

---

## 7. Status (built / designed / open — no round-up)

- **Built:** the court (DISPUTE-SLASHING.md); the I-1 relabel-invariance probe + anti-theater teeth;
  the deployed-path split-immunity regression guard (I-1b); the independence-gate *logic* as a pure
  fn not wired to consensus; the forward fence test (runtime.rs:2800, verified green).
- **Designed, not built:** the CLAWBACK re-fold + tombstone-mask + counterfactual-delta (COLD);
  Tier-1-as-trigger (this doc); the two-tier admission (this doc).
- **Open:** Tier 2's capital-cluster oracle (the complete close depends on it); the adjudicator
  circularity / boundary-anchored court; the content/paraphrase axis (Rosetta, non-deterministic,
  learned-v(S) layer); the general isomorphism-invariance gate (graph-iso-hard, I-2 §6); the
  Samuelson under-vesting dual; the non-interference proof that a compromised soft layer can degrade
  coordination but never mint value or finalize a bad cell (must land before any soft scoring ships).

## 8. Smallest buildable grains (warm, no finality decision)

Deploy-independent, no consensus path, Will-gated to schedule:
- Promote the I-1 invariance residual from a test harness to a **dispute-lead generator** spec:
  a firing residual over finalized cells publishes a machine-readable, replayable challenge lead
  (CLAWBACK §3.6 channel 2 shape), bond-gated, court-convicted. Spec + RED-as-designed fixtures only.
- The two-signal admission state machine as a spec: residual → anergic-publish → bonded-challenge →
  court. No clock-pause; cone-scoped snapshot extension (A4).
- Bond/bounty/W_s calibration re-run with cascade-sized prizes (CLAWBACK §3.8 capture-prize note),
  with the graded-vs-hard-firewall trade (A3) written up as an explicit decision for the finality
  design, not pre-empted here.

**Needs the finality/franchise decision FIRST** (do not build ahead of it): anything that makes the
residual or the refuted-set a `Node::apply` fold input (consensus content); Tier 2 (cluster oracle
drives the franchise); I-2 (subtract relabel-variant energy at scoring time — changes earned standing).
