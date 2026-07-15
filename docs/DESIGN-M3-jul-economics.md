# DESIGN — M3: JUL economics (the money layer)

> **⚠ SUPERSEDED IN PART by `internal/DECISIONS-M3-money-2026-07-15.md` (Will's ratified economics sitting).**
> Read that file as authoritative where they conflict. The three load-bearing reversals it makes vs the text
> below: (1) **Moore's-law decay is CORE to the energy peg, NOT deferred/optional** — the reward is
> `e^(−a_estim·t) · (work·num/den)`, calendar/clock-keyed (NOT work-keyed); flat-in-time pegs to *hashes*, and
> since hashes/joule rises with hardware that silently inflates JUL against energy, so the decay is what holds
> the ENERGY peg. §4/§10#9 and inc-M3-6 ("DEFERRED") are OUT OF DATE. (2) **`infra_bps` = NONE, permanently** —
> infra is funded as measured PoM contribution + VIBE, never a coinbase skim; §4/§10#7's `[200,500]`
> recommendation is WITHDRAWN. (3) TSS L2 (PI) + L3 (rebase) are **app-layer DeFi derivatives on JUL-L1, not
> base consensus**. Still valid below: no launch ramp, no deep-capital problem (elastic reward), finality-exclusion.
>
> Status labels: ✅ built · 🟡 designed-not-built · 🔬 open research · ⚑ Will-gated number.
> Ground truth pinned at HEAD `1686029`. Every code claim carries a `file:line` — re-verify at source
> before quoting (line numbers drift). **This note SPECS and PLANS; it does not build and it does not
> decide any number.** Numbers are RECOMMENDED defaults with rationale; Will ratifies.
>
> **⚠ ERGON-FIDELITY DISCLAIMER (load-bearing).** Per `DESIGN-jul-money-layer.md` §5 ("Ergon fidelity …
> the exact difficulty→issuance proportionality to adopt should be pinned against Ergon's actual public
> design before Lever A is finalized") and `[[never-assert-noesis-protocol-numbers-from-memory]]`, this
> note asserts **no** Ergon-specific number (half-life, fee policy, price history) as fact. Where the
> reasoning would lean on an Ergon number, it is flagged 🔬 UNVERIFIED and deferred to the Ergon-fidelity
> investigation (inc-M3-4). The core economic property M3 relies on — that issuance is decoupled from
> cadence — is defended **from first principles of the built linear rule** (`jul.rs:103`), not by Ergon
> attribution.
>
> This note supersedes the retarget-signal recommendation in the M3 planning workflow (2026-07-13): an
> adversarial pass proved the "PoS-heartbeat" signal is a category error (see §2). Corrections applied.

---

## §0 — TL;DR and the one ruling that unblocks everything

M3 turns the already-built JUL *mechanism* into a live money layer. The mechanism (linear issuance, an
unforgeable single-channel coinbase, a shadow reserve) is ✅ built; what M3 adds is a **difficulty
retarget controller**, the **numbers**, an **N-way coinbase split** (for the L-INFRA slice), a
**work-clock ceiling** (safety), and the **reserve activation set**.

**The single load-bearing decision is the timestamp fork (§2).** A difficulty retarget must sense whether
blocks are arriving fast or slow, and Noesis deliberately has **no block timestamp and no prev-hash
chain**. The adversarial pass established that *no* signal Noesis already has ticks independently of block
production, so a stall cannot be sensed from on-chain state alone. The corrected recommendation:

- **Phase 1 (launch / testnet): no retarget.** Fixed genesis difficulty, genesis bits set **LOW** (one
  modest miner meets cadence). Death-spiral-safe by construction. The whole controller ships **inert**.
  Bounded by a hard height by which Phase 2 must be live (§5) so "uncontrolled emission forever" cannot
  happen by neglect.
- **Phase 2 (mainnet money): a bounded in-block timestamp** (branch **a**), median-anchored to the last
  N *finalized* coord-ordered blocks, feeding **only** the retarget controller — never `now()`, finality,
  or the reserve. This is the shipping mainnet default.
- **Stall *recovery* is a separate liveness-layer concern** (a minimum-difficulty floor + an
  emergency difficulty-reduction rule), **not** the retarget's job. The retarget *measures* elapsed time
  on blocks that landed; it cannot *detect* an ongoing stall (§2).

Everything else (emission scale, splits, ceiling, reserve) ships **provably OFF** behind inert defaults
until Will ratifies the number, exactly like `pow_enforced`, `vesting_w`, and the reserve today.

**What Will must rule on:** the five ⚑ numbers in §10 — chiefly (1) the timestamp-fork ruling, (2)
genesis bits + Phase-1 bound, (3) target JUL-per-block ⇒ `reward_den`. The rest are inert-until-set.

---

## §0.1 — Ratified decisions (Will-delegated, 2026-07-13)

Will delegated the three blocking ⚑ numbers ("you decide on all 3"). Decided per [[what-would-will-do]].
All three ship as **governable Constitution defaults** (revisitable, not an irreversible launch), and the
two that need an integer are **pinned to a build-time benchmark, not a fabricated constant** — a
target/definition is decided here; the exact value is *measured* at build (execute-to-verify, per
[[never-assert-noesis-protocol-numbers-from-memory]]).

1. **Timestamp fork → (c)-at-launch, (a)-committee-at-mainnet.** Phase 1 = fixed genesis difficulty, no
   retarget, controller ships inert; the min-difficulty floor keeps blocks coming so the chain **NEVER
   halts** (target-height **H** = the deadline for full emission-regulation *quality*, not a kill switch —
   [[noesis-never-halt-chain]]). Phase 2 =
   branch (a) in its committee-attested form: the retarget's observed-time term is the bonded finalizer
   set's **BLS-aggregated median local wall-clock**, staleness-bounded against origin, fed ONLY to
   `next_target` (never `now()`/state/finality). Stall recovery = a liveness-layer min-difficulty floor +
   emergency-reduction rule fed by the committee's gossiped local clocks. Branch (b)-internal-cadence
   rejected (category error, §2). Grounded by [[noesis-time-is-read-not-faked]] — the chain *reads* the
   clock humanity already keeps, it does not fake one.

2. **`genesis_bits` → target = one modest commodity GPU × a GENEROUS interval** (err slow-but-reliable,
   since Phase 1 has no retarget). Set `bits` so genesis expected work-per-block `W_g` ≈ what one commodity
   GPU hashes in a minute-scale cadence, so a single fair-launch miner reliably produces blocks and the
   chain cannot be stillborn. **The exact compact target is COMPUTED at build from a measured GPU-hashrate
   benchmark on the chosen PoW hash** — not asserted here. LOW keeps the launch fair-mineable +
   death-spiral-safe without a retarget; the generous interval buys liveness margin; the emission ramp
   (below) neutralises the irreversible windfall a low genesis would otherwise hand early hashrate.

3. **`reward_den = W_g`** (the same genesis expected work-per-block as (2)) ⇒ **1 JUL ≡ the energy of one
   genesis-difficulty block** (`reward_num = 1e8` fixed ⇒ ~1 JUL/block at genesis, difficulty-proportional
   thereafter — Lever A: 10× the energy ⇒ ~10× the JUL). Refuses to cosplay Bitcoin's 50/block; the unit is
   human-legible AND energy-anchored by construction. Exact `W_g` integer = the build benchmark from (2).

> **CORRECTED 2026-07-14 (Will) — NO emission ramp. The ramp is DROPPED (was "ships ON" here).** The ramp
> existed to shrink a *launch-concentration windfall* (§7 finding 5), but that is a **deep-capital** problem
> specific to **inelastic** money (Bitcoin): a fixed per-block reward means early-low-difficulty coins are
> mined near-free and become disproportionately valuable as scarcity bites. **JUL is elastic energy-money** —
> reward is PROPORTIONAL to work (Lever A), so 1 JUL costs the same energy in block 1 as in block 1,000,000;
> the JUL-per-energy ratio is constant by construction. No cheap early coins ⇒ **no deep capital ⇒ no windfall
> to neutralize** (the Ergon-community framing: elastic PoW money has no deep-capital problem). Worse, a `< 1`
> early multiplier would mint *less JUL per unit work* early than late, **breaking the energy-peg
> proportionality** that IS the invariant. So the flat proportional `reward_for_work` (already built) is the
> correct and final design — a ramp would dissolve the very property it was meant to protect. (Concentration
> of JUL is also not concentration of *control*: JUL is the money layer, orthogonal to PoS/PoM finality weight.)

**Net:** (2) and (3) collapse onto ONE measured quantity `W_g` = genesis expected work-per-block, set by
the "one modest GPU, generous interval" target. Deciding the *definition* (1 JUL = one genesis block of
energy) and the *calibration target*, with the exact integers measured at build, keeps this honest.

---

## §1 — What M3 is, the two levers, and what is already built

JUL is the **money layer**: PoW-issued, energy-pegged, **elastic and spend-designed** — the opposite of a
capped store-of-value. It is **finality-excluded** (`FINALITY_MIX = {pow:0.0, pos:1/3, pom:2/3}`,
`runtime.rs:1111`), so any difficulty/timestamp distortion an attacker induces affects **issuance and
liveness only, never finalized safety**. That single fact bounds the whole threat model below.

**Lever A — production-cost anchor.** ✅ The core is built: `reward_for_work(work, num/den)` is linear in
work (`jul.rs:103`), and under `pow_enforced`, `block_work` returns the seal's derived chainwork
(`runtime.rs:540`). Issuance is already difficulty-proportional. M3's Lever-A work = the **retarget
controller** (does not exist), **genesis bits**, and an optional work-keyed decay.

**Lever B — counter-cyclical reserve.** ✅ Built as a consensus-isolated *shadow* module (`reserve.rs`):
trend classification, bounded release, cooldowns, all params default 0 ⇒ ships provably OFF. M3's Lever-B
work = **activation numbers** + a **non-endogenous signal source**, gated on a game-theory pass (inc-4).

**Already built — do NOT redesign:**

| Fact | Where | Status |
|---|---|---|
| Linear issuance `reward = work·num/den`, floor, **no cap** | `jul.rs:103`, `:117` | ✅ |
| `block_work` returns derived chainwork under `pow_enforced` | `runtime.rs:540` | ✅ |
| Coinbase = single constructed cell, amount unforgeable, JUL-conserve-only ⇒ **unique inflation channel** | `runtime.rs:977`, `jul.rs:41` | ✅ |
| `compact_to_target` decoder + `work_from_target` (saturating u64) | noesis-core `lib.rs:996`, `:1049` | ✅ |
| Reserve state machine, all params default 0, endogenous work-trend signal | `reserve.rs` (whole module) | ✅ shadow |
| `now() == self.work` (cumulative work), folded into `state_digest` | `runtime.rs:202`, `:209` | ✅ |
| `target_to_compact` encoder · `next_target` retarget fn | grep: **NONE FOUND** | 🟡 M3 build |
| State-rent decay is a pure **SINK** (value destroyed, not pooled) | node `lib.rs:587` | ✅ (sink) |

---

## §2 — THE TIMESTAMP PROBLEM (the central fork — corrected)

**The problem.** Every production difficulty-adjustment algorithm (Bitcoin, BCH ASERT, LWMA, Digishield,
and Ergon's aserti3-2d) measures **wall-clock time between blocks** to detect a hashrate change. Noesis
has no such signal: `Block = {height, cells, coords, token_txs, coinbase, pow}` and
`PowSeal = {bits, nonce}` carry **no timestamp** (`runtime.rs:448`, `:457`), and `now() == self.work`
(`runtime.rs:209`) advances **only when a block is mined** (`runtime.rs:1003`).

> **⚠ Caveat (do not miss this).** A field literally named `Cell.timestamp: u64` *does* exist
> (`lib.rs:100`), is set (to 0) in the coinbase mint (`runtime.rs:988`), and is committed into the PoW
> header preimage (`runtime.rs:581`). Its meaning is **commit-reveal ORDER**, not wall-clock. It **MUST
> NOT** be repurposed as the retarget clock — doing so folds a producer-influenced value into finality
> ordering and breaks its strategyproof-ordering meaning.

**Three separable needs the naive framing conflates.** Getting this separation right is the whole
insight:

1. **The ECONOMIC clock** — `now() == work`, read by vesting-W, franchise decay, the reserve. Stays
   exactly as-is. Nothing below touches it. Purity/FV/replay-determinism intact.
2. **MEASURING elapsed time between blocks that DID land** — needed to retune difficulty on the *next*
   block after a fast/slow gap. This an on-chain retarget **can** own, via a bounded in-block timestamp
   (branch a). Its output `bits` is re-validated by every replica (`pow_check`, `runtime.rs:641`), so the
   timestamp never needs to enter state (see "two clocks" below).
3. **DETECTING an ongoing STALL to trigger recovery** — this **cannot** be done by any on-chain retarget.
   A signal sampled only at block-apply time can never observe "no block for a long while," because the
   sampling *stops* when blocks stop. Stall detection requires a clock consulted **during the wait**
   (a node's local wall-clock in the p2p/liveness layer) → it belongs to a **minimum-difficulty floor +
   an emergency difficulty-reduction rule**, not the retarget. **The retarget solves (2); the liveness
   layer solves (3). Do not claim one solves the other.**

**Why adding a retarget timestamp does NOT break FV / replay (the two-clocks argument — verified sound
by all three adversarial lenses).** `now()==work` is in `state_digest`, so anything folded into state
must be replay-deterministic. But a timestamp used **only** as an input to `next_target` — whose output
`bits` is *already* independently re-validated per replica (`pow_check → compact_to_target →
work_from_target`, `runtime.rs:641`, `:544`), and only the *derived* work enters `state.work`/reward —
does **not** enter `now()` or the digest. This is the same discipline as `bits` itself today: a
validated-but-not-state-folded input. **Confirmed FV-safe.**

### The corrected branch analysis

| Branch | What | Verdict |
|---|---|---|
| **(c) no retarget** | Fixed genesis difficulty, no controller. | ✅ **Launch phase.** Death-spiral-safe **only if genesis bits are LOW** (one modest miner meets cadence). Emission rate floats with hashrate → viable ONLY as a bounded launch/testnet phase, never permanent. |
| **(a) bounded in-block timestamp** | A `PowSeal.timestamp` consumed only by the retarget; monotone lower bound = median of the last N **finalized** coord-ordered blocks; upper bound = local-clock + a bounded future offset; excluded from `state_digest`; negative-solvetime-tolerant ASERT. | ✅ **Recommended mainnet default (Phase 2).** Decade-hardened (Bitcoin/BCH). Residual: hands the producer a grind surface (bounded — see §7). |
| **(b) "existing Noesis cadence" (PoS heartbeat / coord rounds)** | Source the error term from a signal Noesis already has. | ❌ **Category error — do not pursue as primary.** See below. |

**Why branch (b) fails (critical, code-grounded).** The M3 workflow recommended a "bonded-PoS-finality
heartbeat" as exogenous to hashrate. It is not: the only heartbeat in code (`Validator.last_heartbeat`,
`lib.rs:3818`) is consumed only as `retention(now − last_heartbeat, horizon)` where `now()==work`
(`lib.rs:3839`) — it is **already work-denominated**, inits to 0 in every real wiring, and **nothing
advances it on a real-time cadence**. "PoS epoch" (`runtime.rs:731`) is a logical ballot-grouping of
`(validator_id, proposal_id)` pairs, **not a timed round**. PoS finalizes the **same coord-ordered
blocks PoW produces**, so "PoS blocks per PoW block" is not a real quantity, and if production halts,
finalization halts with it. **The signal collapses to hashrate exactly at the stall it must detect.**
The honest general theorem: *any* signal able to detect a stall must ultimately derive from a wall-clock;
the only design choice is **whose** clock and how it is bounded/aggregated. Branch (a) is the
least-harmful version — the clock is bounded and finality-excluded — so it wins. (A "median over the
bonded finalizer set's local clocks" is a possible hardening of (a), not a distinct branch; it is a
committee timestamp, still a wall-clock.)

**Recommendation:** **Phase 1 = (c)** with LOW genesis bits, bounded by a hard height. **Phase 2 = (a)**
as the shipping mainnet default — and its concrete, *precedented* form is a **bonded-committee-attested
median wall-clock** (§2.1), not a single producer's timestamp. **Stall recovery = a min-difficulty floor +
emergency-reduction rule at the liveness layer**, fed by the one signal that keeps ticking through a halt —
the committee's gossiped local clocks (§2.1) — independent of the retarget. In all phases the two clocks
stay separated.

### §2.1 — The VibeSwap precedent: "whose clock" was solved once already

A scan of VibeSwap (2026-07-13) found it hit — and resolved — the same "whose clock" problem in four
places. Three are **deployed with tests**; one is a **documented principle**. They are *not* a drop-in fix
(VibeSwap always runs on an underlying L1 whose validators supply an independent wall-clock; **Noesis is
the base layer and has no L1 clock to borrow**), but they supply every ingredient Phase-2 needs, proven:

- **Committee-attested time via BLS threshold — the "whose clock" answer (spec + partial contract).**
  VibeSwap's bonded validator set attests canonical facts and aggregates them with a BLS12-381 threshold
  signature; misattestation against the canonical chain is slashable (50% bond) —
  `MessagingValidatorRegistry.sol`, `docs/research/papers/post-layerzero-canonical-messaging.md` §7.
  ⚠ **Honest status:** the messaging layer is spec + partial contracts (implementation chain-native-pending
  per CLAUDE.md), and VibeSwap's committee attests *external block identity*, **not local wall-clocks** — so
  "attest a local clock reading" is a **new adaptation**, not something VibeSwap already ships. But the
  bonded-committee-BLS-median machinery is the concrete realization of *"whose clock = the committee's,
  aggregated."* **This is the upgrade to branch (a):** Phase-2's timestamp source becomes the **bonded
  finalizer set's BLS-aggregated median local-clock reading**, not the single producer's — which also
  recovers the *intent* of branch (b) (a committee signal, not the miner's) **without** its category error,
  because a validator's local wall-clock keeps ticking even while block production is stalled.
- **Staleness bound against ORIGIN, not consumption (deployed, C49-F1).** `TruePriceOracle.sol:469` rejects
  a pulled reading unless `now ≤ origin_deadline + MAX_STALENESS` (`MAX_STALENESS = 5min`, `:47`), boundary
  tests in `test/security/C49F1AggregatorBatchStaleness*.t.sol`. Lesson (`ORACLE_OVERVIEW.md:138`):
  *replay-protection is not freshness — bound a time-signal's age against where it was generated, not
  against your last-seen state.* **Port:** apply the origin-staleness bound to the branch-(a) timestamp's
  future/past window AND to the reserve's non-endogenous signal (§6), to reject stale/manipulated attests.
- **Monotone native quantity + a gap, not wall-clock, where only ordering is needed (deployed,
  TRP-R17-F04).** `CommitRevealAuction.sol:779,810` records `block.number` (not `block.timestamp`) at
  reveal-end and forbids settling in the same block (`block.number > revealEndBlock`), so the last revealer
  cannot grind the derived seed. **Port:** confirms the retarget *schedule half* (`expected =
  interval·height_delta`) needs **no clock** (use block-count), and confirms the anti-grind obligation in
  §3 — a bounded gap so a producer cannot jointly grind `(timestamp, nonce)`.
- **The documented principle: "the substrate's monotone quantity IS the clock; never a participant's
  wall-clock" (`REORG_BEHAVIOR_DESIGN.md` §6 + `omniscient-adversary-post.md`).** Time-sensitive ops bind
  to *block-height-delta* under per-decision finality thresholds, structurally enforced, **Physics >
  Constitution > Governance**. **Port:** confirms Noesis's `now()==work` as the *economic/ordering* clock
  and the governance hierarchy in §8. ⚠ **Honest limit:** this works for VibeSwap *because* it runs on an
  L1. Noesis is the base layer — every native monotone quantity (work, height, count) **freezes when
  production halts**, so the substrate-clock cannot be the stall *detector*. The stall detector must be the
  one signal that keeps ticking through a halt: the **bonded committee's gossiped local wall-clocks** (first
  bullet), consulted out-of-band. That names the §7.1 / §11 liveness-layer signal concretely — **designable
  from proven parts, not yet built** (today's `last_heartbeat` is work-denominated and advanced by nothing,
  `lib.rs:3839`).

**Net effect on this note:** the "whose clock" open question closes toward a concrete answer — a
**bonded-committee BLS-median wall-clock, staleness-bounded against origin, with block-count for the
ordering-only parts** — and the stall-detector gains a named signal (gossiped committee clocks). All of it
is composition of patterns VibeSwap already proved; none of it is a ready-made drop-in.

---

## §3 — The retarget controller

**Scope it correctly (verified sound):** the controller's job is **cadence / liveness regulation, NOT
money-supply gating.** Because issuance is linear in work (`jul.rs:103`), a difficulty drop mints
proportionally *less* JUL — so the classic "throttle → retarget drops difficulty → mine cheap coins"
**mint** attack is not even expressible (dissolved at the root; see §7 for the residual *cost-basis*
channel, which is NOT dissolved). This holds from the built linear rule alone, no Ergon attribution.

**Algorithm — ASERT, schedule-term LIVE, observed-term SEAMED (🟡/⚑).** ASERT's exponent decomposes as
`(observed_elapsed) − (expected_elapsed = ideal_interval · height_delta)`. The **expected** half is pure
schedule (height + a constant, **no clock**); only the **observed** half needs the timestamp. So:

- Ship `next_target(bits, observed: Option<Signal>, params) -> Option<u32>` with the schedule half live
  and the observed half behind an `Option` seam that returns the degenerate "exactly on schedule" value
  (Δ == expected ⇒ target unchanged) when no signal exists. Same "right interface, degenerate constant"
  discipline the codebase already uses (`jul.rs:20`, `pow_enforced` default false). The controller lands
  and is testable while the clock decision stays ⚑.
- ASERT chosen over LWMA (ST-clamp bug history) and Digishield (clamps enable selfish-mining gain), and
  it is the leanest (one fixed-point exponential, consensus-safe integer `2^x` approx). Bitcoin-simple.

**Prerequisite build item — `target_to_compact(target: &[u8;32]) -> Option<u32>`** in noesis-core `pow`
(inverse of `compact_to_target`, identical strict validation: reject sign bit, zero mantissa, overflow).
Confirmed absent (grep NONE FOUND). Without it M3 cannot produce candidate-block `bits`. No number gated.

**MTP source (corrected).** "MTP-11" as literally specified is **not portable** — with no prev-hash and
no header chain (`runtime.rs:560`), there is no canonical "previous 11 blocks" header ancestry to median
over. The monotone lower bound must be the median of the last N blocks **in coord-canonical finalized
order**, retrievable from `Ledger` state, and the timestamp bounded **above** by the validating node's
local clock + a bounded future offset, and **excluded from `state_digest`**. A build-time obligation:
prove that jointly grinding `(timestamp, nonce)` cannot cheapen the PoW.

**Cadence / retarget window.** Recommend a fixed retarget window, **block-count first cut** (conservative,
Bitcoin-analog). A fixed *work-interval* window is novel and wants its own game-theory pass (🔬).

**Ergon divergence:** adopt Ergon's *reward-side* shape (already built, §4); do **not** port its
timestamp inputs (they don't exist here). The `Signal` source is ours to choose per the fork.

---

## §4 — Emission + the N-way coinbase split (incl. L-INFRA)

**Emission shape — KEEP LINEAR (✅ built, do not redesign).** `reward = work · num/den` (`jul.rs:103`) is
the peg primitive: reward proportional to difficulty ⇒ issuance decoupled from cadence. A nonlinear
`f(work)` would make emission *fight* the retarget and re-couple value to cadence — the exact thing
linearity dissolves. The `ERGON SEAM` comment (`jul.rs:96`) already anticipates a body-swap behind the
same signature if ever justified. M3 pins only the **scale**: `reward_num = JUL_BASE_UNITS` (1e8,
hardcoded `jul.rs:31`); `reward_den` = ⚑ derived from Will's target JUL-per-block at genesis difficulty.

> **Honest correction:** under a no-retarget Phase 1, `reward_den` is the **only** thing bounding absolute
> issuance — it is **structurally load-bearing at launch**, not "a pure scaling error." See §5 Phase-1
> bound.

**Moore's-law decay (🟡/⚑ — DEFERRED to future work).** Correcting coins-per-work for hardware-efficiency
drift is pure optionality with no launch need; it adds a Constitution field and a slow-inflation story.
🔬 The specific half-life sometimes attributed to Ergon is **UNVERIFIED** and must not anchor a decision.
If ever adopted, key it to **cumulative work** (`Ledger::now`), not calendar/height, and ship it OFF.

**The N-way coinbase split (🟡, the L-INFRA carve-out).** Today the mint is single-recipient: 100% of
`reward` to `b.coinbase` (`runtime.rs:977`). L-INFRA forbids allocating 100% to the producer and forbids
a 2-way hard-wire that would need reopening to add a 3rd recipient (Ethereum's retrofit trap). **Design
as a genuine `Vec<(recipient-or-role, u16 bps)>` split, remainder → producer** (a Vec is barely more code
than named fields and is the version that actually honors the retrofit-trap argument — lazy-senior-dev
call). Every slice is carved from the **one constructed reward**:

```
Σ slice_bps ≤ 10_000;  each slice_amt = reward · slice_bps / 10_000;  producer = reward − Σ slice_amt
```

so the coinbase stays the **structurally-unique inflation channel** (`jul.rs:41`) and the FV
`Σout ≤ Σin` oracle holds. Defaults: empty split ⇒ **byte-identical to today**. Governance turns each
slice on without a consensus-breaking reopen. RED test: `Σ slices == reward`, unique-channel intact.

**Infra funding source (⚑).** Recommend **rent-preferred, issuance-bootstrap**. Long-run source = a
state-rent pool — but decay is a pure SINK today (`lib.rs:587`), so converting it to a collectable pool
is a **separate increment (inc-3c), off the M3 critical path**. Until rent volume exists, an `infra_bps`
issuance carve-out is the bootstrap bridge; when on, a safe first value is `infra_bps ∈ [200,500]` (2–5%),
routed through **PoM standing** (soulbound, dispute-refutable). 🔬 Any Ergon "infra precedent" claim is
UNVERIFIED; treat infra-as-contribution as a Noesis-native design, not a comparison.

---

## §5 — Genesis bits, the Phase-1 bound, and the work-clock ceiling

**Genesis bits (⚑) — LOW for the launch phase (corrected).** The failure modes are asymmetric: too-low
mints an **irreversible** unfair JUL windfall (violates fair-launch, `jul.rs:123`); too-high risks a
**stuck genesis** (can't mine block 1) which is otherwise backstopped by bonded-PoS finality. The M3
workflow concluded "conservatively HIGH" — but that only holds **once an early downward retarget is
live**, which **Phase 1 (branch c) by definition lacks.** HIGH + no-retarget = stillbirth. **So for the
retarget-inert launch, genesis bits must be LOW-but-not-trivial** (single modest miner meets cadence).
Revisit upward only when the Phase-2 controller ships. Recommend deriving from "a modest commodity-GPU-
hours per block." 🔬 open: one governable value vs separate testnet/mainnet genesis.

**Phase-1 bound (new — closes "uncontrolled emission forever").** During Phase 1 **both levers are off**
(Lever A needs the absent retarget; Lever B ships OFF), so there is **no active inflation governor** —
emission is bounded only by the fixed target and per-block Sybil cost. Bound it **without a kill switch** —
the chain must **NEVER halt** (a hard Noesis invariant, [[noesis-never-halt-chain]]). So a delayed Phase 2
is a *quality* degradation, not a liveness failure: emission stays bounded **per block** by the fixed
target's Sybil cost + the ramp, and if Phase 2 slips the emission *rate* runs unregulated (a known, bounded
degraded mode) while the min-difficulty floor keeps blocks coming. Height **H** is the **deadline for full
emission-regulation quality, NOT a halt threshold**. Quantify worst-case over-issuance if hashrate 10×
during the fixed-target window so the degraded mode is bounded-and-known, never an excuse to stop the chain.

> **CORRECTED 2026-07-14: no emission ramp** (see §0.1). There is nothing to "shrink" — because reward is
> proportional to work, emission per block already tracks energy 1:1, and the JUL-per-energy cost is
> constant, so a low-difficulty launch window mints proportionally little JUL, not a windfall. The only
> Phase-1 emission fact is the flat proportional rule; the ramp would have broken it.

**Work-clock ceiling (🟡/⚑ — safety-critical; make it a PRECONDITION of `pow_enforced`).** `now()` is the
single clock every temporal mechanism reads; the finality frontier = `now() − vesting_w`
(`runtime.rs:779`). One validly-mined enormous-difficulty block can jump `now()` past the `finalized_at`
stamp of **every pending PoM cell at once**, maturing them through the vesting cliff and collapsing the
dispute-during-W window (the L2 safety property). `u64` saturation prevents *overflow* but not this
*cross-mechanism* externality.

- **Design:** clamp the **clock contribution only** — `effective_now_delta = block_work(b).min(ceiling)`,
  `ceiling = K · expected_work_at_current_bits`. **Pay the FULL `reward_for_work(unclamped block_work)`**
  (corrected — clamping reward would under-pay proven energy and install a soft emission cap, breaking
  "NO CAP by design", `jul.rs:114`, and weakening Lever A at high difficulty). Confirm that reward reading
  unclamped `block_work` while the clock reads `effective` does not break the FV `Σout ≤ Σin` oracle
  (reward is its own constructed amount, so this should hold — verify in the build).
- **Clamp, do NOT reject** the block — a validly-mined hard block is honest work; rejecting on excess
  difficulty would be a censorship knob.
- **Make it a hard gate:** a finite ceiling must be a **validation precondition of `pow_enforced`** (and
  of `vesting_w > 0`). A Council RED test: `pow_enforced` with `ceiling == u64::MAX` must FAIL to
  activate. The workflow left the ceiling default-inert and unordered relative to `pow_enforced` — that
  ships the attack live. `K` is a joint calibration with `vesting_w` and the dispute-window length (🔬).

---

## §6 — Reserve (Lever B): ship OFF, activate LAST

**Posture: keep `ReserveParams::default()` all-zero (already so, `reserve.rs:121`); activate LAST, never
in the same increment as a live retarget.** Two composition hazards force this:

1. **Endogenous / manipulable signal.** The v0 trend reads the work series (`reserve.rs:87`); a producer
   with meaningful hashrate can **throttle to trip `bear` and harvest a bounded release** — the module
   flags this (`reserve.rs:44`). Caps bound per-episode theft but not repeated harvesting.
2. **A working controller KILLS the proxy.** Retarget and reserve read the *same* `block_work` series; a
   controller that stabilizes difficulty **flattens** it ⇒ `trend → 0` ⇒ Lever B goes inert exactly when
   Lever A works. They interfere by construction.

**Therefore:** activate only after a **non-endogenous** signal seam exists (a bonded-verified price/demand
feed). `assess` already takes `Option<Signal>` (`reserve.rs:207`) ⇒ zero-mechanism-change source swap.

**Release destination — BURN-ONLY until the reserve is a real cell (corrected).** The seam offers
"coinbase-subsidy top-up," but the reserve is a module-local `u128` with **no cell backing**
(`CONTROL_BINDING_ACTIVE == false`, `runtime.rs:420`). A top-up would synthesize JUL at the coinbase —
**indistinguishable from a second mint** to the conservation oracle unless the release is a two-legged
single-state transition (debit `reserve_balance` by R *and* credit coinbase by R in the same apply, with
`reserve_balance` counted in `Σin`). Until that atomic cross-account exists (inc-3b, needs the cell),
**burn is the default and top-up is forbidden** (burn only debits ⇒ trivially conservation-safe).

**Activation set (all ⚑, all conservative, all 0 until inc-4).** The **load-bearing safety numbers are the
harvest-bounds** — `max_release_per_period`, the cooldown work-windows, and the `bear_threshold_bps`
**deadband width** — deliberately deferred to the inc-4 game-theory pass. The `skim_bps [100,300]` /
`release_rate_bps ≤ 500` ranges are **illustrative only and do NOT bound attacker profit.** A guard: reject
any nonzero reserve param unless `bear_threshold_bps` is above a minimum deadband AND a non-endogenous
signal source is registered.

---

## §7 — Adversarial defenses (ranked; corrected)

1. **[HIGH — liveness] Stuck-difficulty / death-spiral.** A pure work/height retarget cannot lower
   stuck-high difficulty on a stalled chain (the work-clock freezes). **Split defense:** branch-(a)
   timestamp lets a *resumed* chain retune down on the next block (need 2); a **min-difficulty floor +
   emergency-reduction rule at the liveness layer** handles ongoing *stall detection* (need 3). The
   retarget alone owns neither of these fully — say so.
2. **[HIGH — safety] Work-clock inflation vs vesting.** One huge-difficulty block mass-matures pending
   PoM cells past the cliff (`runtime.rs:779`). **Defense:** finite work-clock ceiling as a **precondition
   of `pow_enforced`**, clamp-clock-not-reward (§5).
3. **[MEDIUM — cost-basis reflexivity, NOT fully dissolved].** The "issuance is self-limiting" argument
   closes the **mint-quantity** channel but **not** the **production-cost-basis margin**: a difficulty
   drop lowers the throttling miner's *energy cost per block* while JUL-per-work is unchanged ⇒ a real
   margin gain. **Do not claim this is dissolved.** Close it structurally: the retarget error term must be
   a **median over many producers**, not one; and inc-4 must carry the miner-reflexivity proof as a
   **required artifact**, gating activation — not a "pass."
4. **[MEDIUM — economic-theft, inert] Reserve harvest reflexivity.** Throttle → trip bear → harvest
   (`reserve.rs:44`). **Defense:** ship OFF; non-endogenous signal + inc-4 before any nonzero param (§6).
5. **[DOWNGRADED 2026-07-14 → NON-ISSUE for the money layer] Genesis burst / launch concentration.** Fixed
   genesis bits ⇒ an actor with large launch hashrate mines more early coinbase cells — but this is **NOT a
   windfall**: JUL is elastic energy-money, reward is proportional to work, so every early cell cost its
   miner FULL proportional energy (constant JUL-per-energy). There are no near-free early coins to
   concentrate (the deep-capital problem is specific to **inelastic** money like BTC; §0.1). Acquiring more
   JUL by spending more energy is the intended fair-launch dynamic, not capture. And JUL concentration is not
   *control* concentration — JUL is orthogonal to PoS/PoM finality weight. **No defense needed; the dropped
   ramp (§5) would have made it worse by breaking proportionality.** Genesis bits LOW keeps it fair-mineable.
   (The distinct concern — finalizers *acquiring* JUL — is finding 6, which stands.)
6. **[MEDIUM — anti-plutocracy, must be a CHECK not a norm] Finalizer capture of the genesis burst.**
   `jul.rs` guarantees `issued == 0` at genesis, so the real risk is finalizers *acquiring* the burst.
   Nothing in `validate_block`/genesis checks finalizer JUL holdings today. **Defense:** make it
   structural — snapshot the bonded finalizer set **before block 1** and bind the ramp to it; do not ship
   the invariant as prose.
7. **[LOW-now, expensive-to-retrofit] Coinbase-split rigidity.** **Defense:** genuine `Vec` N-way split
   from day one, all slices default 0 but structurally present (§4).
8. **[If a timestamp is used] Bounded timewarp.** Residual producer grind is **finality-safe** (PoW
   excluded) but still an **issuance/UX** distortion — the money layer's core product, so not benign.
   **Defense + bound:** MTP-from-finalized-set + future-time limit + negative-solvetime-tolerant ASERT;
   the ASERT halflife **caps the per-window difficulty move**, so a timewarp's issuance distortion is
   bounded to ± one halflife-window's drift — state that bound as a number, don't assert "acceptable."
9. **[Composition] Never co-activate a time-retarget and the reserve** in one increment (surfaces
   compose: a throttle harvests the reserve **and** manufactures a cheap-cost window).

---

## §8 — Correspondence-triad check (mechanism-design gate)

- **Substrate-geometry match (✓).** PoW difficulty is exponential/multiplicative (compact target,
  chainwork); JUL-per-block is exponential in the difficulty bits via `work_from_target`. An **ASERT**
  controller is exponential — the controller's geometry matches the substrate's (exponential response to
  an exponential quantity), not a linear knob on a nonlinear surface. Emission stays linear *in work* on
  purpose (that is what decouples value from cadence). Pass.
- **Augmented mechanism design (✓).** M3 augments with **math-enforced invariants** rather than policy:
  the unique-inflation-channel (constructed coinbase + JUL-conserve-only), `Σout ≤ Σin`, the split
  summing ≤ 10000, the work-clock ceiling. Fairness is structural (genesis ramp + pre-block-1 bonded
  snapshot), not discretionary. Pass.
- **Augmented governance (✓, with a note).** Hierarchy preserved: **Physics** (unique inflation channel,
  conservation — a 51% vote cannot mint outside the coinbase) > **Constitution** (JulParams, ceiling,
  splits, reserve params — governable but *bounded*: e.g. ceiling must be finite under `pow_enforced`,
  reserve nonzero only with a deadband + non-endogenous signal) > **Governance** (DAO sets numbers within
  those bounds). The invariant that must stay physics-level: governance can set **bad** numbers but can
  **never** break conservation or the single-channel property. Pass.

---

## §9 — Build-increment plan (sequenced for THE LOOP; corrected ordering + gates)

THE LOOP = 2 planners → build → adversarial Council → Pragma code/doc confluence. Every lever ships
**provably OFF** until Will ratifies its number. **Council process fix (from the last session): review
agents MUST be read-only `Explore` agents — no write access to the shared tree.**

| id | what | consensus? | depends on | ⚑ number? |
|---|---|---|---|---|
| **inc-M3-0** | `target_to_compact` encoder (strict inverse of `compact_to_target`). Prereq for candidate `bits`. | no | none | no |
| **inc-M3-3** | N-way `Vec` coinbase split at `runtime.rs:977`, empty default ⇒ byte-identical. L-INFRA schema — near-free now, retrofit-trap if deferred. | yes | none | no |
| **inc-M3-2** | Work-clock ceiling, clamp **clock-only pay full reward**, **as a precondition of `pow_enforced`** (RED test: `pow_enforced` + `ceiling==u64::MAX` fails to activate). Couple `K` to `vesting_w`. | yes | none (gates M3-1) | yes |
| **inc-M3-4** | DESIGN + INVESTIGATION: rule the timestamp fork; confirm/deny PoS-cadence exogeneity (code predicts NEGATIVE ⇒ branch a). No code. | no | none | yes |
| **inc-M3-1** | ASERT controller `next_target(bits, Option<Signal>, params)`, schedule-live/observed-seamed; add `genesis_bits` + `retarget_window`, inert defaults. **Ceiling (M3-2) must precede activation.** | yes | M3-0, M3-2, (ruling M3-4) | yes |
| **inc-M3-5** | Wire branch-(a) guarded timestamp (median-of-finalized + future-bound + neg-solvetime) into the observed seam. Signal → retarget ONLY. | yes | M3-1, M3-4 | yes |
| **inc-3b** | Reserve cell (protocol-spend-only clause; blocked on `CONTROL_BINDING_ACTIVE`) + wire `accrue` into the skim arm; **burn-only release** until the cell exists. Reserve stays OFF. | yes | M3-3 | yes |
| **inc-4** | Governance-bound params + miner-reflexivity proof (required artifact) — THE gate before any nonzero reserve param. Requires a non-endogenous signal. | yes | 3b, M3-5 | yes |
| **inc-3c** | ORTHOGONAL (off critical path): convert decay SINK (`lib.rs:587`) → collectable rent pool (preferred long-run infra source). | yes | M3-3 | no |
| **inc-M3-6** | DEFERRED / future work: Moore's-law decay on `reward_num`, work-keyed, default OFF. Pure optionality. | yes | M3-3 | yes |

**Unconditionally worth building now (no ⚑):** inc-M3-0 (encoder) and inc-M3-3 (N-way split). Everything
else waits on a Will ruling or an inert-default flip.

---

## §10 — The decision table (the ⚑ numbers Will ratifies)

| # | Parameter | Recommended default | Why / risk if wrong |
|---|---|---|---|
| **1** | **Timestamp-fork ruling + Phase-1 bound** | ✅ **RATIFIED** (§0.1). Phase-1 = **(c) no retarget**, LOW genesis bits, target height H for full regulation (a quality deadline — the chain NEVER halts, [[noesis-never-halt-chain]]). Phase-2 = **(a)** in its precedented form: a **bonded-committee BLS-median wall-clock**, staleness-bounded against origin (§2.1). Stall recovery = min-difficulty floor + emergency rule fed by the committee's gossiped local clocks. Branch (b)-internal-cadence rejected (category error). | The load-bearing decision. Wrong = either a stalled young chain that can't recover, or a state-purity break, or open-ended launch inflation. |
| **2** | **`genesis_bits`** | ✅ **RATIFIED** (§0.1): target = one modest GPU × a generous interval; exact `bits` measured at build. Revisit upward only once the Phase-2 retarget is live. | LOW + no-retarget is death-spiral-safe; HIGH + no-retarget = stillbirth (can't mine block 1). Too-low windfall is irreversible → the ramp (below) covers it. |
| **3** | **`reward_den`** (`reward_num = 1e8` fixed) | ✅ **RATIFIED** (§0.1): `reward_den = W_g` ⇒ 1 JUL = one genesis-block of energy. Flat proportional reward (NO ramp). | Sets JUL-per-block. **Structurally load-bearing at launch** (the only issuance bound in Phase 1), not cosmetic. |
| **4** | **Genesis emission ramp** | ❌ **DROPPED 2026-07-14 (Will)** — §0.1. | JUL is elastic ⇒ no deep-capital ⇒ no windfall to shrink; a `< 1` early multiplier would break the energy-peg proportionality (identical work must mint identical JUL). Flat proportional reward is correct + final. |
| **5** | **Retarget algo + params** | ASERT; halflife/window ⚑ (block-count first cut); observed-term seamed. | Cadence/liveness only, not money supply. Wrong-signed/oscillating = cadence thrash; too-slow = strands difficulty after an exodus. |
| **6** | **`work_clock_ceiling` `K`** | Finite, **precondition of `pow_enforced`**; clamp clock-only. Couple to `vesting_w`. | Too low = soft emission cap / under-pays energy; too high = re-opens vesting-collapse. Joint calibration. |
| **7** | **`infra_bps`** (in the `Vec` split) | ❌ **NONE, permanently (RATIFIED 2026-07-15).** Infra funded as measured PoM contribution + VIBE, never a coinbase skim; `coinbase_split` stays empty. The `[200,500]` recommendation is WITHDRAWN. | A redistributive coinbase tax contradicts the "measured contribution dissolves redistribution" thesis; PoM+VIBE is the correct funding path. |
| **8** | **Reserve activation set** — `skim_bps`, `bear_threshold_bps` **deadband**, `max_release_per_period`, cooldowns | **ALL 0 until inc-4.** Load-bearing safety numbers = the harvest-bounds (deadband, per-period cap, cooldowns), set in the inc-4 game-theory pass. Release = **burn-only** until the reserve cell exists. | Live on an endogenous/zero-deadband signal = repeatable harvesting. Co-activated with a time-retarget = composed attack surface. |
| **9** | **Moore's-law decay** | ✅ **CORE, ON (RATIFIED 2026-07-15) — NOT deferred.** `e^(−a_estim·t)`, CALENDAR/clock-keyed (not work-keyed), `a_estim` governable (illustrative ~3yr efficiency-doubling). Ships decay=0 ⇒ inert until set. | A flat (no-decay) reward pegs to *hashes*; hashes/joule rises with hardware ⇒ silent inflation vs energy. The decay HOLDS the energy peg — it is the peg, not optionality. |

---

## §11 — Honest gaps (🔬 open, carried forward)

- **Ergon-fidelity investigation** (inc-M3-4): pin the reward-side comparison and any half-life/fee claim
  against Ergon's *actual* public design + a block explorer before quoting a single Ergon number.
- **PoS-cadence exogeneity** — code strongly predicts NEGATIVE (§2); the investigation confirms and closes
  the fork toward branch (a).
- **Stall-recovery liveness rule** — the min-difficulty floor + emergency-reduction design is named here
  but not specified; it is a p2p/liveness-layer artifact, adjacent to L7 (genesis/P2P network).
- **Joint calibration** of `K` (ceiling) × `vesting_w` × dispute-window length.
- **Fixed work-interval retarget window** (vs block-count) — novel, wants its own game-theory pass.
- **Miner cost-basis reflexivity** (§7.3) — the proof obligation for inc-4, not yet discharged.

*Complete = ready-for-critique, not validated. This note is the artifact to attack next.*
