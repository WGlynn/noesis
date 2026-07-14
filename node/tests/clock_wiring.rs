//! inc-CLK-1 — committee-attested-clock INERT ADDITIVE data model + wired semantics (the M2a-1
//! precedent: carry the field + ship the pure validation helpers, but NO consensus path reads or
//! enforces it yet). Proves: the `timestamp` field round-trips on the wire (legacy logs default to
//! None); the field is inert w.r.t. state (never enters `state_digest`, replays deterministically even
//! for absurd/ancient values); the NODE-LOCAL admission SEMANTICS (`timestamp_admissible`) are correct
//! and — the load-bearing property — the same block the live node would refuse at admission is still
//! ACCEPTED by the deterministic rulebook (tolerance must never reach replay/sync); and the
//! `observed_elapsed → next_target` seam behaves.
//!
//! DEFERRED to inc-CLK-2 (shipped as ONE coherent enforcement unit, with the ⚑ numbers): the
//! deterministic monotonicity consensus rule + its magnitude-bounding admission INGRESS + the header
//! binding + the live retarget activation. inc-CLK-1 ships no live ordering rule (a monotonicity rule
//! without its magnitude guard permanently bricks the channel on one `Some(u64::MAX)` block — the
//! Council finding that collapsed this increment back to pure additive data). Each test names its
//! anti-theater break.

use noesis::commit_order::Committed;
use noesis::runtime::{apply_block, timestamp_admissible, validate_block, Block, Constitution, Ledger};
use noesis::wallclock::observed_elapsed;
use noesis::wire::{decode_block, encode_block};
use noesis::{Cell, Script};
use noesis_core::pow::{compact_to_target, next_target, RetargetParams};

fn cell(id: u64, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
        parent: None,
        timestamp: 0, // this is the CELL timestamp (unrelated to the block wall-clock field)
        data: data.to_vec(),
    }
}

/// A valid single-cell block at `height`, optionally carrying a committee-attested wall-clock `ts`.
fn blk(height: u64, ts: Option<u64>) -> Block {
    let data = format!("content for height {height} distinct enough for novelty");
    let proposals = vec![(cell(height, data.as_bytes()), Committed { height, secret: [height as u8; 32] })];
    let b = Block::assemble(height, &proposals);
    match ts {
        Some(t) => b.with_timestamp(t),
        None => b,
    }
}

const ANCHOR: u32 = 0x1b00_ffff; // a mid-hardness compact target, well below the pow_limit floor
fn rp() -> RetargetParams {
    // ideal_interval · height_delta(10) = expected 1000; pow_limit_bits is the (easier) min-difficulty
    // floor. All caller-supplied — inc-CLK-1 hard-codes NO economic number (the inc-M3-1 precedent).
    RetargetParams { ideal_interval: 100, half_life: 1000, pow_limit_bits: 0x1f00_ffff }
}

// ---- (a) additive wire round-trip ----

/// timestamp round-trips through the codec, and a pre-CLK block log (no field) decodes as None.
/// ANTI-THEATER: drop `#[serde(default)]` on `WBlock.timestamp` ⇒ the legacy decode fails RED; forget
/// the w_block/r_block mapping ⇒ Some round-trips to None RED.
#[test]
fn timestamp_wire_roundtrips_and_legacy_decodes_none() {
    let b = blk(7, Some(123_456));
    let bytes = encode_block(&b);
    let decoded = decode_block(&bytes).expect("decode a well-formed timestamped block");
    assert_eq!(decoded.timestamp, Some(123_456), "timestamp must survive the round-trip");
    assert_eq!(encode_block(&decoded), bytes, "re-encode is byte-stable");

    let none = decode_block(&encode_block(&blk(7, None))).expect("decode a None-timestamp block");
    assert!(none.timestamp.is_none(), "a None timestamp round-trips as None");

    let legacy = br#"{"height":7,"cells":[],"coords":[],"token_txs":[]}"#;
    let dl = decode_block(legacy).expect("a pre-CLK block log must still decode");
    assert!(dl.timestamp.is_none(), "a missing timestamp field must default to None");
}

// ---- (b) the field is INERT w.r.t. state ----

/// Two otherwise-identical blocks differing ONLY in timestamp yield equal `state_digest` — the field
/// is retarget metadata, not state identity. ANTI-THEATER: fold the timestamp into the `StateDigest`
/// tuple (or into any applied `Ledger` field) ⇒ the None-vs-Some digests differ RED.
#[test]
fn timestamp_never_enters_state_digest() {
    let con = Constitution::default();
    let with_ts = apply_block(Ledger::new(), &blk(1, Some(5000)), &con).expect("some applies");
    let without = apply_block(Ledger::new(), &blk(1, None), &con).expect("none applies");
    assert_eq!(
        with_ts.state_digest(),
        without.state_digest(),
        "the physical timestamp must stay off the digest (inert additive data)"
    );
}

