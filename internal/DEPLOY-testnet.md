# Deploy runbook — Noesis durable public testnet

> The prep artifact for L7 (the deploy pole). Goal: a **durable, reachable** Noesis node a stranger can
> open in a browser, submit a contribution to, and watch PoM/JUL grow — that stays up when your PC
> sleeps. Status discipline: ✅ built · 🟡 designed · 🔬 open · 🔌 deploy-coupled · ⚑ Will-gated.
>
> What this session shipped toward it: `ChainSpec::testnet()` (✅), spec selection via `NOESIS_NET`
> (✅), the self-serving node (`GET /` = whole app, ✅), `go-live.sh` (✅). This doc is the standing
> plan to actually put it online + the honest list of what is NOT yet there.

## 0. The one design decision already resolved — JUL on testnet

**Dial the PARAMETER (`genesis_bits`), never the MECHANISM (`pow_enforced`).** `ChainSpec::testnet()`
keeps PoW fully in consensus (`pow_enforced = true`) so the testnet tests the REAL security model, and
JUL mints from work through the exact same path as mainnet (`block_work → reward_for_work`). Because
difficulty is LOW, a block is a few thousand CPU hashes — microseconds, no GPU, no meaningful
electricity. So testnet JUL is a **functional-but-worthless test token by construction** (distinct
`chain_id` + trivial energy + it is a testnet). The energy-peg is an emergent property of *high*
(mainnet) difficulty, not a coded flag; low difficulty makes JUL a test token with **zero consensus
change**. NEVER set `pow_enforced = false` to "save power" — that removes PoW from consensus, breaks
the genesis-admission invariants, and stops the testnet testing the real thing. Honest scope: low
difficulty ⇒ low Sybil-cost / cheap blocks, which is expected and correct FOR a testnet.

Optional UX (app-layer, not consensus, not needed for launch): JUL accrues to the block PRODUCER (the
seed node), not to submitters — a friend who submits earns PoM standing, the seed earns JUL. If friends
should *hold* JUL to play with the money flows, add a testnet faucet or "seed distributes JUL". 🟡.

## 1. Reachable address — pick the tier

| option | durability | cost | when |
|---|---|---|---|
| **cloudflared quick-tunnel** (`go-live.sh`) | dies when the PC sleeps | free | a live DEMO while you're at the machine |
| **free always-on VM** (recommended for a testnet) | 24/7 | free tier | the durable public testnet |

Free always-on hosts that fit a lean std-TCP Rust node:
- **Oracle Cloud Always Free** — ARM Ampere VM (up to 4 vCPU / 24 GB), genuinely always-on. Best fit.
- **fly.io** free allowance / **Google Cloud e2-micro** free tier / any free-tier VPS.
- NOT Vercel/Netlify (static-only — they cannot run a persistent TCP process).

## 2. Stand up the node on the VM (the actual deploy)

Build is a single static-ish binary; the node is std `TcpListener` + threads, no runtime, no DB.

```bash
# on the VM (Linux):
git clone https://github.com/WGlynn/noesis && cd noesis
cargo build --release -p noesis --bin noesisd

# run it: testnet genesis, bound to all interfaces, durable chain log
NOESIS_NET=testnet ./target/release/noesisd --serve-api 0.0.0.0:9955 /var/lib/noesis/chain.log
# open http://<vm-public-ip>:9955/  — the node serves its own UI (one URL = whole app)
```

Make it survive a reboot with a systemd unit (`/etc/systemd/system/noesisd.service`):

```ini
[Unit]
Description=Noesis testnet node
After=network-online.target

[Service]
Environment=NOESIS_NET=testnet
ExecStart=/home/ubuntu/noesis/target/release/noesisd --serve-api 0.0.0.0:9955 /var/lib/noesis/chain.log
Restart=always
RestartSec=3
User=ubuntu

[Install]
WantedBy=multi-user.target
```
`sudo systemctl enable --now noesisd`. Open port 9955 in the VM firewall / security list. A restart
replays the durable log (✅ `store::load_chain`), so the chain resumes instead of resetting to genesis.
For a real hostname + TLS, point a domain at the IP and front it with Caddy/nginx (optional).

## 3. What a stranger gets TODAY (honest)

