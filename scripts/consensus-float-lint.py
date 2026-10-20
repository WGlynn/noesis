#!/usr/bin/env python3
"""Fail if floating-point math appears in the on-chain crates.

Every node must compute bit-identical results, and floats are not reliable for that
(rounding can differ by CPU; see docs/DESIGN-value-oracle-seam.md, "No floats on the
consensus path"). The on-chain crates are integer / fixed-point today. This keeps them so.
Runs in the pre-commit hook and in CI. Exit 1 lists each offending line.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CRATES = [
    "onchain/noesis-core",
    "onchain/pom-typescript",
    "onchain/finalization-typescript",
    "onchain/commit-order-typescript",
    "onchain/locksig-typescript",
    "onchain/index-typescript",
    "onchain/zk-finalize",
    "onchain/zk-score",
]
# f32/f64 types, and float literals like 1.5 or 2e9 (not tuple fields like x.0.1, not hex).
FLOAT = re.compile(r"\bf(?:32|64)\b|(?<![\w.])\d+\.\d+|(?<![\w.])\d+[eE][+-]?\d+\b")
COMMENT = re.compile(r"/\*.*?\*/|//[^\n]*", re.S)
STRING = re.compile(r'"(?:\\.|[^"\\])*"')


def main():
    hits = []
    for crate in CRATES:
        for path in sorted((ROOT / crate).rglob("*.rs")):
            if "target" in path.parts:
                continue
            src = path.read_text(encoding="utf-8")
            # blank out comments and strings, keeping newlines so line numbers stay right
            src = COMMENT.sub(lambda m: re.sub(r"[^\n]", " ", m.group()), src)
            src = STRING.sub(lambda m: re.sub(r"[^\n]", " ", m.group()), src)
            for n, line in enumerate(src.splitlines(), 1):
                if FLOAT.search(line):
                    hits.append(f"{path.relative_to(ROOT).as_posix()}:{n}: {line.strip()}")
    if hits:
        print("consensus-float-lint: floating-point math in on-chain code:")
        print("\n".join("  " + h for h in hits))
        print("Use integer / fixed-point (see noesis-core Q16.16 / Q32.32 helpers).")
        return 1
    print("consensus-float-lint: PASS, no floats in on-chain crates.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
