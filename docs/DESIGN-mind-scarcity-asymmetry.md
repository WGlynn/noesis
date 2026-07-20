# Mind-Scarcity as the Base Case — the asymmetry the security model rests on

> **Status: DESIGN NOTE / theory, ready-for-critique (2026-07-20).** This note does one thing: it
> names the cost-asymmetry the whole contribution-security model depends on, shows the model's central
> claim is a *recursion* rather than a *base case*, and locates where that recursion must terminate. It
> is not a build and it does not close the problem — it makes the open problem precise. Status
> discipline: ✅ built · 🟡 designed · 🔬 open.
>
> Origin: pressure-test of the A+C fusion (Will, 2026-07-20). Companion:
> `docs/research/something-from-nothing-oracle-free-content-value.md` (the moat as structural defense),
> `docs/VS-AS-COMPLETION-PROCEDURE.md` §5.1 (the finality floor is capital-orthogonality, not a scoring
> backstop — the same circularity seen from the consensus side), `node/examples/adaptive_sim.rs` (the
> relabel-frontier locator this note explains the ceiling of).

## 1. The central claim is a recursion, not a base case

The value chain's headline defense is: *the one signal no attacker can fake is realized downstream
value — "another mind built on this, and it held up."* A relabeling cannot manufacture it, because a
reshuffle / re-parent / self-flow-launder does not make a real external mind build on the work
(`ISOMORPHISM-INVARIANCE-VS.md` §7). `node/examples/adaptive_sim.rs` confirms the relabel half
empirically: every relabeling rung scores `g = 0` (depth-split, forged-edge-no-coverage, forged-edge-
novel-content); only the paraphrase rung is live (`g = 6`, a *content-proxy* leak, not a relabel leak).

But "another mind built on this" is honest evidence of **value** only if the building mind is itself
**honest and independent**. The attestation is a real signed on-chain edge — an honest *fact* — yet its
*meaning as value* is established by the same value measure applied one level down, to the builder. So
the defense does not bottom out in a non-oracle fact. It **recurses**: the signal is un-gameable iff the
minds producing it are independent, which is verified by the signal. One object, two sides — but the
seam between the sides is a loop, and a loop is what an attacker probes at its cheapest point.

## 2. The base case the recursion needs

The recursion terminates only if:

> **Base case (required).** Standing cannot be bootstrapped from a *closed set of colluding minds.* A
> ring of identities cannot manufacture, among themselves, the standing that makes their mutual
> attestations count.

In the built code the intended terminator is the **payment ≠ standing invariant**: standing is
soulbound and must be *earned by surviving challenge*, never bought (`runtime.rs` `MIN_DIM_BPS`
anti-concentration; `docs/something-from-nothing` §5.4). A wash-mind therefore cannot *purchase* the
standing that would weight its attestation — it must earn it by contribution that itself survives.

The invariant is necessary but **not sufficient**, and the gap is exactly the base case: it stops
capital from buying standing, but it does **not** stop a *closed ring of genuinely-distinct minds* from
cross-earning standing off each other's junk, because to the protocol their edges are real and their
identities are distinct. `sybil_sim.rs` states this boundary in code: v0 + per-identity cap "is NOT
claimed to survive a funded coordinated cartel." The cartel case *is* the unclosed base case.

## 3. Why time, content, and structure cannot supply it

The asymmetry that must make faking-a-build costly while doing-a-build genuinely cheap cannot come from:

- **Time.** "Held up over time" is a time-lock, and patient adversaries have beaten every time-based
  filter that mattered — airdrop farming, restaking points, wash-trading all survived. Time raises the
  bill; it does not invert the asymmetry.
- **Content.** Coverage is *costly-to-fake* only up to the proxy: faking coverage = doing coverage,
  **unless** the coverage metric is a byte proxy a paraphrase dodges. `adaptive_sim` rung 3 (`g = 6`)
  is exactly that leak. A semantic floor / Rosetta canonicalization closes it — but that lands in the
  learned/off-chain layer (non-deterministic ⇒ off the consensus path), so content-honesty is
  achievable but not *on-chain-cheap*.
- **Structure.** Relabel-invariance is closed (`adaptive_sim` rungs 0–2, `g = 0`), but structure is
  precisely what an AI sybil can generate *genuinely* and cheaply. A wash-builder that actually writes
  novel content and actually builds real edges is not relabeling anything — it is orthogonal to the
  entire relabel frame, and the instrument cannot see it by construction.

## 4. The asymmetry, named: mind-scarcity

Strip time, content, and structure and one thing remains: the only un-fakeable "someone built on this"
is **someone outside the attacker's control.** The cost of faking a build is therefore the cost of
controlling enough of the *honest, independent outside* to build on your work. That reduces the entire
security model to one scarce resource:

> **The security of the contribution chain is downstream of the scarcity of independent minds.**
> Genuine building is cheap for the genuine builder (they wanted the artifact anyway — the value is
> exogenous). Fake building is costly only insofar as commanding independent minds is costly. The
> asymmetry the model needs is **proof-of-independent-mind**, nothing else.

**And AI is the adversary of exactly this scarcity.** The one input the whole stack silently assumes is
expensive — a distinct, independent mind that builds because it wanted to — is the input AI agents make
cheap. This is not a peripheral risk to a chain whose thesis is *proof of mind*; it is the thesis's
load-bearing assumption meeting its native attacker. Honesty demands stating it as the core, not as an
"admission control" caveat.

