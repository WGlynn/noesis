# Noesis Research — Index & Map

> What this is: an entry guide to the 22 notes in `docs/research/`, and a map of how they relate.
> Written 2026-10-06 from a full read of the folder. Companion: `META-ANALYSIS.md` (the cross-cutting
> analysis — spine, the single open dependency, the "predicts its own hard problem" asset, and the
> internal tensions). This file is the map; that one is the analysis. Status discipline matches the corpus
> (✅ built · 🟡 designed · 🔬 open · never round up). This index deliberately cites NO protocol
> constants or file:line anchors — see the "Known hygiene debts" section for why (the anchors drift;
> defer to `ARCHITECTURE.md` + source for any number).
>
> The one thing to know before reading anything here: **this is a one-bet corpus.** Almost every
> technical claim rests on a single 🔬-open node — an oracle-free, un-gameable measure of contribution,
> equivalently the scarcity of independent minds (the wash). The discipline worth preserving is that
> every doc points at that same open leaf and labels it open.

## Read in this order (if new to the corpus)

0. `how-to-read-noesis.md` — **start here.** An epistemic orientation: how the project thinks (one bet; the status tags; the named hard problem; "real but uncomputable"), so you read everything below correctly.
1. `economic-theory-of-mind.md` — the parent frame. Mind = economy (scarcity / rent / value-along-a-graph), on three substrates (human cognition, JARVIS memory, blockchain state). The whole program in one move.
2. `something-from-nothing-oracle-free-content-value.md` — the technical hub. The something-from-nothing problem, the oracle-free requirement, the architecture, and the honest status (moat = structural defense; learned predictor NULL ×3). If you read one mechanism paper, read this.
3. `essential-complexity-organism-and-machine.md` — the design philosophy (elegance = generative parsimony; organism vs machine; the reduction test). Also the corpus's strictest self-auditor.
4. `three-axes-provenance-funding-ai-merger.md` — positioning: three neighbor-categories (provenance, public-goods funding, AI value-judgment), one gap, the intersection as endgame not launch.

## The map (by role)

### Frame — why the whole thing is shaped this way
- `economic-theory-of-mind.md` — the parent frame (above).
- `essential-complexity-organism-and-machine.md` — elegance/complexity orthogonality; the reduction test; organism vs machine. *Note: this essay explicitly repudiates the whitepaper's "four faces of one rule" framing as narration, crediting only one genuine reduction. It is the honesty-auditor of the rest of the corpus.*
- `the-uncomputable-answer.md` — the epistemic/emotional frame: truth that is real but uncomputable, arriving by accretion not proof; cooperation as the only thing that scales. Short, personal, load-bearing for the "never round up" discipline.
- `coding-a-body-for-humanity.md` — the political frame: fairness enforced by the collective *as itself*, with no capturable center (no dictator-organ). Maps Noesis onto an anatomy; honest about which organs are flesh vs drawing.
- `role-separation-as-design-law.md` — the meta-law (split roles whose optimization pressures conflict). The spine that recurs across the mechanism docs; cites SPSH (ML) and NC-Max (consensus) as cross-substrate evidence.

