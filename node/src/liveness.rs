//! Never-halt liveness — the stall detector (the 2nd of the two never-halt pieces).
//!
//! Noesis's never-halt invariant ([[noesis-never-halt-chain]]): a stall must be STRUCTURALLY
//! impossible; failures degrade QUALITY (cadence, difficulty-regulation) never LIVENESS. The
//! mechanism has two structural pieces:
//!
//!   1. **min-difficulty floor** — ALREADY BUILT, lives in `noesis_core::pow::next_target`'s
//!      `clamp_and_encode`: the ASERT retarget can ease difficulty DOWN to `pow_limit` (the easiest
//!      representable target) but the result is always floor-clamped, so difficulty can never rise
//!      to an unmeetable level via the schedule. This handles the "difficulty drifted too hard"
//!      case whenever the retarget has a clock signal.
//!   2. **stall detector** — THIS module. The failure the floor alone does NOT cover: the chain has
//!      STOPPED producing blocks, so the retarget never runs to ease anything (`observed = None` ⇒
//!      no retarget, `next_target` §1203). The insight that breaks the deadlock: a production halt
//!      does NOT stop the committee members' own wall-clocks. The SAME bonded committee that attests
//!      the physical clock (`wallclock.rs`, `docs/DESIGN-committee-attested-clock.md` §7) keeps
//!      ticking through the halt, so "it has been too long since the last block" stays ATTESTABLE
//!      even though the chain itself is not advancing. When a bonded supermajority attests the stall,
//!      the next block's target snaps to the floor so the easiest possible block revives the chain.
//!
//! CONSENSUS-ISOLATED SHADOW MODULE (the `jul`/`reserve`/`wallclock` precedent): pure, total,
//! integer-only, imports nothing, called from no consensus path, never touches `state_digest`. This
//! ships the DETECTION + SELECTION semantics as a tested kernel. The Phase-2 wiring — the stall
//! attestation gossip, the bonded-supermajority threshold on the stall claim, and calling
//! [`liveness_bits`] in the block-admission/retarget path — is deploy-coupled and NOT here.
//!
//! HONEST BOUNDARY (the never-halt tradeoff, [[noesis-never-halt-chain]]): snapping to the floor
//! trades QUALITY for LIVENESS. During a real stall a modest miner (even an adversary who caused it)
//! can then produce floor-difficulty blocks cheaply — but (a) the stall claim needs a bonded
//! supermajority (as-trustless-as a double-spend), (b) the work-clock ceiling (inc-M3-2) caps the
//! per-block clock advance regardless, and (c) floor difficulty ⇒ LOW `block_work` ⇒ LOW JUL reward,
//! so revival is not a minting windfall. Liveness is preserved; only cadence/issuance-rate degrade.

/// The chain is STALLED iff more than `max_interval` of committee-attested wall-clock time has
/// elapsed since the last block: `now − last_block_time > max_interval`.
///
/// `now` and `last_block_time` are committee-attested physical times (the `wallclock.rs` feed); both
/// survive a production halt because they read the committee's own clocks, not the chain's height.
/// `saturating_sub`: a backward pair (`now < last_block_time`, which a monotone-admission check
/// rejects upstream — `wallclock::advances_monotonically`) yields 0 ⇒ NOT stalled, never underflows.
///
/// `max_interval` is the ⚑ never-halt tolerance (design open-number, testnet-pinned): the maximum
/// acceptable gap before the network declares the chain stuck, naturally expressed as
/// `K · RetargetParams::ideal_interval` (tolerate K missed blocks). Too tight ⇒ a benign slow patch
/// false-triggers the emergency floor (cadence noise); too loose ⇒ a genuine stall persists longer
/// before rescue. Left caller-supplied so this kernel carries NO economic number (the `wallclock`/
/// `next_target` precedent — the mechanism is decision-unblocked, the number is Will/M3-gated).
pub fn stalled(now: u64, last_block_time: u64, max_interval: u64) -> bool {
    now.saturating_sub(last_block_time) > max_interval
}

/// The emergency-floor override: the compact target for the next block given the ASERT schedule's
/// proposal and the stall verdict.
///
/// - not stalled ⇒ the schedule governs, returned UNCHANGED (inert / byte-identical to no override —
///   the anti-theater default that keeps this module invisible on the happy path).
/// - stalled ⇒ `pow_limit_bits`, the easiest representable target (min-difficulty floor). It is
///   meetable by construction, so a stalled chain ALWAYS receives a meetable target ⇒ liveness
///   cannot be lost to difficulty. Once blocks flow again the ASERT schedule re-tightens over the
///   subsequent retargets (that graduated path already lives in `next_target`), so snap-to-floor +
///   ASERT-re-tighten is the complete control loop — no separate graduated-easing mechanism needed.
///
/// Pure selection: it does NOT re-encode or clamp (both inputs are already canonical compacts — the
/// schedule bits from `next_target`, `pow_limit_bits` from the `Constitution`). Total, never panics.
pub fn liveness_bits(schedule_bits: u32, pow_limit_bits: u32, stalled: bool) -> u32 {
    if stalled {
        pow_limit_bits
    } else {
        schedule_bits
    }
}
