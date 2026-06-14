#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Adversary against v(S)/PoM — the moat test (AFK item 1).

PRIVATE. The whole system's honesty rests on un-gameable measurement. This is a
standing adversary that tries cheap PoM inflation and REPORTS whether the synergy
value mechanism resists it. Build-don't-claim: we test sybil-resistance, we don't
assert it. Attacks:

  sybil-split : clone one block into K identical-content copies on K accounts.
                Resistant iff the K copies SHARE the original's value (synergy
                discounts duplicates) instead of multiplying it.
  padding     : add a block whose coverage is a SUBSET of an existing block.
                Resistant iff it earns ~0 marginal value.

coverage = [proxy] for the learned reward model. Myerson via sampling. Stdlib.

  run [N] [K] [T]
"""
import glob
import json
import os
import random
import sys

try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

HOME = os.path.expanduser("~")
BLOCKS = os.path.join(HOME, ".claude", "session-chain", "blocks")


def _cov_of_real(bid):
    p = os.path.join(BLOCKS, bid + ".json")
    t = str(json.load(open(p, encoding="utf-8")).get("response", "")).lower()
    return {t[i:i + 5] for i in range(0, max(0, len(t) - 4))} or {bid}


def vS(S, cov):
    u = set()
    for b in S:
        u |= cov[b]
    return len(u)


def myerson(items, cov, T):
    """No DAG here (the attack set is flat) -> plain Shapley of the coverage game."""
    phi = {b: 0.0 for b in items}
    for t in range(T):
        rnd = random.Random(7000 + t)
        perm = items[:]
        rnd.shuffle(perm)
        run, prev = set(), 0.0
        for b in perm:
            run.add(b)
            cur = vS(run, cov)
            phi[b] += cur - prev
            prev = cur
    return {b: phi[b] / T for b in items}


def main():
    a = sys.argv[1:]
    N = int(a[1]) if len(a) > 1 else 6
    K = int(a[2]) if len(a) > 2 else 5
    T = int(a[3]) if len(a) > 3 else 3000
    blocks = [os.path.basename(f).replace(".json", "")
              for f in sorted(glob.glob(os.path.join(BLOCKS, "block-*.json")))[:N]]
    cov = {b: _cov_of_real(b) for b in blocks}

    base = myerson(blocks, cov, T)
    target = max(blocks, key=lambda b: base[b])
    base_target_val = base[target]
    print(f"baseline: {N} honest blocks. target block {target} honest value = {base_target_val:.1f}\n")

    # ATTACK 1: sybil-split — clone target into K identical-content copies
    items = blocks[:]
    for k in range(K):
        cid = f"SYBIL{k}"
        cov[cid] = set(cov[target])      # identical coverage
        items.append(cid)
    val = myerson(items, cov, T)
    attacker_total = sum(val[f"SYBIL{k}"] for k in range(K)) + val[target]
    print(f"=== ATTACK 1: sybil-split target into {K} identical copies ===")
    print(f"  attacker controls original + {K} clones; attacker total value = {attacker_total:.1f}")
    print(f"  honest single-block value was {base_target_val:.1f}")
    ratio = attacker_total / base_target_val if base_target_val else 0
    print(f"  inflation ratio = {ratio:.2f}x  -> "
          f"{'RESISTANT (clones split one value, no multiplication)' if ratio < 1.25 else 'GAMEABLE — clones multiplied value!'}")

    # ATTACK 2: padding — add a block whose coverage is a subset of an existing one
    cov2 = {b: _cov_of_real(b) for b in blocks}
    donor = blocks[0]
    sub = set(list(cov2[donor])[:len(cov2[donor]) // 2])   # strict subset
    items2 = blocks + ["PAD"]
    cov2["PAD"] = sub
    val2 = myerson(items2, cov2, T)
    print(f"\n=== ATTACK 2: padding — add a block that is a coverage-subset of {donor} ===")
    print(f"  padded block value = {val2['PAD']:.2f}  (its coverage is fully redundant)")
    print(f"  -> {'RESISTANT (redundant block earns ~0)' if val2['PAD'] < 0.5 else 'GAMEABLE — padding paid off'}")

    # ===== DEFENSE (item 4 + 7): temporal-novelty via commit-reveal ordering =====
    # value(b) = coverage NEW relative to all EARLIER-committed blocks. Honest blocks
    # are committed first (their timestamps); an attacker's added blocks come LAST, so
    # they earn 0 novel coverage. This is why commit-reveal is load-bearing for VALUE,
    # not just authorship: it gives the canonical order that kills padding + sybil.
    def novelty(order, c):
        seen, val = set(), {}
        for b in order:
            val[b] = len(c[b] - seen)
            seen |= c[b]
        return val

    print("\n=== DEFENSE: temporal-novelty value (commit-reveal ordering) ===")
    # attack 1 re-test: honest first, sybil clones appended last
    cd = {b: _cov_of_real(b) for b in blocks}
    for k in range(K):
        cd[f"SYBIL{k}"] = set(cd[target])
    nv1 = novelty(blocks + [f"SYBIL{k}" for k in range(K)], cd)
    syb = sum(nv1[f"SYBIL{k}"] for k in range(K))
    print(f"  sybil clones' novel value (attacker, want 0): {syb}  -> "
          f"{'DEFEATED' if syb == 0 else 'still leaks ' + str(syb)}")
    # attack 2 re-test: padding (subset of blocks[0]) appended last
    cd2 = {b: _cov_of_real(b) for b in blocks}
    cd2["PAD"] = set(list(cd2[blocks[0]])[:len(cd2[blocks[0]]) // 2])
    nv2 = novelty(blocks + ["PAD"], cd2)
    print(f"  padding block's novel value (attacker, want 0): {nv2['PAD']}  -> "
          f"{'DEFEATED' if nv2['PAD'] == 0 else 'still leaks ' + str(nv2['PAD'])}")
    # attack 3 re-test: COLLUSION RING — K blocks that recombine existing coverage
    # and mutually attribute (no NEW coverage), committed after the honest set.
    cd3 = {b: _cov_of_real(b) for b in blocks}
    pool = set()
    for b in blocks:
        pool |= cd3[b]
    pool = list(pool)
    rnd = random.Random(99)
    ring = []
    for k in range(K):
        rnd.shuffle(pool)
        cd3[f"RING{k}"] = set(pool[:len(pool) // 3])   # recombined existing coverage, nothing new
        ring.append(f"RING{k}")
    nv3 = novelty(blocks + ring, cd3)
    ring_val = sum(nv3[r] for r in ring)
    print(f"  collusion-ring novel value ({K} mutually-attributing blocks, want 0): {ring_val}  -> "
          f"{'DEFEATED (ring adds no NEW coverage -> 0)' if ring_val == 0 else 'still leaks ' + str(ring_val)}")

    # honest blocks still earn their genuine novelty (not zeroed)
    honest_nv = novelty(blocks, {b: _cov_of_real(b) for b in blocks})
    print(f"  honest blocks still earn novelty (not zeroed): "
          f"{sum(1 for b in blocks if honest_nv[b] > 0)}/{len(blocks)} blocks > 0")

    # ATTACK 4: novelty FRONT-RUN / predictive land-grab. The temporal-novelty defense
    # rests on ONE assumption: honest blocks commit FIRST. A front-runner who commits
    # FIRST a block covering the most COMMON (predictable) atoms steals their novelty
    # from the honest originators who reveal them later. Commit-reveal binds claimed
    # coverage to content (you must possess what you commit), so UNSEEN/original honest
    # work can't be front-run; but high-frequency PUBLIC atoms (boilerplate) are cheaply
    # substantiated and land-grabbable. This is exactly why raw coverage-count is a
    # [proxy] and the LEARNED v(S) (value by outcome, not atom count) is load-bearing.
    from collections import Counter
    cdf = {b: _cov_of_real(b) for b in blocks}
    freq = Counter()
    for b in blocks:
        freq.update(cdf[b])
    common = {atom for atom, c in freq.items() if c >= 2}   # predictable/public atoms
    print("\n=== ATTACK 4: novelty front-run (predictive land-grab, attacker commits FIRST) ===")
    if not common:
        print("  no atom occurs in >=2 honest blocks -> attack N/A on this block set")
    else:
        cdf["GRAB"] = set(common)
        honest_baseline = sum(novelty(blocks, {b: cdf[b] for b in blocks})[b] for b in blocks)
        nvf = novelty(["GRAB"] + blocks, cdf)              # attacker committed FIRST
        grab_raw, honest_after = nvf["GRAB"], sum(nvf[b] for b in blocks)
        gameable = grab_raw > 0 and honest_after < honest_baseline
        print(f"  attacker GRABs {len(common)} common atoms; raw-novelty value = {grab_raw}")
        print(f"  honest novelty after front-run = {honest_after} (was {honest_baseline}) -> "
              f"{'GAMEABLE under raw novelty — front-run stole honest value' if gameable else 'no theft'}")

        # DEFENSE: value-weighted novelty. Weight each atom by RARITY (1/freq) -- a
        # stand-in for the learned v(S), under which common/boilerplate coverage is
        # low-value and original (rare) coverage is high-value. Land-grabbing common
        # atoms now earns ~0; honest original coverage keeps value even if revealed late.
        def vnovelty(order, c, w):
            seen, val = set(), {}
            for b in order:
                new = c[b] - seen
                val[b] = sum(w.get(a, 1.0) for a in new)
                seen |= c[b]
            return val
        w = {a: 1.0 / cnt for a, cnt in freq.items()}      # rarity ~ outcome-value proxy
        vf = vnovelty(["GRAB"] + blocks, cdf, w)
        grab_v, honest_v = vf["GRAB"], sum(vf[b] for b in blocks)
        print("  DEFENSE: value(rarity)-weighted novelty (proxy for the learned v(S)):")
        print(f"    attacker land-grab value = {grab_v:.3f}; honest value = {honest_v:.3f} -> "
              f"{'DEFEATED (front-run earns less than honest)' if grab_v < honest_v else 'still leaks'}")
        # regression: under value-weighting the front-run must NOT out-earn honest work
        assert grab_v < honest_v, "front-run land-grab out-earns honest under value-weighting"

    print("\nnote: temporal-novelty (first-to-reveal-coverage, via commit-reveal order) is "
          "strategyproof vs sybil/padding/collusion (all RE-cover existing coverage) while "
          "preserving honest novelty. But it assumes honest-commits-first: a predictive "
          "front-run of COMMON atoms games raw coverage-count, and only VALUE-weighting (the "
          "learned v(S), outcome not atom-count) defeats it. coverage = [proxy]; v(S) = moat.")


if __name__ == "__main__":
    main()
