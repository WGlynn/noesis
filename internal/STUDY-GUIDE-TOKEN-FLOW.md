# Study Guide — Noesis token flow, uses, and coherence (Will-facing)

> Purpose: answer any question about the four instruments — what each is, how it's earned, what it does,
> how they interlock, and why. Grounded in `docs/TOKENOMICS.md` + `node/src` (numbers verified 2026-07-14).
> Honest labels: ✅ built (reference layer) · 🟡 designed-not-built · 🔬 open research · ⚑ proposal/undecided.

## TL;DR — the one-paragraph mental model

Noesis separates the functions money systems usually fuse — **money, capital/state, governance** — into
three *tradeable* tokens, and holds the one thing that must never be for sale — **consensus weight** —
completely out of all of them as a *soulbound* franchise you can only earn by contributing. So: you can
buy the money, buy storage, buy governance — but you **cannot buy weight, nor a say in what counts as
contribution.** That single split is the whole design.

## The four instruments (memorize this table)

| Instrument | What it is | Earned by | Transferable? | Role / what it does | Status |
|---|---|---|---|---|---|
| **PoM-standing** | Proof-of-Mind: your accumulated, verified novel contribution | **contributing** (novel value that survives the similarity floor + vesting) | **NO — soulbound** | **consensus weight** + the right to mint state-bytes + governs the *measure* | ✅ ref |
| **state-bytes** | on-chain storage capacity (CKB's insight ported: 1 PoM = 1 byte) | **minted by PoM-standing**, then trades freely | Yes | the **capital / PoS** layer — you stake it to validate; decay = state-rent + supply sink | ✅ ref |
| **JUL** | energy-pegged money (Ergon-style proportional design) | **Proof-of-Work** (mining / energy) | Yes | the **money / medium of exchange**; the PoW consensus layer | 🟡 designed |
| **VIBE** | governance instrument | **validating** (operating consensus) — ⚑ *proposal*, was unspecified | Yes | **governance** — votes on the operational dials; orthogonal to the consensus cycle | 🟡 designed |

**The one that trips everyone (it tripped us):** there are **two different triples that do NOT line up.**
- **3 consensus axes** (how a block finalizes): **PoW / PoS / PoM** = `0.10 / 0.30 / 0.60` (`lib.rs:3820`).
- **3 tradeable tokens** (what you can buy): **JUL / state-bytes / VIBE** = money / capital / governance.
- **+ 1 soulbound franchise** (earned, not bought): **PoM-standing** — backs the PoM axis, is *not* a token.

Old design (superseded): "VIBE = the PoM token, 3 tokens = 3 axes." New design: PoM-standing pulled OUT
as soulbound; VIBE spun out as separate governance. If you catch yourself thinking "VIBE = PoM," that's
the old model — correct it to "VIBE = governance, PoM = the soulbound franchise."

## Token flow (earn → use → govern)

```
  CONTRIBUTE novel value ──► PoM-standing (soulbound)
        │                        │  weight in consensus (60% axis, finality)
        │                        │  right to MINT ──► state-bytes (capital) ──► stake to VALIDATE
        │                        │                                                     │
        │                        └── governs the MEASURE (θ_sim, vesting_W, mix)        │
        │                                                                              ▼
  MINE (energy) ──► JUL (money) ──► medium of exchange, pays for state-rent    VALIDATE ──► VIBE (⚑ proposal)
                                                                                          │
                                                                          governs the DIALS (threshold, quorum…)
```

Read it as: **every instrument is earned by a distinct productive act, and each governs (or is) the layer
it has skin in.** Contribution → standing → (mints) capital. Energy → money. Validation → governance.

## The load-bearing invariants (this is what makes it anti-plutocracy)

1. **Consensus weight is unbuyable.** Weight = soulbound PoM-standing. Money never converts to weight.
   (This is *the* sentence the tokenomics exists to protect.)
2. **A say over the measure is unbuyable.** Amending "what counts as contribution" (θ_sim, vesting_W) or
   the mix requires earned PoM authority, not a bought token (governance tiering, this session).
3. **Capital can never outweigh contribution in the mix.** `pos ≤ pom` is enforced in `check_mix`
   (`amendment.rs`) — governance may retune the NCI mix but can never tilt it toward capital-rule.
   Number-free.
4. **Finality can't be captured by capital alone.** `FINALITY_MIX` excludes PoW and is `pos 1/3 : pom 2/3`
   (`runtime.rs:1195`); `MIN_DIM_BPS = 5000` forces each of PoS and PoM to independently supply ≥50% of
   its dimension (`runtime.rs:1432`) — so neither capital nor contribution can finalize alone.
5. **Amendments can't silently break the rules.** Every amendment passes the axiom-preservation gate
   (`verify_amendment` + the Pragma coherence socket) regardless of who voted — the real backstop.

If someone asks "isn't this gameable by paying?" → *money buys tokens; it never buys weight, the
definition of contribution, or a capital-majority in the mix, and it can't push an amendment that breaks
an axiom. Those four are structural, not policy.*

## Governance — who governs what (the tiering)

| Tier | Governs | Authority | Why |
|---|---|---|---|
| **Physics** | conservation, base value axioms | nobody (near-immutable) | the floor |
| **Tier 1 — the measure** | θ_sim, vesting_W, the mix direction, the dimension set | **PoM-standing** (soulbound) | a say over what counts as contribution must be *earned*, not bought |
| **Tier 2 — the dials** | threshold, quorum, mempool cap, decay, horizon | **VIBE** (buyable stewardship) | operational tuning; bounded + gated, so vote-domination can't reach a bad state |
| **all** | — | must pass the axiom gate | the backstop, whoever votes |

**Sybil resistance for governance is free:** it anchors to soulbound PoM-standing (you can't split a
soulbound identity across wallets), which reduces "can I sybil the vote?" to "can I sybil PoM?" — the
same moat the chain already stakes on. The tiering even makes anti-whale (quadratic/log) voting *optional*
rather than required (Tier-1 is unsplittable soulbound; Tier-2 is outcome-bounded). Residual open problem:
bribery/vote-buying (renting real identities) — hard everywhere, honestly unsolved.

## Coherence — why four instruments, not one (Tinbergen / separation of powers)

Tinbergen's rule: one instrument per policy target. Fusing roles into one token is what creates capture
(PoS fuses "wealth" with "influence"; that's the thing Noesis refuses). So each function gets its own
instrument, and crucially the **earning** is separated too — you can't buy your way into the role, you
perform the activity. Contribution earns standing; energy earns money; validation earns governance;
standing mints capital. The capital→governance path (buy state-bytes → stake → validate → earn VIBE) is
*allowed* only on Tier 2, where the outcome-bound neutralizes it — which is exactly why Tier 1 (the
measure) stays soulbound. The instruments interlock; they don't collapse into each other.

## Q&A drill (likely questions, crisp grounded answers)

- **"What are the tokens?"** → Three tradeable: JUL (money), state-bytes (capital/state), VIBE
  (governance). Plus PoM-standing, the soulbound franchise that carries consensus weight — earned, not a
  token you can buy.
- **"How is PoM earned?"** → By verified novel contribution that survives the similarity floor and the
  vesting window. Soulbound; can't be bought, sold, or transferred.
- **"How is VIBE earned?"** → ⚑ *Proposal (was unspecified):* by validating — operating consensus. Not by
  contribution (that's PoM) and not a pure sale. Mechanism specifics still open.
- **"Doesn't buyable governance = plutocracy?"** → No: VIBE only governs bounded operational dials; the
  measure is soulbound-PoM-governed; the mix can't tilt to capital (`pos ≤ pom`); and every amendment
  passes the axiom gate. Money can't reach the levers that matter.
- **"Why does state exist as its own token?"** → Storage is the scarce resource (CKB's insight). 1 PoM =
  1 byte; minted by standing, then trades; decay is the state-rent and the supply sink.
- **"Is JUL required at launch?"** → Yes. All three NCI axes ship at genesis; you can't launch two and
  fork in the third. PoW is only excluded from the *finality* sub-mix (it's reorgeable), not from launch.
- **"What can't governance do?"** → Touch physics; buy consensus weight; buy a say over the measure; make
  capital outweigh contribution; or pass an axiom-breaking amendment.

## Honest status + what's still open

- ✅ built (reference): PoM-standing = weight, state-bytes mint/decay, the finality mix + anti-concentration
  floor, the amendment gate + authority classification + `pos ≤ pom` bound.
- 🟡 designed-not-built: JUL genesis issuance, the VIBE token, the vote-counting layer.
- 🔬 open research: the un-gameable learned value oracle `v(S)` — the **moat**. This is *not* a parameter;
  don't let "just need the right parameters" absorb it. The measure's un-gameability is the one open bet.
- ⚑ undecided: VIBE's exact issuance mechanism; whether the mix also needs a `pom ≥ pow` bound (separate
  energy-security question — PoW is finality-excluded, so a high `pow` isn't a capital-plutocracy vector).
