# DESIGN — Elastic (Ergon-style) Proof-of-Work money layer

> SPEC tick, 2026-06-20 (Will: "it's supposed to be elastic like Ergon per the newer
> whitepaper"). Design-first, no code (Will's call). PRIVATE.

## Why this note (frame correction)
A stale frame was in play: the older `RESEARCH-NETWORK-CONSENSUS.md` (T3) treats PoW as a
near-vestigial sybil-cost dimension "removed from finality." The **v3.2 whitepaper** is canonical
and richer: PoW is a **proportional, Ergon-style energy-money layer** — one of three orthogonal
powers — and the whitepaper itself states it is *"designed, not yet built; the reference stub's
current issuance curve is a placeholder to be replaced with the proportional rule."*

So the honest answer to "did we fix PoW?": **no.** The elastic issuance rule was never built —
there is nothing yet to fix on the money side, only to *build*. The recent cron work was entirely
in the PoM value-attribution layer (the Hodge collusion detector) and never touched PoW.

## Canonical intent (whitepaper v3.2, §"The compute layer is honest money, not waste")
Three orthogonal powers, rock–paper–scissors (minimum for non-domination):
- **PoM (cognition)** — scarce, inelastic, **unbuyable** standing; majority consensus weight;
  governs value. Mints on novel verified contribution; state-rent decay is the sink.
- **Energy money (compute / PoW)** — **elastic**, meant to be *spent* not hoarded; proportional
  Ergon-style issuance; buys two things at once: sybil-resistant security + a stable unit of
  account (an "energy oracle," pricing electricity into money). It is the **liveness floor**
  producer; **earns no PoM, mints no standing, never a value authority**.
- **Capital (PoS)** — stakeholders' stake and voice; conventional role unchanged.

Whitepaper invariants to preserve: *"cognition governs and does not inflate; energy circulates and
does not vote; capital is the stake of the stakeholders."*

## Ground truth (code, verified 2026-06-20)
Two distinct "PoW" surfaces — **do not conflate**:

- **(A) PoW MONEY issuance curve** (reward vs work) — the "elastic like Ergon" thing.
  **UNBUILT.** `grep` over `node/src/` finds NO issuance / emission / energy-money module. The
  only "mint" in code is the ERC token-conservation analogs (`tokens.rs`) and the PoM standing
  mint. Confirms the whitepaper's "designed, not yet built."
- **(B) PoW CONSENSUS-dimension scaling** (a miner's `pow` weight → vote weight). The `Validator`
  carries `pow: f64`, **linear** in `consensus::base_weight` / `effective_weight` (mix 0.10).
  `log_weight` (log₂) fires ONLY in the `realizable_log_share*` analysis helpers + one test —
  **not** in the live weight path. ⇒ the log₂→linear/proportional cleanup decided in
  `CONTINUE.md` (2) is already effectively true in the live consensus path; only dead analysis
  helpers (and possibly the NCI Solidity contract) may still carry log₂.
- **Finality wiring:** `runtime::finality::finalizes_pos_pom` (PoW removed from the finality mix,
  T3 fix) exists **but is not wired into the live path** — `runtime.rs:542`'s `finalizes` wrapper
  still calls `consensus::finalizes_hybrid` with `pow = 0.10` in the sum. So at the reference
  layer PoW is **still counted in finality** (the latent T3 safety bug), and the fix is stranded
  like `log_weight` was. Separate from the money layer; flagged in D4.

## Ergon mechanism (from ergon.moe text; exact constants UNVERIFIED — source PDF is binary)
- **Proportional reward:** block reward is proportional to the work/difficulty solved. "Earning a
  single unit of the currency takes a fixed amount of effort" ⇒ `R_block = k(t) · D_block`.
- **Energy peg:** fixed effort per coin ⇒ price gravitates to the marginal cost of energy (Satoshi:
  "price of any commodity tends to gravitate toward production cost") ⇒ stable purchasing power.
- **Demand-elastic supply (the self-correcting loop):** demand↑ → price↑ → mining more profitable
  → hashrate↑ → difficulty `D`↑ → reward `R`↑ (proportional) → **supply↑** → price↓ back toward
  energy cost. Bitcoin's fixed halving is *inelastic* and cannot self-correct; this can.
- **Decay:** the proportionality constant `k(t)` decays with a ~**2.3-year half-life**; total
  issuance is market-driven (constant hashrate growth ⇒ ~constant supply; plateau ⇒ reward → 0).
- **Damping:** ergon.moe's charts contrast `price_damping`/`hash_damping` vs `_no_damping` ⇒ a
  control term that smooths the proportional arms-race oscillation. **This damping is issuance
  control-theory, NOT a concavity on vote weight — do not conflate with the consensus log₂.**

## Design for Noesis

### D1 — Money lives in its own cell, never commingled with standing
Mining a block mints **energy-money** to the miner's freely-transferable capacity/money cell (rides
the `ownership` fold). It mints **no PoM** and touches no soulbound standing cell — value flow must
never manufacture consensus weight (the same firewall the `token_cells` separation already enforces
for ERC value). This is the **JUL money layer** (`[[jul-is-primary-liquidity]]`: JUL = PoW-objective
money; CKB-native = state-rent capital; VIBE = governance — three orthogonal roles).

### D2 — Issuance rule (the placeholder's replacement)
`R_block = k(t) · D_block`, `D_block` = realized proof-of-work (target met). Energy peg holds by
construction (fixed marginal work per coin). **Open:** `k(t)` decay schedule — Ergon's 2.3yr
half-life as-is, or a Noesis-native φ-geometric schedule per `[[substrate-geometry-match]]`.

### D3 — Damping (recommend INCLUDE, as a hardening tick after pure-proportional)
Add the Ergon damping control term to suppress price/hashrate oscillation. Ship pure-proportional
v1 first (provable energy peg), add damping as a second tick. Pin the "this is NOT the consensus
log₂" distinction in the code/doc to prevent the conflation that started this thread.

### D4 — Finality reconciliation (close the stranded-fix gap)
PoW is the liveness floor + money oracle, *"never a value authority."* The probabilistic layer must
stay OUT of fast finality (T3 safety). Wire `finalizes_pos_pom` as the live finality rule (PoW
removed from the finality sum, PoS+PoM only, with the anti-concentration floor). **Open:** in scope
for this PoW work, or a separate consensus tick?

### D5 — Consensus-dimension scaling
Keep the PoW vote-dimension **linear** (already is). PoM → **linear** (already is). Remove the dead
log₂ analysis helpers if they no longer serve (`CONTINUE.md` (2)). No new PoS damping (capital is
at-risk/slashable and bounded by needing unbuyable PoM for any supermajority).

## Open questions for Will (the design-first gate)
1. **Decay `k(t)`:** Ergon's proven 2.3yr half-life, or a Noesis-native φ-geometric schedule?
2. **Damping:** in v1, or ship pure-proportional first and add damping as a hardening tick?
3. **Finality (D4):** reconcile T3 (wire `finalizes_pos_pom`) as part of this, or a separate tick?
4. **Money ↔ capital:** is the energy-money (JUL) a distinct asset from CKB-native state-rent
   capacity, or the same capacity bytes? (Memory says distinct — confirm.)
5. **The "does not vote" tension:** the whitepaper says *energy circulates and does not vote*, yet
   the `Validator` carries a `pow = 0.10` consensus weight. Is that 0.10 the **liveness-floor
   producer** role (block production / ordering, no value authority) — i.e. "votes for liveness,
   not for value/governance"? Or should the energy-money holder have **zero** consensus weight and
   the `pow` dimension be reinterpreted purely as the floor producer's sybil-cost? This is the one
   genuine internal contradiction between the code and the whitepaper; it needs a ruling.

## Build contract (deferred — design-first; nothing built this tick)
When approved: (1) money/issuance module with `R = k(t)·D`, freely-transferable money cell, no PoM
mint, firewall-tested (a mined block leaves `pom_scores` byte-identical); (2) energy-peg test (fixed
work per unit); (3) demand-elastic test (supply rises with simulated demand/hashrate); (4) optional
damping tick; (5) D4 finality reconciliation if in scope. node unchanged this tick (spec only).

## Honest residuals
- Ergon's exact `k(t)` form, the 2.3yr constant, and the damping equation are **reconstructed** from
  the ergon.moe text + general knowledge — the source `prop-reward.pdf` is binary and did not parse.
  Confirm against a rendered copy before locking constants.
- This is a SPEC tick. No code, no test, no count change.
