# The Adaptive-Adversary Instrument — the correctly-specified test for HCE-3

> **Status: DESIGN / spec, ready-for-critique (2026-07-20). Not a build.** This doc specifies the
> *instrument* that would make a null result on the moat's robustness claim **mean something**. It does
> not claim the moat robust; it specifies the experiment whose outcome decides it. Status discipline
> throughout: ✅ built · 🟡 designed · 🔬 open — never round up.
>
> **Scope:** this is a `v(S)` / gameability measurement harness. It is **not** finality and touches no
> consensus path — it *drives* the real value functions read-only, like `node/examples/moat_sim.rs` and
> `node/examples/sybil_sim.rs`, which it extends. Any patch it admits to the defense inherits the
> deterministic-oracle contract of `docs/DESIGN-value-oracle-seam.md`.

## 0. The one-sentence claim under test

**HCE-3 (adaptive-stability / Goodhart-robustness):** *the structural layered defense, coupled to the
completion loop that closes newly-discovered gaps (`docs/VS-AS-COMPLETION-PROCEDURE.md`), converges — an
adaptive adversary with a bounded search budget cannot sustain captured contribution-share above ε across
rounds, and the defense reaches a fixed point (no admissible new gap) in finite rounds.*

Everything below builds the instrument that returns CONVERGE / DIVERGE / OSCILLATE on that claim.

## 1. Why the existing sims cannot answer it (they are open-loop)

Both shipped sims drive the **real** value functions over a **fixed** attack, scored **once**:

- `sybil_sim.rs` — real `pom_scores_with_similarity_floor_q16` (the deployed v0 franchise) over honest
  vs farmer content, per-identity cap, reports captured share. Result: v0+cap loses to costless keygen
  (share ≈ `F/(N+F)`); allowlist bounding identity **count** is the bootstrap brake. Threat model,
  stated in-file: "casual solo scripted farmer, one weekend" — **explicitly not** a funded adaptive
  adversary.
- `moat_sim.rs` — real `value_v5`, `value_v6`, v0 over a fresh-key Sybil ring. Result: v6 identity
  pricing pays the ring **0** where v0 pays it and v5 leaks 28.3. Plus the honest cold-start symmetry
  (an *isolated* honest newcomer also earns ~0, which is why the v0 floor + admission control bridge
  bootstrap).

Both are **one-shot, open-loop**: the attacker plays a script, the defense is frozen, no one adapts.
They prove **static robustness** — the same thing the 253/253 constructed fixtures prove. But the
`ISOMORPHISM-INVARIANCE-VS.md` §7 result is the warning: **one adversarial pass found 4 new vectors
beyond the probe's own first find** ("4 named axes, 4 new attacks"). An intentional adversary attacks the
axis you did not enumerate — and *then attacks the patch you wrote for the last one.* That recursion is
precisely what an open-loop sim cannot see. HCE-3 is a **closed-loop** property; you need a closed-loop
instrument.

## 2. The instrument: a co-evolving min-max loop (double-oracle red/blue)

Honest lineage: this is an **empirical game-theoretic loop** in the double-oracle / PSRO family
(policy-space response oracles) applied to contribution-gaming. We name the prior art rather than claim
the shape novel; the novelty, if any, is the *defender oracle being the completion loop* and the payoff
being *captured finality-share*.

Three components, iterated:

### 2.1 Attacker oracle `A_t` — best-response search against the current defense `D_t`

