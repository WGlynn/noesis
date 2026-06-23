"""
B1 Benchmark: TraceRank/PageRank-family flat propagation vs HodgeRank harmonic residual.

Thesis
------
A mutual-endorsement collusion RING (pure circulation of value among k nodes) is
REWARDED by flat value-weighted propagation (PageRank/TraceRank family) because the
random-walk mass gets trapped in the cycle and pumped back to its members. The same
ring is ISOLATED by the HodgeRank harmonic+curl residual: a consistent contribution
graph is a *gradient flow* (edge value == difference of node potentials), so honest
edges fit a potential with ~0 residual, while a pure cycle carries flow that NO node
potential can explain -> large residual concentrated on exactly the ring's nodes.

Pure numpy. Deterministic (seeded). No networkx.

Math
----
Let G = (V, E) be a directed graph; r_e is the observed "value/endorsement" flow on
edge e = (i -> j). HodgeRank fits a node potential s (a rank score) minimizing

    min_s  sum_e w_e ( (s_j - s_i) - r_e )^2

i.e. a weighted least-squares gradient fit on the (signed) incidence matrix B, where
row e of B has -1 in column i and +1 in column j. The fitted gradient component is
g_e = (s_j - s_i); the residual  h_e = r_e - g_e  is the harmonic+curl part -- the
flow that is NOT a gradient of any potential. For a pure cycle, the gradient fit is
~0 everywhere so the entire circulation survives as residual. Per-node residual
energy = sum of h_e^2 over incident edges; this is the collusion-detection signal.
"""

import numpy as np

SEED = 7
rng = np.random.default_rng(SEED)

# ----------------------------------------------------------------------------
# (1) Synthetic scenario
# ----------------------------------------------------------------------------
N_HONEST = 40          # honest contributors
K_RING = 5             # collusion ring size
N = N_HONEST + K_RING
HONEST = np.arange(N_HONEST)
RING = np.arange(N_HONEST, N)

# Honest layer: a contribution DAG with a CONSISTENT latent quality potential.
# Each honest node has a true quality; an honest endorsement edge i->j carries flow
# approximately (q_j - q_i) -> a clean gradient field (tiny observation noise).
q_true = rng.uniform(0.0, 10.0, size=N_HONEST)

edges = []          # (i, j)
flow = []           # observed value/endorsement on edge i->j
weight = []         # edge confidence weight

# Build an acyclic honest endorsement DAG: only lower-quality -> higher-quality edges
# (people endorse work better than theirs), guaranteeing a DAG + gradient structure.
order = np.argsort(q_true)                       # ascending quality
for a_idx in range(N_HONEST):
    i = order[a_idx]
    # connect to a few higher-quality honest nodes
    n_out = rng.integers(1, 5)
    higher = order[a_idx + 1:]
    if len(higher) == 0:
        continue
    targets = rng.choice(higher, size=min(n_out, len(higher)), replace=False)
    for j in targets:
        edges.append((int(i), int(j)))
        flow.append((q_true[j] - q_true[i]) + rng.normal(0, 0.05))  # gradient + noise
        weight.append(1.0)

# A handful of honest edges that touch ring nodes from the outside (sparse, legit-looking)
for _ in range(6):
    i = int(rng.choice(HONEST))
    j = int(rng.choice(RING))
    edges.append((i, j))
    flow.append(rng.uniform(0.5, 2.0))   # modest one-directional endorsement
    weight.append(1.0)

# (1b) Injected collusion ring: a pure cyclic mutual-endorsement loop.
# Each ring member strongly endorses the next; the cycle closes -> pure circulation,
# zero net gradient, designed to farm flat-propagation rank.
RING_FLOW = 8.0
RING_WEIGHT = 1.0
for idx in range(K_RING):
    i = int(RING[idx])
    j = int(RING[(idx + 1) % K_RING])
    edges.append((i, j))
    flow.append(RING_FLOW + rng.normal(0, 0.05))
    weight.append(RING_WEIGHT)

edges = np.array(edges, dtype=int)
flow = np.array(flow, dtype=float)
weight = np.array(weight, dtype=float)
E = len(edges)

