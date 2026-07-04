# Noesis FAQ

Short, shareable answers to real questions people ask. Grounded in the actual
design (file:line where it matters), not marketing.

---

## Q: If another chain dies, does that affect the value on Noesis?

Opposite, actually. It is a strict upside, and it comes from two facts that stack.

**1. Nothing bad can flow in.** A contribution's PoM value is a pure function of
the contribution itself. The scoring inputs are self-contained, with no external
oracle and no cross-chain price or state: `value::features()` is
`[ln(1 + data_len), ln(1 + coverage_size), provenance]`,
"no external oracle, only what the type-script can see" (`node/src/lib.rs:860`).
There is no channel through which another chain's death can propagate into a score.
On top of that, Noesis produces the signal and other chains consume it, so contagion
cannot flow upstream from a consumer to the producer (the Chainlink / restaking-layer
shape).

**2. Value flows in and is preserved.** Noesis does not compete with a chain over
who-holds-what. It scores who *contributed* what, a different axis (every normal chain
is a possession chain; Noesis is a contribution chain, see `docs/COMPETITIVE-POSITION.md`).
So the contribution value of a dead ecosystem does not vanish: the work, the code, the
research, the provenance are all still scorable, because scoring depends on the
contribution and its graph, not on the chain being alive.

**Be precise about which value.** What is preserved is the contribution / mind value
(who built what and why it mattered), not the possession / monetary value (token
balances, TVL). Noesis does not reconstitute your dead-chain token holdings. The precise
claim is the stronger one, and it is the whole thesis: the value that survives a chain's
death is the mind, not the money. Possession value was always the ephemeral part;
contribution value is the conserved quantity.

**What "lossless" means here.** No contribution is ever lost or double-counted, because
value is soulbound to the contributor (`type_script.args`, set at mint and never
reassigned, `node/src/lib.rs:50`) and strategyproof (redundant work scores zero). It is
lossless in the *provenance* sense: the ledger of who-contributed-what does not evaporate
when a host chain does. Not a perpetual-motion claim.

**One honest caveat.** For the current ETH deployment, that specific deployment shares
ETH's fate like any app on it. But the scores are recomputable and portable, so the
value-*definition* survives and can redeploy on any chain. That is substrate risk, not
contagion, and it is exactly why a sovereign chain is the endgame.
