//! M2a-1 — PoW work-dimension arithmetic + data model (additive; no consensus wiring yet).
//!
//! Proves the portable target math (`noesis_core::pow`) and the additive block data model
//! (`PowSeal` + `Block.pow` + wire serde-default) are correct and replay-parity-safe BEFORE M2a-2
//! wires them into `validate_block`/`block_work`. The retarget rule + every numeric constant
//! (genesis bits, interval, Ergon params) are ⚑ M3 and deliberately absent. Each test names the
//! anti-theater break that turns it RED.

use noesis::runtime::{Block, PowSeal};
use noesis::wire::{decode_block, encode_block};
use noesis_core::pow::{compact_to_target, hash, put, target_to_compact, work_from_target};

fn empty_block(pow: Option<PowSeal>) -> Block {
    // A wire test needs a Block value, not a consensus-valid one (encode/decode never validates).
    Block { height: 7, cells: vec![], coords: vec![], token_txs: vec![], coinbase: None, pow }
}

// ---- core target math ----

/// A compact target must reject every malformed encoding rather than silently decode a bad target.
#[test]
fn compact_to_target_rejects_malformed() {
    assert!(compact_to_target((3 << 24) | 0x0080_0001).is_none(), "sign bit set must reject");
    assert!(compact_to_target(5 << 24).is_none(), "zero mantissa must reject");
    assert!(compact_to_target(0x0000_0001).is_none(), "exponent 0 ⇒ zero target must reject");
    assert!(compact_to_target((40 << 24) | 0x0000_0001).is_none(), "byte overflow must reject");
    // ANTI-THEATER: drop the sign-bit, final-all-zero-target, or byte-overflow guard ⇒ the matching
    // case returns Some ⇒ RED. (The zero-mantissa guard is a redundant early-out subsumed by the final
    // all-zero-target check, so it alone is not independently exercised here — Council 2026-07-13.)
}

/// A well-formed compact target decodes, and a HARDER (smaller) target is worth strictly MORE work.
/// This is the property `block_work` will rely on: work is monotone in mined difficulty.
#[test]
fn work_is_monotone_in_difficulty() {
    let hard = compact_to_target((3 << 24) | 0x0000_0002).expect("target = 2 decodes"); // tiny target
    let easy = compact_to_target((32 << 24) | 0x0000_ffff).expect("~2^248 target decodes"); // huge target
    assert!(
        work_from_target(&hard) > work_from_target(&easy),
        "a harder (smaller) target must yield more work: hard={} easy={}",
        work_from_target(&hard),
        work_from_target(&easy)
    );
    // a mid pair keeps the order too (no accidental inversion at scale)
    let mid_hard = compact_to_target((10 << 24) | 0x0000_ffff).unwrap();
    let mid_easy = compact_to_target((30 << 24) | 0x0000_ffff).unwrap();
    assert!(work_from_target(&mid_hard) >= work_from_target(&mid_easy));
    // ANTI-THEATER: invert the compare in the long division (ge branch) ⇒ work decreases with
    // difficulty ⇒ RED.
}

/// The easiest possible target (all-ones) is worth the MINIMUM work of 1, and a hard target
/// saturates to u64::MAX (bounded, never wraps).
#[test]
fn work_is_floored_at_one_and_saturates() {
    assert_eq!(work_from_target(&[0xff; 32]), 1, "easiest target ⇒ minimum work 1");
    // target = 2 (NOT 1): its true quotient's low 64 bits are 0x5555_5555_5555_5555, which DIFFERS
    // from u64::MAX — so a wrap (from a missing saturation guard) is distinguishable here, unlike
    // target = 1 whose low 64 bits happen to equal u64::MAX (Council 2026-07-13).
    let hard = compact_to_target((3 << 24) | 0x0000_0002).expect("target = 2 decodes");
    assert_eq!(work_from_target(&hard), u64::MAX, "target 2 ⇒ ~2^255 work ⇒ saturates, never wraps");
    // ANTI-THEATER: remove the `target + 1` overflow early-return (`return 1`) ⇒ all-ones divides by a
    // wrapped-to-0 divisor ⇒ yields u64::MAX not 1 ⇒ the first assert goes RED. Remove the
    // `q > u64::MAX >> 1` saturation guard ⇒ target=2 wraps to 0x5555_5555_5555_5555 ≠ u64::MAX ⇒ the
    // second assert goes RED.
}

/// `target_to_compact` is the strict inverse of `compact_to_target` on the canonical compact space:
/// every `bits` the decoder accepts survives a compact→target→compact round-trip. This is the exact
/// property the M3 retarget controller relies on to emit a candidate block's `bits` from a target.
#[test]
fn target_to_compact_roundtrips_canonical_bits() {
    // canonical nBits: sign bit clear, no redundant leading-zero mantissa byte — except the sign-forced
    // leading zero in 0x1d00ffff, the real Bitcoin max target.
    for &bits in &[0x0102_0000u32, 0x0512_3456, 0x1b04_04cb, 0x1d00_ffff] {
        let target = compact_to_target(bits).expect("canonical bits decode");
        assert_eq!(
            target_to_compact(&target),
            Some(bits),
            "compact→target→compact must round-trip: {bits:#010x}"
        );
    }
    // a wider sweep across exponents with a fixed canonical mantissa (0x0abcde, sign bit clear).
    for exp in 4u32..=32 {
        let bits = (exp << 24) | 0x000a_bcde;
        let target = compact_to_target(bits).expect("swept canonical bits decode");
        assert_eq!(target_to_compact(&target), Some(bits), "round-trip must hold at exponent {exp}");
    }
    // ANTI-THEATER: return a constant, or drop the `size` computation ⇒ the swept exponents no longer
    // match ⇒ RED.
}

