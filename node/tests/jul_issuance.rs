//! JUL money layer, increment 1 — issuance-core invariants.
//!
//! Proves the deterministic, integer-only issuance primitive in ISOLATION: the module imports nothing
//! from the consensus path, so this file exercises `reward_for_work` + `JulSupply` with plain integers
//! only. Each test names the anti-theater break that turns it RED (apply it by hand to confirm the test
//! bites, then revert). Assertions are on SHAPE and DIRECTION, never on the owed economic numbers
//! (`docs/DESIGN-jul-money-layer.md` §5 defers those), so a later parameter pinning can't invalidate them.

use noesis::jul::{reward_for_work, JulParams, JulSupply, JUL_BASE_UNITS};

/// Fair launch: a fresh supply is zero — no pre-mine, no allocation (design §2).
/// RED break: seed a nonzero `issued` in `JulSupply::new`/`Default`.
#[test]
fn genesis_supply_is_zero_no_premine() {
    assert_eq!(JulSupply::new().issued(), 0);
    assert_eq!(JulSupply::default().issued(), 0);
}

/// Determinism + replica convergence: the same work log yields identical rewards on repeated
/// evaluation, and two independently constructed supplies fed that log end `Eq`-identical.
/// RED break: fold any nondeterministic value (clock, address) into `reward_for_work`/`credit`.
#[test]
fn reward_is_deterministic_and_replicas_converge() {
    let p = JulParams::default();
    let works = [0u64, 1, 7, 1_000, 999_983, u32::MAX as u64];

    for &w in &works {
        assert_eq!(reward_for_work(w, p), reward_for_work(w, p), "same inputs → same reward");
    }

    let mut a = JulSupply::new();
    let mut b = JulSupply::new();
    for &w in &works {
        a.credit(reward_for_work(w, p));
    }
    for &w in &works {
        b.credit(reward_for_work(w, p));
    }
    assert_eq!(a, b, "two replicas fed the same log converge to identical state");
    let expected: u128 = works.iter().map(|&w| reward_for_work(w, p)).sum();
    assert_eq!(a.issued(), expected);
}

/// Supply is monotone nondecreasing and equals the exact sum of the rewards folded in (accounting
/// conserves; a zero-work block adds nothing but never subtracts).
/// RED break: flip `credit`'s `saturating_add` to `saturating_sub`, or make a retarget deduct supply.
#[test]
fn supply_is_monotone_and_equals_sum_of_rewards() {
    let p = JulParams::default();
    let mut s = JulSupply::new();
    let mut prev = 0u128;
    let mut sum = 0u128;
    for w in [3u64, 0, 5, 100, 2, 0] {
        let r = reward_for_work(w, p);
        let now = s.credit(r);
        assert!(now >= prev, "supply must never decrease");
        prev = now;
        sum += r;
    }
    assert_eq!(s.issued(), sum, "issued == Σ rewards");
}

/// Totality at the full u64 headroom: the max rational product is exact (fits u128, no wrap), and the
/// cumulative supply saturates at u128::MAX rather than wrapping.
/// RED break: replace a `u128` intermediate with a raw `u64 *`, or `saturating_add` with `+`.
#[test]
fn no_overflow_at_u64_headroom() {
    // (2^64 − 1)^2 = 2^128 − 2^65 + 1 < u128::MAX, so this product is exact.
    let p = JulParams { reward_num: u64::MAX, reward_den: 1 };
    let expected = (u64::MAX as u128) * (u64::MAX as u128);
    assert_eq!(reward_for_work(u64::MAX, p), expected);

    let mut s = JulSupply::new();
    s.credit(u128::MAX);
    assert_eq!(s.credit(1), u128::MAX, "supply saturates, never wraps");
}

/// The anchor direction, expressible today without real PoW: more work never mints less. When
/// `block_work` later returns mined difficulty, this IS "issuance responds to difficulty in the
/// modeled direction" with no new proof needed.
/// RED break: invert the multiply into a divide (reward falling as work rises).
#[test]
fn reward_is_monotone_in_work() {
    let p = JulParams::default();
    let ascending = [0u64, 1, 2, 10, 1_000, 1_000_000, u32::MAX as u64, u64::MAX];
    for pair in ascending.windows(2) {
        assert!(
            reward_for_work(pair[0], p) <= reward_for_work(pair[1], p),
            "reward must be monotone nondecreasing in work: {} then {}",
            pair[0],
            pair[1]
        );
    }
}

/// PoW-gated at the function level (no work ⇒ no money) and exact closed-form accounting: N blocks of
/// constant work `w` issue exactly `N · floor(w · num/den)`.
/// RED break: special-case `work == 0` to a nonzero reward, or use the wrong divisor.
#[test]
fn no_work_no_money_and_exact_accounting() {
    let p = JulParams::default(); // 1 JUL (= JUL_BASE_UNITS base units) per unit of work
    assert_eq!(reward_for_work(0, p), 0, "no work, no money");

    let (n, w) = (12u128, 7u64);
    let per = reward_for_work(w, p);
    assert_eq!(per, (w as u128) * JUL_BASE_UNITS, "closed form for the default rate");

    let mut s = JulSupply::new();
    for _ in 0..n {
        s.credit(per);
    }
    assert_eq!(s.issued(), n * per, "N blocks of constant work w ⇒ issued == N · per-block");
}
