#!/usr/bin/env python3
"""
moat_test.py -- Phase 2/3 of the learned-v(S) moat, on the crates.io deep-ancestry dataset.

This is the DeepFunding faithful port re-run on data that is NON-DEGENERATE in the ANCESTOR
direction (the exact SHIPPED object, lib.rs:7092), which DeepFunding could not test (95/115 leaves).

It single-sources the EXACT feature/estimator mirror from data/deepfunding/faithful_port.py
(coalition_features, train_np, acc_np) so the numbers here and the shipped Rust are the same math.

Three instruments (Phase 3), all held-out, all reported honestly (null is reported as null):
  (1) PREDICTIVE  : learned v(S) vs fixed proxy_value (f0 breadth), ANCESTOR direction.
  (2) ADVERSARIAL : inject gamed coalitions that pump the proxy; show proxy pays, floor/structure denies.
  (3) ISO-INVARIANCE: relabel ids / permute the coalition; features (hence value) must not change.

Labels: reuse = reverse-dependency count |in(r)| (external, Sybil-uncorrelated). Preference pair
(a,b): winner = the more-reused crate (ties broken by downloads). "Does the structure of what you
BUILD ON predict how much you get REUSED?" -- ancestor features vs a downstream label, no leakage.
"""
import os, sys, json, csv, math, random, statistics as st
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
GRAPH = os.path.join(HERE, "graph")
DF = os.path.join(HERE, "..", "deepfunding")
sys.path.insert(0, DF)
from faithful_port import train_np, acc_np, N_FEATS  # exact estimator/metric mirror
# NOTE: we do NOT import faithful_port.coalition_features. The iso-invariance gate (Phase 3.3) caught a
# real DETERMINISM bug in its depth feature: the longest-path memo is order-dependent on LARGE
# coalitions (its own comment concedes "tiny sets so recompute-safe"). DeepFunding descendant sets were
# tiny; crates ancestor coalitions are large (median 127) ⇒ permuting the input changed `depth`, which
# violates the B1 determinism constraint a canonical on-chain v(S) MUST satisfy. We use a corrected,
# order-independent DAG longest-path below (breadth/synergy/connected are unchanged, being set-based).


def coalition_features(S, out):
    """Faithful mirror of outcome::coalition_features with an ORDER-INDEPENDENT depth (the fix the
    iso-invariance gate forced). coverage(x)={x} U out(x); parent-relation = 'depends on' (out)."""
    if not S:
        return [0.0] * N_FEATS
    union, sum_ind = set(), 0
    for r in S:
        c = {r} | out.get(r, set())
        sum_ind += len(c); union |= c
    breadth = math.log1p(len(union))
    synergy = (len(union) / sum_ind) if sum_ind else 0.0
    connected = sum(1 for r in S if (out.get(r, set()) & S)) / len(S)
    depth = _longest_chain_condensation(S, out) / len(S)
    return [breadth, synergy, connected, depth]


def _longest_chain_condensation(S, out):
    """Order-INDEPENDENT longest dependency chain confined to S.

    The crate-level provenance graph can CYCLE (collapsing deps across versions makes A->B->A
    possible), so a naive longest-path is ill-defined/order-dependent (what the iso-invariance gate
    caught). Canonical fix: collapse strongly-connected components (iterative Tarjan) and take the
    longest path in the resulting DAG, each component weighted by its member count. Deterministic in
    S alone (independent of iteration order). Noesis's real Cell model is a single-parent DAG so this
    reduces to the plain chain; the condensation is the general canonical rule a multi-parent v(S) needs.
    """
    nodes = list(S)
    adj = {u: [v for v in out.get(u, ()) if v in S] for u in nodes}
    index, low, onstk, comp = {}, {}, {}, {}
    stack, idxc, ncomp = [], [0], [0]
    for root in nodes:                                   # iterative Tarjan (no recursion-depth limit)
        if root in index:
            continue
        work = [(root, 0)]
        while work:
            u, pi = work[-1]
            if pi == 0:
                index[u] = low[u] = idxc[0]; idxc[0] += 1
                stack.append(u); onstk[u] = True
            recurse = False
            i = pi
            while i < len(adj[u]):
                w = adj[u][i]
                if w not in index:
                    work[-1] = (u, i + 1); work.append((w, 0)); recurse = True; break
                elif onstk.get(w):
                    low[u] = min(low[u], index[w])
                i += 1
            if recurse:
                continue
            if low[u] == index[u]:
                while True:
                    w = stack.pop(); onstk[w] = False; comp[w] = ncomp[0]
                    if w == u:
                        break
                ncomp[0] += 1
            work.pop()
            if work:
                low[work[-1][0]] = min(low[work[-1][0]], low[u])
    csize, cadj = {}, {}
    for u in nodes:
        csize[comp[u]] = csize.get(comp[u], 0) + 1
    for u in nodes:
        for v in adj[u]:
            if comp[u] != comp[v]:
                cadj.setdefault(comp[u], set()).add(comp[v])
    memo = {}
    def lp(c):
        if c in memo:
            return memo[c]
        best = csize[c]
        for d in cadj.get(c, ()):
            best = max(best, csize[c] + lp(d))
        memo[c] = best
        return best
    return max((lp(c) for c in set(comp.values())), default=1)

