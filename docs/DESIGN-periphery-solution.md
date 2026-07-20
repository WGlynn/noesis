# Manufacturing a Periphery — the solution shape for wash-discernment

> **Status: DESIGN / solution-shape, ready-for-critique (2026-07-20).** This is the constructive answer
> to the open problem named in `DESIGN-mind-scarcity-asymmetry.md` and measured in
> `node/examples/wash_sim.rs` (0.0% graph-internal separation of genuine vs acyclic wash). It is a
> composition of existing pieces around a new anchor, not a new primitive, and not a claim of "solved."
> The terminal residual is named in §6. Status discipline: ✅ built · 🟡 designed · 🔬 open.

## 1. The reframe that dissolves the impossibility

`wash_sim` proves no *worth-blind, graph-internal* signal separates a genuine collaboration from a
competently-built wash operation — they are structurally identical. A classifier is therefore the wrong
object. The brain does not classify neurons as honest; it **grounds** them: a signal earns influence
only if it predicts an external reality it cannot fake, and a spurious circuit is pruned because
maintaining it *costs metabolic energy it never earns back.* Discernment is not verification; it is an
**economic filter anchored to a periphery.**

So the solution is not "find the discriminant." It is: **make genuine building net-positive and wash
net-negative, by anchoring reward to an external, un-washable grade.** The honest and the profitable
become the same population — [[filter-coincidence-as-structural-edge]]. This is the move the whole stack
was already reaching for; this note names it and composes the pieces.

## 2. Three layers that manufacture a periphery

Each corresponds to a brain mechanism and to an existing Noesis piece. None suffices alone (consistent
with every finding this session); together they close the gap up to a named terminal assumption.

### Layer A — the anchor: value vests on realized use by *capital-independent* minds

The periphery is **realized external use**: a contribution vests value only when a *later, independent*
contributor builds on it. "Independent" is the load-bearing word, and it is anchored **not in personhood
(a capturable authority) but in capital** — the one substance a ring cannot forge cheaply (Bitcoin's
lesson). The anti-concentration floor already forces PoM ⊥ PoS independence (`runtime.rs` `MIN_DIM_BPS`,
✅ built). Extend it to *vesting*: downstream use counts toward a contributor's standing only when the
using identity's stake is provably not controlled by the used identity (distinct stake origin). A closed
wash-ring can mint identities for free but **cannot mint independent capital for free** — so to fake
"someone independent built on this," the attacker must actually post independent capital, which is the
funded-cartel boundary, now *priced* instead of free.

- Brain analog: the sensory periphery — reward grounded in reality, not in other neurons.
- Status: floor ✅ built; capital-independent *vesting* gate 🟡 designed (this note).

### Layer B — the pruner: carrying cost + slash-at-risk make patient wash negative-EV

The standard objection — *patient farmers beat every time-lock* — is defeated not by longer time but by
making the wait **cost more than the harvest in expectation.** To hold un-vested standing through the
window, an identity pays a Harberger-style carrying cost (state-rent / bond, ✅ substrate present) and
stakes slash-at-risk resolved by the dispute market (✅ `DISPUTE-SLASHING.md`). A wash-ring must fund N
identities' rent for the full window *and* survive challenge, while its closed-loop use produces no
external vesting (Layer A). If `E[harvest] < rent + E[slash]`, patient wash is strictly negative-EV and
prunes itself.

- Brain analog: metabolic cost — a spurious circuit that earns no predictive reward is net-negative and
  is downscaled in consolidation (Tononi SHY). **Carrying cost = metabolic cost.** This is the asymmetry
  `DESIGN-mind-scarcity-asymmetry.md` §4 said the theory hadn't named: it is not time, it is *rent*.
- Status: rent substrate ✅; dispute market ✅; the Harberger self-assessed price that sets rent+slash
  from a self-declared `V` is 🟡 designed (2 open theorems, `something-from-nothing` §5.3).

### Layer C — grounded learning: v(S) stops being null once the periphery emits labels

