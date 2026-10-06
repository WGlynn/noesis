# The wash / un-gameability problem — deep index (where we actually are)

> A single navigable tally of **every** approach in the local tree that addresses, or is being
> considered to address, the wash / un-gameability problem, each with an honest status and a
> `file:line` ground. Built 2026-10-06 from a full-repo relevance scan (197 files scored, weighted
> on the wash/sybil/independence/vesting/boundary term cluster). Status tags are the corpus convention: ✅ BUILT ·
> 🟡 DESIGNED-not-built · 🔬 OPEN. Never rounded up. Where a mechanism is partly each, it is split.
>
> This is a working map for us, not a contribution paper. The authoritative numbers live in
> `ARCHITECTURE.md` + source; this file only points at them.
>
> **Reproduction (2026-10-06):** the three sims were re-run today and match verbatim — `wash_sim`
> (124/124 BLIND, ring cycle-energy 4.0), `periphery_sim` (S=45, genuine vest 0.750 / wash 0.000,
> EV +19.35 / −36.00, break-even 9.00), `peer_prediction_sim` (genuine CA +0.245, ring −0.350,
> separation +0.595, semi-funded leak standing 0.250, crossover γ*=0.70). The crates.io figures (§2
> row 12) were verified against the cached result `data/crates/graph/periphery_grounding.json`; they
> were **not** re-run from the raw dump (the multi-GB `db-dump.tar.gz` is not present locally).

## 0. Honest floor (read this first)

- **What is genuinely solved:** the *dumb* wash (cyclic self-citation) and the *relabeling* class are
  caught by built, tested interior defenses. On **real crates.io data** the capital-independence gate
  *logic* correctly zeroes closed self-owned dependency rings (78.8% of reuse crates are closed; they
  vest 0.00 vs genuine 69.36) — the machine works **given a clean independence signal**.
- **What is NOT solved (the one sentence):** a competently-built *acyclic* wash — distinct keys, novel
  content, honest topology — is **structurally identical** to genuine collaboration; no worth-blind
  interior signal separates them (0.0% in `wash_sim`). The fix relocates measurement to the boundary
  (realized use by independent minds), which does not end the problem — it **recurses** (independence is
  the same question one hop down) and **raises its price**.
- **The load-bearing gap is a single data dependency**, not a missing idea: the independence signal we
  have demonstrated is *identity*-independence (Sybilable); turning it into *capital*-independence that
  actually costs money is unbuilt (the capital-cluster oracle), and the learned `v(S)` that would price
  the residual is OPEN and data-gated. Everything else composes around these two.

## 1. The problem, stated precisely

Interior value scoring is **worth-blind**: every graph-internal discriminant we ship (novelty, synergy,
cycle-energy, circulation) scores a genuine 4-mind collaboration and a topology-matched 4-identity wash
*identically* (novelty 124/124, synergy 124/124; `node/examples/wash_sim.rs`, 5% BLIND threshold at
`wash_sim.rs:123`). Genuine and wash differ **only in the worth of their content**, which no worth-blind
structural signal can see. Proof is an existence proof on a constructed matched case, not a
distributional claim. Spec: `docs/DESIGN-mind-scarcity-asymmetry.md`; test pin:
`node/tests/discernment.rs:8` (OPEN GAP — acyclic wash-tree is graph-internally indistinguishable).

## 2. The tally — every approach, with status and ground

