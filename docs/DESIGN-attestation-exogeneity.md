# DESIGN — Attestation exogeneity: why the wash anchor reduces to Noesis's two irreducibles

> Status: 🔬 open-problem map, not a mechanism spec. This records a reasoning chain and its
> honest terminus. Every proposed fix in it was tried and defeated; the value is knowing
> exactly where the hardness lives so we do not re-walk the dead-ends.
>
> Provenance: derived 2026-09-29, stress-tested via adversarial AI review the same day. The
> external substrate discussed (Cadence `witness`/`estimate` admission) is a public library;
> this note is about the *mechanism*, not any collaboration.

---

## 0. What this documents

The wash-building problem (`docs/DESIGN-mind-scarcity-asymmetry.md`, measured in
`node/examples/wash_sim.rs`: **0% graph-internal separation**) has a known necessary condition —
an external value anchor the suspected set cannot mint for itself. This note walks a concrete
candidate for that anchor (an external-use attestation primitive), tries every way to make it
bind, and shows each attempt deferring onto one of two already-open cores. The terminus is a
cleaner statement of the residual than "the airgap" alone.

---

## 1. The candidate anchor and its PoC

**Idea.** Ground value on *external use by capital-independent minds*. Cadence (a public
predictive-coding library) stamps each learning admission with provenance: `source="witness"`
(an actual external observation) vs `source="estimate"` (a self-generated/derived value), bound
into a monotonic, hash-identified, append-only event log. A population of capital-independent
minds admitting `witness` events for a contribution is a concrete instance of the "external use"
signal `node/examples/periphery_sim.rs` currently only parameterizes.

**PoC result (honest).** A headcount model (`~/cadence-lab/job3_wash_witness_anchor.py`):
external-witness count separates genuine (+12) from wash (0) where the graph cannot (0.000), and
degrades **linearly to zero at 50% attestor capture**. First read: "reduces wash to a
51%-on-attestors assumption." That read is **wrong**, for the reasons below.

---

## 2. The teardown — every fix defers, recorded so we don't re-walk it

### Dead-end A — "impose a cost per attestor"
Any one-time attestor cost is one of two things:
- **capital** (stake/bond) — purchasable, so it violates the capital-independence the anchor
  needs. *A base case that can be bought is a bought anchor.*
- **prior attribution** (standing) — assumes the output of the graph you are bootstrapping.
  Circular.

So the cost lands on the **genesis base case**, which is already open (`docs/…base-case…`; cf.
the exogenous-bootstrap thesis: PoW is the genesis anchor because it is the *only exogenous*
value of the three primitives). **Conceded.**

### Dead-end B — "verify independence, not count"
Cycle-energy and shared-provenance detection (`attribution_cycle_energy`,
`attribution_circulation`, Helmholtz–Hodge; on-chain `WashRingDetector` = mutual/2-cycle only)
see correlated **paths**, not correlated **priors**. A funded ring gives each attestor a distinct
real root and cross-cites only at the target: same topology as honest corroboration — no cycles,
no shared roots. That is the **shared-prior attack**, and it inherits the exact inside-the-graph
invisibility `wash_sim.rs` already proved. Independence verification catches *lazy* rings, not
*funded* ones. **Conceded.**

Note both A and B fail for the same reason: they are **endogenous**, and anything endogenous a
funded ring supplies in kind.

