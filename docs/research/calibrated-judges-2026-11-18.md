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

---

*Everything above this line is the pre-registration as committed in `50ef8b86` (2026-11-18), unchanged.
Everything below was written after the run.*

## Results (run 2026-11-18)

- Sim: `node/examples/calibrated_judges_sim.rs`. Full output: `calibrated-judges-2026-11-18-output.txt`.
- 124 cells x 6 Delta values x 32 seeds = 23,808 runs, 77 s on two threads (release build of the example
  only, in `target/calib-sim`, 47 s). Deterministic: a second run gave identical numbers.
- The dispute variant read `consensus::TWO_THIRDS_BPS = 6667` from the library.

### Verdicts (pre-registered criteria)

| Adversary | Arm B (calibration alone) | Arm C (standing x calibration) |
|---|---|---|
| 1. Plain ring | **CONFIRMED** (WORKS at all four shares) | **CONFIRMED** |
| 2. Selective ring | **BOUNDARY**: WORKS at 0.10 (undecided within 2 SE); PARTIAL at 0.30, 0.49, 0.60 (H1 fails: influence does not halve, it grows) | **BOUNDARY**: WORKS at 0.10, 0.30, 0.49 (0.49 undecided); PARTIAL at 0.60 (H1 fails) |
| 3. Patient ring | **CONFIRMED** (influence halves 5 rounds after defection) | **CONFIRMED** (same) |
| 4. Bought anchor (H4: breaks at about half) | **CONFIRMED**: f_break 0.50 / 0.45 / 0.45 / 0.40 at s = 0.10 / 0.30 / 0.49 / 0.60; f_flip 0.50 at every share | **CONFIRMED**: f_break 0.50 / 0.50 / 0.45 / 0.45; f_flip 0.50 at every share |

### Ring influence over rounds, and what it took

I(t) is the ring's share of judge weight; M is its share of the value minted in the window (all of it wash,
so the honest contributors' share is 1 - M); TV is the mean misallocation. Each arm faces the ring's best
Delta for that arm.

**Adversary 1, plain ring** (window rounds 1-60)

| s | arm | Delta* | I(1) | I(10) | I(30) | I(60) | I_end (SE) | M (SE) | honest share | TV |
|---|---|---|---|---|---|---|---|---|---|---|
| 0.10 | A | 3.00 | 0.100 | 0.066 | 0.042 | 0.028 | 0.030 (0.000) | 0.0163 (0.0000) | 0.9837 | 0.0225 |
|  | B | 3.00 | 0.100 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0037 (0.0000) | 0.9963 | 0.0153 |
|  | C | 3.00 | 0.100 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0035 (0.0000) | 0.9965 | 0.0151 |
| 0.30 | A | 3.00 | 0.300 | 0.335 | 0.377 | 0.412 | 0.408 (0.000) | 0.4315 (0.0005) | 0.5685 | 0.4315 |
|  | B | 3.00 | 0.300 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0296 (0.0001) | 0.9704 | 0.0405 |
|  | C | 3.00 | 0.300 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0301 (0.0001) | 0.9699 | 0.0410 |
| 0.49 | A | 3.00 | 0.490 | 0.630 | 0.753 | 0.824 | 0.817 (0.000) | 0.8815 (0.0002) | 0.1185 | 0.8815 |
|  | B | 3.00 | 0.490 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0642 (0.0002) | 0.9358 | 0.0736 |
|  | C | 3.00 | 0.490 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0658 (0.0002) | 0.9342 | 0.0752 |
| 0.60 | A | 3.00 | 0.600 | 0.744 | 0.849 | 0.901 | 0.896 (0.000) | 0.9529 (0.0001) | 0.0471 | 0.9529 |
|  | B | 3.00 | 0.600 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0848 (0.0002) | 0.9152 | 0.0924 |
|  | C | 3.00 | 0.600 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0859 (0.0002) | 0.9141 | 0.0934 |

**Adversary 2, selective ring** (window rounds 1-60)

