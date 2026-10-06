#!/usr/bin/env python3
"""Experiment 02 — where value and popularity DIVERGE.

exp01 finding: consequence-credit (PageRank) LOST to star_count head-to-head at predicting
human pairwise weights (0.489 vs 0.622 acc), and the label is popularity-contaminated
(stars predict it at rho=0.26). So head-to-head on that label is the wrong test.

This test asks the only fair question: on pairs where STARS ARE ~TIED (popularity can only
coin-flip), does consequence-credit still pick the human-preferred project? And does adding
credit on top of stars beat stars-alone (augmentation delta)? Real data, no synthetic.

HONEST FLOOR: a true temporal test (train graph@T, predict who gains funding after T) needs
timestamped edges the v2 graph lacks -- deferred. This is the divergence slice, not time.
"""
import csv, io, math, os

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GRAPH = os.path.join(ROOT, "data/deepfunding/dependency-graph/datasets/v2-graph/dependency-graph-v2.csv")
LABELS = os.path.join(ROOT, "data/deepfunding/mini-contest/dataset.csv")
STATS = os.path.join(ROOT, "data/deepfunding/dependency-graph/datasets/oso/repo_and_funding_stats.csv")

def norm(u):
    u = (u or "").strip().lower().rstrip("/")
    return u[:-4] if u.endswith(".git") else u

edges, nodes = [], set()
with io.open(GRAPH, encoding="utf-8", errors="replace") as f:
    for row in csv.DictReader(f):
        s, d = norm(row.get("seed_repo")), norm(row.get("dependency_repo"))
        if s and d and s != d:
            edges.append((s, d)); nodes.add(s); nodes.add(d)

out = {}
for s, d in edges: out.setdefault(s, []).append(d)
N = len(nodes); pr = {n: 1.0/N for n in nodes}; DAMP = 0.85
for _ in range(80):
    nxt = {n: (1-DAMP)/N for n in nodes}; dang = 0.0
    for n in nodes:
        o = out.get(n)
        if not o: dang += pr[n]
        else:
            sh = DAMP*pr[n]/len(o)
            for d in o: nxt[d] += sh
    dm = DAMP*dang/N
    for n in nodes: nxt[n] += dm
    pr = nxt

stars = {}
with io.open(STATS, encoding="utf-8", errors="replace") as f:
    for row in csv.DictReader(f):
        try: stars[norm(row.get("url"))] = float(row.get("star_count") or 0)
        except ValueError: pass

# collect evaluable pairs: need stars AND credit for both endpoints
pairs = []
with io.open(LABELS, encoding="utf-8", errors="replace") as f:
    for row in csv.DictReader(f):
        a, b = norm(row.get("project_a")), norm(row.get("project_b"))
        try: wa, wb = float(row["weight_a"]), float(row["weight_b"])
        except (ValueError, KeyError): continue
        if a in pr and b in pr and a in stars and b in stars:
            pairs.append((a,b,wa,wb))
print(f"evaluable pairs (stars+credit both) = {len(pairs)}")

def acc(pairs, score):
    if not pairs: return float('nan'), 0
    c = sum(1 for a,b,wa,wb in pairs if (score(a)-score(b))*(wa-wb) > 0)
    return c/len(pairs), len(pairs)

star_s = lambda x: stars[x]; cred_s = lambda x: pr[x]

a_all, n = acc(pairs, star_s); print(f"\nALL evaluable pairs (n={n}):")
print(f"  stars acc            = {a_all:.3f}")
print(f"  consequence-credit   = {acc(pairs, cred_s)[0]:.3f}")

# popularity-tied slice: |ln(1+sa)-ln(1+sb)| <= tau
for tau in (0.25, 0.5, 1.0):
    tied = [p for p in pairs if abs(math.log1p(stars[p[0]])-math.log1p(stars[p[1]])) <= tau]
    sa,_ = acc(tied, star_s); ca,_ = acc(tied, cred_s)
    print(f"\nSTARS-TIED slice tau={tau} (n={len(tied)}):  stars={sa:.3f} (should ~0.5)  credit={ca:.3f}")

# augmentation delta: stars alone vs stars+credit tie-break (defer to credit when stars ~tied)
def combined(a,b,tau=0.5):
    if abs(math.log1p(stars[a])-math.log1p(stars[b])) <= tau:
        return pr[a]-pr[b]
    return stars[a]-stars[b]
c = sum(1 for a,b,wa,wb in pairs if combined(a,b)*(wa-wb) > 0)
print(f"\nAUGMENTATION DELTA:  stars-alone={a_all:.3f}  stars+credit-tiebreak={c/len(pairs):.3f}  delta={c/len(pairs)-a_all:+.3f}")
