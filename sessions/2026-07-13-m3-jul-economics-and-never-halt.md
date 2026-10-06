# Session recap — 2026-07-13 PM: M3 JUL economics, the wall-clock reframe, and "never halt"

Plain-English recap of an evening's Noesis work, so the day-to-day progress is legible.

## What we set out to do
Point the session at **M3 — JUL economics** (the money layer): the difficulty retarget, emission,
genesis difficulty, the infrastructure funding split, and the reserve numbers. The posture was
"spec it out and design the plan, but no build" at first — because M3's core numbers are monetary
policy, which are Will's to decide, not something to invent.

## The one hard problem, and how it dissolved
A difficulty retarget needs to sense whether blocks are arriving fast or slow — normally by reading
**block timestamps**. But Noesis deliberately has **no timestamp** and no prev-block hash chain. So
the whole session's crux was: how does a chain that refuses to fake a clock still regulate difficulty?

A design workflow (parallel research + adversarial verification) came back with a confident
recommendation — and the adversarial pass **broke it**, correctly: the proposed "PoS-heartbeat" signal
doesn't exist in the code the way it assumed. The honest conclusion: any signal that can detect the
chain stalling must ultimately come from a **real wall-clock**; the only real choice is *whose* clock.

Then Will surfaced the reframe that reorganized everything: **block time was never a real clock — it's
a faked estimate. Meanwhile humanity already runs a decentralized clock** (atomic clocks → UTC → NTP/GPS,
governed by leap-second votes). A chain shouldn't *invent* time; it should *read* the time the world
already agrees on. So the answer is a **bonded committee that attests a median wall-clock reading** —
sampling the consensus reality already runs on, so no single miner can lie about it. And, beautifully,
Will remembered that **VibeSwap already solved this exact "whose clock" problem** — the committee /
threshold-signature machinery ports straight over. VibeSwap keeps being "the gift that keeps giving."

## What actually got built (and shipped to origin)
Once the design was locked and Will delegated the three blocking numbers to me, we moved from spec to
build:
- **The `target_to_compact` encoder** — the piece every later retarget step needs. Pure, tested against
  real Bitcoin difficulty vectors, shipped (`cd915de`).
- **The N-way coinbase split** — so infrastructure can be funded from issuance without Ethereum's
  "retrofit trap." It conserves money by construction and can never over-mint, and it's invisible until
  turned on. Shipped (`6df23e7`).
- The full **design note** and a research write-up, **"Bitcoin's missing wall clock"**, are on origin
  too (`84e3c85`).

## The two ideas that will outlast the session
1. **Read the clock, don't fake it.** A system should read facts the world already agreed on, not
   rebuild worse versions of them. (Time is the clean example; commit-reveal — "seal your move, then
   reveal it" — is another: a centuries-old social protocol we reach past when we invent fancy trust
   machinery.)
2. **The chain must never halt.** Will's design goal: every major chain has had an "emergency halt"
   moment, and a halt requires a coordinating authority — a capturable center. Noesis should make
   halting *structurally impossible*: failures degrade quality (finality, security, emission-regulation)
   but never stop block production. This is now a hard invariant and a review lens on everything.

## Where it stands
Everything is committed and pushed (`84e3c85`); the tree is clean. One honest open thread: the coinbase
split is consensus code and still owes its adversarial verification pass before it's fully hardened —
first thing next session. Then the work-clock ceiling, the ASERT controller, the committee-clock spec,
and the never-halt liveness mechanism.

Will paused here for a separate mission needing full attention.
