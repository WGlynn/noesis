#!/usr/bin/env bash
# fly-deploy.sh — put the Noesis durable public testnet online on fly.io, idempotently.
#
# Prereq: `fly auth login` already done (this script refuses if not). Everything else it handles:
# create the app (picking a free name if the default is taken), create the persistent volume once,
# deploy, then VERIFY the node actually answers — never trusting the deploy exit code alone
# (per feedback_flyio-deploy-verification: confirm via /state + logs, not the return code).
#
#   ./scripts/fly-deploy.sh
#
# Knobs (env): APP (default noesis-testnet) · REGION (default ord) · VOL_SIZE_GB (default 1).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
FLY="$(command -v fly || command -v flyctl)"
APP="${APP:-noesis-testnet}"
REGION="${REGION:-ord}"
VOL_SIZE_GB="${VOL_SIZE_GB:-1}"

# 0. auth
if ! "$FLY" auth whoami >/dev/null 2>&1; then
  echo "!! not logged in. Run:  fly auth login   (choose Continue with GitHub), then re-run this."
  exit 1
fi
echo "==> fly user: $("$FLY" auth whoami 2>/dev/null)"

# 1. app — reuse if it exists, else create; if the default name is taken by someone else, suffix it.
if "$FLY" apps list 2>/dev/null | awk '{print $1}' | grep -qx "$APP"; then
  echo "==> app '$APP' already exists (reusing)"
else
  if ! "$FLY" apps create "$APP" 2>/dev/null; then
    APP="noesis-testnet-$(head -c2 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    echo "==> default name taken; creating '$APP'"
    "$FLY" apps create "$APP"
  fi
fi
# Keep fly.toml's app line in sync with the app we actually created (so `fly deploy` targets it).
if command -v sed >/dev/null 2>&1; then
  sed -i.bak -E "s/^app = .*/app = \"$APP\"/" fly.toml && rm -f fly.toml.bak
fi
echo "==> deploying app: $APP  (region $REGION)"

# 2. volume — create once (durable chain log). Skip if one already exists for this app.
if "$FLY" volumes list -a "$APP" 2>/dev/null | grep -q noesis_data; then
  echo "==> volume 'noesis_data' already exists (reusing)"
else
  echo "==> creating 1x ${VOL_SIZE_GB}GB volume 'noesis_data' in $REGION"
  "$FLY" volumes create noesis_data -a "$APP" -r "$REGION" -s "$VOL_SIZE_GB" -y
fi

# 3. deploy
"$FLY" deploy -a "$APP" --regions "$REGION"

# 4. VERIFY — the deploy exit code is NOT proof. Confirm the node actually serves.
URL="https://$APP.fly.dev"
echo "==> verifying the node is live at $URL/state ..."
ok=""
for i in $(seq 1 20); do
  if curl -sf "$URL/state" >/dev/null 2>&1; then ok=1; break; fi
  sleep 3
done
echo "--- fly status ---"; "$FLY" status -a "$APP" 2>/dev/null | head -20 || true
if [ -n "$ok" ]; then
  echo ""
  echo "  ============================================================"
  echo "   NOESIS TESTNET IS LIVE — share this URL with friends:"
  echo "     $URL"
  echo "   Web wallet:   open the URL, Create wallet, contribute."
  echo "   (This deploy exposes the HTTP wallet/API only. A raw-TCP --listen seed for"
  echo "    full-node --connect joiners is a separate service, not wired here yet.)"
  echo "  ============================================================"
else
  echo "!! $URL/state did not answer yet. Do NOT assume success — check logs:"
  echo "     fly logs -a $APP"
  exit 1
fi
