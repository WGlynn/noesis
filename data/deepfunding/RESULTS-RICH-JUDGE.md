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

## (A-disjoint) REPO-DISJOINT held-out accuracy — the honest hard test (full feature set, 20 seeds)
No repo appears in both train and test, so the model cannot lean on a repo it has already seen.
Avg test pairs per split: 76 (small ⇒ noisier than the pair-split above).
| model | mean acc | std | vs pair-split | vs 0.50 floor |
|---|---|---|---|---|
| logistic | **0.5803** | 0.0624 | -0.0561 | +0.0803 |
| gbm | **0.6000** | 0.0654 | -0.0840 | +0.1000 |

## (B) Pairwise-value validity vs ground truth
Per-repo Bradley-Terry strength (aggregate value implied by ALL jury pairs, no features) vs:
- **actual funding** (gitcoin + retro, over 108 repos, 32 with >0 funding): Spearman ρ = **-0.053**
- log stars (popularity sanity check): Spearman ρ = **+0.256**

## Honest verdict (repo-disjoint corrected — the headline number is 0.60, not 0.68)

**The 0.68 pair-split figure was partly repo-overlap inflation.** Under the honest **repo-disjoint** test
(no repo shared between train and test), the rich-feature judge generalizes to *unseen* repos at
**0.58 (logistic) / 0.60 (GBM)** — down ~0.06–0.08 from the pair-split. The true generalizing signal is:

- **Real, but modest.** 0.60 clears the 0.50 floor (+0.10) and edges the 0.54 structural null (~+0.05–0.06),
  but the margin over the null is now small and per-split noise is high (std ≈ 0.06, ~76 test pairs/split).
- **Popularity-driven, honest-label only.** The judge mostly learns "more-popular/established repos win,"
  and predicts *honest* preferences — it does NOT test the *adversarial* un-gameability the moat claims.

**Net.** A rich-feature judge does beat the graph-topology null and the coin-flip floor on real jury
labels, and it generalizes to unseen repos — but the effect is **modest (~0.60), popularity-shaped, and
not the moat.** This VINDICATES the prior framing (the predictive win is *upside*, small, not load-bearing);
it does not resurrect a strong learned-value-oracle claim. The structural defense (253/253) remains the
moat, unchanged. **(B)** jury value ≠ funding (ρ ≈ 0) and only weakly ~stars (ρ = +0.26).

Reproduce: `python rich_judge_backtest.py` (numpy + pandas + sklearn, Python 3.12).