## 5. What could supply mind-scarcity (and why none is clean yet)

| Lever | Supplies | Cost / failure mode | Status |
|---|---|---|---|
| **Proof-of-personhood / admission** | bounds identity *count* directly | capturable ID authority = a new oracle/airgap; the thing PoM exists to avoid | 🔬 the load-bearing bootstrap brake (`sybil_sim` Regime C), unbuilt |
| **PoS-orthogonality (capital floor)** | forces a wash-ring to *also* command ≥50% of the orthogonal capital axis to finalize | secures **finality**, not the **value signal** — the floor is capital-orthogonality, NOT a scoring backstop (`VS-AS-COMPLETION-PROCEDURE` §5.1) | ✅ floor built · 🟡 franchise-wiring designed |
| **PoW on identity creation** | prices each fresh mind in energy | prices *keys*, not *independence* — one funded actor buys many priced keys | ✅ PoW built for blocks; not applied to identity |
| **Social-graph / web-of-trust attestation** | independence via existing honest minds vouching | bootstraps from an honest core; Sybil-vulnerable at the edges; is itself a value measure (recursion again) | 🔬 undesigned |

The honest reading: **the PoS-orthogonality floor is the strongest *built* lever, and it does not close
the base case** — it makes a wash-attacker also buy capital (a real cost multiplier) but never verifies
the *minds* are independent, because the PoM dimension it consents over is downstream of the same
possibly-wash-inflated value (the §5.1 circularity, now explained: it is the mind-scarcity base case
surfacing on the consensus side). Proof-of-personhood is the only lever that attacks the base case
directly, and it reintroduces a capturable authority — the exact airgap the chain was built to dissolve.
That tension is the open problem, stated cleanly.

## 6. What this note does and does not buy

- **Does:** names the asymmetry (mind-scarcity / proof-of-independent-mind) as the security model's base
  case; shows the central defense is a recursion terminating there; locates it in the built code (the
  §5.1 floor-circularity is the same thing on the consensus side); states AI as its native adversary.
- **Does not:** solve it. There is no clean, oracle-free proof-of-independent-mind here, and it may be
  that there cannot be one — in which case the honest chain-level claim is bounded: *un-gameable against
  relabeling and against capital-only capture; NOT against a coordinated ring of cheap independent-
  looking minds, which is priced (personhood + capital) but not structurally impossible.* That bound
  should be stated wherever the moat is claimed, next to the demonstrated-vs-designed line.

## 7. Consequences for the corpus (honest-number discipline)

- The moat docs should carry a third open item beside `learned-v(S)` and the isomorphism gate:
  **HCE-4 / mind-scarcity** — the wash-building / cheap-independent-mind class, orthogonal to
  relabel-invariance, unaddressed by the structural defense, and eroded by AI.
- The 3-axes positioning (`research/three-axes-provenance-funding-ai-merger.md`) rests on the chain→AI
  signal being clean; this note is the precise statement of the condition under which it is
  (independent minds), and therefore a check that positioning must cash, not assume.
- The next *instrument* grain (after `adaptive_sim`'s generative-search extension) is a **wash-building
  sim**: a cost model for a ring of genuinely-building sybils vs the personhood + capital price, to
  measure whether the ring is ≤0 EV — the same "price the ring ≤ 0 before build" gate the corroboration
  design already mandates (`DESIGN-corroboration.md`).

## 8. The constructive dual — this is the collective-intelligence design problem

Read defensively, mind-scarcity is a vulnerability (wash-building erodes the signal). Read
constructively, it is the **definition of the thing being built.** A collective intelligence *is* a
structure where many minds compound each other's work under fair attribution — a provenance DAG with
value flowing along builds-upon edges, standing accruing to contribution, and an endogenous measure of
worth. That is not a defense bolted onto a value chain; it is Noesis's architecture, and its telos
(`[[noesis-as-body-for-collective-consciousness]]`).

So the "distinguish independent mind from sybil" problem and the "be a collective intelligence" telos are
one problem seen from two sides. A collective intelligence is not defined by *excluding* AI minds — that
is impossible and, given the chain→AI thesis, self-defeating. It is defined by its ability to distinguish
**contribution from extraction endogenously, without an authority.** That is exactly the value measure
`v(S)`. The moat is therefore not armor around the intelligence; it **is** the intelligence's cognition —
what the collective recognizes as valuable is what the collective *is*. This reframes the base case: the
goal is not proof-of-*independent*-mind (probably unachievable, and it would gate participation), but
proof-of-*genuine-contribution* — which is the self-completing value measure itself
(`VS-AS-COMPLETION-PROCEDURE`), run at collective scale. Mind-scarcity is the honest statement of *why
that measure has to keep working*: the moment it can't tell contribution from extraction, the collective
stops being intelligent and becomes a farm.

## 9. One line

The chain's security is not "value can't be faked by relabeling" (true, and now measured) — it is
"independent minds are scarce," a base case the payment≠standing invariant needs but does not supply,
that the capital floor prices but does not verify, and that AI is built to erode. Named constructively:
a collective intelligence is exactly a structure that tells contribution from extraction without an
authority — so the base case is not a bug to patch but the property to *be*. Bound the claim to it, price
the wash-ring before trusting the signal, and keep the value measure self-completing — because that
measure is the collective's mind.
