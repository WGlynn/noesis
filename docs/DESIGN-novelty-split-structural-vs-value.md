# DESIGN: splitting STRUCTURAL novelty (gates finality) from VALUE novelty (pays)

> Status: **audit finding + proposed split. The split does NOT exist in code today.**
> Date: 2026-10-05. Discharges the open correction flagged in `ADR-value-layer-placement.md` §8.1
> ("the novelty equivocation ... an open correction, not a resolved one"). Reconcile against the
> binding-sufficiency lemma (`ISOMORPHISM-INVARIANCE-VS.md` / the burn-down defect audit) before
> building further on novelty.
>
> Grounding: every file:line below was read this session, not recalled. Status markers: built /
> designed / open. Never round up.

## 0. Honest floor (what this is and is not)

This is a **read** of the source, not a run. I classified call sites as test vs production by the
surrounding context (test-fn names, `#[cfg(test)]` bands), not by parsing cfg attributes. I traced the
live value signal into standing, finality, and bond resolution; I did **not** exhaustively trace every
JUL issuance path, so "no production reward-from-`value_v*` path exists" is "I did not find one in
`node/src`", not a proof of absence.

## 1. The question

ADR §8.1 says novelty is used BOTH as the finality-weighting proxy AND as a reward seed, and that "a
quantity cannot be both objective and paying." Does the code already split STRUCTURAL novelty
(distinctness, may gate finality, must not pay) from VALUE novelty (pays, must stay off the finality
path)?

## 2. Finding: NO split exists. One base quantity feeds everything.

The single base quantity is `temporal_novelty_with_similarity_floor[_q16]` (coverage-distinctness +
near-duplicate floor; `lib.rs:187` plain, `:207` f64-floor, and the Q16 integer variant behind the
oracle). It feeds all three consumers:

1. **Finality franchise (LIVE).** `runtime.rs:1480` sets `state.pom =
   pom_scores_with_similarity_floor_q16(...)` → `NoveltyOracleV0` (`lib.rs:348`). `finality_pom_weight`
   (`runtime.rs:1092`) reads the same over cleared+unrefuted cells. So **PoM standing = raw floored
   novelty**, and standing is what weights finality.
2. **The `value_v5..v8` reward chain (seed shared).** Every one of `value_v5/v6/v7/v8` begins with
   `let floored = temporal_novelty_with_similarity_floor(...)` (`lib.rs:1189, 1241, 1309`, and the v8
   body) and then gates it by downstream flow (v5), standing floor (v6), semantic floor (v7), and a
   realized-outcome model (v8). Same seed as the finality franchise.
3. **JUL bond forfeit/refund (LIVE, narrow).** `runtime.rs:1481-1487` resolves Bound-B commit-deposits
   "by the SAME per-cell novelty the attribution just used": a cell that banks zero novelty forfeits
   its JUL bond; one that banks novelty is refunded. Gated on `submission_deposit > 0` (inert at 0).

So the same objective-but-gameable quantity simultaneously (a) earns the consensus franchise and (b)
has a money consequence. That is exactly the equivocation §8.1 names, and it is partly wired.

## 3. What is actually LIVE vs test-only (de-escalation, honestly)

The full "same gamed quantity buys both franchise and minted reward" coupling is **not** live, because
the reward chain is mostly not wired:

- `value_v5`, `value_v7`, `value_v8`: **zero production call sites** (definition + `#[cfg(test)]`
  only).
- `value_v6`: **one** production caller, `dispute::causal_share` (`lib.rs:4719/4722`) — sizes certifier
  slashes on a landed refutation; does not set standing or issue reward.
- The live "paying" use of novelty is therefore the **Bound-B bond forfeit/refund** (return-of-deposit,
  anti-spam; `jul_supply.issued` untouched per the comment), not issuance via `value_v*`.

Implication: today a gamed-novelty ring buys (i) finality-franchise standing and (ii) its bond back.
The moment `value_v*` is wired to issuance/reward, the same gamed quantity also buys minted money. **The
split should land before that wiring, not after.**

## 4. Second-order finding: the finality franchise reads the LEAST-gated novelty

Standing (the live finality input) is raw floored novelty with **none** of the `value_v5..v8` gates
(no downstream-flow gate, no standing price, no semantic floor, no outcome gate). The anti-gaming
refinements those versions were built for (sybil-identity ring in v6, noise-certifies-nothing in v7)
do **not** protect the live finality path. This is consistent with ADR §8's residual-risk note: the
finality franchise is contained by **vesting + dispute + anti-concentration**, not by `value_v*`. Worth
stating plainly because it is counterintuitive — the irreversible layer is seeded by the *less*
filtered signal, and the filtered ones are off the live path.

## 5. Proposed split (the §8.1 prescription, made concrete)

- **STRUCTURAL novelty** — canonical-form distinctness only (coverage novelty + similarity floor; the
  current `temporal_novelty_with_similarity_floor`). Objective, deterministic, replica-identical. **MAY
  gate finality. MUST NOT pay.**
- **VALUE novelty** — the realized-use / outcome signal the `value_v*` chain moves toward (downstream
  flow, outcome gate). **Pays. MUST stay off the finality path** (it is the subjective open seam, L1).
- **The seam.** `finality_pom_weight` / standing reads STRUCTURAL only (it already does — this change
  mostly *names and fences* the current behavior, and forbids ever routing a `value_v*`-gated quantity
  into standing). Reward/issuance reads VALUE only, and **reward seeds must stop being raw structural
  novelty** — the shared `let floored = temporal_novelty_with_similarity_floor(...)` first line of
  every `value_v*` is the exact coupling to break: reward should be seeded by realized value, using
  structural novelty at most as an eligibility gate, never as the paid amount.
