# 2026-07-12 — Vesting-`W` Phase 2 shipped, and a self-measuring architect council

**In one line:** the part of Noesis that lets contributions "vote" on finalizing blocks now has a real,
safety-gated production wiring for the first time — and we spun up an experiment where a simulated
world-class review team critiques the whole design and measures its own usefulness.

---

## What shipped today

### The cleared-score finality bridge (vesting-`W` Phase 2)

Some background in plain terms. In Noesis, a block gets *finalized* (made irreversible) by a mix of two
things: **capital** (staked coins, "PoS") and **contribution** (Proof-of-Mind standing, "PoM"). The
whole thesis is the *conjunction*: neither capital alone nor contribution alone can finalize — you need
both. That's the anti-plutocracy property (money can't finalize without the consent of the people doing
the work).

Here's the problem we'd been carrying: the code that turns "how much contribution has this person
earned" into "how much finalization weight do they get" **only existed in test scaffolding**. In the
real production path, that number was never actually wired up. So the beautiful conjunction property was,
in code, half-built.

Today we built the real bridge — and added the safety rule it needed:

- A contribution **can't vote on finalizing its own block right away.** It has to *age* past a vesting
  window `W` first. Think of it like a check clearing at a bank: the value is credited, but it isn't
  *spendable as finality weight* until it clears.
- Why the delay matters: `W` gives the dispute system a window to **catch fraudulent contributions
  before they can ever influence consensus.** Gamed value has to survive `W` of public challenge before
  it counts. This is the launch stand-in for the deeper "un-gameability" guarantee we're still building.
- A contribution that's still "pending" (younger than `W`) **still earns its reward** — it just can't
  vote yet. Reward and finality-power are deliberately split.
- At genesis, nothing has aged yet, so contribution weight is zero and **staked capital carries
  finality from block zero** — the system bootstraps cleanly with no special-case code.

There's a small piece of math I'm proud of: because contributions are stamped in order and the scoring
is a left-to-right scan, the "cleared" set is always a clean prefix of history. That means a contributor's
cleared score is *exactly* their normal score, just with the not-yet-aged part held back — no weird
re-scoring surprises. Clean by construction.

**Status:** built and committed (`ff8a0f6`), node test suite green at **284** (up from 281 — three new
tests covering the aging cliff, the genesis bootstrap, and the "off by default" behavior). Zero
regressions. The window `W` defaults to zero (feature dormant) until governance turns it on, so nothing
existing changed.

**What's left on this thread:** Phase 3 — letting a dispute that lands *during* the `W` window remove a
contribution before it ever clears into finality weight. That's the last consensus-touching piece, and
it's the one that makes the whole bridge provably non-circular. Building it cold in a fresh window.

**Honest caveat:** `W` is a *stand-in*, not the real moat. The real moat — making contribution value
genuinely un-gameable via a learned model on real data — is still open and data-blocked. We don't round
that up.

---

## The experiment: a council that reviews the design and grades itself

Separately, we tried something new. Will's idea: instead of one reviewer, simulate a **whole team of the
best minds** critiquing the Noesis design — Satoshi, Vitalik, Gavin Wood (Polkadot), Jan Xie (Nervos),
Will himself, a cryptography red-team, a BFT safety engineer — plus, deliberately, people *outside*
crypto: a physicist (Feynman), a logician (Gödel), a game theorist, a mechanism-design economist, a
philosopher of value, a complexity scientist, and an adversarial MEV attacker.

The twist is that it **measures whether the roleplay actually works.** Every finding gets independently
fact-checked against the real code (is this a genuine problem, or does the design already handle it?).
And we included a plain "no-persona" reviewer as a control, so we can tell whether *pretending to be
Feynman* actually surfaces things a normal review wouldn't.

To decide which council members earn their seat, we use **Shapley attribution** — the same fair-credit
math Noesis itself is built on. Each real problem's credit is split among whoever found it, so a member
who finds *unique, important* issues scores high, and one who just piles onto what everyone already saw
scores low. That gives us a principled way to **keep the valuable voices and prune the rest** — and to
grow the council over time.

