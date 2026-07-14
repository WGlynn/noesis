//! Sub-blocks slice 1 (T9) — the data model + contribution-weight gate + provisional-overlay
//! conservation + the confirmation-tier read side. Pure SHADOW validation: no networking, no
//! finalized-state mutation (revertible by construction). Each test names its anti-theater break.

use noesis::runtime::{Ledger, TokenStandard, TokenTx};
use noesis::subblock::{
    tier_of_output, validate_sub_block, ConfirmationTier, SubBlock, SubBlockViolation,
};
use noesis::{Cell, Script};

const TC: [u8; 32] = [7u8; 32]; // the token type-script code_hash
const ALICE: &[u8] = b"alice";
const BOB: &[u8] = b"bob";
const CAROL: &[u8] = b"carol";
const ISS: &[u8] = b"issuer";

/// A fungible cell of `amount`, id `id`, owned by `owner`, issued by `issuer`.
fn tok(id: u64, amount: u128, owner: &[u8], issuer: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [1u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: TC, args: issuer.to_vec() },
        parent: None,
        timestamp: 0,
        data: noesis::tokens::fungible::encode(amount),
    }
}

/// A conserving transfer of the whole `input` cell to `new_owner` (same amount, same issuer).
fn transfer(input: Cell, new_owner: &[u8], out_id: u64) -> TokenTx {
    let issuer = input.type_script.args.clone();
    let amount = noesis::tokens::fungible::amount(&input);
    let output = tok(out_id, amount, new_owner, &issuer);
    TokenTx {
        standard: TokenStandard::Fungible,
        code_hash: TC,
        args: issuer,
        inputs: vec![input],
        outputs: vec![output],
        auths: vec![],
    }
}

fn sub(height: u64, seq: u64, producer: &[u8], txs: Vec<TokenTx>) -> SubBlock {
    SubBlock { ordering_height: height, seq, producer: producer.to_vec(), txs }
}

/// A ledger at ordering height `h` with a PoM standing map and a finalized token set.
fn ledger_with(h: u64, standing: &[(&[u8], u64)], cells: Vec<Cell>) -> Ledger {
    let mut l = Ledger::new();
    l.height = h;
    l.token_cells = cells;
    for (id, s) in standing {
        l.pom.insert(id.to_vec(), *s);
    }
    l
}

/// A sub-block belongs to the interval AFTER the current ordering tip — it must reference that height.
#[test]
fn binds_to_current_ordering_tip() {
    let ledger = ledger_with(5, &[(ALICE, 100)], vec![]);
    let stale = sub(4, 0, ALICE, vec![]);
    assert_eq!(
        validate_sub_block(&ledger, &[], &stale, 0),
        Err(SubBlockViolation::WrongOrderingHeight { tip: 5, got: 4 }),
        "a sub-block on the wrong ordering height (wrong interval) is rejected"
    );
    let ontip = sub(5, 0, ALICE, vec![]);
    assert!(validate_sub_block(&ledger, &[], &ontip, 0).is_ok(), "a sub-block on the current tip is accepted");
    // ANTI-THEATER: drop the ordering_height check ⇒ the stale sub-block passes ⇒ RED.
}

/// Sub-blocks form a sequential soft-chain within the interval: `seq` == number of prior accepted ones.
#[test]
fn sub_blocks_must_be_sequential() {
    let ledger = ledger_with(5, &[(ALICE, 100)], vec![]);
    let prior = vec![sub(5, 0, ALICE, vec![])];
    let gap = sub(5, 2, ALICE, vec![]);
    assert_eq!(
        validate_sub_block(&ledger, &prior, &gap, 0),
        Err(SubBlockViolation::NonSequential { expected: 1, got: 2 }),
        "a seq gap in the soft-chain is rejected"
    );
    let next = sub(5, 1, ALICE, vec![]);
    assert!(validate_sub_block(&ledger, &prior, &next, 0).is_ok(), "the next sequential sub-block is accepted");
    // ANTI-THEATER: ignore `seq` ⇒ the gap passes ⇒ RED.
}

