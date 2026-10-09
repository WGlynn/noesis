#!/usr/bin/env python3
"""Doc-code coherence gate (PRIVATE).

A stale doc is worse than no doc: a reader acts on false information with full
confidence. That information asymmetry is the thing this gate closes structurally.

Mechanism (not a discipline -- a gate):
  - Code carries a content-hash over CODE_GLOBS (the Rust core + the Python prototypes).
  - The manifest `.doc-coherence.json` records the code-hash the docs were last
    reconciled against, plus the doc list.
  - `--check` (default, and the pre-commit hook): if the current code-hash != the
    reconciled hash, the docs may lag the code -> report which code files changed and
    FAIL. You cannot commit code that outruns the docs without seeing this.
  - `--stamp`: after you have reconciled the docs, record the current code-hash. This
    is the conscious "docs reviewed, they match" act -- the release valve.

Plus machine-checkable invariants that never need a human eye:
  - no doc may reference the old repo name `jarvis-private` (repo is `noesis`).
  - any "<n>/<n> passing" or "<n> tests" claim in a doc must match `cargo test`'s count.

Stdlib only. Run from the repo root (or any subdir; it finds the root via .git).
"""
import glob
import hashlib
import json
import os
import re
import subprocess
import sys

CODE_GLOBS = ["node/src/**/*.rs", "*.py", "scripts/*.py"]
DOC_GLOBS = ["*.md", "node/*.md", "docs/*.md"]
MANIFEST = ".doc-coherence.json"
STALE_NAME = "jarvis-private"  # old repo name; must not appear in any doc


def repo_root() -> str:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True, text=True, check=True,
        )
        return out.stdout.strip()
    except Exception:
        return os.getcwd()


def code_files(root: str) -> list[str]:
    files: set[str] = set()
    for g in CODE_GLOBS:
        for p in glob.glob(os.path.join(root, g), recursive=True):
            if os.path.isfile(p):
                files.add(os.path.relpath(p, root).replace("\\", "/"))
    return sorted(files)


def code_hash(root: str) -> tuple[str, dict]:
    """sha256 over (path, content) of every code file -> one hash + per-file hashes."""
    h = hashlib.sha256()
    per_file: dict[str, str] = {}
    for rel in code_files(root):
        with open(os.path.join(root, rel), "rb") as f:
            body = f.read()
        fh = hashlib.sha256(body).hexdigest()
        per_file[rel] = fh
        h.update(rel.encode())
        h.update(fh.encode())
    return h.hexdigest(), per_file


def docs(root: str) -> list[str]:
    out: set[str] = set()
    for g in DOC_GLOBS:
        for p in glob.glob(os.path.join(root, g)):
            rel = os.path.relpath(p, root).replace("\\", "/")
            out.add(rel)
    return sorted(out)


def load_manifest(root: str) -> dict:
    p = os.path.join(root, MANIFEST)
    if os.path.exists(p):
        with open(p, encoding="utf-8") as f:
            return json.load(f)
    return {}


def save_manifest(root: str, m: dict) -> None:
    with open(os.path.join(root, MANIFEST), "w", encoding="utf-8") as f:
        json.dump(m, f, indent=2)
        f.write("\n")


def cargo_test_count(root: str) -> int | None:
    """Total passing unit tests, or None if cargo unavailable."""
    node = os.path.join(root, "node")
    if not os.path.isdir(node):
        return None
    try:
        out = subprocess.run(
            ["cargo", "test"], cwd=node, capture_output=True, text=True, timeout=180
        )
    except Exception:
        return None
    total = 0
    seen = False
    for m in re.finditer(r"test result: ok\. (\d+) passed", out.stdout):
        total += int(m.group(1))
        seen = True
    return total if seen else None


def name_violations(root: str) -> list[str]:
    bad = []
    for d in docs(root):
        with open(os.path.join(root, d), encoding="utf-8") as f:
            for i, line in enumerate(f, 1):
                if STALE_NAME in line:
                    bad.append(f"{d}:{i} references old name '{STALE_NAME}'")
    return bad


