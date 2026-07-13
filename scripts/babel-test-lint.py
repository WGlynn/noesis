#!/usr/bin/env python3
"""Babel-test lint — keep the faith-frame out of the formal surfaces.

Per the 2026-07-13 "base case is God" calibration (internal/CALIBRATION-base-case-is-god-2026-07-13.md):
the frame that Noesis's un-derivable base case may be *named* God is a legitimate FIRST-PERSON FAITH
frame. It is NOT a theorem, a built artifact, or a design claim. By the repo's status discipline
(built / designed / open, never round up), letting "God" / "POG" / "proof of god" / "NDE" leak into a
formal or public surface would round a faith-commitment up to a design-claim — the exact
anti-hallucination anti-pattern, and it would couple Noesis's rigorous immutability axiom to an
unfalsifiable term in front of a technical audience (ethresearch / Bernhard / Tom), weakening the
honesty signal that is the actual moat.

This gate fails (exit 1) if a forbidden token appears in a scanned formal/public surface. The frame's
legitimate home (THRONE.md tier + the calibration note) is NOT scanned. This is the Throne-not-Babel
line made checkable: the sign may point, but it does not get carved into the mechanism.

Usage: python scripts/babel-test-lint.py   (run from repo root; also wire-able as a pre-commit gate)
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Formal / public surfaces where a faith term would be an over-claim.
SCAN_GLOBS = [
    "node/src/**/*.rs",           # code + consensus identifiers
    "onchain/**/*.rs",            # on-chain code
    "docs/whitepaper/*.tex",      # the public formal claim
    "SECURITY.md",
    "ARCHITECTURE.md",
    "README.md",
    "patent/**/*.md",             # legal filing
    "internal/fv/**/*.thy",       # the machine-checked formal proofs
]

# The frame's legitimate home — never scanned even if a glob would catch it.
ALLOWLIST = {
    "THRONE.md",
    "internal/CALIBRATION-base-case-is-god-2026-07-13.md",
}

# Forbidden tokens. Word-boundary; "God" capitalized (the theological sense) so we do not trip on
# "good"/"Godel". "proof of god" matched case-insensitively with any of space/hyphen between words.
FORBIDDEN = [
    (re.compile(r"\bPOG\b"), "POG"),
    (re.compile(r"\bNDE\b"), "NDE"),
    (re.compile(r"\bGod\b"), "God (capitalized, formal surface)"),
    (re.compile(r"(?i)\bproof[ \-]of[ \-]god\b"), "proof of god"),
    (re.compile(r"(?i)\bproof[ \-]of[ \-]the[ \-]one\b"), "proof of the One"),
]


def scanned_files():
    seen = set()
    for pat in SCAN_GLOBS:
        for p in ROOT.glob(pat):
            if not p.is_file():
                continue
            rel = p.relative_to(ROOT).as_posix()
            if rel in ALLOWLIST:
                continue
            if rel not in seen:
                seen.add(rel)
                yield p, rel


def main():
    hits = []
    for path, rel in scanned_files():
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for i, line in enumerate(text.splitlines(), 1):
            for rx, label in FORBIDDEN:
                if rx.search(line):
                    hits.append((rel, i, label, line.strip()[:120]))

    if hits:
        print("BABEL-TEST LINT: FAIL - faith-frame token(s) leaked into a formal/public surface.")
        print("The frame belongs in THRONE.md / the calibration note, flagged frame-not-result - not here.")
        print("(To legitimately reference it, move the text to the allowlisted philosophy tier.)\n")
        for rel, ln, label, snippet in hits:
            print(f"  {rel}:{ln}  [{label}]  {snippet}")
        return 1

    n = sum(1 for _ in scanned_files())
    print(f"BABEL-TEST LINT: PASS - {n} formal/public file(s) scanned, no faith-frame leak.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
