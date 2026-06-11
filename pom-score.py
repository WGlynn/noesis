#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Proof of Mind score = per-owner Myerson credit over verified owned blocks.

PRIVATE (krabby patty). Ties the three layers into the actual consensus weight:
  value (synergy Myerson over the provenance DAG)  x  ownership (who holds each
  block)  ->  PoM score per owner  ->  normalized consensus weights.

A validator's PoM is the synergy-weighted value of the blocks it owns: provable,
sybil-resistant (redundant blocks discounted by the synergy game), and earned, not
bought. Standalone; reuses the v2 value math (coverage v(S) [proxy] + sampled
Myerson over parent-DAG). Stdlib only.

  score [N] [T]    PoM scores + consensus weights over the first N blocks
"""
import glob
import json
import os
import random
import sys

try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

HOME = os.path.expanduser("~")
BLOCKS = os.path.join(HOME, ".claude", "session-chain", "blocks")
REGISTRY = os.path.join(HOME, ".claude", "projects", "C--Users-Will",
                        "memory", "_system", "block_ownership.json")


def _block(bid):
    return json.load(open(os.path.join(BLOCKS, bid + ".json"), encoding="utf-8"))


def coverage(bid):
    txt = str(_block(bid).get("response", "")).lower()
    return {txt[i:i + 5] for i in range(0, max(0, len(txt) - 4))} or {bid}


def vS(S, cov):
    u = set()
    for b in S:
        u |= cov[b]
    return len(u)


def edges_of(blocks):
    bs = set(blocks)
    E = set()
    for b in blocks:
        par = _block(b).get("parent")
        if isinstance(par, str):
            pid = os.path.basename(par).replace(".json", "")
            if pid in bs:
                E.add(frozenset((b, pid)))
    return E


def components(S, E):
    parent = {x: x for x in S}

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    for e in E:
        a, b = tuple(e)
        if a in parent and b in parent:
            parent[find(a)] = find(b)
    comp = {}
    for x in S:
        comp.setdefault(find(x), []).append(x)
    return list(comp.values())


def v_graph(S, cov, E):
    return sum(vS(c, cov) for c in components(set(S), E))


def myerson(blocks, cov, E, T):
    phi = {b: 0.0 for b in blocks}
    for t in range(T):
        rnd = random.Random(2000 + t)
        perm = blocks[:]
        rnd.shuffle(perm)
        running, prev = set(), 0.0
        for b in perm:
            running.add(b)
            cur = v_graph(running, cov, E)
            phi[b] += cur - prev
            prev = cur
    return {b: phi[b] / T for b in blocks}


def owner_of(bid, reg):
    o = reg["genesis_owner"]
    for t in reg.get("transfers", []):
        if t["block_id"] == bid:
            o = {"id": t["new_id"], "fpr": t["new_fpr"]}
    return o


def main():
    a = sys.argv[1:]
    N = int(a[1]) if len(a) > 1 else 8
    T = int(a[2]) if len(a) > 2 else 3000
    blocks = [os.path.basename(f).replace(".json", "")
              for f in sorted(glob.glob(os.path.join(BLOCKS, "block-*.json")))[:N]]
    cov = {b: coverage(b) for b in blocks}
    E = edges_of(blocks)
    val = myerson(blocks, cov, E, T)
    reg = json.load(open(REGISTRY, encoding="utf-8"))

    pom = {}
    for b in blocks:
        o = owner_of(b, reg)["id"]
        pom[o] = pom.get(o, 0.0) + max(0.0, val[b])
    tot = sum(pom.values()) or 1.0

    print(f"Proof-of-Mind scores over {len(blocks)} blocks (T={T} samples, DAG edges={len(E)})")
    print(f"{'validator (owner)':<26}{'PoM score':>12}{'consensus wt':>14}")
    print("-" * 52)
    for o, s in sorted(pom.items(), key=lambda x: -x[1]):
        print(f"{o[:25]:<26}{s:>12.3f}{s/tot*100:>13.1f}%")
    print("-" * 52)
    print("PoM = synergy-weighted (Myerson) value of owned blocks. Sybil-resistant: "
          "redundant blocks are discounted by the synergy game, so splitting a mind "
          "into many accounts does not multiply PoM. coverage v(S) is a [proxy] for a "
          "learned outcome-evaluator. One owner today (genesis); transfers distribute it.")


if __name__ == "__main__":
    main()
