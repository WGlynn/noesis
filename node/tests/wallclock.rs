//! Wall-clock validation kernel (Phase-2 timestamp source) — the pure predicates behind the
//! committee-attested clock (`docs/DESIGN-committee-attested-clock.md`). Additive shadow module; each
//! test names the anti-theater break that turns it RED.

use noesis::wallclock::{advances_monotonically, observed_elapsed, within_tolerance};

/// A reported time within ±delta of a node's own clock is accepted; beyond it (future OR past) rejected.
/// This is the per-node universal-validation check whose supermajority-to-defeat property is the whole
/// trust argument (design §1/§2).
#[test]
fn within_tolerance_accepts_inside_the_band_rejects_outside_both_ways() {
    let (local, delta) = (1_000u64, 60u64);
    assert!(within_tolerance(1_000, local, delta), "exact match accepted");
    assert!(within_tolerance(1_060, local, delta), "+delta boundary accepted (future edge)");
    assert!(within_tolerance(940, local, delta), "-delta boundary accepted (past edge)");
    assert!(!within_tolerance(1_061, local, delta), "just past +delta rejected (a future-dated lie)");
    assert!(!within_tolerance(939, local, delta), "just past -delta rejected (a stale/backdated lie)");
    // ANTI-THEATER: make the check one-sided (reported <= local + delta only) ⇒ the -delta reject RED.
}

/// The tolerance check never overflows/underflows at the u64 extremes — it is total (the transition-purity
/// contract), computed via abs-diff not a subtraction that could wrap.
#[test]
fn within_tolerance_is_total_at_the_extremes() {
    assert!(!within_tolerance(u64::MAX, 0, 5), "MAX vs 0 is way outside any small band — no wrap to 'accept'");
    assert!(!within_tolerance(0, u64::MAX, 5), "0 vs MAX likewise rejected (symmetric, no underflow)");
    assert!(within_tolerance(u64::MAX, u64::MAX, 0), "equal extremes accepted at delta 0");
    // ANTI-THEATER: replace abs_diff with `reported - local` ⇒ panics/wraps on 0 - MAX ⇒ RED (or false-accept).
}

/// Monotonicity is non-decreasing (equal allowed) so two blocks in the same clock tick are legal, but a
/// backward-running time is rejected. This is the deterministic, replay-safe half of timestamp validity.
#[test]
fn monotonicity_allows_equal_but_rejects_backward() {
    assert!(advances_monotonically(1_000, 1_000), "same tick allowed (honest fast blocks)");
    assert!(advances_monotonically(1_001, 1_000), "forward allowed");
    assert!(!advances_monotonically(999, 1_000), "backward rejected (no timewarp-back)");
    // ANTI-THEATER: require strict `>` ⇒ the same-tick case RED (would reject honest same-second blocks).
}

/// The retarget feed is `now - anchor`, and it is total: a non-monotone `(now < anchor)` pair yields `None`
/// (the caller rejects it) rather than underflowing. Feeds `next_target`'s `observed` term.
#[test]
fn observed_elapsed_subtracts_and_is_total() {
    assert_eq!(observed_elapsed(1_500, 1_000), Some(500), "elapsed = now - anchor");
    assert_eq!(observed_elapsed(1_000, 1_000), Some(0), "zero elapsed at the anchor itself");
    assert_eq!(observed_elapsed(999, 1_000), None, "now < anchor ⇒ None (non-monotone, never underflow)");
    assert_eq!(observed_elapsed(0, u64::MAX), None, "extreme non-monotone ⇒ None, no wrap");
    // ANTI-THEATER: use `now - anchor` (unchecked) ⇒ the now<anchor cases panic/wrap ⇒ RED.
}
