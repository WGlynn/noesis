# 2026-07-22 — The Last Bottleneck, Part 3 published (council-revised) + trilogy complete

Plain-English recap. Reading artifact, not an operator handoff.

## What happened

Part 3 of the Last Bottleneck trilogy went from a plan to a published essay, and the interesting part is that it almost went out with a lie in it.

**The write.** Part 3 ("The Seams Are Where Extraction Lives") argues what the base of a value system has to BE, following Part 1 (the neck: positive-sum coordination) and Part 2 (fairness must live at the base). The spine: extraction lives in the *seams* — the handoffs between separated functions where one side's rule ignores the other side's action. MEV is the ordering/settlement seam, governance capture is the decide/bear-cost seam, and so on. The prescription: fuse the functions so there is no unbound handoff for extraction to live in.

**The catch.** The permanent pre-ship gate — a diverse adversarial persona council — came back **needs-major-revision**, and four of five seats independently, against our actual code, caught two real problems:
1. The v1 seam definition was **circular**: seam = unpriced handoff, extraction = acting in a seam, coherence = no seams. That is one identity restated, true by definition, predicting nothing — and it fails Part 1's own falsifiability standard.
2. v1 claimed **"closed by construction" exactly what our code marks OPEN.** It said provenance makes contribution un-fakeable and capture impossible. But `ARCHITECTURE.md:109-111` measures wash-building at ~0% graph-internal separation ("the deepest open item"), and `dim_ok`/`MIN_DIM_BPS` (`runtime.rs:1619-1624`) is a per-dimension floor, not a per-identity cap — a single dominant contributor satisfies it alone (confirmed in `anti-plutocracy-attack-surfaces-2026-07-17.md`).

Every code citation the council made was re-verified against the actual files before rewriting (standing anti-hallucination rule: never assert a Noesis number from a secondary source, including the council).

**The fix (v2, published).** The honest version is stronger:
- Seam got an **extraction-independent** definition: B binds A's action iff B's rule is a non-constant function of A's action — checkable a priori, before any exploit. Plus an explicit falsifier.
- Thesis retired from "no seams" to **"no unbound handoff."** Our deliberate value-oracle seam (kept for upgradability) became a *positive* example rather than a contradiction.
- Memory/value/governance/coordination rows rewritten to our own built/designed/open status discipline. Provenance binds *origin* (closed for structural fakery); binding *value* (telling worthwhile contribution from elaborate worthless contribution — wash-building) is named as the real open frontier, "the last neck under the last neck."
- Kernel analogy corrected (Spectre/confused-deputy make "no privileged crack" false; reframed as a minimal binding core at a concentration cost, seL4-style).
- Prior art situated honestly (Pigou/Coase externality, Grossman-Hart-Moore residual control rights); the genuinely new claim isolated — those literatures *manage* the boundary, a coherent base can *remove* it.

## State

- Trilogy complete and public in `research/`: `the-last-unbroken-neck.md`, `fairness-is-a-substrate-not-an-app.md`, `the-seams-are-where-extraction-lives.md`. Part 3 commit `759d555`.
- Parts 1 & 2 also got Boardy acknowledgments this session + the last "cannibalism"→taint-by-dependency scrub (`f8d0335`).
- Two Boardy reply drafts + a "be the change" attribution note staged on Desktop, pending Will relay.

## The meta-lesson

The council did exactly what it is for: it stopped us from publishing a claim that our own code refutes. The essay is *against* mistaking efficiency for fairness and rounding open problems up to closed ones — and it nearly committed that sin about itself. Catching that in the gate rather than in public is the methodology working, not failing.
