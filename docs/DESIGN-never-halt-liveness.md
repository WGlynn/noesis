# DESIGN — Never-halt liveness: the stall detector

> Status: ready-for-critique. Kernel ✅ built (`node/src/liveness.rs`, `node/tests/liveness.rs`,
> 6 RED-first tests). Phase-2 wiring 🟡 designed-not-built. `max_interval` ⚑ (testnet-pinned).
> Grounded in [[noesis-never-halt-chain]] + `docs/DESIGN-committee-attested-clock.md` §7 + DESIGN-M3 §5.

## 1. The invariant

Halting must be **structurally impossible**. Failures degrade **quality** (cadence, difficulty
regulation, issuance rate) — never **liveness**. No L1 pause, no "halt issuance at height H" fallback
(DESIGN-M3 §5 corrected: that fallback is FORBIDDEN). This is a standing design lens on every
mechanism; this note builds the piece that owns it directly.

## 2. Two structural pieces (one was already built)

**Piece 1 — min-difficulty floor. ✅ already built** in `noesis_core::pow::next_target`'s
`clamp_and_encode` (lib.rs:1241): the ASERT retarget can ease difficulty DOWN to `pow_limit` (the
easiest representable target) and every result is floor-clamped, so difficulty can never rise to an
unmeetable level *through the schedule*. This covers "difficulty drifted too hard" **whenever the
retarget runs**.

**Piece 2 — stall detector. ✅ this increment.** The gap piece 1 does not cover: the chain has
**stopped producing blocks**, so the retarget never runs (`observed = None` ⇒ anchor unchanged,
next_target:1203) and nothing eases. Circular: no blocks ⇒ no retarget ⇒ difficulty stuck ⇒ no blocks.

## 3. The insight that breaks the deadlock

A production halt does **not** stop the committee members' own wall-clocks. The SAME bonded committee
that attests the physical clock (`wallclock.rs`, committee-clock design §7) keeps ticking through the
halt, so *"it has been too long since the last block"* stays **attestable even though the chain is not
advancing**. The stall is observable from a clock source that is independent of the very thing that
stalled. When a bonded supermajority attests the stall, the next block's target snaps to the floor and
the easiest possible block revives the chain.

This is why the stall detector belongs to the committee-clock family and reuses its trust model
(as-trustless-as a double-spend: moving the attested "stalled" verdict requires a bonded supermajority,
every node an independent witness against its own clock).

## 4. The kernel (built)

Pure/total/integer shadow module (the `jul`/`reserve`/`wallclock` precedent — no consensus wiring,
never touches `state_digest`):

- `stalled(now, last_block_time, max_interval) -> bool` — `now.saturating_sub(last_block_time) >
  max_interval`. `now`/`last_block_time` are committee-attested physical times (the `wallclock` feed),
  both survive a halt. `saturating_sub` ⇒ a backward pair (rejected upstream by
  `wallclock::advances_monotonically`) is not a stall and never underflows. Strict `>` at the boundary
  (equality is not yet a stall — cadence noise fence).
- `liveness_bits(schedule_bits, pow_limit_bits, stalled) -> u32` — not stalled ⇒ the ASERT schedule
  passes through **unchanged** (inert / invisible on the happy path); stalled ⇒ `pow_limit_bits`, the
  min-difficulty floor, **unconditionally** (any schedule ⇒ floor). The floor is meetable by
  construction ⇒ a stalled chain always gets a meetable target ⇒ liveness cannot be lost to difficulty.

**The complete control loop:** snap-to-floor on stall + ASERT re-tighten as blocks resume (the
graduated re-tightening already lives in `next_target`). No separate graduated-easing mechanism —
that would be YAGNI over what `next_target` already does.

## 5. Honest boundary — the never-halt tradeoff

Snapping to the floor trades **quality for liveness** (the invariant's explicit bargain). During a real
stall a modest miner — even an adversary who caused it — can then produce floor-difficulty blocks
cheaply. Three structural fences keep that from becoming an exploit:

1. the stall claim needs a **bonded supermajority** attestation (not a single node's clock);
2. the **work-clock ceiling** (inc-M3-2) caps the per-block cumulative-work advance regardless of
   difficulty, so the emergency floor cannot fast-forward the economic clock;
3. floor difficulty ⇒ **low `block_work`** ⇒ **low JUL reward**, so revival is not a minting windfall.

Liveness is preserved; only cadence and issuance-rate degrade — exactly the intended shape.

## 6. Phase-2 wiring (🟡 designed-not-built, deploy-coupled)

Not in this increment (touches `validate_block`, the wire format, the dispute/attestation path, the
bonded set's keys):

- a **stall attestation** message: committee members sign `(last_block_time, their_now)` when their
  local clock shows the gap past `max_interval`;
- the **bonded-supermajority threshold** on aggregated stall attestations (the same threshold family as
  finality / the clock deviation-challenge);
- calling `liveness_bits` in the **block-admission / retarget path** so the emergency floor governs the
  next accepted block's target when a stall is attested.

## 7. Open

- ⚑ **`max_interval`** — the never-halt tolerance, naturally `K · RetargetParams::ideal_interval`
  (tolerate K missed blocks). Testnet-pinned with δ/max_staleness (committee-clock design §5/§9).
- 🔬 whether the stall attestation is a plain aggregate or BLS-signed (cost only lands in a rare fired
  stall — leanest-that-adjudicates wins; mirrors the committee-clock in-dispute-reference question).
- 🔬 hysteresis: should the floor persist for N blocks after revival to avoid re-stall flapping, or is
  a single floor-block + ASERT re-tighten sufficient? (Default: single block — simplest; revisit if a
  testnet shows flapping.)
