#!/usr/bin/env bash
# build-wp.sh — compile the Noesis whitepaper AND emit a NEW dated PDF to the Desktop.
#
# RULE (Will 2026-06-19): a new doc for every change. Never overwrite the file Will reads;
# stamp a fresh, version+timestamp-named copy each build. A new filename also dodges the PDF
# viewer cache that made a same-name overwrite look like "no changes."
#
# Usage: bash build-wp.sh   (run after ANY edit to noesis-whitepaper.tex)
set -euo pipefail
cd "$(dirname "$0")"

TEX=noesis-whitepaper.tex
PDF=noesis-whitepaper.pdf
DESK="$HOME/Desktop"

# two passes for tikz/pgfplots cross-refs
pdflatex -interaction=nonstopmode -halt-on-error "$TEX" >/tmp/wp-build1.log 2>&1
pdflatex -interaction=nonstopmode -halt-on-error "$TEX" >/tmp/wp-build2.log 2>&1

# version from \date{Version X.Y ...}, slugged; default 'draft'
VER=$(grep -oE '\\date\{Version [0-9.]+' "$TEX" | grep -oE '[0-9.]+' | head -1)
VER=${VER:-draft}
STAMP=$(date +%Y-%m-%d-%H%M)
OUT="$DESK/Noesis-Whitepaper-v${VER}-${STAMP}.pdf"

cp "$PDF" "$OUT"
PAGES=$(pdfinfo "$PDF" 2>/dev/null | awk '/^Pages/{print $2}')
echo "BUILT: $OUT  (${PAGES}pp)"
echo "       newest dated copy on Desktop; open THIS one, not an older cached file."
