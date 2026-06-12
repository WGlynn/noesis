#!/usr/bin/env python3
"""Living study guide generator — STUDY-GUIDE.md is built FROM the repo so it cannot
go stale (same philosophy as doc-coherence.py).

Reads: the repo's *.md docs (title + first paragraph synopsis), the node/ module map
(`pub mod` + doc-comment one-liner), the live `cargo test` count, and a fixed glossary
of load-bearing terms. Emits a read-in-order path, per-doc synopsis, module map, test
inventory, key decisions, and progress checkboxes Will ticks as he internalizes each
piece.

Usage:
  python scripts/study-guide.py            # regenerate STUDY-GUIDE.md
  python scripts/study-guide.py --check    # exit 1 if STUDY-GUIDE.md is out of date

Pairs with [F.will-learning-goals]. Wire alongside the doc-coherence stamp so it tracks
contents automatically.
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIB = ROOT / "node" / "src" / "lib.rs"
OUT = ROOT / "STUDY-GUIDE.md"

# Curated read order — pedagogy, not filesystem order. Docs not listed are appended
# under "Reference" so nothing is silently dropped.
READ_ORDER = [
    "WHITEPAPER-FOR-DAD.md",
    "WHITEPAPER.md",
    "ROADMAP.md",
    "BLOCK-ECONOMY-SPEC.md",
    "POM-CONSENSUS.md",
    "DISPUTE-SLASHING.md",
    "OUTCOME-EVALUATOR.md",
    "COHERENCE-LAWS.md",
    "COORDINATION-SCHELLING.md",
    "CRYPTOECONOMICS.md",
    "THRONE.md",
]

GLOSSARY = {
    "PoM (Proof of Mind)": "verified, synergy-weighted contribution as consensus weight, replacing Proof of Work.",
    "Noeum": "the unit — 1 Noeum = 1 byte of state = 1 PoM unit.",
    "temporal-novelty": "value = coverage novel vs earlier-committed blocks (commit-reveal order); strategyproof by construction.",
    "floored novelty": "temporal-novelty after the similarity floor zeroes near-duplicates.",
    "realized-flow gate (v5)": "value = floored_novelty x g(downstream_flow); quality is a realized GATE, not a predicted boost.",
    "priced identity (v6)": "flow seeds count only from contributors whose soulbound standing clears a floor — identity costs earned standing.",
    "soulbound standing": "earned, non-transferable franchise; valid_transition rejects reassignment (no simony).",
    "dispute window (W)": "value vests W epochs after the flow that paid it; refutable while unvested.",
    "causal-share slash": "a refuted certifier loses lambda x (their zero-seed marginal on the target's value) + alpha.",
    "escalation court": "a round-1 PoM-only veto is appealed to the AND-composed full NCI mix; overturned jurors are slashed.",
    "role-bounded evaluator": "the learned v(S) may advance timing + inform disputes, never mint; corrupt-evaluator bound is tested.",
    "Myerson value": "graph-restricted Shapley — value flows only along provenance-connected coalitions.",
    "core / nucleolus": "cooperative-game stability: an allocation no coalition can profitably defect from.",
    "NCI mix": "Nakamoto-Consensus-Infinity weighting PoW 10 / PoS 30 / PoM 60 bps, 2/3 finalization bar.",
}


def first_paragraph(md: Path) -> str:
    lines = md.read_text(encoding="utf-8", errors="replace").splitlines()
    out: list[str] = []
    for ln in lines:
        s = ln.strip()
        if s.startswith("#") or s.startswith(">") or not s:
            if out:
                break
            continue
        out.append(s)
        if len(" ".join(out)) > 200:
            break
    syn = " ".join(out)
    return (syn[:240] + "...") if len(syn) > 240 else syn or "(no synopsis)"


def module_map() -> list[tuple[str, str]]:
    text = LIB.read_text(encoding="utf-8", errors="replace").splitlines()
    mods: list[tuple[str, str]] = []
    for i, ln in enumerate(text):
        m = re.match(r"\s*pub mod (\w+)", ln)
        if not m:
            continue
        # Walk up over the contiguous /// block, collecting it top-down, then take its
        # OPENING sentence (the module's purpose) — not the last line above the `mod`.
        block: list[str] = []
        j = i - 1
        while j >= 0 and text[j].strip().startswith("///"):
            block.append(text[j].strip().lstrip("/").strip())
            j -= 1
        block.reverse()
        doc = " ".join(b for b in block if b)
        doc = doc.split(". ")[0].rstrip(".") if doc else "(module)"
        mods.append((m.group(1), doc))
    return mods


def test_count() -> str:
    try:
        r = subprocess.run(
            ["cargo", "test"], cwd=ROOT / "node",
            capture_output=True, text=True, timeout=300,
        )
        m = re.search(r"(\d+) passed", r.stdout + r.stderr)
        return m.group(1) if m else "?"
    except Exception:
        return "?"


def build() -> str:
    docs = {p.name: p for p in ROOT.glob("*.md") if p.name != OUT.name}
    ordered = [d for d in READ_ORDER if d in docs]
    rest = sorted(n for n in docs if n not in READ_ORDER)
    tc = test_count()
    mods = module_map()

    L: list[str] = []
    L.append("# Noesis — living study guide (generated; do not hand-edit)")
    L.append("")
    L.append("> Regenerated FROM the repo by `scripts/study-guide.py`, so it cannot lag the")
    L.append("> code. Tick the boxes as you internalize each piece. Re-run to refresh.")
    L.append(f"> Node test suite: **{tc} passing**.")
    L.append("")
    L.append("## Read in this order")
    L.append("")
    for i, name in enumerate(ordered, 1):
        L.append(f"{i}. [ ] **{name}** — {first_paragraph(docs[name])}")
    L.append("")
    if rest:
        L.append("### Reference (not on the critical path)")
        for name in rest:
            L.append(f"- [ ] {name} — {first_paragraph(docs[name])}")
        L.append("")
    L.append("## Code map (`node/src/lib.rs`)")
    L.append("")
    for name, doc in mods:
        one = doc[:140] + ("..." if len(doc) > 140 else "")
        L.append(f"- [ ] `{name}` — {one}")
    L.append("")
    L.append("## Glossary (the load-bearing terms)")
    L.append("")
    for term, defn in GLOSSARY.items():
        L.append(f"- [ ] **{term}** — {defn}")
    L.append("")
    L.append("## The one-sentence spine")
    L.append("")
    L.append("Reward is paid only as others build on your work (service, structurally);")
    L.append("identity that certifies must be earned and is slashable when it certifies")
    L.append("garbage; and the learned judge can advance or inform but never mint — so the")
    L.append("measurement stays un-gameable without trusting any model.")
    L.append("")
    return "\n".join(L) + "\n"


def main() -> int:
    content = build()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != content:
            print("[study-guide] STALE — run `python scripts/study-guide.py`")
            return 1
        print("[study-guide] up to date")
        return 0
    OUT.write_text(content, encoding="utf-8")
    print(f"[study-guide] wrote {OUT.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
