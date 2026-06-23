#!/usr/bin/env python3
"""
build_labels.py -- ETL from the DeepFunding dependency graph + jury comparisons
into the EXACT on-disk contract `outcome::load_prefs` consumes (node/src/lib.rs).

On-disk format (see node/src/fixtures/outcome_labels_demo.txt):
  - blank / '#...' lines ignored
  - a line of N_FEATS=4 whitespace floats  -> one repo's feature row, FILE-ORDER indexed
  - `pref <winner_idx> <loser_idx>`         -> one pairwise preference into those rows

We restrict to repos present in BOTH the dependency graph AND the comparisons, emit
one feature row per surviving repo (in a deterministic sorted order), then emit a
`pref` line per comparison (winner = the project with the HIGHER jury weight),
indexing by file-order position of the two repos.

------------------------------------------------------------------------------------
FEATURE DEFINITIONS  (4 graph-derived proxies for the Rust DAG semantics; the Rust
features f0..f3 are SET-level over a coalition's provenance-DAG -- here we approximate
their single-repo analogues over the *dependency* graph, which is NOT a provenance DAG,
so these are honest proxies, not the same quantity. Edge = seed DEPENDS ON dependency.)

  f0 breadth     = ln(1 + in_degree)
                   Rust f0 = ln(1+|union coverage|): how much ground the work covers.
                   Proxy: in-degree = # of repos that DEPEND ON this repo (its dependents).
                   A widely-depended-on repo "covers" more of the ecosystem's surface.

  f1 synergy     = (#distinct neighbors) / (1 + in_degree + out_degree)
                   Rust f1 = |union|/Sum|individual| in [0,1]: disjointness / non-redundancy.
                   Proxy: distinct-neighbor fraction of total degree -- a repo whose
                   edges reach many DISTINCT repos (vs. multi-edges to few) has less
                   redundant connectivity. With a simple graph this is ~1, so in practice
                   it leans on the distinct in+out neighbor count normalized by degree.

  f2 connected   = (in_degree + out_degree) / (max_degree in joined set)
                   Rust f2 = fraction of S whose parent is in S: internal provenance /
                   work-built-on-work vs. orphaned garbage.
                   Proxy: a repo's total connectivity, normalized to [0,1]. A repo wired
                   into the dependency fabric (high degree) is "connected"; an isolated
                   node (degree 0) is the orphan analogue.

  f3 depth       = node level (1 or 2 = BFS depth from Ethereum), normalized: (level)/2
                   Rust f3 = longest parent chain in S / |S|: real lineage vs. shallow dump.
                   Proxy: the graph's own provided `level` (depth-from-Ethereum) is the most
                   direct lineage-depth signal available. level 1 = direct dep of a client;
                   level 2 = transitive. Normalized to (0,1].

  None needs an oracle; all are derivable from the shipped graph. The LABELS (jury
  preferences) carry the outside signal, exactly as the Rust module intends.
------------------------------------------------------------------------------------
"""
import json
import csv
import os
import math

HERE = os.path.dirname(os.path.abspath(__file__))
GRAPH_JSON = os.path.join(HERE, "dependency-graph", "graph", "unweighted_graph.json")
DATASET = os.path.join(HERE, "mini-contest", "dataset.csv")
OUT = os.path.join(HERE, "outcome_labels_deepfunding.txt")


def norm(u: str) -> str:
    return u.strip().lower().rstrip("/")


def main():
    g = json.load(open(GRAPH_JSON, encoding="utf-8"))
    # node level lookup
    level = {norm(n["id"]): int(n["level"]) for n in g["nodes"]}
    graph_repos = set(level.keys())

    # degree accounting over the dependency links (source depends on target)
    in_deg = {r: 0 for r in graph_repos}
    out_deg = {r: 0 for r in graph_repos}
    in_neighbors = {r: set() for r in graph_repos}
    out_neighbors = {r: set() for r in graph_repos}
    for e in g["links"]:
        s, t = norm(e["source"]), norm(e["target"])
        if s in graph_repos:
            out_deg[s] += 1
            out_neighbors[s].add(t)
        if t in graph_repos:
            in_deg[t] += 1
            in_neighbors[t].add(s)

    # repos referenced in the comparisons
    dataset_repos = set()
    comparisons = []  # (winner_url, loser_url)
    n_rows = 0
    with open(DATASET, encoding="utf-8") as f:
        for r in csv.DictReader(f):
            n_rows += 1
            a, b = norm(r["project_a"]), norm(r["project_b"])
            dataset_repos.add(a)
            dataset_repos.add(b)
            wa, wb = float(r["weight_a"]), float(r["weight_b"])
            winner, loser = (a, b) if wa >= wb else (b, a)
            comparisons.append((winner, loser))

    joined = sorted(graph_repos & dataset_repos)  # deterministic file order
    idx = {u: i for i, u in enumerate(joined)}

    # max degree over the JOINED set (for f2 normalization)
    max_deg = max((in_deg[r] + out_deg[r] for r in joined), default=1) or 1

    # build feature rows
    rows = []
    for u in joined:
        ind = in_deg[u]
        outd = out_deg[u]
        deg = ind + outd
        distinct = len(in_neighbors[u] | out_neighbors[u])
        f0 = math.log1p(ind)
        f1 = (distinct / (1.0 + deg)) if deg > 0 else 0.0
        f2 = deg / max_deg
        f3 = level[u] / 2.0
        rows.append((f0, f1, f2, f3))

    # surviving preferences (both repos joined)
    prefs = []
    dropped = 0
    for w, l in comparisons:
        if w in idx and l in idx:
            prefs.append((idx[w], idx[l]))
        else:
            dropped += 1

    # write the load_prefs-format file
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("# DeepFunding outcome labels -- emitted by build_labels.py\n")
        f.write("# feats: [breadth, synergy, connected, depth] (graph-derived proxies)\n")
        f.write("# %d repo feature rows (file-order indexed), then %d pref lines\n"
                % (len(rows), len(prefs)))
        for (f0, f1, f2, f3) in rows:
            f.write("%.6f %.6f %.6f %.6f\n" % (f0, f1, f2, f3))
        f.write("# pref <winner_idx> <loser_idx>  (winner = higher jury weight)\n")
        for (w, l) in prefs:
            f.write("pref %d %d\n" % (w, l))

    # honest coverage report
    refs = 0
    missing_refs = 0
    for w, l in comparisons:
        for u in (w, l):
            refs += 1
            if u not in graph_repos:
                missing_refs += 1
    print("=== build_labels.py coverage ===")
    print("dataset comparison rows         :", n_rows)
    print("distinct repos in dataset       :", len(dataset_repos))
    print("distinct repos in graph         :", len(graph_repos))
    print("repos in BOTH (survived join)   :", len(joined))
    print("dataset repos NOT in graph      :", len(dataset_repos - graph_repos),
          sorted(dataset_repos - graph_repos))
    print("comparison rows kept (prefs)    :", len(prefs))
    print("comparison rows dropped (>=1 missing repo):", dropped)
    print("total project refs              :", refs, "| refs to missing repos:", missing_refs)
    print("wrote:", OUT)


if __name__ == "__main__":
    main()
