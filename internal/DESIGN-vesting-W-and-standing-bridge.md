# DESIGN — Vesting window `W` + the Standing→Validator.pom finality bridge

> **STATUS: design-first, build-cold, Will-gated.** Consensus-affecting (it defines what the PoM
> finality input *is* in production). Grounded 2026-07-11 against `docs/POM-FINALITY-TEMPORALITY.md`,
> `docs/VS-AS-COMPLETION-PROCEDURE.md` §Target-2, and the live code cited inline. This closes the
> roadmap top-blocker (MVP-SCOPE §1.A.1–2). **Phase 1 (§3.1, the `finalized_at` finalization stamp)
> is BUILT** — commit `5f5c7e6`, 2026-07-12, node lib 278→281 green, additive / non-consensus (off
> the state digest). Phases 2–3 (the consensus-affecting cleared-score bridge + dispute-during-`W`)
> remain build-cold for a fresh low-context window.

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

## 2. Mechanism (build-cold, staged) — CORRECTED after grounding 2026-07-11

**Grounding correction (load-bearing):** `Ledger.pom` is a **stateless recompute** every block —
`self.ledger.pom = pom_scores_with_similarity_floor_q16(&self.ledger.cells, θ_sim)`
(`runtime.rs:583`). It is NOT an accrual event-stream, so there are no deposit events to bucket. The
earlier "two-bucket `VecDeque` on `Standing`" model does not fit this architecture and is retracted.
The correct model vests at the **cell-age** layer, not the standing-scalar layer, and leaves the
soulbound `Standing` Molecule encoding (`lib.rs:465`) **untouched** — which also avoids an on-VM
type-script change.

### 2.1 Cell finalization stamp (the data-model change — SAFE / additive)
Add a per-cell cumulative-work finalization time using the clock we already shipped
(`Ledger::now()`, commit `6711065`): a node-side index `Ledger.finalized_at: HashMap<cell_id, u64>`
populated in `apply()` when a cell enters the ledger. Replica-deterministic (same blocks, same order
⇒ same map) and derivable from history, so it need not enter the hashed state digest. **Additive,
non-consensus-affecting until 2.3 reads it.** Do NOT use the `Cell.timestamp` field — block-logistics
§4D and `lib.rs:765` forbid timestamp as a consensus lever; the cumulative-work clock is the lever.

### 2.2 Clearing rule (cliff — RATIFIED D2)
A cell is **cleared** at `now` iff `finalized_at(cell) ≤ now − W` — a hard cliff (Will ruled: cliff,
not ramp). Cells younger than `W` are **pending**: they still earn reward/influence (unchanged), but
contribute **zero** finality-safety weight. This is the "usable-face vs gameable-face" split the
audit forced, applied at the cell-age granularity.

### 2.3 The bridge (the consensus-affecting step)
`fn finality_pom_weight(ledger, now, W) -> per-contributor f64` =
`pom_scores_with_similarity_floor_q16(cells filtered to finalized_at ≤ now − W, θ_sim)`, aggregated
per contributor. This is the production `Standing → Validator.pom` map: `Validator.pom` in the live
finality path becomes this **cleared** score instead of a test-constructor value. Cleared score =
the score computed from only work old enough to have survived a full dispute window. **Build-cold /
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

## 3. Build stages (each RED→GREEN, blast-radius increasing) — CORRECTED
1. **Phase 1 — cell finalization stamp (SAFE, additive).** Add `Ledger.finalized_at: HashMap<cell_id,
   u64>`, populated in `apply()` with `Ledger::now()` when a cell enters the ledger. Tests: a cell is
   stamped with the work-time of the block that finalized it; replica-deterministic; existing scoring
   unchanged (nothing reads `finalized_at` for finality yet). Non-consensus.
2. **Phase 2 — cleared-score + bridge (CONSENSUS-AFFECTING, cold).** `finality_pom_weight(now, W)` =
   `pom_scores` over cells with `finalized_at ≤ now − W`, aggregated per contributor; wire it as the
   `Validator.pom` source in the live finality path. Tests: work younger than `W` contributes zero;
   work older than `W` counts; exact cliff boundary at `now − W`; genesis ⇒ nothing aged ⇒ all-zero
   ⇒ PoS-only finality (matches POM-FINALITY-TEMPORALITY §Genesis).
3. **Phase 3 — dispute-during-`W` (CONSENSUS-AFFECTING, cold).** A slash/refutation landing on a cell
   while it is still pending (< `W` old) removes its contribution before it can age into the cleared
   score; forward-only (a past finalized block is never un-finalized). Tests: finalize-then-slash a
   cell within `W` ⇒ it never reaches finality weight; a cell that survives `W` clears; past finalized
   blocks untouched by a later slash. Uses the existing dispute `window` (D1) as the exposure clock.

Phase 1 lands safely; Phases 2–3 are the consensus-affecting core and should be built cold in a fresh
low-context window (per repo convention for consensus surgery). All D1–D5 rulings are in (§4).

## 4. DECISIONS — RATIFIED (Will, 2026-07-11)

- **D1 — `W` anchor: ✅ ANCHOR TO THE DISPUTE-WINDOW.** `W ≥` the dispute/challenge window length, so
  no cell can clear faster than a challenge could land against it. A real referent already exists:
  the dispute-slashing `window` param (`lib.rs:4303`, `DISPUTE-SLASHING.md` §2, commented
  `window = W`). *Build note:* unify `W` with (or floor it to) that existing `window` rather than
  introducing a second, independent number — one dispute clock, denominated in cumulative-work units.
  Governed constant, not a controller; exact value tracks the dispute window.
- **D2 — vesting shape: ✅ CLIFF.** A cell contributes 0 finality weight until `finalized_at + W`,
  then full weight. Simplest, sharpest fraud boundary, kernel-lean (one comparison per cell).
- **D3 — data model: ✅ CELL-AGE, node-side (revised from the retracted two-bucket).** Vest at the
  cell layer via `Ledger.finalized_at` (§2.1); the soulbound `Standing` scalar and its on-VM Molecule
  encoding are **unchanged**. Lower blast radius than mutating `Standing`.
- **D4 — bound: ✅ n/a under the revised model.** No per-contributor pending deque exists to bound;
  `finalized_at` is one `u64` per finalized cell (cells are already bounded state). The old
  unbounded-deque concern is dissolved by the cell-age model.
- **D5 — `MIN_DIM_BPS` safety-path raise: ✅ DEFER.** The quorum floor now supplies the participation
  backstop; treat `MIN_DIM_BPS` as a separate lever to tune on data, not to raise blind now.

## 5. What this closes / does not close
- **Closes:** the PoM-direction circularity *at launch* (gamed v(S) must survive `W` of dispute
  before it can vote finality), and the missing production franchise bridge. Makes the coupled-PoM
  finality design honest in code rather than 🟡-designed.
- **Does NOT close:** v(S) un-gameability itself (the moat — separate, structurally-demonstrated /
  learned-open per STATUS-LEDGER MOAT-1). `W` is the *stand-in* that buys time for the immune system;
  it is not the immune system.
