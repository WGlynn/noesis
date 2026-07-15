#!/usr/bin/env python3
"""
build_dataset.py -- the DEEP-ANCESTRY, outcome-labelled dataset (Phase 1 of the learned-v(S) moat).

WHY THIS EXISTS (reconcile before building -- STATUS-LEDGER MOAT-1 + MVP-SCOPE R1):
The DeepFunding real-data test went NULL twice, and the faithful port (data/deepfunding/
RESULTS-FAITHFUL.md) diagnosed the cause precisely: a TOPOLOGY block, not a label block --
95/115 judged repos are LEAVES, so the SHIPPED ancestor coalition ({r} U provenance-ancestors,
lib.rs:7092 coalition_features) collapses to a singleton and f1/f2/f3 become constants. The one
named-open obligation MOAT-1 leaves is verbatim: "a deep-ancestry outcome-labelled dataset ...
commit/PR-level lineage, not repo-level leaf deps, so the shipped ancestor object is non-degenerate."

The crates.io dependency graph IS that dataset: every crate has a REAL, DEEP transitive-dependency
chain (ancestry that is non-degenerate by construction), and downstream REUSE (reverse-dependency
count) + downloads is a real, external, Sybil-uncorrelated OUTCOME label. A merged/published crate
that many others depend on is a maintainer-and-ecosystem authoritative value signal -- the exact
"borrow authority off-chain as a training seed" unlock in RESEARCH-learned-vs-from-merged-prs.md.

WHAT THIS EMITS (data/crates/graph/):
  nodes.tsv     crate_id \t name \t downloads \t created_ts
  edges.tsv     src_crate_id \t dst_crate_id     (src depends-on dst; dst is a provenance ANCESTOR)
  stats.json    topology diagnosis: ancestor-closure size distribution (the anti-degeneracy proof)

Edge semantics (mirrors the faithful port's provenance-DAG reading):
  r -> d  means r depends on d  ==  d is prior work r built upon  ==  d is a provenance ANCESTOR of r.
  "out(r)" = r's dependencies (its ANCESTORS).  "in(d)" = d's reverse-deps (its DESCENDANTS / REUSE).

Only NORMAL (kind=0), non-optional, non-target-gated dependencies are kept (build/dev deps and
platform-gated deps are not provenance the way a runtime dependency is). Edges are crate-level
(collapsed across versions) and de-duplicated. Self-edges dropped.
"""
import tarfile, csv, io, json, os, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
DUMP = os.path.join(HERE, "db-dump.tar.gz")
OUT = os.path.join(HERE, "graph")
os.makedirs(OUT, exist_ok=True)
csv.field_size_limit(1 << 30)


def open_csv(tar, suffix):
    """Return a csv.DictReader over the single archive member whose path ends with `suffix`."""
    member = next(m for m in tar.getmembers() if m.name.endswith(suffix))
    f = tar.extractfile(member)
    return csv.DictReader(io.TextIOWrapper(f, encoding="utf-8", newline=""))