| # | Approach | Layer | What it does against wash | Status | Ground (`file:line`) | Residual it leaves |
|---|---|---|---|---|---|---|
| 1 | **Cyclic interior defenses** (`cycle_energy`, `circulation`) | interior | catches a *dumb* wash-RING that closes a cycle | ✅ BUILT | `lib.rs` (`attribution_cycle_energy/_circulation`); demo `wash_sim.rs:111-118` | BLIND to acyclic wash-TREE |
| 2 | **`value_v5..v8` layered value fn** (novelty × flow_gate, seed gated on standing) | interior/value | v6 zeroes a multi-identity sybil ring (unvested child pumps nothing); composed multiplicatively | ✅ BUILT / deployed | `lib.rs:1178` (v6 gap CLOSED), `lib.rs:1908`, `value_v5..v8` | worth-blind on acyclic wash; `v8` outcome factor is a floor, not the moat |
| 3 | **Soulbound identity + one-time leaf + fresh-vest-0** | identity | a free-keygen sybil vests nothing | ✅ BUILT | `lib.rs:7306`, `node/src/rpc.rs`, `runtime.rs` | does not stop *funded* identities |
| 4 | **Capital-independent vesting gate (Layer A)** `independent_use_gate` | boundary | downstream use counts only from a *different* capital cluster; a closed ring shares one cluster → vests 0 | gate LOGIC ✅ BUILT+tested; capital-cluster ORACLE 🟡; consensus WIRING 🟡 | logic `lib.rs:7297-7351`, test `discernment.rs:84-112`; oracle unbuilt `lib.rs:7305`; one call site only | predicate is a bare `a != b`; a capital-splitting whale passes; leaks to a semi-funded ring |
| 5 | **Periphery EV model + Layer B (rent + slash)** | boundary/econ | Harberger carrying cost + challenge-slash make patient/closed wash negative-EV | EV model ✅ (1 input measured: S=45); rent/slash/window 🟡 params | `periphery_sim.rs`; closed-wash EV −36, genuine +19.35, break-even 9.00/id; test `discernment.rs:132-172` | sign gap assumes a slash asymmetry (`p_slash` 0.5 vs 0.02) that presupposes the discernment §1 says is impossible |
| 6 | **Dispute / slashing market** (bonded challenge → 2/3 vested-PoM verdict) | econ/authority | prices lying; the conviction authority that backs §5's slash | ✅ BUILT | `docs/DISPUTE-SLASHING.md`; reopening-trigger doc L33 | what *adjudicates* a challenge re-imports a judge (open assumption) |
| 7 | **Peer-prediction content signal** (Correlated-Agreement T1/T2) | boundary/content | CA separates genuine from collusion **only on capital-independent references**; stake makes truthful the unique surviving equilibrium | 🟡 DESIGNED; 🔬 2 open theorems | `peer_prediction_sim.rs`, `DESIGN-harberger-peer-prediction-theorems.md:141-153` | not built; candidate for the `v8` outcome factor, NOT a separate mechanism; closes only task-constant bias (γ* ≈ 0.70 coordination still wins) |
| 8 | **Layer C — learned `v(S)` on realized-use labels** (THE moat) | content/moat | train `v(S)` to predict external realized use; Layer-A vesting events = the named data source | 🔬 OPEN, data-gated | `DESIGN-periphery-solution.md:67-75`; `something-from-nothing…md:230` (NULL ×3 on structural data; ~0.60 on honest rich-feature labels) | ~0.60 is honest-label prediction, **not** adversarial un-gameability |
| 9 | **Isomorphism-invariance gate for `v(S)`** | content/guard | require `v(S)` invariant under structure-preserving relabelings | 🔬 OPEN (cand-A, hard) | `ROADMAP.md:162`, `docs/ISOMORPHISM-INVARIANCE-VS.md` | unbuilt; gates the moat before it can ship |
| 10 | **Solver / legitimacy guard** (Hacker-Fixer-Solver) | content/guard | every `v(S)` claim must clear a solver guard | 🟡 DESIGNED (cand-B, cheap) | `ROADMAP.md:170` | unbuilt |
| 11 | **`v(S)`-refresh cron + anti-recycling "seen-vectors" memory** (GVU) | content/guard | deny recycling known-good vectors | 🟡 DESIGNED (cand-C) | `ROADMAP.md:173` | unbuilt |
| 12 | **Crates.io real-data grounding** | empirical | runs the *built* Layer-A rule over 1.94M real reverse-dep edges; zeroes real closed rings in the wild | ✅ DEMONSTRATED on real data **given an identity-independence signal** | `data/crates/RESULTS-PERIPHERY.md`, `ROADMAP.md:38-53` (111,689 crates; genuine 69.36 vs closed 0.00; strips 15.4% self-inflation) | 🔬 independence used was *identity* (Sybilable GitHub owner), **not capital**; no adaptive adversary |
| 13 | **Attestation-exogeneity reframe** | theory | wash-building is not a NEW problem — it re-expresses two existing ones; the anchor cannot be built from endogenous data (class-level result) | analysis / design | `docs/DESIGN-attestation-exogeneity.md:104-105` | states *why* an exogenous signal is required; does not supply one |
| 14 | **Re-opening trigger** (clawback / self-healing, two-tier S1/S2) | consensus-adjacent | re-open a finalized cell later revealed as wash; Tier 1 interior residual, Tier 2 boundary/realized-use | 🟡 DESIGNED-not-built, Will-gated | `docs/DESIGN-reopening-trigger-bio-priors-ungameable-game.md`; `docs/CLAWBACK-CASCADE-SELF-HEALING.md` | Tier 2 depends on the unbuilt capital-cluster oracle; adjudicator open |
| 15 | **v0 Sybil failure envelope + bootstrap admission** | bootstrap | honest map of the *deployed* v0 (farmable; captured share ≈ F/(N+F)); per-identity cap + initial allowlist as the load-bearing brakes | ✅ measured RED; allowlist 🟡 | `docs/research/v0-sybil-failure-envelope-2026-07-19.md`, `docs/SYBIL-SURFACE-deployed-franchise-2026-07-19.md`, `docs/DESIGN-bootstrap-admission.md` | admission control is an *imported* authority (honest) |
| 16 | **Vested-certifier-endorsing-garbage gap** (the built system's own pin) | residual | names the real residual: a *vested* identity certifying junk | 🔬 OPEN, pinned | `lib.rs:4608` (`adversary::vested_certifier_endorsing_garbage_open_gap`) | closed only by #8 (learned `v(S)` on real labels) |

## 3. Status rollup

- **✅ BUILT and doing real work:** interior defenses (#1,2), soulbound + fresh-vest-0 (#3), Layer-A gate
  *logic* (#4), dispute/slash market (#6), the crates.io real-data demonstration of #4's rule (#12).
- **🟡 DESIGNED-not-built:** Layer-A capital-cluster **oracle** + consensus wiring (#4), Layer-B rent as
  a live parameter set (#5), peer-prediction/CA wrapper (#7), solver guard (#10), seen-vectors
  refresh (#11), re-opening trigger (#14), bootstrap allowlist (#15).
- **🔬 OPEN (genuinely unsolved):** learned `v(S)` on real realized-use labels (#8 — THE moat),
  isomorphism-invariance gate (#9), the vested-certifier-garbage residual (#16), and the recursion base
  case (independence verified by the signal it secures).

## 4. The single load-bearing open, and its dependency chain

Every open item funnels into **one** place. The crates.io run (#12) proved the *machine* works given a
clean independence signal. So the whole remaining moat is making that signal **cost capital, not a free
identity**:

```
capital-cluster oracle (#4, unbuilt)  ──┐
                                        ├──▶ a real capital-independence signal
Layer-A vesting events (#4, built)  ────┘        │
                                                 ▼
                                   learned v(S) on realized-use labels (#8, OPEN, data-gated)
                                                 │  gated before ship by:
                                                 ├── isomorphism-invariance (#9, OPEN)
                                                 ├── solver/legitimacy guard (#10)
                                                 └── seen-vectors anti-recycle (#11)
                                                 ▼
                              closes the vested-certifier-garbage residual (#16)
```

Peer-prediction (#7) is **not** a parallel escape hatch — it reconciled to a *candidate for the `v8`
outcome factor*, i.e. another floor feeding the same `v(S)` seat, and it too needs the independence
signal (its T1 holds only on capital-independent references).

## 5. Artifact map (navigable index, grouped by role)

Top of a 197-file relevance scan. Read-order within each group is by signal.

**Primary mechanism (code):**
- `node/src/lib.rs` — value layer, `independent_use_gate` (`:7297-7351`), v5..v8, the pinned residuals.
- `node/src/runtime.rs` — vesting cliff / finality fence; where wash containment meets consensus.
- `node/examples/wash_sim.rs` — the 0.0% interior-blindness proof (the problem, numeric).
- `node/examples/periphery_sim.rs` — the boundary EV receipt (closed-wash −36).
- `node/examples/peer_prediction_sim.rs` — CA T1/T2 numeric + the semi-funded-ring leak correction.
- `node/tests/discernment.rs`, `node/tests/gaming.rs` — the pins that fail if a claim regresses.

**Design / theory (docs):**
- `docs/DESIGN-mind-scarcity-asymmetry.md` — the base case (why interior is worth-blind).
- `docs/DESIGN-periphery-solution.md` — Layers A/B/C, the canonical layer decomposition.
- `docs/DESIGN-harberger-peer-prediction-theorems.md` — #7, the two open theorems.
- `docs/DESIGN-attestation-exogeneity.md` — #13, the class-level "why exogenous".
- `docs/DESIGN-reopening-trigger-bio-priors-ungameable-game.md`, `docs/CLAWBACK-CASCADE-SELF-HEALING.md` — #14.
- `docs/research/boundary-measurement-ungameable-game.md` — the honest synthesis of §1-§4 here.
- `docs/research/something-from-nothing-oracle-free-content-value.md` — the oracle-free content-value stack + the NULL×3.

**Calibration / adversarial (docs):**
- `docs/CALIBRATION-ci-argument-2026-07-21.md`, `docs/CALIBRATION-backstop-conjunctive-2026-07-21.md` — what capital-independence does and does **not** close.
- `docs/research/v0-sybil-failure-envelope-2026-07-19.md`, `docs/SYBIL-SURFACE-deployed-franchise-2026-07-19.md` — #15, the deployed RED.
- `docs/ISOMORPHISM-INVARIANCE-VS.md` — #9.

**Empirical (data):**
- `data/crates/RESULTS-PERIPHERY.md` + `data/crates/periphery_grounding.py` — #12, the real-data demonstration.

**Status / ledger (internal):**
- `ROADMAP.md` (the live loop log), `internal/CONTINUE.md` (crown-jewel framing), `internal/STATUS-LEDGER.md`, `ARCHITECTURE.md`.

## 6. One line

We have **built** everything that prices the *closed* wash to a loss and **proven on real data** that it
works given a clean independence signal; what is **open** is the single thing that makes that signal
honest — a capital-rooted independence oracle feeding a learned `v(S)` on realized-use labels — and every
other open item is a guard on *that*. We are not missing a mechanism; we are missing one exogenous,
cost-bearing signal and the data to train against it.
