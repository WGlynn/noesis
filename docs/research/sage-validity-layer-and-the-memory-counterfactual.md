# SAGE and Noesis: a validity layer, a value layer, and the counterfactual that prices memory

> **Status: research note, ready for critique (2026-10-20). Nothing in Noesis changed.** A comparison with
> a neighboring open source project, written to find where each one's work helps the other. Sources:
> SAGE `main` at `76ba18d` (v11.23.27) and its four papers (Zenodo DOIs 10.5281/zenodo.18856658, .18856774,
> .18856845, .18888597); Noesis at `69b43f08`. Noesis code is cited as the current sketch, not as law.
> Status words as everywhere in this corpus: built, designed, open.

## 1. What SAGE is

SAGE ((Sovereign) Agent Governed Experience, github.com/l33tdawg/sage, Apache-2.0, by Dhillon Andrew
Kannabhiran) is governed memory for AI agents. An agent signs a memory, a CometBFT validator network
admits or rejects it by a weighted 2/3 vote, and later agents retrieve what was admitted. The vote
weighting is called Proof of Experience (PoE): Paper 1 gives it as a weighted geometric mean of a
validator's past accuracy, domain relevance, recency and corroboration, with a 10% cap per validator.

It is a shipping product with real operational discipline: consensus changes are fork-gated so old blocks
replay byte-identical, and the repo publishes its own corrections (the papers README narrows an earlier
reproducibility claim). It also has gaps, as Noesis does. The ones that matter here are below, each paired
with what Noesis learned on the same problem.

## 2. The shared wall: validity is not veracity, and every justification has to stop somewhere

SAGE's README says it plainly: the checks "govern what is committed; they do not prove every remembered
claim is true." That is the distinction Noesis writes as **validity versus veracity**
(`docs/DESIGN-committee-attested-clock.md` section 1): cryptography and consensus certify that a record is
well formed, signed and admitted. Whether it matches reality is settled by incentives, or by something
outside the vote. Outside proof of work, nothing on any chain has cryptographic veracity.

The second half is the Munchhausen trilemma: every chain of justification ends in an infinite regress, a
circle, or a chosen stopping point. Each system picks one, and the honest ones say which.

| System | Where justification stops |
|---|---|
| Bitcoin | A stopping point in physics: `hash <= target` is checkable from the artifact alone. |
| SAGE | Two places. Validator "accuracy" is agreement with the eventual consensus, which is a circle. Above it sits a human Governance Authority that can veto from outside the BFT model (Paper 1 section 2.1), which is a chosen stopping point, stated openly. |
| Noesis | Proof of work for on-chain facts. For value, "another independent mind built on this and it held up," which `docs/DESIGN-mind-scarcity-asymmetry.md` shows is a recursion. It ends only in the scarcity of independent minds, and that base case is open. |
| OPH | A circle: the world as the fixed point of observers reading each other (`docs/CONVERGENCE-oph-noesis-cadence-fixed-point.md`). |

## 3. Agreement only counts where disagreement was possible

One observation ties together everything found in this comparison.

**SAGE's validators agree by construction.** At `76ba18d` a node's vote is a fixed rule check
(`internal/voter/decision.go`): not a duplicate, at least 20 characters, no noise phrase, and a confidence
of at least 0.3 (0.7 for facts). The proposing agent supplies that confidence itself. Every honest node
runs the same rules on the same input, so they vote the same way, and "accuracy as agreement with
consensus" cannot tell one validator from another. Agreement that could not have gone otherwise carries
no information.

**The one signal in SAGE that could have gone otherwise is the red team.** In Paper 4 a separate model
attempts a blind solve of each challenge the pipeline built. The flag is captured or it is not, and that
result can be checked against the artifact. The learning trend (Spearman rho 0.716, p 0.020, n 10, against
rho 0.04 for an expert-prompt control) appears once those results are fed back into memory. The paper's
own caveats apply: small n, an ordinal difficulty scale, and two routing fixes made mid-run. But the shape
is clear: the vote over memories did not produce learning; piping in an outside signal did.

