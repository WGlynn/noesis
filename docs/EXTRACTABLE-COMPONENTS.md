# Extractable Components — what in Noesis is useful outside Noesis

> A living deconstruction of the Noesis repo: pieces that are **domain-agnostic** and would
> stand on their own as a crate, tool, or technique with no Proof-of-Mind thesis attached.
> Started 2026-09-29. Each entry cites a real symbol (`file:line`) and carries an honest
> **coupling** marker (how much surgery to lift it out) and **maturity** marker.
>
> Markers: coupling = LOW (copy-paste / thin generify) / MED (depends on `Cell` or a few
> types) / HIGH (woven through consensus). maturity = SOLID (has passing in-repo tests) /
> DEMO (runs, thinly tested) / THEORY (doc/proof, not code).
>
> **Honest baseline (do not round up):** Noesis has NO live production network (deploy-axis at
> zero, per `docs/COMPETITIVE-POSITION.md`). SOLID here means "green tests in the repo," NOT
> "battle-tested in production." Nothing below has run under real adversarial load on a live
> chain. Treat every entry as extract-worthy *code/idea*, not proven-in-the-wild infrastructure.

---

## 1. Graph credit & attribution (works on ANY directed weighted graph)

The strongest cluster. These answer "who deserves credit in a dependency/citation/flow graph"
and "is this graph being gamed" — questions far larger than Noesis.

- **Exact set-cover Shapley (`nsvg_shares`)** — `node/src/lib.rs:3740`. Closed-form Shapley
  value for a coverage-union game: `phi_i = sum over e in cov_i of 1/d(e)`. Fair credit split
  with zero sampling variance, deterministic, `O(sum |cov_i|)`. The Shapley-Sets recursion
  (Sivill & Flach 2023) collapsed to its closed form; pinned equal to brute-force exact Shapley
  to 1e-9. **General use:** any "fair attribution over overlapping contributions" — OSS
  dependency credit, dataset/feature attribution, co-authorship. coupling LOW (takes `&[&[u8]]`).
  maturity SOLID (tested this session; note: not yet wired into any production value path —
  `recurse_shares` delegates to it but has no live consumer).

- **Damped-Jacobi credit propagation (`value_flow`, `value_flow_with_own`)** —
  `node/src/lib.rs:3654, 3576`. PageRank-family: `flow(b) = own(b) + d * sum_{c built on b} flow(c)`,
  `d < 1` guarantees contraction/convergence even on cycles. Includes a single-joint geometric
  damping (`RHO = 1/phi`) that bounds sybil-split amplification. **General use:** transitive
  credit/influence/trust propagation on any DAG-ish graph with cycle safety. coupling MED
  (`&[Cell]`). maturity SOLID.

- **Helmholtz-Hodge circulation / cycle-energy detector (`attribution_circulation`,
  `attribution_cycle_energy`)** — `node/src/lib.rs:373, 426`, driver
  `node/examples/real_graph_cycles.rs`. Detects circular fund/credit flow (wash trading,
  citation rings, collusion) as non-zero curl in a flow field. **Measured 12.9-sigma signal on
  the real 1,770-repo / 4,222-edge DeepFunding graph** vs a degree-preserving null. **General
  use:** point it at ANY transfer CSV (DEX trades, token transfers, citations) —
  `cargo run --example real_graph_cycles -- <csv> <from> <to> <skip_header>`. This is arguably
  the single most sellable extract. coupling MED. maturity SOLID (one null model tested; wants
  1-2 more).

- **Submodular / graph-restricted cooperative game kit (`recurse_two`, `synergy::*`,
  Myerson `v_graph`)** — `node/src/lib.rs:3696`, `synergy` mod `:3301`. Sampled Data-Shapley,
  Myerson graph-restricted value (only provenance-connected coalitions create value), Copeland
  additive baseline, and an L1 "is the cooperative game actually load-bearing vs additive"
  statistic. **General use:** a small, dependency-free cooperative-game-theory toolkit.
  coupling MED. maturity SOLID.

## 2. Content novelty & anti-duplication

- **Shingle coverage + temporal novelty (`coverage`, `temporal_novelty`)** —
  `node/src/lib.rs:160, 187`. Strategyproof "novel contribution" scoring: a later duplicate /
  padding / near-copy earns 0 because it adds no new coverage vs earlier commits. **General
  use:** dedup-aware contribution scoring, plagiarism/copy detection, novelty rewards in any
  append-only corpus. coupling LOW-MED. maturity SOLID.

- **Semantic / entropy floor (`semantic_floor`, `semantic_floor_q16`)** —
  `node/src/lib.rs:7099, 6859`. Gates out high-structure-but-no-content noise. coupling MED.
  maturity SOLID.

## 3. Deterministic replicated computation (float-free, bit-identical across nodes)

A genuinely reusable engineering technique, orthogonal to the economics.