/// A chain of ANCIENT (and one ABSURD) timestamps replays to a byte-identical digest from two fresh
/// ledgers, and every block is accepted by the deterministic rulebook — the determinism-boundary leak
/// detector. ANTI-THEATER: wire the local clock (or any monotonicity/magnitude rule) into the rulebook
/// ⇒ these epoch-ancient / u64::MAX blocks get rejected against the test machine's real clock or a
/// bound ⇒ RED (the failure a late-joining node would hit on `sync.rs`'s re-validation).
#[test]
fn absurd_and_stale_timestamps_replay_deterministically() {
    let con = Constitution::default();
    let chain = [blk(1, Some(1)), blk(2, Some(u64::MAX)), blk(3, Some(2))];

    let mut a = Ledger::new();
    let mut b = Ledger::new();
    for blk in &chain {
        a = apply_block(a, blk, &con).expect("replica A applies the stale/absurd chain");
        b = apply_block(b, blk, &con).expect("replica B applies the stale/absurd chain");
    }
    assert_eq!(a.state_digest(), b.state_digest(), "an absurd-timestamp chain must replay deterministically");
    assert_eq!(a.height, 3, "anti-theater: the chain was non-trivial");
}

// ---- (c) node-local admission SEMANTICS — and they must NEVER be in the rulebook ----

/// The tolerance band gates ADMISSION (node-local, live clock) but the SAME block passes the
/// deterministic rulebook — the two surfaces give different answers on purpose. ANTI-THEATER: merge
/// `within_tolerance` into `validate_block`/`Node::validate` ⇒ the rulebook rejects the ancient block
/// RED (and a real late joiner would reject the whole chain).
#[test]
fn tolerance_gates_admission_but_never_the_replay_rulebook() {
    let con = Constitution::default();
    let delta = 600u64;
    let local_now = 10_000u64;

    let ancient = blk(1, Some(1)); // far outside any sane tolerance of `local_now`
    assert!(!timestamp_admissible(&ancient, local_now, delta), "ancient timestamp is inadmissible live");
    assert!(
        validate_block(&Ledger::new(), &ancient, &con).is_ok(),
        "the deterministic rulebook must ACCEPT it — tolerance is an admission concern, not a replay one"
    );

    assert!(timestamp_admissible(&blk(1, Some(local_now)), local_now, delta), "in-band admissible");
    assert!(
        timestamp_admissible(&blk(1, Some(local_now + delta)), local_now, delta),
        "boundary +delta is admissible"
    );
    assert!(
        !timestamp_admissible(&blk(1, Some(local_now + delta + 1)), local_now, delta),
        "one past +delta is inadmissible"
    );
    assert!(timestamp_admissible(&blk(1, None), local_now, delta), "a None timestamp is inert-admissible");
}

// ---- (d) the observed_elapsed → next_target seam ----

/// Elapsed time feeds the retarget in the right direction, and the no-signal case is inert (anchor
/// unchanged), distinct from a zero-elapsed case. ANTI-THEATER: swap now/anchor in the feed ⇒ direction
/// inverts; feed `Some(0)` where `None` is meant ⇒ the "no data" case hardens maximally RED.
#[test]
fn observed_elapsed_feeds_next_target_direction_and_none_is_inert() {
    let p = rp();
    let anchor = compact_to_target(ANCHOR).unwrap();

    let slow = compact_to_target(next_target(ANCHOR, 10, observed_elapsed(6000, 1000), p).unwrap()).unwrap();
    let fast = compact_to_target(next_target(ANCHOR, 10, observed_elapsed(1100, 1000), p).unwrap()).unwrap();
    assert!(slow > anchor, "slower than schedule (elapsed 5000 > expected 1000) ⇒ easier (larger) target");
    assert!(fast < anchor, "faster than schedule (elapsed 100 < expected 1000) ⇒ harder (smaller) target");

    let none = compact_to_target(next_target(ANCHOR, 10, None, p).unwrap()).unwrap();
    assert_eq!(none, anchor, "no clock signal ⇒ anchor unchanged (Phase-1 inert)");

    // observed_elapsed is total + correctly typed for the seam.
    assert_eq!(observed_elapsed(6000, 1000), Some(5000), "elapsed = now − anchor");
    assert_eq!(observed_elapsed(1000, 6000), None, "now < anchor ⇒ None (non-monotone, rejected upstream)");

    // the sneaky one: Some(0) ("infinitely fast") is NOT the same retarget input as None ("no data").
    let zero = compact_to_target(next_target(ANCHOR, 10, Some(0), p).unwrap()).unwrap();
    assert!(zero < anchor, "Some(0) elapsed ⇒ maximally hard, distinct from None's no-op");
}

// ---- (e) never-halt ----

/// A chain with NO timestamps validates, applies, and the retarget over it returns the anchor (a
/// value, never None/panic) — a missing clock degrades to Phase-1, never a halt. ANTI-THEATER: make a
/// missing timestamp a Violation ⇒ every pre-CLK block is invalid RED (the never-halt guard).
#[test]
fn missing_clock_signal_never_halts() {
    let con = Constitution::default();
    let mut state = Ledger::new();
    for h in 1..=3 {
        state = apply_block(state, &blk(h, None), &con).expect("a None-timestamp block must never be rejected");
    }
    assert_eq!(state.height, 3, "the clock-less chain produced blocks");
    assert_eq!(
        next_target(ANCHOR, 10, None, rp()),
        Some(ANCHOR),
        "no clock ⇒ retarget returns the anchor (never None/panic) ⇒ Phase-1 difficulty, never a halt"
    );
}