**The same rule found a bug.** Each SAGE node is internally consistent about its PoE weights. Comparing
two CPU architectures exposed a divergence: on arm64 the Go compiler fuses the weight sum into
multiply-adds and on amd64 it does not, and in an emulation 14.0% of realistic inputs produce different
float64 bits. Those bits feed the state hash, so a mixed quorum would likely stop agreeing. No single
machine could have seen it; only the comparison between machines that could disagree did. Filed as
l33tdawg/sage#458 with a reproduction. Noesis avoided this class by writing its finality rule in Q32.32
fixed point (`onchain/noesis-core`, `finalizes_pos_pom_fixed`; see `ARCHITECTURE.md`).

**And it restates the wash ring.** A closed ring of minds that only vouch for each other passes every
consistency check, because there is no outside overlap that could fail. Consistency over no overlaps is
trivially satisfied. OPH's world is a fixed point too, but it is the fixed point over all overlaps; a wash
ring is a private fixed point that never meets anyone it did not write. That is what the periphery design
asks for (`docs/DESIGN-periphery-solution.md`, layer A: value vests only on use by capital-independent
minds) and why the mind-scarcity base case is the real one. The observation does not close that base
case. "Could have disagreed" still needs the other side to be independent, and independence is exactly
what is open. What it adds is a test that applies the same way to validators, judges, CPUs and rings:
ask whether the agreement could have gone the other way.

Will's framing of why notes like this one are worth writing: collaboration across projects is itself
observer-overlap consensus. Two projects that built different records of the same problem compare them,
and the places they disagree are where each learns something.

## 4. Institutional memory is a counterfactual you can actually run

Most contributions cannot be priced by their counterfactual, because the world cannot be rerun without
them. Nobody can replay the last ten years without a given paper. Noesis therefore falls back on evidence
of downstream use. Where the counterfactual is computable, Noesis already uses it: `causal_share`
(`node/src/lib.rs`, at line 4769 in `69b43f08`) reruns the value function with and without a certifier's
standing and takes the difference.

A memory is different, because its consumer is an agent that can be replayed. SAGE already ships the
switch: the memory layer toggles with an environment flag (Paper 1 section 10.1), and Paper 4's control
arm is the whole-memory counterfactual. Pricing one memory follows directly. Mask it from retrieval, rerun
the same tasks under the same frozen judge, and take the difference; that is leave-one-out. Redundant
memories each score near zero under leave-one-out (removing either one changes nothing while the other
remains), so the fair split is the Shapley value over memories, estimated by permutation sampling as the
Noesis whitepaper does for contributions (`docs/WHITEPAPER.md`, Estimation). The proposing agent's
standing is then the sum of what its memories measurably added.

The definition is easy; estimating it is expensive, which is where Noesis is too. The costs are concrete,
and each has a known move.

- Agent runs are noisy, so every counterfactual needs repeats. The move is to budget repeats per memory
  and report an interval, not a point.
- The judge is the base case. Paper 4's own limitation 8 (the operator's growing familiarity may have
  shifted the ratings) is a judge absorbing state from what it judges. The move is to freeze the judge
  model, rubric and prompt before the run, keep it blind to which arm it is scoring, and give it no state
  the system wrote (`docs/REVIEW-2026-10-19-ledger-explainer.md`, "law below state").
- A memory that is never retrieved measures zero, so its price depends on the retrieval policy (Paper 4
  limitation 9: two routing fixes changed results). The move is to measure two numbers separately: what
  the memory adds when it is forced into context, and how often retrieval actually surfaces it. The first
  prices the contribution; the second prices the routing.

### What this does to the measured scope

Noesis's scope rule is to start as narrow as today's measurement forces and widen with the science, with
less measurable value earning under stricter terms (larger deposit, longer vesting, lower cap) rather than
nothing. The v1 scope has been domains where the counterfactual can be run: code, models, prompts,
datasets.

