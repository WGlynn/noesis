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

## 5b. Adversarial council findings (2026‑07‑15) — calibrated

A three‑seat adversarial council (sybil/economic · content/ML · systems/liveness, read‑only) red‑teamed
this design then blue‑teamed each attack. Severity is calibrated honestly (several raw findings were
over‑rated, deferred, or already‑mitigated). The genuinely load‑bearing results:

- **FIXED — unbounded ingress seen‑set (OOM):** the screen's seen‑set grew without bound (~24 GB/yr) =
  a self‑inflicted host‑liveness failure. Now capped (`node/src/screen.rs` `MAX_SEEN`; production =
  Bloom filter + snapshot). This matters because **never‑halt is a *protocol* property and is vacuous if
  the *physical* node set → 0** (Will 2026‑07‑15).

- **🔴 Host‑liveness collapse in a value downturn (new).** If JUL value craters, rational operators
  unplug and the chain halts because nobody mines — never‑halt cannot prevent it. ⚠ **CALIBRATION: the
  council's proposed "base‑reward floor" CONFLICTS with the ratified energy‑peg** (`DECISIONS-M3-money-
  2026-07-15.md` §1: identical work must mint identical JUL; a floor mints JUL for no work). **JUL is
  NEVER minted for no work (Will 2026‑07‑15).** The peg‑preserving liveness income is a DIFFERENT token:
  **pay the keep‑a‑node‑alive role (validation) in VIBE**, which is governance, not energy‑money, so
  minting it for validation breaks no peg — and this is ALREADY the design (`DESIGN-governance-authority-
  tiering.md:109‑113`: "VIBE is earned by validating"). So: operators stay online for **VIBE‑by‑validation**
  (+ dispute bounties) even when JUL mining isn't profitable; the **counter‑cyclical reserve**
  (`node/src/reserve.rs`, Lever B) smooths the JUL side; emergency governance quorum‑lowering is the
  last resort. Do NOT add a JUL coinbase floor. (Residual, already flagged in the tiering doc:
  VIBE‑by‑validation lets capital‑that‑operates earn governance ⇒ acceptable only because the tiering
  makes Tier‑2 governance outcome‑bounded — reconcile there, not here.)

- **🔴 Data‑poisoning the learned‑`v(S)` (new, deepest).** The moat (§4) is not only data‑*gated* but
  data‑*poisonable*: if `v(S)` trains on on‑chain citation frequency, a Sybil ring poisons the training
  labels so the model learns to score junk highly. **Defense — repurposes the §1 authority insight:**
  seed the model on OFF‑chain ground truth = **real merged PRs** (a maintainer's merge IS an
  authoritative value label — borrow it for the training SEED without making it consensus), plus
  adversarial‑robust training + an auditable outcome‑label trail. Never trust raw on‑chain citations as
  the label.

- **🔴 Cold‑start deadlock (new).** Early network has no downstream data ⇒ `v(S)` falls back to novelty
  ⇒ junk scores > 0 ⇒ repels real users ⇒ no outcome labels ⇒ the moat never forms. The obvious fix (a
  supervised bootstrap screen) re‑introduces authority. Unsolved; the merged‑PR seed above is the most
  promising bridge (it supplies labels the network can't yet generate).

- **🟡 Coherent LLM‑filler.** Random‑noise padding is ALREADY zeroed by the consensus value layer
  (`value::is_incompressible_q16` + `production_value_zeroes_incompressible_noise`, `lib.rs:1836` — the
  council missed this), so no entropy floor was added to the screen. Grammatical‑but‑worthless LLM text
  still passes every heuristic; only learned‑`v(S)` catches it, near‑term lever = the submission bond.

- **🟡 Design‑notes for when the slash/dispute path is wired (deploy‑coupled, deferred):**
  retroactive‑slash *griefing* (forged disputes against honest work → dispute bond + appeal window +
  slash‑freeze cooldown + public dispute log); retroactive‑slash *inversion* (earn PoM influence during
  the vesting‑`W` window, vote once before a refutation lands → embed refutations on‑chain for atomic
  convergence); bond/dust flooding → per‑block cell caps + UTXO compaction.

- **⬇ Confirmed strengths / downgraded alarms:** a balanced‑cycle collusion ring **fails by
  construction** (single‑parent DAG can't form a balanced cycle); the screen cannot fork consensus (it is
  explicitly advisory) and cannot identity‑censor (it sees only content, not the contributor — residual:
  never let it become the *only* submission path, keep a gossip‑forward/bypass route); FNV
  shingle‑collision gaming is real but pre‑existing in `coverage()` (consensus‑critical, used everywhere)
  ⇒ a Will‑gated call (crypto‑hash shingles), not an autonomous change.

**Through‑line:** every layer's teeth rest on `v(S)`; its honest state is now **data‑gated +
data‑poisonable + cold‑start‑deadlocked**, and the merged‑PR training seed is the single most promising
thread because it borrows an external authoritative value label to bootstrap what the network cannot yet
produce itself.

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
