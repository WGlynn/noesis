//! Never-halt liveness stall-detector kernel (`node/src/liveness.rs`) — additive shadow, no consensus
//! wiring. Proves the DETECTION predicate + the emergency-floor SELECTION are correct and total. The
//! stall attestation gossip, the bonded-supermajority threshold, and the retarget-path call are
//! Phase-2 (deploy-coupled) and deliberately absent. `max_interval` is ⚑ (testnet-pinned), so every
//! test supplies its own. Each test names the anti-theater break that turns it RED.

use noesis::liveness::{liveness_bits, stalled};

// A stand-in floor-difficulty compact and a (harder) schedule compact. Values are arbitrary canonical
// compacts — the selector never decodes them, it only chooses between them.
const POW_LIMIT_BITS: u32 = 0x1d00_ffff; // easiest / min-difficulty floor (Bitcoin genesis-style)
const SCHEDULE_BITS: u32 = 0x1b00_ffff; // a harder scheduled target

// ---- stall detection ----

/// A gap strictly greater than `max_interval` is a stall; equal-or-less is not. RED if the boundary
/// uses `>=` (would false-trigger the emergency floor exactly at tolerance — cadence noise).
#[test]
fn stalled_only_strictly_past_tolerance() {
    let max = 600u64;
    assert!(!stalled(1000, 500, max), "gap 500 ≤ 600 must NOT be a stall");
    assert!(!stalled(1100, 500, max), "gap 600 == tolerance must NOT be a stall (boundary)");
    assert!(stalled(1101, 500, max), "gap 601 > 600 must be a stall");
}

/// A backward pair (now < last) is NOT a stall and must never underflow. RED if `saturating_sub` is
/// replaced by `-`: `500 - 1000` panics (debug) / wraps to a huge gap (release) ⇒ false stall.
#[test]
fn stalled_backward_pair_is_not_a_stall_and_never_underflows() {
    assert!(!stalled(500, 1000, 600), "now before last ⇒ 0 elapsed ⇒ not stalled, no underflow");
    assert!(!stalled(0, u64::MAX, 0), "extreme backward pair saturates to 0, still not stalled");
}

/// Zero tolerance means ANY positive gap is a stall (a valid ⚑ extreme, e.g. a test/step network).
/// RED if the predicate special-cases 0 rather than using the uniform `> max_interval`.
#[test]
fn stalled_zero_tolerance_flags_any_positive_gap() {
    assert!(!stalled(500, 500, 0), "no elapsed time is never a stall, even at zero tolerance");
    assert!(stalled(501, 500, 0), "one tick past, zero tolerance ⇒ stall");
}

// ---- emergency-floor selection ----

/// NOT stalled ⇒ the schedule target passes through UNCHANGED (inert / invisible on the happy path).
/// RED if the selector ever perturbs the schedule when there is no stall.
#[test]
fn liveness_bits_inert_when_not_stalled() {
    assert_eq!(
        liveness_bits(SCHEDULE_BITS, POW_LIMIT_BITS, false),
        SCHEDULE_BITS,
        "no stall ⇒ ASERT schedule governs, byte-identical to no override"
    );
}

/// Stalled ⇒ the next target snaps to the min-difficulty floor so the chain revives. RED if a stall
/// leaves the (harder) schedule difficulty in place — the exact halt this mechanism exists to prevent.
#[test]
fn liveness_bits_snaps_to_floor_when_stalled() {
    assert_eq!(
        liveness_bits(SCHEDULE_BITS, POW_LIMIT_BITS, true),
        POW_LIMIT_BITS,
        "stall ⇒ emergency min-difficulty floor, never the unmet schedule target"
    );
    assert_ne!(
        liveness_bits(SCHEDULE_BITS, POW_LIMIT_BITS, true),
        SCHEDULE_BITS,
        "anti-theater: the stall override must actually CHANGE the target (floor ≠ schedule here)"
    );
}

/// Never-halt property: for ANY schedule bits, a stall yields the floor — liveness cannot be lost to
/// difficulty regardless of how hard the schedule drifted. RED if the override is conditional on the
/// schedule value instead of unconditional on the stall verdict.
#[test]
fn liveness_bits_stall_always_yields_floor_for_any_schedule() {
    for schedule in [0x0300_0001u32, 0x1700_ffff, 0x1b00_ffff, 0x1d00_ffff, u32::MAX] {
        assert_eq!(
            liveness_bits(schedule, POW_LIMIT_BITS, true),
            POW_LIMIT_BITS,
            "stall must yield the floor for every schedule target (never-halt is unconditional)"
        );
    }
}
