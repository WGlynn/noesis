//! JUL money layer, increment 3 — the counter-cyclical reserve (Lever B).
//!
//! Proves the reserve is a BOUNDED, work-clocked, fail-closed smoother that can never mint JUL and never
//! touch attribution or finality — and, the headline theorem, that exercising a FULLY ACTIVE reserve
//! (funded, bear, releasing) alongside a real coinbase chain leaves `state_digest`, `pom`, and
//! `jul_supply` byte-identical (increment 3 is additive/shadow ⇒ replay-parity is unaffected). Each test
//! names the anti-theater break that turns it RED.

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::jul::{self, JulParams};
use noesis::reserve::{Reserve, ReserveParams, Signal};
use noesis::runtime::{Constitution, Node};
use noesis::{Cell, Script};

// ============ fixtures (the jul_settlement idiom) ============

fn genesis() -> Node {
    let validators = vec![Validator {
        id: 0,
        pow: 0.0,
        pos: 1000.0,
        pom: 0.0,
        last_heartbeat: 0,
        staked_balance: 1000.0,
    }];
    Node::new(0, validators, Constitution::default())
}

fn cell(id: u64, contributor: &[u8], ts: u64, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: b"own".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent: None,
        timestamp: ts,
        data: data.to_vec(),
    }
}

fn committed(height: u64, s: u8) -> Committed {
    Committed { height, secret: [s; 32] }
}

fn recipient(owner: &[u8]) -> Script {
    Script { code_hash: [0u8; 32], args: owner.to_vec() }
}

/// Produce + apply one carrier block carrying an optional coinbase recipient.
fn produce(node: &mut Node, height: u64, data: &[u8], coinbase: Option<Script>) {
    node.submit(cell(height, b"alice", height, data), committed(height, height as u8));
    let mut block = node.propose();
    if let Some(r) = coinbase {
        block = block.with_coinbase(r);
    }
    assert!(node.validate(&block), "carrier block must validate");
    node.apply(&block);
    node.clear_mempool();
}

/// A bear signal (short below long): used to flip the market to a drawdown deterministically.
fn bear_signal() -> Signal {
    Signal { short: 90, long: 100 } // trend = (90−100)·10000/100 = −1000 bps
}

// ============ mechanism tests (bounds, fail-closed, conservation) ============

/// A release is bounded by BOTH the per-release rate and the remaining per-period budget, and repeated
/// releases never sum past the period cap (the sol:228-231 clamp).
/// RED break: delete the remaining-budget clamp so each release takes rate_bps of the balance unbounded.
#[test]
fn release_is_bounded_by_rate_and_period_budget() {
    let p = ReserveParams {
        skim_bps: 10_000,          // 100% skim so `accrue` funds an exact amount for the test
        bear_threshold_bps: 100,
        release_rate_bps: 2_500,   // 25% of balance per release
        max_release_per_period: 300,
        period_work: 1_000_000,    // never expires within this test
        cooldown_work: 0,
        min_assessment_work: 0,
    };
    let mut r = Reserve::new();
    assert_eq!(r.accrue(1000, p), 1000, "100% skim funds the reserve to 1000");
    r.assess(Some(bear_signal()), 1, p);
    assert!(r.is_bear());

    let first = r.release(2, p); // min(1000·0.25, 300, 1000) = 250
    let second = r.release(3, p); // min(750·0.25=187, 300−250=50, 750) = 50
    let third = r.release(4, p); // remaining budget 0 ⇒ 0
    assert_eq!((first, second, third), (250, 50, 0));
    assert!(first + second <= p.max_release_per_period, "Σ releases ≤ per-period cap");
    assert_eq!(r.balance(), 1000 - 250 - 50, "balance debited by exactly what was released");
}

/// Releases respect the cooldown on the WORK clock: refused when `now − last_release < cooldown_work`,
/// permitted exactly at the boundary. Same inputs ⇒ same outputs (no wall-clock anywhere).
/// RED break: stamp `last_release` from anything but the caller's `now`, or compare with `<=` off-boundary.
#[test]
fn releases_respect_the_cooldown_on_the_work_clock() {
    let p = ReserveParams {
        skim_bps: 10_000,
        bear_threshold_bps: 100,
        release_rate_bps: 5_000, // 50%
        max_release_per_period: u128::MAX,
        period_work: 0,
        cooldown_work: 10,
        min_assessment_work: 0,
    };
    let mut r = Reserve::new();
    r.accrue(1000, p);
    r.assess(Some(bear_signal()), 0, p);

    assert_eq!(r.release(5, p), 500, "first release (no prior) permitted");
    assert_eq!(r.release(10, p), 0, "now−last=5 < cooldown 10 ⇒ blocked");
    assert_eq!(r.release(15, p), 250, "now−last=10 == cooldown ⇒ permitted at the boundary");
}

