//! L5 — Bound-B commit-deposit (`docs/RESOURCE-DOS-BOUNDING.md` §Bound B). Consensus-adjacent
//! (forfeiture is value movement), so the load-bearing property is VALUE CONSERVATION: a bond is
//! either left live (refund) or retired (burn), never created or moved. Proves a K-junk flood costs
//! `K·d`, an honest contribution is made whole, the refund/burn is PER-CELL (not per-contributor), and
//! a fake/underfunded/non-JUL/duplicate/double-spent bond is rejected. `submission_deposit` is ⚑
//! Will-gated; every test supplies its own. Each test names the anti-theater break that turns it RED.

use noesis::commit_order::Committed;
use noesis::jul::{is_jul, JUL_BASE_UNITS, JUL_CODE_HASH, JUL_ISSUER};
use noesis::runtime::{apply_block, validate_block, Block, Bond, Constitution, Ledger, TokenStandard, TokenTx, Violation};
use noesis::tokens::fungible;
use noesis::{Cell, Script};

const D: u128 = 100 * JUL_BASE_UNITS; // an arbitrary active deposit

fn con() -> Constitution {
    Constitution { submission_deposit: D, ..Constitution::default() }
}
fn cell(id: u64, contributor: &[u8], data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: b"own".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent: None,
        timestamp: 0,
        data: data.to_vec(),
    }
}
fn jul_cell(id: u64, owner: &[u8], amount: u128) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: JUL_CODE_HASH, args: JUL_ISSUER.to_vec() },
        parent: None,
        timestamp: 0,
        data: fungible::encode(amount),
    }
}
fn bond(for_cell: u64, deposit: Cell) -> Bond {
    Bond { for_cell, deposit, auth: vec![] } // empty auth = pre-deploy inert (CONTROL_BINDING_ACTIVE off)
}
fn com(height: u64, s: u8) -> Committed {
    Committed { height, secret: [s; 32] }
}
fn live_jul(l: &Ledger) -> u128 {
    fungible::total(&l.token_cells, &JUL_CODE_HASH, JUL_ISSUER)
}
// A ledger seeded with `n` live JUL cells (ids 9000.., amount D, owner "sub") to bond against.
fn seeded(n: u64) -> (Ledger, Vec<u64>) {
    let mut l = Ledger::new();
    let ids: Vec<u64> = (0..n).map(|i| 9000 + i).collect();
    for id in &ids {
        l.token_cells.push(jul_cell(*id, b"sub", D));
    }
    (l, ids)
}
const NOVEL: &[u8] = b"a genuinely novel first contribution with plenty of distinct coverage material";

// ---- inert default ----

/// submission_deposit == 0 ⇒ no bonds allowed (fail-closed), and a normal unbonded block applies
/// exactly as pre-L5. ANTI-THEATER: make the bond gate unconditional (require bonds at 0) ⇒ the whole
/// legacy suite + this go RED; allow bonds at 0 ⇒ the reject half goes RED.
#[test]
fn inert_at_zero_deposit() {
    let c0 = Constitution::default(); // submission_deposit == 0
    let n = cell(1, b"alice", NOVEL);
    let plain = Block::assemble(1, &[(n.clone(), com(1, 1))]);
    assert!(validate_block(&Ledger::new(), &plain, &c0).is_ok(), "unbonded block valid when inert");
    let bonded = plain.clone().with_bonds(vec![bond(1, jul_cell(9000, b"sub", D))]);
    assert!(
        matches!(validate_block(&Ledger::new(), &bonded, &c0), Err(Violation::BondCoverage)),
        "a bond present under an inert deposit is fail-closed"
    );
}

// ---- flagship: the flood costs K·d ----

/// A K-junk flood forfeits exactly K·d, while an honest bonded contribution in the same run is
/// refunded. ANTI-THEATER (the doc's named break): stub the refund predicate to always-refund ⇒ the
/// K·d drop and the burned-cell assertions all go RED (in this escrow-by-validation design the forfeit
/// branch IS the only action, so "floods are free" is exactly its absence).
#[test]
fn junk_flood_forfeits_k_times_d() {
    const K: u64 = 5;
    let (mut l, ids) = seeded(K + 1);
    let before = live_jul(&l);

    // block 1: one genuinely novel cell, bonded ⇒ refunded (left live).
    let novel = cell(1, b"alice", NOVEL);
    let b1 = Block::assemble(1, &[(novel.clone(), com(1, 1))])
        .with_bonds(vec![bond(1, jul_cell(ids[0], b"sub", D))]);
    l = apply_block(l, &b1, &con()).expect("novel bonded block applies");

    // block 2: K junk cells, each a byte-identical duplicate of NOVEL ⇒ zero novelty ⇒ all forfeited.
    let mut props = vec![];
    let mut bonds = vec![];
    for k in 0..K {
        let j = cell(100 + k, b"mallory", NOVEL); // identical coverage ⇒ similarity floor ⇒ value 0
        props.push((j.clone(), com(2, 100 + k as u8)));
        bonds.push(bond(j.id, jul_cell(ids[1 + k as usize], b"sub", D)));
    }
    let b2 = Block::assemble(2, &props).with_bonds(bonds);
    l = apply_block(l, &b2, &con()).expect("junk-flood block applies (rejection is per-bond, not the block)");

    let after = live_jul(&l);
    assert_eq!(before - after, (K as u128) * D, "the flood forfeited exactly K·d");
    assert!(l.token_cells.iter().any(|c| c.id == ids[0]), "the honest (novel) bond was refunded — still live");
    for k in 0..K {
        assert!(!l.token_cells.iter().any(|c| c.id == ids[1 + k as usize]), "each junk bond was burned");
    }
    assert_eq!(l.jul_supply.issued(), 0, "forfeiture never touches issued supply (no coinbase here)");
}

