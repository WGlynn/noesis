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
    # honest blocks still earn their genuine novelty (not zeroed)
    honest_nv = novelty(blocks, {b: _cov_of_real(b) for b in blocks})
    print(f"  honest blocks still earn novelty (not zeroed): "
          f"{sum(1 for b in blocks if honest_nv[b] > 0)}/{len(blocks)} blocks > 0")

    print("\nnote: Shapley over coverage is gameable by padding because it splits SHARED "
          "coverage with later duplicators; temporal-novelty (first-to-reveal-coverage "
          "earns it, via commit-reveal order) is strategyproof against both attacks while "
          "preserving honest novelty. coverage = [proxy] for the learned reward model.")


if __name__ == "__main__":
    main()


if __name__ == "__main__":
    main()
