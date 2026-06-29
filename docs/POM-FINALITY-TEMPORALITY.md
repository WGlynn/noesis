# PoM, finality, and the temporality problem

**The hardest fair question about Noesis:** a contribution's value depends on *downstream*
work that hasn't happened yet (who builds on it, what flow it generates), so a Proof-of-Mind
score can change after the fact. Consensus weight in PoW/PoS is *instantaneous* — knowable in
the block it applies to. How can a retroactive measure carry consensus weight?

This doc is the reconciliation, after an adversarial coherence audit (a consensus mechanism is
pass/fail on closed-loop completeness, not on any one clever part). Status: **design** — the
model below is coherent and complete; one decision remains open (last section).

## The keystone: no circular self-reference
The check a new consensus mechanism lives or dies on: *what finalizes the block that contains the
contributions whose value isn't known yet?*

**Answer: the pre-existing franchise** — bonded stake + already-vested prior standing. A block is
finalized by the franchise that existed *before* it; its contributions earn *future* franchise and
never vote on their own block. No value is ever an input to its own finalization. **This is the
load-bearing invariant; everything below serves it.**

## PoM has two faces, on two clocks

| Face | Asks | Clock | Known at block `t`? | Gameable? |
|---|---|---|---|---|
| **Instantaneous — temporal novelty** | novel vs everything committed *before* it? | backward | **yes**, fixed at `t` | **yes** (coverage proxy) |
| **Retroactive — realized value** | did others build on it? what flow? | forward | **no**, accrues over time | **hard** (the moat) |

Note the bind this exposes: the face usable *at finality-time* is the **gameable** one; the face
that is *hard to game* is the one you **cannot** use instantly.

## The reconciliation: standing is a stake-like account that clears in arrears

> **Standing is a running account, exactly like a PoS stake balance. Realized-value is *deposits*
> into it that clear after a vesting window `W` (like a check clearing). Finality at block `t` reads
> the *current cleared balance* at `t` — never a future value, never a past one. Past finalized
> blocks are immutable. The arrow only ever points forward.**

This dissolves the temporality problem cleanly:
- **"Scores change after the fact"** → only as *deposits into the forward account*. Like revenue
  settling into accounts-receivable; you never retro-edit a past ledger entry.
- **"Finality is instantaneous like PoW/PoS"** → yes: it reads the balance at `t`, just as PoS reads
  stake at `t`. Stake changes over time too; finality at `t` uses stake-at-`t`; past blocks aren't
  rewritten. Identical safety argument.
- **No infinite-horizon dependency** → you never need a contribution's "true" lifetime value (which
  never settles — someone could build on it years later). You only ever need *the account balance at
  each moment of use*, which is always finite and defined.

### The vesting window `W` is the one load-bearing parameter
`W` is the deposit-clearing delay. Its job: let disputes catch fraudulent value **before** it clears
into usable franchise. The tradeoff is the whole design tension — **long `W`** = fraud caught before
use (safer); **short `W`** = franchise tracks reality (more responsive). This is the same knob as the
v(S)-refresh cadence flagged in the loop-engineering research. Designed, not yet set.

### Slashing is forward-only (same as PoS)
If a validator finalized block `t` with standing `S` and a later dispute slashes part of `S`, block
`t` stays final (the quorum at `t` existed); the validator loses *future* franchise. `W` exists to
minimize the chance fraudulent standing is used before it's caught. No retroactive un-finalization.

## What the audit forced (two independent results, same conclusion)
1. **The usable face is the gameable face.** Only the instantaneous novelty proxy is available at
   finality-time, and it is the gameable one. A gameable quantity must not carry *safety* weight.
2. **Genesis bootstrap.** PoM franchise needs finalized blocks; finalized blocks need franchise. At
   genesis there is no vested PoM — so **bonded PoS must bootstrap finality**, with PoM franchise
   phasing in as it vests. PoS is doing the safety work from block zero regardless.

Both point the same way:

> **Safety = bonded stake** (instantaneous *and* ungameable-at-`t`). **Influence + reward = PoM**, a
> stake-like account corrected in arrears by the retroactive face. **The gameable instant-novelty
> face never sits on the safety path.** "Consensus follows the minds, not the money" is preserved —
> PoM decides *whose voice counts and who earns*; bonded stake decides *that the chain stays safe*.

This is a complete closed loop: no self-reference (keystone), no infinite-horizon settlement
(account framing), no gameable input on the safety path (face split), and it bootstraps (PoS-first).

## The one open decision (Will's call to ratify)
**Should PoM weight finality *safety* at all, or only influence/reward + governance?** The audit
*recommends decoupling PoM from finality safety* — both independent results above land there, and it
removes the gameability↔rollback coupling entirely. The current *built* design keeps PoM in finality
with an anti-concentration hedge.

- **If ratified (decouple):** `finalizes_pos_pom` changes so PoS carries the BFT safety threshold and
  PoM weights influence *within* the bonded set; update [SECURITY.md](../SECURITY.md) §4 and the
  NCI-mix framing.
- **Until ratified:** the built design stands. This reconciliation holds *either way* — whatever
  weights finality must be the *settled/cleared account balance*, never the live retroactive score.
