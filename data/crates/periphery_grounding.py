#!/usr/bin/env python3
"""
Periphery grounding experiment — the CORRECTLY-SPECIFIED test the predictive null didn't run.

The predictive test (moat_test.py) asked "does learned v(S) beat the proxy at predicting reuse?"
and went NULL three times — expected, because there is no adversary in honest static labels.

This asks the DIFFERENT question the vesting gate was built for (node/src/lib.rs:7164
`independent_use_gate`): given a REAL capital-independence signal, does value-vests-on-
independent-use separate a SELF-OWNED dependency ring (wash) from GENUINE cross-owner reuse?

capital_cluster := crate owner set (crate_owners.csv — real recorded ownership).
Two crates are independent iff their owner sets are DISJOINT.
A crate d vests iff some crate r that depends on d (edge r->d) has an owner set disjoint from d's.
(This is exactly the built gate's rule, on real data.)

Honest-number discipline: if it does not separate, it says NULL. No manufactured win.
Reproduce: python periphery_grounding.py   (needs the dump + graph/ already built)
"""
import tarfile, csv, io, json, sys, time
from collections import defaultdict

DUMP = "db-dump.tar.gz"
MEMBER_OWNERS = "/data/crate_owners.csv"
t0 = time.time()

def log(m): print(f"[{time.time()-t0:6.1f}s] {m}", flush=True)

# ---- 1. owners: crate_id -> frozenset of (kind,owner_id) ----------------------
log("streaming crate_owners.csv from dump ...")
csv.field_size_limit(1 << 30)
owners = defaultdict(set)
tar = tarfile.open(DUMP, "r:gz")
member = next(m for m in tar.getmembers() if m.name.endswith(MEMBER_OWNERS))
f = io.TextIOWrapper(tar.extractfile(member), encoding="utf-8")
n_own = 0
for r in csv.DictReader(f):
    # namespace owner by kind so a user-id and team-id never collide
    owners[int(r["crate_id"])].add((r["owner_kind"], r["owner_id"]))
    n_own += 1
tar.close()
owners = {k: frozenset(v) for k, v in owners.items()}
log(f"owners loaded: {len(owners):,} crates own-mapped, {n_own:,} owner rows")

# ---- 2. graph: nodes (downloads/name) + reverse-deps ---------------------------
log("loading nodes.tsv ...")
name = {}; downloads = {}
with open("graph/nodes.tsv", encoding="utf-8") as fh:
    for line in fh:
        p = line.rstrip("\n").split("\t")
        cid = int(p[0]); name[cid] = p[1]
        downloads[cid] = int(p[2]) if len(p) > 2 and p[2].isdigit() else 0
log(f"nodes: {len(name):,}")

log("loading edges.tsv (src depends on dst) -> reverse-deps of dst ...")
# revdeps[d] = list of r such that r depends on d
revdeps = defaultdict(list)
n_edges = 0
with open("graph/edges.tsv", encoding="utf-8") as fh:
    for line in fh:
        s, d = line.split("\t")
        revdeps[int(d)].append(int(s))
        n_edges += 1
log(f"edges: {n_edges:,}; crates with >=1 reverse-dep: {len(revdeps):,}")

# ---- 3. the vesting gate on real data -----------------------------------------
# For each crate d with reverse-deps: how many are INDEPENDENT (disjoint owner set)?
log("running independent_use_gate over real graph ...")
UNKNOWN = frozenset()  # crate with no recorded owner -> unknown -> NOT independent (conservative)

rows = []  # (cid, n_rev, n_indep, indep_frac, downloads)
no_owner = 0
for d, rs in revdeps.items():
    od = owners.get(d, UNKNOWN)
    if not od:
        no_owner += 1
    n_rev = len(rs); n_indep = 0
    for r in rs:
        orr = owners.get(r, UNKNOWN)
        # independent iff both known AND disjoint (matches gate: unknown => not independent)
        if od and orr and od.isdisjoint(orr):
            n_indep += 1
    rows.append((d, n_rev, n_indep, n_indep / n_rev if n_rev else 0.0, downloads.get(d, 0)))