def main():
    t0 = time.time()
    print(f"[{time.time()-t0:6.1f}s] opening {DUMP} ({os.path.getsize(DUMP)/1e6:.0f} MB)")
    tar = tarfile.open(DUMP, "r:gz")

    # --- crates.csv: id -> name, created_at ---
    crate_name, crate_created = {}, {}
    for i, r in enumerate(open_csv(tar, "/data/crates.csv")):
        cid = int(r["id"])
        crate_name[cid] = r["name"]
        crate_created[cid] = r.get("created_at", "")
    print(f"[{time.time()-t0:6.1f}s] crates: {len(crate_name):,}")

    # re-open per table (tarfile members are one-shot streams)
    tar.close(); tar = tarfile.open(DUMP, "r:gz")
    # --- crate_downloads.csv: crate_id -> downloads (outcome signal #2) ---
    downloads = {}
    try:
        for r in open_csv(tar, "/data/crate_downloads.csv"):
            downloads[int(r["crate_id"])] = int(r["downloads"])
    except StopIteration:
        pass
    print(f"[{time.time()-t0:6.1f}s] downloads rows: {len(downloads):,}")

    # --- versions.csv: version_id -> source crate_id ---
    tar.close(); tar = tarfile.open(DUMP, "r:gz")
    ver_crate = {}
    for r in open_csv(tar, "/data/versions.csv"):
        ver_crate[int(r["id"])] = int(r["crate_id"])
    print(f"[{time.time()-t0:6.1f}s] versions: {len(ver_crate):,}")

    # --- dependencies.csv: edges src_crate -> dst_crate (normal, non-optional, untargeted) ---
    tar.close(); tar = tarfile.open(DUMP, "r:gz")
    edges = set()
    kept = dropped = 0
    for r in open_csv(tar, "/data/dependencies.csv"):
        if r.get("kind", "0") != "0":            # 0=normal 1=build 2=dev
            dropped += 1; continue
        if r.get("optional", "f") in ("t", "true", "1"):
            dropped += 1; continue
        if r.get("target"):                      # platform-gated -> not universal provenance
            dropped += 1; continue
        vid = int(r["version_id"]); dst = int(r["crate_id"])   # crate_id here = the depended-upon crate
        src = ver_crate.get(vid)
        if src is None or src == dst:
            dropped += 1; continue
        edges.add((src, dst)); kept += 1
    tar.close()
    print(f"[{time.time()-t0:6.1f}s] dep rows kept {kept:,} dropped {dropped:,}; unique crate-edges {len(edges):,}")

    # nodes that actually participate (have a name); keep all crates as potential nodes
    nodes = set(crate_name)

    # write nodes + edges
    with open(os.path.join(OUT, "nodes.tsv"), "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, delimiter="\t")
        for cid in sorted(nodes):
            w.writerow([cid, crate_name[cid], downloads.get(cid, 0), crate_created.get(cid, "")])
    with open(os.path.join(OUT, "edges.tsv"), "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, delimiter="\t")
        for s, d in sorted(edges):
            w.writerow([s, d])

    # --- topology diagnosis: transitive ANCESTOR-closure sizes (the anti-degeneracy receipt) ---
    out_adj = {}
    for s, d in edges:
        out_adj.setdefault(s, set()).add(d)

    def anc_closure_size(r, cap=100000):
        seen, stk = set(), [r]
        while stk:
            x = stk.pop()
            if x in seen:
                continue
            seen.add(x)
            if len(seen) > cap:
                break
            stk.extend(out_adj.get(x, ()))
        return len(seen)  # includes r

    # sample for the distribution (full closure over 150k nodes is heavy; a uniform sample is honest)
    import random
    rng = random.Random(0)
    sample = rng.sample(sorted(nodes), min(4000, len(nodes)))
    sizes = sorted(anc_closure_size(r) for r in sample)
    singletons = sum(1 for s in sizes if s == 1)
    n = len(sizes)
    revdeg = {}
    for s, d in edges:
        revdeg[d] = revdeg.get(d, 0) + 1
    reused = sum(1 for d in nodes if revdeg.get(d, 0) > 0)

    stats = {
        "dump": os.path.basename(next(iter([m.name for m in tarfile.open(DUMP,'r:gz').getmembers()[:1]]))),
        "crates_total": len(nodes),
        "edges_normal_crate_level": len(edges),
        "crates_with_ge1_reverse_dep": reused,
        "ancestor_closure_sample_n": n,
        "ancestor_closure_singletons": singletons,
        "ancestor_closure_singleton_frac": round(singletons / n, 4),
        "ancestor_closure_median": sizes[n // 2],
        "ancestor_closure_p90": sizes[int(n * 0.9)],
        "ancestor_closure_max": sizes[-1],
        "COMPARE_deepfunding_singleton_frac": round(95 / 115, 4),
    }
    with open(os.path.join(OUT, "stats.json"), "w") as f:
        json.dump(stats, f, indent=2)
    print(f"[{time.time()-t0:6.1f}s] DONE. topology diagnosis:")
    print(json.dumps(stats, indent=2))


if __name__ == "__main__":
    main()