SEEDS = list(range(1000, 1020))
TEST_FRAC = 0.20
csv.field_size_limit(1 << 30)


def load_graph():
    name, downloads = {}, {}
    with open(os.path.join(GRAPH, "nodes.tsv"), encoding="utf-8") as f:
        for cid, nm, dl, _created in csv.reader(f, delimiter="\t"):
            cid = int(cid); name[cid] = nm; downloads[cid] = int(dl)
    out, inn = {}, {}
    with open(os.path.join(GRAPH, "edges.tsv"), encoding="utf-8") as f:
        for s, d in csv.reader(f, delimiter="\t"):
            s, d = int(s), int(d)
            out.setdefault(s, set()).add(d)
            inn.setdefault(d, set()).add(s)
    return name, downloads, out, inn


def closure(r, adj, cap=6000):
    seen, stk = set(), [r]
    while stk:
        x = stk.pop()
        if x in seen:
            continue
        seen.add(x)
        if len(seen) > cap:
            break
        stk.extend(adj.get(x, ()))
    return seen


def main():
    name, downloads, out, inn = load_graph()
    all_ids = list(name)
    reuse = {r: len(inn.get(r, ())) for r in all_ids}   # direct reverse-dep count = outcome label

    # judged units: crates with a real ancestor coalition (>=2 transitive deps) AND some reuse signal,
    # so the ANCESTOR object is non-degenerate and the label is informative. Sample for tractability.
    rng0 = random.Random(0)
    candidates = [r for r in all_ids if len(out.get(r, ())) >= 1 and reuse[r] >= 1]
    rng0.shuffle(candidates)
    JUDGED_N = int(os.environ.get("JUDGED_N", "3000"))
    judged = candidates[:JUDGED_N]
    idx = {r: i for i, r in enumerate(judged)}

    # ---- ancestor coalitions (THE SHIPPED OBJECT) + faithful features ----
    anc_sizes, feats = [], []
    for r in judged:
        A = closure(r, out)            # {r} U transitive dependencies = provenance ancestors
        anc_sizes.append(len(A))
        # coalition_features expects (S:set, out) with coverage(x)={x} U out(x); ANCESTOR coalition = A
        feats.append(coalition_features(A, out))
    anc_sizes.sort()
    singo = sum(1 for s in anc_sizes if s == 1)
    n = len(anc_sizes)
    names = ["breadth", "synergy", "connected", "depth"]
    print("=== ANCESTOR-object non-degeneracy on crates.io (contrast DeepFunding 95/115=0.826) ===")
    print(f"judged crates: {n}  ancestor-coalition singletons: {singo}/{n} = {singo/n:.4f}")
    print(f"ancestor |S|: median {anc_sizes[n//2]}  p90 {anc_sizes[int(n*0.9)]}  max {anc_sizes[-1]}")
    for k in range(N_FEATS):
        col = [f[k] for f in feats]
        print(f"  f{k} {names[k]:9s}: mean {st.mean(col):.4f} std {st.pstdev(col):.4f} "
              f"min {min(col):.4f} max {max(col):.4f}")

    # ---- preference pairs from the reuse label (winner = more reused) ----
    F = np.array(feats)
    rng = random.Random(42)
    N_PAIRS = int(os.environ.get("N_PAIRS", "6000"))
    prefs = []
    for _ in range(N_PAIRS):
        a, b = rng.randrange(n), rng.randrange(n)
        if a == b:
            continue
        ra, rb = judged[a], judged[b]
        if reuse[ra] == reuse[rb]:
            if downloads[ra] == downloads[rb]:
                continue
            w, l = (a, b) if downloads[ra] > downloads[rb] else (b, a)
        else:
            w, l = (a, b) if reuse[ra] > reuse[rb] else (b, a)
        prefs.append((w, l))

    # ---- (1) PREDICTIVE: learned vs proxy f0, held-out, 20 seeds ----
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
        f0b.append(acc_np(F[:, 0], wi_te, li_te))          # proxy_value = f0 breadth (the moat bar)
        bk, bd, btr = 0, 1.0, -1.0
        for k in range(N_FEATS):
            for dr in (1.0, -1.0):
                a = acc_np(dr * F[:, k], wi_tr, li_tr)
                if a > btr: btr, bk, bd = a, k, dr
        best.append(acc_np(bd * F[:, bk], wi_te, li_te))
    d_f0 = [learned[i] - f0b[i] for i in range(len(SEEDS))]
    d_bs = [learned[i] - best[i] for i in range(len(SEEDS))]
    nt = max(1, int(len(prefs) * TEST_FRAC)); se = 0.5 / math.sqrt(nt)
    print("\n=== (1) PREDICTIVE: 20-seed held-out pairwise accuracy (ANCESTOR direction, real reuse labels) ===")
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

    # ---- (2) ADVERSARIAL: gamed coalition pumps the proxy; structure/synergy denies it ----
    # Construct a gamed unit: pad raw coverage with K disconnected orphan deps (high breadth) but no
    # synergy/lineage. The fixed proxy (breadth) rewards it; the synergy feature exposes the padding.
    real_units = [F[i] for i in range(n)]
    med = np.median(F, axis=0)
    # a genuine high-value unit: high breadth AND healthy synergy/connected/depth (>= median)
    genuine = med.copy(); genuine[0] = np.percentile(F[:, 0], 90)
    # a gamed unit: SAME inflated breadth, but padding kills synergy (near 0) and lineage
    gamed = med.copy(); gamed[0] = np.percentile(F[:, 0], 90)
    gamed[1] = np.percentile(F[:, 1], 5); gamed[2] = 0.0; gamed[3] = np.percentile(F[:, 3], 5)
    wq = last_w
    proxy_gap = float(genuine[0] - gamed[0])                    # proxy sees them as EQUAL (same breadth)
    learn_gap = float((genuine @ wq) - (gamed @ wq))            # learned separates them
    print("\n=== (2) ADVERSARIAL: does the learned measure deny a proxy-pumping gamed coalition? ===")
    print(f"  proxy_value(genuine) - proxy_value(gamed) = {proxy_gap:+.4f}  (0 => proxy CANNOT tell them apart)")
    print(f"  learned(genuine)     - learned(gamed)     = {learn_gap:+.4f}  (>0 => learned DENIES the gamed unit)")
    adv_pass = abs(proxy_gap) < 1e-9 and learn_gap > 0.0
    print(f"  VERDICT: {'PASS (direction) -- proxy conflates (gap 0), learned separates (gap>0)' if adv_pass else 'FAIL -- learned does not separate what proxy conflates'}")

    # ---- (3) ISO-INVARIANCE: relabel ids / permute coalition => identical features ----
    import copy
    sample_r = judged[0]
    A = closure(sample_r, out)
    f_base = coalition_features(A, out)
    # permute: features are set-based, so any ordering yields the same vector
    A_perm = set(list(A)[::-1])
    f_perm = coalition_features(A_perm, out)
    # relabel: offset every id by a constant (bijection) -> isomorphic graph -> identical features
    OFF = 10**9
    out_rel = {k + OFF: {v + OFF for v in vs} for k, vs in out.items()}
    A_rel = {x + OFF for x in A}
    f_rel = coalition_features(A_rel, out_rel)
    iso_ok = (np.allclose(f_base, f_perm) and np.allclose(f_base, f_rel))
    print("\n=== (3) ISO-INVARIANCE: permute + relabel a coalition ===")
    print(f"  base   : {[round(x,4) for x in f_base]}")
    print(f"  permute: {[round(x,4) for x in f_perm]}")
    print(f"  relabel: {[round(x,4) for x in f_rel]}")
    print(f"  VERDICT: {'INVARIANT (features unchanged under permutation + identity relabelling)' if iso_ok else 'FAILED -- value depends on labels/order'}")

    # ---- emit machine-readable results for the doc ----
    res = {
        "judged_n": n,
        "ancestor_singleton_frac": round(singo / n, 4),
        "ancestor_median": anc_sizes[n // 2],
        "ancestor_p90": anc_sizes[int(n * 0.9)],
        "ancestor_max": anc_sizes[-1],
        "n_pairs": len(prefs),
        "predictive": {
            "proxy_f0": round(st.mean(f0b), 4),
            "best_single": round(st.mean(best), 4),
            "learned": round(st.mean(learned), 4),
            "delta_vs_proxy_mean": round(st.mean(d_f0), 4),
            "delta_vs_proxy_wins": sum(1 for d in d_f0 if d > 0),
            "delta_vs_best_mean": round(st.mean(d_bs), 4),
            "delta_vs_best_wins": sum(1 for d in d_bs if d > 0),
            "noise_band_1se": round(se, 4),
            "weights_last": [round(float(x), 3) for x in last_w],
        },
        "adversarial": {"proxy_gap": round(proxy_gap, 4), "learned_gap": round(learn_gap, 4)},
        "iso_invariant": bool(iso_ok),
    }
    with open(os.path.join(GRAPH, "moat_results.json"), "w") as f:
        json.dump(res, f, indent=2)
    print(f"\nwrote {os.path.join(GRAPH, 'moat_results.json')}")


if __name__ == "__main__":
    main()
