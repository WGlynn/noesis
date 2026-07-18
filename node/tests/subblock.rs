//! Sub-blocks slice 1 (T9) — the data model + contribution-weight gate + provisional-overlay
//! conservation + the confirmation-tier read side. Pure SHADOW validation: no networking, no
//! finalized-state mutation (revertible by construction). Each test names its anti-theater break.

use noesis::commit_order::Committed;
use noesis::runtime::{apply_block, Block, Constitution, Ledger, TokenStandard, TokenTx};
use noesis::subblock::{
    absorb, subblock_txs_root, tier_of_output, validate_sub_block, verify_absorption_root,
    ConfirmationTier, SubBlock, SubBlockViolation,
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

/// A JUL cell (the money type) — used to construct the JUL-smuggle the settlement tier rejects.
fn jul_cell(id: u64, owner: &[u8], amount: u128) -> Cell {
    use noesis::jul::{JUL_CODE_HASH, JUL_ISSUER};
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: JUL_CODE_HASH, args: JUL_ISSUER.to_vec() },
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

/// The fast tier must apply the SAME conservation rule as settlement. A soft-confirm of a tx that
/// settlement will ALWAYS reject — JUL minted from nothing (declare a benign token, smuggle a JUL output),
/// or a token output squatting a reserved coinbase id — is a soft-confirm of a guaranteed revert, which
/// misleads a merchant. Single-sourcing sub-block conservation onto `runtime::txs_conserve_and_single_use`
/// (the settlement gate) closes the gap: the fast tier now runs the JUL-conservation + coinbase-squat checks
/// its own copy was missing, so the two tiers can never drift again.
#[test]
fn fast_tier_matches_settlement_conservation() {
    let coin = tok(1, 100, ALICE, ISS);
    let ledger = ledger_with(5, &[(ALICE, 100)], vec![coin.clone()]);

    // (a) JUL smuggle: declare benign token TC (conserves 100→100) but smuggle a 50-JUL output (jul_in=0).
    // `is_valid_in_ledger` conserves only the DECLARED token, so this passes it — the JUL check is what
    // catches the mint-from-nothing (the exact runtime::token_txs_conserve_and_single_use rationale).
    let mut smuggle = transfer(coin.clone(), BOB, 3);
    smuggle.outputs.push(jul_cell(2, BOB, 50));
    assert_eq!(
        validate_sub_block(&ledger, &[], &sub(5, 0, ALICE, vec![smuggle]), 0),
        Err(SubBlockViolation::TxInvalidOrDoubleSpend),
        "the fast tier must reject a JUL-from-nothing smuggle, exactly as settlement does"
    );

    // (b) coinbase-id squat: a token output claiming a reserved coinbase id could grief a real reward.
    let mut squat = transfer(coin, BOB, 3);
    squat.outputs[0].id = noesis::jul::COINBASE_ID_BIT | 3;
    assert_eq!(
        validate_sub_block(&ledger, &[], &sub(5, 0, ALICE, vec![squat]), 0),
        Err(SubBlockViolation::TxInvalidOrDoubleSpend),
        "the fast tier must reject a coinbase-id squat, exactly as settlement does"
    );
    // ANTI-THEATER: sub-block's old private conserve check omitted BOTH the JUL-conservation and the
    // coinbase-squat gates ⇒ both txs passed validation ⇒ both asserts RED before the single-source.
}

/// The provisional overlay must be RECEIPT-ORDER-INDEPENDENT, exactly as `absorb` is: it must fold the
/// prior sub-blocks in `seq` order, not input order. A multi-hop soft-chain retires-then-produces, so an
/// unsorted fold computes a DIFFERENT live set and `validate_sub_block` would accept/reject the SAME
/// sub-block differently depending on how the prior chain was delivered — a determinism divergence from
/// the absorption the security analysis relies on.
#[test]
fn provisional_overlay_is_seq_ordered_not_input_ordered() {
    let coin = tok(1, 100, ALICE, ISS);
    let ledger = ledger_with(5, &[(ALICE, 100)], vec![coin.clone()]);

    // A valid two-hop soft-chain: sub0 ALICE→BOB (out id 2), sub1 spends BOB's out (id 2) → CAROL (out id 3).
    let s0 = sub(5, 0, ALICE, vec![transfer(coin, BOB, 2)]);
    let bob_coin = tok(2, 100, BOB, ISS); // == s0's output
    let s1 = sub(5, 1, ALICE, vec![transfer(bob_coin, CAROL, 3)]);

    // The next sub-block (seq 2) RE-SPENDS BOB's intermediate output (id 2). In the correct seq-ordered
    // fold s1 already RETIRED id 2 (it hopped it to CAROL), so re-spending it is a soft-chain double-spend
    // ⇒ MUST be rejected. `sub.seq == prior.len()` holds regardless of prior order, so the only thing that
    // can change the verdict is the fold order of the overlay.
    let bob_coin_again = tok(2, 100, BOB, ISS); // == s0's output, already retired by s1
    let s2 = sub(5, 2, ALICE, vec![transfer(bob_coin_again, CAROL, 4)]);

    // Prior delivered IN ORDER ⇒ the double-spend of the retired intermediate is rejected.
    assert_eq!(
        validate_sub_block(&ledger, &[s0.clone(), s1.clone()], &s2, 0),
        Err(SubBlockViolation::TxInvalidOrDoubleSpend),
        "re-spending an intermediate output a later sub-block already retired is a double-spend"
    );
    // Prior delivered OUT OF ORDER (same chain, shuffled) ⇒ MUST get the SAME verdict (seq-ordered fold).
    assert_eq!(
        validate_sub_block(&ledger, &[s1, s0], &s2, 0),
        Err(SubBlockViolation::TxInvalidOrDoubleSpend),
        "receipt order of the prior soft-chain must not change validation — the overlay is seq-ordered"
    );
    // ANTI-THEATER: fold the prior in INPUT order ⇒ the shuffled case processes s1 before s0, so it retires
    // id 2 (absent, no-op) BEFORE s0 produces it ⇒ id 2 is left live ⇒ the double-spend is wrongly ACCEPTED
    // ⇒ the second assert goes RED. This is the exact absorb()/provisional_live() determinism divergence.
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

/// Absorption is DETERMINISTIC: the interval's accepted sub-blocks flatten into ordering-block txs in
/// `seq` order regardless of the order an absorber received them, so two honest absorbers agree byte-for-
/// byte (the security-relevant property).
#[test]
fn absorb_is_deterministic_regardless_of_receipt_order() {
    let s0 = sub(5, 0, ALICE, vec![transfer(tok(1, 100, ALICE, ISS), BOB, 10)]);
    let s1 = sub(5, 1, BOB, vec![transfer(tok(2, 100, BOB, ISS), CAROL, 11)]);
    let ids = |txs: &[TokenTx]| -> Vec<u64> {
        txs.iter().flat_map(|t| t.outputs.iter().map(|o| o.id)).collect()
    };
    let in_order = absorb(&[s0.clone(), s1.clone()]);
    let received_shuffled = absorb(&[s1, s0]); // same soft-chain, delivered out of order
    assert_eq!(ids(&in_order), vec![10, 11], "flattened in seq order");
    assert_eq!(
        ids(&in_order),
        ids(&received_shuffled),
        "receipt order must not affect absorption — two honest absorbers agree"
    );
    // ANTI-THEATER: drop the sort_by_key(seq) ⇒ the shuffled input flattens to [11, 10] ⇒ RED.
}

/// End-to-end soft → final: a tx soft-confirmed in a sub-block becomes `Final` once an ordering block
/// absorbs it into its `token_txs` and that block finalizes (ordinary validation + apply).
#[test]
fn absorbed_sub_block_txs_become_final() {
    let coin = tok(1, 100, ALICE, ISS);
    let ledger = ledger_with(5, &[(ALICE, 100)], vec![coin.clone()]);

    // sub0 soft-confirms ALICE → BOB (output id 2).
    let s0 = sub(5, 0, ALICE, vec![transfer(coin, BOB, 2)]);
    assert!(validate_sub_block(&ledger, &[], &s0, 0).is_ok(), "the soft-confirm is valid");
    assert_eq!(
        tier_of_output(&ledger, std::slice::from_ref(&s0), 2),
        Some(ConfirmationTier::SoftConfirmed),
        "before absorption the output is SoftConfirmed (revertible)"
    );

    // the next ORDERING block (height 6) absorbs the soft-chain into its token_txs.
    let contrib = Cell {
        id: 6,
        lock: Script { code_hash: [1u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [2u8; 32], args: b"alice".to_vec() },
        parent: None,
        timestamp: 0,
        data: b"ordering block 6 contribution, genuinely novel".to_vec(),
    };
    let ordering = Block::assemble(6, &[(contrib, Committed { height: 6, secret: [7u8; 32] })])
        .with_token_txs(absorb(std::slice::from_ref(&s0)));
    let after = apply_block(ledger, &ordering, &Constitution::default()).expect("ordering block applies");

    assert_eq!(
        tier_of_output(&after, &[], 2),
        Some(ConfirmationTier::Final),
        "once absorbed + finalized the output is Final (soft → final)"
    );
    // ANTI-THEATER: if absorb() dropped the txs, output 2 never enters token_cells ⇒ tier stays None ⇒ RED.
}

/// Commit-by-root: the absorption root is DETERMINISTIC and receipt-order-independent (seq-ordered), so two
/// honest producers commit the same 32-byte root regardless of the order sub-blocks arrived.
#[test]
fn absorption_root_is_deterministic_and_order_independent() {
    let s0 = sub(5, 0, ALICE, vec![transfer(tok(1, 100, ALICE, ISS), BOB, 10)]);
    let s1 = sub(5, 1, BOB, vec![transfer(tok(2, 100, BOB, ISS), CAROL, 11)]);
    let r1 = subblock_txs_root(&[s0.clone(), s1.clone()]);
    let r2 = subblock_txs_root(&[s1, s0]); // same soft-chain, received out of order
    assert_ne!(r1, [0u8; 32], "a non-empty interval has a non-zero root");
    assert_eq!(r1, r2, "the root is receipt-order-independent (seq-ordered) ⇒ honest producers agree");
    // ANTI-THEATER: drop the seq sort in absorb() ⇒ r1 != r2 ⇒ RED.
}

/// The root BINDS the tx content (a changed tx ⇒ a changed root) and `verify_absorption_root` accepts the
/// correct root, rejects a wrong one.
#[test]
fn absorption_root_binds_content_and_verifies() {
    let s0 = sub(5, 0, ALICE, vec![transfer(tok(1, 100, ALICE, ISS), BOB, 10)]);
    let committed = subblock_txs_root(std::slice::from_ref(&s0));
    assert!(verify_absorption_root(std::slice::from_ref(&s0), committed), "the correct root verifies");
    assert!(!verify_absorption_root(std::slice::from_ref(&s0), [9u8; 32]), "a wrong root is rejected");

    // a different tx (same input, DIFFERENT recipient) must change the root.
    let s0b = sub(5, 0, ALICE, vec![transfer(tok(1, 100, ALICE, ISS), CAROL, 10)]);
    assert_ne!(
        subblock_txs_root(std::slice::from_ref(&s0b)),
        committed,
        "changing a tx must change the committed root (content binding)"
    );
    // ANTI-THEATER: return a constant root ⇒ the wrong-root reject AND the content-binding assert go RED.
}

/// Slice-2c: the absorption root is BOUND INTO the ordering-block header (`Block.subblock_root`), so the
/// PoW commits it — a solved seal cannot be replayed onto a block whose absorbed soft-chain was swapped.
/// `None` (the default) is byte-identical to a pre-sub-block header (replay parity, the coinbase/pow/
/// timestamp precedent); `Some(root)` changes the digest; the committed root equals `subblock_txs_root`.
#[test]
fn subblock_root_binds_into_header_digest() {
    use noesis::commit_order::Committed;
    use noesis::runtime::header_digest;

    let contrib = |id: u64| Cell {
        id,
        lock: Script { code_hash: [1u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [2u8; 32], args: b"alice".to_vec() },
        parent: None,
        timestamp: 0,
        data: b"ordering block 6 contribution, genuinely novel".to_vec(),
    };
    let props = [(contrib(6), Committed { height: 6, secret: [7u8; 32] })];

    // The interval's absorbed soft-chain + its deterministic root.
    let s0 = sub(6, 0, ALICE, vec![transfer(tok(1, 100, ALICE, ISS), BOB, 10)]);
    let accepted = [s0];
    let root = subblock_txs_root(&accepted);

    // Baseline: no sub-block root ⇒ digest MUST equal the same block built without touching the field
    // (default `None`) ⇒ replay-parity with every pre-sub-block block.
    let bare = Block::assemble(6, &props);
    let none_explicit = Block::assemble(6, &props); // subblock_root defaults to None
    assert_eq!(
        header_digest(&bare),
        header_digest(&none_explicit),
        "a None sub-block root is byte-identical to a pre-sub-block header (replay parity)"
    );

    // Binding: committing the root CHANGES the header digest ⇒ the PoW now proves the absorbed set.
    let committed = Block::assemble(6, &props).with_subblock_root(root);
    assert_ne!(
        header_digest(&bare),
        header_digest(&committed),
        "committing a sub-block root must change the header digest (the seal binds the absorbed set)"
    );

    // Swap the absorbed soft-chain (different recipient ⇒ different root) ⇒ a DIFFERENT header ⇒ the seal
    // cannot be replayed onto it.
    let s0b = sub(6, 0, ALICE, vec![transfer(tok(1, 100, ALICE, ISS), CAROL, 10)]);
    let swapped = Block::assemble(6, &props).with_subblock_root(subblock_txs_root(&[s0b]));
    assert_ne!(
        header_digest(&committed),
        header_digest(&swapped),
        "swapping the absorbed soft-chain post-solve changes the header ⇒ replay is impossible"
    );

    // The committed root a verifier recomputes over the block's re-included txs matches the header field.
    assert!(
        verify_absorption_root(&accepted, committed.subblock_root.expect("root committed")),
        "the header's committed root equals the recomputed absorption root over the re-included txs"
    );
    // ANTI-THEATER: drop the subblock_root arm from header_digest ⇒ bare == committed ⇒ the binding
    // assert goes RED (a solver could swap the absorbed set after solving).
}

/// An empty interval (no sub-blocks, or sub-blocks carrying no txs) commits the all-zero root.
#[test]
fn empty_interval_has_zero_root() {
    assert_eq!(subblock_txs_root(&[]), [0u8; 32], "no sub-blocks ⇒ zero root");
    let empty = sub(5, 0, ALICE, vec![]);
    assert_eq!(
        subblock_txs_root(std::slice::from_ref(&empty)),
        [0u8; 32],
        "sub-blocks with no txs ⇒ zero root"
    );
    assert!(verify_absorption_root(&[], [0u8; 32]), "the zero root verifies for an empty interval");
}
