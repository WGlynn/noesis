# DESIGN — parametric clawback / owner-published constraints (justice at the wallet layer)

> Design tick (no code; PCP-gate — this touches `is_valid_in_ledger` / `spend_is_authorized` = the
> spend-validation trust boundary at highest blast radius; decide cold, build fresh, per the
> (v)/(dd)/(ii)/(kk)/(rr) discipline). Crystallized in a strategic design conversation with Will
> 2026-06-23 (right after the (ss) on-VM lock script landed). Advances justice-at-the-wallet-layer
> from open question → **DECIDED principle + first build contract**.

## The question
Should clawback cascades be consensus-native (a justice-like presence at the wallet level that
deals with theft / fraud / harm), or should the broad-harm case be governed by slashing?

## The deciding principle — the topology-shadow test
The chain may only adjudicate what casts a shadow on the value-flow graph it already computes
(`v(S)` provenance, the Hodge/Shapley machinery). So the cut is NOT theft/fraud/society; it is:
- **Harm with a flow-topology** (value moved out of a rightful owner, or was manufactured/extracted)
  → structural, decided by math on the provenance graph, no human blessing.
- **Harm with no on-chain topology** ("damaged society/the ecosystem") → the chain MUST NOT adjudicate
  it (the un-closable airgap; any mechanism that claims to is a captured corrector — DAO-blessing — or
  Babel, the chain as final moral arbiter). Judgment stays in math; math is silent on diffuse harm.
  This stays OFF-chain (the CMD frame: off-chain moral commitment is the enforcer the chain
  deliberately does not replace). Throne-not-Babel.

## Clawback and slashing are ONE surface, remedy selected by the value's legitimacy
Not competing philosophies. One justice surface (provenance evidence → remedy):
- Value **pre-existed legitimately and was misappropriated** (theft) → RETURN it → **clawback** =
  the provenance cascade run backward from the theft point.
- Value was **fabricated, never legitimate** (collusion, fake `v(S)`) → no one to return it to →
  **slash** (destroy standing, redistribute) — already built: `collusion_slash`, `unified_slash`.
So "damage to the economy" that surfaces as measurable EXTRACTION is slash-with-redistribution; the
part with no graph-shadow is not the chain's to govern at all.

## Bona-fide-purchaser = the collusion detector run in reverse (structure does the work)
A clawback cascade must not confiscate from whoever currently holds the value (that re-creates the
confiscation power we exist to dissolve). The "holder in due course" rule: an innocent party who gave
real consideration is protected; the loss falls on the thief. We already have the exact test —
**real arms-length exchange = bidirectional flow (Hodge gradient); a wash/colluding hop = circulation
(harmonic)**, which `attribution_circulation` / `attribution_cycle_energy` already fire on. The cascade
passes THROUGH wash hops and STOPS at the first bona-fide receiver, eating the loss against the thief's
other standing. The justice mechanism is the provenance machinery read backward, same gradient-vs-curl
decomposition deciding who is protected. No new module.

## The objectivity dial (Will 2026-06-23) — parametric when measurement is thermometer-grade
Some theft recovery should be PARAMETRIC (automatic, no court) when there is high confidence in the
OBJECTIVITY of the measurement — the way people trust a thermometer. A thermometer is trusted because
the reading is (a) reproducible, (b) measured against a FIXED EXTERNAL REFERENT set in advance, and
(c) not a judgment. Map onto theft:
- **Thermometer-grade (parametric-eligible):** double-spend / conflicting authorization; a spend by a
  key the owner ALREADY published as revoked; a spend that breaks a constraint the owner PRE-DECLARED
  on the cell (rate-limit, allowlist, time-lock, ceiling). Strongest class = owner-published rule:
  "unauthorized" is measured against the owner's OWN published referent, not inferred intent.
- **NOT thermometer-grade:** "I was tricked into signing," scam, deception — resolves to a mental state
  no one can read on-chain → court, forever.
The boundary is a CALIBRATED CONFIDENCE DIAL, and the remedy scales along it:
- Deductively certain (double-spend, revoked-key, violated-published-constraint) → immediate, no window.
- High-but-not-certain → parametric FREEZE (reversible) + short auto-resolve window.
- Medium → bonded court (measurement = evidence, not verdict).
- Low / no graph-shadow → off-chain.

