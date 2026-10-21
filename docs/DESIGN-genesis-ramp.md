# DESIGN: the genesis ramp (proposed 2026-10-21)

**In one line: contribution's weight in finality grows as fast as its independence can be shown, and no
faster.**

**Status: PROPOSED.** Not ratified, not built. Code cited is the current sketch, not law (pointers checked
2026-10-21 at `8083bb05`). Companion to `docs/RULING-2026-10-21-stall-over-hack.md` and SECURITY.md section 5.

## The problem

Since `42f83715` each side of finality is counted as a share of its own total. Capital holds the whole
finality vote at genesis, and exactly a third of it the moment any contribution weight exists. That step
has two costs, and they share one cause:

1. **A hack path.** The first contributors to vest carry the contribution side's full 2/3 at once. So a
   small captured early contribution side, plus half the stake, can finalize. Before normalization, with
   raw units and stake dominant, that took about 2/3 of the stake.
2. **A stall path.** The half-of-each-side rule binds as soon as contribution weight exists. Half of a
   handful of early contributors can stall finality, and so can their simple absence, which the stall
   ruling names as the likeliest stall.

The shared cause: the contribution side gets its full constitutional weight before it is hard to capture
or reliably online.

## The proposal: phase contribution in by measured diversity

One number `r` between 0 and 1, the ramp, computed from finalized state every epoch.

- **Mix:** stake weighs `1 - (2/3)r`, contribution weighs `(2/3)r`, each still measured as a share of its
  own total. Once contribution exists, capital's share of the finality vote is `1 - (2/3)r`. That's 1 at
  `r = 0` and 1/3 at `r = 1`. **This is the curve to publish as the falsifier.**
- **Contribution's floor:** the contribution side must supply `r` times the usual floor (`MIN_DIM_BPS`,
  `runtime.rs:1676`) of its own weight. That's nothing at `r = 0` and half at `r = 1`.
- **Capital's floor stays at half, always.** Safety never rests on the contribution side alone.
- **Unchanged:** the 2/3 bar, the per-side normalization, the silence ruling (silence counts as no,
  forever), and everything anyone earns. The ramp changes how much contribution weighs in finality while
  the chain is young. It never changes credit, standing or rewards. That's the lesson of the JUL emission
  ramp deleted on 2026-07-14: identical work must earn identically.

What it does across the range, for a captured contribution side voting as one (`b = 1`):

