//! L4 launch-copy honesty pins (`internal/LOOP-PLAN-to-golive.md`, loop L4).
//!
//! These regression-guard the three economic properties Noesis launch copy is allowed to
//! claim about the value-settlement layer:
//!   1. **burn-only slashing** — slashed standing is destroyed, never transferred to another
//!      account (the topological/collusion cross-path slash has no payout target);
//!   2. **zero protocol fee** — a closed dispute's sink set is EXACTLY
//!      `{challenger_payout, author_compensation, burned}` and closes to its sources with no
//!      residual; there is no fee bucket skimming value (grep of `node/src` finds no `fee`);
//!   3. **bounded challenger bounty** — β is fenced to `[0,1]`, so the resolver can never
//!      become a mint (`burned >= 0`) even under a misconfigured β.
//!
//! Each test names the anti-theater break that turns it RED, so a future edit that quietly
//! violates a launch claim fails here. Grounded at `node/src/lib.rs` (`dispute` module):
//! `resolve_refuted` :4543, `resolve_upheld` :4655, `unified_settlement` :4854.

use noesis::dispute::{
    resolve_refuted, resolve_upheld, unified_settlement, Challenge, Params, Settlement,
    VestingEntry,
};
use std::collections::{HashMap, HashSet};

const EPS: f64 = 1e-9;

/// P1 — a pure (topological/collusion) slash destroys standing; it is never transferred.
/// `unified_settlement` composes two source slashes into ONE settlement whose only sink is
/// `burned`. If a future edit ever routed slashed standing to a payee, this goes RED.
#[test]
fn slash_burns_are_destroyed_never_transferred() {
    let collusion = Settlement {
        slashes: vec![(b"ring".to_vec(), 5.0)],
        burned: 5.0,
        ..Default::default()
    };
    let refutation = Settlement {
        slashes: vec![(b"certifier".to_vec(), 3.0)],
        burned: 3.0,
        ..Default::default()
    };
    let mut standing = HashMap::new();
    standing.insert(b"ring".to_vec(), 100u64);
    standing.insert(b"certifier".to_vec(), 100u64);

    let s = unified_settlement(&collusion, &refutation, &HashSet::new(), &standing);
    let total_slashed: f64 = s.slashes.iter().map(|(_, a)| *a).sum();

    // Every slashed unit is burned; NOTHING is paid to any account.
    assert!(
        (s.burned - total_slashed).abs() < EPS,
        "slashed standing must be fully burned (mint<->sink), got burned={} slashed={}",
        s.burned,
        total_slashed
    );
    assert_eq!(s.challenger_payout, 0.0, "a pure slash has no bounty recipient");
    assert_eq!(s.author_compensation, 0.0, "a pure slash compensates no one");
    assert_eq!(s.canceled, 0.0, "a cross-path slash cancels no vesting value");
    // ANTI-THEATER: divert any slash amount into `challenger_payout`/`author_compensation`
    // (a transfer) instead of `burned` in `unified_settlement` (lib.rs:4854) -> RED.
}

/// P2 — β is fenced. Even a misconfigured β > 1 cannot turn the resolver into a mint:
/// the `.clamp(0.0, 1.0)` on β keeps `burned >= 0` and keeps the bounty <= the slashed pool.
#[test]
fn beta_bounty_is_fenced_and_cannot_mint() {
    let p = Params {
        window: 10,
        lambda: 1.0,
        alpha: 0.0,
        beta: 3.0, // MISCONFIG: > 1. Unclamped this would pay bounty = 3 * slashed.
        gamma: 0.0,
    };
    let mut entries = vec![VestingEntry { cell_id: 7, amount: 10.0, realized_epoch: 0 }];
    let c = Challenge {
        target: 7,
        challenger: b"challenger".to_vec(),
        bond: 1.0,
        opened_epoch: 5, // < realized_epoch + window (0 + 10) => still unvested => cancelable
    };
    let shares = vec![(b"cert".to_vec(), 10.0)];

    let s = resolve_refuted(&mut entries, &c, &p, &shares);
    let total_slashed: f64 = s.slashes.iter().map(|(_, a)| *a).sum();

    // The fence: the clamp prevents a negative burn (a mint from nowhere).
    assert!(
        s.burned >= 0.0,
        "beta clamp must prevent the resolver from minting (burned={})",
        s.burned
    );
    // The bounty can never exceed the slashed pool it is drawn from.
    assert!(
        s.challenger_payout <= c.bond + total_slashed + EPS,
        "bounty must stay within bond + slashed pool"
    );
    // Full mint<->sink balance: sources == sinks, nothing created or destroyed off-book.
    let sources = c.bond + s.canceled + total_slashed;
    let sinks = s.challenger_payout + s.author_compensation + s.burned;
    assert!((sources - sinks).abs() < EPS, "mint<->sink must balance");
    // ANTI-THEATER: drop the `.clamp(0.0, 1.0)` on beta (lib.rs:4567) -> bounty=30 ->
    // burned = 10 + 10 - 30 = -10 -> RED.
}

/// P3 — zero protocol fee. Across both dispute outcomes, the sink set is complete
/// (`challenger_payout`, `author_compensation`, `burned`) and closes exactly to its sources.
/// There is no fourth "fee" bucket siphoning value to the protocol.
#[test]
fn settlement_has_no_protocol_fee_leak() {
    // Refuted path (challenge succeeds).
    let p = Params { window: 10, lambda: 1.0, alpha: 0.5, beta: 0.2, gamma: 0.3 };
    let mut entries = vec![VestingEntry { cell_id: 1, amount: 8.0, realized_epoch: 0 }];
    let c = Challenge { target: 1, challenger: b"c".to_vec(), bond: 2.0, opened_epoch: 3 };
    let shares = vec![(b"x".to_vec(), 4.0), (b"y".to_vec(), 4.0)];

    let s = resolve_refuted(&mut entries, &c, &p, &shares);
    let total_slashed: f64 = s.slashes.iter().map(|(_, a)| *a).sum();
    let sources = c.bond + s.canceled + total_slashed;
    let sinks = s.challenger_payout + s.author_compensation + s.burned;
    assert!(
        (sources - sinks).abs() < EPS,
        "refuted: no value may leak to a protocol fee (sources={sources} sinks={sinks})"
    );

    // Upheld path (nuisance challenge): the bond splits into author compensation + burn ONLY.
    let up = resolve_upheld(&c, &p);
    assert!(
        (up.author_compensation + up.burned - c.bond).abs() < EPS,
        "upheld: the whole bond is author-compensation or burned -- no protocol fee"
    );
    assert_eq!(up.canceled, 0.0, "upheld cancels nothing");
    assert_eq!(up.challenger_payout, 0.0, "a forfeited bond pays the challenger nothing");
    // ANTI-THEATER: add any `protocol_fee = k * bond` skim to either resolver and the
    // matching conservation assertion goes RED -- there is no fee term in the code today.
}
