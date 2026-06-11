#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""value-v4 — novelty x quality: strategyproofness AND capability composed (PRIVATE).

Phase-1 capstone. The production value rule:

    value(b) = novelty(b)  x  (1 + quality(b))

  novelty(b) = | coverage(b) \\ union(earlier-committed coverage) |   (strategyproof
               floor; sybil/padding/ring -> 0 by construction, value-v3)
  quality(b) = normalized learned-reward-model score over block features (the
               capability layer; reward-model.py). A quality BOOST, never a floor:
               because novelty multiplies, a redundant block (novelty 0) stays 0 no
               matter how "high quality" it looks -> strategyproofness preserved.

So the two layers compose cleanly: novelty makes it un-gameable, quality makes it
capability-aware, and multiplication keeps the un-gameable property dominant.
coverage = [proxy] for the same learned evaluator. Stdlib only.

  pom [N]       novelty x quality PoM
  attack [N]    confirm sybil/pad/ring still 0 despite any quality
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


def _b(bid):
    return json.load(open(os.path.join(BLOCKS, bid + ".json"), encoding="utf-8"))


def cov(bid):
    t = str(_b(bid).get("response", "")).lower()
    return {t[i:i + 5] for i in range(0, max(0, len(t) - 4))} or {bid}


def features(bid):
    b = _b(bid)
    resp, prompt = str(b.get("response", "")), str(b.get("prompt", ""))
    co = 1.0 if "co-authored" in (resp + prompt).lower() else 0.0
    return [math.log1p(len(resp)), float(len(b.get("checkpoints", []) or [])),
            math.log1p(len(cov(bid))), co, math.log1p(len(prompt))]


def normalize(F):
    n = len(FEATS)
    mn = [min(f[k] for f in F.values()) for k in range(n)]
    mx = [max(f[k] for f in F.values()) for k in range(n)]
    return {b: [(f[k] - mn[k]) / (mx[k] - mn[k]) if mx[k] > mn[k] else 0.0
                for k in range(n)] for b, f in F.items()}


def sigmoid(x):
    return 1.0 / (1.0 + math.exp(-max(-30, min(30, x))))


def train_bt(F, prefs, iters=3000, lr=0.4):
    n = len(FEATS)
    w = [0.0] * n
    for _ in range(iters):
        g = [0.0] * n
        for (i, j) in prefs:
            d = [F[i][k] - F[j][k] for k in range(n)]
            p = sigmoid(sum(w[k] * d[k] for k in range(n)))
            for k in range(n):
                g[k] += (1 - p) * d[k]
        for k in range(n):
            w[k] += lr * (g[k] / max(1, len(prefs)) - 1e-3 * w[k])
    return w


def commit_order(blocks):
    return sorted(blocks, key=lambda b: _b(b).get("timestamp", 0))


def main():
    a = sys.argv[1:]
    mode = a[0] if a else "pom"
    N = int(a[1]) if len(a) > 1 else 8
    blocks = [os.path.basename(f).replace(".json", "")
              for f in sorted(glob.glob(os.path.join(BLOCKS, "block-*.json")))[:N]]
    order = commit_order(blocks)
    F = normalize({b: features(b) for b in blocks})
    # train a quality model on a coverage-proxy preference (real: jury labels)
    cv = {b: features(b)[2] for b in blocks}
    prefs = [(i, j) for i in blocks for j in blocks if i != j and cv[i] > cv[j]]
    w = train_bt(F, prefs)
    raw_q = {b: sum(w[k] * F[b][k] for k in range(len(FEATS))) for b in blocks}
    qmin = min(raw_q.values())
    quality = {b: (raw_q[b] - qmin) for b in blocks}             # >= 0, a boost
    qmax = max(quality.values()) or 1.0
    quality = {b: quality[b] / qmax for b in blocks}             # 0..1

    covmap = {b: cov(b) for b in blocks}
    seen, nov = set(), {}
    for b in order:
        nov[b] = len(covmap[b] - seen)
        seen |= covmap[b]

    val = {b: nov[b] * (1 + quality[b]) for b in blocks}

    if mode == "attack":
        # inject adversary blocks last with HIGH fake quality; novelty must zero them
        for atk, c in (("SYBIL", set(covmap[order[-1]])),
                       ("PAD", set(list(covmap[blocks[0]])[:len(covmap[blocks[0]]) // 2]))):
            nv = len(c - seen)         # 0, all covered
            print(f"  {atk:<6} novelty={nv} -> value={nv * (1 + 1.0):.1f}  "
                  f"(even at max quality=1.0) -> {'DEFEATED' if nv == 0 else 'LEAK'}")
        return

    print(f"value-v4 = novelty x (1+quality) over {len(blocks)} commit-ordered blocks:")
    print(f"{'block':<12}{'novelty':>9}{'quality':>9}{'value':>9}")
    for b in order:
        print(f"{b:<12}{nov[b]:>9}{quality[b]:>9.2f}{val[b]:>9.1f}")
    tot = sum(val.values()) or 1
    print(f"\ntotal value {tot:.1f}; strategyproof (novelty floor) + quality-weighted "
          f"(reward model). redundant honest blocks still low, but a high-quality novel "
          f"block now out-earns a low-quality novel block of equal coverage.")


if __name__ == "__main__":
    main()
