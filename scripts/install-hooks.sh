#!/usr/bin/env bash
# Install the tracked pre-commit gate into this clone's .git/hooks.
# Run once per clone (git hooks are not version-controlled). Idempotent.
set -eu
repo="$(git rev-parse --show-toplevel)"
src="$repo/scripts/pre-commit"
dst="$repo/.git/hooks/pre-commit"
cp "$src" "$dst"
chmod +x "$dst" 2>/dev/null || true
echo "[install-hooks] pre-commit installed -> $dst"
echo "[install-hooks] runs babel-test-lint (always) + doc-coherence/study-guide --check on every commit."
