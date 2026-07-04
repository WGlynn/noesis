# The Verifiable Reduction / Export Layer

*Design note. PRIVATE (front-run-sensitive; keep off public substrate). Origin 2026-07-03: Will (the reduction layer) + Tom Lindeman / Pragma (the cross-chain export). Status discipline: ✅ built · 🟡 designed · 🔬 open. Nothing here is built yet; this names a frontier, it does not report one.*

## The one-sentence idea

Between contribution-gathering and mathematical consensus sits a single component whose only job is to be **faithful and checkable**: it reduces a heterogeneous contribution into the commensurable form the consensus math needs, and *the same property that makes that reduction consensus-safe makes it exportable to other chains.* Compute and export are the same primitive seen from two sides.

## The picture

```
   contribution            REDUCTION                 consensus math              EXPORT
   gathering        →      (translate/reduce)   →     (v(S), NCI)        →     (other chains
   (any mind,              to a common,               scores, finalizes         consume the
   any modality)          value-relevant form)        the PoM signal)           PoM output)
                          └── must be VERIFIABLE ──┘                    └── must be VERIFIABLE ──┘
```

Two seams, one requirement on both: a consumer (the on-VM consensus in seam 1; a foreign chain in seam 2) must be able to trust the output **without trusting the producer** — by re-deriving it or by checking a proof of faithful production.

## Part 1 — the reduction layer (Will, 2026-07-03)

**What it is.** The "reduce" step of a map-reduce over minds. Map: heterogeneous contributions (a blog post, a proof, a commit, a dataset) arrive in any language. Reduce: each is translated into the one representation the consensus math compares. That translation is where subjectivity lives (which dimensions count) and where the dimension-consensus we described to Tom gets applied.

**It already exists, deterministically.** In the reference node the reduction is:
- `coverage()` — raw cell data → a set of content shingles, described in source as *"a proxy for the learned reward-model evaluation of what the cell contributes"* (`node/src/lib.rs:60`). ✅ built.
- `value::features()` — a cell → a 3-vector `[ln(1+data_len), ln(1+coverage_size), provenance_flag]`, "no external oracle, only what the type-script can see" (`node/src/lib.rs:854`). ✅ built.
- the `v(S)` pipeline `temporal_novelty → pom_scores → value_v5..v8` reduces contribution + graph-context to a scalar (`ARCHITECTURE.md:91`). ✅ built (reference), 🔬 the learned tier open.

**The ambition.** Lift the reducer out as a first-class, richer, possibly-AI, swappable component, so *how we value contributions* can evolve (better models, new value-dimensions) **without forking consensus**, as long as the reducer's output contract is stable.

**The wall.** Consensus requires the reduction be deterministic / re-derivable — the architecture's own maxim: *"don't let the attacker choose a security-critical input; every consensus value is re-derived on-VM, rejected if not reconstructable"* (`ARCHITECTURE.md:110`). A rich, non-deterministic, AI reducer collides with this. If the reducer is a **trusted, non-verifiable oracle, it reopens the exact airgap Noesis exists to close** — whoever controls the translation controls value, and you have reintroduced the trusted relay the whole design removes. This is the failure mode.

