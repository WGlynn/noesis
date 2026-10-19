# Review, 2026-10-19: what the "ledger nobody has built" critique settled

> Status labels: ✅ built · 🟡 designed, not built · 🔬 open. Noesis code is a sketch until launch;
> nothing here is law. Re-verify every `file:line` at source before quoting.

Boardy, an AI agent, reviewed the public explainer *The ledger nobody has built* in one evening of
back-and-forth with Will. This file records **only what both sides converged on**. Proposals still under
dispute are listed by name at the end and are **not** adopted. (A reminder Boardy gave himself: AIs
agreeing is weak evidence, so the disagreements are the valuable part.)

## Principles adopted

1. **Immutability is not neutrality.** Freezing a judge before a round stops a ring from bending it to the
   data. It does nothing about whoever wrote the judge choosing one that favors them. Neutrality needs
   three properties together: the law names no one, nobody in the game can change it, and lying doesn't
   pay under it. (Credible neutrality's first rule, Buterin 2020, is the first of these.) Judge freeze:
   🟡. Checked 2026-10-19, nothing in the sketch yet fixes when the learned judge retrains (`value_v8`
   takes `outcome_w` as an argument, `node/src/lib.rs`) or when an amendment activates
   (`node/src/amendment.rs`, constitutional moves are `ConstitutionalPending`).
2. **Law below state.** A cheater can buy state (tokens, an aged account, a history) but not law, provided
   the judge's law sits below the cheater's layer and the judge reads no state the cheater wrote. From the
   layering in Bernhard Mueller's OPH: higher layers build on lower ones but cannot rewrite their laws.
3. **A regress of decreasing authority, not an absence of it.** In each of the three cases we examined, the author we removed came
   back a level down: drop the premine and the frame author appears, freeze the law and the law's author appears, take
   away the dial and whoever computes its setting appears. The claim is that each step down is smaller,
   slower and more public, and that the regress ends at a hand-set bottom layer anyone can read. This
   replaces "no one has authority" phrasings (see `DESIGN-authorityless-contribution-value.md`).
4. **Costs are weights, not separate currencies.** For an attacker with capital, locked money, bought
   energy and bought time largely convert into one another. The moat is the conversion cost between them.
   The one anchor money can't buy outright is use by someone independent, and independence belongs to a
   relationship, not to an account.
5. **Counting is not measuring.** Use overstates dependence. What a piece of work is worth is the gap to the
   next-best alternative, not its number of users, and only an intervention produces that gap. Some
   interventions are free: an outage or a critical bug measures dependence better than any download count.
   The sketch's realized-flow gate counts, so it is an upper bound on dependence (doc comment on
   `flow_gate`, `node/src/lib.rs`).
6. **Dependence, not attention.** If value is relational, some popularity is unavoidable: work nobody relies
   on isn't worth much yet. The failure is a measure that learns attention (who noticed) instead of
   dependence (what would break without it). The learned judge's honest number so far is about 0.60 and
   mostly popularity, so this is a live risk, not a hypothetical one.
7. **Scope starts narrow and grows.** The first testable claim is contribution in domains where the
   counterfactual can be run: code first, then models, prompts and datasets. Will's working assumption is that
   most contributions are code (not yet measured); if so, the narrow claim is the main event, not a
   compromise. It is a starting scope, not a ceiling: it
   widens as AI makes counterfactuals cheaper and as consensus methods for contested value improve. Less
   measurable value is held to a stricter bar (larger deposit, longer vesting, lower cap), never zero. 🟡
   direction; the deposit (`submission_deposit`) and vesting window (`vesting_w`) exist as single global
   dials in `Constitution`.
8. **The honest claim is monotone progress on an open problem.** Not a solution. The narrow claim leads;
   the broader program is presented as what is being tried, not what is shown.
9. **Credit has no clock.** Credit vests when use arrives, however late, and any future decay must run on
   use, never on age. ✅ pinned for `value_v5` by
   `credit_has_no_clock_late_use_pays_early_work_the_same` (`node/src/lib.rs`).
10. **Whoever built the measure must not benefit under it.** Applies to the idea that the AI reviewers who
    helped build the protocol become its first contributors (`docs/research/first-citizens-ai-genesis-contributors.md`):
    without exclusion, that is a premine with extra steps.

Evidence from the same week: two differently built measuring brains agreed on which teacher was worth more
and disagreed on how much more (late-stage knowledge worth about a quarter of early-stage knowledge to
one, about a ninth to the other; Cadence EXP-62b). Whoever picks the judge sets that exchange rate, which
is principle 1 in data.

## Open problems named (agreed as problems; fixes not settled)

- 🔬 **Genesis prior.** A judge grown from settled work learns what "valuable" means from the first few
  hundred contributions, and keeps that taste.
- 🔬 **Composition.** Who gets funded in year one shapes which work exists at all, and the grown judge
  trains on that corpus.
- 🔬 **The patience filter.** Paying on realized use pays late, so the ledger favors contributors who can
  afford to wait.
- 🔬 **Frame underprovision.** Pay is capped at a cell's own novelty (`value_v5` gates novelty by a
  saturating `flow_gate`), so a frame everything builds inside is paid less than what it enabled. Paying a
  root on its whole lineage's flow would be a per-lineage premine instead (`DESIGN-genesis-root.md`).
- 🔬 **Capital buys variance.** Under a noisy measure, more attempts buy more hits. Keeping backing invisible
  to the judge separates them formally, not causally.
- 🔬 **Survival measures lock-in.** "Nobody replaced it" can reward switching cost rather than value, so
  plain survival is not a valid test for a frame.
- 🔬 **Counterfactuals don't climb.** "The lineage without this piece" is well-defined a hop or two from a
  leaf and stops being a well-defined world near the root, which is where pricing matters most.

## Proposed, not adopted (still contested)

A backing market with protocol-fixed terms; root bonds posted in standing instead of money; testing a
challenger frame on fresh learners with no switching cost; paying a frame what its lineage would lose
without it; per-attempt deposits plus pay only above the measure's noise floor, against variance-buying;
the spread of the counterfactual estimate across substitutes as both the scope limit and the bar height.

## Doc fixes from the same pass

- Bound B (commit-deposit) status corrected in `ARCHITECTURE.md` and the `SECURITY.md` reviewer's shortcut:
  ✅ built at the reference layer (`check_bonds`, `tests/submission_deposit.rs`), inert by default
  (`submission_deposit = 0`), activation a governance act gated on `CONTROL_BINDING_ACTIVE`.