def test_count_violations(root: str, count: int | None) -> list[str]:
    if count is None:
        return []
    bad = []
    # A real test-count claim is either the slash form "N/N passing" (group 2) or "N tests passing"
    # (group 3). The bare "N test(s)" form is NOT a count claim — it matches prose like "19 test
    # literals" or "+1 test (foo)" in the adversarial log, which are false positives the unconditional
    # check can never --stamp away. Require "passing" adjacency so only genuine count claims are caught.
    pat = re.compile(r"\b(\d+)\s*/\s*(\d+)\s+passing|\b(\d+)\s+tests?\s+passing\b")
    for d in docs(root):
        with open(os.path.join(root, d), encoding="utf-8") as f:
            for i, line in enumerate(f, 1):
                for m in pat.finditer(line):
                    claimed = m.group(2) or m.group(3)
                    if claimed and int(claimed) != count:
                        bad.append(
                            f"{d}:{i} claims {claimed} tests; cargo reports {count}"
                        )
    return bad


def pointer_violations(root: str) -> list[str]:
    """Every `file.rs:NN` code-pointer in a doc must actually resolve: the symbol the
    doc pairs with that pointer must occur AT that line in the code. This is the machine
    enforcement of ARCHITECTURE.md's own load-bearing mandate ("open the cited file:line
    and read the constant"). Line numbers drift on every edit above them; a stale pointer
    lands a trusting reader on unrelated code -- the exact 2026-07-12 finding. Here it
    becomes a gate: a pointer that no longer resolves FAILS instead of rotting silently.

    Matching is deliberately conservative to avoid false positives:
      - the pointer must be `path.rs:NN` (a .rs file with a line), and
      - a backticked symbol (`NCI`, `FINALITY_MIX`, `dim_ok`, ...) must appear on the same
        doc line. We check that this symbol occurs on the cited code line (a small +/- window
        absorbs the off-by-a-few from a definition spanning lines / attribute above the item).
    Docs with no such pointers, or pointers whose symbol we cannot isolate, are skipped."""
    # `path/to/file.rs:NN`  -- capture the (possibly abbreviated) file and the line
    ptr = re.compile(r"`([\w./-]+\.rs):(\d+)`")
    # a plausible code symbol in backticks on the same line (const/fn/type-ish identifiers)
    sym = re.compile(r"`([A-Za-z_][A-Za-z0-9_]*)`")
    # Docs abbreviate paths (`lib.rs`, `src/lib.rs` == `node/src/lib.rs`). Resolve a cited
    # path against the actual .rs files by suffix; a path that matches exactly one real file
    # is checkable, anything ambiguous/unresolvable is SKIPPED (out of scope for a pointer-
    # staleness gate -- flagging it would be a false-positive flood that gets the gate muted).
    real_rs = [f for f in code_files(root) if f.endswith(".rs")]
    WINDOW = 3  # tolerate a definition/attribute a few lines off the exact cite
    src_cache: dict[str, list[str]] = {}

    def resolve(rel: str) -> str | None:
        rel = rel.lstrip("./")
        matches = [f for f in real_rs if f == rel or f.endswith("/" + rel)]
        return matches[0] if len(matches) == 1 else None

    def lines_for(rel: str) -> list[str]:
        if rel not in src_cache:
            with open(os.path.join(root, rel), encoding="utf-8") as f:
                src_cache[rel] = f.read().splitlines()
        return src_cache[rel]

    bad: list[str] = []
    for d in docs(root):
        with open(os.path.join(root, d), encoding="utf-8") as f:
            for i, line in enumerate(f, 1):
                for pm in ptr.finditer(line):
                    target = resolve(pm.group(1))
                    if target is None:
                        continue  # ambiguous/unresolvable path -> not this gate's job
                    lineno = int(pm.group(2))
                    src = lines_for(target)
                    # symbols named on this doc line, minus any file token (`...rs`)
                    syms = [s for s in sym.findall(line) if not s.endswith("rs")]
                    if not syms:
                        continue  # no symbol to anchor on -> can't check, skip
                    lo, hi = max(1, lineno - WINDOW), min(len(src), lineno + WINDOW)
                    window_text = "\n".join(src[lo - 1:hi])
                    # a pointer resolves if ANY symbol the doc line names is found near the cite
                    if not any(re.search(rf"\b{re.escape(s)}\b", window_text) for s in syms):
                        bad.append(
                            f"{d}:{i} pointer {pm.group(0)} is STALE -- "
                            f"none of {syms} found at {target}:{lineno} (+/-{WINDOW})"
                        )
    return bad