| r | contribution's weight | its floor | stake a captor needs |
|---|---|---|---|
| 0 (genesis) | 0 | 0 | 2/3 (stake alone, as today) |
| 0.25 | 1/6 | 1/8 | 0.6 |
| 0.5 | 1/3 | 1/4 | 1/2 |
| 1 | 2/3 | 1/2 | 1/2 (today's rule) |

From the bar, the stake needed is `(2/3 - (2/3)r) / (1 - (2/3)r)`, never below capital's own floor of 1/2. So the extra
protection lasts until `r = 1/2`, and the trigger should only reach 1/2 once capture is genuinely hard.

## The trigger: the effective number of independent lineages

Boardy's proposal (2026-10-21): trigger on lineage diversity, meaning weight spread across roots with no
shared ancestry and no co-signing history, not on a head count or a date. Here's that idea made concrete
for the sketch:

1. **Weigh by realized use, not by finality weight.** In the sketch, finality weight is novelty: new
   content past the vesting window `W` and not refuted (`finality_pom_weight`, `runtime.rs:1146`, built on
   `pom_scores_with_similarity_floor_q16`, `lib.rs:341`). It does not need anyone to have used the work.
   Value does: `flow_gate` is zero until something downstream builds on a cell (`lib.rs:1157`), and
   `value_v8` (`lib.rs:1391`) multiplies novelty by it. So the trigger weighs each vested, unrefuted cell
   by its flow-gated value. Counting raw finality weight would let one actor fake diversity with new
   content alone.
2. **Two effective numbers.** Using those weights, compute the effective number twice. `N_eff = 1 /
   sum(s_i^2)` is the inverse Herfindahl index, the standard way to ask "how many equal players is this
   worth". Two equal shares count as 2, and a thousand dust shares add almost nothing.
   - `N_lin`, over **lineages**: connected components of the provenance graph (`Cell.parent`, the
     forest of `DESIGN-genesis-root.md` section 1). This captures "no shared ancestry".
   - `N_id`, over **contributor identities** (`type_script.args`). This captures "no shared author": one
     actor running K lineages under one identity scores `N_id` near 1.
3. **Take the lower one.** `N_div = min(N_lin, N_id)`. Diversity has to show up both structurally and
   among authors. There is deliberately no yes/no merging of lineages that share an author: one identity
   dropping a cell into many lineages would fuse them all for the cost of a few cells (see attack 2).
   Concentration measured by weight gives a tiny contributor a tiny effect.
4. **Target and rate limit.** `r_target = min(1, N_div / K)`. Each epoch `r` moves toward `r_target` by at
   most `delta`, up or down. An attacker has to fake diversity, or suppress real diversity, for many
   epochs in a row.

Why not a count or a date: a count can be padded with one actor's extra accounts, and a date is known in
advance, so anyone can pre-position cheap standing before it. Here a lineage or an identity only counts
in proportion to realized use of vested, unrefuted work.

The sketch has no co-signing. Bootstrap admission Phase B (`DESIGN-bootstrap-admission.md`, proposed)
would add vouching, and whether a voucher and the newcomers it vouched for should count as one author in
`N_id` is open.

## Attacks and answers

1. **Fake diversity, to raise `r` early.** One actor seeds K separate lineages under K identities. Each one
   needs flow-gated value, meaning something built on it, then vesting past `W`, then holding for as long
   as the rate limit takes. *Honest floor:* that use can come from the attacker's own extra identities
   inside each lineage, so this adds cost (more cells, more identities, more time) but doesn't make it
   impossible. It's the same open measurement problem as the rest of `v(S)` (SECURITY.md section 1). The
   collusion alarm (`attribution_circulation`, `lib.rs:373`) catches two-way citation only, and directed
   cycles slip past it.
2. **Merge griefing, to lower `r` (which helps a capital captor).** Under a yes/no merge rule, one identity
   could drop a cell into each honest lineage, add one throwaway cell built on it so it carries value, and
   fuse every lineage into one cluster, all for two cells per lineage. That's why the trigger uses
   weight-proportional concentration instead. The attacker's identity gets an `N_id` share equal to its
   share of realized value, which is small unless its work is really used. Lineages are never fused by a
   shared author, so `N_lin` doesn't move. *Residual:* an attacker who does earn a large share of
   realized value lowers `N_id`. But then that attacker really does hold a large share of the contribution
   side, and lowering `r` is the right response.
3. **Suppression by disputes.** A captor refutes lineages to lower `r`. Refutations are bonded, only land
   inside `W`, and the rate limit bounds how fast `r` can fall.
4. **Whiplash at a threshold.** `r` is continuous, not a switch, and it is rate-limited. The rate limit
   buys time, not prevention. A patient attacker still gets there, but only after the disputes window
   and anyone watching the published curve have had their chance.
5. **Replica disagreement.** `r` is computed from finalized state only, so every replica gets the same
   number. On-VM it has to be integer or fixed point, like the rest of finality.
6. **A healthy ecosystem with one dominant lineage.** Real ecosystems are heavy-tailed: a few lineages
   usually hold most of the use, the way most software builds on a few foundations. With Zipf
   shares across 500 lineages, `N_eff` is about 28 at exponent 1, about 5 at exponent 1.5, and about
   2.5 at exponent 2, and it barely grows with the number of lineages (computed 2026-10-21). Under the
   `min`, one dominant but honestly used lineage could keep `r` low indefinitely. So `K` has to be
   calibrated against the measured tail of real use, not a count. A `K` of 5 is reachable at exponent 1.5
   and not at 2. Whether `N_lin` belongs in the `min` or in a softer
   combination (for example a geometric mean with `N_id`) is open. `N_id` measures what capture costs;
   `N_lin` measures shared priors, which is a correlated-error risk, not a capture risk.

## What it costs

- Early on, capital can finalize with little contribution consent: none at `r = 0`, the same as genesis
  today. The anti-plutocracy property phases in with `r` instead of switching on at the first vested
  contributor. That's the price of not handing a veto and 2/3 of the vote to whoever vests first.
- A new consensus input, `r`, which every replica has to compute identically, in integers on-VM.
- Two new constitutional parameters, `K` and `delta`.
- A mismatch to be honest about: the trigger counts realized use, but the finality weight itself is still
  novelty (see trigger step 1). Whether finality weight should also be flow-gated is a separate open
  question. The ramp doesn't settle it, but it does keep a novelty farm from turning on contribution's full
  weight early. Once `r` reaches 1, a novelty farm faces the same exposure SECURITY.md section 5 already
  names: `W` delays it, and only an un-gameable `v(S)` bounds it.

## Open: Will's calls

- **K**, the effective number of independent lineages needed for full weight. Proposal: pick a value a
  testnet can actually reach (for example 5), measure, and fix it before mainnet.
- **delta**, the most `r` can move per epoch. Proposal: a full ramp takes no less than about 30 days of
  epochs.
- **How `N_lin` and `N_id` combine.** Proposal: start with the strict `min`, and measure the tail of real
  use on testnet. If one honest lineage dominates (attack 6), move `N_lin` into a softer combination.
  `N_id` alone is what capture costs.
- **Whether capital's floor also ramps.** Proposal: no, it stays at half.
- **Whether `r` may fall.** Proposal: yes, rate-limited both ways, so a contribution side that collapses or
  gets captured doesn't keep weight it no longer earns.

## Build plan (after ratification)

Reference first, off by default (`r` fixed at 1, today's behavior, so nothing changes until a chainspec
turns it on):

- `runtime::finality`: the mix and contribution's floor as functions of `r`. Lineage clusters and `N_eff`
  over the vested map. A ramp state with the rate limit, updated per checkpoint round
  (`Node::checkpoint_round`, `runtime.rs:1101`). Constitution fields for `K` and `delta`.
- The on-VM mirror in `noesis-core`, fixed point, with the same conservative rounding as `42f83715`.
- Tests: both ends equal today's rules (`r = 0` is stake-only, `r = 1` is the current region). A captured
  tiny contribution side needs 2/3 of stake at `r = 0`. Dust cells barely move `N_lin` or `N_id`. One
  identity spreading cells across many lineages doesn't fuse them. Novel but unused content adds nothing
  to the trigger. The rate limit
  holds both ways. Denomination invariance still holds. The fixed-point and f64 rules agree off the
  boundary and never disagree in the unsafe direction.
- `capital_share` gains `r`, so a testnet can publish the curve directly.
