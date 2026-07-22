# CALIBRATION — Backstop / Conjunctive-Composition (BK-1, BK-2, BK-3)

Date: 2026-07-21
Scope: Do BK-1/BK-2/BK-3 (the "peer-prediction CA residual is backstopped by the Layer A vest gate")
hold as properties of the BUILT system, or are they design proposals over a v(S) the code does not
implement? All judgments grounded in file:line; no assertion from memory. Uncharitable to elegant
claims (Because / Direction / Removal tests applied).

Status legend: BUILT / 🟡 designed-not-built. No round-up.

---

## Load-bearing fact that governs all three claims

**The conjunctive v(S) = retained(CA) * vest does NOT exist in the built value function.**

- `value_v5` (lib.rs:1193-1197), `value_v6` (lib.rs:1256-1261), `value_v7` (lib.rs:1324-1329),
  `value_v8` (lib.rs:1440-1444) ALL terminate identically: `floored_novelty * flow_gate(downstream)`.
- The standing / semantic / outcome gates apply ONLY to the flow SEED, and the seed gate keys on
  soulbound PoM `standing` (`standing.get(&c.type_script.args) >= standing_floor`, lib.rs:1246-1247,
  1314-1315, 1408-1409) — NOT on the vesting / independent-use signal.
- `vesting::independent_use_gate` (lib.rs:7164-7202) is a standalone pure function. Grep over lib.rs
  for `independent_use_gate|vesting::` returns EXACTLY ONE hit: line 7164, its own definition. It is
  called by NO `value_vN`, NO `pom_scores`, NO oracle path.
- Its own docstring says so verbatim: lib.rs:7140 "this is a PURE value-layer function, **not** wired
  into consensus/finality... The source of that signal (a capital-cluster oracle) is itself unbuilt 🟡".
- The product form `standing = retained(CA) * vest * v` exists ONLY as a local closure in the sim
  (peer_prediction_sim.rs:156), over HARDCODED vest fractions (genuine 0.75, ring 0.0;
  peer_prediction_sim.rs:153-154), not over any deployed composition.
- Status per design docs: vesting-gate LOGIC ✅ built as a standalone tested fn, but the capital-cluster
  SOURCE + consensus WIRING are 🟡 (DESIGN-periphery-solution.md:41-45); the peer-prediction / CA wrapper
  is 🟡 designed-not-built (peer_prediction_sim.rs:6-8).

Consequence: "v(S) requires the vest gate" is a DESIGN PROPOSAL. On the running system NEITHER gate is
composed into v(S), so there is currently no built backstop for anything to leak from.

---

## LEDGER

### BK-1 — "cheap coordination earns ZERO standing under conjunctive v(S) = retained(CA)*vest"

**VERDICT: OVERSTATED — narrower-scope. Exactly true only under (a) an assumed conjunctive v(S) absent
from code AND (b) a strictly-closed single-capital-cluster ring. Breaks on the deployed value function
(premise absent) and against a semi-funded / 3rd-party-sybil ring (vest becomes > 0, standing partial).**

Reconciliation: primary + both cross-checks agree (all "partial"). No disagreement to surface.

Evidence:
- Arithmetic is faithful to the sim: `retained(+0.5) = 1` (peer_prediction_sim.rs:94),
  `vest_coord_ring_frac = 0.0` (:154), `standing = 1 * 0 * v = 0.0`, labeled
  ":162 DEFEATED — cheap coordination earns 0". So BK-1's "retained(+0.5)*0 = 0" is arithmetically correct.
- BUT the sim flags this exact claim as a LOAD-BEARING ASSUMPTION, not a result:
  peer_prediction_sim.rs:166-169 "holds ONLY IF v(S) composes CONJUNCTIVELY... under a weighted SUM,
  high CA partially compensates zero vest ⇒ cheap coordination earns partial standing... 'the two
  residuals compose' is a hypothesis, not proven."
- The conjunctive v(S) is not built (see governing fact above). So on the deployed value function BK-1's
  premise is simply absent.
- **Removal test breaks the "vest = 0 for the ring" leg.** `independent_use_gate` sets
  `has_independent_use[parent] = true` iff SOME child is in a DIFFERENT capital cluster
  (lib.rs:7191-7192 `(Some(a),Some(b)) => a != b`); the cell then vests its FULL per-cell value
  (lib.rs:7199-7200 `if has_independent_use[i] { per_cell_value[i] } else { 0 }`). The ring vests 0
  ONLY because the sim hard-codes `vest_coord_ring_frac = 0.0` on the premise the ring is one closed
  cluster — exactly how the test models it (discernment.rs:100-101 all wash keys → cluster 0;
  assertion :158 "closed wash (one capital cluster) vests nothing"). A semi-funded ring that recruits
  or rents ONE distinct-cluster 3rd-party identity to buy SOME downstream use flips that child's
  `a != b` branch to true ⇒ the parent vests its FULL value ⇒ partial standing, cheaply, below the 51%
  floor. The gate is binary per-cell (full-or-zero), so minimal independent use buys full vest on the
  touched cells, not a proportional trickle.
- The docs name this exact gap: "the 3rd-party-sybil gap in the capital proxy"
  (theorems.md:178) and "independent_use_gate (a pure cluster-id compare)"
  (DESIGN-...-theorems.md, residual §). It is bundled INTO the same residual the backstop claims to
  close — so the backstop is claimed to close a residual that provably contains its own defeating attack.

