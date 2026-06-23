#!/usr/bin/env python3
"""
faithful_port.py -- THE faithful set-level moat test, and an honest topology diagnosis.

Mirrors node/src/lib.rs::outcome::{coalition_features, train, v_outcome, proxy_value,
pairwise_accuracy} EXACTLY, at the set level, on REAL DeepFunding jury labels.

KEY FINDING this script makes explicit (run it): on the DeepFunding mini-contest graph
the judged repos are LEAVES. Their provenance-ANCESTOR closures (the coalition the
shipped Rust `outcome` model scores -- cell U its provenance ancestors, lib.rs:1173) are
SINGLETONS for 95/115 repos, so the set-level features f1/f2/f3 degenerate to constants.
The exact shipped quantity therefore cannot be meaningfully learned on THIS data -- a
TOPOLOGY block, not a label block. (The prior RESULTS.md null used single-repo degree
proxies and missed this; the faithful port reveals it.)

So we test the moat in the ONLY direction this topology supports: the DESCENDANT
coalition D(r) = {r} U {repos that transitively depend on r} -- the body of work built
ON r, the graph-derivable "foundational-ness" object. Same Rust feature formulas, dual
DAG direction. Honest caveat carried in the output: this is the provenance-DAG features
in the descendant direction, not the shipped ancestor direction.

coverage(x) = {x} U out_neighbors(x)  (the unit plus the deps it integrates).
"""
import json, csv, os, math, random, statistics as st
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
GRAPH_JSON = os.path.join(HERE, "dependency-graph", "graph", "unweighted_graph.json")
DATASET = os.path.join(HERE, "mini-contest", "dataset.csv")
N_FEATS, ITERS, LR = 4, 5000, 0.5
SEEDS = list(range(1000, 1020))
TEST_FRAC = 0.20


def norm(u): return u.strip().lower().rstrip("/")


def load():
    g = json.load(open(GRAPH_JSON, encoding="utf-8"))
    nodes = {norm(n["id"]) for n in g["nodes"]}
    out = {r: set() for r in nodes}
    inn = {r: set() for r in nodes}
    for e in g["links"]:
        s, t = norm(e["source"]), norm(e["target"])
        if s in nodes and t in nodes and s != t:
            out[s].add(t); inn[t].add(s)
    drepos, comps = set(), []
    for r in csv.DictReader(open(DATASET, encoding="utf-8")):
        a, b = norm(r["project_a"]), norm(r["project_b"])
        drepos |= {a, b}
        wa, wb = float(r["weight_a"]), float(r["weight_b"])
        comps.append((a, b) if wa >= wb else (b, a))
    return nodes, out, inn, drepos, comps


def closure(r, adj):
    seen, stk = set(), [r]
    while stk:
        x = stk.pop()
        if x in seen: continue
        seen.add(x)
        stk.extend(adj.get(x, ()))
    return seen


def coalition_features(S, out):
    """Faithful mirror of outcome::coalition_features. parent-relation = 'depends on' (out)."""
    if not S: return [0.0] * N_FEATS
    union, sum_ind = set(), 0
    for r in S:
        c = {r} | out.get(r, set())
        sum_ind += len(c); union |= c
    breadth = math.log1p(len(union))
    synergy = (len(union) / sum_ind) if sum_ind else 0.0
    connected = sum(1 for r in S if (out.get(r, set()) & S)) / len(S)
    # longest dependency chain confined to S (induced sub-DAG longest path), cycle-safe
    memo = {}
    def lp(u, stack):
        if u in memo: return memo[u]
        best = 1
        for v in out.get(u, ()):
            if v in S and v not in stack:
                best = max(best, 1 + lp(v, stack | {v}))
        # memo valid only when no in-stack edge pruned; tiny sets so recompute-safe
        memo[u] = best
        return best
    longest = max((lp(r, {r}) for r in S), default=1)
    depth = longest / len(S)
    return [breadth, synergy, connected, depth]


def sigmoid(x):
    return 0.0 if x < -60 else (1.0 if x > 60 else 1.0 / (1.0 + math.exp(-x)))


def train_np(F, wi, li):
    """Vectorized EXACT mirror of outcome::train. F=(n,4) feats; wi/li=winner/loser idx."""
    w = np.zeros(N_FEATS)
    diff = F[wi] - F[li]                     # (m,4)
    denom = max(1, len(wi))
    for _ in range(ITERS):
        p = 1.0 / (1.0 + np.exp(-(diff @ w)))   # (m,)
        g = ((1.0 - p)[:, None] * diff).sum(0) / denom - 1e-3 * w
        w += LR * g
    return w


