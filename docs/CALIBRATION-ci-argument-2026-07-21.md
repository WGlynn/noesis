# Calibration ledger — the CI / capital-independence argument (2026-07-21)

> Adversarial claim-calibration of the three load-bearing claims behind
> `DESIGN-harberger-peer-prediction-theorems.md` T1 (conditional-independence = capital-independence).
> Every judgment is grounded in a file:line read this session; nothing asserted from memory.
> Status discipline: no round-up. Gate LOGIC built/tested; capital-cluster ORACLE + consensus wiring 🟡.
>
> Domain frame: peer-prediction agreement is evidence of a shared latent truth ONLY IF peer signals are
> conditionally independent given that truth (Miller-Resnick-Zeckhauser / Dasgupta-Ghosh / Shnayder et al.
> Correlated Agreement). A shared common cause *other than the truth* breaks CI. The question for each
> claim is whether "capital cluster" is the ONLY CI-breaking channel (isomorphism) or one of several
> (seductive over-round).

## Verdict ledger

| Claim | Verdict | One-line |
|---|---|---|
| CI-1 | **narrower-scope** (biconditional; only forward direction sound) | Capital-correlation ⇒ CI-fail is sound; "holds EXACTLY on / restored by capital-independence" is necessary-not-sufficient. |
| CI-2 | **OVERSTATED** — flag for correction | "IDENTICALLY / the same set / (CI) restored" rounds a necessary condition up to a sufficient one. Correct as "necessary CI condition against the wash-ring common cause, reused for scoring + vesting (bought once)". |
| CI-3 | **sound-with-caveat** | The CA inversion (collusion scores above genuine) is real, numerically reproduced, and correctly scoped to the capital channel as an EXISTENCE claim. |

---

## CI-1 — "CI fails EXACTLY on capital-correlated neighbors and holds on capital-independent neighbors"

**Verdict: narrower-scope-with-the-scope.** The statement is a biconditional and only the forward
direction is sound. Primary and both cross-checks AGREE (`partial`); no disagreement to surface.

- **Direction A — capital-correlation ⇒ CI-fails — SOUND (true structural isomorphism).** A shared
  controller is a genuine common cause other than ω_x, i.e. exactly the elicitation-literature
  CI-breaker. Grounded: `DESIGN-harberger-peer-prediction-theorems.md:41-42` defines
  `(CI) s_i ⊥ s_j | ω_x ... makes agreement evidence of truth rather than evidence of a shared cause`;
  `:53-55` names the load-bearing failure as `capital-correlated neighbors — a wash ring's cells share a
  hidden common cause (one actor), so s_i and s_j are dependent even given ω_x`. Numerically confirmed by
  the sim (`peer_prediction_sim.rs:58-60`, `same_ring_corr=1.0 / cross_ring_corr=0.5`).

- **Direction B — capital-independence ⇒ CI holds (the "EXACTLY / holds on independent" half) —
  OVERCLAIM; downgrade to NECESSARY-BUT-NOT-SUFFICIENT.** `independent_use_gate` is a pure cluster-id
  compare — `lib.rs:7191-7194`: `let independent = match (cluster(parent_owner), cluster(child_owner)) {
  (Some(a), Some(b)) => a != b, _ => false };` over `capital_cluster: &HashMap<Vec<u8>, u64>`
  (`lib.rs:7167`), keyed on `type_script.args` (`lib.rs:7189-7190`), absent ⇒ NOT independent
  (`lib.rs:7156-7157`). This closes exactly ONE common-cause channel: shared controlling capital. It does
  NOT exclude the other CI-breakers the domain frame names — shared prior / herding on public info,
  semantic copying, sybil-of-a-third-party — all of which pass `a != b` yet violate CI.

- **Removal test (the decisive one):** strip capital-correlation (distinct clusters) but keep a shared
  public prior / upstream benchmark between two genuinely capital-independent builders, and CI still
  breaks. So distinct-cluster is not sufficient to restore CI ⇒ "EXACTLY on capital-correlated" is false
  as an iff. **The source concedes this itself:** `:79-82` boundary (iii) flags the per-neighborhood
  common-prior and requires `the detail-free (prior-free) CA variant ... flagged open in §4`; `:133`
  table row repeats `needs detail-free CA to drop the per-neighborhood common-prior`. The pivotal
  proof-sketch line `:71-72` (`the only remaining dependence ... is through ω_x`) holds ONLY under an
  ASSUMED common prior — itself the unclosed channel.