This run is **iteration 1** (my domain — the technical and hard-science council). Its results seed
**iteration 2**, which Will wants to open up to *humanity*, not just blockchain experts: historians,
biologists, ethicists, ordinary people the system is meant to serve. The council improving itself, round
over round, is its own recursive loop.

**How it actually went (the honest part).** The first council was built too big — 14 personas over two
rounds, which multiplies into ~170 agents. It burned a large chunk of the day's token budget in half an
hour before we caught it and killed it. That was a real, expensive mistake. But it wasn't wasted: we
salvaged 13 code-grounded findings from the partial run, and — the important part — computed the Shapley
attribution retroactively (it's just cheap arithmetic, which is exactly why stopping before it ran was the
avoidable loss). The result was honest and a little humbling: **Jan Xie (the Nervos lens) earned 42% of the
credit** because Nervos is the actual substrate Noesis forks, while the abstract outsider lenses (physicist,
philosopher) scored zero — their findings *sounded* sharp but didn't survive verification. The no-persona
control also scored zero.

Out of that we banked three durable things: a **Council Protocol v1** (size lean, do the multiplication
before launching, hard budget ceiling, Shapley is mandatory and runs even on a kill), a **persistent roster**
that adds/prunes seats by measured Shapley credit, and a sharper rule Will named — *don't trust verify* (an
LLM checking an LLM isn't verification; real confirmation is a deterministic check against the code). Applied
that rule immediately: the "easy" findings mostly evaporated on inspection (the top one, a claimed doc drift,
was a false positive — the doc was accurate). The salvaged findings live at
`~/Desktop/noesis-council-salvage-2026-07-12.md`.

The most expensive mistakes are lessons in disguise, and this one bought the protocol.

## Afternoon — acting on the council's findings (lean, verified, one real build)

After the council, we turned its 13 findings into real work — but carefully, applying *don't trust
verify*: each finding gets a deterministic check against the actual code before we believe it.

**Built one thing (B — accountable safety).** The council's scariest finding was real: Noesis could
detect a validator double-voting (`is_equivocation`) and could slash, but neither was wired to the live
finality path — the code even labelled it a `[GAP]`. We built the missing mechanism: an equivocation
*guard* that strips a double-voter's weight from the count entirely **before** it's tallied
(slash-before-count) and reports them for slashing. It's deterministic (so every node agrees) and tested
(node suite 285 green). Honest caveat: this builds and proves the mechanism, but it isn't yet the live
finalize entry point — wiring it in needs per-epoch vote tracking, which is a cold-window job. So the gap
is now *closable*, not *closed*. Committed as `5f76fa4`.

**Verified four findings, changed zero in a panic.** Every one turned out real *and* already tracked as
open in our own docs — no hidden surprises:
- Accountable safety → real, self-labelled `[GAP]` (now has a built mechanism).
- Deployed franchise is plain novelty, not the v5–v8 moat → real, already noted in CONTINUE.md.
- Anti-concentration floor at 50% → real observation but by design (the floor is for participation; the
  2/3 bar does intersection). Decided **keep `MIN_DIM_BPS` at 50%** — reaffirms ruling D5 (tune on data,
  not raise blind).
- MEV last-revealer grinding → the powerful half (secret-grinding) is already closed because our shuffle
  *is* VibeSwap's DeterministicShuffle ported; the residual 1-bit withhold is the classic RANDAO issue,
  closable by the commit-deposit ("Bound B") we already designed — i.e. lean on VibeSwap's slash-on-non-
  reveal. Designed-not-built.

**Process:** Story Mode is on (menu-driven, steer by number). Phase 3 (A) and B's live-wiring are both
teed cold in `CONTINUE.md`. The council experiment, for all its cost, did its job: it found real gaps,
and our honesty discipline had already named every one.

---

## Why today mattered

Two things moved. The **thesis got more honest in code** — the anti-plutocracy conjunction now has a
real production bridge instead of a test-only stub, with a safety window that buys time for disputes.
And we started building a **way to pressure-test the whole design against the best critics we can
simulate, and to prove whether that method is worth doing** — measured, not assumed.