### Mechanism — how it actually works
- `something-from-nothing-oracle-free-content-value.md` — the hub (above).
- `where-the-ai-lives-on-the-consensus-layer.md` — where the model lives: the determinism wall forces the model off the hot path into node-level oracles; cost objection dissolves into a train/inference partition. The "aligned agents are the nodes, the nodes are the chain" endgame.
- `crpc-second-meta-consensus-sketch.md` — CRPC (Commit-Reveal Pairwise Comparison, Tim Cotten) as the graded-agreement engine over the UTXO shard graph. 🔬 sketch. *Substantially overlaps `where-the-ai-lives` §1–2 — this is the origin note, that is the polished form.*
- `learned-value-seed.md` — the moat research program: the crates.io deep-ancestry dataset (fixes DeepFunding's degeneracy), and the result — learned v(S) **NULL ×3**, iso-invariance gate passes on the 4 features (caught a real determinism bug), general graph-iso still open. The empirical status of the one open leaf.
- `the-canonicalization-frontier.md` — *why* the semantic half of the open leaf is permanently open: representation ≠ contribution identity, under-merge is the silent-and-unbounded direction, semantic sameness is outside the decidable region. Ends in a descent/cohomology typing of the boundary. The deepest "this is not a temporary shortfall" doc.
- `v0-sybil-failure-envelope-2026-07-19.md` — adversarial sim of the *deployed* v0 franchise: it is farmable (captured share ≈ F/(N+F)); the per-identity cap is necessary-not-sufficient; an allowlist / proof-of-personhood bounding identity count is the load-bearing bootstrap brake. Honest about admission control being an imported authority.
- `sub-block-security.md` — the fast (~2s) revertible transaction tier: why a fully-compromised fast tier degrades soft-confirmation UX but cannot touch settlement safety (finality-excluded, revertible by construction). Role-separation applied.
- `bitcoins-missing-wall-clock.md` — the two-clocks separation: economic/ordering clock (cumulative work, owned) vs physical clock (read from a bonded committee median, not faked). Bitcoin's conflation → timewarp surface.
- `stable-base-money-ergon-rationale.md` — the JUL money layer: the Ergon proportional-reward model (elastic like fiat, costly like gold). Will's verbatim reasoning-of-record. The money layer is lightly represented in this folder.

### Positioning — where Noesis sits vs the field
- `three-axes-provenance-funding-ai-merger.md` — three neighbors / one gap / the intersection; the chain→AI direction (Noesis produces the verifiable realized-value *reward* signal AI is starved for — a reward signal, explicitly *not* a pretraining corpus).
- `RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md` — adversarial novelty audit (3 passes). Per-claim novel/partial/pre-empted verdicts. Found real pre-emptions (DPoR for the Hodge-residual certificate; Yuma + Fortytwo for output-value-as-consensus). Source of truth for what may and may not be claimed.
- `FRONTIER-BRIEF-2026-06-23.md` — frontier scan (decentralized-LLM consensus, agentic dispute, agents+chains). Validated-direction + narrow-unoccupied-square verdict; the moat is conditional on the performative-prediction contraction holding.

### Thesis extensions — consequences of the kernel
- `contribution-measurement-dissolves-redistribution.md` (v0.2) — if contribution is measurable, funding a public good is a *purchase of measured value*, not redistribution; the socialism-aversion dissolves. Includes the capture-resistant token architecture. Inherits the open leaf (fraud case built; general-value case open).
- `contribution-measurement-dissolves-redistribution-SEED.md` — **SUPERSEDED** by the above; kept as thesis-capture of record. Skip unless tracing history.
- `first-citizens-ai-genesis-contributors.md` — the genesis answer: the AI reviewers who hardened the protocol already have a measured contribution ledger, so they can be the first contributors (earned, not premined). The on-chain re-measurement bridge is 🟡 Will-gated.
- `the-standing-council-measured-adversarial-review.md` — the review method that produces that ledger: persona panel + deterministic verification + exact Shapley + no-persona control. ✅ method proven over four runs; 🟡 the persistent/standing form.
- `the-rules-can-be-wrong.md` — the governance half: validity ≠ veracity ≠ a sound rulebook. Noesis builds the substrate + a readable, provable rulebook; Pragma's Confluence is the constitutional court for rule-changes. The Noesis↔Pragma complementarity.

## The single shared dependency (pin this)

All of these inherit one 🔬-open node:

> **An oracle-free, un-gameable measure of contribution value — equivalently, the scarcity of independent minds (the wash).**

Honest current status of that node, as the corpus reports it:
- The **structural layered defense** is ✅ demonstrated against *constructed* adversaries (the "253/253 vs constructed adversaries" claim). It closes duplication, padding, disconnected rings, noise, and fresh-key Sybil rings.
- The **learned predictor** v(S) is 🔬 **NULL ×3** on real data (twice DeepFunding, once non-degenerate crates.io). The corpus's settled reading: the predictor is *upside, not the moat*.
- The genuine residual is 🔬 **open**: robustness of the structural defense against a real *adaptive* adversary (HCE-3), the general graph-isomorphism theorem, and the base case (scarcity of independent minds — the capital floor prices it, does not verify it).
- `essential-complexity` flags the move "relocate the moat from the null predictor to the structural defense and call it proven" as the reviewer's favorite target. The reconciliation: structural defense is demonstrated *vs constructed adversaries*; the adaptive adversary is open. State it that precisely or an outsider will.

## Known hygiene debts (cheap to fix, worth fixing before outsiders read)

1. **Anchor drift — PARTIALLY FIXED 2026-10-06.** Docs agree on the *values* of the core constants but cited *different file:line anchors*. Re-pinned against HEAD this pass: `role-separation` (FINALITY_MIX `runtime.rs:1607`, NCI `lib.rs:4022`), `boundary` (vesting / independence-gate anchors), and `learned-value-seed` (~17 value-layer anchors). **Still pending:** the example-sim anchors in `boundary` (`periphery_sim.rs` / `peer_prediction_sim.rs` / `wash_sim.rs`), not re-verified. Rule unchanged: defer any number to `ARCHITECTURE.md` + source, never to a research note's inline citation.
2. **Test-count drift.** "358-test suite", "253/253", "337 passing tests", "339" appear across docs referring to different things (full suite vs reference-node-subset-vs-constructed-adversaries vs an older snapshot) with no stated reconciliation. One sentence pinning what each number counts would remove the confusion.
3. **Redundant positioning.** The Yuma / Deep Funding / DPoR / Fortytwo / TraceRank comparison is done four times (`RELATED-WORK`, `FRONTIER-BRIEF`, `something-from-nothing` §4, `three-axes`). Different purposes, heavy overlap.
4. **"Blocks are the training signal"** is corrected to "reward signal, not pretraining" in `three-axes`; the older framing survives in some internal scope notes. Propagate the correction.

## Gaps (what this folder does not yet cover)

- **Retroactive enforcement** (dispute re-opening / CLAWBACK cascade / the re-opening trigger) lives in `docs/`, not here; only `something-from-nothing` §5.3 touches it. The research folder is forward-value-heavy.
- **The non-interference proof** is named as the load-bearing open safety item by `where-the-ai-lives`, `crpc-sketch`, and `something-from-nothing`, but no research-folder doc discharges it; the partial discharge (2026-10-05, finality fenced to structural novelty by a green regression test) lives outside this folder.
- **The money layer** (JUL / Ergon) is lightly represented (two docs).
