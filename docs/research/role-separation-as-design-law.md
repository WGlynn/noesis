# Role Separation as a Design Law

> Status: research note / design-lens (ready-for-critique). The claim is a *pattern*, grounded in
> Noesis code at HEAD + two external instances (one ML, one consensus). Honest labels throughout.
> Companion memory: `role-conflation-is-the-bottleneck`. Parent frames: bottleneck-dissolution ⇐
> structure-does-the-work.

## The law

**When one mechanism is forced to carry two roles whose optimization pressures conflict, the
conflation *is* the bottleneck; the fix is to give each role its own pathway.** Optimizing the shared
carrier cannot escape the conflict — it only trades one role's loss for the other's. Separation
dissolves the constraint instead of pushing it.

This is not a metaphor we apply after the fact. It is a design lens we run *before* building any
mechanism: does this carrier serve two roles? Do their pressures conflict? If yes, split.

## Why it is a real abstraction, not a framing coincidence

The same shape appears independently across three unrelated substrates:

| Substrate | The conflated carrier | The two roles in tension | The separation |
|---|---|---|---|
| **ML architecture** | a single transformer forward pass / layer | *state maintenance* (preserve & update context) vs *prediction* (emit the next token) | the State-Prediction Separation Hypothesis: architecturally distinct pathways so each optimizes independently (arXiv 2607.01218 — see note) |
| **Consensus** | a single Bitcoin block | *transaction confirmation/propagation* vs *leader election / proof-of-work ordering* | NC-MAX (Nervos CKB): two-step propose→commit + orphan-aware difficulty, so throughput stops being hostage to the security parameter |
| **Noesis (this repo)** | several — see below | safety⊥liveness, time⊥safety, observer⊥observed, money⊥governance⊥capital | a dedicated pathway per role, as a repeated design law |

Convergence across ML, consensus, and our own stack is the evidence the abstraction is real: three
designs that never talked to each other reached for the same move because the same disease was
present. The disease is **overloading one carrier with two intents**; the cure is **a dedicated
pathway per intent** — the same shape as "never overload an implicit signal (presence / data-shape /
a magic value) to carry intent; promote it to an explicit predicate."

## Noesis runs this as a repeated design law

Every entry below is grounded at HEAD; the two clock-family entries shipped in the session this note
was written.

- **PoW's two jobs, split.** Bitcoin's PoW does ordering-*security* AND issuance/liveness/Sybil-cost
  at once. Noesis excludes PoW from finality — `FINALITY_MIX = { pow: 0.0, pos: 1/3, pom: 2/3 }`
  (`node/src/runtime.rs:1195`) — because PoW is reorgeable, so safety must not depend on it. PoW's
  role is reduced to JUL issuance + liveness + per-block Sybil-cost; finality-safety runs on PoS+PoM.
  (Overall consensus weight is still 3-dimensional: `NCI = { pow: 0.10, pos: 0.30, pom: 0.60 }`,
  `node/src/lib.rs:3820` — PoW is separated *only* from the finality-safety role, not from consensus.)
- **The clock's two jobs, split.** Reading physical time and securing the chain have opposite
  requirements (one wants an external fact, the other must never trust one). The committee-attested
  clock feeds *only* difficulty, never the finality path (`FINALITY_MIX.pow == 0`), so total clock
  compromise degrades cadence and never safety (`docs/DESIGN-committee-attested-clock.md`).
- **Halt detection's two jobs, split — the purest instance.** You cannot detect that a mechanism has
  halted *using that same mechanism*. The never-halt stall detector separates the observer (the
  committee's wall-clocks, which keep ticking through a production halt) from the observed (block
  production). The deadlock it dissolves — no blocks ⇒ no retarget ⇒ difficulty stuck ⇒ no blocks —
  is literally a role-conflation: block height was doubling as the clock
  (`node/src/liveness.rs`, `docs/DESIGN-never-halt-liveness.md`).
- **The token's three jobs, split.** Money, governance, and state-rent capital are three orthogonal
  roles most chains weld into one asset: JUL (elastic energy-money) / VIBE (governance) / CKB-native
  (PoS + state-rent capital).
- **Truth's two jobs, split.** "The rule was followed" and "the fact is true" are different claims;
  conflating them is the Chainlink/Nazarov error. Noesis keeps Validity distinct from Veracity (PoW
  gives on-chain self-evident validity; off-chain veracity is the oracle problem, never "cryptographic
  truth").

## Why this matters beyond a nice parallel

1. **It is a check, not a story.** Making the law explicit turns it into a pre-build predicate ("does
   this carrier serve two conflicting roles?") — itself the promote-implicit-to-explicit move applied
   to our own design process.
2. **It is the spine of the contribution-measurement thesis.** "Measured contribution dissolves the
   redistribution dichotomy" is itself a role separation: *measuring* value and *reallocating* value
   are welded together in a treasury/tax mechanism; separate them and the redistributive step (and the
   socialist framing that rides on it) disappears — you are left with a price on measured value. See
   `contribution-measurement-dissolves-redistribution-SEED.md`.

## Honest boundaries

- The two external instances are cited for the *pattern*, not asserted at engineering precision. The
  SPSH claim is from a thin automated read of arXiv 2607.01218 — verify against the paper before any
  publication leans on its specifics. NC-MAX's two-step and orphan-aware difficulty details should be
  re-grounded from the CKB/NC-MAX source before citation.
- The law is a *lens*, not a theorem: not every two-role carrier must be split (splitting has its own
  cost — extra pathways, coordination). The trigger is *conflicting optimization pressures*. When the
  roles are aligned, welding is fine. The judgment is whether the pressures actually pull apart.
