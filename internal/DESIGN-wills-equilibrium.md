# M1 — Formalization: the Honest-Contribution Equilibrium (PRIVATE / stealth)

> Renamed 2026-06-23 (Will: "neutral everywhere"). Was "Will's Equilibrium"; the eponym is retired
> per the v4.0 whitepaper decision (¬name-after-me). Internal handle and public name are both
> **Honest-Contribution Equilibrium (HCE)**. This file is M1 of `ROADMAP-WILLS-EQUILIBRIUM.md`:
> the paper-grade statement of the game, the HCE definition, the honest positioning vs prior art, and
> the proof obligations marked demonstrated-vs-designed. Companion: `DESIGN-adaptive-convergence-theorem.md`
> (M2, the one open linchpin). Run `/critical-qa` before this enters the whitepaper.

## 0. The one-line claim
Proof of Mind reaches an equilibrium where honest contribution **and** honest self-reporting is the
profile no one wants to leave — and it stays that way not only against unilateral and coalition
deviations (standard, achievable with a fixed payoff rule) but against an adversary who *learns*,
because the value measure `v(·)` itself retrains on realized outcomes. The third property is the
contribution; it is the formal statement of *"any fixed formula gets gamed the moment it is public;
only a measure that adapts is un-gameable."*

## 1. The game `Γ = (N, A, M, u)`
- **Players** `N = {1,…,n}` participants, plus a principal `P` = the protocol, who fixes the measure.
- **Action** of `i`: `a_i = (c_i, r_i)`.
  - `c_i` = contribution action over the provenance space: what to produce, and whether to pad,
    sybil-split one identity into many, or form a mutual-citation ring (i.e. `c_i` ranges over honest
    work *and* every known graph-level manipulation).
  - `r_i` = self-report: a claim about a fact the chain cannot verify on its own (provenance / quality
    of `i`'s contribution), backed by a bond `b_i`.
- **State**: contributions induce a provenance DAG `G`; the mechanism scores a cooperative-game
  characteristic function `v : 2^{contribs} → ℝ_{≥0}` and allocates standing + state-capacity.
- **Mechanism** `M = (v, 𝒢, T)`:
  - `v` — the value measure (today: novelty-weighted, saturated, Hodge-corrected `v(S)`).
  - `𝒢` — the **guards**, each a *structural enforcer* (not a deterrent cost):
    - temporal **novelty** ⇒ padding / sybil contribution → 0 (no new information, no value);
    - geometric **saturation** ⇒ raw volume cannot pump standing (concave in quantity);
    - **HodgeRank harmonic residual** ⇒ circulation/collusion flagged on topology alone, wired to
      `collusion_slash` / `unified_slash`;
    - **standing-gating** ⇒ allocation gated on soulbound standing, so identity-multiplication is inert.
  - `T` — the **retraining operator**: `v_{t+1} = T(v_t)` learns `v(S)` on *realized downstream
    outcomes* (the AMD control loop / the moat). This is the only adaptive part.
- **Payoff** `u_i(a) = φ_i(v; G) − ℓ_i(a)` where `φ_i` is `i`'s allocation from the measure and `ℓ_i`
  is the expected slash (bond `b_i` lost with catch-probability `p`, plus clawback of any over-claim).

## 2. Definition — Honest-Contribution Equilibrium
A profile `s* = (a_i*)` with every `a_i* =` (honest contribution, truthful report) is an **HCE** under
`M` if:

1. **(Nash)** ∀ `i`: `u_i(s*) ≥ u_i(a_i, s*_{−i})` for every `a_i`. No profitable *unilateral* deviation.
2. **(Coalition-proof)** for every coalition `C ⊆ N` there is no joint deviation `a_C` that strictly
   improves all of `C` — the provenance-geometry guards `𝒢` zero the gains from rings / mutual-citation
   / sybil pools *structurally*, so the deviation is not merely expensive but valueless. (Strong-NE /
   coalition-proof-NE notion, Aumann / Bernheim–Peleg–Whinston.)
3. **(Adaptive-stability / Goodhart-robust)** `s*` remains a (1)+(2) equilibrium under the measure
   dynamic `{v_t = T^t(v_0)}` against an adversary who best-responds to the *current* `v_t`. Formally:
   `s*` is a fixed point of the joint map `(participant best-response) ∘ (retraining T)`, so that no
   *eventually-discovered* deviation is profitable, not only no *currently-known* one.

(1)+(2) is close to a coalition-proof NE under a fixed rule. **(3) is the novelty**: an equilibrium of
a game whose payoff function is itself a learned object co-adapting with the adversary.

## 3. Honest positioning vs prior art (claim only the fusion)
| concept | deviations handled | payoff | what HCE adds |
|---|---|---|---|
| Nash | unilateral | fixed | (2) coalitions + (3) adaptive payoff |
| Strong-NE (Aumann) / Coalition-proof-NE (Bernheim–Peleg–Whinston) | coalitions | **fixed** | (3) the measure adapts |
| ESS (Maynard Smith) | dynamic population | **fixed fitness landscape** | the landscape moves *adversarially* (a principal's control loop) |
| Performative prediction (Perdomo et al. 2020) / strategic classification / Stackelberg learning | the measure adapts to those measured | — | coalition-proofness *from provenance geometry* + the measure is the consensus object |

Honest statement of what HCE *is*: **the equilibrium of a performative value-measurement game on a
value chain, whose coalition-proofness comes from provenance geometry.** That three-way fusion
(performative/Goodhart-robust ⊕ coalition-proof ⊕ the measure *is* the consensus object) is the
defensible novelty. The lineage above must be cited, not pretended-invented.

## 4. Status by property — demonstrated (✓) / designed (◐) / open (○)
- **(1) Nash** — ✓ honest contribution structurally rational (novelty→0, saturation, standing-gating;
  the gaming suite); ✓ honest self-report IC **proven** (`nash_honesty`: `p·b ≥ (1−p)·g`, 4 tests
  green). ◐ *conditional on the catch-probability `p`*: the layer that supplies a high `p` with no
  ground truth (peer-prediction, truthful report a BNE over peers' reports) is **designed, not built**
  (M3). Until M3, (1) for self-reporting is "proven modulo one component."
- **(2) Coalition-proof** — ✓ cyclic collusion (rings / mutual-citation / manufactured flow) zeroed by
  the HodgeRank harmonic certificate, wired to slash (demonstrated, tested). ○ the **symmetric-lie
  self-report collusion equilibrium** (everyone agrees on the same lie — the classic peer-prediction
  weakness) is killed only by a **designed** bonded + Bayesian-Truth-Serum information-score backstop
  (M4), not yet proven.
- **(3) Adaptive-stability** — ◐ learned-`v(S)` retraining harness wired; **data-blocked** (needs real
  DeepFunding outcome labels). ○ the **convergence theorem** (does `T` reach a *unique* un-gameable
  fixed point rather than oscillate?) is M2 — both unproven and load-bearing. See companion file.

**Therefore HCE today = a result for (1) + the hard half of (2); a labeled conjecture for the full
three properties.** It enters the whitepaper marked demonstrated-for-(1)+(2-cyclic),
designed-for-(2-self-report)+(3). It is **not** claimed as a finished theorem.

## 4b. Escaping the Cheng-Friedman Sybil impossibility (A3 — load-bearing for property 2)
Cheng & Friedman (*Sybilproof reputation mechanisms*, 2005) prove an impossibility: any reputation /
ranking mechanism satisfying a natural axiom set — crucially **symmetry / anonymity** (identities are
interchangeable; the mechanism sees only the graph, not who is who) — is **Sybil-attackable**: an agent
strictly gains by splitting into fresh pseudonyms. Every anonymity-respecting reputation system inherits
this wall (it is *why* Yuma needs an honest stake majority and why TraceRank's flat propagation is
Sybil-exposed).

**PoM escapes by relaxing exactly the anonymity axiom — structurally, not by patching.** Two mechanisms
make a fresh identity *worth zero by construction*:
1. **Commit-reveal timestamp-priority** — standing accrues to the *first* commitment covering a piece of
   work. A new identity has no history, so it cannot inherit or back-date priority; splitting produces
   pseudonyms that are all strictly later than the original.
2. **PoW-anchored (JUL) cost-of-identity** — standing is gated on a soulbound credential whose creation
   costs real PoW-backed value. A Sybil swarm pays the identity cost `N` times for no franchise gain
   (cost-of-identity ≠ 0 — the Mazorra–Della Penna 2023 condition; cf. Yokoo's false-name-proofness, 2000).

Because the mechanism conditions on *temporal priority* and *PoW-anchored identity cost*, it is **not
anonymous** — it distinguishes a standing-bearing identity from a fresh one — so the Cheng-Friedman
hypotheses do not hold and the impossibility does not bind. **This makes the JUL money-layer load-bearing
for coalition-proofness (property 2), not incidental:** the cost-of-identity anchor IS the Sybil defense.

> **Positioning line (whitepaper §9):** *"PoM escapes the Cheng-Friedman Sybil impossibility by relaxing
> anonymity: commit-reveal timestamp-priority on a PoW-anchored (JUL) identity makes a fresh identity
> structurally worth zero, so false names cannot inherit standing."*

Honest scope: this defeats *identity-multiplication* Sybils. It does NOT by itself defeat a single-identity
**self-report collusion** ring (everyone agrees on the same lie) — that is the separate M4 obligation (BTS
information-score backstop). Sybil-proofness (here) and collusion-proofness (M4) are distinct halves of
property (2).

## 5. Existence — stated honestly
- **Proposition (partial, holds today).** Under the guards `𝒢` with a fixed measure `v`, the honest
  profile satisfies (1) for contribution and self-report (given catch-probability `p` and bond
  `b ≥ (1−p)/p · g`) and (2) for all *cyclic* coalition deviations. [Proof: `nash_honesty` for the
  report IC; the novelty/saturation/standing guards for contribution; the HodgeRank residual for
  cyclic coalitions. All demonstrated in the suite.]
- **Conjecture (the full object).** The PoM game admits a profile that is simultaneously Nash,
  coalition-proof (including self-report collusion), and adaptive-stable — the HCE. Existence of the
  *adaptive* fixed point reduces to M2 (Brouwer gives existence under weak conditions; uniqueness /
  convergence is the open conditional theorem). Self-report collusion-proofness reduces to M4.

A named conjecture, openly labeled, is defensible. A named theorem without the theorem is not — this
file keeps the line on the defensible side.

## 6. Proof obligations → named structural enforcer (claim-needs-structural-enforcer)
| property | enforcer (exists / status) |
|---|---|
| (1) contribution | novelty + saturation + standing-gating — ✓ in `value` + gaming suite |
| (1) self-report | `nash_honesty` IC + bond — ✓; catch-prob `p` via PEG peer-elicitation, no oracle — **proven-in-template (A1)**, modulo graph-generalization of the determinant-MI score. See `DESIGN-peg-proof-template-for-hce.md`. |
| (2) cyclic | HodgeRank harmonic residual → `collusion_slash`/`unified_slash` — ✓ |
| (2) self-report collusion | SD-PP SD-truthfulness (any monotone utility) + bonded BTS — **proven-in-template (A1)**, modulo graph-generalization + C4 inner-uniqueness (M4). |
| (3) | learned-`v(S)` retrain-on-outcomes loop — ◐ designed, data-blocked; convergence — ○ (M2) |

## 7. Next
1. M2 convergence theorem (companion file) — the one open linchpin; do it cold, `/critical-qa`.
2. M1 → whitepaper: a short §, marked demonstrated-vs-designed per the §4 table; cite §3 lineage.
3. M3 (peer-prediction `p`) and M4 (collusion-eq elimination) are bounded builds that turn ◐→✓.