/// The CONTRIBUTION-WEIGHT gate (the Ergo-`T/64` → PoM-standing re-derivation): a producer must hold
/// finalized PoM standing ≥ the threshold. This is the fast tier's Sybil resistance.
#[test]
fn contribution_weight_gate() {
    let ledger = ledger_with(5, &[(ALICE, 100), (BOB, 5)], vec![]);
    assert!(
        validate_sub_block(&ledger, &[], &sub(5, 0, ALICE, vec![]), 50).is_ok(),
        "a producer with standing ≥ threshold may propose"
    );
    assert_eq!(
        validate_sub_block(&ledger, &[], &sub(5, 0, BOB, vec![]), 50),
        Err(SubBlockViolation::InsufficientStanding { have: 5, need: 50 }),
        "a producer below the standing threshold is rejected"
    );
    assert_eq!(
        validate_sub_block(&ledger, &[], &sub(5, 0, b"ghost", vec![]), 1),
        Err(SubBlockViolation::InsufficientStanding { have: 0, need: 1 }),
        "an unknown producer has zero standing"
    );
    // ANTI-THEATER: drop the standing gate ⇒ BOB and the ghost pass ⇒ RED (a no-contribution Sybil could
    // flood the fast tier).
}

/// Value txs conserve + single-use against the PROVISIONAL overlay of prior sub-blocks in the interval:
/// a coin retired by an earlier sub-block cannot be re-spent (soft-chain double-spend), but a prior
/// sub-block's OUTPUT can be spent (a valid multi-hop A→B→C soft-chain).
#[test]
fn conserves_across_provisional_overlay() {
    let coin = tok(1, 100, ALICE, ISS);
    let ledger = ledger_with(5, &[(ALICE, 100)], vec![coin.clone()]);

    // sub0: ALICE → BOB (output id 2), valid against the base ledger.
    let sub0 = sub(5, 0, ALICE, vec![transfer(coin.clone(), BOB, 2)]);
    assert!(validate_sub_block(&ledger, &[], &sub0, 0).is_ok(), "first spend of the live coin is valid");

    // DOUBLE-SPEND: re-spend the SAME original coin in sub1 — sub0 retired it in the overlay.
    let ds = sub(5, 1, ALICE, vec![transfer(coin.clone(), CAROL, 3)]);
    assert_eq!(
        validate_sub_block(&ledger, std::slice::from_ref(&sub0), &ds, 0),
        Err(SubBlockViolation::TxInvalidOrDoubleSpend),
        "re-spending a coin retired by a prior sub-block is a soft-chain double-spend ⇒ rejected"
    );

    // MULTI-HOP: sub1 spends BOB's NEW output from sub0 (id 2) ⇒ a valid soft-chain hop.
    let bob_coin = tok(2, 100, BOB, ISS); // == sub0's output
    let hop = sub(5, 1, ALICE, vec![transfer(bob_coin, CAROL, 3)]);
    assert!(
        validate_sub_block(&ledger, &[sub0], &hop, 0).is_ok(),
        "spending a prior sub-block's output is a valid multi-hop soft-chain"
    );
    // ANTI-THEATER: validate against the base ledger only (skip the provisional overlay) ⇒ the
    // double-spend passes AND the multi-hop fails ⇒ both asserts RED.
}

/// The honest UX contract read side: a finalized output is `Final`; a sub-block output is `SoftConfirmed`
/// (revertible); an unknown id is neither. Final outranks Soft (the transition is monotone).
#[test]
fn confirmation_tier_soft_vs_final() {
    let ledger = ledger_with(5, &[(ALICE, 100)], vec![tok(1, 100, ALICE, ISS)]);
    let soft = sub(5, 0, ALICE, vec![transfer(tok(9, 50, ALICE, ISS), BOB, 2)]);

    assert_eq!(
        tier_of_output(&ledger, std::slice::from_ref(&soft), 1),
        Some(ConfirmationTier::Final),
        "an output live in the finalized token set is Final"
    );
    assert_eq!(
        tier_of_output(&ledger, &[soft], 2),
        Some(ConfirmationTier::SoftConfirmed),
        "an output seen only in an accepted sub-block is SoftConfirmed (revertible)"
    );
    assert_eq!(tier_of_output(&ledger, &[], 99), None, "an id unknown to both tiers is neither");
    // ANTI-THEATER: return SoftConfirmed unconditionally ⇒ the Final assert RED; check sub-blocks before
    // the finalized set ⇒ id 1 mis-reports Soft ⇒ RED.
}
