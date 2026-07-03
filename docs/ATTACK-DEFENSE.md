# Noesis — Attack / Defense (partner-facing)

Adversarial self-review, 2026-07-01. Every claim grounded to `file:line`; every residual named.
Status discipline: ✅ built (reference layer, tested) · 🟡 designed-not-built · 🔬 open frontier.

## The frame: structural security bottoms out at the Nakamoto bound

When security is a bolt-on (a check, a threshold, a patch), scrutiny finds the gap the patch
didn't cover and it gets *weaker* the harder you look. When security is a *structural property* —
honesty made the profitable strategy, authority made a claim on contribution, capture made to
require a majority — there is no gap to find, so every attack reduces to the **same irreducible
floor**: a majority of the *un-buyable* base defecting. The three attacks below each collapse to
that floor. Noesis survives them **and** surfaces its own residuals, which is the posture a serious
reviewer reads as real rather than sold.

The load-bearing invariant everything protects (`docs/TOKENOMICS.md:113-119`):

> You can buy the tokens (JUL, VIBE) and buy storage (state-bytes) — but you **cannot buy
> consensus weight**. Weight is PoM-standing: soulbound and unpurchasable.

---

## Attack 1 — "I'll sell (or lend) my key"

**The attack.** Soulbound blocks on-chain *transfer of the token*, but nothing stops me handing my
private key to a friend or a buyer. Isn't the soulbound property just theatre?

**Why it fails structurally.** Soulbound turns a clean market into a lemon market:
- **all-or-nothing** — it is your *entire* identity; you cannot split or rent a slice.
- **no clean title** — the buyer cannot verify you kept no copy of the key; you cannot claw it
  back. Illiquid, trust-based, un-auditable — the opposite of a transferable token's clean transfer.
- **the buyer inherits the bond + slash risk** — they buy a position that is *slashed* the moment
  it is used dishonestly (dispute-slashing), not free authority.
- **the seller permanently exits** the franchise they spent real contribution to earn.
- **deepest:** standing is a claim on *ongoing contribution*, not a static asset. Buying the key
  buys the accumulated stock, **not the ability to contribute** — to keep it valuable the buyer
  must actually be a productive mind. The key is transferable; the mind is not.

To *concentrate* this way you must convince a **majority** of contributors to each irreversibly sell
their whole slash-exposed identity — which is the Nakamoto bound, not a market buy.

**Honest residual (🔬).** A single contributor *can* quietly hand a key to a trusted friend; at small
scale it is undetectable. Soulbound makes it unattractive and non-scalable, not impossible — the same
shape as "you can't stop two miners privately colluding, you make it not worth it."

---

## Attack 2 — "Reusing standing every block: isn't that inflation?"

**The attack.** My standing lets me pick/finalize blocks and get rewarded. I reuse the same standing
every block forever. Isn't that inflationary rent from a one-time contribution?

**Why it fails structurally.** Two different things the word "reuse" blurs:
- **Weight-reuse is not inflation.** Standing is the *weight* the floor + supermajority read each
  block. Voting weight is not consumed by voting — same as stake in PoS or hashpower in PoW.
  Reusing a weight creates **no new supply**.
- **Minting is contribution-backed.** New standing accrues *only* when new contribution is scored
  (`temporal_novelty → pom_scores`; the accumulator accrues from novelty-value, append-only except
  via decay/slash — `node/src/lib.rs:462-473,488`). Sybil / padding / collusion mint **zero**
  (`TOKENOMICS.md:68-70`). Supply grows only as *real contribution* grows — backed 1:1, not debasement.
- **Finalizing does not mint standing.** The reward for the *service* of finalizing lives in the
  money/capital layer (state-bytes / JUL), not in fresh soulbound standing — so consensus
  participation does not inflate the standing supply at all (`TOKENOMICS.md:72-88`).
- **Decay closes the "earn once, rule forever" tail.** Consensus vote-weight *retention-decays* with
  inactivity: `retention(elapsed, horizon)` linear 1→0 from `last_heartbeat`
  (`node/src/lib.rs:3507`), applied by `effective_weight` (`:3516`) — and it decays the **vote, not
  the balance** (`:3473,:3497`). The PoM credit itself also has a rent/decay supply-sink
  (`Op::Decay`, `:473,:488`). Authority you stop earning fades.

