# Contribution value without a maintainer — the authority vacuum and the booger‑chain attack

> Status labels: ✅ built · 🟡 designed‑not‑built · 🔬 open research · 🔌 deploy‑coupled.
> Every code claim carries a `file:line`; re‑verify at source before quoting (lines drift).
> Origin: Will 2026‑07‑15 — "a merged GitHub PR is *authority* accepting a contribution; we don't
> have that authority in Noesis" + the "booger‑chain" attack framing.

## 1. The crux: value in GitHub is an AUTHORITY signal; Noesis has no authority

On GitHub, a contribution becomes "real" when a **maintainer merges the PR**. The merge is an
**authoritative acceptance**: a human with write‑authority over the repo judges the work valuable and
admits it. Downstream value (a merged commit others build on) is anchored to that authority event. The
value oracle is *a trusted party with the power to say yes*.

**Noesis is permissionless — there is no maintainer, no merge button, no party with the authority to
accept a contribution as valuable.** Anyone can submit; the ledger finalizes what consensus orders. That
is the whole point (open, capture‑resistant), but it removes the exact signal GitHub leans on. **The
value question cannot be answered by "someone with authority accepted it," because no one has that
authority.** This is the airgap problem in its purest form for a contribution chain.

## 2. The attack the vacuum invites — the "booger‑chain" (Will, 2026‑07‑15)

Without an authority to reject worthless work, an attacker (or a colluding ring / Sybil identities)
writes trivial content — "booger" — and has its own identities **reply to and build on each other**:
`booger₁ ← booger₂ ← booger₃`. This fabricates a dense **provenance graph among themselves**,
manufacturing the *appearance* of downstream value (things built on my contribution!) with no real
value at all. If PoM standing tracks "how much was built on you," a ring mints arbitrary standing by
citing itself.

The booger‑chain exploits two early‑network facts:
1. **novelty ≠ value.** The launch value proxy is temporal novelty (`pom_scores_with_similarity_floor`,
   `lib.rs:194`) — first‑appearance coverage, near‑duplicates zeroed. Distinct‑but‑worthless content
   ("booger", "booger sandwich", "the blue booger") passes novelty while carrying no value.
2. **no authority rejects junk.** GitHub's maintainer would just… not merge "booger." Noesis has no one
   to not‑merge it.

## 3. The reframe that dissolves it: replace *capturable authority* with *harder‑to‑capture structure*

Noesis does not lack merge‑authority and suffer for it. It **replaces a single maintainer's
(capturable, biased, lazy‑or‑corrupt) authority with structural + economic properties that are harder
to capture than one human.** A maintainer can be bribed, fooled, or absent; the following cannot be, at
least not cheaply. Four layers, honestly staged:

### Layer A — literal repetition is already worth zero ✅
The θ_sim near‑duplicate floor (`Constitution.theta_sim_q16 = 0.95`, `runtime.rs`;
`temporal_novelty_with_similarity_floor`, `lib.rs:194`) zeroes any cell whose coverage overlaps
committed coverage past θ. So `booger`/`booger` earns nothing on the second copy — the attacker is
already forced to produce *distinct* content each time, which raises cost. **Built.** (It does not yet
judge value, only originality — §4.)

### Layer B — value only from provenance‑CONNECTED, DIVERSELY‑certified coalitions ✅ (value fn 🔬)
Inter‑block value is a **Myerson‑restricted Shapley** (`lib.rs:3271‑3428`): "only provenance‑connected
sub‑coalitions create value" (`:3334`), estimated by Data‑Shapley permutation sampling (`:3342`),
deterministic in `(cells, samples)`. A booger‑ring *is* connected — but Myerson restriction means value
is shared submodularly across the coalition that produced it, not multiplied by adding more ring
members: overlapping/duplicative contributions **share** their credit (`:3418`), they don't stack. So a
ring of N identities citing one idea splits one idea's value N ways instead of minting N×. **The graph
game is built; what it multiplies is `v(S)` — and a *learned* `v(S)` that scores a booger‑coalition's
`v` at ~0 is the open moat (§4).**

### Layer C — collusion is named and slashable ✅ (slash‑wiring 🔌)
Per‑identity **collusion attribution** (`lib.rs:498‑515`) detects a ring and returns a *bounded* slash
share per colluding identity (`Σ slash ≤ manufactured value`), keyed on the soulbound identity. Crucially,
**"honest provenance (acyclic, diverse certification) attributes 0 to every identity — no false slash"**
(`:510`): a genuine tight collaboration is not punished; only a self‑certifying cycle is. This is the
authority‑free analog of "the maintainer noticed the ring" — except it is a deterministic graph property,
not a human's attention. **Computed; wiring the shares into the dispute‑settlement slash path (composing
with the refutation slash, never double‑slashing) is the deploy‑coupled (dd) step, deferred** (`:513`).