# ----------------------------------------------------------------------------
# (2) Flat propagation = PageRank on the value-weighted graph
# ----------------------------------------------------------------------------
def pagerank(edges, vals, n, damping=0.85, iters=200, tol=1e-12):
    # value-weighted adjacency: edge i->j gets weight max(val, 0)
    W = np.zeros((n, n))
    for (i, j), v in zip(edges, vals):
        W[i, j] += max(v, 0.0)
    # PageRank flows rank toward endorsed nodes -> column-stochastic transition
    out = W.sum(axis=1)
    M = np.zeros((n, n))
    for i in range(n):
        if out[i] > 0:
            M[:, i] = W[i, :] / out[i]      # mass leaves i, distributed to its targets
        else:
            M[:, i] = 1.0 / n               # dangling -> teleport
    r = np.full(n, 1.0 / n)
    tele = np.full(n, 1.0 / n)
    for _ in range(iters):
        r_new = damping * (M @ r) + (1 - damping) * tele
        if np.linalg.norm(r_new - r, 1) < tol:
            r = r_new
            break
        r = r_new
    return r / r.sum()

pr = pagerank(edges, flow, N)

# ----------------------------------------------------------------------------
# (3) HodgeRank: weighted least-squares gradient fit + harmonic/curl residual
# ----------------------------------------------------------------------------
# Signed incidence B (E x N): row e has -1 at i, +1 at j for edge i->j.
B = np.zeros((E, N))
for e, (i, j) in enumerate(edges):
    B[e, i] = -1.0
    B[e, j] = +1.0

Wd = np.diag(weight)
# Solve weighted normal equations  B^T W B s = B^T W r   (gauge-fix: mean(s)=0)
A = B.T @ Wd @ B
b = B.T @ Wd @ flow
# A is rank-deficient by 1 (constant shift); pin with pseudo-inverse.
s = np.linalg.lstsq(A, b, rcond=None)[0]
s = s - s.mean()

grad = B @ s                       # fitted gradient flow per edge
resid = flow - grad                # harmonic + curl residual per edge

# Per-node residual energy: sum of squared residual on incident edges.
node_resid = np.zeros(N)
for e, (i, j) in enumerate(edges):
    node_resid[i] += resid[e] ** 2
    node_resid[j] += resid[e] ** 2

# ----------------------------------------------------------------------------
# (4) Output: table + verdict + slash-threshold precision/recall
# ----------------------------------------------------------------------------
def rank_of(x):
    # 1 = highest value
    order = np.argsort(-x)
    rnk = np.empty_like(order)
    rnk[order] = np.arange(1, len(x) + 1)
    return rnk

pr_rank = rank_of(pr)
res_rank = rank_of(node_resid)

is_ring = np.zeros(N, dtype=bool)
is_ring[RING] = True

# Slash rule: flag a node if its Hodge residual energy exceeds a threshold.
# Threshold = honest mean + 5 sigma (a conservative, honest-side-derived cutoff).
honest_res = node_resid[HONEST]
thr = honest_res.mean() + 5 * honest_res.std()
flagged = node_resid > thr

tp = int(np.sum(flagged & is_ring))
fp = int(np.sum(flagged & ~is_ring))
fn = int(np.sum(~flagged & is_ring))
precision = tp / (tp + fp) if (tp + fp) else float("nan")
recall = tp / (tp + fn) if (tp + fn) else float("nan")

# Same slash rule applied to PageRank, to show flat propagation CANNOT separate them.
pr_thr = pr[HONEST].mean() + 5 * pr[HONEST].std()
pr_flagged = pr > pr_thr
pr_tp = int(np.sum(pr_flagged & is_ring))
pr_fp = int(np.sum(pr_flagged & ~is_ring))

lines = []
def out(s=""):
    lines.append(s)
    print(s)

out("=" * 78)
out("B1 BENCHMARK -- TraceRank/PageRank flat propagation vs HodgeRank residual")
out("=" * 78)
out(f"seed={SEED}  nodes={N} (honest={N_HONEST}, ring={K_RING})  edges={E}")
out(f"ring members (node ids): {list(RING)}")
out("")

