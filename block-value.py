#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Multiplicative block value: pairwise comparison -> Shapley -> credit shares.

The last layer of the block-economy. Ownership (block-ownership.py) says WHO holds
a block; this says HOW MUCH it is worth -- and value is MULTIPLICATIVE (a share of
a whole), not an additive count, because the credit is the Shapley value of a
cooperative game defined by PAIRWISE comparisons (DeepFunding's elicitation +
VibeSwap's Shapley aggregation, applied to JARVIS's own blocks).

Pipeline (each layer pluggable, per the cooperative-game-elicitation-stack):
  elicit   : pairwise judgments W[i][j] = 1 if block i contributed more than j.
             Real = human jury / model panel (distilled, DeepFunding-style).
             Demo = a transparent PROXY signal, marked [proxy], so the math is
             exercised on real blocks without faking a jury.
  aggregate: v(S) = intra-coalition pairwise wins; Shapley phi_i over that game.
  normalize: phi -> shares summing to 1 (the multiplicative credit distribution).
  assign   : each block's share flows to its current owner (ownership registry).

Exact Shapley is 2^N -- demo scale (small N). Production = sampled approximation
(Shapley-5-axiom / DeepFunding distill). Stdlib only.

  value <id...>      compute pairwise->Shapley credit shares for the given blocks
"""
import json
import math
import os
import sys
from itertools import combinations

try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

HOME = os.path.expanduser("~")
BLOCKS = os.path.join(HOME, ".claude", "session-chain", "blocks")
SYS = os.path.join(HOME, ".claude", "projects", "C--Users-Will", "memory", "_system")
REGISTRY = os.path.join(SYS, "block_ownership.json")


def _block(bid):
    p = os.path.join(BLOCKS, bid if bid.endswith(".json") else bid + ".json")
    return json.load(open(p, encoding="utf-8"))


def _proxy_signal(bid):
    """[proxy] elicitation stand-in: a block 'contributes more' the more output it
    produced and the more checkpoints it carried. Transparent + deterministic.
    Replace with jury/model pairwise judgments for real scoring."""
    b = _block(bid)
    out = len(str(b.get("response", "")))
    chk = len(b.get("checkpoints", []) or [])
    return out + 50 * chk


def _pairwise(blocks):
    """W[i][j] = 1 if block i beats j under the elicitation. (proxy here)."""
    sig = {b: _proxy_signal(b) for b in blocks}
    W = {(i, j): (1 if sig[i] > sig[j] else 0) for i in blocks for j in blocks if i != j}
    return W, sig


def _v(S, W, universe):
    """characteristic function: total pairwise wins of coalition members against
    EVERYONE. v(N) = C(n,2) > 0 (every comparison has one winner), so shares
    normalize cleanly (efficiency: sum phi = v(N)).

    HONEST FINDING (surfaced by running it): two earlier games failed --
    intra-coalition wins are symmetric (every size-k coalition has C(k,2) internal
    wins -> equal Shapley), and external wins give v(N)=0 (can't normalize). The
    deeper truth: pairwise wins alone form an ADDITIVE game, so Shapley here equals
    the normalized Copeland win-share -- there is NO coalition synergy in pairwise
    data. Real multiplicativity (synergy) needs an OUTCOME-value v(S) measuring
    what a coalition of blocks actually achieves together; pairwise comparison is
    the elicitation layer, outcome-value is a separate layer (elicitation-stack).
    This is honest: we ship the additive fair-share now and name the synergy gap."""
    Sset = set(S)
    return sum(W.get((i, j), 0) for i in Sset for j in universe if i != j)


def _shapley(blocks, W):
    n = len(blocks)
    phi = {b: 0.0 for b in blocks}
    others = lambda b: [x for x in blocks if x != b]
    for b in blocks:
        rest = others(b)
        for k in range(len(rest) + 1):
            # weight = |S|! (n-|S|-1)! / n!  for each coalition S of size k from rest
            w = math.factorial(k) * math.factorial(n - k - 1) / math.factorial(n)
            for S in combinations(rest, k):
                marg = _v(tuple(S) + (b,), W, blocks) - _v(S, W, blocks)
                phi[b] += w * marg
    return phi


def _owner_of(bid, reg):
    owner = reg["genesis_owner"]
    for t in reg.get("transfers", []):
        if t["block_id"] == bid:
            owner = {"id": t["new_id"], "fpr": t["new_fpr"]}
    return owner


def cmd_value(blocks):
    if len(blocks) < 2:
        print("need >=2 blocks to compare"); return 1
    W, sig = _pairwise(blocks)
    phi = _shapley(blocks, W)
    total = sum(phi.values()) or 1.0
    shares = {b: phi[b] / total for b in blocks}  # multiplicative: sums to 1
    reg = json.load(open(REGISTRY, encoding="utf-8")) if os.path.exists(REGISTRY) else \
        {"genesis_owner": {"id": "jarvis@local", "fpr": "?"}, "transfers": []}
    print("block          [proxy]signal   Shapley value   credit share   owner")
    print("-" * 78)
    owner_credit = {}
    for b in sorted(blocks, key=lambda x: -shares[x]):
        o = _owner_of(b, reg)
        owner_credit[o["id"]] = owner_credit.get(o["id"], 0.0) + shares[b]
        print(f"{b:<14} {sig[b]:>12}   {phi[b]:>12.3f}   {shares[b]*100:>10.1f}%   {o['id']}")
    print("-" * 78)
    print("owner credit (multiplicative shares, sum=1):")
    for oid, c in sorted(owner_credit.items(), key=lambda x: -x[1]):
        print(f"  {oid}: {c*100:.1f}%")
    print("\nnote: pairwise signal is a [proxy] (output size + checkpoints); real "
          "scoring = jury/model pairwise judgments distilled (DeepFunding). Shapley "
          "is EXACT here (demo N); production = sampled approximation.")
    return 0


def main():
    a = sys.argv[1:]
    if not a or a[0] != "value":
        print(__doc__); return 0
    return cmd_value(a[1:])


if __name__ == "__main__":
    sys.exit(main() or 0)