Scope where BK-1 is exactly true: closed single-cluster ring + assumed conjunctive v(S). Outside it:
overstated.

---

### BK-2 — "peer-prediction's CA residual is BACKSTOPPED by Layer A; the two residuals are complementary;
attacker forced up to the independent-capital 51% floor (~9/identity)"

**VERDICT: OVERSTATED / UNSOUND as a property of the built system. It asserts as established a
complementarity that (i) the code does not implement, (ii) the sources self-flag as an unproven
hypothesis, and (iii) a concrete dual attack defeats without paying 51%.**

Reconciliation: primary ("unsound") + both cross-checks ("unsound") agree. No disagreement to surface.
This is the strongest overstatement in the set.

Evidence:
- Built v(S) does not compose vest at all (governing fact; value_v8 seed = standing-gate × semantic_floor
  × v_outcome, lib.rs:1403-1430; independent_use_gate never called by any value_vN, grep = 1 hit at 7164).
  So "v(S) also requires vest" is a design proposal, not a running property.
- The "backstop" arithmetic is a hardcoded closure over hand-set constants
  (peer_prediction_sim.rs:153-158); the ring's "vest 0" is ASSERTED, not computed by running its cells
  through the gate.
- Self-flagged as the exact CI-2 overstatement shape: peer_prediction_sim.rs:166-169 and
  theorems.md:181-189 ("'the two residuals compose' is the next claim to CALIBRATE — it is the exact
  shape (X's residual = Y's strength) the CI-2 calibration just caught overstated").
- **The complementarity is a seductive pattern-match, not general.** The same dual attack that breaks
  BK-1 defeats BOTH gates without 51%: coordinate CA reports to clear γ > 0.70 (free, no capital;
  crossover measured at γ*≈0.70, theorems.md:176) AND post minimal independent capital via ONE
  distinct-cluster sybil to earn (full, per-touched-cell) vest. Neither leg pays a network-51% cost.
- **The "51% floor / ~9/identity" quantity is conflated.** The sim's "~9/identity" is a per-identity
  break-even opportunity cost, NOT the network-majority-capital (51%-class) residual, which is a
  SEPARATE majority-capital-cartel case. BK-2 welds a per-identity break-even to a network-51% claim;
  they are different quantities. The honest floor the cheap dual attack must clear is the per-identity
  break-even, not 51%.
- Both the CA wrapper and the vest gate's consensus wiring are 🟡 (peer_prediction_sim.rs:6-8;
  DESIGN-periphery-solution.md:41-45; theorems.md status header "DESIGN / theory").

BK-2 should be downgraded to a designed-not-built HYPOTHESIS conditional on (a) a conjunctive v(S) that
does not exist in code and (b) a capital-independence oracle that is also 🟡. As currently stated it is
overstated.

---

### BK-3 — "the backstop holds ONLY under CONJUNCTIVE composition; under a weighted-SUM v(S), high CA
partially compensates zero vest, so cheap coordination earns PARTIAL standing and the backstop leaks"

**VERDICT: SOUND (narrower-scope caveat required). BK-3's conditional is arithmetically correct and is
itself the anti-overstatement guardrail — NOT an overclaim. The mandatory caveat: the conjunctive v(S)
it conditions on is 🟡 designed-not-built, so nothing in the running system currently HAS this backstop
to leak from.**

Reconciliation: primary + both cross-checks agree ("sound-with-caveat"). No disagreement to surface.

Evidence:
- Because/Direction/Removal all pass on the conditional's own terms: product `retained(CA)*vest*v` with
  vest = 0 gives 0 (peer_prediction_sim.rs:156,158,162); weighted sum `w1*CA + w2*0 = w1*CA > 0` gives
  partial standing (leak). Direction (SUM leaks, AND holds) is right; Removal (drop conjunctivity ⇒ leak)
  holds.
- BK-3 is the SAME text the sources use to hedge: peer_prediction_sim.rs:166-169 and theorems.md:186-189
  state it verbatim as the assumption-to-calibrate. So BK-3 restates a caveat the authors already made;
  it does not round up.
- Mandatory scope: the conjunctive v(S) it presupposes is not wired into value_v5..v8 (governing fact);
  `independent_use_gate` docstring lib.rs:7140 confirms "not wired into consensus/finality". The product
  form is a sim-local closure (peer_prediction_sim.rs:156). Where vest actually meets value in a test it
  is SUBTRACTIVE EV, not a conjunctive standing product: discernment.rs:163
  `ev = harvest * (vest - rho - p_slash*sigma)`.

BK-3 is sound. It is the guardrail; the overstatements it warns against live upstream in BK-1 and BK-2.

---

## Summary of corrections needed before anything is built on this

- BK-1: add "under an assumed conjunctive v(S) (not in code) AND a strictly-closed single-cluster ring."
  Replace "earns ZERO" with "earns zero only for the closed-ring special case; a semi-funded ring earns
  partial standing via one distinct-cluster sybil, cheaply, below the 51% floor."
- BK-2: strike "backstopped / residuals complementary / forced up to the 51% floor" as an asserted
  property. Downgrade to 🟡 hypothesis. Do not weld the per-identity break-even (~9/identity) to a
  network-51% claim — different quantities. Name the dual attack (coordinate CA free + one distinct-cluster
  sybil for vest) as the open defeater.
- BK-3: keep. It is correct and is the calibration guardrail; only add the "conjunctive v(S) is
  designed-not-built" scope so readers do not infer a running backstop.