## THE governance resolution — self-binding ⇒ permissionless signals, ONE scope invariant
Owner-published constraints are **self-binding, not other-binding**: a rule you publish only restricts
spends of YOUR OWN cells; it can never reach another wallet. So "anyone can design any signal" is true
AND safe — you are writing the terms of your own custody (a trust deed / a will), not a law. Governance
is only needed where one party's choice imposes on another; self-binding constraints have ZERO
externality by construction ⇒ permissionless by construction. No registry, no acceptance vote — the
governance is about SCOPE, never CONTENT.
The protocol owes exactly **one invariant** (a slow-ossifying constant, not a vote — the
governance-decay-timebomb frame: ossify fast → immutable):
> A published constraint can only RESTRICT the publisher's own value, and a reversal can NEVER reach a
> bona-fide third party who took that value in good faith.
That single scope rule is the whole governance surface. It is ALSO the defense against the one real
attack on permissionless self-binding — weaponized self-binding (buyer's-remorse fraud: publish "this
spend was invalid," let the merchant deliver, then reverse). Bona-fide-receiver finality blocks it:
the merchant who delivered against a confirmed payment is bona-fide, so the self-published rule can't
reach the settled value they hold. **Permissionless self-binding is safe IFF bona-fide finality is also
consensus-native — a matched pair; either alone breaks.**

## Prevention over clawback
Most "theft" collapses into PREVENTION: an owner-published constraint as a spend-time VALIDITY
PREDICATE means a violating spend simply never finalizes — no value moves, no third party, no cascade.
You don't claw back what you can pre-empt. Clawback (the reversal) shrinks to the hard residue: a
CONFORMING spend by a compromised key (the thief obeyed every published rule). That residue is the only
place bona-fide-protection + court are needed.

## The court is shrinking scaffolding
The parametric/court boundary is not a wall — it is the objectivity dial's threshold, and it SLIDES
DOWN as cognition cheapens (Will's latency point). Cases "too expensive to judge → court" today become
"instant objective readout" tomorrow; the objective tier EATS the court tier. Build the boundary as a
PARAMETER, not hardcoded, so it slides without a fork. Justice withers into physics over time =
source-war "state withers by obsolescence" at the justice layer.

## WHAT SHIPS NOW (Will greenlit) — the deductively-certain parametric tier
Owner-published constraint as a **spend-time validity predicate on the cell, self-binding-scoped,
riding `spend_is_authorized`.** Confidence = 1 BY CONSTRUCTION (deductive — no calibration, no data,
distinct from the empirical/learned signals which inherit the learned-`v(S)` data pull and are
deferred). Revoked-key is the first, simplest instance (constraint = "key X is dead").

### Build contract (fresh low-context — spend path, highest blast radius)
1. A cell may carry a `published constraint` (a structural predicate over the spending tx — e.g. a
   revocation root, a per-window ceiling, an allowlist of permitted next-owners / type-scripts). Define
   the minimal predicate set; START with revocation (a spend whose `lock.args` key is named in a
   published revocation = invalid). Predicate is committed content (consensus-present), never a free
   witness ([dont-let-attacker-choose-critical-input]).
2. Wire into `is_valid_in_ledger` AFTER existence + control: each consumed input must also SATISFY its
   own published constraint, else the spend is rejected. Self-binding by construction (the predicate
   reads only the cell's own published rule + the tx).
3. **Scope invariant (the one consensus rule):** the predicate may only RESTRICT — it can reject the
   publisher's own spend, never enable a reach into another cell. Assert structurally (a constraint
   that would alter a cell outside the publisher's authority lineage is itself rejected).
4. Sentinel-gated inert pre-deploy (mirror `CONTROL_BINDING_ACTIVE` / `CONTROL_ENFORCED`): a cell with
   NO published constraint behaves exactly as today (honest flows unchanged); a PRESENTED constraint is
   enforced for real.
5. Tests: a revoked-key spend is rejected; an honest spend under a live key validates; a constraint
   cannot reach another owner's cell (scope invariant); existence ∧ control ∧ constraint compose (no
   gate masks another); anti-theater — rubber-stamp the predicate ⇒ the revoked-key test goes RED.
6. LEAN (PONYTAIL, lean-like-Bitcoin): reuse the existing identity tuple + the lock-sig digest; NO new
   nullifier/identity type; revocation only in v1, the richer predicate algebra is a later grain.

## Honest scope / deferred
- This ships PREVENTION (validity predicate) + the deductive parametric tier. The RECOVERY cascade
  (reverse a conforming-but-stolen spend) + bona-fide-purchaser test + the freeze-with-window tier are
  the next grains (the bona-fide test reuses the Hodge gradient/circulation decomposition above).
- Empirical/calibrated objectivity signals = data-blocked, same pull as learned-`v(S)`.
- The off-chain diffuse-harm boundary is deliberately OUT of scope (Throne-not-Babel).

## NEXT after this build
Recovery cascade + bona-fide-receiver finality (the matched pair) · the freeze-with-window tier ·
fold the objectivity dial into a single sliding parameter · then the empirical signals when the moat
data lands.
