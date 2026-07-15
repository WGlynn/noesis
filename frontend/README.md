# Noēsis frontend

A single self-contained page (`index.html`, no build step, no framework) that talks to a running node's
HTTP API (`node/src/rpc.rs`): submit a contribution, watch the chain's height, JUL issuance, and the
per-contributor PoM standing update live.

## Run it (two terminals)

```bash
# 1. the node's live API (from the repo root)
cargo run -p noesis --bin noesisd -- --serve-api 127.0.0.1:9955

# 2. serve the page (any static server works)
cd frontend && python -m http.server 8088
#   then open http://127.0.0.1:8088
```

Or just open `index.html` directly in a browser — the API sends permissive CORS, so `file://` works too.
The node address is editable in the page footer (default `http://127.0.0.1:9955`), so a friend can point
their browser at your node.

## What it shows

- **Contribute** — a handle + a contribution; on submit the node builds a cell, mines + finalizes a
  block, and returns the new state. Genuinely novel text earns standing; near-duplicates of already
  committed work score zero (the θ_sim novelty floor).
- **The chain** — height, JUL issued (energy-anchored, minted from mined work, no pre-mine), and a
  live leaderboard of PoM standing (soulbound, earned by finalized contribution).

## Honest scope

Local devnet: the mining difficulty is a low placeholder for instant blocks, one block per submitted
contribution, no auth or rate limiting. Multi-node live gossip and the hardening a public node needs are
not here yet — this is the invite-a-few-friends tier.
