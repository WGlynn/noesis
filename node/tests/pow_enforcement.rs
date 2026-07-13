//! M2a-2 — PoW enforcement (the consensus wiring; flag-gated on `Constitution.pow_enforced`).
//!
//! M2a-1 shipped the arithmetic + data model (additive). This proves the WIRING: under enforcement
//! `validate_block` requires a valid work PROOF, `block_work` returns the seal's DERIVED chainwork
//! (making JUL Lever-A live), and PoW stays out of finality. Flag-OFF parity is proven for free by
//! the 323 lib + parity suites (they run with the default `pow_enforced=false`). The difficulty
//! RETARGET rule + genesis bits are ⚑ M3, absent here. Each test names its anti-theater break.

use noesis::commit_order::Committed;
use noesis::runtime::finality::FINALITY_MIX;
use noesis::runtime::{
    apply_block, block_work, header_digest, validate_block, Block, Constitution, Ledger, PowSeal,
    TokenStandard, TokenTx, Violation,
};
use noesis::{Cell, Script};
use noesis_core::pow::{compact_to_target, work_from_target};

// EASY_BITS: a near-maximal target ⇒ almost any hash meets it (work ~1), so mining is instant.
const EASY_BITS: u32 = (33 << 24) | 0x0000_ffff;
// HARDER_BITS: ~2^248 target ⇒ hash needs a zero top byte (~256 tries), still a fast unit-test mine,
// and strictly more chainwork than EASY_BITS.
const HARDER_BITS: u32 = (32 << 24) | 0x0000_ffff;

fn valid_block(height: u64) -> Block {
    // A block valid on every axis EXCEPT the PoW seal (so the pow_check is what the test isolates).
    let c = Cell {
        id: height,
        lock: Script { code_hash: [1u8; 32], args: b"owner-key".to_vec() },
        type_script: Script { code_hash: [2u8; 32], args: b"alice".to_vec() },
        parent: None,
        timestamp: height,
        data: format!("m2a2 enforcement payload at height {height}, genuinely novel content").into_bytes(),
    };
    Block::assemble(height, &[(c, Committed { height, secret: [7u8; 32] })])
}

/// Mine: find a nonce whose header hash meets target(bits). Bounded — EASY/HARDER converge fast.
fn mine(mut b: Block, bits: u32) -> Block {
    let target = compact_to_target(bits).expect("valid bits");
    for nonce in 0u64..2_000_000 {
        b.pow = Some(PowSeal { bits, nonce });
        if header_digest(&b) <= target {
            return b;
        }
    }
    panic!("no nonce met {bits:#x} within budget (target too hard for a unit test)");
}

fn enforced() -> Constitution {
    Constitution { pow_enforced: true, ..Constitution::default() }
}

/// Under enforcement the seal must be a valid work PROOF: no seal ⇒ PowMissing, an unmet or malformed
/// target ⇒ PowUnmet, a properly mined seal ⇒ accepted. This is the not-attacker-choosable property.
#[test]
fn enforced_requires_a_valid_work_proof() {
    let con = enforced();
    let ledger = Ledger::new();

    // (a) no seal under enforcement ⇒ PowMissing.
    let bare = valid_block(1);
    assert!(
        matches!(validate_block(&ledger, &bare, &con), Err(Violation::PowMissing)),
        "enforced + no seal ⇒ PowMissing"
    );

    // (b) a seal claiming HARDER_BITS whose header does NOT meet that target ⇒ PowUnmet.
    let target = compact_to_target(HARDER_BITS).unwrap();
    let mut unmet = valid_block(1);
    unmet.pow = Some(PowSeal { bits: HARDER_BITS, nonce: 0 });
    for nonce in 0u64..100_000 {
        unmet.pow = Some(PowSeal { bits: HARDER_BITS, nonce });
        if header_digest(&unmet) > target {
            break;
        }
    }
    assert!(header_digest(&unmet) > target, "constructed an unmet seal");
    assert!(
        matches!(validate_block(&ledger, &unmet, &con), Err(Violation::PowUnmet)),
        "enforced + header does not meet its claimed target ⇒ PowUnmet"
    );

    // (c) a malformed compact target (sign bit set) ⇒ PowUnmet.
    let mut malformed = valid_block(1);
    malformed.pow = Some(PowSeal { bits: (3 << 24) | 0x0080_0001, nonce: 0 });
    assert!(
        matches!(validate_block(&ledger, &malformed, &con), Err(Violation::PowUnmet)),
        "enforced + malformed bits ⇒ PowUnmet"
    );

    // (d) a properly mined seal ⇒ accepted.
    let mined = mine(valid_block(1), EASY_BITS);
    assert!(
        validate_block(&ledger, &mined, &con).is_ok(),
        "enforced + a valid mined proof ⇒ accepted"
    );
    // ANTI-THEATER: make pow_check return Ok() unconditionally ⇒ (a),(b),(c) pass validation ⇒ RED.
}