| s | arm | Delta* | I(1) | I(10) | I(30) | I(60) | I_end (SE) | M (SE) | honest share | TV |
|---|---|---|---|---|---|---|---|---|---|---|
| 0.10 | A | 3.00 | 0.100 | 0.065 | 0.040 | 0.027 | 0.029 (0.000) | 0.0150 (0.0000) | 0.9850 | 0.0213 |
|  | B | 0.50 | 0.100 | 0.073 | 0.052 | 0.048 | 0.049 (0.001) | 0.0035 (0.0000) | 0.9965 | 0.0142 |
|  | C | 3.00 | 0.100 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0033 (0.0000) | 0.9967 | 0.0149 |
| 0.30 | A | 3.00 | 0.300 | 0.288 | 0.277 | 0.269 | 0.270 (0.000) | 0.2639 (0.0002) | 0.7361 | 0.2639 |
|  | B | 0.25 | 0.300 | 0.338 | 0.396 | 0.390 | 0.394 (0.004) | 0.0385 (0.0002) | 0.9615 | 0.0403 |
|  | C | 3.00 | 0.300 | 0.000 | 0.000 | 0.000 | 0.000 (0.000) | 0.0243 (0.0001) | 0.9757 | 0.0353 |
| 0.49 | A | 3.00 | 0.490 | 0.542 | 0.581 | 0.603 | 0.601 (0.000) | 0.6222 (0.0003) | 0.3778 | 0.6222 |
|  | B | 0.25 | 0.490 | 0.564 | 0.651 | 0.651 | 0.655 (0.004) | 0.1294 (0.0004) | 0.8706 | 0.1294 |
|  | C | 0.25 | 0.490 | 0.367 | 0.307 | 0.227 | 0.239 (0.003) | 0.0690 (0.0004) | 0.9310 | 0.0695 |
| 0.60 | A | 3.00 | 0.600 | 0.667 | 0.713 | 0.736 | 0.734 (0.000) | 0.7593 (0.0002) | 0.2407 | 0.7593 |
|  | B | 0.25 | 0.600 | 0.685 | 0.776 | 0.772 | 0.775 (0.003) | 0.2174 (0.0004) | 0.7826 | 0.2174 |
|  | C | 0.25 | 0.600 | 0.485 | 0.461 | 0.377 | 0.390 (0.005) | 0.1441 (0.0008) | 0.8559 | 0.1441 |

**Adversary 3, patient ring** (honest through round 20; window rounds 21-60)

| s | arm | Delta* | I(20) | I(30) | I(60) | I_end (SE) | M (SE) | honest share | TV |
|---|---|---|---|---|---|---|---|---|---|
| 0.10 | A | 3.00 | 0.100 | 0.084 | 0.058 | 0.061 (0.000) | 0.0261 (0.0001) | 0.9739 | 0.0299 |
|  | B | 3.00 | 0.101 | 0.000 | 0.000 | 0.000 (0.000) | 0.0049 (0.0001) | 0.9951 | 0.0161 |
|  | C | 3.00 | 0.101 | 0.000 | 0.000 | 0.000 (0.000) | 0.0047 (0.0001) | 0.9953 | 0.0160 |
| 0.30 | A | 3.00 | 0.300 | 0.314 | 0.347 | 0.343 (0.000) | 0.3838 (0.0004) | 0.6162 | 0.3838 |
|  | B | 3.00 | 0.299 | 0.000 | 0.000 | 0.000 (0.000) | 0.0412 (0.0002) | 0.9588 | 0.0517 |
|  | C | 3.00 | 0.299 | 0.000 | 0.000 | 0.000 (0.000) | 0.0415 (0.0002) | 0.9585 | 0.0520 |
| 0.49 | A | 3.00 | 0.490 | 0.551 | 0.669 | 0.656 (0.000) | 0.8073 (0.0003) | 0.1927 | 0.8073 |
|  | B | 3.00 | 0.488 | 0.000 | 0.000 | 0.000 (0.000) | 0.0878 (0.0002) | 0.9122 | 0.0968 |
|  | C | 3.00 | 0.488 | 0.000 | 0.000 | 0.000 (0.000) | 0.0887 (0.0002) | 0.9113 | 0.0977 |
| 0.60 | A | 3.00 | 0.600 | 0.666 | 0.778 | 0.766 (0.000) | 0.9150 (0.0001) | 0.0850 | 0.9150 |
|  | B | 3.00 | 0.599 | 0.000 | 0.000 | 0.000 (0.000) | 0.1127 (0.0002) | 0.8873 | 0.1200 |
|  | C | 3.00 | 0.599 | 0.000 | 0.000 | 0.000 (0.000) | 0.1133 (0.0002) | 0.8867 | 0.1206 |