def law_violations(root: str) -> list[str]:
    """Structural guard on COHERENCE-LAWS.md: the numbered laws must be contiguous
    (L1..LN, no gaps or dupes) so a careless edit cannot silently delete or renumber a
    law, and the load-bearing invariants must be present by keyword. This is how the
    L13 provisioning-floor (and the L12 AND-over-OR composition rule) get machine-enforced
    presence -- a runtime economic invariant can't be unit-tested here, but its STANDING
    in the laws doc can be guarded."""
    path = os.path.join(root, "docs", "COHERENCE-LAWS.md")
    if not os.path.exists(path):
        return ["COHERENCE-LAWS.md missing"]
    with open(path, encoding="utf-8") as f:
        text = f.read()
    nums = [int(n) for n in re.findall(r"^##\s*L(\d+)\b", text, re.MULTILINE)]
    if not nums:
        return ["COHERENCE-LAWS.md has no '## L<n>' law headers"]
    bad = []
    if sorted(nums) != list(range(1, max(nums) + 1)):
        bad.append(f"law numbering not contiguous 1..{max(nums)}: found {sorted(nums)}")
    required = {
        "L12 composition (AND over OR)": r"AND\s+over\s+OR",
        "L13 provisioning floor": r"security-provisioning floor",
    }
    for label, pat in required.items():
        if not re.search(pat, text, re.IGNORECASE):
            bad.append(f"COHERENCE-LAWS.md missing load-bearing invariant: {label}")
    return bad


def main() -> int:
    root = repo_root()
    mode = sys.argv[1] if len(sys.argv) > 1 else "--check"
    cur_hash, per_file = code_hash(root)
    m = load_manifest(root)

    if mode == "--stamp":
        m["reconciled_code_hash"] = cur_hash
        m["docs"] = docs(root)
        m["code_file_hashes"] = per_file
        save_manifest(root, m)
        print(f"[doc-coherence] STAMPED {len(m['docs'])} docs against code-hash {cur_hash[:12]}")
        return 0

    # --check
    problems: list[str] = []
    prev = m.get("reconciled_code_hash")
    prev_files = m.get("code_file_hashes", {})
    if prev is None:
        problems.append("no reconciled_code_hash recorded -- run --stamp after reconciling docs")
    elif prev != cur_hash:
        changed = [f for f, h in per_file.items() if prev_files.get(f) != h]
        added = [f for f in per_file if f not in prev_files]
        removed = [f for f in prev_files if f not in per_file]
        detail = changed or (added + removed) or ["(code changed)"]
        problems.append(
            "CODE MOVED PAST DOCS -- reconcile docs, then run `python scripts/doc-coherence.py --stamp`:\n    "
            + "\n    ".join(detail)
        )

    problems.extend(name_violations(root))
    problems.extend(test_count_violations(root, cargo_test_count(root)))
    problems.extend(pointer_violations(root))
    problems.extend(law_violations(root))

    if problems:
        print("[doc-coherence] DRIFT DETECTED (docs may lag code):", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    print(f"[doc-coherence] OK -- docs reconciled against code-hash {cur_hash[:12]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
