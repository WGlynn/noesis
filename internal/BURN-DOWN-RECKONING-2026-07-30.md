# NOESIS BURN-DOWN RECKONING — the verdict
**2026-07-30. Adversarial first-principles pass. Every constant read from code this session or measured by running the probe, never asserted from memory. The job was to find straw, not defend the design.**

> Directive (Will): "burn noesis to the ground and find what remains." Method: for each load-bearing subsystem, ask if a from-zero honest rebuild would bring it back the same. Sort into METAL (irreducible), SCAFFOLDING (served its purpose, set down without shame), STRAW (unproven / accidental / not how you build a system).

## What I verified from the running system (not from docs)
- NCI mix `pow 0.10 / pos 0.30 / pom 0.60` — read at `node/src/lib.rs:3863` (`consensus::NCI`), Sol mirror `1000/3000/6000 BPS`.
- Finality mix `pow 0.0 / pos 1/3 / pom 2/3` — read at `runtime.rs:1607` (`FINALITY_MIX`); `FINALITY_MIX.pow == 0.0` asserted `runtime.rs:3094`.
- Anti-concentration `MIN_DIM_BPS = 5000` (50% per-dimension floor) — read at `runtime.rs:1619`; `dim_ok` at `:1621` requires `weight_for >= weight_all * 0.50`.
- Equivocation `finalizes_with_equivocation_guard` does slash-before-count (strips tainted weight from BOTH support set AND basis) — `runtime.rs`.
- `value_v8` pipeline: standing-gated seed -> semantic entropy floor -> Bradley-Terry outcome gate (AND-composed, can only lower) -> flow over canonical identity-quotient graph — `lib.rs:1384`.
- Builds clean: `cargo build --offline` exit 0. 339/339 tests stamped; 366 test fns in source.
- **The wash-building probe RUN (not read):** `cargo run --release --example wash_sim` -> every graph-internal discriminant BLIND (0.0%) on genuine-vs-wash-tree.
- Doc drift caught: `ARCHITECTURE.md` line pointers were stale (said `FINALITY_MIX@726`, actual 1607; `NCI@3705`, actual 3863). Values matched; only line numbers drifted as files grew. Minor, but logged: the canonical reference's own pointers are not hook-verified.

---

## THE LEDGER

