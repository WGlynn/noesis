#!/usr/bin/env python3
"""Experiment 01 — does consequence-credit reproduce human pairwise worth judgments?

Project: attribution-by-consequence (Fork B, cross-project ecosystem credit).
Started 2026-09-29.

HONEST FLOOR:
- Predicting DeepFunding pairwise weights is the KNOWN contest task; existing solutions
  exist. This is NOT a contest entry and NOT novel. Its only job: sanity-check whether a
  consequence-credit score (here: weighted PageRank over the dependency graph — off-the-shelf)
  tracks human relative-worth AT ALL, and by how much it beats a dumb baseline (star count).
- PageRank here is a stand-in for the wash-resistant Noesis flow/Shapley credit. If even
  PageRank correlates, the consequence thesis has a floor; if it doesn't, that's the finding.

Data (local):
  graph  : data/deepfunding/dependency-graph/datasets/v2-graph/dependency-graph-v2.csv
           (seed_repo depends_on dependency_repo)
  labels : data/deepfunding/mini-contest/dataset.csv  (project_a, project_b, weight_a, weight_b)
  base   : data/deepfunding/dependency-graph/datasets/oso/repo_and_funding_stats.csv (star_count)
"""
import csv, io, math, os

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GRAPH = os.path.join(ROOT, "data/deepfunding/dependency-graph/datasets/v2-graph/dependency-graph-v2.csv")
LABELS = os.path.join(ROOT, "data/deepfunding/mini-contest/dataset.csv")
STATS = os.path.join(ROOT, "data/deepfunding/dependency-graph/datasets/oso/repo_and_funding_stats.csv")

def norm(u):
    u = (u or "").strip().lower().rstrip("/")
    if u.endswith(".git"): u = u[:-4]
    return u

# --- load graph: edge seed -> dependency (seed uses dependency) -------------
edges = []
nodes = set()
with io.open(GRAPH, encoding="utf-8", errors="replace") as f:
    r = csv.DictReader(f)
    for row in r:
        s, d = norm(row.get("seed_repo")), norm(row.get("dependency_repo"))
        if s and d and s != d:
            edges.append((s, d)); nodes.add(s); nodes.add(d)
print(f"graph: {len(nodes)} nodes, {len(edges)} edges")

# --- weighted PageRank; credit flows seed->dependency so widely-depended-upon
#     repos score high (consequence = being built upon). damping 0.85. --------
out = {}
for s, d in edges:
    out.setdefault(s, []).append(d)
N = len(nodes)
pr = {n: 1.0 / N for n in nodes}
DAMP = 0.85
for _ in range(60):
    nxt = {n: (1 - DAMP) / N for n in nodes}
    dangling = 0.0
    for n in nodes:
        outs = out.get(n)
        if not outs:
            dangling += pr[n]
        else:
            share = DAMP * pr[n] / len(outs)
            for d in outs:
                nxt[d] += share
    # redistribute dangling mass uniformly
    dm = DAMP * dangling / N
    for n in nodes:
        nxt[n] += dm
    pr = nxt

# --- baseline: star_count ---------------------------------------------------
stars = {}
if os.path.exists(STATS):
    with io.open(STATS, encoding="utf-8", errors="replace") as f:
        for row in csv.DictReader(f):
            try:
                stars[norm(row.get("url"))] = float(row.get("star_count") or 0)
            except ValueError:
                pass

# --- evaluate against human pairwise labels ---------------------------------
def spearman(xs, ys):
    def rank(v):
        order = sorted(range(len(v)), key=lambda i: v[i])
        rk = [0.0] * len(v); i = 0
        while i < len(v):
            j = i
            while j + 1 < len(v) and v[order[j+1]] == v[order[i]]: j += 1
            avg = (i + j) / 2.0
            for k in range(i, j+1): rk[order[k]] = avg
            i = j + 1
        return rk
    rx, ry = rank(xs), rank(ys)
    n = len(xs); mx = sum(rx)/n; my = sum(ry)/n
    num = sum((rx[i]-mx)*(ry[i]-my) for i in range(n))
    den = math.sqrt(sum((rx[i]-mx)**2 for i in range(n)) * sum((ry[i]-my)**2 for i in range(n)))
    return num/den if den else 0.0

def evaluate(score, name):
    n_pairs = n_cov = correct = 0
    pred_share, human_share = [], []
    with io.open(LABELS, encoding="utf-8", errors="replace") as f:
        for row in csv.DictReader(f):
            n_pairs += 1
            a, b = norm(row.get("project_a")), norm(row.get("project_b"))
            try:
                wa, wb = float(row["weight_a"]), float(row["weight_b"])
            except (ValueError, KeyError):
                continue
            sa, sb = score.get(a), score.get(b)
            if sa is None or sb is None or (sa == 0 and sb == 0):
                continue
            n_cov += 1
            # pairwise direction accuracy
            if (sa - sb) * (wa - wb) > 0: correct += 1
            elif (sa - sb) == 0 and (wa - wb) == 0: correct += 1
            pred_share.append(sa / (sa + sb) if (sa + sb) else 0.5)
            human_share.append(wa)
    acc = correct / n_cov if n_cov else 0.0
    rho = spearman(pred_share, human_share) if n_cov > 2 else 0.0
    print(f"\n[{name}]")
    print(f"  pairs total={n_pairs}  evaluable(both nodes scored)={n_cov}  coverage={n_cov/n_pairs:.1%}")
    print(f"  pairwise direction accuracy = {acc:.3f}   (0.5 = coin flip)")
    print(f"  Spearman(predicted share, human weight_a) = {rho:.3f}")

evaluate(pr, "consequence-credit (weighted PageRank)")
if stars:
    evaluate(stars, "baseline (star_count)")
