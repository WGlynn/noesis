# Calibrated judges: does weighting judges by track record break the circle? (pre-registered simulation)

> Pre-registration written 2026-11-18, before any data, under Will's approval of the run the same day. The
> Pre-registration section is fixed at its commit. Results, deviations and reflection are appended below it
> after the run; nothing above the results line changes after the commit.
>
> Status of what this tests: calibration-weighted judging is **designed, not built** (no Noesis code
> computes a judge's calibration; ARCHITECTURE.md "Success measure" says the calibration measurement is an
> offline script in the ETM repo). This is a measure-only simulation. It does not touch consensus code.

## Pre-registration (written 2026-11-18, before any data)

### Question

Will, 2026-11-18: "What if the circularity of the value works?" In Noesis, judges of value are weighted by
standing, and standing is what the value layer issues. A circle has more than one resting point: an honest
community that values real work, or a ring that values its own members. From inside, both look consistent:
`node/examples/wash_sim.rs` (2026-07-20) found no separation between a genuine collaboration and an acyclic
wash on any of the five worth-blind graph signals it tested (value_v8, novelty, synergy, cycle energy,
circulation).

The proposal: weight each judge by its calibration track record, not by standing alone. A judge's track
record is how well its past valuations predicted later independent use. Influence then flows to whoever
keeps predicting the outside world. The question for this run: **when judges are weighted this way, does a
wash ring's influence shrink round after round, or does it hold?** And which kinds of ring beat it?

### What this models, and what it does not

- Today's verdict rule: a dispute verdict weights judges by vested standing on a PoM-only mix and needs the
  same 2/3 the consensus layer uses (`dispute::POM_ONLY`, `node/src/lib.rs:4768`; `dispute::verdict_refutes`,
  `lib.rs:4778`; `consensus::TWO_THIRDS_BPS = 6667`, `lib.rs:4086`). Edge-connected judges are recused
  (`verdict_refutes_excluding_conflicted`, `lib.rs:5283`); its stated residual is an identity-separated
  cartel with no provenance edge into its targets. The ring in this simulation is that residual: it has no
  edges to recuse on.
- The circle as modeled: value minted to a contribution's author adds to the author's standing, and standing
  weights the judges who value the next round. In the reference node today the finality franchise reads
  structural novelty only and `value_v5..v8` have no runtime caller (ARCHITECTURE.md, value measure, item 2).
  So the simulated loop is the designed value loop, abstracted: no cells, no novelty scores, no `v(S)`. It is
  a model of judges, weights and minting, not of the node.
- "Independent use" is modeled as a noisy, unbiased reading of true worth that arrives after a delay, except
  where an adversary has bought part of it (adversary 4). In Noesis that signal is designed (the periphery,
  `docs/DESIGN-periphery-solution.md`; layered anchors, `docs/research/layered-anchors-2026-10-16.md`), and
  its source, the capital-cluster independence oracle, is not built.
- Payment here is on the judged value at the time of judging, not on realized use. Paying on realized use
  (the periphery design) is a different lever and is not tested here.

### Model (every number fixed now)

- **Population.** N = 100 identities. Each is both a contributor and a judge. Every identity starts with
  standing 1.0.
- **The ring.** n_R = s x N identities, s in {0.10, 0.30, 0.49, 0.60}, so the ring starts with share s of
  standing, of identities and of submitted items. (Decoupling identity count from standing is a reported
  variant, R3 below.)