// ---- honest contribution made whole ----

/// A single genuinely novel bonded cell keeps its full bond. ANTI-THEATER: invert the predicate
/// (forfeit unconditionally) ⇒ the honest bond is destroyed ⇒ RED.
#[test]
fn honest_contribution_is_made_whole() {
    let (l, ids) = seeded(1);
    let before = live_jul(&l);
    let novel = cell(1, b"alice", NOVEL);
    let b1 = Block::assemble(1, &[(novel, com(1, 1))]).with_bonds(vec![bond(1, jul_cell(ids[0], b"sub", D))]);
    let l = apply_block(l, &b1, &con()).expect("applies");
    assert_eq!(live_jul(&l), before, "a novel contribution forfeits nothing");
    assert!(l.token_cells.iter().any(|c| c.id == ids[0]), "the exact bond cell is still live");
}

// ---- the per-cell (not per-contributor) trap ----

/// One block, one contributor, 1 novel + 2 junk cells ⇒ exactly the 2 junk bonds burn. ANTI-THEATER:
/// key the refund on the per-CONTRIBUTOR pom map instead of the per-CELL value ⇒ that contributor's
/// map-pom is > 0, so a map-keyed impl refunds the junk ⇒ RED (the single most likely wrong impl).
#[test]
fn mixed_block_refunds_per_cell_not_per_contributor() {
    let (mut l, ids) = seeded(4);
    // establish coverage so the "junk" duplicates score zero.
    let seed = cell(1, b"alice", NOVEL);
    let b1 = Block::assemble(1, &[(seed, com(1, 1))]).with_bonds(vec![bond(1, jul_cell(ids[0], b"sub", D))]);
    l = apply_block(l, &b1, &con()).expect("seed applies");
    let before = live_jul(&l);

    // block 2, ALL by "alice": one novel + two byte-identical duplicates of NOVEL.
    let novel = cell(10, b"alice", b"a second wholly distinct contribution, different words entirely here");
    let junk1 = cell(11, b"alice", NOVEL);
    let junk2 = cell(12, b"alice", NOVEL);
    let props = vec![(novel, com(2, 10)), (junk1, com(2, 11)), (junk2, com(2, 12))];
    let bonds = vec![
        bond(10, jul_cell(ids[1], b"sub", D)),
        bond(11, jul_cell(ids[2], b"sub", D)),
        bond(12, jul_cell(ids[3], b"sub", D)),
    ];
    let b2 = Block::assemble(2, &props).with_bonds(bonds);
    l = apply_block(l, &b2, &con()).expect("mixed block applies");

    assert_eq!(before - live_jul(&l), 2 * D, "exactly the two junk bonds burned, the novel one refunded");
    assert!(l.token_cells.iter().any(|c| c.id == ids[1]), "the novel cell's bond is refunded");
}

// ---- fake / underfunded / non-JUL / duplicate / double-spent bonds rejected ----

/// The bond must be a REAL live JUL cell of the required amount, authorized, single-use. Each rejection
/// has a passing control. ANTI-THEATER: trust the carried cell / a declared amount without the ledger
/// existence+identity match ⇒ the fabricated/underfunded/non-JUL cases validate ⇒ RED.
#[test]
fn fake_underfunded_or_nonjul_bond_rejected() {
    let (mut l, ids) = seeded(1);
    let n = cell(1, b"alice", NOVEL);
    let mk = |dep: Cell| Block::assemble(1, &[(n.clone(), com(1, 1))]).with_bonds(vec![bond(1, dep)]);

    // control: the real seeded bond validates.
    assert!(validate_block(&l, &mk(jul_cell(ids[0], b"sub", D)), &con()).is_ok(), "the real bond validates");

    // (i) fabricated: a JUL cell never finalized into token_cells (do NOT seed it).
    assert!(matches!(validate_block(&l, &mk(jul_cell(7777, b"sub", D)), &con()), Err(Violation::BondInvalid)), "fabricated bond rejected");
    // (ii) underfunded: a live cell carrying less than D. validate_block is read-only ⇒ seed into `l`.
    let under = jul_cell(8888, b"sub", D - 1);
    l.token_cells.push(under.clone());
    assert!(matches!(validate_block(&l, &mk(under), &con()), Err(Violation::BondInvalid)), "underfunded bond rejected");
    // (iii) non-JUL: a self-issued token (live) used as a bond.
    let mut fake = jul_cell(9500, b"sub", D);
    fake.type_script.args = b"not-jul".to_vec();
    assert!(!is_jul(&fake));
    l.token_cells.push(fake.clone());
    assert!(matches!(validate_block(&l, &mk(fake), &con()), Err(Violation::BondInvalid)), "non-JUL bond rejected");
}