/// block_work returns the seal's DERIVED chainwork under enforcement (never a carried number), and
/// WORK_PER_BLOCK (1) when the flag is off — so a harder mined target contributes strictly more work.
#[test]
fn enforced_block_work_is_derived_chainwork() {
    let con = enforced();
    let easy = mine(valid_block(1), EASY_BITS);
    let harder = mine(valid_block(1), HARDER_BITS);

    let w_easy = block_work(&easy, &con);
    let w_harder = block_work(&harder, &con);

    // DERIVED from the target, bit-for-bit — not read from a producer-carried field.
    assert_eq!(w_easy, work_from_target(&compact_to_target(EASY_BITS).unwrap()));
    assert_eq!(w_harder, work_from_target(&compact_to_target(HARDER_BITS).unwrap()));
    assert!(w_harder > w_easy, "harder mined target ⇒ more work: {w_harder} > {w_easy}");

    // Flag OFF ⇒ WORK_PER_BLOCK regardless of the seal (byte-identical to pre-M2). Asserted on
    // `harder` (derived work 256), NOT `easy` (derived work 1): at easy the flag-off result (1) is
    // indistinguishable from always-derive, so the break below would slip through (Council 2026-07-13).
    assert_eq!(
        block_work(&harder, &Constitution::default()),
        1,
        "flag off ⇒ block_work == WORK_PER_BLOCK"
    );
    // ANTI-THEATER: have block_work ignore `c.pow_enforced` (always derive) ⇒ the flag-off assert
    // returns 256 not 1 ⇒ RED; have it return a constant under enforcement ⇒ the derivation/ordering
    // asserts go RED.
}

/// JUL Lever-A is live under enforcement: the coinbase mints reward_for_work(DERIVED work), so a
/// harder mined block issues strictly more JUL. This is the whole point of the work dimension.
#[test]
fn enforced_issuance_scales_with_mined_work() {
    let con = enforced();
    let recipient = Script { code_hash: [9u8; 32], args: b"miner".to_vec() };
    let easy = mine(valid_block(1).with_coinbase(recipient.clone()), EASY_BITS);
    let harder = mine(valid_block(1).with_coinbase(recipient.clone()), HARDER_BITS);

    let after_easy = apply_block(Ledger::new(), &easy, &con).expect("easy coinbase block applies");
    let after_harder = apply_block(Ledger::new(), &harder, &con).expect("harder coinbase block applies");

    assert!(after_easy.jul_supply.issued() > 0, "a coinbase block under enforcement mints JUL");
    assert!(
        after_harder.jul_supply.issued() > after_easy.jul_supply.issued(),
        "harder mined block ⇒ more work ⇒ strictly more JUL (Lever A live): {} > {}",
        after_harder.jul_supply.issued(),
        after_easy.jul_supply.issued()
    );
    // ANTI-THEATER: if block_work ignored the seal under enforcement, both blocks would mint the same
    // JUL ⇒ RED.
}

/// M2a-2 wires PoW into issuance + the work-clock ONLY; finality safety stays PoS+PoM (PoW is
/// reorgeable ⇒ off the safety path). This pins that the exclusion is intact.
#[test]
fn pow_stays_out_of_finality() {
    assert_eq!(FINALITY_MIX.pow, 0.0, "PoW must never weight finality");
    // ANTI-THEATER: add a nonzero pow term to FINALITY_MIX ⇒ RED (locks the reorgeable-layer exclusion).
}

fn out_cell(id: u64) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [4u8; 32], args: vec![id as u8] },
        type_script: Script { code_hash: [5u8; 32], args: b"tok".to_vec() },
        parent: None,
        timestamp: 0,
        data: vec![id as u8],
    }
}

/// header_digest must bind token-tx OUTPUT ORDER and AUTHS (Council 2026-07-13 binding fix): a solved
/// PoW must NOT replay onto a block with reordered outputs (state-divergent under apply, since
/// apply_transition appends in slice order) or swapped auths (differently-authorized once
/// CONTROL_BINDING_ACTIVE flips). So changing either MUST change the digest.
#[test]
fn header_digest_binds_token_tx_order_and_auths() {
    let tx = |outputs: Vec<Cell>, auths: Vec<Vec<u8>>| TokenTx {
        standard: TokenStandard::Fungible,
        code_hash: [3u8; 32],
        args: vec![],
        inputs: vec![],
        outputs,
        auths,
    };
    let block = |t: TokenTx| valid_block(1).with_token_txs(vec![t]);
    let (x, y) = (out_cell(1), out_cell(2));

    let d_xy = header_digest(&block(tx(vec![x.clone(), y.clone()], vec![vec![1], vec![2]])));
    let d_yx = header_digest(&block(tx(vec![y.clone(), x.clone()], vec![vec![1], vec![2]])));
    let d_auth = header_digest(&block(tx(vec![x.clone(), y.clone()], vec![vec![2], vec![1]])));

    assert_ne!(d_xy, d_yx, "token-output ORDER must change the header digest (else state-divergent replay)");
    assert_ne!(d_xy, d_auth, "token AUTHS must change the header digest (else auth-divergent replay)");
    // ANTI-THEATER: revert header_digest to commit token_txs via tx.digest() (canonicalized +
    // auth-free) ⇒ d_xy == d_yx AND d_xy == d_auth ⇒ RED.
}
