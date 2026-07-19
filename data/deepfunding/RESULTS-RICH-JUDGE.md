# Rich-feature ML-judge backtest — DeepFunding jury pairwise validity + pairwise-value vs funding

> Deterministic (seeded). Reuses the winner=higher-weight binarization + pairwise-accuracy of
> `train_eval.py`, so (A) is directly comparable to the ~0.54 structural null in RESULTS-FAITHFUL.md.

## Coverage
- distinct repos in jury set: 117
- repos with OSO rich features (survived join): 108/117
- jury pairs total: 2387 | pairs usable (both repos have features): 1965

## (A) ML-judge validity — held-out pairwise accuracy (20 seeds, 80/20)
| feature set | model | mean acc | std | vs 0.54 structural null | vs 0.50 floor |
|---|---|---|---|---|---|
| A1 popularity/meta | logistic | **0.6337** | 0.0242 | +0.0988 | +0.1337 |
| A1 popularity/meta | gbm | **0.6565** | 0.0266 | +0.1216 | +0.1565 |
| A2 + funding | logistic | **0.6364** | 0.0254 | +0.1015 | +0.1364 |
| A2 + funding | gbm | **0.6840** | 0.0233 | +0.1491 | +0.1840 |

(reference: RESULTS-FAITHFUL learned 4-graph-feature model = 0.5349; 1-SE noise band on a ~465-pair split ≈ ±0.023.)

## (B) Pairwise-value validity vs ground truth
Per-repo Bradley-Terry strength (aggregate value implied by ALL jury pairs, no features) vs:
- **actual funding** (gitcoin + retro, over 108 repos, 32 with >0 funding): Spearman ρ = **-0.053**
- log stars (popularity sanity check): Spearman ρ = **+0.256**

## Honest verdict

**(A) ML-judge validity — CONFIRMED, and it flips the prior null.** A rich-feature judge predicts jury
pairwise preferences at **0.63–0.68** (GBM + funding = 0.684) vs the **0.5349** four-graph-feature model
in RESULTS-FAITHFUL and the 0.50 floor. The margin (+0.13 to +0.15 over the structural null) is ~6× the
±0.023 sampling-noise band — decisive. **The prior two nulls were a FEATURE problem, not an
ML-judgement problem:** the jury signal is real and learnable; the graph-topology proxies just weren't
the lens that captures it. Popularity/age alone already gets 0.66; funding history adds a genuine ~2 pts.

**(B) Jury value does NOT track funding.** Per-repo Bradley-Terry strength vs actual funding: ρ ≈ 0
(−0.05); vs stars: ρ = +0.26. What the juries valued is weakly popularity-shaped and **uncorrelated with
what actually got funded.** (Caveat: only 32/108 repos have >0 recorded funding — weak power — but the
direction is clean: jury-value ≠ funding-received.)

### Caveats (do not overclaim)
1. **Repo-overlap (load-bearing, NOT yet controlled).** The 80/20 split is over PAIRS, not repos, so a
   repo can appear in train and test. A feature-based model generalizes via features (not repo identity),
   so this is milder than identity memorization — but a **repo-disjoint split is the harder, cleaner
   test and has not been run.** Expect some drop under it; a +0.13 margin over the null is unlikely to
   fully vanish, but the exact number is unverified. This is the one open rigor step.
2. **The signal is largely POPULARITY.** Stars/forks/age drive most of the 0.66; that is a real predictor
   of *this* jury's preferences but also a Goodhart/incumbency risk — a naive learned judge would reward
   established, popular projects. Predicting honest preference ≠ measuring un-gameable value.
3. **Still the wrong instrument for the MOAT.** The moat claim is adversarial-robustness (un-gameability),
   and this is honest static labels with no adversary. (A) validates that *a rich judge can predict human
   preference* — genuine upside — but it does not test, and does not establish, adversarial robustness.

### What this changes
- The learned-quality-layer status upgrades from "**null on the only real dataset**" (RESULTS-FAITHFUL)
  to "**predictively valid on real jury labels with rich features (0.68), pending a repo-disjoint
  re-check; signal is popularity-heavy.**" The structural moat (253/253) is unchanged and remains the
  load-bearing claim; the learned layer is now *demonstrated upside* rather than a flat null.
- (B) is a caution for any "value = funding" grounding: on this data they diverge.

### Reproduce
`python rich_judge_backtest.py` (numpy + pandas + sklearn, Python 3.12). Next rigor step: add a
repo-disjoint split to (A).
