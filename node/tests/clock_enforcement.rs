//! inc-CLK-2 — clock enforcement (the consensus wiring; flag-gated on `Constitution.clock_enforced`).
//!
//! inc-CLK-1 shipped the block `timestamp` field + the pure admission SEMANTICS as inert additive data.
//! This proves the WIRING of the enforcement BUNDLE (the interdependent-enforcement lesson: the ordering
//! rule ships WITH the admission guard its never-halt safety depends on):
//!   1. `header_digest` binds the timestamp (a solved seal cannot be replayed onto an altered time),
//!   2. `validate_block` enforces a PRESENT, NON-DECREASING timestamp (deterministic, replay-safe),
//!   3. `Node::admits` is the NODE-LOCAL forward-skew bound (non-deterministic, kept OUT of replay).
//! Flag-OFF parity is proven for free by the lib + parity suites (they run with `clock_enforced=false`).
//! `clock_enforced ⇒ pow_enforced` (the timestamp's anti-malleability needs the seal), so blocks here
//! are mined. Each test names its anti-theater break. ⚑ δ = 120 s ratified 2026-07-14 (≈ the interval).

use noesis::commit_order::Committed;
use noesis::runtime::{
    apply_block, header_digest, validate_block, Block, Constitution, Ledger, Node, Violation,
};
use noesis::runtime::PowSeal;
use noesis::{Cell, Script};
use noesis_core::pow::compact_to_target;

// EASY_BITS: a near-maximal target ⇒ almost any hash meets it (work ~1), so mining is instant.
const EASY_BITS: u32 = (33 << 24) | 0x0000_ffff;
// A finite work-clock ceiling — the genesis-admission precondition of `pow_enforced` (inc-M3-2), so a
// clock-enforced Constitution passes `Node::new`.
const CEIL: u64 = 1_000_000;
// ⚑ node-local clock tolerance (Will 2026-07-14): 120 s, ≈ the 120 s ordering-block interval.
const DELTA: u64 = 120;

fn valid_block(height: u64) -> Block {
    let c = Cell {
        id: height,
        lock: Script { code_hash: [1u8; 32], args: b"owner-key".to_vec() },
        type_script: Script { code_hash: [2u8; 32], args: b"alice".to_vec() },
        parent: None,
        timestamp: height, // the CELL's own field — distinct from the BLOCK timestamp under test
        data: format!("clk enforcement payload at height {height}, genuinely novel content").into_bytes(),
    };
    Block::assemble(height, &[(c, Committed { height, secret: [7u8; 32] })])
}

/// Mine a nonce whose header hash meets target(bits). The block's `timestamp` is now header-bound, so
/// callers MUST set it (via `.with_timestamp`) BEFORE mining, exactly as they set the coinbase/token_txs.
fn mine(mut b: Block, bits: u32) -> Block {
    let target = compact_to_target(bits).expect("valid bits");
    for nonce in 0u64..2_000_000 {
        b.pow = Some(PowSeal { bits, nonce });
        if header_digest(&b) <= target {
            return b;
        }
    }
    panic!("no nonce met {bits:#x} within budget");
}

fn clock_con() -> Constitution {
    Constitution {
        pow_enforced: true,
        clock_enforced: true,
        work_clock_ceiling: CEIL,
        ..Constitution::default()
    }
}

/// Under enforcement a block MUST carry a timestamp (the seal-missing analog) that is NON-DECREASING
/// over the previous block's — present + `≥ prev`. Equality is allowed (same-tick blocks); running
/// backward is rejected. This is the deterministic, replay-safe half of clock validity.
#[test]
fn enforced_requires_present_nondecreasing_timestamp() {
    let con = clock_con();
    let mut ledger = Ledger::new();
    ledger.last_timestamp = 100; // a prior block already stamped time 100

    // (a) no timestamp under enforcement ⇒ TimestampMissing.
    let bare = mine(valid_block(1), EASY_BITS);
    assert!(
        matches!(validate_block(&ledger, &bare, &con), Err(Violation::TimestampMissing)),
        "enforced + no timestamp ⇒ TimestampMissing"
    );

    // (b) a backward timestamp (99 < 100) ⇒ TimestampNotMonotone.
    let back = mine(valid_block(1).with_timestamp(99), EASY_BITS);
    assert!(
        matches!(
            validate_block(&ledger, &back, &con),
            Err(Violation::TimestampNotMonotone { prev: 100, got: 99 })
        ),
        "enforced + backward timestamp ⇒ TimestampNotMonotone"
    );

    // (c) an EQUAL timestamp (100 == 100) ⇒ accepted (same-tick is allowed, per the kernel's non-strict rule).
    let equal = mine(valid_block(1).with_timestamp(100), EASY_BITS);
    assert!(
        validate_block(&ledger, &equal, &con).is_ok(),
        "enforced + equal timestamp ⇒ accepted (non-decreasing, not strict)"
    );

    // (d) a forward timestamp (101 > 100) ⇒ accepted.
    let fwd = mine(valid_block(1).with_timestamp(101), EASY_BITS);
    assert!(validate_block(&ledger, &fwd, &con).is_ok(), "enforced + forward timestamp ⇒ accepted");
    // ANTI-THEATER: make clock_check return Ok() unconditionally ⇒ (a),(b) pass validation ⇒ RED. Make
    // it STRICT (`>`) ⇒ (c) goes RED (contradicting wallclock::advances_monotonically's non-strict contract).
}

