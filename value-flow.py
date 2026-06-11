#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Eigenvector value-flow over the provenance DAG + two-level recursion.

PRIVATE (AFK items 1 + 3). Two pieces:

  (3) FLOW: a block gets credit not only for its own value but for the value of
      what was built ON it. Propagate value backward along parent edges with a
      damping factor d<1 (PageRank / EigenTrust style): converges, and damping
      defeats self-referential loops. Uses the REAL chain linkage (parent hash ->
      block), not a guess.

  (1) RECURSION: split a block's flow-value among its INTRA-block contributors
      (operator = the prompt, model = the response) by the Shapley value of a
      2-player sub-game with genuine synergy: v(both) is the full output, v(op
      only) and v(model only) are the partial outputs. Same machinery one level
      down -> the economy is two-level recursive.

Stdlib only. coverage = [proxy] for the learned reward model.

  flow [N] [iters]    value-flow over first N blocks + recursion on the top block
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


def _block(bid):
    return json.load(open(os.path.join(BLOCKS, bid + ".json"), encoding="utf-8"))


def _cov(txt):
    t = str(txt).lower()
    return {t[i:i + 5] for i in range(0, max(0, len(t) - 4))}


def own_value(bid):
    return float(len(_cov(_block(bid).get("response", "")))) or 1.0


def build_dag(blocks):
    """Real parent edges via hash -> id map (child -> parent it built on)."""
    h2id = {}
    for b in blocks:
        h = _block(b).get("hash")
        if h:
            h2id[h] = b
    children = {b: [] for b in blocks}   # parent -> [children]
    for b in blocks:
        par = _block(b).get("parent")
        pid = h2id.get(par)
        if pid and pid in children and pid != b:
            children[pid].append(b)
    return children


def value_flow(blocks, children, d=0.85, iters=100):
    """flow(b) = own(b) + d * sum over children c of flow(c)  (credit for what
    built on b). Damping d<1 guarantees convergence + kills self-loops."""
    own = {b: own_value(b) for b in blocks}
    flow = dict(own)
    for _ in range(iters):
        nxt = {}
        for b in blocks:
            nxt[b] = own[b] + d * sum(flow[c] for c in children[b])
        if max(abs(nxt[b] - flow[b]) for b in blocks) < 1e-9:
            flow = nxt
            break
        flow = nxt
    return own, flow


def recurse_block(bid):
    """Split a block's value between operator (prompt) and model (response) by the
    Shapley value of a 2-player synergy sub-game. v(both)=union coverage,
    v(op)=prompt coverage, v(model)=response coverage."""
    b = _block(bid)
    cp, cr = _cov(b.get("prompt", "")), _cov(b.get("response", ""))
    v_op, v_md, v_both = len(cp), len(cr), len(cp | cr)
    # Shapley of 2 players: phi_i = 1/2 * v({i}) + 1/2 * (v(N) - v({other}))
    phi_op = 0.5 * v_op + 0.5 * (v_both - v_md)
    phi_md = 0.5 * v_md + 0.5 * (v_both - v_op)
    tot = (phi_op + phi_md) or 1.0
    synergy = v_both - (v_op + v_md)   # <0 => redundant overlap (sub-additive)
    return {"operator": phi_op / tot, "model": phi_md / tot, "synergy": synergy,
            "v_op": v_op, "v_md": v_md, "v_both": v_both}


def main():
    a = sys.argv[1:]
    N = int(a[1]) if len(a) > 1 else 12
    blocks = [os.path.basename(f).replace(".json", "")
              for f in sorted(glob.glob(os.path.join(BLOCKS, "block-*.json")))[:N]]
    children = build_dag(blocks)
    edges = sum(len(v) for v in children.values())
    own, flow = value_flow(blocks, children)

    print(f"=== (3) eigenvector value-flow over {len(blocks)} blocks, {edges} real DAG edges ===")
    print(f"{'block':<12}{'own':>8}{'flow':>10}{'uplift':>9}  (flow = own + credit for what built on it)")
    for b in sorted(blocks, key=lambda x: -flow[x])[:8]:
        up = flow[b] / own[b] if own[b] else 1.0
        print(f"{b:<12}{own[b]:>8.0f}{flow[b]:>10.1f}{up:>8.2f}x")

    top = max(blocks, key=lambda x: flow[x])
    r = recurse_block(top)
    print(f"\n=== (1) two-level recursion: split {top}'s value among intra-block contributors ===")
    print(f"  operator (prompt): {r['operator']*100:.1f}%   model (response): {r['model']*100:.1f}%")
    print(f"  sub-game: v(op)={r['v_op']} v(model)={r['v_md']} v(both)={r['v_both']} "
          f"synergy={r['synergy']:+d} ({'complementary' if r['synergy']>0 else 'overlapping/redundant'})")
    print("\nnote: own-value uses coverage [proxy] (-> learned reward model in prod); flow "
          "damping d=0.85 (PageRank-style) converges + defeats self-reference; recursion is "
          "the same Shapley machinery one level down.")


if __name__ == "__main__":
    main()
