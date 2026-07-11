# SPEC — Noesis dynamic block logistics (size + interval) under the PoM/PoW/PoS hybrid

> **STATUS: DESIGN ONLY — nothing in this spec is built.** Verified 2026-07-10 against `node/src/lib.rs`, `node/src/runtime.rs`, `ARCHITECTURE.md`, `docs/POM-FINALITY-TEMPORALITY.md`, and `internal/DESIGN-block-logistics.md`. Grep of `node/src` confirms there is **no** block-size cap, block-interval, difficulty adjustment, cumulative-work clock, orphan/uncle record, or emission module today; the **only** resource bound in code is `Constitution.max_mempool = 10_000` (`runtime.rs:78`, DoS Bound A). This spec locks the FORM of the mechanism; every numeric constant is deliberately unset (see §9) because asserting a value would violate the no-assert-from-memory rule and there is no code line to read.
>
> **Provenance:** synthesized from three gated candidate designs. Base chassis = Candidate A (safety-first min-controller), the only candidate that survived the safety-path/selfish-mining gate. Grafts and one deletion applied per the convergent gate fix-hints. Two breaks survive in all candidates and are stated honestly in §8, not papered over.

## 0. Grounding (constants re-read at source this session)

Every constant below was opened and read at its real `file:line`. **The #1 trap held:** `ARCHITECTURE.md` (and the task prompt) cite `NCI` at `lib.rs:3289`; that pointer is **stale**. The real definition is at **`lib.rs:3705`** (values match). All candidates caught this independently; I confirmed it.

| Constant | Value | Real location |
|---|---|---|
| `NCI` overall consensus mix | `pow 0.10 / pos 0.30 / pom 0.60` | `lib.rs:3705` (NOT 3289) |
| `TWO_THIRDS_BPS` supermajority | `6667` | `lib.rs:3707` |
| `FINALITY_MIX` (PoW excluded) | `pow 0.0 / pos 1/3 / pom 2/3` | `runtime.rs:584-588` |
| `MIN_DIM_BPS` anti-concentration floor | `5000` (≥50% per dimension, CONSTITUTIONAL / not governance-tunable) | `runtime.rs:596` |
| `dim_ok` (per-dimension **weight** check) | `weight_for ≥ weight_all·5000/10000` | `runtime.rs:598-601` |
| `finalizes_pos_pom` (safety predicate) | 2/3 bar AND both dims clear `dim_ok` | `runtime.rs:608-633` |
| `MIN_STAKE` registration floor | `100.0` | `lib.rs:3813` |
| `max_sybils` = `floor(capital / MIN_STAKE)`; **identity split is weight-neutral** | — | `lib.rs:3825`, `lib.rs:4107` |
| Only slashable fault = **equivocation** (double-vote); withholding is "no *objective* fault", not slashable | — | `lib.rs:3868`; `ARCHITECTURE.md` §finality |
| Ordering = commit/slice position, **NOT** timestamp | — | `lib.rs:765` |
| Only current resource bound | `max_mempool = 10_000` | `runtime.rs:78` |

**Load-bearing facts that shape every rule below:**
- **A produced-block orphan is NOT a safety event** — PoW is excluded from finality (`FINALITY_MIX.pow = 0.0`, `runtime.rs:585`); finality selects canonical via `finalizes_pos_pom` regardless of PoW orphans (`DESIGN-block-logistics.md` §1). ⇒ Block size/interval live on the **production clock**; a mis-set interval wastes JUL + grants producer-griefing surface, it does not (directly) break safety.
- **The keystone** (`POM-FINALITY-TEMPORALITY.md` §12-19): a block is finalized by the franchise that existed *before* it; contributions never vote on their own block. Safety = bonded stake (instantaneous, ungameable-at-`t`); influence+reward = PoM (a stake-like account clearing in arrears after vesting window `W`). **The gameable instant-novelty face must NEVER be a controller input on the safety path.**
- **`dim_ok` is a WEIGHT floor, not a breadth floor.** A single validator holding ≥50% of a dimension's summed weight satisfies it (confirmed against the test at `runtime.rs:1545-1558`: `FINALITY_MIX.pow==0`, `MIN_DIM_BPS==5000`, and a 1-per-dimension set finalizes). This is why the mind-diversity guardrail cannot be built on `dim_ok` and cannot be a raw participant count (see §5 and §8).