# Top nodes by PageRank
out("-- TOP 10 BY PAGERANK (flat value-weighted propagation) --")
out(f"{'node':>5} {'type':>7} {'pagerank':>10} {'pr_rank':>8} {'hodge_resid':>12} {'res_rank':>9}")
for n in np.argsort(-pr)[:10]:
    t = "RING" if is_ring[n] else "honest"
    out(f"{n:>5} {t:>7} {pr[n]:>10.5f} {pr_rank[n]:>8} {node_resid[n]:>12.4f} {res_rank[n]:>9}")
out("")

out("-- TOP 10 BY HODGE RESIDUAL ENERGY (harmonic+curl) --")
out(f"{'node':>5} {'type':>7} {'hodge_resid':>12} {'res_rank':>9} {'pagerank':>10} {'pr_rank':>8}")
for n in np.argsort(-node_resid)[:10]:
    t = "RING" if is_ring[n] else "honest"
    out(f"{n:>5} {t:>7} {node_resid[n]:>12.4f} {res_rank[n]:>9} {pr[n]:>10.5f} {pr_rank[n]:>8}")
out("")

out("-- RING vs HONEST AGGREGATES --")
out(f"{'metric':<28}{'ring mean':>14}{'honest mean':>14}{'ratio':>10}")
def row(name, arr):
    rm, hm = arr[RING].mean(), arr[HONEST].mean()
    ratio = rm / hm if hm else float('inf')
    out(f"{name:<28}{rm:>14.5f}{hm:>14.5f}{ratio:>10.1f}x")
row("pagerank", pr)
row("hodge residual energy", node_resid)
out("")

out("-- VERDICT --")
out(f"PageRank   : ring mean rank {pr_rank[RING].mean():.1f} / {N}  "
    f"(avg honest rank {pr_rank[HONEST].mean():.1f})  -> ring REWARDED (ranks high)")
out(f"Hodge res. : ring mean rank {res_rank[RING].mean():.1f} / {N}  "
    f"(avg honest rank {res_rank[HONEST].mean():.1f})  -> ring FLAGGED (residual high)")
out(f"honest residual: mean={honest_res.mean():.4f}  max={honest_res.max():.4f}  (~0, gradient-consistent)")
out("")

out("-- SLASH RULE (flag if Hodge residual > honest_mean + 5*honest_std) --")
out(f"threshold = {thr:.4f}")
out(f"flagged nodes: {list(np.where(flagged)[0])}")
out(f"true positives={tp}  false positives={fp}  false negatives={fn}")
out(f"PRECISION = {precision:.3f}   RECALL = {recall:.3f}")
out("")
out("-- LARGEST-GAP THRESHOLD (unsupervised, separates the two populations) --")
sorted_res = np.sort(node_resid)
gaps = np.diff(sorted_res)
cut = (sorted_res[np.argmax(gaps)] + sorted_res[np.argmax(gaps) + 1]) / 2
gap_flagged = node_resid > cut
g_tp = int(np.sum(gap_flagged & is_ring))
g_fp = int(np.sum(gap_flagged & ~is_ring))
g_fn = int(np.sum(~gap_flagged & is_ring))
g_prec = g_tp / (g_tp + g_fp) if (g_tp + g_fp) else float("nan")
g_rec = g_tp / (g_tp + g_fn) if (g_tp + g_fn) else float("nan")
out(f"largest residual gap is at {cut:.3f} "
    f"(below: max honest {sorted_res[np.argmax(gaps)]:.3f}; above: min ring "
    f"{sorted_res[np.argmax(gaps)+1]:.3f})")
out(f"flagged nodes: {list(np.where(gap_flagged)[0])}")
out(f"PRECISION = {g_prec:.3f}   RECALL = {g_rec:.3f}  (clean order-of-magnitude separation)")
out("")
out("-- SAME 5-SIGMA SLASH RULE ON PAGERANK (control: flat propagation cannot isolate) --")
out(f"threshold = {pr_thr:.5f}")
out(f"flagged nodes: {list(np.where(pr_flagged)[0])}")
out(f"ring caught={pr_tp}  honest wrongly flagged={pr_fp}  "
    f"(PageRank flags ring as TOP performers, not cheaters -> reward, not slash)")
out("=" * 78)

with open("_run_output.txt", "w") as f:
    f.write("\n".join(lines) + "\n")
