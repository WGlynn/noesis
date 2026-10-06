# 2026-07-15 — The chain came alive locally, and the "we need a Linux box" wall fell for free

Plain-English recap of a long morning session. Several things landed; here they are in order, newest last.

## 1. The money ruling got written into the docs (and a stale hallucination pulled out)
You ran an economics sitting first thing and ratified how JUL's value works. I reconciled the docs to
match it: the Moore's-law decay is now marked as CORE to the energy peg (not "deferred"), the coinbase
infra-tax is NONE (funded by contribution instead), and a made-up "~2.3 year half-life" number that had
crept into an old design note was withdrawn. No code, just honesty cleanup so nobody builds against the
wrong story.

## 2. Built the Moore's-law decay (the thing that keeps JUL pegged to energy, not to hashes)
A fixed reward per unit of mining silently inflates over time, because hardware gets cheaper per hash
every year. The fix is a decay term that halves the coins-per-hash each time hardware efficiency doubles,
so coins-per-ENERGY stays flat. I built and tested that function (it reuses the same power-of-two math the
difficulty retarget already uses, so there's one shared piece of math, not two that can drift). It ships
switched OFF by default — turning it on with a real rate is a later, deliberate step.

## 3. Produced our first real zero-knowledge receipt — on infrastructure that cost nothing
The fairness check (capital can't finalize without contribution) used to run off to the side and say
"trust me, I ran it." Now the exact same code runs inside a zero-knowledge VM and emits a cryptographic
proof anyone can verify. The catch was that the prover only runs on Linux and this is a Windows box with
no budget for a server. The unlock: the public GitHub repo already comes with free Linux machines (the CI
runners). So the repo IS the Linux box. It proved all three test cases on the first try. That flipped the
proof-of-concept from "designed" to "done."

## 4. The chain now BOOTS its real economics and mines locally (this session's headline)
Until today, the local node driver booted a money-INERT genesis (proof-of-work turned off, no coins
issued). I lifted the genesis definition into a proper `ChainSpec` object — one shared definition of
"what block zero is" that a devnet, a seed, and a joiner all read identically — and turned the ratified
economics ON in it. Now running the node locally actually mines each block and issues JUL from block zero:

```
block 1: FINALIZED  work=256  jul=256   pom[alice=27 bob=27]
block 2: FINALIZED  work=512  jul=512   pom[alice=52 bob=27]
block 3: FINALIZED  work=768  jul=768   pom[bob=62 alice=52 carol=40]
block 4: FINALIZED  work=1024 jul=1024  pom[alice=97 bob=62 carol=40]
JUL issued from mined work (energy-anchored, no pre-mine): 1024 JUL over 4 blocks.
```

Mining is real (grind a nonce until the block's hash meets the difficulty target — the same check the
validator enforces), finality is gated every block, PoM standing is earned not pre-minted, and the
two-process join test still converges (a joiner validates the seed's mined blocks and reaches identical
state). Honest scope: the difficulty is a LOW placeholder for instant local mining, not a measured
mainnet number, and the Moore's decay from item 2 isn't wired into the mint yet (JUL issues flat here).

## Why this matters for the plan
You set the north star: a friends-ready node + frontend BEFORE any public seed. This is step 1 of that —
the chain now visibly runs its real economics on your own machine. Next toward "invite friends": a node
that accepts friends' actual contributions (not a hardcoded script) with live block propagation, then a
frontend to submit + watch, then the free public seed.

Commits: 461e845 (docs reconcile) · 7ba3e9d (Moore's decay) · 0476e60 (L2/L3 stub) · c480f39 (zk CI) ·
1e95ba8 (Fit-1 done) · 34853fa (chainspec + live devnet).
