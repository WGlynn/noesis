#!/usr/bin/env python3
"""
rich_judge_backtest.py -- does a PROPER ML judge (rich repo features, not 4 graph-structural
proxies) validly predict DeepFunding jury pairwise preferences, and does the pairwise-derived
value track REAL funding?

Motivation. Rounds 1-2 (RESULTS.md / RESULTS-FAITHFUL.md) tested a LEARNED v(S) over 4 graph
features and got a NULL (~0.54 held-out pairwise accuracy vs 0.50 floor), with the honest note
that "the dominant signal the juries used is not structural-topological." The dataset ships rich
per-repo features (stars, forks, age, Gitcoin + retro funding) that NEITHER prior backtest used.
This script asks two questions the prior work explicitly left open:

  (A) ML-JUDGE VALIDITY: can a rich-feature model predict jury pairwise prefs above the 0.54
      structural null / 0.50 floor? Same 20-seed / 80-20 held-out-pairwise-accuracy protocol as
      train_eval.py, so numbers are directly comparable. Two feature sets:
        A1 = popularity/metadata only (stars, forks, is_fork, active-days, has-license)
        A2 = A1 + funding history (gitcoin grants/donors/rounds, retro funding/rounds)
      Two models: LogisticRegression (linear, the rich-feature analog of Bradley-Terry) and
      GradientBoosting (nonlinear, captures interactions).

  (B) PAIRWISE-VALUE VALIDITY vs GROUND TRUTH: fit per-repo Bradley-Terry strengths from ALL jury
      pairs (the aggregate value the pairwise data implies, no features), then Spearman-correlate
      that value with ACTUAL funding received (gitcoin + retro). Does the jury's pairwise value
      track what the ecosystem actually funded?

Honest-number-over-marketing-number. Deterministic (seeded). Reuses the winner=higher-weight
binarization + pairwise-accuracy notion of train_eval.py so A is comparable to the prior null.
"""
import csv
import math
import os

import numpy as np
import pandas as pd
from sklearn.ensemble import GradientBoostingClassifier
from sklearn.linear_model import LogisticRegression

HERE = os.path.dirname(os.path.abspath(__file__))
DATASET = os.path.join(HERE, "mini-contest", "dataset.csv")
OSO = os.path.join(HERE, "dependency-graph", "datasets", "oso", "repo_and_funding_stats.csv")
OUT_DOC = os.path.join(HERE, "RESULTS-RICH-JUDGE.md")

N_SEEDS = 20
TEST_FRAC = 0.20
REF_DATE = pd.Timestamp("2025-01-01", tz="UTC")  # fixed ref ⇒ deterministic ages


def norm(u: str) -> str:
    return str(u).strip().lower().rstrip("/")


def load_features():
    """Per-repo rich features from the OSO stats CSV. Returns (meta_feats, full_feats, funding)
    dicts keyed by normalized url, plus the ordered feature-name lists."""
    df = pd.read_csv(OSO)
    df["key"] = df["url"].map(norm)
    df = df.drop_duplicates("key", keep="first")

    created = pd.to_datetime(df["created_at"], errors="coerce", utc=True)
    updated = pd.to_datetime(df["updated_at"], errors="coerce", utc=True)
    active_days = (updated - created).dt.days.fillna(0).clip(lower=0)

    def num(col):
        return pd.to_numeric(df[col], errors="coerce").fillna(0.0)

    meta = pd.DataFrame({
        "log_stars": np.log1p(num("star_count")),
        "log_forks": np.log1p(num("fork_count")),
        "is_fork": (df["is_fork"].astype(str).str.lower() == "true").astype(float),
        "log_active_days": np.log1p(active_days),
        "has_license": df["license_name"].notna().astype(float),
    })
    funding = pd.DataFrame({
        "log_gitcoin_usd": np.log1p(num("gitcoin_grants_usd")),
        "log_gitcoin_donors": np.log1p(num("gitcoin_unique_donors")),
        "gitcoin_rounds": num("gitcoin_num_rounds"),
        "log_retro_usd": np.log1p(num("retro_funding_usd")),
        "retro_rounds": num("num_retro_funding_rounds"),
    })
    meta_names = list(meta.columns)
    full_names = meta_names + list(funding.columns)

    meta_feats, full_feats, raw_funding = {}, {}, {}
    for i, k in enumerate(df["key"].tolist()):
        m = meta.iloc[i].to_numpy(dtype=float)
        fu = funding.iloc[i].to_numpy(dtype=float)
        meta_feats[k] = m
        full_feats[k] = np.concatenate([m, fu])
        raw_funding[k] = float(num("gitcoin_grants_usd").iloc[i] + num("retro_funding_usd").iloc[i])
    return meta_feats, full_feats, raw_funding, meta_names, full_names