/// The per-period budget resets deterministically INSIDE the sizing once `period_work` elapses (the
/// Solidity view-function wart, sol:214-217, fixed by construction), and two reserves fed identical
/// (signal, now) sequences are Eq-identical.
/// RED break: read stale `deployed_this_period` without the in-function period-expiry reset.
#[test]
fn period_reset_is_deterministic_not_informational() {
    let p = ReserveParams {
        skim_bps: 10_000,
        bear_threshold_bps: 100,
        release_rate_bps: 10_000, // 100%
        max_release_per_period: 100,
        period_work: 50,
        cooldown_work: 0,
        min_assessment_work: 0,
    };
    let run = || {
        let mut r = Reserve::new();
        r.accrue(1000, p);
        r.assess(Some(bear_signal()), 0, p);
        let a = r.release(10, p); // period [0,50]: min(1000, 100, 1000) = 100 (spends the budget)
        let b = r.release(20, p); // still in period, budget spent ⇒ 0
        let c = r.release(60, p); // 60 > 0+50 ⇒ period reset ⇒ fresh 100 budget ⇒ 100
        (r, a, b, c)
    };
    let (r1, a, b, c) = run();
    let (r2, _, _, _) = run();
    assert_eq!((a, b, c), (100, 0, 100));
    assert_eq!(r1, r2, "same (signal, now) sequence ⇒ Eq-identical reserves (replica determinism)");
}

/// A bear→bull transition resets the period budget (sol:161-165), but RE-assessing while STILL bear does
/// NOT — so the per-period cap holds under repeated in-bear assessments.
/// RED break: drop the was-bear guard so every assessment resets the budget (unbounded releases).
#[test]
fn bear_to_bull_transition_resets_the_period() {
    let p = ReserveParams {
        skim_bps: 10_000,
        bear_threshold_bps: 100,
        release_rate_bps: 5_000,
        max_release_per_period: 1000,
        period_work: 1_000_000, // isolate the bear→bull reset from period-expiry
        cooldown_work: 0,
        min_assessment_work: 0,
    };
    let mut r = Reserve::new();
    r.accrue(2000, p);
    r.assess(Some(bear_signal()), 1, p);
    assert_eq!(r.release(2, p), 1000, "spends the whole per-period budget (min(2000·0.5, 1000))");

    // still bear: a re-assessment must NOT reset the budget.
    r.assess(Some(bear_signal()), 3, p);
    assert_eq!(r.should_release(3, p), (false, 0), "budget stays spent while bear persists");

    // recover: bear→bull transition resets the budget.
    r.assess(Some(Signal { short: 110, long: 100 }), 4, p); // trend +1000 ⇒ not bear
    assert!(!r.is_bear());

    // back to bear: a fresh budget is available BECAUSE the recovery reset it.
    r.assess(Some(bear_signal()), 5, p);
    let (ok, amt) = r.should_release(5, p);
    assert!(ok && amt > 0, "the recovery reset the period ⇒ releases resume");
}

/// Conservation: after any accrue/release sequence `balance == Σ skims − Σ releases`, and the reserve can
/// never release more than it was funded — the coinbase stays the structurally unique inflation channel.
/// RED break: credit the skim twice in `accrue`, or let `release` exceed the balance.
#[test]
fn reserve_conserves_and_cannot_mint() {
    let p = ReserveParams {
        skim_bps: 2_500, // 25%
        bear_threshold_bps: 100,
        release_rate_bps: 5_000,
        max_release_per_period: u128::MAX,
        period_work: 0,
        cooldown_work: 0,
        min_assessment_work: 0,
    };
    let mut r = Reserve::new();
    let skimmed: u128 = [1000u128, 2000, 3000].iter().map(|reward| r.accrue(*reward, p)).sum();
    assert_eq!(skimmed, 250 + 500 + 750);
    assert_eq!(r.balance(), skimmed, "balance == Σ skims");

    r.assess(Some(bear_signal()), 1, p);
    let released: u128 = [2u64, 3, 4].iter().map(|now| r.release(*now, p)).sum();
    assert_eq!(r.balance(), skimmed - released, "balance == Σ skims − Σ releases (conservation)");
    assert!(released <= skimmed, "cannot release more than funded ⇒ cannot mint");

    // an empty reserve in a deep bear releases nothing (no mint-from-nothing).
    let mut empty = Reserve::new();
    empty.assess(Some(bear_signal()), 1, p);
    assert_eq!(empty.release(2, p), 0, "an unfunded reserve cannot release");
}

