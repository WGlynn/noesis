# B1 Benchmark — TraceRank/PageRank flat propagation vs HodgeRank harmonic residual

**Claim proven:** A mutual-endorsement collusion **ring** (pure cyclic circulation of
value among `k` nodes) is **rewarded** by flat value-weighted propagation
(PageRank / TraceRank family) but **isolated** by the HodgeRank harmonic+curl residual.

Pure `numpy`. No `networkx`. Deterministic (`seed=7`).

```bash
python b1_benchmark.py     # prints the table below, also writes _run_output.txt
```

## Why it works (mechanism)

A *consistent* contribution graph is a **gradient flow**: an honest endorsement edge
`i -> j` carries flow ≈ `q_j - q_i`, the difference of latent node qualities. HodgeRank
fits a single node potential `s` by weighted least squares on the signed incidence
matrix `B`:

```
min_s  Σ_e w_e ( (s_j - s_i) - r_e )^2
```

The fitted gradient is `g_e = s_j - s_i`; the **residual** `h_e = r_e - g_e` is the
harmonic+curl part — flow that **no node potential can explain**.

- **Honest edges** are (noisy) gradients → they fit a potential → residual ≈ 0.
- **A pure cycle** (`40→41→42→43→44→40`, each edge ≈ +8) has *zero net gradient*: the
  least-squares fit assigns it ≈ 0, so the **entire circulation survives as residual**,
  concentrated on exactly the ring's nodes.

PageRank, by contrast, lets the cyclic mass recirculate among the ring and pumps rank
back to its members — collusion looks like merit.

## Scenario

- 40 honest contributors + a 5-node injected collusion ring = 45 nodes, 99 edges.
- Honest layer: an acyclic DAG where lower-quality nodes endorse higher-quality ones,
  flow ≈ quality difference + tiny noise (a clean gradient field).
- Ring: a closed 5-cycle of strong mutual endorsements (flow ≈ 8.0) — pure circulation.
- 6 sparse honest→ring edges so the ring is not trivially disconnected.

## Headline results (`seed=7`, actual run)

| metric | ring mean | honest mean | ratio |
|---|---|---|---|
| **PageRank** | 0.06221 | 0.01722 | **3.6×** (ring rewarded) |
| **Hodge residual energy** | 129.41744 | 0.34496 | **375.2×** (ring flagged) |

- **PageRank ranks the ring HIGH** — mean rank **5.0 / 45** (honest avg 25.2). Five of the
  top-7 PageRank nodes are ring members. Flat propagation *rewards* the collusion.
- **Hodge residual ranks the ring HIGH** — mean rank **3.0 / 45**; the ring occupies the
  **top 5** residual slots. Honest residual mean **0.345** (≈ 0, gradient-consistent).
- **Clean separation:** max honest residual **4.93** vs min ring residual **112.41** —
  a **23× order-of-magnitude gap**.

### Top 10 by PageRank (ring is rewarded)

```
 node    type   pagerank  pr_rank  hodge_resid  res_rank
   19  honest    0.10409        1       0.0528        23
   16  honest    0.08494        2       0.0495        25
   44    RING    0.06599        3     135.3241         3
   40    RING    0.06382        4     144.3041         1
   41    RING    0.06115        5     137.0389         2
   43    RING    0.06048        6     112.4093         5
   42    RING    0.05962        7     118.0107         4
    1  honest    0.04666        8       0.0523        24
   27  honest    0.04498        9       0.0450        28
    5  honest    0.03843       10       0.1710        18
```

### Top 10 by Hodge residual energy (ring is flagged)

```
 node    type  hodge_resid  res_rank   pagerank  pr_rank
   40    RING     144.3041         1    0.06382        4
   41    RING     137.0389         2    0.06115        5
   44    RING     135.3241         3    0.06599        3
   42    RING     118.0107         4    0.05962        7
   43    RING     112.4093         5    0.06048        6
   18  honest       4.9325         6    0.01304       16
   23  honest       2.8569         7    0.00690       40
   10  honest       1.8196         8    0.00841       30
   30  honest       0.7739         9    0.01253       17
   22  honest       0.5236        10    0.01798       13
```

## Slash thresholds (precision / recall)

**5-sigma rule** — flag if Hodge residual > `honest_mean + 5·honest_std` (= 4.86):

```
flagged: [18, 40, 41, 42, 43, 44]
TP=5  FP=1  FN=0   ->  PRECISION = 0.833   RECALL = 1.000
```

One honest false positive (node 18, residual 4.93) sits just above the conservative
5σ line. It is reported honestly rather than tuned away.

**Largest-gap rule** (unsupervised, no labels) — cut at the biggest gap in sorted
residuals (= 58.67, between honest 4.93 and ring 112.41):

```
flagged: [40, 41, 42, 43, 44]
PRECISION = 1.000   RECALL = 1.000   (clean order-of-magnitude separation)
```

**Same 5σ slash rule applied to PageRank (control):**

```
threshold = 0.11979   flagged: []   ring caught = 0
```

Flat propagation flags **zero** ring members as cheaters — it rates them as *top
performers*. PageRank cannot be turned into a collusion slash signal; HodgeRank can.

## Verdict

> The collusion ring is simultaneously **#3–#7 in PageRank** (rewarded by flat value
> propagation) and **#1–#5 in Hodge residual** (flagged by the harmonic decomposition),
> while honest nodes sit at ≈ 0 residual. A residual-energy slash rule isolates the ring
> with **recall 1.000** and **precision 0.833 (5σ) → 1.000 (largest-gap)**. The
> harmonic residual is the part of the endorsement field that mutual back-scratching
> cannot fake.

## Files

- `b1_benchmark.py` — self-contained experiment (numpy only, seeded).
- `_run_output.txt` — captured stdout of the run.
- `README.md` — this file.