def load_pairs():
    """Jury pairs: (winner_url, loser_url), winner = higher weight (train_eval binarization)."""
    pairs = []
    repos = set()
    with open(DATASET, encoding="utf-8") as f:
        for r in csv.DictReader(f):
            a, b = norm(r["project_a"]), norm(r["project_b"])
            wa, wb = float(r["weight_a"]), float(r["weight_b"])
            winner, loser = (a, b) if wa >= wb else (b, a)
            pairs.append((winner, loser))
            repos.add(a); repos.add(b)
    return pairs, repos


def pairwise_dataset(pairs, feats):
    """Antisymmetric pairwise-diff design: each pref → (fw-fl, 1) and (fl-fw, 0). Only pairs where
    BOTH repos have features. Returns X, y, and the surviving pair list (for split bookkeeping)."""
    kept = [(w, l) for (w, l) in pairs if w in feats and l in feats]
    X, y = [], []
    for (w, l) in kept:
        d = feats[w] - feats[l]
        X.append(d); y.append(1)
        X.append(-d); y.append(0)
    return np.array(X, dtype=float), np.array(y, dtype=int), kept


def held_out_accuracy(model, feats, train_pairs, test_pairs):
    """Fit on train pairs (antisym-augmented), score held-out pairs by P(winner>loser)>0.5."""
    Xtr, ytr = [], []
    for (w, l) in train_pairs:
        d = feats[w] - feats[l]
        Xtr.append(d); ytr.append(1)
        Xtr.append(-d); ytr.append(0)
    Xtr = np.array(Xtr, float); ytr = np.array(ytr, int)
    model.fit(Xtr, ytr)
    correct = 0.0
    for (w, l) in test_pairs:
        d = (feats[w] - feats[l]).reshape(1, -1)
        p = model.predict_proba(d)[0, 1]
        correct += 1.0 if p > 0.5 else (0.5 if p == 0.5 else 0.0)
    return correct / max(len(test_pairs), 1)


def run_ml_judge(kept_pairs, feats, label):
    """20-seed 80/20 held-out pairwise accuracy for logistic + GBM. Mirrors train_eval protocol."""
    logi = {"seeds": []}
    gbm = {"seeds": []}
    for seed in range(N_SEEDS):
        rng = np.random.default_rng(1234 + seed)
        idx = rng.permutation(len(kept_pairs))
        n_test = int(round(len(kept_pairs) * TEST_FRAC))
        test = [kept_pairs[i] for i in idx[:n_test]]
        train = [kept_pairs[i] for i in idx[n_test:]]
        logi["seeds"].append(held_out_accuracy(
            LogisticRegression(fit_intercept=False, C=1.0, max_iter=2000), feats, train, test))
        gbm["seeds"].append(held_out_accuracy(
            GradientBoostingClassifier(n_estimators=120, max_depth=3, learning_rate=0.1,
                                       random_state=seed), feats, train, test))
    out = {}
    for name, d in (("logistic", logi), ("gbm", gbm)):
        a = np.array(d["seeds"])
        out[name] = (a.mean(), a.std())
    return out


def bradley_terry_strengths(pairs, repos, iters=8000, lr=0.5, reg=1e-3):
    """Per-repo BT latent strength MLE from ALL pairs (no features): maximize Σ log σ(s_w - s_l)."""
    rlist = sorted(repos)
    ridx = {r: i for i, r in enumerate(rlist)}
    W = np.array([ridx[w] for (w, l) in pairs])
    L = np.array([ridx[l] for (w, l) in pairs])
    s = np.zeros(len(rlist))
    denom = max(len(pairs), 1)
    for _ in range(iters):
        d = s[W] - s[L]
        g = 1.0 / (1.0 + np.exp(d))  # (1 - sigmoid(d))
        grad = np.zeros_like(s)
        np.add.at(grad, W, g)
        np.add.at(grad, L, -g)
        s += lr * (grad / denom - reg * s)
    return {r: s[ridx[r]] for r in rlist}