// ============ consensus-facing theorems (the increment-3 headline) ============

/// THE increment-3 theorem: a FULLY ACTIVE reserve (funded, bear, releasing) exercised alongside a real
/// coinbase chain leaves `state_digest`, `pom`, and `jul_supply` BYTE-IDENTICAL to the same chain that
/// never constructs a `Reserve`. The reserve is additive/shadow ⇒ replay-parity is unaffected. This proves
/// the CURRENT pure API is side-effect-free on the ledger (the strong, narrow property); the shape
/// regression "a reserve field folded into `state_digest`" is a separate concern guarded structurally by
/// the `StateDigest` tuple itself, not catchable here.
/// RED break: make any reserve fn take `&mut Ledger` / push a cell into `token_cells` — the exercised
/// chain would then diverge from the control.
#[test]
fn active_reserve_leaves_consensus_byte_identical() {
    let live = ReserveParams {
        skim_bps: 1_000,        // 10% of each coinbase reward
        bear_threshold_bps: 50,
        release_rate_bps: 3_000,
        max_release_per_period: u128::MAX,
        period_work: 0,
        cooldown_work: 0,
        min_assessment_work: 0,
    };
    let per = jul::reward_for_work(1, JulParams::default());
    // a crafted drawdown series (the SIGNAL SEAM feed — independent of the pre-PoW-constant ledger clock).
    let works = [10u64, 10, 8, 6, 4, 2];
    let sig = Signal::from_work_series(&works, 2, 4); // short=3, long=5 ⇒ −4000 bps ⇒ bear

    let build = |exercise: bool| -> (Node, u128) {
        let mut node = genesis();
        let mut r = Reserve::new();
        let mut released = 0u128;
        let datas: [&[u8]; 4] =
            [b"alpha novel content", b"beta novel content", b"gamma novel content", b"delta novel content"];
        for (i, d) in datas.iter().enumerate() {
            produce(&mut node, i as u64 + 1, d, Some(recipient(b"miner")));
            if exercise {
                r.accrue(per, live);
                r.assess(sig, node.ledger.now(), live);
                released += r.release(node.ledger.now(), live);
            }
        }
        (node, released)
    };

    let (a, released_a) = build(true);
    let (b, released_b) = build(false);

    assert_eq!(a.ledger.state_digest(), b.ledger.state_digest(), "reserve pipeline must not perturb consensus");
    assert_eq!(a.ledger.jul_supply.issued(), b.ledger.jul_supply.issued(), "JUL supply unaffected");
    assert_eq!(a.ledger.pom, b.ledger.pom, "attribution unaffected");
    assert!(released_a > 0, "anti-triviality: the exercised reserve genuinely funded AND released");
    assert_eq!(released_b, 0, "the control chain never ran the reserve");
}

/// MONEY NEVER BUYS STANDING (the anti-plutocracy invariant, made executable). Even a large 'donation'-
/// scale funder plus an active release leave `ledger.pom` and every digest component byte-identical — no
/// reserve operation can mint PoM standing or feed finality.
/// RED break: credit a `pom` entry to the reserve funder, or fold reserve balance into any Validator weight.
#[test]
fn money_never_buys_standing() {
    let live = ReserveParams {
        skim_bps: 10_000,
        bear_threshold_bps: 50,
        release_rate_bps: 5_000,
        max_release_per_period: u128::MAX,
        period_work: 0,
        cooldown_work: 0,
        min_assessment_work: 0,
    };
    let mut node = genesis();
    let datas: [&[u8]; 3] = [b"one novel thing here", b"two novel things here", b"three novel here"];
    for (i, d) in datas.iter().enumerate() {
        produce(&mut node, i as u64 + 1, d, Some(recipient(b"miner")));
    }
    let pom_before = node.ledger.pom.clone();
    let digest_before = node.ledger.state_digest();

    // a big funder + a deep bear + an active release — the strongest form of the invariant.
    let mut r = Reserve::new();
    r.accrue(1_000_000_000, live);
    r.assess(Some(Signal { short: 1, long: 100 }), node.ledger.now(), live); // −9900 bps ⇒ deep bear
    let out = r.release(node.ledger.now(), live);

    assert_eq!(node.ledger.pom, pom_before, "no reserve op mints PoM standing");
    assert_eq!(node.ledger.state_digest(), digest_before, "attribution + token + work digest unchanged by the reserve");
    assert!(r.balance() > 0 && out > 0, "anti-triviality: the reserve was genuinely funded and released");
}