## 1. Design posture (which candidate won, and why)

Locked posture = **safety-first min-controller** (Candidate A chassis), because it is the only design where an adversary inflating orphans or lag can only push production difficulty **up** (longer/safer interval), never down — so NC-Max all-blocks-in-difficulty selfish-mining resistance is genuinely preserved. Candidates B and C both key difficulty on the orphan *rate* directly (`D ∝ (o_hat/o*)^γ`), which is the exact coupling NC-Max removes, and both add novel machinery (B's single hard finality HALT; C's "grow the epoch to drain backlog" term) that their own gates rate `breaks` for manufacturing new minority-triggerable attack surfaces. This spec keeps A's chassis and applies the fixes the gates converged on.

## 2. The three clocks (respect all three)

Per `DESIGN-block-logistics.md` §3:

| Clock | Driven by | Binds on | Signal |
|---|---|---|---|
| **Production** (mint cadence + size) | PoW | propagation bandwidth | orphan rate (NC-Max) |
| **Finality** (confirm cadence) | PoS + PoM | v(S)-eval compute + finalizer breadth | finality lag; anti-concentration health |
| **Vesting `W`** (realized value clears into franchise) | temporality model | fraud-catch vs responsiveness | dispute-window closure |

**Decision 3 (decoupled):** production runs fast micro-blocks; finality batches them into checkpoints (`finalizes_pos_pom`) on a separate cadence. **Decision 4 (cumulative-work clock):** `W`, franchise decay, and dispute windows are denominated in **cumulative PoW work**, not block-height and not wall-clock — mandatory because ordering is commit-position not timestamp (`lib.rs:765`) and a floating interval would make any block-height/wall-clock window wobble in real time (`DESIGN-block-logistics.md` §4D).

## 3. Actuation: difficulty only (one knob, JUL folds in)

