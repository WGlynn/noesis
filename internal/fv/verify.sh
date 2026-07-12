#!/usr/bin/env bash
#
# One-command re-verification of the Phase-4 FV Isabelle spec — the OWNED, reproducible check
# (not a one-time attestation). Anyone can re-prove the rulebook invariants from source:
#
#   ./verify.sh                         # if `isabelle` is on PATH
#   ISABELLE=/path/to/Isabelle2025/bin/isabelle ./verify.sh
#
# Pinned tool: Isabelle2025 (exact URL + sha256 in README.md §"reproducible verification").
# A green run means `conservation` + `no_double_spend` were machine-checked THIS run, on THIS machine.
#
set -euo pipefail
ISABELLE="${ISABELLE:-isabelle}"
DIR="$(cd "$(dirname "$0")" && pwd)"

if ! command -v "$ISABELLE" >/dev/null 2>&1 && [ ! -x "$ISABELLE" ]; then
  echo "error: Isabelle not found. Set ISABELLE=/path/to/Isabelle2025/bin/isabelle (see README)." >&2
  exit 127
fi

echo "verifying Noesis_FV session with: $ISABELLE"
exec "$ISABELLE" build -D "$DIR" -v
