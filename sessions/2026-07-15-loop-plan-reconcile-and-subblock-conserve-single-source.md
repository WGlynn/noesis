# 2026-07-15 (PM) — The map caught up to the territory, and a fast-tier loophole got closed

Plain-English recap. Two shipped units. HEAD == origin == `0ae5a1a`.

## 1. Reconciled the "path to go-live" doc to reality (`4a701c8`)

`LOOP-PLAN-to-golive.md` — the doc that turns "I don't know when it'll launch" into "here are the
countable steps" — was written 2026-07-13 and had gone stale: 40 commits landed since, quietly
finishing almost everything it listed as "remaining." I updated it to the honest current picture.

The short version it now tells: **the reference node and the money layer's whole mechanism are built and
green** (proof-of-work with real difficulty, difficulty retarget, the chain's clock and its enforcement,
the never-halt safeguard, the commit-deposit anti-spam, governance tiering, and all four on-chain
enforcement programs). What stands between here and a public testnet is **no longer building** — it's the
*deploy pole*: turning those enforcement programs on against a real chain (five "binding" switches still
off by design), wiring genesis + peer-to-peer into an actual network, one Linux box for a
zero-knowledge receipt this Windows machine can't produce, and a short sitting to set the economic
numbers (Will's call). The un-gameability moat stays honest open research we won't date. The old
"~10 loops" / "~4-5 loops" counts are marked superseded.

## 2. Closed a fast-tier / settlement drift in the sub-block code (`0ae5a1a`)

While surveying, I found a real (if not-yet-live) bug. Noesis has two speeds: slow **settlement** blocks
(final) and fast **sub-blocks** (optimistic, revertible soft-confirmations). Both check that a
transaction is valid — but the two checks had **drifted apart**. The settlement check refuses to let JUL
(the money) be created out of thin air and refuses transactions that squat a reserved "coinbase" id. The
sub-block check was missing **both** of those.

The consequence: the fast tier would happily *soft-confirm* a payment that the settlement tier is
guaranteed to reject — a green checkmark for a transaction that will definitely bounce. That could
mislead a merchant. (Not exploitable today: sub-blocks aren't wired into the live network yet — they're a
tested shadow. But it's a real correctness gap, and exactly the kind of "two copies of one rule quietly
diverge" bug that bites later.)

Fix: I merged the two into **one** shared rule that both tiers now call, so they can't drift again
(this is Will's standing "single-source it, keep the code lean" design law). I wrote the failing test
first — it proved the fast tier accepted a mint-from-nothing transaction — then made it pass. The
settlement side is byte-for-byte unchanged (the parity tests confirm it); the fast tier just got the two
checks it was missing. 328 core tests green, all parity/sub-block/JUL suites green.

## What's next
Remaining hardening is now smaller: the *other* half of the single-source (two commit/hash helpers that
are byte-identical duplicates — pure hygiene, zero behavior change, I left it as a clean follow-up rather
than touch the consensus-critical header path late in a session) + the sub-block networking half +
equivocation-slashing wiring. None are launch gates. The real remaining pole is unchanged: the deploy
pole. North star: get online.
