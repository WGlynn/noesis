# DESIGN — Vesting window `W` + the Standing→Validator.pom finality bridge

> **STATUS: design-first, build-cold, Will-gated.** Consensus-affecting (it defines what the PoM
> finality input *is* in production). Grounded 2026-07-11 against `docs/POM-FINALITY-TEMPORALITY.md`,
> `docs/VS-AS-COMPLETION-PROCEDURE.md` §Target-2, and the live code cited inline. This closes the
> roadmap top-blocker (MVP-SCOPE §1.A.1–2). Nothing here is built yet.

## 0. Why this is the top blocker
Two coupled facts, both verified in code:

1. **The finality PoM floor is circular in the PoM direction.** `Ledger.pom =
   pom_scores_with_similarity_floor_q16(...)` (the v(S)/attribution output, `runtime.rs:583`) is the
   same quantity that becomes `Standing.pom` (soulbound credit, `lib.rs:469`), which is the same
   quantity the anti-concentration floor gates via `dim_ok` (`runtime.rs`). So inflating v(S) inflates
   the gamer's *own* PoM finality weight — the gaming propagates **into** the floored dimension, not
   bounded by it (`VS-AS-COMPLETION-PROCEDURE.md:222–245`). The floor's genuine property is only
   *cross-axis* (a v(S)-gamer must ALSO command ≥50% real staked capital — a capital-cost multiplier,
   not a scoring backstop).
2. **The bridge that the whole property assumes does not exist.** `consensus::Validator.pom` (f64
   finality weight, `lib.rs:3716`) is set ONLY in test constructors and the slash-to-zero path
   (`lib.rs:3836`). The standing ledger is `Ledger.pom: HashMap<Vec<u8>,u64>` (`runtime.rs:94`). **No
   production map→weight bridge exists** (`VS-AS:256–263`).

⇒ Until a bridge is built, PoM-finality is inert in production; and if the bridge is a *raw
pass-through* of `pom_scores`, it bakes the circularity in. The fix is to build the bridge such that
it reads **only cleared standing** — v(S) that has survived a dispute window `W`.

## 1. The invariant this serves (from POM-FINALITY-TEMPORALITY.md, non-negotiable)
> A block is finalized by the **pre-existing** franchise (bonded stake + already-vested prior
> standing). Contributions earn **future** franchise and never vote on their own block. **No value is
> ever an input to its own finalization.**

Standing is a stake-like running account. Realized value = **deposits** that clear after a vesting
delay `W` (like a check clearing). Finality at `now` reads the **current cleared balance** — never a
future value, never a live/un-cleared one. `W` exists so disputes can catch fraudulent value *before*
it clears into usable finality weight. Slashing is forward-only (past finalized blocks stay final).

**Resolved, do not re-litigate:** the "decouple PoM from finality safety?" question
(POM-FINALITY-TEMPORALITY §"one open decision") was **ruled by Will 2026-06-29 → PoM stays COUPLED**
("coupled is the only real answer"). So this design keeps PoM in finality; the anti-concentration
floor + `W` are the two protections. What remains open is the **parameter + wiring surface** below.

## 2. Mechanism (build-cold, staged)

### 2.1 Timestamped accrual (the data-model change)
`Standing.pom` is currently a scalar `u64` (`lib.rs:469`). To vest, a deposit must know *when* it
accrued. Add a cumulative-work timestamp to each accrual using the clock we already shipped
(`Ledger::now()`, commit `6711065`). Model (recommend the **two-bucket** form for leanness):

- `Standing { contributor, cleared: u64, pending: VecDeque<(amount: u64, accrued_at: u64)> }`
  where `accrued_at = Ledger::now()` at the `Op::Accrue`. (A bounded deque; see D4 for the bound.)
- **Reference-layer only, additive, NON-consensus-affecting until the bridge (2.3) reads it.** This
  phase can ship safe.

### 2.2 Clearing rule
At time `now`, a pending deposit with `accrued_at ≤ now − W` has **cleared**: move `amount` from
`pending` → `cleared`. Deterministic, monotone (the clock is monotone). Cleared standing is what earns
finality weight; pending standing still earns **reward/influence** (that path is unchanged) but **no
finality-safety weight**. This is the exact "usable-face vs gameable-face" split the audit forced.

### 2.3 The bridge (the consensus-affecting step)
`fn finality_pom_weight(ledger, now, W) -> per-contributor f64` = each contributor's **cleared**
balance at `now`. This is the production `Standing → Validator.pom` map. `Validator.pom` in the live
finality path becomes this cleared balance instead of a test constructor value. **This is build-cold /
Will-gated — it changes what PoM-finality *is* in production.**

