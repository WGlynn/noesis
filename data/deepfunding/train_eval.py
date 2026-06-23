#!/usr/bin/env python3
"""
train_eval.py -- replicate the Rust `outcome::train` (Bradley-Terry gradient ascent)
in numpy, train/test split the DeepFunding jury pairs with NO leakage, and report:

  (a) LEARNED model held-out pairwise accuracy
  (b) PROXY baseline held-out accuracy = best SINGLE fixed feature (the "fixed
      structural proxy" the moat must beat). We also report each feature alone.
  (c) the delta (learned - best proxy)
  (d) coin-flip 50% floor

Matches Rust `train` exactly (node/src/lib.rs ~L6558):
  w += lr * (grad/denom - 1e-3*w),  grad += (1-sigmoid(w.(fi-fj)))*(fi-fj)  over prefs
and `pairwise_accuracy`: score(winner) > score(loser) -> 1, tie -> 0.5, else 0.
Score = w . feats (dot product), same as v_outcome pre-sigmoid (sigmoid is monotone
so it does not change ranking / accuracy).

Deterministic: numpy seeded.
"""
import os
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
LABELS = os.path.join(HERE, "outcome_labels_deepfunding.txt")

N_FEATS = 4
SEED = 1234
ITERS = 5000
LR = 0.5
TEST_FRAC = 0.20


def load_prefs(path):
    """Mirror of the Rust outcome::load_prefs parser."""
    feats = []
    prefs = []
    with open(path, encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("pref "):
                parts = line[5:].split()
                nums = []
                for t in parts:
                    try:
                        nums.append(int(t))
                    except ValueError:
                        pass
                if len(nums) == 2:
                    prefs.append((nums[0], nums[1]))
                continue
            vals = []
            for t in line.split():
                try:
                    vals.append(float(t))
                except ValueError:
                    pass
            if len(vals) == N_FEATS:
                feats.append(vals)
    feats = np.array(feats, dtype=float)
    n = len(feats)
    prefs = [(w, l) for (w, l) in prefs if w < n and l < n]  # bounds guard, as Rust
    return feats, prefs


def sigmoid(x):
    return 1.0 / (1.0 + np.exp(-x))


def train(feats, prefs, iters=ITERS, lr=LR):
    """Exact replica of Rust outcome::train (L2-reg Bradley-Terry grad ascent)."""
    w = np.zeros(N_FEATS)
    denom = max(len(prefs), 1)
    F = feats
    pairs = np.array(prefs)  # (P,2)
    if len(pairs) == 0:
        return w
    D = F[pairs[:, 0]] - F[pairs[:, 1]]  # (P, N_FEATS)  fi - fj
    for _ in range(iters):
        dot = D @ w
        p = sigmoid(dot)
        g = ((1.0 - p)[:, None] * D).sum(axis=0)
        w += lr * (g / denom - 1e-3 * w)
    return w


def pairwise_accuracy(scores, prefs):
    """Rust outcome::pairwise_accuracy: win>lose ->1, tie ->0.5, else 0."""
    if len(prefs) == 0:
        return 0.0
    correct = 0.0
    for (w, l) in prefs:
        sw, sl = scores[w], scores[l]
        if sw > sl:
            correct += 1.0
        elif sw == sl:
            correct += 0.5
    return correct / len(prefs)


def main():
    feats, prefs = load_prefs(LABELS)
    print("=== train_eval.py ===")
    print("repos (feature rows):", len(feats), "| total preferences:", len(prefs))

    rng = np.random.default_rng(SEED)
    prefs = list(prefs)
    idx = rng.permutation(len(prefs))
    n_test = int(round(len(prefs) * TEST_FRAC))
    test_idx = set(idx[:n_test].tolist())
    train_prefs = [prefs[i] for i in range(len(prefs)) if i not in test_idx]
    test_prefs = [prefs[i] for i in range(len(prefs)) if i in test_idx]
    print("train pairs:", len(train_prefs), "| held-out test pairs:", len(test_prefs))

    # (a) LEARNED Bradley-Terry over all 4 features
    w = train(feats, train_prefs)
    learned_scores = feats @ w
    learned_acc = pairwise_accuracy(learned_scores, test_prefs)
    # also report train accuracy to check for over/underfit
    learned_train_acc = pairwise_accuracy(learned_scores, train_prefs)

    # (b) PROXY baselines: each SINGLE fixed feature, ranked directly (no training).
    # The moat's "fixed structural proxy" = best of these. We try +feature and, for
    # honesty, also -feature (a proxy could correlate negatively); we report the best
    # single-feature held-out accuracy as the bar to beat.
    names = ["breadth", "synergy", "connected", "depth"]
    proxy_results = {}
    for k in range(N_FEATS):
        col = feats[:, k]
        acc_pos = pairwise_accuracy(col, test_prefs)
        acc_neg = pairwise_accuracy(-col, test_prefs)
        # report the better orientation but label it
        if acc_pos >= acc_neg:
            proxy_results[names[k]] = (acc_pos, "+")
        else:
            proxy_results[names[k]] = (acc_neg, "-")

    best_proxy_name = max(proxy_results, key=lambda n: proxy_results[n][0])
    best_proxy_acc, best_sign = proxy_results[best_proxy_name]

    print("\n--- held-out pairwise accuracy ---")
    print("coin-flip floor            : 0.5000")
    for nm in names:
        acc, sign = proxy_results[nm]
        print("proxy [%-9s] (%s)    : %.4f" % (nm, sign, acc))
    print("BEST single-feature proxy  : %.4f  (%s, %s)"
          % (best_proxy_acc, best_proxy_name, best_sign))
    print("LEARNED Bradley-Terry (4f) : %.4f  (train acc %.4f)"
          % (learned_acc, learned_train_acc))
    print("learned weights            :", np.round(w, 4).tolist())
    print("\nDELTA (learned - best proxy): %+.4f" % (learned_acc - best_proxy_acc))
    print("DELTA (learned - coinflip)  : %+.4f" % (learned_acc - 0.5))

    verdict = ("LEARNED BEATS PROXY" if learned_acc > best_proxy_acc + 1e-9
               else "LEARNED DOES NOT BEAT PROXY (null/negative result)")
    print("\nVERDICT:", verdict)


if __name__ == "__main__":
    main()