The interval controller moves **difficulty `D`** only; interval is emergent from `D` against realized hashrate, exactly as Bitcoin/NC-Max — **no magic block-time constant is introduced.** JUL emission is `R_block ∝ D` (Ergon-style, reward∝difficulty; `DESIGN-elastic-pow-money.md`), so emission-per-unit-work stays constant as interval floats and the interval controller **is** the emission controller — Decision 6 satisfied with **no second money knob**. (Joint-stability with the money layer's three-timescale TSS is an open item, §9.)

## 4. INTERVAL controller (production clock) — two signals, conservative max-combine

Two proportional-with-damping arms on the cumulative-work clock; **take the max difficulty (⇒ the longer, safer interval).** All arithmetic is integer/fixed-point (the codebase already uses Q16.16, e.g. `theta_sim_q16`) to guarantee bit-identical replica evaluation.

**Controller A — orphan rate (NC-Max, producer breadth):**
```
D_A = D · clamp( 1 + g_A·(r_orphan − r*)/r* ,  1/s, s )
```
`r_orphan > r*` ⇒ `D_A` up ⇒ interval lengthens. **Difficulty is computed over ALL validly-produced blocks incl. orphans** (NC-Max move 3) so inflating orphans cannot *lower* difficulty (selfish-mining resistance). **GRAFT / FIX (determinism):** `r_orphan` is a pure function of canonical state ONLY via **uncle-inclusion** — each canonical block MUST embed the headers of the orphans it observed at its height, with a bounded inclusion window; `r_orphan` is then computed over the uncle-inclusive canonical chain, never over a gossip-relative view. Without this, orphan-set membership is producer/network-choosable (gate 4 break). This is CKB NC-Max's actual mechanism, adopted literally, not merely cited.

**Controller B — finality lag (the safety arm), FIXED to be minority-robust:**
```
D_B = D · clamp( 1 + g_B·(L_fin − L*)/L* ,  1/s, s ),   g_B > g_A
```
`L_fin > L*` (production outrunning finalization) ⇒ `D_B` up until finality catches up.

**GRAFT / FIX (capture-resistance — the central break):** `L_fin` is derived from the **FINALIZABLE set, not the realized/finalized set.** Concretely, `L_fin` = cumulative-work distance from the production tip to the most recent checkpoint that a **fully-participating eligible validator set WOULD finalize given propagation physics**, NOT the distance to the last checkpoint that actually cleared `finalizes_pos_pom`. Reason (grounded): withholding a co-signature is **costless** — only `is_equivocation` is slashable (`lib.rs:3868`) and `ARCHITECTURE.md` explicitly rules an honest staker who withholds "committed no *objective* fault" and cannot be fairly slashed. If `L_fin` reads the *realized* finalized frontier, a >⅓-of-one-dimension minority (enough to deny the 2/3 bar or fail `dim_ok`, `runtime.rs:601-606`) holds `L_fin` high at zero cost and, because `g_B > g_A` makes `D_B` binding, throttles the entire honest production layer — a cross-power capture (PoS/PoM cartel throttles PoW, pricing out small miners) with no hashpower spent. Basing `L_fin` on the finalizable set removes the minority's ability to inflate the signal.

**Additionally:** a finality stall routes to the existing **liveness path** (attributable non-signers), not silently onto the difficulty knob. `D_B`'s authority is bounded by its own `clamp(·, 1/s, s)` and the lag must persist past a cumulative-work dispute window before it moves production.

**Conservative combine + governance band:**
```
D_next = clamp( max(D_A, D_B),  D_floor, D_ceil )
```
`max()` ⇒ production difficulty is always ≥ the finality-lag arm's demand ⇒ production is structurally forbidden from outrunning finality, the strongest form of "safety-path never subordinated to throughput." `[D_floor, D_ceil]` is the VIBE-governed outer band (Decision 5), verifier-gated, bounded, nothing zeroable (mirrors the value-matrix bounded-mutation rule).

**Interval floor (correctness bound, `DESIGN-block-logistics.md` §4C):** `D_floor` is additionally lower-bounded so the interval can never fall below the commit→reveal propagation buffer `T_reveal`. Pushing below `T_reveal` would grief honest revealers and let a well-connected producer manipulate temporal-novelty pricing (which rests on commit ORDER, `lib.rs:765`).

## 5. SIZE controller (production clock) — min of two ceilings, breadth-gated growth

```
MaxBlockSize_next = floor( min( B_cap ,  α · C_budget / c_per_cell ) )
```
- `B_cap` = size the current propagation headroom supports within the reveal window (reveal window ≥ propagation, §4C).
- `C_budget` = the finalizer-set v(S)-compute ceiling.
- **GRAFT / FIX (determinism):** `c_per_cell` is read from a **DECLARED integer/rational cost-model over block content**, never a measured runtime:
  ```
  C_eval(n, E) = a1·n + a2·n·log(n) + a3·E      (a1,a2,a3 = constitutional cost coefficients)
  ```
  where `n` = #contributions, `E` = #attribution edges. This makes `C_eval` bit-identical across heterogeneous hardware and forecloses the "understate compute on fast hardware / CG-friendly topology to inflate the size ceiling" attack (gate 4 break in candidate A, which used *measured* `C_eval`). The actual v(S) solve (Myerson sampling, HodgeRank Laplacian, `attribution_circulation`/`attribution_cycle_energy`) is separate finality *work*; only the declared cost-model gates the size decision. If the real superlinear curve is worse than `n·log(n)`, the model under-protects — flagged §9.
- `α` = safety margin `< 1`, keeping realized eval load well below the ceiling so modestly-provisioned independent agent-operators stay in the finalizer set.

**GRAFT / FIX + DELETION (anti-concentration — the second central break):** Candidate A's live-quorum margin override (`m_pos`, `m_pos` of *whoever voted*) is **DELETED** — gate 3 showed it is (a) the wrong quantity (weight, not breadth; a 1-per-dimension monopoly passes), (b) survivorship-biased (measured over voters who already survived the thinning it should prevent), and (c) minority-pinnable. It is replaced by a **breadth gate on SIZE GROWTH, keyed to the ELIGIBLE registered set sampled BEFORE the block is sized:**

```
grow size only if  M_pom ≥ N_min          (M_pom = count of Sybil-DISCOUNTED
                                            independent PoM contributors, per §8 Boundary 2)
```
where `M_pom` is the collusion-discounted contributor count (a ring / near-dup cluster ⇒ ~1), **NOT effective weight** (which is Sybil-inflatable — superseded by Will's 2026-07-10 ruling). The gate is on the **PoM** dimension only; capital is deliberately Sybil-permissive (§8 Boundary 2). Growth per epoch is step-capped by `s`. Size **freezes** (and may step down) when `M_pom < N_min`. This makes "how many *genuinely-independent minds* can still afford the current block" a *pre-sizing precondition*, anchored in un-fakeable earned provenance rather than splittable weight.

`C_budget` = a p-quantile over the **eligible** set (or a governance floor), **never** an adaptive p-quantile over live *survivors* — killing the centralization ratchet (survivors drop out ⇒ ceiling rises ⇒ next tier priced out) that gate 3 broke candidate A on.

**All structural constants** (`r*`, `L*`, `g_A`, `g_B`, `s`, `α`, `N_min`, `a1..a3`) are automatic (Decision 5); VIBE moves only the verifier-gated outer band `[D_floor, D_ceil]`, `[MinSize, MaxSize]`.

## 6. Invariant handling (what holds, verified)

- **Safety path (HOLDS):** no controller is an argument to `finalizes_pos_pom` (`runtime.rs:608`), `FINALITY_MIX`, or `TWO_THIRDS_BPS`. Every controller input is physical (uncle-inclusive work, propagation headroom, declared `C_eval`) or the finalizable-set lag — never the live retroactive v(S) score, never instant-novelty. Keystone intact: finality still reads the pre-existing bonded+vested franchise.
- **Selfish-mining (HOLDS):** Controller A ports all-blocks-in-difficulty over the uncle-inclusive chain; `max(D_A, D_B)` means adversarial orphan/lag inflation can only raise difficulty, never lower it.
- **Determinism (HOLDS after grafts):** `r_orphan` from uncle-inclusive canonical state; `L_fin` from the finalizable set over the same replicated validator vectors; `C_eval` from the declared integer cost-model; `Neff` from eligible-set effective weights. All fixed-point. The one caveat: the v(S) solve itself must be proven bit-identical on the finalization VM before any part of its output feeds consensus (§9).
- **Capture-resistance (HOLDS, via §8 Boundary 1 ruling):** the forward direction is closed (miners can't bloat size; capital can't raise the bar; VIBE touches only bounded outer limits). The finalizable-set `L_fin` fix closes the throttle-weaponization; the costless-withholding *stall* residual is resolved by the §8 heartbeat-deactivation ruling (silence → deactivated not slashed, bounded by the grace window).
- **Anti-concentration (HOLDS in shape, via §8 Boundary 2 ruling; strength tracks the moat):** the mind-diversity guarantee moves off block-logistics onto a soulbound-PoM contributor-count floor Sybil-discounted by the existing collusion detectors; the declared cost-model + non-adaptive `C_budget` still remove the survivorship ratchet at the sizing layer. Guarantee strength is coupled to `v(S)` un-gameability (§8).

## 7. Lean check

Adds exactly: one finality-lag arm + one breadth-gated growth precondition beyond NC-Max, one declared cost-model, one uncle-inclusion rule. **Deletes** candidate A's live-quorum override. Reuses `finalizes_pos_pom`, `dim_ok`, the commit-reveal two-step, and Q16.16 fixed-point already in the tree. Introduces **no magic block-time constant** (interval emergent from difficulty) and **no second money knob** (JUL folds into `D`). Earns its place under the Bitcoin-simple constraint.

## 8. HONEST BOUNDARY — the two convergent gate-breaks, BOTH RESOLVED (Will ruled 2026-07-10)

The gate found two breaks in all three candidates. Neither was a finality-forgery (the keystone and 2/3 bar hold throughout); both were capture/liveness/anti-plutocracy limits block-logistics alone couldn't close. Will ruled on both 2026-07-10, and both resolve by **leaning on mechanisms the stack already has** — VibeSwap's heartbeat-deactivation + commit-reveal slashing (Boundary 1), and soulbound PoM as the Sybil anchor (Boundary 2). Each carries an honest residual/ceiling, stated below, not papered over.

**BOUNDARY 1 — Costless finality-withholding — RESOLVED (Will ruled 2026-07-10) by porting a mechanism already built and tested in VibeSwap.** The earlier framing ("needs an inactivity-leak that contradicts 'no objective fault' ⇒ re-ratify") was wrong. Our own `NakamotoConsensusInfinity.sol` already solves this *without punishing silence*, and it composes with the temporality ruling rather than contradicting it. The resolution, ported:
- **Silence is *deactivated*, not slashed.** A non-participating validator is removed from the active-weight set after a grace window (VibeSwap: `HEARTBEAT_INTERVAL 24h` / `HEARTBEAT_GRACE 48h`, `contracts/consensus/NakamotoConsensusInfinity.sol:556-573`, `_checkHeartbeats`), and **reactivation is free** (`heartbeat()` re-adds `totalActiveWeight`). Deactivation is not a penalty ⇒ *consistent* with the "no objective fault" ruling, so no re-ratification needed. A withholding minority stops counting toward the 2/3 denominator + `dim_ok` after grace ⇒ finality re-bases on the live set and proceeds; the stall is bounded by the **grace window**, not by slow franchise decay.
- **The affirmative-default half is already slashable.** Commit-then-no-reveal / invalid-reveal is a *provable broken promise* (you opted in, then defaulted) — slashed 50% in VibeSwap (`contracts/core/CommitRevealAuction.sol` `SLASH_RATE_BPS = 5000`, `_slashCommitment`), and the Noesis whitepaper already declares commit-then-no-reveal slashable. So the part of "withholding" that IS a provable fault is covered; only *pure absence* gets costless deactivation. **The unifying principle (from VibeSwap): slash only a provable fault on something affirmatively DONE (equivocation, or defaulting on a voluntary commitment); mere absence costs your *seat*, not your stake.**
- **Denominator-shrink guarded by the quorum floor.** Deactivation shrinks the active denominator; the hybrid quorum-floor mechanism EXISTS in `consensus::finalizes_hybrid` (floor param, tested at 3333/5000 BPS) — `finalize ⇔ W_for ≥ ⅔·max(W_eff, Q)` — which prevents the remaining set from finalizing a *minority* (safe halt instead if honest participation < Q). Composition sequence: withhold → stall for ≤ grace → deactivated → finality resumes on the live set if ≥ floor, else safe halt. **No free indefinite stall AND no minority-finalization.** *Composition check owed:* confirm the live `finalizes_pos_pom` path wires a **nonzero Q** (CONTINUE.md notes a `quorum_floor=0` hardcode in the fixed mirror — verify before relying on the floor).

**Honest residual (design detail, §9 — NOT a blocking boundary):** VibeSwap's trigger is a bare *heartbeat* ping, which de-weaponizes the fully-OFFLINE withholder but not an *online-but-abstaining* one (heartbeat while never voting ⇒ stays "active" and counted). For a finality gadget the deactivation trigger must therefore be **missed finalization-participation**, not a bare ping — which also catches the online abstainer while staying blameless (no slash, free reactivation on next vote). Honest disagreement (voting for a competing *valid* proposal) is participation and is untouched; only non-voting is deactivatable. The exact "participation" predicate + grace-window length are calibration, not a new principle.

**BOUNDARY 2 — Sybil-robust mind-diversity — RESOLVED (Will ruled 2026-07-10): anchor breadth in soulbound PoM, leave capital deliberately Sybil-permissive.** Soulbound PoM standing was designed as the primary Sybil defense; the breadth floor uses it for its designed purpose (Will: *"soulbound was designed as the be-all end-all of Sybil attacks; lean on it"*). The resolution:
- **Don't Sybil-harden capital — it's the wrong dimension.** Splitting stake is free (`max_sybils = floor(capital/MIN_STAKE)`, `lib.rs:3825`; identity split weight-neutral, `lib.rs:4107`) and always will be. But capital is already power-capped (cannot finalize without PoM clearing its own floor), so capital-Sybil is *free but harmless* — it inflates the apparent breadth of a dimension that never protected anti-plutocracy. Enforce nothing there.
- **Put the breadth floor on PoM, discounted by the EXISTING collusion machinery.** Finality precondition = **≥ N Sybil-discounted independent PoM contributors**, where the discount REUSES the built detectors — `attribution_circulation` + `attribution_cycle_energy` (HodgeRank harmonic), the `θ_sim` near-dup floor, and the `μ^m`/`λ^r`/`ρ^j` damping — the same machinery that already collapses a ring's *value* to ~1, now collapsing its *breadth count* to ~1. Faking N then costs N genuinely-independent bodies of valued work: the one thing structurally scarce to an AI. *Rationale (AI-for-AI): agents Sybil at machine speed ⇒ detect-and-slow loses the race ⇒ ENFORCE, not detect; and the only un-fakeable notion of an independent mind is an independent provenance of valued work — you can fake identities, you cannot fake provenance.* This SUPERSEDES the earlier effective-weight `Neff` idea (§5 updated): effective weight is Sybil-inflatable; a collusion-discounted contributor COUNT is not.
- **Accepted as a consensus change with its scope:** a new finality precondition on `finalizes_pos_pom`, which **phases in as PoM vests** (PoS bootstraps safety at genesis per `POM-FINALITY-TEMPORALITY.md`), and dropping below the floor **safe-halts** (never finalize a thin set) — same discipline as the quorum floor.

**Honest ceiling (what ratifying this accepts):** the *strength* of the breadth guarantee tracks the moat's maturity. The detectors catch **demonstrated** Sybil/collusion vectors today (mutual-citation 2-cycles, near-dups, coordinated volume); the general isomorphism-invariance gate, directed-k-cycle detection, and learned-`v(S)`-on-real-labels remain 🔬 open / data-blocked. So mind-diversity is **not a separate problem with its own risk — it dissolves into the moat** and matures exactly as `v(S)` un-gameability matures. Ratifying this ratifies that coupling: real for demonstrated vectors, strengthening with the core, never claimed as a finished proof.

**Consequence, stated plainly:** this spec makes block logistics *safe* (no gameable input on the safety path, selfish-mining-resistant, deterministic after the uncle-inclusion/declared-cost grafts) and *capture-resistant in form*. It does **not**, on its own, guarantee the anti-plutocracy / mind-diversity property under an adversary who (a) withholds finality for free or (b) Sybil-inflates breadth — because those levers live in the *finality/identity* protocol, not the block-logistics controller. Those two are the honest frontier.

## 9. Residual open questions (calibration + unbuilt state)

1. **Numeric calibration of ALL structural constants** (`r*`, `L*`, `g_A`, `g_B`, `s`, `α`, `N_min`, `a1..a3`, `D_floor/D_ceil`, `T_reveal`). None exist in code; each must be derived from MEASURED propagation latency and MEASURED v(S)-eval cost curves (`DESIGN-block-logistics.md` §5). Asserting any value now would violate no-assert-from-memory. The spec fixes the FORM; magnitudes are an explicit calibration task.
2. **Cumulative-work clock unit is unspecified.** The finality path takes an abstract `now/horizon` counter (`runtime.rs:611-613`) for franchise decay; concretizing that AS cumulative work and re-denominating `W`, decay, and dispute windows onto it (Decision 4) is a dedicated tick. Also: do production HALT/throttle periods freeze `W`'s clearing, and is that correct or exploitable?
3. **Uncle-inclusion must be fully specified before `r_orphan` is trusted:** bounded uncle count per block, orphan refs verified against the all-blocks work-ledger, reference set derived from consensus not producer discretion — plus the DoS surface of the NEW produced-DAG + work-ledger state (attacker-writable; today only `max_mempool` bounds anything) must be bounded before build.
4. **v(S)-solve cross-replica determinism is asserted, not proven.** `value_v5..v8` uses f64 (`attribution_cycle_energy` returns f64; HodgeRank = weighted-least-squares Laplacian solve with early-exit CG). The DECLARED cost-model gates the size decision safely, but the actual v(S) work that PRODUCES standing must be proven bit-identical on the finalization VM (fixed soft-float / seeded RNG / fixed CG iteration count) before its output feeds consensus.
5. **Declared cost-model superlinearity:** is `n·log(n)` the right curve for HodgeRank + collusion detectors, or steeper? If steeper, `C_eval` under-protects and size-clamp-passing blocks can still stall finality. Measure against the built value path, then freeze coefficients as constitutional constants.
6. **Production/finality cadence ratio (Decision 3):** micro-block size and finality-epoch batching factor. A larger epoch batch raises the natural `L_fin` baseline ⇒ micro:epoch ratio needs joint tuning with `L*`.
7. **JUL coupling joint-stability (Decision 6):** the interval controller (moving `D`) and the money layer's TSS (`k(t)` decay + elastic rebase + 120d PI, `DESIGN-elastic-pow-money.md`) both touch difficulty-adjacent quantities. "The interval controller IS the emission controller" must be JOINTLY DERIVED — confirm the interval controller's damping frequency is separated from the TSS's three timescales so they compose rather than beat.
8. **Cross-controller oscillation:** the interval floor `T_reveal` depends on propagation time, which depends partly on epoch SIZE (§5 output) — so §4's floor depends on §5, a loop that could oscillate. The step-caps (`s`) + finality-lag arm are the intended dampers; a stability/damping proof is owed.
9. **BOUNDARY 1 formal scope:** exactly how long can a >⅓-of-one-dimension costless-withholding cartel stall finality (bounded only by franchise-decay horizon), and does the produced-but-unfinalized backlog create a griefing / wasted-JUL surface large enough to matter economically (not a safety event)? Needs the money layer worked jointly.
10. **BOUNDARY 2 resolution path:** either (a) a Sybil-cost-weighted breadth metric with proven cost-to-fork-a-soulbound-PoM-identity > marginal breadth gain, or (b) a real minimum-effective-participants floor in `finalizes_pos_pom` (a consensus change with its own DoS/liveness analysis). Both out of block-logistics scope, require separate ratification; until one lands, mind-diversity under max throughput is approximated (visible + growth-gated), not enforced.
11. **`POM-FINALITY-TEMPORALITY.md` §76-86 open decision** (decouple PoM from finality safety, or keep the anti-concentration-hedged coupling). This spec assumes the CURRENTLY-BUILT coupled finality per `ARCHITECTURE.md`'s 2026-06-29 ruling. If PoM were decoupled from safety, `L_fin` becomes a PoS-only lag and the breadth gate moves to the reward/influence layer — both would need re-deriving.