**Sharp point.** As-built NCI decays PoW+PoM but not PoS, which *drifts the effective mix toward
capital under staleness*; the **symmetric franchise-decay** fix decays capital too, so capital cannot
win merely by *outlasting* stale contributors (`node/src/lib.rs:3469-3470, 3514-3525`) — the
anti-plutocracy thesis defended at the decay layer.

**Honest residual (🔬/🟡).** "Earn once, collect service fees forever" is a staleness/rent concern, not
inflation; it is dampened by contribution-as-flow and closed by decay. Full consensus is still marked
*designed-not-built* — the decay lives in the tested **reference model**, not yet the on-VM core.

---

## Attack 3 — "Doesn't existing standing inflate away a newcomer's chances?"

**The attack.** Relative to someone who never contributed, doesn't an incumbent's accumulated standing
crowd the newcomer out — rich-get-richer?

**Why it fails structurally.**
- **Standing does not self-compound.** Holding standing yields *zero* new standing — only
  *contribution* does. This is the sharpest break from PoS, where stake earns stake and incumbency is
  mechanical. In Noesis there is **no compounding loop** to inflate a newcomer away with.
- **Earning is absolute, not zero-sum.** The measure prices *the contribution*, not the contributor's
  balance. A newcomer who ships something valuable earns the *same* standing an incumbent would for the
  identical work. They add directly to the numerator; they do not overcome a wall of incumbent supply.
- **Contributing is the only way to hold relative share.** When a newcomer contributes, incumbents'
  relative share falls (they didn't contribute that round) and decay erodes the inactive. Authority
  continuously reallocates toward whoever is *currently* contributing.
- **The floor is about capture, not participation.** A newcomer with small standing still
  participates (weighted); the 50% per-dimension floor (`MIN_DIM_BPS`, `runtime.rs`) blocks *capture*,
  not entry.

**Honest residual (🔬).** (a) *Absolute head-start is real* — a single newcomer contribution is a small
dent against a large incumbent; climbing requires sustained contribution (meritocratic, and
non-permanent thanks to decay). (b) *Cold-start discovery* — a valuable newcomer starting at 0 must be
*seen and scored*; this is exactly what the **finder mechanism** solves (finders surface newcomers'
work, the newcomer earns author standing, the finder earns a transferable cut). The finder flywheel is
the newcomer on-ramp.

---

## Foundational — why PoM is soulbound (non-negotiable)

**"Are we sure?"** Yes — it is the definition, not a knob. `TOKENOMICS.md:25-27`: *"A transferable
[standing] collapses to a stake-for-vote and Noesis degenerates to proof-of-stake — so
non-transferability is load-bearing, not a nicety."* Make PoM transferable and capital buys standing →
the floor is meaningless → you have rebuilt PoS. There is no Noesis to fall back to.

- **No liquidity sacrifice.** Everything transferability is *wanted* for lives in the right layer:
  state-bytes (transferable, minted by standing) and JUL (money, made to circulate). You monetize and
  exit through capital/money; you never sell authority (`TOKENOMICS.md:72-88`).
- **Irreversibility is the guarantee.** A credibly un-buyable franchise *requires* that transferability
  can never be switched on — otherwise capital lobbies to flip it and "unpurchasable" is hollow.
  Optionality here is a weakness, not a hedge.
- **The one honest cost:** soulbound forecloses a market *price* for authority and financializing
  authority (collateral, AMMs). That foreclosure *is* the intent — financialized authority is
  buy-authority through the back door.
- **Delegation is still reachable** without transferability: revocable *vote*-delegation is a separate,
  addable feature, and the code already separates vote-weight from the bound token
  (`effective_weight` decays the vote, not the balance) — so liquid-democracy flexibility never
  requires touching the invariant.

---

## One-line summary for a skeptic

Every attack on Noesis bottoms out at the same floor every consensus shares — a majority of the
*un-buyable* contribution base itself defecting — because security here is a structural property, not a
patch. It survives scrutiny *and* names its own residuals (small-scale key handoff, absolute
head-start, cold-start discovery, consensus still designed-not-built). Soulbound PoM is the axiom the
whole thing rests on; making it transferable would not tweak Noesis, it would delete it.
