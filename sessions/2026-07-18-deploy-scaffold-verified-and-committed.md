# 2026-07-18 — The public-testnet deploy is now turnkey (reconcile + verify + land)

Plain-English recap. Short session: no new mechanism, but the single biggest thing standing between
"code-complete" and "friends can reach it" got de-risked and committed.

## What I walked into
The handoff said "deploy the durable public testnet — code-complete, HEAD is `2a8dc24`." That pointer
was stale. Real HEAD was `bdba954`, and a prior session on **July 16** had quietly done a lot:
- Built a complete fly.io deploy path — a `Dockerfile`, a `fly.toml`, and a `fly-deploy.sh` script —
  but never committed them (they were just sitting loose in the folder) and never wrote a recap.
- Actually ran the node on your machine and produced a real 4-block chain (`noesis-chain.log`): real
  proof-of-work nonces, difficulty that retargeted harder, real signed contributions. The economics
  genuinely run.

So the work existed but was invisible to git and to the next session — a loose end.

## What I did
1. **Reconciled the real state** (the standing "check HEAD before quoting status" rule). Confirmed the
   node source hasn't changed since that Jul-16 binary, the frontend the Docker build needs is present,
   and the `.dockerignore` correctly keeps it.
2. **Verified it actually works, end to end** — not just "it compiles." Rebuilt the node (clean, 3.4s),
   ran it, and hit the two URLs fly's health check will hit:
   - `GET /state` → real JSON: testnet chain id, proof-of-work ON, JUL issuing, fresh genesis.
   - `GET /` → the embedded wallet web page. One URL = the whole app, as designed.
   That means when you deploy, fly's health check will pass on the first try instead of you finding out
   the hard way.
3. **Committed the deploy scaffold** (`281edbb`, pushed to the public repo) so it's turnkey and no longer
   invisible. The commit is just the four deploy files, WGlynn-authored, nothing swept in.

## What's left — and it needs you (one step)
The deploy itself needs your fly.io login; I can't do that part. It is now literally two commands:

```
fly auth login          # choose "Continue with GitHub"
./scripts/fly-deploy.sh # creates the app + durable volume, deploys, and verifies /state answers
```

The script refuses if you're not logged in, picks a free app name if `noesis-testnet` is taken, creates
the persistent chain volume once, and — importantly — confirms the node actually answers before it
prints "LIVE" (it never trusts the deploy exit code alone). When it succeeds it prints the public URL to
share with friends.

Honest scope (unchanged, not blockers for this single-node deploy): live peer-to-peer block gossip
between full nodes is still a separate piece (this deploy exposes the HTTP wallet/API only), and the
five on-VM enforcement "flips" remain your deliberate go-live call. Neither stops friends from reaching
the node and contributing today.

Commit: `281edbb` (deploy: fly.io turnkey path for the durable public testnet).
