# Session recap — 2026-07-14: the committee-attested wall-clock (timestamp source)

Plain-English recap. This was an interactive design session with Will, not a pure autonomous build.

## The one-sentence version

We designed how Noesis will trustlessly know what time it is — the piece that feeds the difficulty
controller built this morning — and shipped the design spec plus the small validation kernel. Commits
`6184ac9` + `e4e35af`, pushed.

## The problem

The difficulty controller (built earlier today) needs to know how much real time has passed. But a
blockchain has no clock — Bitcoin fakes one and pays for the fiction with its whole retarget apparatus.
Will's reframe (from last night's "Bitcoin's missing wall clock" idea): humanity already runs a
decentralized clock (atomic time, NTP, GPS, governed by leap-second votes). The chain should *read* it,
not fake it.

## What we settled (the trust model — Will pushed it hard, four times)

The whole session was Will sharpening the trust model until it was airtight:

1. **The timekeeper must be as trustless as the chain itself.** Answer: the attesting body *is* the
   chain's own bonded validator set — not a new, weaker committee. So the clock inherits exactly the
   chain's own trust, no new assumption added.

2. **Every node is its own witness.** Each node checks the reported time against its own clock. Faking
   network time therefore needs a colluding supermajority — the same bar as a double-spend. And it's
   *unmotivated*: you'd spend a chain takeover just to mistune difficulty (which can't touch safety).

3. **Time is an airgap instance, not a scary external oracle.** Importing "what time is it" is the same
   kind of problem as importing any off-chain fact — and it's the *easy* version (a scalar you can check
   against your neighbors), not the hard one (judging what a contribution is worth). If the honesty
   machinery works at all, it works here first.

4. **The honesty guardrail (Will's sharpest point).** Proof-of-work is the *only* thing a blockchain can
   prove to be objectively true — the proof is self-evident, no outside reference needed. Everything else
   — balances, stake, value, time — is the "oracle problem," settled by incentives, not math. And you
   must never market incentive-truth as cryptographic truth: that was Chainlink's mistake (the signature
   is valid, but whether the number is *true* is still just incentives). So Noesis labels this
   trust-*minimized*, never "trustless" or "cryptographic truth." That claim discipline is now saved as a
   standing principle for the whitepaper and every public statement.

We also scoped where the Pragma partnership could help: it can machine-check that the *rules* which process
time attestations are coherent (all honest nodes reach the same verdict), but it cannot make the external
time reading cryptographically true — an important boundary, honestly drawn.

## What shipped

- **The design spec** (`docs/DESIGN-committee-attested-clock.md`) — ready-for-critique, with the full
  trust model, the optimistic "only gossip on disagreement" mechanism, the safety fence (a compromised
  clock can only mistune difficulty, never break safety), and the two open numbers (the tolerance band and
  the freshness window) flagged as Will's calls.
- **A small tested code kernel** (`node/src/wallclock.rs`) — the pure validation checks (is a reported
  time within tolerance of mine; does it run forward; how much elapsed) with tests. The heavy wiring
  (gossip, challenges, slashing) is deliberately deferred to Phase-2; this pins the semantics first.

## What's next

Will's pick: the never-halt liveness mechanism (now the natural next step — it reuses the same committee),
the Phase-2 wiring of the clock into the chain, or drafting the whitepaper. All the pure design/decision
work on the clock is done and captured.