def acc_np(scores, wi, li):
    """pairwise_accuracy: 1 if score(w)>score(l), 0.5 tie, 0 else."""
    if len(wi) == 0: return 0.0
    sw, sl = scores[wi], scores[li]
    return float((np.where(sw > sl, 1.0, np.where(sw == sl, 0.5, 0.0))).mean())


def main():
    nodes, out, inn, drepos, comps = load()
    joined = sorted(nodes & drepos)
    idx = {u: i for i, u in enumerate(joined)}

    # ---- headline topology diagnosis: ancestor object is leaf-degenerate ----
    anc = [len(closure(r, out)) for r in joined]
    des = [len(closure(r, inn)) for r in joined]
    anc_singletons = sum(1 for s in anc if s == 1)
    print("=== TOPOLOGY DIAGNOSIS (why the exact shipped object can't be tested here) ===")
    print(f"judged repos joined: {len(joined)}")
    print(f"ANCESTOR closure (the SHIPPED Rust object): singletons {anc_singletons}/{len(joined)} "
          f"-> f1/f2/f3 DEGENERATE; med |S|={int(st.median(anc))} max {max(anc)}")
    print(f"DESCENDANT closure (foundational-ness, testable): singletons "
          f"{sum(1 for s in des if s==1)}/{len(joined)} med |S|={int(st.median(des))} max {max(des)}")

    # ---- testable direction: descendant-coalition features ----
    feats = []
    for u in joined:
        D = closure(u, inn)          # r + everything that (transitively) builds on r
        feats.append(coalition_features(D, out))
    prefs = [(idx[w], idx[l]) for (w, l) in comps if w in idx and l in idx]

    names = ["breadth", "synergy", "connected", "depth"]
    print(f"\n=== descendant-coalition feature variation (prefs kept {len(prefs)}/{len(comps)}) ===")
    for k in range(N_FEATS):
        col = [f[k] for f in feats]
        print(f"  f{k} {names[k]:9s}: mean {st.mean(col):.4f} std {st.pstdev(col):.4f} "
              f"min {min(col):.4f} max {max(col):.4f}")

    F = np.array(feats)
    learned, f0b, best = [], [], []
    last_w = None
    for seed in SEEDS:
        rnd = random.Random(seed)
        order = prefs[:]; rnd.shuffle(order)
        nt = max(1, int(len(order) * TEST_FRAC))
        test, tr = order[:nt], order[nt:]
        wi_tr = np.array([p[0] for p in tr]); li_tr = np.array([p[1] for p in tr])
        wi_te = np.array([p[0] for p in test]); li_te = np.array([p[1] for p in test])
        w = train_np(F, wi_tr, li_tr); last_w = w
        learned.append(acc_np(1.0 / (1.0 + np.exp(-(F @ w))), wi_te, li_te))
        f0b.append(acc_np(F[:, 0], wi_te, li_te))  # proxy_value = f0 breadth (defined moat bar)
        bk, bd, btr = 0, 1.0, -1.0
        for k in range(N_FEATS):
            for dr in (1.0, -1.0):
                a = acc_np(dr * F[:, k], wi_tr, li_tr)
                if a > btr: btr, bk, bd = a, k, dr
        best.append(acc_np(bd * F[:, bk], wi_te, li_te))

    d_f0 = [learned[i]-f0b[i] for i in range(len(SEEDS))]
    d_bs = [learned[i]-best[i] for i in range(len(SEEDS))]
    nt = max(1, int(len(prefs)*TEST_FRAC)); se = 0.5/math.sqrt(nt)
    print("\n=== 20-seed held-out pairwise accuracy (descendant framing) ===")
    print(f"  coin-flip floor       : 0.5000")
    print(f"  proxy f0 (breadth)    : mean {st.mean(f0b):.4f} std {st.pstdev(f0b):.4f}")
    print(f"  best single feature   : mean {st.mean(best):.4f} std {st.pstdev(best):.4f}")
    print(f"  LEARNED BT (4 feats)  : mean {st.mean(learned):.4f} std {st.pstdev(learned):.4f}")
    print(f"  delta vs proxy f0     : mean {st.mean(d_f0):+.4f} std {st.pstdev(d_f0):.4f} "
          f"wins {sum(1 for d in d_f0 if d>0)}/20")
    print(f"  delta vs best single  : mean {st.mean(d_bs):+.4f} std {st.pstdev(d_bs):.4f} "
          f"wins {sum(1 for d in d_bs if d>0)}/20")
    print(f"  1-SE noise band ({nt}-pair): +/-{se:.4f}")
    print(f"  learned weights (last seed): {[round(x,3) for x in last_w]}")


if __name__ == "__main__":
    main()