/// Flag OFF (the default) ⇒ the timestamp is inert additive data: not required, monotonicity unchecked,
/// and — because `last_timestamp` is EXCLUDED from `state_digest` — carrying a timestamp does not change
/// consensus state. Byte-identical to pre-CLK-2.
#[test]
fn clock_off_is_inert_and_excluded_from_digest() {
    let off = Constitution::default(); // clock_enforced == false, pow_enforced == false
    let ledger = Ledger::new();

    // not required when off (no seal needed either — pow is off): a plain block validates.
    let plain = valid_block(1);
    assert!(validate_block(&ledger, &plain, &off).is_ok(), "clock off ⇒ timestamp not required");

    // the timestamp must not leak into consensus state: a stamped and an unstamped block converge.
    let stamped = valid_block(1).with_timestamp(12_345);
    let a = apply_block(Ledger::new(), &plain, &off).expect("plain applies");
    let b = apply_block(Ledger::new(), &stamped, &off).expect("stamped applies");
    assert_eq!(
        a.state_digest(),
        b.state_digest(),
        "the block timestamp must not enter state_digest (last_timestamp is excluded)"
    );
    // ...but the baseline DID advance under the hood (it is real, just non-consensus-hashed).
    assert_eq!(b.last_timestamp, 12_345, "apply still advances the monotonicity baseline");
    assert_eq!(a.last_timestamp, 0, "an unstamped block leaves the baseline unchanged");
    // ANTI-THEATER: add `last_timestamp` to the state_digest tuple ⇒ the equal-digest assert goes RED.
}

/// `header_digest` binds the block timestamp: a solved PoW cannot be replayed onto a block with an
/// altered time (which would let a producer change the monotonicity comparand / retarget input
/// post-solve). So changing the timestamp — including presence vs absence — MUST change the digest.
#[test]
fn header_digest_binds_timestamp() {
    let t1 = valid_block(1).with_timestamp(1000);
    let t2 = valid_block(1).with_timestamp(2000);
    let none = valid_block(1);
    assert_ne!(header_digest(&t1), header_digest(&t2), "timestamp VALUE must change the header digest");
    assert_ne!(header_digest(&none), header_digest(&t1), "timestamp PRESENCE must change the header digest");
    // ANTI-THEATER: drop the timestamp binding from header_digest ⇒ both asserts go RED (a solved seal
    // would replay across times).
}

/// `Node::admits` is the NODE-LOCAL forward-skew bound — the never-halt guard. A far-future timestamp
/// PASSES deterministic validation (it is non-decreasing), which is exactly the gap a live clock must
/// close: admission rejects it against the node's own clock so it never enters the canonical chain,
/// where it would freeze the clock's forward progress. The bound is kept OUT of the replay path.
#[test]
fn admits_gates_forward_skew_that_validation_cannot() {
    let node = Node::new(1, vec![], clock_con());
    let local_now = 1_000_000u64;

    // in-tolerance (timestamp == local clock, monotone over baseline 0) ⇒ admitted.
    let ok = mine(valid_block(1).with_timestamp(local_now), EASY_BITS);
    assert!(node.validate(&ok), "in-tolerance block is deterministically valid");
    assert!(node.admits(&ok, local_now, DELTA), "a timestamp within δ of the local clock is admitted");

    // far-future (u64::MAX) ⇒ PASSES deterministic monotonicity but admission REJECTS.
    let far = mine(valid_block(1).with_timestamp(u64::MAX), EASY_BITS);
    assert!(
        node.validate(&far),
        "far-future block passes deterministic monotonicity — the gap the admission bound exists to close"
    );
    assert!(
        !node.admits(&far, local_now, DELTA),
        "a timestamp far beyond δ is rejected at node-local admission (the never-halt / anti-freeze guard)"
    );
    // ANTI-THEATER: define admits == validate (drop the timestamp_admissible term) ⇒ the far-future
    // reject goes RED (the deterministic path alone cannot catch a far-future stamp).
}

/// Genesis-admission: `clock_enforced` without `pow_enforced` is a misconfigured Constitution — the
/// monotone-timestamp rule would run on a field the seal does not header-bind (the inc-CLK-1
/// weaponizable shape). `Node::new` fails LOUD at genesis, never a live hole.
#[test]
#[should_panic(expected = "clock_enforced requires pow_enforced")]
fn clock_enforced_without_pow_is_rejected_at_genesis() {
    let con = Constitution { clock_enforced: true, pow_enforced: false, ..Constitution::default() };
    let _ = Node::new(1, vec![], con);
}

/// End-to-end over a two-block chain: apply advances the baseline, and the NEXT block is checked
/// against the advanced baseline — a backward successor is rejected, a non-decreasing one accepted.
#[test]
fn monotone_chain_advances_baseline() {
    let con = clock_con();
    let b1 = mine(valid_block(1).with_timestamp(1000), EASY_BITS);
    let l1 = apply_block(Ledger::new(), &b1, &con).expect("b1 applies");
    assert_eq!(l1.last_timestamp, 1000, "apply advances the monotonicity baseline to the block's timestamp");

    // a backward successor (999 < 1000) is rejected against the advanced baseline.
    let b2_back = mine(valid_block(2).with_timestamp(999), EASY_BITS);
    assert!(
        matches!(validate_block(&l1, &b2_back, &con), Err(Violation::TimestampNotMonotone { .. })),
        "a successor running backward vs the advanced baseline is rejected"
    );

    // a forward successor advances the baseline again.
    let b2 = mine(valid_block(2).with_timestamp(1001), EASY_BITS);
    assert!(validate_block(&l1, &b2, &con).is_ok(), "a non-decreasing successor is accepted");
    let l2 = apply_block(l1, &b2, &con).expect("b2 applies");
    assert_eq!(l2.last_timestamp, 1001, "the baseline advances across the chain");
    // ANTI-THEATER: skip the last_timestamp update in apply_transition ⇒ l1.last_timestamp stays 0 ⇒ the
    // first assert goes RED and the backward-successor check no longer bites.
}