/// Coverage failures: a missing bond, a duplicate bond, and a bond for an absent cell. ANTI-THEATER:
/// drop the 1:1 coverage requirement ⇒ an unbonded cell slips through free ⇒ RED.
#[test]
fn bond_coverage_is_enforced() {
    let (l, ids) = seeded(2);
    let a = cell(1, b"alice", NOVEL);
    let b = cell(2, b"bob", b"another distinct contribution with its own separate coverage material here");
    let props = vec![(a.clone(), com(1, 1)), (b.clone(), com(1, 2))];

    // missing: 2 cells, 1 bond.
    let miss = Block::assemble(1, &props).with_bonds(vec![bond(1, jul_cell(ids[0], b"sub", D))]);
    assert!(matches!(validate_block(&l, &miss, &con()), Err(Violation::BondCoverage)), "a cell without a bond is rejected");

    // duplicate: 2 bonds both for cell 1.
    let dup = Block::assemble(1, &props)
        .with_bonds(vec![bond(1, jul_cell(ids[0], b"sub", D)), bond(1, jul_cell(ids[1], b"sub", D))]);
    assert!(matches!(validate_block(&l, &dup, &con()), Err(Violation::BondCoverage)), "a duplicate bond is rejected");

    // absent cell: a bond for a cell id not in the block.
    let ghost = Block::assemble(1, &props)
        .with_bonds(vec![bond(1, jul_cell(ids[0], b"sub", D)), bond(99, jul_cell(ids[1], b"sub", D))]);
    assert!(matches!(validate_block(&l, &ghost, &con()), Err(Violation::BondCoverage)), "a bond for an absent cell is rejected");
}

/// THE escape exploit (Council flagship): a bond whose deposit is ALSO spent by a token_tx in the same
/// block — the token phase would pay it out while the burn silently no-ops, so the deposit escapes
/// forfeiture. `check_bonds` seeds the single-use set with token-tx inputs, so the collision is
/// rejected. ANTI-THEATER: drop the token-input seeding (or narrow the bond identity key) ⇒ the block
/// validates and the escape opens ⇒ RED.
#[test]
fn bond_cell_also_spent_by_a_token_tx_is_rejected() {
    let (l, ids) = seeded(1);
    let x = jul_cell(ids[0], b"sub", D); // the cell we try to bond AND spend
    let c = cell(1, b"alice", NOVEL);
    // a conserving JUL transfer that spends x (out == in == D, so it is ledger-valid).
    let tx = TokenTx {
        standard: TokenStandard::Fungible,
        auths: vec![],
        code_hash: JUL_CODE_HASH,
        args: JUL_ISSUER.to_vec(),
        inputs: vec![x.clone()],
        outputs: vec![jul_cell(5555, b"sub", D)],
    };
    let blk = Block::assemble(1, &[(c, com(1, 1))]).with_token_txs(vec![tx]).with_bonds(vec![bond(1, x)]);
    assert!(
        matches!(validate_block(&l, &blk, &con()), Err(Violation::BondInvalid)),
        "a bond whose deposit is also a token-tx input in the same block must be rejected"
    );
}

/// A forfeit burns EXACTLY ONE instance of the bonded cell, never a byte-identical clone too (Council
/// MAJOR: `retain` remove-all would over-burn). ANTI-THEATER: revert the burn to `retain` ⇒ both
/// duplicates retire ⇒ `before − after == 2D` and zero remain ⇒ RED.
#[test]
fn forfeit_burns_exactly_one_instance_not_a_clone() {
    let mut l = Ledger::new();
    l.token_cells.push(jul_cell(100, b"sub", D)); // bond cell for the coverage-establishing block
    l.token_cells.push(jul_cell(7000, b"sub", D)); // duplicate #1 (the bonded cell)
    l.token_cells.push(jul_cell(7000, b"sub", D)); // duplicate #2 — byte-identical
    let before = live_jul(&l); // 3·D

    // block 1: a novel cell (refunded) establishes the coverage that makes block 2's cell "junk".
    l = apply_block(
        l,
        &Block::assemble(1, &[(cell(1, b"alice", NOVEL), com(1, 1))]).with_bonds(vec![bond(1, jul_cell(100, b"sub", D))]),
        &con(),
    )
    .expect("coverage block applies");

    // block 2: a junk duplicate of NOVEL, bonded to the (duplicated) 7000 cell ⇒ forfeit.
    l = apply_block(
        l,
        &Block::assemble(2, &[(cell(2, b"mallory", NOVEL), com(2, 2))]).with_bonds(vec![bond(2, jul_cell(7000, b"sub", D))]),
        &con(),
    )
    .expect("forfeit block applies");

    assert_eq!(before - live_jul(&l), D, "a forfeit burns exactly one instance (D), never the clone too (2D)");
    assert_eq!(l.token_cells.iter().filter(|c| c.id == 7000).count(), 1, "exactly one duplicate remains");
}
