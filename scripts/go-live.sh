#!/usr/bin/env bash
# go-live.sh — ONE command to put the Noesis devnet online for friends.
#
# Builds the release node, serves its self-contained UI+API (the node embeds its own frontend, so a
# single origin is the whole app — see node/src/rpc.rs `GET /`), and, IF cloudflared is present, opens a
# public quick-tunnel so ONE URL is shareable. PREP, not auto-deploy: you run this when YOU decide to go
# live; Ctrl+C tears the node and the tunnel down together. Nothing here exposes anything until you run it.
#
#   ./scripts/go-live.sh
#
# Knobs (env): NOESIS_PORT (default 9955) · NOESIS_BIND (default 127.0.0.1; set 0.0.0.0 for LAN access
# without a tunnel) · NOESIS_CHAIN (default <repo>/noesis-chain.log — durable, a restart resumes the chain).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
PORT="${NOESIS_PORT:-9955}"
BIND="${NOESIS_BIND:-127.0.0.1}"
CHAIN="${NOESIS_CHAIN:-$ROOT/noesis-chain.log}"

echo "==> building release noesisd (first build is slow; later builds are cached)"
cargo build --release -p noesis --bin noesisd

NODE_BIN="$ROOT/target/release/noesisd.exe"
[ -f "$NODE_BIN" ] || NODE_BIN="$ROOT/target/release/noesisd"
[ -f "$NODE_BIN" ] || { echo "!! build succeeded but no noesisd binary found under target/release/"; exit 1; }

echo "==> starting node on $BIND:$PORT (durable chain: $CHAIN)"
"$NODE_BIN" --serve-api "$BIND:$PORT" "$CHAIN" &
NODE_PID=$!

cleanup() {
  echo ""
  echo "==> shutting down"
  kill "$NODE_PID" 2>/dev/null || true
  [ -n "${TUN_PID:-}" ] && kill "$TUN_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# health check — the node answers /state once it is listening
for i in $(seq 1 20); do
  if curl -sf "http://127.0.0.1:$PORT/state" >/dev/null 2>&1; then break; fi
  sleep 1
done
if ! curl -sf "http://127.0.0.1:$PORT/state" >/dev/null 2>&1; then
  echo "!! node did not answer on :$PORT — check the output above"
  exit 1
fi
echo "==> node is UP. local: http://127.0.0.1:$PORT/"

if command -v cloudflared >/dev/null 2>&1; then
  echo "==> opening a public cloudflared quick-tunnel (free, ephemeral URL)..."
  TUN_LOG="$(mktemp)"
  cloudflared tunnel --url "http://127.0.0.1:$PORT" >"$TUN_LOG" 2>&1 &
  TUN_PID=$!
  URL=""
  for i in $(seq 1 30); do
    URL="$(grep -oE 'https://[a-z0-9-]+\.trycloudflare\.com' "$TUN_LOG" | head -1 || true)"
    [ -n "$URL" ] && break
    sleep 1
  done
  if [ -n "$URL" ]; then
    echo ""
    echo "  ============================================================"
    echo "   NOESIS IS LIVE — share this URL with friends:"
    echo "     $URL"
    echo "   (the node serves the whole app at that one URL — no CORS,"
    echo "    no address to configure; same-origin fetches just work)"
    echo "  ============================================================"
    echo ""
  else
    echo "!! tunnel process started but no URL appeared yet — tail $TUN_LOG"
  fi
else
  cat <<EOF

  cloudflared is NOT installed — serving LOCALLY only for now.
  To go PUBLIC (one shareable URL), install it and re-run this script:
     winget install --id Cloudflare.cloudflared
     (or) https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/

  For same-network friends without a tunnel, re-run with a LAN bind:
     NOESIS_BIND=0.0.0.0 ./scripts/go-live.sh
  then share  http://<your-LAN-IP>:$PORT/

EOF
fi

echo "==> serving. Ctrl+C to stop (tears down the node and the tunnel)."
wait "$NODE_PID"