- **Q32.32 fixed-point mirrors with drift-guards (`value_fixed`, `settlement_fixed`,
  `finalization_fixed` mods)** — `node/src/lib.rs:6808, 8624, 8945`. Each floating-point value
  rule has a fixed-point twin proven to track the f64 reference within a band over a
  deterministic sweep, so heterogeneous nodes/VMs converge bit-identically. **General use:** the
  pattern for ANY consensus/replicated system that must avoid float non-determinism (blockchain
  VMs, distributed ML aggregation, deterministic lockstep sims). coupling MED (pattern is the
  product, not the specific rules). maturity SOLID.

- **Verifiable computation with proofs (`novelty_with_proofs`, `unique_shingles`,
  `proven_floored_novelty_q16`)** — `node/src/lib.rs:9221, 9203, 9247`. Emits the witness a
  verifier replays. coupling MED. maturity SOLID.

- **SplitMix64 deterministic PRNG** — `synergy::splitmix64`, `node/src/lib.rs:3306`. Tiny
  reproducible sampling with no crate dep. coupling LOW. maturity SOLID.

## 4. Verifiable persistence & state (the Cadence-convergent piece)

- **Append-only hash-linked log + byte-identical replay (`wire.rs`, `store.rs`)** —
  `BlockLog::append` `node/src/wire.rs:255`, `store::load_blocks` `:29`. "State = f(canonical
  log)": a node restarted from its log replays to a byte-identical `state_digest`. **General
  use:** any event-sourced system needing trustless "send me your log, I'll replay it"
  persistence + audit. This is the machinery Cadence independently converged on (see the
  2026-09-29 attestation work). coupling MED. maturity SOLID.

- **State/header digest (`state_digest`, `header_digest`)** — `node/src/runtime.rs:284, 774`.
  coupling MED. maturity SOLID.

- **Trustless snapshot verification (`UtxoCommitment::verify_snapshot`)** —
  `node/src/utxo_commitment.rs:164`. Verify a committed set without trusting the blob. coupling
  MED. maturity DEMO (validity proof is Phase-3 / zkVM).

## 5. Distributed-systems primitives (small, clean, liftable)

- **Committee-attested clock (`wallclock.rs`: `within_tolerance`, `advances_monotonically`,
  `observed_elapsed`)** — `node/src/wallclock.rs:30`. Time agreement without a trusted source.
  coupling LOW. maturity SOLID. See `docs/DESIGN-committee-attested-clock.md`.

- **ASERT-style retarget / never-halt liveness (`liveness.rs`)** — `node/src/liveness.rs:65`.
  A general snap-to-floor + re-tighten control loop. coupling LOW. maturity SOLID.

- **Terminating gossip flood (`gossip.rs`)** — dedup seen-set guaranteeing flood termination,
  transport-abstracted + unit-testable. coupling LOW. maturity SOLID (wiring to live mesh is a
  later slice).

## 6. Mechanism-design theory (portable results, not code)

- **Isomorphism-invariance testing** — `docs/ISOMORPHISM-INVARIANCE-VS.md`. The methodology
  "is my metric gameable by relabeling / graph-automorphism?" — a reusable adversarial test for
  ANY scoring mechanism. maturity THEORY + partial code (the identity-quotient in
  `downstream_flow_canonical`).

- **Conservation-of-hardness / attestation-exogeneity** —
  `docs/DESIGN-attestation-exogeneity.md`. The general result: you cannot extract a judgment
  about the external world from a system closed to it; any "external anchor" price = cost of
  capturing dispute generation. Applies to any oracle / reputation / attestation design.
  maturity THEORY.

- **Harberger + peer-prediction theorems** — `docs/DESIGN-harberger-peer-prediction-theorems.md`.
  maturity THEORY.

## 7. Adversarial simulation harness

- **Mechanism stress-test instruments** — `node/examples/`: `sybil_sim`, `wash_sim`,
  `adaptive_sim`, `moat_sim`, `peer_prediction_sim`, `periphery_sim`. A reusable pattern +
  scaffolding for empirically attacking your own mechanism (adaptive adversary, sybil rings,
  wash). **General use:** template for "prove my incentive design survives a strategic
  attacker." coupling MED. maturity DEMO-to-SOLID.

---

## Highest-leverage extraction candidates (first cuts)

1. **`hodge-cycles`** — a standalone crate/CLI wrapping `attribution_circulation` +
   `attribution_cycle_energy` + the real-graph loader. Detects wash/rings in any transfer or
   citation graph. Already has real-data validation. Lowest lift, highest external legibility.
2. **`fair-shapley`** — `nsvg_shares` + the submodular/Myerson kit as a tiny dependency-free
   cooperative-attribution crate.
3. **`detfixed`** — the Q32.32-mirror-with-drift-guard *pattern* written up as a technique
   (blog + reference impl) for float-free replicated computation.

## Cross-cutting note on coupling

Most graph functions take `&[Cell]` (id + parent + data + type_script). A clean extraction
would generify over a minimal `Node { id, parent, payload }` trait; the algorithms themselves
carry no PoM assumptions. That trait-lift is the main mechanical work standing between "in
Noesis" and "a crate anyone can `cargo add`."