- ✅ Open one URL → the embedded UI. Submit `{contributor, data}` → the node runs the FULL proven
  per-block engine (`ChainSpec::produce_block`), mines at testnet difficulty, finality-gates, applies,
  persists. Watch height / JUL issued / per-contributor PoM standing update live.
- ✅ Durable: the chain survives node restarts (block-log replay).
- ✅ Node-local ingress screen rejects trivial / near-duplicate spam (advisory, not consensus).

## 4. NOT yet there — the honest gap list (do not round up)

- 🔌 **The 5 on-VM binding activations stay `false`** (`CONTROL_BINDING_ACTIVE` runtime.rs:542;
  `CONTROL_ENFORCED`, `COORDS_BOUND`, `REGISTRY_BINDING_ACTIVE`, `PROVENANCE_BOUND` in `onchain/*`).
  Flipping them IS the go-live milestone — high blast-radius, needs real-entropy keygen + populated
  `auths` + a deploy env + your explicit "we're live" (⚑ **Will-gated, PCP — not a casual flip**).
  Until then the node runs the reference-layer rules (the proven suite), not the on-VM enforced twins.
- 🔬 **Live P2P gossip (slice-5b).** `--serve-api` (the durable live node) and `--listen`/`--connect`
  (the T1 seed/joiner, which serve a *scripted* chain over framed TCP) are still SEPARATE drivers. A
  stranger can join-and-replay a seed's history, but a block produced *after* they joined does not yet
  propagate live. A single hosted seed + the API is a real one-node public testnet; a multi-node
  gossiping testnet is the next build (unify serve-api with net/gossip; wire the gossip reader loop).
- ⚑ **`genesis_bits` is a testnet PLACEHOLDER** (`0x1f00_ffff`) — harder than dev, still CPU-instant,
  NOT a measured value. The measured mainnet difficulty is the standing ⚑ (GPU benchmark at build).
- 🟡 Faucet / seed-distributes-JUL (§0), TLS/hostname, connection-cap hardening for a public seed
  (thread-per-connection is currently unbounded — `noesisd.rs` residual note).
- ✅ **Real key-based wallet identity (Will 2026-07-16, "no theater ever").** The plaintext handle is
  GONE. A wallet is now a real **post-quantum hash-based keypair** generated in the browser: Lamport
  one-time leaves under an **XMSS Merkle root** (`noesis_core::xmss`, height-8 = 256 sigs), address =
  the root. Every contribution signs `contribution_digest(address, index, data)`; the node VERIFIES
  (`rpc::submit_signed`) and credits standing to the address. One-time-leaf anti-replay is enforced
  per address (monotonic index, rebuilt from the chain via `lock.args = address‖index_le`). Browser
  crypto (`frontend/crypto.js`) is **parity-tested byte-for-byte** against the Rust
  (`rpc::tests::xmss_parity_vectors_are_pinned` ↔ `frontend/parity-test.mjs`). Existence ≠ control:
  standing accrues only to a key someone actually holds.
  - **Portable identity across nets** — now POSSIBLE (same key on any `chain_id`). Standing policy is
    unchanged and deliberate: **testnet standing must NOT auto-translate to mainnet** (testnets are
    wiped; carrying test-earned standing invites farming). Carry the **key/address**, start mainnet
    **standing from zero**; any migration is a governance choice, not an accident.
  - 🟡 **Web-wallet 2FA still to build (Will 2026-07-16, "later").** The secret key currently lives in
    `localStorage` with a manual backup-your-seed reveal. Next: protect it with WebAuthn/passkeys
    (device biometric / hardware key) — reuse the proven VibeSwap `useDeviceWallet` pattern. The real
    keypair is the prerequisite and it's now in place.

## 5. The sequence to "friends are on it"

1. ✅ testnet spec + self-serving node + go-live.sh (this session).
2. ⬜ Provision a free always-on VM (your account/credentials — I cannot).
3. ⬜ `cargo build --release`, systemd unit, open the port (steps above).
4. ⬜ Share `http://<ip>:9955/`. Friends submit; you watch the chain grow. **This is a live one-node
   public testnet at the floor** — real consensus mechanism, real durable chain, worthless test JUL.
5. later: multi-node gossip (slice-5b), then — when you say "we're live" — the 5 binding flips.
