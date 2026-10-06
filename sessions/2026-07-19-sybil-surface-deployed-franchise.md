# 2026-07-19 — What's actually enforced on the testnet we're about to ship

Plain-English recap. You said "everything is pointing to Noesis testnet, something is missing — keep
auditing." This is what we found.

## The short version

The testnet as it would ship today is **sound where it claims to be, but the one property Noesis is
about — that standing means genuine contribution — is not enforced on it yet.** The doc-comment said
the testnet "tests the REAL security model." That's only half true, and I corrected it.

## What we checked (and what turned out fine)

The live testnet has exactly one way to write to it: you sign a contribution and POST it. That path is
solid — it checks your signature against a real key, refuses a reused one-time signature, and refuses a
forged identity. There is **no** way to send someone else's coins on the deployed testnet (I chased
that hypothesis first and it's a dead end — the coin-spending code exists but is switched off *and* not
exposed by the web app). So the scary-sounding "anyone can spend anyone's JUL" is **not** reachable on
the deploy. Good.

## What's actually missing

Every contribution earns "PoM standing" (your reputation/voting weight) based on **novelty** — how much
new content it adds that hasn't been seen. The catch: the shipping scorer only knows how to detect
*duplicates* and *near-duplicates*. It has no idea whether content is *worthwhile*. So **random
gibberish scores maximally** — the more random and unlike-anything-else it is, the more standing it
earns. And it costs nothing (no deposit on testnet). A script could farm unlimited reputation with
junk. That is the exact opposite of "proof of mind."

The defenses that would stop this (the semantic floor, the outcome-gate, and ultimately the learned
value model — the moat) either aren't wired into the deployed scorer or can't work on a brand-new chain
with no history yet. And the economic brake (a submission deposit) can't be turned on until the
owner-signature layer is turned on — the same go-live flip we've been deferring. So the two gaps are
really one knot.

## What I did (all reversible, no consensus change)

1. Wrote the full audit: `docs/SYBIL-SURFACE-deployed-franchise-2026-07-19.md` (every claim cites a
   file:line I read).
2. Corrected the testnet doc-comment in `node/src/chainspec.rs`: it tests the real **PoW/issuance**
   model; the **contribution franchise** ships at the floor, not the moat.
3. Committed + pushed to origin (`4c3e726`).

## The one decision left for you

A **private/permissioned** single-node testnet is totally fine to ship now (you're the only one
submitting). A **public, open-to-strangers** one should wait for the go-live knot: turn on the
signature layer, turn on a small deposit, and cap or allowlist standing until the chain has enough real
history for the value model to matter. That flip is yours to make (PCP).
