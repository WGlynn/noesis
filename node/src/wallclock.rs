//! Wall-clock validation kernel (Phase-2 timestamp source) — the pure predicates behind the
//! committee-attested clock (`docs/DESIGN-committee-attested-clock.md`).
//!
//! Noesis runs TWO clocks (`[[noesis-time-is-read-not-faked]]`): the economic/ordering clock is
//! cumulative work (`now() == work`, deterministic, the chain owns it); the PHYSICAL clock is
//! time-of-day, an external fact the chain READS trust-minimized rather than fakes. This module is the
//! read side's validation logic: the per-node checks a validator applies to a reported wall-clock time,
//! and the elapsed-time value that feeds `noesis_core::pow::next_target`'s `observed` term.
//!
//! CONSENSUS-ISOLATED SHADOW MODULE (the `jul`/`reserve` precedent): pure, total, integer-only, no
//! consensus wiring, never touches `state_digest`. The block `timestamp` field, the deviation-challenge
//! gossip path, stake-weighted dispute adjudication, and slashing are Phase-2 (deploy-coupled) and NOT
//! here. This ships the validation SEMANTICS as a tested kernel so the Phase-2 wiring builds on verified
//! ground.
//!
//! DETERMINISM BOUNDARY (load-bearing, design §4): [`within_tolerance`] is a NODE-LOCAL admission check
//! (each node uses its OWN live clock, like Bitcoin's future-time bound) — it must NEVER enter a state
//! transition or `state_digest`. [`advances_monotonically`] and [`observed_elapsed`] are deterministic
//! (functions of block-carried values only) and safe on the replay path.

/// A reported wall-clock time is ACCEPTABLE to a validating node iff it sits within `delta` of that
/// node's own local clock reading: `|reported − local| ≤ delta`. This is the per-node universal-validation
/// predicate — every node is an independent witness with its own ground-truth clock, so moving network
/// time past `delta` requires a bonded supermajority (the same threshold as a double-spend). Symmetric:
/// it bounds both a future-dated and a past-dated `reported` by `delta`.
///
/// NODE-LOCAL / NON-DETERMINISTIC: `local` is the caller's live clock, so this is an admission rule, not a
/// replay rule. Never fold its result into `state_digest`. `delta` is the ⚑ tolerance band (design §5):
/// too tight ⇒ honest clock-skew false-alarms; too loose ⇒ a bounded, finality-safe ±delta grind margin.
pub fn within_tolerance(reported: u64, local: u64, delta: u64) -> bool {
    reported.abs_diff(local) <= delta
}

/// A reported time must not run backward relative to the previous accepted time: `reported ≥ prev`.
/// NON-DECREASING (not strictly increasing) on purpose — two blocks may legitimately land within the same
/// clock tick, and forbidding equality would reject honest fast blocks. Monotonicity is the deterministic,
/// replay-safe half of timestamp validity (it reads only block-carried values, never a local clock).
pub fn advances_monotonically(reported: u64, prev: u64) -> bool {
    reported >= prev
}

/// The retarget's observed-elapsed feed: the time between the retarget anchor and now, as consumed by
/// `noesis_core::pow::next_target`'s `observed` argument. `None` when `now < anchor` (a non-monotone pair
/// the caller must reject before it reaches the retarget) — total, never underflows. Deterministic: a
/// function of two block-carried attested times, safe on the replay path.
pub fn observed_elapsed(now: u64, anchor: u64) -> Option<u64> {
    now.checked_sub(anchor)
}