Given the current defense, search a **parameterized, composable** strategy space for the maneuver that
maximizes captured share per unit cost. The strategy space is the enumerated gaming axes **as composable
operators** (this composability is the point — §7's lesson is that attacks *chain*):

| Operator | Axis | Where it's known to bite |
|---|---|---|
| `split(k)` | identity Sybil split | damped by μ^m (cross-identity) |
| `pad(m)` | volume | damped by λ^r (within-identity) |
| `ring(k)` | cyclic re-attribution | caught by `attribution_cycle_energy` |
| `paraphrase(δ)` | near-dup content, low-overlap reshingle | **A3 — dodges θ_sim; the known deployed gap** |
| `depth_split(d)` | self-flow laundering (vertical edge → external) | **I-1 — g=+16.7 on value_v8; breadth dampers blind (μ⁰=1)** |
| `reparent(π)` | topology fabrication (pointer, not shared coverage) | **A1 — value_v8 trusts parent pointer** |
| `depth×breadth` | composition of `depth_split` + one-child-per-parent | **A4 — every breadth damper stays at rank 0** |

`A_t` returns the best `(share, cost, witness_maneuver)` it found under a fixed budget `(n_identities,
m_cells, k_search_steps)`. Search can be stochastic and off-chain — it is a test harness, not consensus.

### 2.2 Defender oracle `D_t` — the completion loop (`VS-AS-COMPLETION-PROCEDURE`)

1. Run the **cand-A relabel-invariance probe** on `A_t`'s witness: does `g(σ) = v(σ·S) − v(S)` move on a
   maneuver whose honest baseline is 0? A non-zero `g` is a **measured incompleteness certificate**
   (`ISOMORPHISM-INVARIANCE-VS.md` §5, the built `relabel_invariance_*` tests).
2. Propose a patch `Δv` that drives that residual → 0.
3. **Admit `Δv` iff** it drives the probe residual → 0 **AND** the **cand-B legit-regression set** stays
   green (known-honest contributions still score above threshold). This is the anti-Goodhart gate: a
   patch that games its own completion (manufactures a "dimension" to flip scores) fails because it also
   cuts honest work.
4. `D_{t+1} = D_t + Δv`. If no admissible `Δv` exists, `D` is unchanged and the round is a *defender
   pass*.

The admissibility rule is load-bearing and is **why cand-B must be built first** (§5): without the
legit-regression guard, the defender oracle is undefined — it cannot tell a real close from a Goodhart
patch.

### 2.3 The loop and the measured output — the convergence signature

```
D_0 := current structural defense (v6 + built probes)
for t in 0..R_max:
    (s_t, cost_t, w_t) := A_t.best_response(D_t)         # attacker searches the patched defense
    record (t, s_t, cost_t, class_of(w_t))
    if s_t <= ε:  defender_pass_streak += 1
                  if defender_pass_streak >= R_stable: return CONVERGE(t)
                  continue
    defender_pass_streak := 0
    Δv := D_t.completion_patch(w_t)                       # cand-A detect → cand-B guard
    if Δv is None:            return DIVERGE(t, w_t)       # gap found, NO admissible close
    if reopens_prior_axis(Δv): return OSCILLATE(t, w_t)   # patch X reopens axis Y
    D_{t+1} := D_t + Δv
```

The **entire deliverable is the trajectory `{s_t}` and its terminal class:**

- **CONVERGE** — `s_t` falls below ε and the defender passes `R_stable` consecutive rounds (attacker
  finds no admissible gap). **This is the demonstration.** A "null" here — *the adversary cannot win* —
  is now MEANINGFUL, because the instrument **contains a real, searching, adapting adversary** (the exact
  fix the mis-specified-null primitive demands: a robustness claim tested against an actual adversary, not
  honest-static labels).
- **DIVERGE** — `s_t` stays above ε; every patch spawns a fresh exploitable gap faster than it closes.
  This is **also a win**: you've found a live exploit **class**, not a point bug — the honest,
  publishable negative result.
- **OSCILLATE** — closing axis X reopens axis Y (`VS-AS-COMPLETION-PROCEDURE` §5's "gamed through its own
  completion"). The completion loop is Goodhart-unstable; cand-B's coverage is insufficient. Names exactly
  which axis-pair cycles.

Secondary channels logged every round: **cand-B regression health** (does closing gaps cost honest
work? — the moat_sim scenario-3 newcomer symmetry, tracked over rounds not once) and **attacker
cost-per-share** (is the defense at least making gaming *more expensive* each round, even if not zero?).

## 3. Why this is the check B and C both depend on

- **B (dispute adjudication).** The wrapper adjudicates `V − peer-score`. If the adaptive attacker drives
  `v(S)` gaming *faster than disputes resolve*, the wrapper's teeth are theater. The loop measures
  attack-rate vs dispute-latency directly — it is the quantitative gate on whether the Harberger +
  peer-prediction wrapper is load-bearing or decorative.
- **C ("blocks are the training signal").** If the realized-value signal can be adaptively gamed, the AI
  training substrate is **poisoned** — you'd train on gamed provenance. This is the reward-hacking /
  RLHF-Goodhart problem in its native habitat. The loop is precisely the test of whether the signal
  survives an adversary who *knows it is a training target*. C's whole positioning is a check B and A have
  to cash; this instrument is where A cashes it.

## 4. The honest boundary — what the instrument does NOT buy

- **Empirical, not a proof.** Convergence over the *searched* strategy class ≠ convergence over all
  strategies. The §7 recursion reappears one level up: `A_t` only searches axes we parameterized in §2.1;
  an axis neither we nor the attacker-oracle enumerated stays invisible. Honest output is therefore
  **"robust against the strongest adaptive attacker in strategy-class 𝒜,"** with 𝒜 named and its
  coverage stated — the same demonstrated-not-solved discipline the whitepaper and `SECURITY.md` hold.
- **Determinism firewall.** Attacker search may be stochastic/off-chain; an admitted `Δv` **must** remain
  expressible in the deterministic value layer (no floats on the consensus path,
  `DESIGN-value-oracle-seam.md` contract). A patch that requires non-deterministic canonicalization (e.g.
  LLM-Rosetta for the `paraphrase` axis) is flagged **"off-path — belongs to the learned/training layer,"**
  not admitted to consensus. The instrument surfaces that boundary rather than hiding it.
- **Ceiling = cand-B coverage.** The defender can only admit what cand-B allows, so the loop inherits
  cand-B's legit-regression coverage as its ceiling. A thin cand-B set makes CONVERGE cheap and
  meaningless — so cand-B set richness is itself a reported metric.

## 5. Build plan (reuse-first, measurement-first, grounded)

Extends the existing sims; reuses shipped code. Ordering is forced by the admissibility rule.

- **Prereq — cand-B legit-regression guard (🟡→build first).** Already the ROADMAP + §8 priority. The
  defender oracle is undefined without it. Pin a KNOWN-LEGIT contribution set asserting `value_v8` keeps
  it above threshold; a patch is admissible **iff** it drives the probe residual → 0 **and** leaves the
  fixture green.
- **Step 1 — attacker strategy library.** The §2.1 operators as composable `Fn(&mut Scenario)` over the
  real `Cell`/`Script` builders (reuse `moat_sim.rs` `cellc`, `sybil_sim.rs` splitmix `Rng`). Each
  operator already has a known witness in the docs — start by *replaying* I-1 / A1 / A3 / A4 as unit
  fixtures so the library is validated against measured `g` values (e.g. `depth_split` must reproduce
  g ≈ +16.7 on value_v8).
- **Step 2 — the min-max driver** (`node/examples/adaptive_sim.rs`, matching the examples/ convention as
  a measurement artifact) — the §2.3 loop, deterministic, no wall-clock.
- **Step 3 — convergence-signature reporter** — trajectory `{s_t}`, terminal class, cand-B health,
  cost-per-share. Print like `moat_sim` does.

### Smallest first grain — `I-adaptive-1` (two-round loop, pin RED-as-designed)

The minimal instance that produces the **first honest HCE-3 data point**:

1. Round 0: attacker plays `depth_split` (known `g ≈ +16.7` on `value_v8`, `lib.rs:2899-2934` pinned).
2. Defender admits the **I-2 relabel-variant-energy subtraction** (subtract the laundered flow energy at
   scoring time so the same edge earns the same labeled internal or external —
   `ISOMORPHISM-INVARIANCE-VS.md` §5, currently 🔬 unbuilt).
3. Round 1: attacker re-searches the *patched* defense — plays `depth×breadth` (A4).
4. **Measure:** is round-1 share `< round-0` share, or did A4 fully recover the laundered gain?

Because I-2 is unbuilt, this grain is **RED-as-designed on first run** — and that RED *is the first
measured fact about HCE-3*, exactly per the repo's measure-the-gap-then-close pattern. It converts
"adaptive robustness is untested" from a slogan into a number the suite tracks.

> Consensus-affecting caveat: the I-2 subtraction changes earned standing (drives the franchise) ⇒ the
> *real* patch builds **cold, Will-gated, after the finality decision** (`VS-AS-COMPLETION-PROCEDURE`
> §8). The **instrument** measuring it is deploy-independent and safe to build now.

## 6. One-line statement

The moat is an adversarial-robustness property; the only instrument that can validate it is one that
**contains an adversary that adapts.** The existing sims contain a fixed attacker and are correct for
static robustness (253/253, v6=0) but silent on HCE-3. This instrument closes the loop — attacker
best-responds, defender completes, repeat — and returns CONVERGE (the demonstration, where a null finally
means the moat holds), DIVERGE (a found exploit class), or OSCILLATE (Goodhart instability). It is the
check `B`'s dispute market and `C`'s training-signal claim both silently depend on, and its first grain
(`I-adaptive-1`) is a two-round loop that produces the first real HCE-3 number the day it runs.
