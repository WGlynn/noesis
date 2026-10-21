#!/usr/bin/env python3
"""Fail when a checked-in on-VM ELF fixture is older than the sources it was built from.

Why (2026-10-21): the integration tests run the RISC-V type-scripts from binaries checked in under
node/tests/fixtures/. Nothing checked that those binaries matched their sources. The finalization
fixture dated from 2026-06-13 while its script was rerouted through `finalizes_pos_pom_fixed` on
2026-10-08, so for two weeks the on-VM finality tests ran an old binary. A rebuild that day found four
of the five fixtures differed from a fresh build of current source (all on-VM tests then passed).

How: each fixture's inputs (its crate's src/, Cargo.toml, Cargo.lock and rust-toolchain.toml, plus the
shared onchain/noesis-core src/ and Cargo.toml) are hashed with line endings normalized. The hashes are
recorded in node/tests/fixtures/SOURCES.sha256 when the fixtures are rebuilt (`make elf` stamps).
`--check` (the default; pre-commit and CI) fails if any input changed since the stamp. A comment-only
change also asks for a rebuild: builds are reproducible on the pinned toolchain, so the rebuilt ELF may
come out byte-identical, and then re-stamping is all that is needed.

    python scripts/fixture-freshness.py            # check
    python scripts/fixture-freshness.py --stamp    # after rebuilding the ELFs

Not covered: node/tests/fixtures/proof-of-mind-lock-script, which has no source crate in this repo.
"""
import hashlib, os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FIXTURES = ["pom-typescript", "finalization-typescript", "commit-order-typescript", "locksig-typescript",
            "index-typescript"]
SHARED = ["onchain/noesis-core/src", "onchain/noesis-core/Cargo.toml"]
MANIFEST = os.path.join(ROOT, "node", "tests", "fixtures", "SOURCES.sha256")
TARGET = "riscv64imac-unknown-none-elf"


def inputs(name):
    # Files git tracks (index included, so a newly staged source counts), never stray local files: the
    # same set on every clone, so a laptop and CI hash the same inputs.
    own = [f"onchain/{name}/{p}" for p in ("src", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml")]
    out = subprocess.run(["git", "-C", ROOT, "ls-files", "--", *own, *SHARED], capture_output=True, text=True,
                         check=True).stdout
    return sorted(f for f in out.splitlines() if f)


def digest(name):
    h = hashlib.sha256()
    for rel in inputs(name):
        data = open(os.path.join(ROOT, rel), "rb").read().replace(b"\r\n", b"\n")
        h.update(rel.encode() + b"\0" + hashlib.sha256(data).digest())
    return h.hexdigest()


def recorded():
    if not os.path.exists(MANIFEST):
        return {}
    rows = (line.split() for line in open(MANIFEST, encoding="utf-8") if line.strip() and not line.startswith("#"))
    return {name: h for h, name in rows}


def main():
    if "--stamp" in sys.argv:
        missing = [n for n in FIXTURES if not os.path.isfile(os.path.join(ROOT, "node", "tests", "fixtures", n))]
        if missing:
            print(f"fixture-freshness: cannot stamp, fixture missing: {', '.join(missing)}")
            return 1
        with open(MANIFEST, "w", encoding="utf-8", newline="\n") as fh:
            fh.write("# Source hashes of the on-VM ELF fixtures (scripts/fixture-freshness.py). Stamp only after\n"
                     "# rebuilding: `make elf`, or build each crate and copy it here, then --stamp.\n")
            for n in FIXTURES:
                fh.write(f"{digest(n)}  {n}\n")
        print(f"fixture-freshness: stamped {len(FIXTURES)} fixtures")
        return 0
    rec = recorded()
    stale = [n for n in FIXTURES if rec.get(n) != digest(n)]
    if not stale:
        print(f"fixture-freshness: {len(FIXTURES)} fixtures match their sources")
        return 0
    print("fixture-freshness: STALE on-VM fixtures (sources changed since the ELF was last built):")
    for n in stale:
        print(f"  {n}{'' if n in rec else ' (no recorded hash)'}")
    print("Rebuild them, then stamp:")
    for n in stale:
        print(f"  (cd onchain/{n} && cargo build --release --target {TARGET}) && "
              f"cp onchain/{n}/target/{TARGET}/release/{n} node/tests/fixtures/{n}")
    print("  python scripts/fixture-freshness.py --stamp   # then rerun the ckb_vm_* tests")
    return 1


if __name__ == "__main__":
    sys.exit(main())