/// The encoder never lets the mantissa's top bit read as a sign bit — it grows the exponent and inserts
/// a leading-zero mantissa byte instead (why Bitcoin's max target is 0x1d00ffff, not 0x1cffff00).
#[test]
fn target_to_compact_never_sets_the_sign_bit() {
    // the real Bitcoin max target must encode to its canonical 0x1d00ffff.
    let max_target = compact_to_target(0x1d00_ffff).unwrap();
    assert_eq!(target_to_compact(&max_target), Some(0x1d00_ffff), "max target ⇒ canonical 0x1d00ffff");
    // a target whose top significant byte has bit 7 set must still produce a sign-bit-clear mantissa
    // and decode back byte-exactly.
    let mut t = [0u8; 32];
    t[10] = 0x80;
    let bits = target_to_compact(&t).expect("a nonzero target encodes");
    assert_eq!(bits & 0x0080_0000, 0, "the encoded mantissa must never set the sign bit");
    assert_eq!(compact_to_target(bits), Some(t), "sign-normalized bits decode back to the exact target");
    // ANTI-THEATER: remove the `mantissa & 0x00800000` shift-and-grow ⇒ the max target encodes to a
    // sign-bit-set value the decoder would reject ⇒ the first assert goes RED.
}

/// The encoder is total: a zero target is rejected (unmeetable, mirroring the decoder), and even the
/// not-exactly-representable all-ones target encodes to a valid compact rather than panicking.
#[test]
fn target_to_compact_is_total() {
    assert!(target_to_compact(&[0u8; 32]).is_none(), "the zero target is unmeetable ⇒ None");
    let bits = target_to_compact(&[0xff; 32]).expect("all-ones target must encode (truncated), not panic");
    assert!(compact_to_target(bits).is_some(), "the produced bits must itself be a decodable compact");
    // ANTI-THEATER: drop the all-zero `?` early-return ⇒ target_to_compact(&[0;32]) returns Some(0) ⇒ RED.
}

// ---- data model + wire ----

/// A block carrying a PoW seal round-trips byte-stably and the seal survives decode intact.
#[test]
fn pow_seal_wire_roundtrips() {
    let b = empty_block(Some(PowSeal { bits: (20 << 24) | 0x0000_abcd, nonce: 0xDEAD_BEEF }));
    let bytes = encode_block(&b);
    let decoded = decode_block(&bytes).expect("decode a well-formed block");
    let bytes2 = encode_block(&decoded);
    assert_eq!(bytes, bytes2, "encode must be byte-stable across a round-trip");
    assert_eq!(decoded.pow, b.pow, "the PoW seal must survive decode intact");
    // ANTI-THEATER: drop the pow field from WBlock or forget the w_block/r_block conversion ⇒
    // decoded.pow is None ≠ Some ⇒ RED.
}

/// A pre-M2 block log (no `pow` field) still decodes — to `pow: None` — via the serde-default
/// contract. This is the replay/restart-compatibility guarantee (identical to the coinbase precedent).
#[test]
fn pre_m2_block_log_decodes_pow_none() {
    // A block line written before M2a-1 existed: no `pow` (and no `coinbase`) key at all.
    let legacy = br#"{"height":7,"cells":[],"coords":[],"token_txs":[]}"#;
    let decoded = decode_block(legacy).expect("a pre-M2 block log must still decode");
    assert!(decoded.pow.is_none(), "a missing pow field must default to None");
    assert!(decoded.coinbase.is_none(), "the coinbase serde-default precedent still holds");
    // ANTI-THEATER: remove #[serde(default)] on WBlock.pow ⇒ this decode fails with a missing-field
    // error ⇒ RED (the persistence-divergence guard).
}

/// The domain-separated hasher and the length-prefix framer (also M2a-1 scope) are deterministic
/// and injective — the header preimage they will assemble in M2a-2 is unambiguous.
#[test]
fn pow_hash_and_put_are_deterministic_and_injective() {
    assert_eq!(hash(b"abc"), hash(b"abc"), "hash must be deterministic");
    assert_ne!(hash(b"abc"), hash(b"abd"), "distinct inputs ⇒ distinct digests");
    // Length-prefix framing must be injective: ["a"] ++ ["bc"] must not collide with ["ab"] ++ ["c"]
    // (the concatenation ambiguity an unframed encoding would allow).
    let mut a = Vec::new();
    put(&mut a, b"a");
    put(&mut a, b"bc");
    let mut b = Vec::new();
    put(&mut b, b"ab");
    put(&mut b, b"c");
    assert_ne!(a, b, "length-prefix framing must be injective (no concatenation ambiguity)");
    // ANTI-THEATER: replace `put`'s length prefix with a bare extend_from_slice ⇒ a == b ⇒ RED.
}