### METAL — survives any honest rebuild
1. **Anti-concentration floor + AND-composition** (`dim_ok`, `MIN_DIM_BPS 5000`, `runtime.rs:1619`). The thesis in four lines. It is the ONLY thing that makes "capital cannot finalize without contribution's consent" true, and it is mirrored in Q32.32 fixed-point on RISC-V. Delete it and you rebuild it identically.
2. **PoW excluded from finality** (`FINALITY_MIX.pow = 0.0`). Reorgeable work kept off the safety path. Any honest rebuild reaches the same call; putting probabilistic work on the safety path would itself be the hazard.
3. **Soulbound PoM standing on `type_script.args`** (`lib.rs:143`, franchise rides the soulbound side, ownership rides `lock.args`). The mechanism that makes the franchise non-transferable. Real in code.
4. **The determinism wall -> `ValueOracle` seam -> train/inference partition.** Correct systems thinking. The wall is a FACT (neural inference is not bit-identical), not a preference. The expensive judge is off the hot path by construction; `v8` as Bradley-Terry (cheap aggregation) is defensible on-path.
5. **Equivocation slash-before-count** (`finalizes_with_equivocation_guard`). Genuine accountable-safety engineering a stateless weight predicate structurally cannot provide.
6. **HodgeRank cyclic-collusion detector** (HCE-2-cyclic, demonstrated). Detects circulation on topology alone in the same pass that prices flow. Confirmed by the probe: it catches the wash-RING (cycle_energy 4.0, circulation flagged).
7. **The honesty discipline itself** (STATUS-LEDGER; three recorded nulls; the 0% wash finding printed by the code's own probe). This is the most metal thing in the ash. A dishonest version of this project buries these. It is what lets the pattern survive its author.

### SCAFFOLDING — served its purpose, set down without shame
1. **The exact weights 60/30/10 and 1/3:2/3.** The code itself says WEIGHT != EFFECTIVE POWER (`ARCHITECTURE.md:56`); the floor does the work, not the ratios. Tunable, not load-bearing.
2. **The predictive learned-v(S) win.** NULL three times (round-1 single-repo, faithful set-level port, crates.io 299k-node deep-ancestry). The ledger already reframed it as upside, not load-bearing. Keep as research; it is not the moat.
3. **JUL exact economics** (Ergon fidelity, 50/block). Mechanism is built and tested (`jul.rs`: `reward_with_decay`, energy-peg assert) — richer than ARCHITECTURE's "designed, not built" claims; economics params are owed. Honest direction (rounds down).
4. **The v5/v6/v7/v8 layering as separate strata.** An accretion trail. Probably collapsible. By the project's own "essential complexity" standard, a 9,937-line `lib.rs` is an indictment to answer, not a badge.

### STRAW — unproven / accidental / "not how you build a system"
1. **Wash-building discernment (HCE-4) — the foundational straw, MEASURED.** Ran the probe: genuine vs competently-built acyclic wash = BLIND 0.0% on value_v8, novelty, synergy, cycle_energy, and circulation. The contribution measure — the object the ENTIRE anti-plutocracy thesis rests on — cannot distinguish four genuine minds from four sybil minds emitting distinct novel garbage in matched topology. It measures novel-flow-in-a-graph, which a competent adversary reproduces exactly. The probe's own honest verdict: closing it needs one of (a) a semantic learned v(S) [null on structural features], (b) an external personhood anchor [a capturable authority — kills the centerless thesis], or (c) realized external use [recurses + is a time-lock patient wash beats]. The periphery fix (`periphery_sim.rs`) is DESIGNED, not built, and it changes the base assumption.
2. **The un-gameable-learned-measure moat, as sold.** Internal tension the fire exposes: the DEMONSTRATED defense (253/253) is a FIXED structural rule set; the CLAIMED moat (the essay's own load-bearing line) is that "the only un-gameable measure is one that ADAPTS to the gaming." A fixed rule set is exactly the thing the essay says cannot be un-gameable. So the demonstrated moat and the claimed moat are different objects. Honest status: fixed defense holds against DEMONSTRATED vectors; adaptive un-gameability is unproven (and its cleanest instrument returned null 3x).
3. **Self-report IC + symmetric-lie collusion** (HCE-1-report, HCE-2-selfreport). Two named open theorems (graph-generalization TH-graph-MI, inner-uniqueness TH-C4-unique) plus a shared open bribery attack (ATK-bribery). The self-report Nash claim is CONDITIONAL on a catch-probability `p` supplied by a peer-elicitation layer (M3) that is not built.
4. **Complexity sprawl.** 9,937-line `lib.rs`, 3,140-line `runtime.rs`. It builds and tests, so this is disorganized-but-working, not broken — but the kernel is not yet disciplined into the small thing the project claims it is.
5. **"Aligned agents are the nodes, and the nodes are the chain."** Beautiful; labeled design-direction. Straw-as-spec: load-bearing to the pitch, absent from the code. The most narrative-dependent claim in the stack.

---

## THE META-QUESTION: "is this even how you build a system?"

Two honest halves.

**The chassis — consensus, finality, tokens, the on-VM mirror — yes, this is how you build a system.** Compiles clean, 339 tests green, the load-bearing invariant is four lines and mirrored in fixed-point on RISC-V, PoW correctly off the safety path, the determinism wall respected with a swappable oracle seam. Real systems engineering. Will's fear ("the design might be wrong") is NOT true of the chassis.

**The engine — un-gameable contribution measurement — is a research bet that has not paid out.** It rests on a problem measured at 0% separation today; its cleanest instrument returned null three times; its fix is designed-not-built and changes the base assumption from "graph-internal PoM" to "externally-anchored PoM."

One line: **Noesis is a well-built PoS+PoM chain wrapped around an unsolved research problem.** The chassis is metal. The engine is straw-until-proven. That is not "the design is wrong." It is "the systems design is sound, the core scientific claim is unproven, and the project is honest that it is unproven."

## THE UNCOMFORTABLE COROLLARY (what the fire actually reveals)
The floor's security argument is only as strong as the measure. "Capital cannot finalize without contribution's consent" assumes PoM is expensive to game. The probe shows a sophisticated wash-builder manufactures PoM standing at ~0% detection. So the floor converts "gaming OR capital" into "gaming AND capital" (`ARCHITECTURE.md:67`) — but if gaming the PoM dimension is cheap, the conjunction is thin protection. The code's own authors already wrote this down (`ARCHITECTURE.md:76`: "the residual — severe undetected gaming + cleared floor — is exactly what the moat closes; coupling is the bet that the moat wins"). The reckoning confirms it and states the true size: **the central security property is bounded by the unsolved measure.** The bet is explicit and honestly labeled. It is just not won.

## WHAT REMAINS STANDING IN THE ASH
Two things survive the fire clean:
1. **The separation-of-powers consensus** (floor + AND-composition + PoW-out-of-finality + soulbound franchise). It works today, tested, on-VM. It is a real anti-plutocracy mechanism EVEN WITH an imperfect measure, because the floor only needs PoM to be hard ENOUGH to game that capital cannot manufacture it for free — and even a mediocre measure raises that cost above zero. It just does not yet deliver the STRONG form the headline claims.
2. **The honesty discipline.** The ledger, the three nulls, the printed 0%, the never-round-up. This is what makes the whole thing rebuildable by someone else — the walkaway property.

Everything else is scaffolding around those two or a frontier that has to be earned. That is the irreducible Noesis, at its true size: a sound anti-plutocracy chassis, an honest culture, and one genuinely open problem it has correctly identified and not yet solved.