Replayable agents widen that by one notch, in kind rather than degree. Written know-how (lessons,
postmortems, design decisions, explanations of what failed) is usually treated as unmeasurable, because
its effect lives in people. When an agent consumes it, its effect can be measured the same way as code:
the same task, with and without it, under a frozen judge. The widened scope is therefore any written
contribution that an agent consumes on a task with a fixed external judge. What that makes possible is
pricing know-how by what it measurably changes, per contributor, instead of waiting for evidence that
someone later built on it. Two limits keep it honest. It
measures value to the agent on the judged task, so know-how that helps people but not agents still scores
zero here and falls under the stricter terms. And it widens only as far as frozen judges exist, so the
boundary is a property you can check per task, not a promise. Status: designed. The counterfactual
machinery for contributions exists in the current sketch; nothing applies it to agent memory yet.

## 5. Overlap map

| Concept | SAGE (at `76ba18d`) | Noesis (at `69b43f08`) |
|---|---|---|
| Consensus weight that is not capital | PoE: accuracy, domain, recency, corroboration (`internal/poe/engine.go`) | PoM in the finality mix, PoS 1/3 to PoM 2/3 with proof of work excluded (`node/src/runtime.rs` `FINALITY_MIX`) |
| Neither factor finalizes alone | Geometric mean, described as "joint competence," but every factor is floored at 0.01 (`EpsilonFloor`), so a zero does not zero the weight | Each of PoS and PoM must independently supply at least 50% of its own dimension (`MIN_DIM_BPS`) |
| Concentration cap | 10% per validator (`RepCap`), no organization above 1/3 (Paper 1 section 7) | The per-dimension floor; per-identity caps in the sybil simulation |
| Signed provenance | Agent-signed memories, append-only revision chain | Per-block Ed25519 signing, signed Merkle root |
| Consensus-safe arithmetic | Sorted-key summation fix; float `log`/`exp` remain (sage#458) | Finality in Q32.32 fixed point |
| Base case, stated | Human Governance Authority outside BFT | Proof of work on chain; mind scarcity for value (open) |
| Learning from outcomes | Paper 4 red-team loop | Learned `v(S)` on real labels (open) |
| Collusion | Phi tracker exists, not yet fed votes at `76ba18d` | Wash and sybil simulations; a funded cartel is the named unclosed case |

## 6. Where each could borrow from the other

**From Noesis, for SAGE** (offered as one project's experience, not a fix list):
- Score validators against the outside signal rather than against the final vote: credit a validator when memories it accepted later held up under the red team.
- Price memories by their counterfactual (section 4), so "experience" means demonstrated contribution.
- Keep "admitted" (validity) and "verified" (veracity) as separate statuses.
- Do not let the proposing agent's own confidence score decide whether its memory is admitted. A record should never be the only judge of its own value (`docs/WHITEPAPER.md`, self-reference and circularity).
- Freeze the judge before each round.
- Compute consensus weights in fixed point (filed as sage#458), and enforce a real per-factor floor if joint competence is the intent.

**From SAGE, for Noesis:**
- A replayable outcome harness with a control arm is the data shape the learned `v(S)` needs: contributions in, an outside judge's result out, and the ability to remove one input and rerun. It is the closest working instance we have seen of the exogenous evaluation task the convergence note names as the shared open item, with one gap: the judged tasks are generated by the system being judged, so the judge is independent but the task is not yet disjoint.
- Fork-gated, byte-identical replay is a mature pattern for the governance version bumps Noesis plans for activating value-layer changes.
- Publishing your own corrections next to the claims is the same honesty-as-moat stance, practiced.

## 7. What this note does not claim

We did not rerun any SAGE experiment; the pipeline and run records are not public, as SAGE's papers
README discloses. The 14.0% divergence is an emulation on amd64, not a run on arm64 hardware. The OPH
reading comes from this corpus's convergence note and is an analogy, not a theorem. The widened scope in
section 4 is designed, not built.
