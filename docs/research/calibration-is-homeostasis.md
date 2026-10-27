# Calibration is Noesis's homeostasis: the success score

*Decision, Will Glynn, 2026-10-27. Status: adopted as the target success measure. Not yet measured; no code computes it yet.*

## The decision

The unbiased measure of whether Noesis works is **calibration: how well Noesis's valuations today predict independent use later.**

## Why this, and not something else

Any judge inside a system is biased toward what the system selected. The contributors Noesis rewards will tend to find its scoring fair, because it rewarded them. A scoring rule tends to look good to whoever it keeps (survivorship bias). So the success measure should not rest on a judgment made from inside.

A planet's measure of success is homeostasis, and homeostasis is not a judgment. It is a physical variable held within bounds against conditions the system does not control: surface temperature, against what life can tolerate, while the sun brightens. Noesis needs the same kind of measure: a variable that the outside world sets and that Noesis cannot argue with.

The variable is the gap between what Noesis says a contribution is worth and what independent parties later actually do with it. A healthy Noesis keeps that gap small and stable, including under attack. That is calibration, and it is Noesis's homeostasis.

The Economic Theory of Mind program (github.com/WGlynn/ETM) proposes that the same variable is regulated at every scale. An organism holds its internal state steady against its environment. A brain, on the predictive-processing account, keeps its prediction error small. A market's prices are supposed to anticipate scarcity. Noesis's prices should anticipate use. In each case, health would be a small, stable gap between what the system expects and what the outside world then does. This is a proposal, not yet a tested result.

## Working definition

For contributions valued at time t, compare their value ranking with the independent use they receive over the following window (t to t + delta), using a rank correlation or a proper scoring rule. Healthy means high and stable across windows, and stays so when the system is under attack.

Two axes together make the full picture:
1. **Calibration** (above): are the prices right?
2. **Persistence under perturbation:** does the system keep finalizing safely while attacked? See the finality thresholds measured in the ETM program (experiment N1): with the current rule, halting finality needs far less contribution standing than capturing it, so liveness is the soft spot.

## Independence is a price, not a fact

True independence does not exist: any two users share some upstream (money, friends, information, incentives). So "independent use" cannot be a yes-or-no test. The working line:

- **A user's use counts to the degree that faking it would cost more than the reward it unlocks.** Independence is a cost to the attacker, not a property of the user.
- Use counts more when the user puts something at risk on it that the contributor does not control: building on it with their own capital, paying for it, staking on it.
- Use is discounted by closeness to the contributor in the funding and ownership graph (the capital-cluster independence oracle, designed in ARCHITECTURE.md, not built).
- This matches the honest floor already stated in ARCHITECTURE.md: for an attacker with capital, locked money, bought energy and bought time largely convert into one another, so the moat is the conversion cost.

Nature works the same way. A legume does not ask whether its root bacteria are independent. It responds to the nitrogen actually delivered, which is physically expensive to fake. Nodules that fix no nitrogen lose much of their reproductive success, most likely because the plant cuts their oxygen supply (Kiers et al. 2003, *Nature* 425: 78-81). Noesis's version of "delivered nitrogen" is use that is costly to fake.

## A fully realized Noesis does this at every scale (Will, 2026-10-27)

Calibration with cost-to-fake inputs is not one check at the top. A fully realized system implements it at every scale, the way living systems regulate at the cell, organ, organism and ecosystem levels, each against its own conditions. Two cautions: regulation costs something at every level (living systems pay in energy; Noesis pays in measurement and lag), and "every scale" does not mean fragmenting consensus. The ETM program's experiment N1 found that splitting finality into patches helps a strategic attacker, so each scale grades its own gap while security stays aggregated.

Where Noesis stands at each scale (statuses as of 2026-10-27, commit be219951 unless noted):

| Scale | What calibration means there | Status |
|---|---|---|
| Function | A valuation function's outputs are graded against later use, and its inputs are counted by cost to fake. | The rule that decay may run only on use, never on age, is pinned by a test (`node/src/lib.rs:1150-1156`). `value_v5` (pays by downstream use) is built and kept off the live path. Nothing grades a function's outputs yet. |
| Contribution | A contribution's value vests on realized use by capital-independent parties. | Designed and simulated (`node/examples/periphery_sim.rs`), not built (ARCHITECTURE.md, periphery). |
| Participant | A participant's standing reflects use of their work that is costly to fake. | The capital-cluster independence oracle is designed, not built. The deployed v0 per-identity cap is farmable (`docs/research/WASH-PROBLEM-INDEX.md`, row 15). |
| Protocol | Finality keeps working under attack (persistence), and the value layer stays off the safety path until graded. | Value layer kept out of finality, pinned by a test (`node/src/runtime.rs:3209`). Liveness is the soft spot: halting needs far less standing than capturing (ETM N1). |
| Ecosystem | Noesis's value signal is graded by its consumers outside Noesis, and rival chains can merge in. | Other chains consuming the signal is described in `docs/FAQ.md`; the convergence mechanism is not built. |

No scale is fully realized yet. The table is the build list.

## Known failure modes

1. **Fake outsiders.** If "independent" users are not, the score is gamed. This is the grounding problem, still open.
2. **Self-fulfilling scores.** High scores bring exposure, and exposure brings use. Fix: randomize exposure for a slice of contributions, so some use is measured without the score's influence.
3. **Lag.** Calibration needs time. Fast proxies must stay off the money path, as finality already does: the test `standing_and_finality_are_structural_novelty_only_no_value_layer_leak` (`node/src/runtime.rs:3209`) keeps the value layer out of finality.

## First test (proposed, not run)

Use the existing crates.io data (`data/crates/`): value crates at one point in time with the Noesis value model, then measure whether the values predict reuse growth by unrelated owners in the following period. To be pre-registered with written hit and miss criteria in the ETM repository before it runs.