### 2.4 Dispute-during-`W` (what makes it non-circular)
The dispute/`Op::Slash` path already exists (`lib.rs:474–475, 489`). A slash landing on a deposit
while it is still `pending` removes it **before it clears** ⇒ gamed v(S) caught within `W` never
reaches finality weight. Forward-only: a slash never un-finalizes a past block (the quorum at `t`
existed). ⇒ **cleared standing = v(S) that survived `W` of dispute exposure**, which is the genuinely
non-raw-pass-through input the fix (`VS-AS:265–269`) requires. It does NOT make v(S) un-gameable (that
is the moat, separate); it ensures gamed v(S) has a `W`-window to be caught before it can vote. This
is precisely "`W` is the moat's stand-in at launch."

### 2.5 Genesis bootstrap (automatic, confirm only)
At genesis nothing has cleared ⇒ `finality_pom_weight = 0` for all ⇒ **bonded PoS carries finality
from block zero**, PoM phasing in as deposits clear. This falls out of 2.3 for free and matches
POM-FINALITY-TEMPORALITY §Genesis. No new code; just confirm it is the intended bootstrap.

## 3. Build stages (each RED→GREEN, blast-radius increasing)
1. **Phase 1 — timestamped accrual (SAFE, additive).** Add the deposit timestamps + two-bucket
   Standing; wire `Op::Accrue` to stamp `Ledger::now()`. Tests: accrual stamps the right time;
   replica-deterministic; existing accrual sums unchanged. Non-consensus (nothing reads the buckets
   for finality yet).
2. **Phase 2 — clearing + bridge (CONSENSUS-AFFECTING, cold).** `finality_pom_weight(now, W)` counts
   only cleared; wire it as the `Validator.pom` source in the live path. Tests: fresh standing
   excluded; cleared standing counted; exact boundary at `now − W`; genesis ⇒ all-zero ⇒ PoS-only
   finality.
3. **Phase 3 — dispute-during-`W` (CONSENSUS-AFFECTING, cold).** Slash on a pending deposit removes
   it pre-clear; forward-only invariant pinned. Tests: accrue-then-slash-within-`W` never reaches
   finality weight; survive-`W` ⇒ clears; a past finalized block is untouched by a later slash.

Phase 1 can land immediately and safely. Phases 2–3 wait on the D1–D4 rulings below.

## 4. DECISIONS TEED UP (Will's call)

- **D1 — `W` value (THE load-bearing parameter).** In cumulative-work-clock units (pre-PoW,
  1 unit = 1 block). Long `W` = more fraud caught before clearing (safer); short `W` = franchise
  tracks reality (responsive). *Rec:* a **governed constant** (like `MIN_DIM_BPS`, not a controller),
  start conservative and long, tune on real data (the Ergon discipline). Need a starting number;
  *Rec default:* on the order of the dispute-window length so a deposit cannot clear faster than a
  challenge can land — pick `W = dispute_window` as the anchor, exact value ratify-or-defer.
- **D2 — vesting shape: cliff vs ramp.** Clear all-at-once at `accrued_at + W` (cliff), or ramp
  linearly across `W`. *Rec:* **cliff** — simplest, matches the "check clears" metaphor, kernel-lean.
  (Linear ramp could reuse the existing `retention` geometry if smoothness is wanted later.)
- **D3 — data model: two-bucket queue.** Recommend `{cleared, pending: VecDeque<(amt, at)>}`; it is
  consensus state (soulbound) so it must be bounded + deterministic. *Rec:* accept two-bucket; bound
  the pending deque (D4).
- **D4 — pending-deque bound.** A contributor accruing every block would grow `pending` unboundedly.
  *Rec:* coalesce accruals within the same clock-unit into one deposit, and/or cap pending depth with
  oldest-first clearing — cheap, keeps state bounded. Confirm the bound is acceptable as consensus
  state.
- **D5 — `MIN_DIM_BPS` safety-path raise (the standing rider).** Adjacent to the quorum floor just
  wired. *Rec:* **defer** — the quorum floor now supplies the participation backstop; treat
  `MIN_DIM_BPS` as a separate lever to tune on data, not to raise blind now.

## 5. What this closes / does not close
- **Closes:** the PoM-direction circularity *at launch* (gamed v(S) must survive `W` of dispute
  before it can vote finality), and the missing production franchise bridge. Makes the coupled-PoM
  finality design honest in code rather than 🟡-designed.
- **Does NOT close:** v(S) un-gameability itself (the moat — separate, structurally-demonstrated /
  learned-open per STATUS-LEDGER MOAT-1). `W` is the *stand-in* that buys time for the immune system;
  it is not the immune system.