- **Rounds.** T = 60. Each round every identity submits one item.
- **True worth.** An honest item has worth w ~ Uniform[0.5, 1.5] (mean 1.0). A wash item has worth 0.
- **Reports.** Every judge reports a value for every item. Reports are bounded to [-3, 3] (V_max = 3, three
  times the honest mean; this cap is a design parameter and arm A's results depend on it).
  - Honest judge: r = w + e, e ~ Normal(0, 0.3^2) (sigma_J = 0.3), clipped to [-3, 3]. Reports may be
    negative; the aggregate is clipped at 0 before minting, so honest noise alone does not pay wash.
  - Ring judges: by adversary, below.
- **Aggregate and mint (primary).** For each item, the aggregate value is the weight-averaged report (linear
  pooling), clipped at 0. Each round mints a budget of 10 (10% of the starting total standing), split across
  that round's items in proportion to their aggregate values (no mint if every aggregate is 0). Minted value
  adds to the author's standing.
- **Independent use.** Item c's use signal u_c = w_c + n_c, n_c ~ Normal(0, 0.3^2) (sigma_U = 0.3),
  revealed d = 3 rounds after the item's round (items of round t are revealed at the end of round t + 3).
  Under adversary 4 a fraction f of it is bought (below).
- **Calibration score.** When round t's items are revealed, each judge j's loss for that batch is the mean
  over the batch of (r_jc - u_c)^2. Decayed loss L_j <- gamma x L_j + loss, gamma = 0.9 (half-life about
  6.6 rounds). Calibration factor k_j = exp(-eta x (L_j - min over judges of L)), eta = 5. (Exponential
  weights, the Hedge rule; before the first reveal every k_j = 1.)
- **Order within a round.** Weights are computed from standing and L as they stand at the start of the round;
  reports are made; aggregates are formed; the budget is minted and standing updated; then the batch from
  three rounds back is revealed and L updated.

### Arms (judge weight, normalized to sum to 1 over all judges)

- **A, standing alone (today's circle):** weight_j proportional to standing_j.
- **B, calibration alone:** weight_j proportional to k_j. (Standing is ignored; every identity is a judge.)
- **C, a mix:** weight_j proportional to standing_j x k_j (standing as the prior, calibration as the
  multiplier).

Each arm runs its own dynamics (its own standing trajectory) on the same random draws (common random
numbers: every seed draws the same worths, use noise and judge noise in the same order, for every
configuration).

### Adversaries

The ring's reports are identical across its members. Delta is the value the ring reports on its own items.

1. **Plain ring.** Every ring item is wash (worth 0). The ring reports Delta on its own items and 0 on
   everyone else's.
2. **Selective ring.** Every ring item is wash. The ring reports Delta on its own items and reports
   everyone else's items like an honest judge (w + its own noise). It builds a calibration record on
   everyone else and cheats only on its own members: the attack aimed at a global calibration score.
3. **Patient ring.** Rounds 1 to 20: the ring behaves exactly like honest identities (genuine items with
   honest worth, honest reports). From round 21: plain ring (wash items, Delta on own, 0 on others).
4. **Bought anchor.** The plain ring that also controls a fraction f of the independent-use signal and makes
   that fraction mirror its own reports: u_c = (1 - f)(w_c + n_c) + f x r_ring,c, where r_ring,c is the
   ring's report on c (Delta on ring items, 0 on others). f in {0, 0.1, 0.2, 0.3, 0.4, 0.45, 0.5, 0.55, 0.6,
   0.8}.

**The ring best-responds.** For every cell (adversary, s, f where relevant) and every arm separately, the
ring picks Delta from {0.1, 0.25, 0.5, 1.0, 2.0, 3.0} to maximize its mean (over seeds) share of minted value
in the scoring window. The chosen Delta is reported. (Choosing the max of six noisy means favors the ring a
little; that bias works against calibration, the conservative direction for the claim being tested.)

### Seeds

32 seeds per cell, seed indices 0 to 31. RNG: splitmix64 seeded from the seed index only (so the stream is
shared across every configuration). Draw order per round per item: worth uniform, use noise, then one judge
noise per judge (all drawn whether used or not, to keep streams aligned). Standard errors are across seeds.

### Metrics

- **Ring influence I(t):** the ring's share of judge weight in round t. Reported at rounds 1, 5, 10, 20, 30,
  40, 50, 60 for arms A, B and C. I_end = mean of I(t) over rounds 51 to 60.
- **Ring mint share M:** the ring's share of value minted in the scoring window. The honest contributors'
  share is 1 - M (every ring item in the window is wash).
- **Misallocation:** per round, the total-variation distance between the minted shares and the true-worth
  shares of that round's items (half the sum of absolute differences), averaged over the window.
- **Scoring window:** rounds 1 to 60 for adversaries 1, 2 and 4; rounds 21 to 60 (after defection) for
  adversary 3, which also reports I(20) and the number of rounds after defection until I(t) first falls to
  half of I(20).

### Verdict criteria

For each adversary, each share s, and each arm X in {B, C}, with A as the baseline, each arm at its own
best-response Delta:

- **H1 (influence decays):** I_end(X) <= 0.5 x s.
- **H2 (less value to the ring):** M(X) <= 0.5 x M(A) and M(A) - M(X) > 2 SE (paired across seeds); or
  M(X) < 0.005 (the ring gets under half a percent of the mint).
- **Cell outcome:** WORKS if H1 and H2 hold; FAILS if neither holds; PARTIAL if one holds. A cell is
  UNDECIDED if moving any criterion's mean by 2 SE flips that criterion.

Per adversary, per arm:
- **CONFIRMED:** WORKS at all four shares.
- **REFUTED:** FAILS at all four shares.
- **INCONCLUSIVE:** two or more of the four cells UNDECIDED.
- **BOUNDARY:** anything else, naming the shares and the failing criterion.

For adversary 4, per share and arm: **f_break** = the smallest f in the grid at which the cell is not WORKS;
**f_flip** = the smallest f at which M(X) > M(A) + 2 SE (calibration weighting now does worse than standing
alone). Hypothesis H4: the bought anchor breaks calibration weighting at about half the use signal.
CONFIRMED if 0.4 <= f_break <= 0.55 at every share for that arm; BOUNDARY if f_break exists at every share
but falls outside that band at some share (named); REFUTED if some share has no f_break up to 0.8;
INCONCLUSIVE under the same 2-SE rule.

### Reported, not part of the verdict

- **R1, other aggregators** (adversaries 1 to 3, all shares, the Delta grid, 32 seeds):
  - weighted median of reports, clipped at 0, mint in proportion;
  - dispute-style 2/3 to refute: every item claims a worth-blind value of 1.0 (the structural score cannot
    tell wash from genuine, per `wash_sim`); a judge votes to refute iff its report is below 0.5; an item is
    refuted iff the refuting weight is at least TWO_THIRDS_BPS / 10000, the constant read from the node
    library; surviving items split the budget equally.
- **R2, calibration settings, selective ring:** eta in {1, 20} at gamma 0.9; and gamma = 1.0 (no forgetting)
  at eta 5.
- **R3, concentrated selective ring:** n_R = 10 identities holding standing share s (each ring identity
  starts with s x 100 / 10, each honest one with (1 - s) x 100 / 90). Arms A, B, C. Arm B starts at 10%
  because it ignores standing; that is B's Sybil exposure, out of scope here and named.
- **R4, selective ring with a bought anchor:** adversary 4's f grid on the selective base (the ring's bought
  use mirrors Delta on its own items and is honest elsewhere).

