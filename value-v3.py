#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""value-v3 — the canonical STRATEGYPROOF PoM value rule (PRIVATE).

The production rule, resolving the inter/intra split discovered this session:

  INTER-block (the chain, which HAS a commit order via commit-reveal):
    value(b) = | coverage(b) \\ union(coverage of all EARLIER-committed blocks) |
    First-to-reveal-coverage earns it. Strategyproof by construction: a later
    sybil/padding/collusion block adds no NEW coverage -> 0. (Myerson over coverage
    was gameable by padding; temporal-novelty is not. Proven by adversarial-game.py.)

  INTRA-block (co-authors of ONE block, no order among them):
    Myerson value of the 2+-player sub-game (see value-flow.recurse_block). Synergy
    matters there because the contributors ARE simultaneous.

PoM(owner) = sum of inter-block novel-value of owned blocks (then split intra-block
to co-authors). coverage = [proxy] for the learned reward model, which would WEIGHT
novel coverage by quality (value = novelty x quality) — kept separable so the
strategyproofness (novelty) and capability (quality) layers compose. Stdlib only.

  pom [N]       strategyproof PoM over first N blocks (commit-ordered)
  attack [N]    same, but inject a sybil+padding+ring block last; show each earns 0
"""
import glob
import json
import os
import sys

try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

HOME = os.path.expanduser("~")
BLOCKS = os.path.join(HOME, ".claude", "session-chain", "blocks")
REG = os.path.join(HOME, ".claude", "projects", "C--Users-Will",
                   "memory", "_system", "block_ownership.json")


def _b(bid):
    return json.load(open(os.path.join(BLOCKS, bid + ".json"), encoding="utf-8"))


def cov(bid):
    t = str(_b(bid).get("response", "")).lower()
    return {t[i:i + 5] for i in range(0, max(0, len(t) - 4))} or {bid}


def commit_order(blocks):
    return sorted(blocks, key=lambda b: _b(b).get("timestamp", 0))


def novel_values(order, covmap):
    seen, val = set(), {}
    for b in order:
        c = covmap[b]
        val[b] = len(c - seen)
        seen |= c
    return val


def owner_of(bid, reg):
    o = reg["genesis_owner"]
    for t in reg.get("transfers", []):
        if t["block_id"] == bid:
            o = {"id": t["new_id"]}
    return o["id"]


def main():
    a = sys.argv[1:]
    mode = a[0] if a else "pom"
    N = int(a[1]) if len(a) > 1 else 10
    blocks = [os.path.basename(f).replace(".json", "")
              for f in sorted(glob.glob(os.path.join(BLOCKS, "block-*.json")))[:N]]
    order = commit_order(blocks)
    covmap = {b: cov(b) for b in blocks}

    if mode == "attack":
        pool = set()
        for b in blocks:
            pool |= covmap[b]
        pool = list(pool)
        # three adversary blocks appended AFTER the honest set (later commit time):
        covmap["SYBIL"] = set(covmap[order[-1]])                 # clone of an existing block
        covmap["PAD"] = set(list(covmap[blocks[0]])[:len(covmap[blocks[0]]) // 2])  # subset
        covmap["RING"] = set(pool[:len(pool) // 3])              # recombined, nothing new
        order2 = order + ["SYBIL", "PAD", "RING"]
        v = novel_values(order2, covmap)
        print("strategyproofness of the PRODUCTION rule (adversary blocks committed last):")
        for atk in ("SYBIL", "PAD", "RING"):
            print(f"  {atk:<6} novel value = {v[atk]}  -> {'DEFEATED (0)' if v[atk] == 0 else 'LEAK ' + str(v[atk])}")
        hp = sum(1 for b in blocks if v[b] > 0)
        print(f"  honest blocks still > 0: {hp}/{len(blocks)}")
        return

    v = novel_values(order, covmap)
    reg = json.load(open(REG, encoding="utf-8")) if os.path.exists(REG) else \
        {"genesis_owner": {"id": "jarvis@local"}, "transfers": []}
    pom = {}
    for b in blocks:
        pom[owner_of(b, reg)] = pom.get(owner_of(b, reg), 0) + v[b]
    tot = sum(pom.values()) or 1
    print(f"strategyproof PoM over {len(blocks)} commit-ordered blocks "
          f"(value = novel coverage; later duplicates earn 0):")
    print(f"{'block (commit order)':<22}{'novel value':>12}")
    for b in order:
        print(f"{b:<22}{v[b]:>12}")
    print(f"\n{'owner':<22}{'PoM':>8}{'weight':>10}")
    for o, s in sorted(pom.items(), key=lambda x: -x[1]):
        print(f"{o:<22}{s:>8}{s/tot*100:>9.1f}%")
    print("\nnote: inter-block = temporal-novelty (strategyproof, this); intra-block "
          "co-author split = Myerson (synergy, value-flow.recurse_block). coverage = "
          "[proxy] -> production weights novelty by the learned reward model (novelty x quality).")


if __name__ == "__main__":
    main()