def spearman(x, y):
    x = np.asarray(x, float); y = np.asarray(y, float)
    if len(x) < 3:
        return float("nan")
    rx = pd.Series(x).rank().to_numpy()
    ry = pd.Series(y).rank().to_numpy()
    return float(np.corrcoef(rx, ry)[0, 1])


def main():
    meta_feats, full_feats, funding, meta_names, full_names = load_features()
    pairs, repos = load_pairs()

    covered = sum(1 for r in repos if r in full_feats)
    _, _, kept_meta = pairwise_dataset(pairs, meta_feats)
    _, _, kept_full = pairwise_dataset(pairs, full_feats)

    lines = []
    p = lines.append
    p("# Rich-feature ML-judge backtest — DeepFunding jury pairwise validity + pairwise-value vs funding\n")
    p("> Deterministic (seeded). Reuses the winner=higher-weight binarization + pairwise-accuracy of")
    p("> `train_eval.py`, so (A) is directly comparable to the ~0.54 structural null in RESULTS-FAITHFUL.md.\n")
    p("## Coverage")
    p(f"- distinct repos in jury set: {len(repos)}")
    p(f"- repos with OSO rich features (survived join): {covered}/{len(repos)}")
    p(f"- jury pairs total: {len(pairs)} | pairs usable (both repos have features): {len(kept_full)}\n")

    print(f"coverage: {covered}/{len(repos)} repos, {len(kept_full)}/{len(pairs)} pairs usable")

    # (A) ML-judge validity
    p("## (A) ML-judge validity — held-out pairwise accuracy (20 seeds, 80/20)")
    p("| feature set | model | mean acc | std | vs 0.54 structural null | vs 0.50 floor |")
    p("|---|---|---|---|---|---|")
    baseline_null = 0.5349  # RESULTS-FAITHFUL best learned (descendant), for reference
    for label, feats, kept in (("A1 popularity/meta", meta_feats, kept_meta),
                               ("A2 + funding", full_feats, kept_full)):
        res = run_ml_judge(kept, feats, label)
        for model in ("logistic", "gbm"):
            m, sd = res[model]
            print(f"[A] {label:22s} {model:9s}: {m:.4f} (std {sd:.4f})")
            p(f"| {label} | {model} | **{m:.4f}** | {sd:.4f} | {m-baseline_null:+.4f} | {m-0.5:+.4f} |")
    p("")
    p(f"(reference: RESULTS-FAITHFUL learned 4-graph-feature model = {baseline_null:.4f}; 1-SE noise band on a "
      "~465-pair split ≈ ±0.023.)\n")

    # (B) pairwise-value validity vs funding
    strengths = bradley_terry_strengths(pairs, repos)
    common = [r for r in repos if r in funding]
    s_vals = [strengths[r] for r in common]
    fund_vals = [funding[r] for r in common]
    star_vals = [meta_feats[r][0] if r in meta_feats else 0.0 for r in common]  # log_stars
    n_funded = sum(1 for r in common if funding[r] > 0)
    rho_fund = spearman(s_vals, fund_vals)
    rho_star = spearman(s_vals, star_vals)
    p("## (B) Pairwise-value validity vs ground truth")
    p("Per-repo Bradley-Terry strength (aggregate value implied by ALL jury pairs, no features) vs:")
    p(f"- **actual funding** (gitcoin + retro, over {len(common)} repos, {n_funded} with >0 funding): "
      f"Spearman ρ = **{rho_fund:+.3f}**")
    p(f"- log stars (popularity sanity check): Spearman ρ = **{rho_star:+.3f}**\n")
    print(f"[B] BT-strength vs funding: rho={rho_fund:+.3f} ({n_funded} funded) | vs stars: rho={rho_star:+.3f}")

    with open(OUT_DOC, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print("wrote", OUT_DOC)


if __name__ == "__main__":
    main()
