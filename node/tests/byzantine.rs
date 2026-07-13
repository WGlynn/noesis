//! Adversarial 2-node tests (RSAW — every milestone meets the adversary the moment it lands).
//! The convergence test (two_node.rs) proves two HONEST nodes agree. This proves the dual: an
//! honest node REJECTS what a Byzantine peer proposes, a faulty/equivocating proposer cannot
//! corrupt the honest replica's state, and a Byzantine MINORITY cannot finalize without the
//! honest supermajority. Safety under a faulty node is the property that makes "2 nodes" mean
//! something beyond a happy path.

use noesis::commit_order::Committed;
use noesis::consensus::{is_equivocation, Validator};
use noesis::runtime::{finalizes, Block, Constitution, Node};
use noesis::{Cell, Script};

fn validator(id: u64, pom: f64) -> Validator {
    Validator { id, pow: 0.0, pos: 0.0, pom, last_heartbeat: 0, staked_balance: 0.0 }
}

fn cell(id: u64, contributor: &[u8], ts: u64, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent: None,
        timestamp: ts,
        data: data.to_vec(),
    }
}

fn secret(b: u8) -> [u8; 32] {
    [b; 32]
}

fn new_node(id: u64, validators: &[Validator]) -> Node {
    Node::new(id, validators.to_vec(), Constitution::default())
}

#[test]
fn honest_node_rejects_block_at_wrong_height() {
    let vs = vec![validator(0, 100.0), validator(1, 100.0)];
    let node = new_node(0, &vs);
    // node is at height 0; a Byzantine leader proposes a block claiming height 2 (skips 1).
    let proposals = vec![(cell(1, b"alice", 2, b"some valid content here"), Committed { height: 2, secret: secret(1) })];
    let block = Block::assemble(2, &proposals);
    assert!(!node.validate(&block), "honest node accepted a height-skipping block");
}

#[test]
fn byzantine_leader_cannot_corrupt_honest_replica() {
    let vs = vec![validator(0, 100.0), validator(1, 100.0)];
    let mut honest = new_node(1, &vs);

    // Byzantine leader sends a non-canonical (producer-reordered) block.
    let proposals = vec![
        (cell(1, b"alice", 1, b"first content block alpha"), Committed { height: 1, secret: secret(9) }),
        (cell(2, b"bob", 1, b"second content block beta"), Committed { height: 1, secret: secret(3) }),
    ];
    let mut bad = Block::assemble(1, &proposals);
    bad.cells.swap(0, 1);
    bad.coords.swap(0, 1);

    assert!(!honest.validate(&bad), "honest node accepted a Byzantine reorder");
    // the honest node does NOT apply what it rejected — its replica stays pristine.
    assert_eq!(honest.ledger.height, 0);
    assert!(honest.ledger.cells.is_empty());

    // and a subsequent HONEST block still applies cleanly (rejection didn't wedge the node).
    let good = Block::assemble(1, &proposals);
    assert!(honest.validate(&good));
    honest.apply(&good);
    assert_eq!(honest.ledger.height, 1);
    assert_eq!(honest.ledger.cells.len(), 2);
}

#[test]
fn equivocation_is_detected() {
    // a validator voting for TWO different proposals in one epoch is slashable equivocation;
    // re-voting the same proposal, or a first vote, is not.
    assert!(is_equivocation(Some(100), 200), "double-vote not flagged");
    assert!(!is_equivocation(Some(100), 100), "re-vote of same proposal wrongly flagged");
    assert!(!is_equivocation(None, 100), "first vote wrongly flagged");
}

#[test]
fn byzantine_minority_cannot_finalize() {
    // four equal validators; a lone Byzantine vote (25%) cannot reach the 2/3 bar, while the
    // honest supermajority (3 of 4 = 75%) clears it. (Note: 2-of-3 = 0.6666 sits JUST under the
    // 6667-BPS bar by construction — exact-thirds never finalize, a deliberate conservative edge.)
    let vs = vec![validator(0, 100.0), validator(1, 100.0), validator(2, 100.0), validator(3, 100.0)];
    let node = new_node(0, &vs);

    let lone = vec![vs[3].clone()];
    assert!(
        !finalizes(&node.constitution, &lone, &vs, 1),
        "a single (Byzantine) validator finalized below the 2/3 bar"
    );
    // the honest supermajority (3 of 4) does finalize.
    let honest_super = vec![vs[0].clone(), vs[1].clone(), vs[2].clone()];
    assert!(finalizes(&node.constitution, &honest_super, &vs, 1));
}

#[test]
fn empty_block_is_rejected() {
    // a proposer cannot finalize an empty block (nothing to order, no contribution).
    let vs = vec![validator(0, 100.0), validator(1, 100.0)];
    let node = new_node(0, &vs);
    let empty = Block { height: 1, cells: vec![], coords: vec![], token_txs: vec![], coinbase: None, pow: None };
    assert!(!node.validate(&empty), "honest node accepted an empty block");
}