### Candidate C — "per-attestation bets resolved by a later exogenous outcome"
The right *shape*: make the cost **recurring** (N false attestations pay N separate prices, no
amortizing one entry cost) and pay it **to reality** (commit-reveal pairwise slashing — already in
the repo — pointed at attestation instead of evaluation, with a delayed outcome anchor). This is also the economic form of *witness = observed
consequence*, not *witness = declaration* — the distinction Cadence cannot currently make because
its learning underfits without heavy calibration (`~/cadence-lab/explore_01_lifecycle.py`), so a
`witness` today is a declaration, not a measured downstream effect (`brain.py:275`: "the brain
does not independently authenticate it").

C is a strict improvement over A/B but does **not** close, on three counts:
1. **Proposition selection.** If the attestor picks the claim, they fill it with true-anyway
   facts and collect vindication for free. Defense: the resolving proposition must be
   **protocol-pinned to the contribution** ("did X produce realized external value by T"), not
   attestor-chosen — else the attestation certifies safe facts, not the ring's content.
2. **Hedge.** Even pinned + adversarially resolved: if the ring is long the outcome *out there*
   while short (attesting-false) *in here*, the slash is a hedge cost, not a loss. Enforcement by
   exogenous settlement binds only if **the resolution market is closed to the judged parties.**
   Reality is non-fungible; *access* to reality is a market like any other.
3. **Dispute-generation capture.** The only picker that prices wash is a **permissionless
   adversarial challenger paid from the slashed stake** (a fraud-proof bounty — "why bother" =
   they keep what they slash). But the ring can be the challenger, bribe it, or out-fund it. So
   the anchor's effective price = the cost of **capturing dispute generation** — the open layer.

---

## 3. The terminus — a conservation law

Every fix moved the hardness; none destroyed it. That is not coincidence:

> **You cannot extract a judgment about the external world from a system closed to the external
> world.** Any value oracle is a channel to outside, and channels can be owned. The hardness is
> *conserved*: it relocates (base case → attestor cost → dispute layer → resolution market) but
> never vanishes, because it is the airgap itself — the map is not the territory.

So the wash anchor's residual is not "51%-on-attestors" and not "the airgap" in the abstract. It
is precisely the two cores Noesis already knows are the frontier:
1. **the exogenous base case** (genesis; "base case is God"; PoW as the only exogenous value), and
2. **dispute-generation / oracle capture resistance.**

**Wash-building is not a new open problem. It is a re-expression of those two.** That is a
class-level result (cf. class-elimination over instance-patch): the wash anchor cannot be built
from anything endogenous, and every exogenous reach re-enters through a capturable gate.

---

## 4. Load-bearing scope correction — "oracle-free"

Landing on a resolution source makes the top layer **oracle-dependent**. Therefore:

- **"oracle-free" is a claim about the ATTRIBUTION layer only** — the seven mechanical `v(S)`
  gates every node recomputes identically (`node/src/lib.rs`, `value_v5..v8`).
- **It is NOT a claim about the wash anchor.** The wash anchor is oracle-dependent by
  construction, because it must reach an exogenous resolution.

Any document asserting "oracle-free" must carry this scope, or it overclaims.

---

## 5. Why this is fundamentally hard (the frame)

Value is not a property of a thing; it is a relationship between the thing and the minds that
receive it. Energy is physical (PoW anchors there); capital is a ledger fact (PoS anchors there);
worth lives only in reception, so there is nothing local to measure.

**Bitcoin did not solve the airgap — it dodged it**, by redefining money to be the thing the
ledger controls. A bitcoin *is* the record; there is no external fact to check, so the ledger is
authoritative by construction. Noesis cannot use that hatch: it prices something irreducibly
**exogenous** — the worth of a contribution — so it inherits the full airgap money got to skip.
Detecting wash = detecting worth = crossing the airgap. If wash were detectable from inside,
worth would be an internal property, and then it would not be worth — it would be another token
the ledger controls, and the ring would mint it. **Wash is hard for the same reason value is
real.**

---

## 6. Open research items (what would move it)

- **Witness = measured consequence, not declaration.** Requires Cadence-class learning to
  actually fit (currently underfits). Turns the shallow signal into a real downstream-effect
  measurement. Gates the whole anchor's *meaning*.
- **A resolution market closed to the judged parties.** Without eligibility-gating the resolver,
  the hedge (2.2) neutralizes the slash. This re-enters the base-case problem (who is eligible).
- **Dispute-generation capture resistance** (`docs/DESIGN-corroboration.md` — staked
  peer-attestation is the closest existing lever). The anchor's price equals this; it is the true
  residual.

## 7. One-line status

The external-use anchor is real in *shape* and defers cleanly onto {exogenous base case,
dispute-generation capture resistance}; it is **oracle-dependent**, its security is **not**
inherited from any external substrate (Cadence supplies the attestation *format*, zero scarcity),
and its effective price is set by the one layer not yet solved. Knowing that precisely is the
result.
