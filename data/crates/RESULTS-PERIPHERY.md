# Periphery grounding — `independent_use_gate` on real crates.io data

> The correctly-specified test the predictive null (`RESULTS.md`) never ran. The predictive test asked
> "does learned v(S) beat the proxy at predicting reuse?" and went NULL three times — expected, because
> honest static labels contain no adversary. This asks the question the vesting gate was actually built
> for (`node/src/lib.rs:7164`): **given a real capital-independence signal, does value-vests-on-
> independent-use separate a self-owned dependency ring from genuine cross-owner reuse, on real data?**
>
> Reproduce: `python periphery_grounding.py`. Source: crates.io dump `2026-07-15-020010`.
> Status discipline: never round up. What is demonstrated is stated as demonstrated; what is still
> open is stated as open, and it is the same open item as before, now sharpened to one word.

## Setup
- `capital_cluster` := crate owner set (`crate_owners.csv`, real recorded ownership; 338,926 owner rows over 299,764 crates).
- Two crates are **independent** iff their owner sets are **disjoint**. Unknown owner ⇒ not independent (conservative, matches the gate).
- A crate `d` **vests** iff some crate `r` that depends on it (`r → d`) has an owner set disjoint from `d`'s. This is the built gate's rule, unchanged, run over 1,940,630 real edges.

## Result — it separates (measured, not asserted)

| quantity | value |
|---|---|
| crates with ≥1 reverse-dep (known owner) | 111,689 |
| **genuine** — ≥1 independent (cross-owner) reuse | **21.2%** |
| **closed** — reuse exists but is 100% same-owner | **78.8%** |
| raw reverse-dep edges | 1,940,612 |
| vested edges (independent-owner only) | 1,641,831 |
| **self-owned inflation the gate strips** | **15.4%** of all reuse |
| mean independent-reuse fraction, genuine crates | 0.79 |
| mean independent-reuse fraction, closed crates | 0.00 |

**Separation, stated as `wash_sim`/`periphery_sim` do:**
- genuine (≥1 cross-owner reuse): vested value/crate = **69.36**
- closed (100% same-owner reuse): vested value/crate = **0.00** — the gate zeroes it.

Real closed-wash structures exist in the wild and the gate correctly zeroes them. Top self-citing owners by intra-owner dependency edges, with their external-use ratio:

| owner | self edges | external edges | self-ratio | gate verdict |
|---|---|---|---|---|
| (user 373951) | 2,875 | **0** | 1.00 | closed ring — vests 0 |
| (user 152437) | 4,563 | 5 | 1.00 | closed ring — vests ~0 |
| (user 73351) | 1,582 | 5 | 1.00 | closed ring — vests ~0 |
| (user 111777) | 2,363 | 55 | 0.98 | near-closed — vests ~0 |
| (user 980) | 2,844 | **7,023** | 0.29 | genuine large project — vests high |
| (user 187432) | 4,223 | 7,878 | 0.35 | genuine large project — vests high |

The gate does exactly the right thing on real structures: a user with 2,875 internal dependency edges and **zero** external use vests nothing, while a multi-crate project with 7,000+ independent downstream users vests high. This is the first time the mechanism has been shown to separate on the real ecosystem rather than a 4-node synthetic fixture (`wash_sim`).

## What this DEMONSTRATES (new)
1. `independent_use_gate`, given a real independence signal, produces a clean non-trivial separation on 111,689 real crates: closed same-owner reuse → 0 vested, genuine cross-owner reuse → high vested. Previously shown only on a synthetic fixture.
2. Real closed-ring structures exist and are correctly zeroed (users 373951, 152437, 73351: self-ratio ~1.0, ~0 external use).
3. The gate measurably strips 15.4% of ecosystem reuse as same-owner self-inflation — a real gaming surface, closed.

## What this does NOT demonstrate (honest scope — do not round up)
1. **The independence signal here is IDENTITY-independence, not CAPITAL-independence.** crates.io ownership is a recorded GitHub identity, which is free to Sybil. A sophisticated wash-builder registers crates under N distinct accounts and cross-cites them; those read as "independent" and would vest. So this proves the *mechanism* separates given an independence signal — and it empirically confirms **exactly why the periphery design anchors independence in capital, not identity** (`DESIGN-periphery-solution.md` §2 Layer A): identity is the Sybilable signal, capital is the costly one. The crates.io run is a faithful real-data proof of Layer A's mechanism **and** a faithful real-data illustration of its dependency on the signal's un-forgeability.
2. **No adaptive adversary.** These are naturally-occurring same-owner clusters, not an attacker optimizing against the gate. The result is "the gate separates the wash that exists," not "the gate survives an adversary who knows the rule."
3. **On-chain, `capital_cluster` remains the unbuilt oracle** (`node/src/lib.rs:7167` takes it as input; the source is 🟡). This experiment supplies the input for free because crates.io records ownership; a permissionless chain does not, and detecting distinct capital origin there is the open problem, forgeable-at-cost.

## Where this moves the ledger
- The vesting-gate / Layer-A mechanism: **fixture-demonstrated → real-data-demonstrated** (given the independence signal). This is genuine movement.
- HCE-4 / wash-discernment: still **open**, but the open part is now precisely one thing — a capital-independence signal that is costly to forge. The mechanism that consumes it is proven on real data; the missing piece is the signal, not the machine.
- The predictive learned-v(S) null is unchanged and unaffected — this is a different, correctly-specified instrument (separation given independence), not prediction on honest labels.

## One line
On real data, the built vesting gate cleanly separates self-owned dependency rings (vest 0) from genuinely reused contributions (vest high), and the naturally-occurring closed rings in crates.io are caught. What it proves is that the machine works given a clean independence signal; what it sharpens is that the whole remaining problem is making that signal cost capital rather than a free identity.