**Correct scope:** capital-independence is the NECESSARY predicate closing the load-bearing
shared-CONTROLLER channel — the closed wash ring, the case the mechanism is built to price
(`DESIGN-periphery-solution.md:35-38`). On that sub-case the claim is sound. It is not the only
CI-breaking channel and not sufficient for full CI.

**Honest scope:** gate LOGIC built/tested (`independent_use_gate`, `node/tests/discernment.rs`,
`periphery-solution:41-45`); capital-cluster ORACLE unbuilt 🟡 (`lib.rs:7145-7146` "The source of that
signal (a capital-cluster oracle) is itself unbuilt"); consensus wiring 🟡 (`lib.rs:7140-7142`, activation
= governance version bump post-finality). No deployed guarantee.

---

## CI-2 — "The CI-valid reference set is IDENTICALLY Layer A's capital-independence predicate; same set, bought once"

**Verdict: OVERSTATED — flag for correction.** Primary and both cross-checks AGREE (`partial`); no
internal disagreement. This is the claim most in need of correction before anything is built on it,
because it is the one that would be TAKEN as a guarantee ("the protocol buys the independence oracle
once" reads as "CI is thereby secured").

- **What the gate checks:** capital-cluster distinctness ONLY (`lib.rs:7191-7194`, as CI-1). Excludes one
  channel: shared controlling capital.
- **What CI requires:** `s_i ⊥ s_j | ω_x` (`:41-42`) — "a shared cause OTHER than the truth breaks it."
- **The load-bearing "only":** `:70-72` "distinct-cluster identities share no controlling actor, so the
  only remaining dependence between s_i, s_j is through ω_x." That "only" is false in general (removal
  test above; source's own §2 (iii) / §4 concession).
- **Direction test on the biconditional:** necessity (wash-ring ⇒ one cluster) HOLDS
  (`periphery-solution:35-36`; `theorems:53-54`). Sufficiency (capital-independent ⇒ CI) is FALSE —
  `theorems:60` "distinct capital cluster ⇒ no hidden common cause ⇒ (CI) restored" overreaches from
  *capital* common-cause to *all* common-cause.
- **Same-set / "bought once":** operationally fair at the implementation layer — the scoring reference and
  the vesting gate read the same `capital_cluster` map, and the sim asserts it
  (`peer_prediction_sim.rs:69-70` "identically Layer A's independent_use_gate predicate. Same condition,
  bought once"). BUT the map yields a *capital-independence* verdict, not a *CI* verdict. The sim
  HARD-CODES CI ≡ capital-independence: the "independent honest peer" draws signal purely from ω_x
  (`:42-51`) and the only CI-failure ever instantiated is the correlated ring member (`:55-60`); no
  distinct-capital shared-prior pair is modeled. So the two sets coincide as the same
  CAPITAL-INDEPENDENCE predicate — they are NOT shown to be the same CI-VALID predicate.
- **The doc's own status table already grades this row down:** `:134` "T1 = Layer A's predicate (scoring
  set ≡ vesting set) — **argued**" (not proven), and `:133` flags the shared-prior gap open. So the doc
  is internally more honest than the CI-2 statement; the word **IDENTICALLY** in the claim is the
  round-up.

**Correct restatement (safe to build on):** "Capital-independence is *the* necessary CI condition against
the wash-ring / shared-controlling-capital common cause, and that same predicate is reused for both the
scoring reference and the vesting gate — so the protocol pays for the shared-controller filter once. It
closes one of several CI-breaking channels; the per-neighborhood common-prior channel remains open
(detail-free CA, §4)."

**Honest scope:** 🟡 designed-not-built. `lib.rs:7146` oracle unbuilt; `peer_prediction_sim.rs:6-12`
"parametric proof of the SOLUTION SHAPE ... not a shipped guarantee." No deployed guarantee to credit.

---

## CI-3 — "A capital-correlated ring can FABRICATE a fake worth-correlation that CA rewards as if genuine — collusion scores ABOVE genuine work"

**Verdict: sound-with-caveat.** Primary and both cross-checks AGREE (`sound-with-caveat`); both
cross-checkers independently re-derived the sim arithmetic and reproduced it bit-for-bit. No disagreement.

- **Existence/mechanism claim — SOUND, directly instantiated.** Fabrication is verbatim in the source
  (`peer_prediction_sim.rs:55-60`: "coordinate identical reports per task (same-task agree = 1) and
  coordinate to differ across tasks (cross-task agree = 0.5). This manufactures the correlation CA
  rewards"). CA is exactly `same_task_agree - cross_task_agree` (`:20-22`), so the scorer sees only the
  scalar gap and cannot distinguish a fabricated `1.0-0.5=0.5` gap from a genuine ω-mediated one.
- **Inversion — confirmed numerically.** Genuine independent-ref score `+0.245`; fabricated
  correlated-ref score `+0.500`; ring-with-independent-ref score `-0.35`. `0.500 > 0.245` is a STRICT
  inversion (`:65` "scores ABOVE genuine"; `:68` "correlated ref = -0.255 (≤0 ✗ blind)").
- **Uncharitable tests pass.** Because: the fabricated same−cross gap IS the CA payment, not incidental.
  Direction: capital-correlation → one controller → shared common cause → CI fails → CA blind
  (`theorems:53-55`). Removal: swap to an independent ref and the inversion vanishes (`+0.5 → -0.35`) —
  restoring CI kills it.

**Caveats (why not bare "sound"):**
- **(a) Scope / deploy honesty.** Property of the ANALYTIC designed scorer, not a shipped guarantee; the
  numbers depend on chosen design params (prior 0.5, P(H|high)=0.9, P(H|low)=0.2)
  (`peer_prediction_sim.rs:6-12`, `30-33`). CI-3 makes no deployment claim, so it does not violate
  honest-scope, but the caveat stands.
- **(b) EXISTENCE not COMPLETENESS.** CI-3 is correctly worded as "a capital-correlated ring CAN
  fabricate," not "capital-correlation is the ONLY way to fabricate the CA correlation." Any shared common
  cause (semantic copying, herding) produces the same same>cross signature and CA is equally blind; the
  gate does not exclude those (`lib.rs:7191-7194`). That completeness over-round lives in the CI-2
  corollary (Layer-A-restores-CI), NOT in CI-3. As literally worded CI-3 stays in its scope and is sound.
- **Where it would break (not asserted by CI-3, noted for boundary):** if paraphrased to drop
  "capital-correlated" and claim capital-correlation is the only fabrication route, it becomes false.

---

## Cross-reader reconciliation

For all three claims the primary reading and both cross-checks converged on the SAME verdict
(CI-1/CI-2 `partial`, CI-3 `sound-with-caveat`). There is no primary-vs-crosscheck disagreement to
surface; the cross-checks strengthen rather than contest — CI-2 cross-checks add the "sim never
instantiates a shared-prior pair" observation, CI-3 cross-checks add an independent bit-identical
recompute of the sim numbers.

## The single most important correction

**CI-2 is OVERSTATED and must be corrected before anything treats "bought once" as securing CI.** The
gate proves *capital-independence*, which is NECESSARY but NOT SUFFICIENT for the conditional independence
peer-prediction needs. It closes exactly one CI-breaking channel — shared controlling capital (the wash
ring) — and the source's own §2 boundary (iii) / §4 table already flag the per-neighborhood common-prior
channel as OPEN. Downgrade "IDENTICALLY / the same set / (CI) restored" to "the same predicate is the
necessary filter against the shared-controller common cause, reused for scoring and vesting (bought
once); other CI-breakers (shared prior, herding, semantic copying, sybil-of-a-third-party) remain,
detail-free CA still open." And credit nothing as deployed: capital-cluster oracle + consensus wiring 🟡.