Under A the ring's influence never falls to half of I(20) within the horizon (it grows at s >= 0.30); under B
and C it halves 5 rounds after defection at every share. Five rounds is the reveal lag: wash from round 21 is
graded at the end of round 24 and the weights move in round 25. Most of what the patient ring took under B
and C it took in those rounds. Part of every B and C figure is a floor that needs no ring weight: honest
judges' noise on a worthless item averages to a small positive value once the aggregate is clipped at 0. By
arithmetic (not measured separately) it is about 3% of the mint when 60 of 100 items are wash.

**Adversary 4, bought anchor (plain base).** Up to f = 0.40 calibration weighting holds (B's ring takes
0.4%, 3.1%, 9.1% of the mint at s = 0.10, 0.30, 0.49; at s = 0.60 it already keeps influence 0.56 and takes
33%). At f = 0.45 it starts to give. At **f = 0.50 it flips at every share, for both B and C**: the ring holds
0.90 to 1.00 of the judge weight at the end and takes 56% / 88% / 96% / 98% of the mint under B (73% / 92% /
97% / 99% under C), against 1.6% / 43% / 88% / 95% under standing alone. That is worse than standing alone by
more than 2 SE at every share. Above f = 0.5 the ring holds all the weight.

### Reported, not decisive

- **R4, selective ring with a bought anchor.** The flip is again at f = 0.50 at every share for both arms.
  Arm B is already PARTIAL at f = 0 for s >= 0.30, so its f_break is 0.00 to 0.10; arm C holds to f = 0.45 at
  s <= 0.30.
- **R1, weighted median.** The median alone (arm A) already neutralizes a minority selective ring (it takes
  0.3%, 2.4%, 14% at s = 0.10, 0.30, 0.49), and calibration weighting made it **worse**: B's ring grew to
  influence 0.14 / 0.39 / 0.66 and took 0.6% / 8.8% / 19% (C: 0.3% / 3.8% / 19%), all above A by more than
  2 SE. Plain and patient rings: B and C WORKS at every share (A's ring captures 97% to 100% of the mint at
  s >= 0.49).