### Layer D — fabrication is negative‑EV: anyone can refute and get paid 🟡
The dispute mechanism + β‑bounty (`dispute::resolve_refuted`) turns "reject the booger" from an
authority's job into an **open, incentivized adversarial game**: a challenger proves a contribution is
worthless/plagiarized, the fabricator's standing is slashed (Layer C names the targets), and the
challenger earns the bounty. This is the real replacement for merge‑authority: **not one gatekeeper, but
a market where catching junk pays.** It bites only when stakes are real and the dispute market is liquid
(§5), and it composes with a **submission bond** (`Constitution.submission_deposit`, Bound‑B,
`runtime.rs`) that makes a K‑junk flood cost `K · deposit`, refunded only if the cell banks value.

## 4. Where the honest gap is: `v(S)`, and why the moat is the endgame

Layers B–D all reduce to one question: **does the value function `v(S)` score a booger‑coalition at
~0?** Today `v(S)` is the *designed novelty proxy* behind the `ValueOracle` seam (`lib.rs:256‑298`),
which "rewards first‑appearance coverage and zeroes near‑duplicates, but **does not yet model value
beyond novelty**" (`lib.rs:284`). A booger that is novel‑but‑worthless still scores > 0. The
**learned‑`v(S)`‑on‑real‑outcome‑labels** upgrade — same seam, swapped implementation — is THE moat
(`lib.rs:266`): a contribution nothing real ever uses has ~0 outcome‑value no matter how many fake
citations decorate it. **🔬 open, data‑gated, undated.** So the *structural* machinery (B/C/D) is built
or designed, but its **teeth are only as sharp as `v(S)`**, and the un‑gameable `v(S)` is the open moat.

## 5. Therefore: the bootstrap heuristic (agreed 2026‑07‑15) — scaffolding for the pre‑robust phase

Early, before disputes are liquid, bonds circulate, and `v(S)` is learned, the structural layers are
weak and the authority vacuum is exploitable. Bridge it with an **advisory, node‑local ingress screen**
— NOT a consensus rule, NOT a trusted oracle baked into `v(S)` (that would re‑introduce the very
authority we removed). It runs at `POST /submit` before a cell enters the mempool, and it is the *node's
own* filter (nodes may differ; it never forks consensus):

- **Quality floor (kills booger):** reject content below a minimum information content — distinct‑shingle
  count / token entropy / length. "booger booger booger" has near‑zero distinct coverage ⇒ rejected.
  Reuses the built `coverage`/shingle machinery (`lib.rs:147`), no LLM, deterministic.
- **Originality floor (kills imported plagiarism):** overlap against the node's local corpus (already the
  on‑chain θ_sim story) **plus an optional bundled external reference set** — catching content copied
  from off‑chain that the on‑chain floor can't see (the gap identified 2026‑07‑15).

It is honest scaffolding: a heuristic that **converges to the structural mechanism** (Myerson value +
collusion slash + dispute market + learned‑`v(S)`) as the network robustifies — the same shape as
vesting‑`W` and the moat. It buys the early network time; it is not the endgame, and it must never be
mistaken for authority (it is a spam filter, not a maintainer).

## 6. One‑paragraph summary (the honest claim)

GitHub validates contributions with a maintainer's *authority* to merge. Noesis has no such authority,
and that vacuum invites the booger‑chain: fabricate a self‑citing provenance ring to fake downstream
value. Noesis's answer is not to appoint an authority but to **replace a capturable human gate with
graph‑structural and economic accountability**: literal copies are already worthless (θ_sim ✅), value
accrues only to provenance‑connected coalitions and is *shared*, not multiplied, across them
(Myerson‑restricted Shapley ✅), self‑certifying rings are named and bounded‑slashable (collusion
attribution ✅, slash‑wiring 🔌), and refuting junk *pays* (disputes + β‑bounty + submission bond 🟡).
All of it ultimately rests on a value function that scores worthless‑but‑novel content at zero — the
learned‑`v(S)` moat (🔬 open). Until that and the dispute market are robust, an **advisory ingress
quality+originality screen** (agreed 2026‑07‑15) is the honest bootstrap. The authority vacuum is not a
weakness to hide; it is the reason the construction has to be sturdier than one maintainer.