**The resolution.** Make the *translation* verifiable, not the *translator* deterministic (RSP's Proof-of-Translation-Integrity applied to consensus): the reducer may be rich / AI / off-chain if it emits a commitment whose faithful reduction is checkable — a ZK proof of faithful reduction, commit-reveal + a challenge window, or a pinned model re-run on-VM. Today the code deliberately keeps the reducer thin-and-deterministic (shingles + 3 features) *precisely to dodge this*; the rich reducer's gate is verifiability. 🔬 open — this is the same frontier as the learned `v(S)` (first real-DeepFunding-label run returned NULL, `ARCHITECTURE.md:101`) and the general isomorphism-invariance gate.

## Part 2 — the export layer / PoM as a consumed layer (Tom, 2026-07-03)

**The move.** If a foreign chain / L2 can take the reduction's scored output and build on it, Noesis stops being *a chain competing for blocks* and becomes *a layer the ecosystem consumes* — the EigenLayer / Chainlink shape. But what is exported is neither capital-security nor price data: it is **proof-of-mind, contribution scored as a consensus input.** Per our own related-work audit (`docs/research/RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md`), the consensus object elsewhere is always something *other* than mind — so this export has no incumbent. Uncontested surface.

**Why it is the same primitive.** For a foreign chain to consume the PoM output, that output must be verifiable off our chain — identical to seam 1's requirement. Faithful-and-checkable makes the reduction *computable by our consensus* AND *exportable to anyone*. One property, two payoffs.

**It dissolves "could be slow" (Tom's own flag).** Mind accrues slowly, so gating a hot block path on accumulating enough PoM is genuinely slow. But if PoM is a **slowly-updating standing registry** that consumers *read* at their own cadence (not a hot path they wait on), slow accumulation is fine — it is only fatal for a hot path. PoM standing is already soulbound and slowly-evolving (✅ built ref), which fits "registry other chains read" better than "per-block feed."

**The cross-chain half is not a fresh problem.** The verifiable-messaging shape needed for export is the same one designed for the VibeSwap DEX (canonical burn-and-mint, BLS12-381 threshold sigs, bonded validators; post-LayerZero). 🟡 designed / interface, implementation in progress, **not shipped** — do not claim built.

## Design forks (with current leans)

1. **Reducer richness.** Thin-deterministic (today, consensus-safe by construction) ↔ rich/AI-with-verifiability (the ambition, gated on the open verifiability edge). Lean: keep the deterministic reducer as the consensus floor; let a rich reducer *advise* until its output is provably faithful.
2. **PoM output granularity.** Per-block feed ↔ slowly-updating standing registry. Lean: **standing registry** — dissolves "slow," matches the soulbound standing model.
3. **Export relationship (the big one).** Consumers *inherit* mind-security (restaking-style, cryptoeconomically bound) ↔ consumers *read* the signal (oracle-style, advisory). Lean: **ship the readable signal first, earn security-inheritance later.** Different products; this choice changes everything downstream.

## Honest status

🟡/🔬 throughout. The reduction exists deterministically (✅); the rich/verifiable reducer and the cross-chain export are **designed, not built**, and both rest on the *same* open problem — making the reduction verifiable-enough that a non-deterministic or off-chain producer cannot become a trusted relay. That is not a bolt-on; it is the load-bearing frontier, and it is the same one the value model itself faces.

## Why this is one idea, not two

The reducer makes PoM *computable*; the same verifiability makes it *exportable*. Compute and export are the identical requirement — an invariant (what the reduction *means*) decoupled from a variable process (how it is produced), joined by a verifiable mapping. That is the RSP / provenance-of-mind primitive again: the recurring fixed point the whole stack points at. Cf. `~/JARVIS/papers/rosetta-stone-protocol.md`, and the Rodney claim-compiler (same shape at the scholarship layer).

---
## BUILD IN FLIGHT — 2026-07-03 ~09:42 CDT
Will: "I like Tom's idea. build it as is... take initiative tom would respect and actually want to use." Building the export layer AS IS (no gold-plating), local-only, NO commit/push (front-run-sensitive + public/private repo question is Will's call).
- **Workflow `wf_46e69ec3-5a8` (task wgq544v12)** running: Map (v(S) API / serialization surface / build+test conventions) -> Build (`pom_export` module: `PomExport` serde-JSON = per-contribution pom_value + aggregate + commitment; `export(cells)` calls EXISTING v(S); `verify(export,cells)` = deterministic re-derivation; tests: determinism / tamper-reject / JSON round-trip; cargo build+test) -> Verify (adversarial: real-not-theater, reuses-existing-reduction, honest verifiability).
- **Verifiability = deterministic re-derivation ONLY** (what we have); NOT ZK/proofs (the open edge, deliberately not added).
- **ON COMPLETION:** read result (PomExport shape + verbatim test output + Tom-usability verdict); if green + reviewer-clean, report to Will with "how a foreign chain consumes it" one-liner; HOLD commit/push for Will. If resuming a fresh session: check `wgq544v12` result at `~/.claude/.../tasks/wgq544v12.output`, or resume `Workflow({scriptPath: ".../build-pom-export-layer-wf_46e69ec3-5a8.js", resumeFromRunId: "wf_46e69ec3-5a8"})`.
- Companion: Tom reply staged Gmail `r6993560315218836076`; design note = this file; Desktop PDF copy present.

---
## PoM-ON-ETH — the real, decentralized MVP (2026-07-03 ~10:00 CDT; Will + Tom)
Front-run caution DROPPED (see memory feedback_structural-honesty-cannot-be-front-run) — build/share in the open.

**Architecture Will sketched (all real, mostly reuses shipped code):**
- PoM computation runs OFF-CHAIN (the `pom_export` Rust reduction) — too expensive to run inside the EVM.
- A set of VOLUNTEER node operators / signers, BONDED, attest to the export (threshold/multisig).
- ETH (or any EVM chain) VERIFIES the attestation + consumes the PoM scores on-chain.
- Signers earn an ERC-20 reward; bond + slashing keep them honest.
- **Trust model = bonded signer set + deterministic re-derivation fraud-proofs (anyone re-runs `pom_export` and challenges). NOT ZK-of-computation** (that stays the open edge; not needed for the attestation path).

**SHIPPED + carbon-copy-reusable (verified via Explore, vibeswap/contracts/messaging/):**
- `MessagingValidatorRegistry.sol` (503L, well-tested) — bonded BLS validators, epochs, snapshots, threshold=⌈2n/3⌉+1, slashing. The trust root. Reuse as-is.
- `VibeSwapCanonicalToken.sol` (real+tested), `SupplyAccountant.sol` (real+tested), `MessagingPoM.sol` (v0.1 slashing, governance-asserted facts).
- `CrossChainRouter.sol` = LIVE but LayerZero order-router, NOT the token bridge (do not conflate).

**GAP to build (~200L Solidity, ~90% reuses registry):**
- `AttestationVerifier.sol` — interface only at `contracts/messaging/interfaces/IAttestationVerifier.sol:88` (`verify(message, proof)`). The on-arrival signature check. NOT BUILT.
- `MessagingHub.sol` — interface only at `interfaces/IMessagingHub.sol:156` (`receiveAttestation`). Orchestrator. NOT BUILT.
- NonceRegistry (replay) — not built.

**THE FORK (decides ships-today vs waits) — AWAITING WILL'S PICK:**
- **BLS12-381 threshold** (reuses the shipped BLS registry, elegant) but on-chain BLS verify needs EIP-2537 precompile — NOT on most EVM chains until ~Q3 2026 (CKB native).
- **secp256k1 multisig** (each signer ECDSA-signs; contract checks M-of-N via native `ecrecover`) — ships on EVERY EVM chain TODAY, cheap, no precompile; needs a lighter secp256k1 signer registry (not the BLS one).
- **Jarvis recommendation: secp256k1-multisig v1 (ships now on ETH), BLS-threshold v2 upgrade.**

**NEXT BUILD (once fork picked):** the EVM side on top of the `pom_export` foundation — verifier + hub + reward ERC + a real end-to-end run (off-chain reduce -> signers attest -> EVM verify -> consume scores). Carbon-copy the shipped registry pattern; fill the ~200L gap. Best done in a FRESH context (this one is ~704k). Foundation build: workflow wgq544v12 (pom_export module + tests). Design intent: this file, Parts 1-2.

## pom_export FOUNDATION — BUILT + GREEN (2026-07-03, workflow wgq544v12)
- `node/src/pom_export.rs` (~245L, new): `PomExport { theta_sim_q16, scores:[{id,index,pom_value}], total, commitment }` (serde-JSON, blake2b commitment) + `export()/export_with_theta()/verify()`. `pub mod pom_export;` added to lib.rs; serde/serde_json added to Cargo.toml.
- **Reuses the EXACT consensus reduction** (`value_fixed::temporal_novelty_with_similarity_floor_q16`, same as runtime.rs:525) — scoring NOT reinvented. Verified against source by adversarial reviewer.
- Tests 5/5 (determinism, verify-accepts-honest, verify-rejects-tampered-value, verify-rejects-tampered-commitment, JSON-round-trip); full suite 267/0; clippy-clean. NOT committed (working-tree only).
- Verifiability = deterministic re-derivation ONLY (honest; not ZK). Verdict: usable, real-not-theater.
- **3 MINOR refinements to fold into the EVM build:** (a) reframe blake2b commitment as a portable out-of-band anchor (not an independent verify gate); (b) document that verify() trusts the caller-supplied `cells` — consumer MUST source canonical cells from chain/bonded attestation (this is the seam-2 trust root); (c) per-cell scores; expose/regroup per-contributor (type_script.args) for the soulbound-franchise consumer.
- NEXT: the EVM side per "PoM-ON-ETH" section above (fork: secp256k1-multisig v1 recommended). Build in a fresh context.

## ARCHITECTURE DECISION 2026-07-03 — PoM-on-ETH drops the NCI triad (Will)
For the ships-now PoM-on-ETH MVP, **PoM = just the scoring ALGORITHM/signal; do NOT build the Noesis
PoW/PoS/PoM triad.** The HOST chain (ETH) provides consensus (block production / ordering / finality)
via its own stake-and-work. The triad was only needed for a SOVEREIGN PoM chain (mind can't
produce/order/sybil-resist blocks alone → PoW/PoS anchor it). On ETH, ETH is the anchor.
- **Simplifies the EVM build:** NO consensus mechanism to build — just the attestation verifier + bonded
  signer registry + reward ERC + the pom_export signal. That's it.
- **Honest trade (state it):** on ETH, PoM is DATA a contract reads, NOT a consensus power. You LOSE the
  anti-plutocracy veto (MIN_DIM_BPS: neither axis finalizes alone / capital can't finalize without
  contribution's consent). ETH stakers finalize everything; PoM has no vote.
- This IS the "readable signal first (PoM-on-ETH) / security-inheritance later (sovereign triad)" fork.
  MVP = signal on ETH. Endgame = mind-as-consensus triad. Not either/or; sequence.

## PRODUCT FRAMING 2026-07-03 (Will) — PoM: consensus → value-defining algorithm → truly autonomous DAO
The build isn't "an oracle feed." It's **the value-defining engine for a DAO where CONTRIBUTION, not
capital, defines worth.** Progression: PoM-as-consensus (sovereign triad, heavy) → PoM-as-value-ALGORITHM
(attested signal on ETH, ships) → that algorithm driving a DAO's value/reward/governance-weight =
**a truly autonomous DAO.**
- Every DAO today = plutocracy (token-vote) or multisig (humans decide). Not autonomous — discretion-run.
- PoM-DAO = value-attribution/reward/governance-weight set by a **deterministic, verifiable, attested
  algorithm** over contributions. "Autonomous" = the org's value system is a checkable algorithm, ¬ a vote/vibe.
- **Recovers the anti-plutocracy property at the APPLICATION layer:** dropping the triad loses "mind vetoes
  capital in finality," but a fully PoM-defined DAO means capital NEVER defines worth (the algorithm does) —
  ships on ETH today, arguably a purer thesis-expression than the consensus veto.
- **Realness guard:** "autonomous" = verifiable value-attribution (deterministic + re-derivable + attested =
  the green pom_export), NOT decentralized-vibes. The verifiability is the differentiator from every other
  "autonomous DAO" claim. THIS is the pitch to Tom / the world.

## THE SOUL 2026-07-03 (Will) — a PURE DAO = a value router, nothing else
"it doesn't do anything other than routing value. it's a value router and that's it. no politics, no
games, no degenerate finance. just a value flow."
- The DAO reduced to its IRREDUCIBLE function: value in → contribution scored (PoM) → value routed out.
  Governance-voting / tokenomics / yield = accreted cruft that ISN'T the function. Deleted, not guarded.
- **This is WHY it can be truly autonomous:** most DAOs can't be autonomous because they do 3 gameable
  things at once (politics + finance + coordination), each needing discretion / a capture surface. A pure
  value-router has nothing to politick over, no yield to farm, no vote to seize → autonomy is trivial.
- **Un-gameable by construction:** no proxy to spoof, no financial layer to lever, no discretion to
  corrupt — the routing IS the deterministic, re-derivable algorithm (pom_export). No politics (no
  discretion) · no games (nothing to game) · no degen finance (no finance, just flow).
- The tagline made literal: "a coordination primitive, not a casino." Deleted the casino, kept the one
  honest function under it: reward goes to who actually contributed, by a checkable algorithm, no
  rent-seeking skim. THIS is the moral core / the pitch.

## WORKING CODENAME 2026-07-03 (Will) — "The DAO 2"
Callback to the OG Ethereum "The DAO" (2016) that died. Earned, not just a meme: The DAO died from the
exact 3 things this deletes — POLITICS (the ETH/ETC fork war), GAMES (the reentrancy drain), DEGENERATE
FINANCE (a complex leveraged investment vehicle whose complexity WAS the attack surface). "The DAO 2" =
the pure value router the original never was; survives because there is nothing to drain, fork over, or
game. Working codename (memorable, tells the story). Honest flag: the name carries the hack baggage —
great as a rallying codename ("we fixed what killed it"), riskier as a final public brand; lock the
public name later.

## EVM SIDE — BUILT + GREEN 2026-07-03 (Opus, ultracode session)
The Part-2 export consumer is now BUILT on ETH (secp256k1 path DROPPED for multisig-stigma;
Will 2026-07-03 "just not multisigerino" → **optimistic re-derivation** instead). Location:
`vibeswap/contracts/pom/` (reuses the vibeswap foundry harness; scoped `FOUNDRY_PROFILE=pom`
to dodge a pre-existing broken import in `contracts/identity/AgentRegistry.sol`).
- **PoMOperatorRegistry.sol** — bonded operators, address-keyed (lean fork of MessagingValidatorRegistry; no BLS, no epochs).
- **PoMReward.sol** — mintable ERC-20 (POMR), minter=hub, **zero premine** (fair launch).
- **PoMExportHub.sol** — optimistic state machine: propose → challenge → finalize → resolveDispute/expireChallenge; consumer `verifyContributionScore` via OZ Merkle over `scoresRoot`.
- Interfaces + `SECURITY-NOTES.md` (v1 trust boundaries + review triage).
- **pom_export.rs** extended: `per_contributor` regroup + EVM-portable keccak `scores_root` (OZ double-hash leaf + commutative pairs); `scores_root` field added to `PomExport`. `tiny-keccak` dep added.
- **Trust model:** happy path = no quorum; safety = 1-of-N-honest freeze that does NOT trust the resolver (a challenged standing is always discarded — the resolver only slashes); liveness via `expireChallenge` (resolver not a SPOF). v1 resolver = governance adjudicator; **v2 = the ZK/RISC-V one-step proof** in the same swappable slot (this is "A now, C later" — Will 2026-07-03).
- **Tests:** Solidity 14/14 (`test/pom/PoMExportHub.t.sol`), Rust node 270/270 (incl 8 pom_export). Cross-language Merkle root pinned + conformed (`daf99dca…09d38bc0`). Hardened per adversarial-review workflow `wkp84x0vz` (resolver-can't-finalize, expireChallenge, min-window, CEI).
- **NOT committed** — on-disk only, held for Will (two repos: vibeswap + noesis).
- **NEXT:** reward → **meta-block subsidy** ("Ethereum Cogcoin", Economitra generalized; Will 2026-07-03). Subsidy routes to CONTRIBUTORS by PoM score via the shipped `scoresRoot` Merkle (the value-router made literal). Design under gated workflow `w0qpj9yx7`.