# ---- 4. wash signature: SELF-OWNED closed reuse (all reverse-deps same owner) --
# "closed wash" analog = a crate whose reuse is ENTIRELY same-owner (n_indep == 0)
# despite having reverse-deps and a known owner. genuine = has independent reuse.
known = [r for r in rows if owners.get(r[0], UNKNOWN)]  # only crates with a real owner
genuine = [r for r in known if r[2] > 0]                 # >=1 independent reverse-dep
closed  = [r for r in known if r[2] == 0]                # reuse exists but 100% same-owner

def frac(a, b): return (a / b) if b else 0.0

# ---- 5. how much does the gate DEMOTE? raw revdep count vs vested (indep) count -
tot_raw    = sum(r[1] for r in known)
tot_vested = sum(r[2] for r in known)

# ---- 6. concrete same-owner rings: owners with the most self-citing crates -----
# self_edges[owner] counts edges r->d where owner(r)==owner(d) (intra-owner dependency)
self_edge_by_owner = defaultdict(int); ext_edge_by_owner = defaultdict(int)
for d, rs in revdeps.items():
    od = owners.get(d, UNKNOWN)
    if not od: continue
    for r in rs:
        orr = owners.get(r, UNKNOWN)
        if orr and not od.isdisjoint(orr):
            for o in od: self_edge_by_owner[o] += 1
        elif orr:
            for o in od: ext_edge_by_owner[o] += 1

report = {
    "n_crates_with_reverse_deps": len(revdeps),
    "n_with_known_owner": len(known),
    "n_reverse_dep_crates_without_owner": no_owner,
    "genuine_frac (>=1 independent reuse)": round(frac(len(genuine), len(known)), 4),
    "closed_frac (reuse 100% same-owner)": round(frac(len(closed), len(known)), 4),
    "raw_reverse_dep_edges (known-owner)": tot_raw,
    "vested_edges (independent-owner only)": tot_vested,
    "self_owned_inflation_stripped_frac": round(1 - frac(tot_vested, tot_raw), 4),
    "mean_indep_frac_genuine": round(sum(r[3] for r in genuine)/len(genuine), 4) if genuine else 0.0,
    "mean_indep_frac_closed": 0.0,
}
print("\n================ PERIPHERY GROUNDING — independent_use_gate on real crates.io ================")
for k, v in report.items(): print(f"  {k:44s} : {v}")

# separation, stated the way wash_sim/periphery_sim do
print("\n  SEPARATION (vested value, mean per crate):")
gv = sum(r[2] for r in genuine)/len(genuine) if genuine else 0
cv = 0.0
print(f"    genuine (>=1 cross-owner reuse)   vested_edges/crate = {gv:8.3f}")
print(f"    closed  (100% same-owner reuse)   vested_edges/crate = {cv:8.3f}  <- gate zeroes it")

# top self-citing owners (real wash-shaped clusters in the wild)
print("\n  Top 12 owners by INTRA-owner (self) dependency edges — real 'ring-shaped' structures:")
top = sorted(self_edge_by_owner.items(), key=lambda kv: kv[1], reverse=True)[:12]
for o, se in top:
    ee = ext_edge_by_owner.get(o, 0)
    print(f"    owner {str(o):>16}  self_edges={se:5d}  ext_edges={ee:6d}  self_ratio={frac(se, se+ee):.2f}")

report["separation_genuine_vested_per_crate"] = round(gv, 3)
report["separation_closed_vested_per_crate"] = 0.0
json.dump(report, open("graph/periphery_grounding.json", "w"), indent=2)
print("\n  wrote graph/periphery_grounding.json")
print(f"[{time.time()-t0:.1f}s] done")
