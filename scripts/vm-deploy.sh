#!/usr/bin/env bash
# vm-deploy.sh — put the Noesis durable public testnet on any always-on Ubuntu VM, idempotently.
# Written for Oracle Cloud's Always-Free Ampere (ARM) shape; works on any Ubuntu box with sudo.
# Run it ON the VM (re-running pulls the latest master and redeploys; the chain survives):
#
#   git clone https://github.com/WGlynn/noesis && ./noesis/scripts/vm-deploy.sh
#
# It builds the repo Dockerfile (the same image fly-deploy.sh ships, so the two paths can't drift),
# runs it with the chain log on a Docker volume, opens the host firewall, and verifies /state.
#
# One step it can't do for you (Oracle console, once): open the port in the cloud firewall.
#   Networking > Virtual Cloud Networks > your VCN > Security Lists > Default Security List >
#   Add Ingress Rules: source 0.0.0.0/0, TCP, destination port 9955 (and 80, 443 if you set DOMAIN).
#
# Knobs (env): PORT (default 9955) · REPO_DIR (default: the repo this script sits in) ·
# DOMAIN (optional: a hostname that resolves to this VM, e.g. 203-0-113-7.sslip.io; if set, Caddy
# serves the node over HTTPS on 443 with an automatic certificate).
#
# Sizing: the Rust build wants a few GB of RAM. Ampere A1 (free up to 4 cores / 24 GB) is fine; the
# 1 GB x86 micro shape will likely run out of memory while compiling.
set -euo pipefail

PORT="${PORT:-9955}"
REPO_DIR="${REPO_DIR:-$(cd "$(dirname "$0")/.." && pwd)}"

# 1. docker + git
if ! command -v docker >/dev/null 2>&1; then
  sudo apt-get update
  sudo apt-get install -y docker.io git
fi

# 2. latest source
git -C "$REPO_DIR" pull --ff-only

# 3. build natively (an ARM VM builds an ARM image)
sudo docker build -t noesis-node "$REPO_DIR"

# 4. (re)start the node. Host networking, so the host firewall below is the only gate.
sudo docker rm -f noesis >/dev/null 2>&1 || true
sudo docker run -d --name noesis --restart unless-stopped --network host \
  -v noesis_data:/data noesis-node noesisd --serve-api "0.0.0.0:$PORT" /data/chain.log

# 5. host firewall. Oracle's Ubuntu images reject everything but SSH in iptables; plain Ubuntu uses ufw.
open_port() {
  sudo iptables -C INPUT -p tcp --dport "$1" -j ACCEPT 2>/dev/null \
    || sudo iptables -I INPUT -p tcp --dport "$1" -j ACCEPT
  if sudo ufw status 2>/dev/null | grep -q "Status: active"; then sudo ufw allow "$1/tcp"; fi
}
open_port "$PORT"

# 6. optional HTTPS
if [ -n "${DOMAIN:-}" ]; then
  open_port 80
  open_port 443
  sudo docker rm -f noesis-caddy >/dev/null 2>&1 || true
  sudo docker run -d --name noesis-caddy --restart unless-stopped --network host \
    -v caddy_data:/data caddy:2 caddy reverse-proxy --from "$DOMAIN" --to "localhost:$PORT"
fi
if command -v netfilter-persistent >/dev/null 2>&1; then sudo netfilter-persistent save; fi

# 7. verify locally; reachability from outside also needs the console rule above
for _ in $(seq 1 30); do
  curl -sf "http://127.0.0.1:$PORT/state" >/dev/null 2>&1 && break
  sleep 2
done
if ! curl -sf "http://127.0.0.1:$PORT/state" >/dev/null 2>&1; then
  echo "!! node did not answer on :$PORT. Logs: sudo docker logs noesis"
  exit 1
fi
echo "==> node is up: $(curl -s "http://127.0.0.1:$PORT/state")"
if [ -n "${DOMAIN:-}" ]; then
  echo "==> public: https://$DOMAIN/  (the certificate can take a minute on first run)"
else
  echo "==> public: http://<this VM's public IP>:$PORT/  (plain HTTP; set DOMAIN for HTTPS)"
fi