- **R1, dispute-style 2/3 to refute (today's verdict shape).** Standing alone stops every ring below a third
  (it takes 0% to 1.8%) and loses to every ring above a third: at s = 0.49 / 0.60 the plain ring takes 53% /
  94%, and the selective ring takes exactly its share, 49% / 60% (its wash survives because it blocks
  refutation: the club resting point). With calibration weighting, all three rings fall to 3% to 7% of the
  mint at s = 0.49 and 0.60, and the selective ring's influence falls to 0.02 / 0.01 (B) and 0.003 / 0.002
  (C). The reason: to block a refutation the ring must report at least 0.5 on worthless items, which is
  outside honest judges' noise (0.3), so calibration sees it. At s = 0.30 arm B did slightly worse than A
  (1.1% against 0.5%, more than 2 SE).
- **R2, calibration settings (selective ring).** Sharper weighting (eta 20) or no forgetting (gamma 1.0)
  makes the selective ring stronger under B: influence 0.67 to 0.99 at s >= 0.30. Gentler weighting (eta 1)
  weakens it, and arm C at eta 1 was WORKS at all four shares (ring influence 0.000 / 0.000 / 0.068 / 0.092,
  mint 0.4% / 2.5% / 6.8% / 13%). eta 1 was not run against the other adversaries.
- **R3, concentrated selective ring** (10 identities holding share s). Both B and C WORKS at every share, but
  standing alone also shrinks this ring: with only 10 items a round it cannot mint enough to keep its
  standing even with 60% of it (A: influence 0.60 to 0.16, mint 8%). B starts at 0.10 because it counts
  identities, not standing.

### Predictions against outcomes

| Prediction (written before data) | Outcome |
|---|---|
| A: ring grows at s = 0.30, 0.49, 0.60; shrinks at 0.10 | Held for the plain ring. Missed for the selective ring at 0.30: it shrinks slowly (0.30 to 0.27), because valuing others honestly leaves honest items their full value. |
| Plain: B and C CONFIRMED | Held. Weight gone by round 5 (the first reveal). |
| Selective: B BOUNDARY, H1 failing at most shares, best Delta below 0.3 | Held. Delta* = 0.25 at s >= 0.30 (0.50 at s = 0.10), and influence grew. |
| Selective: C BOUNDARY | Held (fails only at 0.60). |
| Patient: B and C CONFIRMED | Held; the take is confined to the 5-round lag. |
| Bought anchor: f_break 0.4 to 0.5, f_flip about 0.5 | Held: f_break 0.40 to 0.50, f_flip exactly 0.50 everywhere. |

Nearly every outcome matched a prediction derived from the model's own arithmetic before the run. So the
run mostly checks that arithmetic and measures the dynamics (the standing feedback in C, the lag burst, the
ring's best response). It is not independent evidence about real judges.

### Deviations from the pre-registration

1. After the first run, a print-only change added a flag ('!') to the compact tables marking an arm whose
   ring mint share is above arm A's by more than 2 SE. The rerun's numbers are identical to the first run's
   (checked by diff).
2. Details the pre-registration left open, fixed in the code before any run: the misallocation average is
   over rounds that minted anything (every round under the mean aggregator); the patient ring's "rounds to
   halve" is read from the mean trajectory; INCONCLUSIVE is checked before CONFIRMED; H2 counts as undecided
   unless it can be read as "holds by 2 SE" or "fails by 2 SE" from its parts; for H4, INCONCLUSIVE means
   that at two or more shares the f_break cell or the cell before it is undecided (one share each for B and
   C, so not triggered).

## What it means, in plain words

1. **The circle has the ring resting point, and it is a real attractor.** With judges weighted by standing
   alone and a weighted-average price, a ring holding 30% or more of standing grows: the plain ring goes from
   0.30 to 0.41 of the weight and from 0.60 to 0.90, taking 43% to 95% of what is minted. A smaller ring
   shrinks, because the report cap limits what it can steal.
2. **Calibration weighting removes the crude ring and the patient ring.** The plain ring loses all its
   weight at the first reveal. The patient ring's 20 rounds of good behavior buy it nothing: its influence
   halves 5 rounds after it defects, which is the reveal lag. A good record does not shield a later
   defection when the score forgets.
3. **The selective ring keeps its influence, and grows it.** Calibration rewards precision, and a ring knows
   its own items are worthless. So it inflates them by less than honest judges' noise (0.25 against 0.3) and
   looks better calibrated than the honest judges. At 30% to 60% of the weight its influence rises (0.30 to
   0.39, 0.60 to 0.78). What calibration does do is cap the theft: the ring takes 4% to 22% of the mint,
   against 26% to 76% under standing alone. **Global calibration bounds the ring's theft to the honest
   noise band; it does not remove the ring.** Under a weighted median it is worse than standing alone.
4. **Mixing standing back in helps, up to a point.** Arm C held the selective ring at shares up to 0.49,
   because wash that mints little loses standing. It failed at 0.60.
5. **Calibration is only as good as the anchor.** A ring that controls half the use signal turns calibration
   weighting against the honest judges: at f = 0.5 it holds 90% to 100% of the weight at every share, worse
   than standing alone. Calibration converts "own half the judges" into "own half the use signal". That helps
   only if independent use costs more to buy than standing does, which is the capital-independence oracle's
   job (designed, not built).

## The one design change the results point to

**Have calibration-weighted judges cast coarse verdicts, not fine prices, and let realized use set the
price.** The selective ring's whole edge is a lie smaller than honest judges' noise, which a continuous
valuation cannot see and a squared-error score rewards. A coarse verdict ("is this worth at least half its
claim?", today's dispute shape) forces any lie that changes the outcome to be larger than that noise, and
then calibration catches it: in R1 the selective ring's influence fell to 0.02 or less at 49% and 60% of the
weight, and every ring took 3% to 7% of the mint, against 49% to 94% under standing alone. The design rule:
the verdict's threshold must sit well outside honest judges' noise band. The price itself then comes from
realized use paid later, which is the periphery design's "pay the honest outcome" (ARCHITECTURE.md, value
measure, item 4).

Status: this is a pointer from a reported variant, not a verdict. It needs its own pre-registered run
across all four adversaries, including the bought anchor (not run under the dispute aggregator here), and a
check of the regime where honest noise approaches the threshold. A second candidate aimed at the same root
cause, grading each judge on its signed error per author it judges (a consistent generosity toward
particular authors), was not tested; its known weak point is fresh identities with no history.

## What this run adds to "What this cannot show"

- The selective ring's edge depends on its own reports being noise-free while honest judges' are not. A ring
  that cannot value its own items precisely, or honest judges much more precise than 0.3, would shrink it.
- The flip at f = 0.5 is partly by construction: the bought use mirrors the ring's reports and the loss is
  symmetric squared error. The measurement adds where it breaks first (0.40 to 0.45 at higher shares) and
  how completely (0.90 to 1.00 of the weight).