The learned v(S) is null *because it was trained on the inside* — structural features of the graph, with
no external target. Once Layer A exists, it emits exactly the label v(S) needs: **which contributions
were built upon by independent minds and held up.** Train v(S) to predict *that* — external realized use
— and it is no longer predicting a topology artifact; it is the brain's dopamine reward-prediction-error,
grounded in an outcome the ring cannot fake. The null was a mis-specified-instrument result
([[adversarial-instrument-for-mis-specified-null]]); the correctly-specified target is external use, and
it only exists once there is a periphery to measure it against.

- Brain analog: reward-prediction-error / predictive coding — trusting a signal iff it reduces surprise
  about the world.
- Status: 🔬 open, data-gated — but now with a *named data source* (Layer A's vesting events), not "we
  need labels from somewhere."

## 3. Why this composes rather than stacks

The three are one loop, not three defenses: Layer A defines the external grade, Layer B prices the wait
so only genuine contributions survive to be graded, Layer C learns to predict the grade and feeds it
back into scoring. Remove any one and it reopens — A without B lets patient wash wait for a lucky
external touch; B without A prices a wait against nothing; C without A trains on the inside and is null
again. This is why every single-road attempt (semantic-only, personhood-only, time-only) failed: the
solution is the *cycle* that manufactures what a brain is born with — a body wired to a world.

## 4. Noesis already has the body — it is called real economic use

The periphery is not hypothetical. **VibeSwap-on-Noesis is a periphery**: a DEX is real external use with
real capital staked on whether a contribution (a mechanism, a fix, a pool) actually works. Any
application layer where outcomes are settled against reality — money moved, a product shipped, a market
cleared — is a sensory surface a wash-ring cannot fake, because faking it requires actually delivering
the outcome (at which point it is genuine). The `RELEASE-PLAN-VIBESWAP-ON-NOESIS.md` endgame is, in this
frame, Noesis growing its first sense organ.

## 5. What to build, in order (measurement-first)

1. **Extend `wash_sim` with Layers A+B priced in** — add capital-independent vesting + carrying cost, and
   measure the wash-ring's EV. Target result: `E[wash] < 0` at honest parameter ranges, with the
   break-even capital cost reported (the "price the ring ≤ 0 before build" gate, `DESIGN-corroboration.md`).
   This is the numeric proof the composition works, or the honest boundary where it doesn't.
2. **Capital-independent vesting gate** (Layer A) — the smallest consensus-affecting piece; build cold,
   Will-gated, after the finality decision.
3. **Harberger self-assessed price** (Layer B) — sets rent + slash from `V`; 2 open theorems first.
4. **Re-train v(S) on realized-use labels** (Layer C) — the moment Layer A emits vesting events; the
   held-out harness already runs unchanged (`OUTCOME-EVALUATOR.md`).

## 6. The terminal residual (honest floor — do not round up)

This defeats wash **up to the cost of independent capital deployed across the window.** A funded cartel
that commands genuinely independent capital on N identities, pays the rent, and waits — a
majority-capital adversary — is the irreducible residual. That is not a weakness; it is the **same
security class as Bitcoin's 51%**: to break it you must out-resource the honest majority, and the
resource is capital anchored to reality rather than hashpower. Terminating there is a *good* place to
terminate — it is where every credible decentralized system bottoms out. The claim this note supports,
bounded exactly: *wash is negative-EV against any adversary who cannot cheaply command independent
capital; a majority-independent-capital cartel is priced at 51%-class cost, not structurally excluded.*

## 7. One line

You cannot tell an honest contribution from a wash by looking at the graph — no more than a brain in a
vat can tell true from confabulated. You solve it the way the brain does: give the collective a
periphery (realized use by independent capital), a metabolic cost (rent + slash) that prunes what the
periphery doesn't reward, and a grounded value model (v(S) predicting external use, no longer null). The
honest and the profitable become one population, and the residual is Bitcoin's — out-capital the honest
majority or lose. That is how a collective intelligence tells contribution from extraction without an
authority: it grows a body.