- **Bond resolution** (Bound-B) keying on structural novelty is defensible (it is an eligibility /
  anti-spam deposit, not a payout) — but call that out explicitly so it is a conscious exception, not
  drift.

## 6. What this blocks / sequencing

1. Fence the invariant now: a code-level assertion/doc that standing is a function of STRUCTURAL
   novelty only, mirroring the ADR §8 "never wire learned/CRPC into `finality_pom_weight`" invariant,
   extended to "never wire a `value_v*`-gated amount into standing either."
2. Before wiring `value_v*` to issuance: break the shared seed so VALUE novelty (paid) is distinct from
   STRUCTURAL novelty (gate), per §5.
3. Reconcile the definition of "structural" against the binding-sufficiency lemma
   (`ISOMORPHISM-INVARIANCE-VS.md`): structural novelty must be isomorphism-invariant or it is just
   another gameable byte-proxy, and the wash result (0% graph-internal separation, `wash_sim.rs`) is
   the standing counterexample.

## 7. One-line verdict

The split is absent; the equivocation is real and baked into the shared novelty seed. Its dangerous
form (one gamed quantity buying both franchise and minted money) is **not** live yet only because the
reward chain is unwired — which makes now, before that wiring, the right time to fence structural from
value novelty.

## 8. Soundness verdict (2026-10-05): is the split sound, and is structural novelty invariant enough to gate finality?

Reconciled the proposed split against `research/binding-sufficiency-lemma.md` and
`docs/research/the-canonicalization-frontier.md` (the ADR's `ISOMORPHISM-INVARIANCE-VS.md` reference
does not exist under that name; these are the live sources). Verdict in three parts.

**(a) What structural novelty actually is.** `coverage(data)` (`lib.rs:160`) = the set of FNV hashes
of all 4-byte sliding windows of the raw bytes; structural novelty = count of coverage shingles not
seen earlier, with near-duplicates zeroed by the Q16.16 similarity floor. This is a **byte-shingle /
representation-identity** proxy by construction (the comment calls it "a proxy for the learned
reward-model evaluation").

**(b) It is NOT isomorphism-invariant, except inside the canonicalization frontier.** Per the
canonicalization-frontier note, representation identity cannot stand in for contribution identity, and
the dangerous direction is **under-merge**: re-encode the same contribution, get different bytes,
different shingles, fresh novelty, silent inflation. Coverage-novelty is invariant only for
exact/near-duplicate byte forms (what the similarity floor catches, explicitly named there as "a
*structural* canonicalization of near-duplicates, not a semantic one, strictly inside the frontier").
Semantic/behavioral sameness is outside the decidable region, a social anchor, not an objective gate.

**(c) Binding-sufficiency says the split is necessary but not sufficient.** The lemma: a gated
consequence `c(phi(a))` is extraction-safe iff `phi` is gain-covering (separates every profitable
deviation from honest). Here `phi = novelty` and wash-building is the deviation with `g(a) > g(a0)`
and `phi(a) = phi(a0)`: distinct worthless work novelty cannot tell from worthwhile. So the quantity
gating finality is **not** gain-covering; the value seam is open (~0% graph-internal separation).
Isomorphism-invariance is the lemma's named second gate, for closing the *relabel* route.

**Conclusion.** The split is **sound as a decorrelation / separation-of-concerns, NOT as an
un-gameability claim.** Its real payoff (from the lemma's "correlated failure domain", §77): finality
and reward today share ONE projection `phi`, so a single wash deviation is simultaneously a value,
governance, AND consensus seam. Splitting structural-gates from value-pays gives finality and reward
*different* `phi` maps, so gaming reward no longer automatically games the franchise. Worth doing even
though neither `phi` is individually gain-covering yet. But "structural novelty MAY gate finality" is
objective only **inside the canonicalization frontier**; outside it, gating finality on coverage-novelty
stays wash-exposed, contained by vesting + dispute + anti-concentration (ADR §8), not by the split. Do
not caption the fence as "finality is now un-gameable."

## 9. Fence built + NI enumeration (2026-10-05)

**Fence (BUILT, green).** Regression guard
`standing_and_finality_are_structural_novelty_only_no_value_layer_leak` (`node/src/runtime.rs`, in
`mod tests`). Pins that `Ledger::pom` (standing) and `finality_pom_weight()` are reproducible from the
structural oracle alone, so any future wiring of a `value_v*`-gated term into standing breaks the
identity. The checkable form of ADR §8's "never wired in", extended per §8.1 to "nor a
`value_v*`-gated amount into standing."

**Full non-interference (established for the CURRENT build, by enumeration).** Read the entire hard
state transition `apply_transition` (`runtime.rs:1378-1508`). Every writer (`token_cells` for
retire / append / JUL coinbase / Bound-B bond burn; `jul_supply`; `index`; `cells`; `height`; `work`;
`finalized_at`; `last_timestamp`; `pom`) reads only `(prev ledger, block, constitution)`. The sole
value signal anywhere on the hard path is the **structural** oracle (`NoveltyOracleV0` / `coverage` /
`pom_scores_with_similarity_floor_q16`); even the Bound-B bond forfeit (`:1489`) uses the structural
oracle, not `value_v*`. `value_v5..v8` have zero hard-path callers (only `dispute::causal_share`,
which feeds `refuted` via a deterministic PoM vote, reduce-only). The soft layer (CRPC / learned v(S))
is not wired in at all, so NI holds for the built system because there is no soft-to-hard channel to
violate. Forward obligation stands: when L1/CRPC is built, its ONLY write path to hard state must
remain the deterministic `ValueOracle` seam, and the fence above now guards the finality sub-path.
