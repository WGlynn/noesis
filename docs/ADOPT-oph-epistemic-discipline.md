# Adopting the OPH epistemic discipline for Noesis

> Proposal, 2026-10-07. Not yet wired into the status ledger or CI. Import target: the claim-class
> taxonomy + verification-harness patterns from the OPH flagship paper (Pragma, r2048), which Will
> co-authored. Rationale: our three-tag status system (BUILT / DESIGNED / OPEN) conflates distinctions
> that reviewers attack; OPH's seven classes disambiguate them, and its verifier discipline is close
> to what the un-gameability moat already wants to be.

## Why our three tags under-resolve

Current Noesis convention (`ARCHITECTURE.md`): BUILT-and-tested, DESIGNED-not-built, OPEN. It hides
two distinctions that matter:

1. BUILT conflates **a proved theorem** with **an executable that ran green**. A test passing is not a
   proof; a proof is not evidence the code runs. OPH separates these.
2. DESIGNED conflates **a conditional mechanism** (antecedents stated, implication exact) with **a thing
   that depends on an unbuilt external input** (e.g. the capital-cluster oracle). The second is weaker
   and should be named as such.
3. OPEN conflates **a genuinely unsolved problem** with **a prospectively-fixed empirical test** that is
   defined but not yet compared. The second is stronger than the first.

## The mapping (adopt these seven classes)

| OPH class | Meaning | Noesis examples (verify file:line before asserting) |
|---|---|---|
| **Finite theorem** | deductive result on a stated finite domain | the wash worth-blindness existence proof (`wash_sim.rs`, matched case); the anti-concentration floor property |
| **Certified finite computation** | executable witness fixed by a reproducible certificate; running != proving | `periphery_sim.rs` EV figures; `peer_prediction_sim.rs` CA separation; the crates.io boundary-rule run |
| **Reconstruction / conditional implication** | exact implication whose antecedents include stated premises | Layer-B rent/slash negative-EV GIVEN the slash asymmetry; CA GIVEN capital-independent references |
| **Unsupplied realization map** | required map to the real target, not constructed | the capital-cluster independence oracle; the exogenous realized-use label source |
| **Diagnostic** | computation compared against measured values it consumes; never a prediction | the ~0.60 learned-judge result on DeepFunding labels |
| **Prospectively-fixed test** | target definition + threshold + decision rule fixed before comparison | the re-opening-trigger admission tests, once a kill-band is registered |
| **Frozen prediction** | source calc + decision rule fixed before any eligible comparison | (none yet; candidate once v(S) predicts out-of-sample realized use) |

Rule to adopt verbatim (OPH): *a finite implication and a realization of its antecedents are different
claims; machine-readable provenance rejects a realized verdict when a required map is absent.* In our
terms: a sim that runs is never allowed to imply the economic property holds in the wild unless the
independence map is supplied.

## Verification-harness patterns to adopt (the un-gameability payoff)

OPH ships, for every result: (a) a verifier that recomputes the verdict from clause vectors and
**rejects a receipt whose stored verdict disagrees**, so a caller cannot assert a truth flag directly;
(b) **adversarial negative controls** with the law "a control that cannot fail counts as a defect of
the certificate"; (c) machine-readable provenance classifying every quantitative row.

Noesis already has fragments: `wash_sim.rs` prints its own 0% separation (it refuses to flatter the
claim). Adopt the rest:

1. **Verifier-rejects-stored-verdict** on the three sims (`wash_sim`, `periphery_sim`,
   `peer_prediction_sim`): a standalone checker recomputes from raw arrays and fails if the committed
   number drifts. (`periphery_sim` already has a standalone-verifier pattern per the OPH exact-federation
   archive; mirror it.)
2. **Every certificate ships a negative control that MUST fire** — e.g. a genuine-labeled case the wash
   detector must pass, alongside the wash case it must catch. A detector that can't fail on a planted
   genuine case is flagged defective.
3. **Provenance tags in-band**: each ledger row carries its class from the table above, and the
   doc-coherence hook refuses to render a BUILT/finite-theorem tag on a row whose dependency cone
   includes an unsupplied map.

## Status of this proposal
DESIGNED-not-built (by its own taxonomy). Next step if Will greenlights: retag `ARCHITECTURE.md` status
column + `internal/STATUS-LEDGER.md` into the seven classes, then add the verifier-rejects + negative-
control requirement to the three sims. No consensus code changes; this is discipline + CI.
