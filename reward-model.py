#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Learned reward model for block value — Bradley-Terry over FEATURES (AFK item 2).

PRIVATE. The production replacement for the [proxy] coverage v(S): a model that
LEARNS what makes a block valuable from pairwise preferences and GENERALIZES to
unseen blocks via features. This is an RLHF reward model:
  P(i preferred over j) = sigmoid(w . (f_i - f_j)); fit w by gradient descent on
the Bradley-Terry / logistic loss. value(block) = w . f(block). Stdlib only.

  train [N] [iters]    fit on first N-1 blocks, predict the held-out block
"""
import glob
import json
import math
import os
import sys

try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

HOME = os.path.expanduser("~")
BLOCKS = os.path.join(HOME, ".claude", "session-chain", "blocks")
FEATS = ["log_outlen", "checkpoints", "log_coverage", "co_authored", "log_inlen"]


def _block(bid):
    return json.load(open(os.path.join(BLOCKS, bid + ".json"), encoding="utf-8"))


def features(bid):
    b = _block(bid)
    resp, prompt = str(b.get("response", "")), str(b.get("prompt", ""))
    cov = len({resp.lower()[i:i + 5] for i in range(0, max(0, len(resp) - 4))})
    co = 1.0 if "co-authored" in (resp + prompt).lower() else 0.0
    return [math.log1p(len(resp)), float(len(b.get("checkpoints", []) or [])),
            math.log1p(cov), co, math.log1p(len(prompt))]


def normalize(F):
    n = len(FEATS)
    mn = [min(f[k] for f in F.values()) for k in range(n)]
    mx = [max(f[k] for f in F.values()) for k in range(n)]
    return {b: [(f[k] - mn[k]) / (mx[k] - mn[k]) if mx[k] > mn[k] else 0.0
                for k in range(n)] for b, f in F.items()}


def sigmoid(x):
    return 1.0 / (1.0 + math.exp(-max(-30, min(30, x))))


def train_bt(F, prefs, iters=4000, lr=0.4, l2=1e-3):
    n = len(FEATS)
    w = [0.0] * n
    for _ in range(iters):
        grad = [0.0] * n
        for (i, j) in prefs:
            d = [F[i][k] - F[j][k] for k in range(n)]
            p = sigmoid(sum(w[k] * d[k] for k in range(n)))
            for k in range(n):
                grad[k] += (1.0 - p) * d[k]
        for k in range(n):
            w[k] += lr * (grad[k] / max(1, len(prefs)) - l2 * w[k])
    return w


def main():
    a = sys.argv[1:]
    N = int(a[1]) if len(a) > 1 else 10
    iters = int(a[2]) if len(a) > 2 else 4000
    blocks = [os.path.basename(f).replace(".json", "")
              for f in sorted(glob.glob(os.path.join(BLOCKS, "block-*.json")))[:N]]
    F = normalize({b: features(b) for b in blocks})
    train_b, held = blocks[:-1], blocks[-1]
    cov = {b: features(b)[2] for b in blocks}            # ground-truth proxy = coverage
    prefs = [(i, j) for i in train_b for j in train_b if i != j and cov[i] > cov[j]]
    w = train_bt({b: F[b] for b in train_b}, prefs, iters=iters)
    value = lambda b: sum(w[k] * F[b][k] for k in range(len(FEATS)))

    print(f"learned reward model — {len(train_b)} train blocks, {len(prefs)} pairwise prefs")
    print("learned weights (what drives value):")
    for k, name in enumerate(FEATS):
        print(f"  {name:<14}{w[k]:>7.3f}  {'#' * int(abs(w[k]) * 8)}")
    print(f"\nHELD-OUT {held} (unseen) predicted value: {value(held):.3f}  "
          f"(its coverage-feat {F[held][2]:.2f})")
    ranked = sorted(blocks, key=lambda b: -value(b))
    print("predicted ranking (generalizes from features):",
          " > ".join(b.replace("block-", "") for b in ranked[:6]))
    print("note: replaces the coverage [proxy] in production; ground-truth here is a "
          "coverage proxy, real labels = jury/model. This IS the v(S) outcome-evaluator.")


if __name__ == "__main__":
    main()
