# The Coordination Schelling Point — inward and outward consensus (PRIVATE)

> Stealth. Synthesis captured 2026-06-11 (Will, in-flight). The deployment thesis
> for why Proof of Mind spreads, and why the *same* reconciliation primitive
> produces a coherent self and a coherent network. Folds into `WHITEPAPER.md` §5.2.

## The claim (Will, 2026-06-11)

> "In the future, everyone would just install JARVIS as a coordination Schelling
> point — that way the consensus emerges inwardly and outwardly by being in the middle."

The strong version is correct. State it precisely:

**The same reconciliation fold runs at two radii.** PoM is not only a network
consensus rule; it is a reconciliation primitive that operates at two scales of the
same shape.

- **Inward consensus.** A locally-run agent (JARVIS) reconciles *one participant's
  own* scattered contexts, sub-agents, drafts, and memories into a single coherent
  will. This is the Economic-Theory-of-Mind layer: a mind treated as an economy that
  must reach internal agreement before it has a preference to express. Most consensus
  systems never model this — they assume each node already holds one coherent
  preference. JARVIS *manufactures* that coherent preference (the WWWD fold).
- **Outward consensus.** Many JARVIS-running minds reconcile *across* each other via
  PoM-weighted agreement. The unit a node commit-reveals outward is the same
  contribution primitive a mind uses to reconcile itself inward.

Same fold, two radii. Not a metaphor — a scale-invariance: the macro shape (network
consensus over minds) and the micro shape (a coherent self over sub-minds) are the
*same fractal*. This is the substrate-geometry-match property applied to consensus:
the coordination mechanism is scale-free.

**"By being in the middle."** The coordinator sits between the participant and their
own internal noise (inward) *and* between the participant and everyone else (outward).
Occupying the middle on both sides is what lets it be the honest broker at both scales
— the augmented-governance role: augment the invariant, do not replace the parties.

## The two edges that make or break it

The naive reading collapses into the *opposite* of consensus. Two conditions are
load-bearing:

### Edge 1 — Schelling point on the PROTOCOL, not the INSTANCE
"Everyone installs *the same* JARVIS" must mean the same **protocol**, not one shared
instance. A shared instance/server is centralization wearing a consensus costume. The
version that works: every participant runs a **sovereign** instance — their own
memory, their own WWWD projection — that speaks the **same consensus language**. The
Schelling point is the shared protocol everyone converges on *without coordinating*,
not a shared brain. (This is exactly the substrate-decentralization roadmap: primitives
decentralize, each node sovereign.) Inward/outward duality holds **iff** JARVIS is a
protocol, not a platform.

### Edge 2 — openness + neutrality is what makes it focal
A Schelling point needs a *reason* to be the obvious choice. A black-box or extractive
coordinator does not get adopted — participants fork away from it. JARVIS lands "in the
middle" only because it is *honestly* in the middle: open files, open weights,
equal-rights-to-the-AI. The honesty is not decoration; it is the mechanism that makes
it focal. **Dishonest-in-the-middle = not-focal = no consensus.** This is
honesty-as-structural-load-bearing at the adoption layer: the property that secures the
chain is the same property that makes people converge on it.

## What this does NOT rescue

Neither edge removes the one load-bearing risk: the value function `v(S)` must be
**un-gameable**. The Schelling framing explains *spread*; it does not substitute for
strategyproof measurement. Temporal-novelty gives the floor (sybil/padding/collusion
earn 0); the learned `v(S)` must preserve it. That remains the moat and the open
problem.

## Tie-in: fair launch (Will's open question, 2026-06-11)

Will, thinking aloud: at launch either **reset the chain** (forego the creator's
early-contribution advantage) or **program an insta-burn on all pre-launch blocks**.

These are not equivalent, and the coordination-Schelling logic decides it:

- **Reset** = a *claim* of fair launch. The history is gone; outsiders must trust that
  nothing was kept. Trust-me does not make a focal point (Edge 2).
- **Genesis-burn** = a *proof* of fair launch. The chain stays continuous from genesis
  through the launch height; pre-launch blocks **exist** (provenance preserved,
  auditable) but their PoM-standing and state-value are programmatically burned to zero
  at the launch block. Anyone can verify the zeroing on-chain.

**Recommendation: genesis-burn.** It is the structurally-honest version of reset — it
*dissolves* the hidden-premine suspicion class (detection-independent: no instance has
a head-start, provably) instead of asking the network to trust a deletion. It also
preserves the pre-launch run as on-chain evidence the mechanism worked. This is the
same honesty-is-focal property from Edge 2: the fair launch must be *provable*, not
asserted, or it fails to be a Schelling point. (Decision still Will's.)

## Ties
- `WHITEPAPER.md` §5.1 (ToM → ETM → PoM), §6 (consensus), §7 (backwards-enforcement).
- [P·substrate-geometry-match] — macro fractal ⇒ micro fractal, here at consensus scale.
- [P·economic-theory-of-mind] — mind-as-economy is the inward-consensus substrate.
- [P·honesty-as-structural-load-bearing-property] — at the adoption/Schelling layer.
- [P·jarvis-substrate-decentralization-roadmap] — protocol-not-platform (Edge 1).
- [P·augmented-governance] — coordinator augments the invariant, in the middle.
- [P·class-dissolution-vs-case-defeat] — genesis-burn dissolves the premine class.
