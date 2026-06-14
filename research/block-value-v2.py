#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""block-value v2 — synergy-bearing value, Myerson over the provenance DAG, sampled.

PRIVATE (krabby patty). Fixes the v0 misuse (pairwise wins -> additive game ->
Shapley collapses to normalized Copeland). The roadmap Will approved:

  elicit   : Bradley-Terry MLE turns pairwise wins into CARDINAL strengths
             (principled, replaces the ad-hoc win-count).
  value    : a SUBMODULAR outcome-value v(S) = |union of block coverage sets|.
             Real synergy -- a redundant block adds little, a novel block adds a
             lot. [proxy] for a learned outcome-evaluator (RLHF reward model) in
             production; here coverage = content shingles, no API needed.
  aggregate: MYERSON value -- Shapley of the graph-restricted game v^g(S) =
             sum over connected components C of S (in the provenance DAG) of v(C).
             Value flows along parent links, not arbitrary coalitions.
  scale    : Monte-Carlo permutation sampling (Data-Shapley style); exact is 2^N.

Proves the math now earns its keep: on a synergy game Shapley/Myerson != the
normalized win-share, and they reward pivotal / penalize redundant blocks.

  demo [N] [T]    run on the first N real blocks, T sample permutations
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


def _block(bid):
    return json.load(open(os.path.join(BLOCKS, bid + ".json"), encoding="utf-8"))


def coverage(bid):
    """[proxy] what the block 'covers' = set of 5-char shingles of its response.
    Submodular union -> genuine synergy. Production: learned outcome-evaluator."""
    txt = str(_block(bid).get("response", "")).lower()
    return {txt[i:i + 5] for i in range(0, max(0, len(txt) - 4))} or {bid}


def vS(S, cov):
    u = set()
    for b in S:
        u |= cov[b]
    return len(u)


# ---- Bradley-Terry: pairwise wins -> cardinal strengths (MM algorithm) ----
def bradley_terry(blocks, W, iters=200):
    p = {b: 1.0 for b in blocks}
    wins = {b: sum(W[(b, j)] for j in blocks if j != b) for b in blocks}
    for _ in range(iters):
        np_ = {}
        for i in blocks:
            denom = sum((1.0) / (p[i] + p[j]) for j in blocks if j != i)
            np_[i] = (wins[i] or 1e-9) / (denom or 1e-9)
        s = sum(np_.values()) or 1.0
        p = {b: np_[b] / s for b in blocks}
    return p


# ---- provenance DAG: connected components of a coalition under parent edges ----
def edges_of(blocks):
    bs = set(blocks)
    E = set()
    for b in blocks:
        par = _block(b).get("parent")
        # parent may be an id or a hash; match by block-id stem if present
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
    """Myerson restricted game: only connected sub-coalitions create value."""
    return sum(vS(c, cov) for c in components(set(S), E))


def sampled_value(blocks, cov, E, T, restricted):
    phi = {b: 0.0 for b in blocks}
    val = (lambda S: v_graph(S, cov, E)) if restricted else (lambda S: vS(S, cov))
    idx = list(range(len(blocks)))
    for t in range(T):
        # deterministic-but-varied shuffle (seed by t) so runs are reproducible
        rnd = random.Random(1000 + t)
        perm = blocks[:]
        rnd.shuffle(perm)
        running, prev = set(), 0.0
        for b in perm:
            running.add(b)
            cur = val(running)
            phi[b] += cur - prev
            prev = cur
    for b in phi:
        phi[b] /= T
    return phi


def shares(phi):
    tot = sum(phi.values()) or 1.0
    return {b: phi[b] / tot for b in phi}


def main():
    a = sys.argv[1:]
    N = int(a[1]) if len(a) > 1 else 6
    T = int(a[2]) if len(a) > 2 else 3000
    blocks = [os.path.basename(f).replace(".json", "")
              for f in sorted(glob.glob(os.path.join(BLOCKS, "block-*.json")))[:N]]
    cov = {b: coverage(b) for b in blocks}
    # pairwise from coverage size (proxy elicitation)
    W = {(i, j): (1 if len(cov[i]) > len(cov[j]) else 0)
         for i in blocks for j in blocks if i != j}
    E = edges_of(blocks)

    bt = bradley_terry(blocks, W)
    sh = shares(sampled_value(blocks, cov, E, T, restricted=False))   # Shapley (synergy)
    my = shares(sampled_value(blocks, cov, E, T, restricted=True))    # Myerson (graph)
    wins = {b: sum(W[(b, j)] for j in blocks if j != b) for b in blocks}
    wtot = sum(wins.values()) or 1
    cop = {b: wins[b] / wtot for b in blocks}                          # additive Copeland

    print(f"blocks={len(blocks)}  T={T} samples  |  DAG edges among them: {len(E)}")
    print(f"{'block':<12}{'cover':>6}{'Copeland%':>11}{'Shapley%':>10}{'Myerson%':>10}{'BradleyT%':>11}")
    print("-" * 60)
    for b in sorted(blocks, key=lambda x: -sh[x]):
        print(f"{b:<12}{len(cov[b]):>6}{cop[b]*100:>10.1f}%{sh[b]*100:>9.1f}%{my[b]*100:>9.1f}%{bt[b]*100:>10.1f}%")
    print("-" * 60)
    # the proof: Shapley (synergy) must DIFFER from additive Copeland
    diff = sum(abs(sh[b] - cop[b]) for b in blocks)
    print(f"L1 |Shapley - Copeland| = {diff:.3f}  -> "
          f"{'SYNERGY captured: Shapley is load-bearing (≠ win-share)' if diff > 0.02 else 'collapsed (additive)'}")
    print("note: coverage v(S) is a [proxy] for a learned outcome-evaluator; Myerson "
          "restricts to DAG-connected coalitions; values are sampled (Data-Shapley).")


if __name__ == "__main__":
    main()