### Predictions (written before data)

- **Arm A:** under the weighted mean with the report cap, a best-responding ring's influence grows over
  rounds at s = 0.30, 0.49 and 0.60 (the circle settles on the ring's resting point) and shrinks at s = 0.10
  (the cap bounds what 10% of the weight can steal). About 80%.
- **Plain ring:** B CONFIRMED (about 85%), C CONFIRMED (about 80%). Its zero reports on honest items make its
  calibration loss large from the first reveal, so its weight collapses within a few rounds of round 4;
  its gain is confined to the first d rounds.
- **Selective ring:** B BOUNDARY (about 70%): H2 holds but H1 fails at most shares. Reason, derived before
  data: the ring knows its own items are worthless, so a report of Delta on them costs it Delta^2 in squared
  error against an honest judge's sigma_J^2 = 0.09. A Delta below 0.3 makes the ring *better* calibrated than
  honest judges on its own items, so the best response is a small Delta that keeps (or grows) its influence
  while extracting only a fraction of an honest item's worth per wash item. C BOUNDARY (about 55%); standing
  feedback may still shrink the ring at high shares because a small Delta mints little.
- **Patient ring:** B CONFIRMED (about 75%) and C CONFIRMED (about 70%): the good record does not shield the
  defection under decayed exponential weights; extraction is confined to the reveal lag after round 20.
- **Bought anchor (plain base):** f_break between 0.4 and 0.5 for B and C at every share (about 70%), slightly
  below one half because the ring's reports are noise-free and honest judges' are not. Above f_break, B and C
  hand the ring the weight, worse than standing alone at s = 0.10 (f_flip about 0.5).
- **Most likely overall reading (written now):** calibration weighting removes the crude ring and the patient
  ring, does not remove the selective ring's influence (it bounds its theft to roughly the honest noise band),
  and fails once about half of the outside signal is bought.

### What this cannot show

- It is a model of judges and weights, not the node. No cells, novelty, `v(S)` or vesting.
- Honest judges are homogeneous (one noise level) and truthful; real judges differ in skill and effort.
- The ring's strategy space is a fixed family with one tuned number (Delta). A smarter ring (varying Delta by
  item, cheating on a subset of its members, rotating identities) is not searched.
- Identity count equals standing share in the primary sweep. Sybil splitting is out of scope (B weights
  identities, so it is Sybil-exposed by construction).
- The use signal is unbiased unless bought. Self-fulfilling scores (high value brings exposure brings use),
  slow use and fake-but-unbought independence are not modeled.
- No slashing, deposits or carrying cost (the pruner layer of the periphery design) are modeled.

### Run plan and machine rules

- New example `node/examples/calibrated_judges_sim.rs`; pure Rust, std only, deterministic; it reads one
  constant from the node library (`consensus::TWO_THIRDS_BPS`) and calls nothing else in it.
- Built as an example only, in release, in a separate target directory (`CARGO_TARGET_DIR=target/calib-sim`),
  so the live node's binary and `target/release` are never touched. One cargo process at a time, at most two
  worker threads in the run.
- The full printed output is saved next to this doc as `calibrated-judges-2026-11-18-output.txt`.
- Any change to the above after this commit is listed in the results as a deviation, with its reason.
