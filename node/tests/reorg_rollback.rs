//! inc-1 — reorgeable ledger: the §3.4 invariant, proven end to end.
//!
//! The load-bearing correctness property of multi-producer Nakamoto (DESIGN-multi-producer-nakamoto.md
//! §3.4): when the chain reorgs to a heavier fork, PoM standing AND the novelty index roll back with
//! it. An orphaned contribution loses its standing and frees its novelty slot. If they did NOT, the
//! same content re-submitted on the winning fork would be wrongly rejected as a duplicate and standing
//! would double-count across forks — a consensus split in value-space.
//!
//! The strongest possible assertion: after the reorg the tip is byte-identical to a FRESH replay of
//! the winning fork, so EVERY finalized-state field (cells, novelty root, PoM map, token set, work
//! clock) rolled back — not just token UTXOs.

use noesis::chainspec::ChainSpec;
use noesis::commit_order::Committed;
use noesis::reorg::ReorgTip;
use noesis::runtime::{Block, Node};
use noesis::{Cell, Script};

fn cell(id: u64, contributor: &[u8], data: &str) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent: None,
        timestamp: id,
        data: data.as_bytes().to_vec(),
    }
}

/// Produce one finalized block containing a single contribution, from `node`, via the real per-block
/// engine. Returns the block object to replay onto competing tips.
fn mint(spec: &ChainSpec, node: &mut Node, id: u64, contributor: &[u8], data: &str, height: u64, secret: u8) -> Block {
    node.submit(cell(id, contributor, data), Committed { height, secret: [secret; 32] });
    spec.produce_block(node).expect("block finalizes")
}

#[test]
fn reorg_to_heavier_fork_rolls_back_standing_and_novelty() {
    let spec = ChainSpec::dev();

    // Immutable finalized base = genesis (height 0).
    let (base_node, _keys) = spec.genesis_node();
    let base = base_node.ledger.clone();

    // Fork A (lighter): ONE block, alice contributes X.
    let (mut a_node, _) = spec.genesis_node();
    let a1 = mint(&spec, &mut a_node, 1, b"alice", "alice writes about winter light on the river", 1, 1);

    // Fork B (heavier): TWO blocks, bob then carol — different contributors, different content.
    let (mut b_node, _) = spec.genesis_node();
    let b1 = mint(&spec, &mut b_node, 1, b"bob", "bob writes an entirely separate account of cold mornings", 1, 2);
    let b2 = mint(&spec, &mut b_node, 2, b"carol", "carol adds a third distinct subject about tides and moons", 2, 3);

    // Build the tip on fork A.
    let mut chain = ReorgTip::from_finalized(base, spec.constitution.clone());
    assert!(chain.extend(a1), "fork A block is valid on the base");
    assert!(chain.tip().pom.get(b"alice".as_slice()).copied().unwrap_or(0) > 0, "alice earned standing on fork A");
    let root_a = chain.tip().index.root();

    // Reorg to the heavier fork B.
    assert!(chain.try_reorg(vec![b1.clone(), b2.clone()]), "heavier two-block fork wins fork choice");

    // §3.4: alice's orphaned contribution lost its standing; the novelty index rolled back.
    assert!(!chain.tip().pom.contains_key(b"alice".as_slice()), "orphaned alice standing is gone after reorg");
    assert!(chain.tip().pom.get(b"bob".as_slice()).copied().unwrap_or(0) > 0, "bob (winning fork) has standing");
    assert_ne!(chain.tip().index.root(), root_a, "novelty index root reflects fork B, not A");

    // Strongest statement: the reorged tip is byte-identical to a FRESH replay of the winning fork.
    let (mut oracle, _) = spec.genesis_node();
    oracle.apply(&b1);
    oracle.apply(&b2);
    assert_eq!(chain.tip().state_digest(), oracle.ledger.state_digest(), "reorg == fresh replay of the winner");
    assert_eq!(chain.height(), 2);
}

#[test]
fn a_lighter_fork_does_not_win() {
    let spec = ChainSpec::dev();
    let (base_node, _) = spec.genesis_node();
    let base = base_node.ledger.clone();

    // Incumbent tip: two blocks (heavier).
    let (mut b_node, _) = spec.genesis_node();
    let b1 = mint(&spec, &mut b_node, 1, b"bob", "bob writes an entirely separate account of cold mornings", 1, 2);
    let b2 = mint(&spec, &mut b_node, 2, b"carol", "carol adds a third distinct subject about tides and moons", 2, 3);

    // Competing: one block (lighter).
    let (mut a_node, _) = spec.genesis_node();
    let a1 = mint(&spec, &mut a_node, 1, b"alice", "alice writes about winter light on the river", 1, 1);

    let mut chain = ReorgTip::from_finalized(base, spec.constitution.clone());
    assert!(chain.extend(b1));
    assert!(chain.extend(b2));
    let digest_before = chain.tip().state_digest();

    assert!(!chain.try_reorg(vec![a1]), "a strictly-lighter fork loses fork choice");
    assert_eq!(chain.tip().state_digest(), digest_before, "tip unchanged after a rejected reorg");
}

#[test]
fn finalize_to_pins_the_prefix_against_reorg() {
    let spec = ChainSpec::dev();
    let (base_node, _) = spec.genesis_node();
    let base = base_node.ledger.clone();

    // Tip on fork A: two blocks.
    let (mut a_node, _) = spec.genesis_node();
    let a1 = mint(&spec, &mut a_node, 1, b"alice", "alice writes about winter light on the river", 1, 1);
    let a2 = mint(&spec, &mut a_node, 2, b"alice", "alice continues with a distinct second passage on frost", 2, 5);

    let mut chain = ReorgTip::from_finalized(base, spec.constitution.clone());
    assert!(chain.extend(a1));
    assert!(chain.extend(a2));

    // Finalize height 1: block a1 is now immutable, the floor advances.
    chain.finalize_to(1);
    assert_eq!(chain.finalized_height(), 1);

    // Any fork that would rewrite the FINALIZED block (height 1) can't even replay: the floor is now 1,
    // so a candidate's first block must extend height 2. A height-1 candidate is rejected outright.
    let (mut b_node, _) = spec.genesis_node();
    let b1 = mint(&spec, &mut b_node, 9, b"bob", "bob tries to rewrite the finalized first block", 1, 2);
    let b2 = mint(&spec, &mut b_node, 10, b"bob", "bob rewrite block two entirely distinct subject alpha", 2, 6);
    assert!(!chain.try_reorg(vec![b1, b2]), "a fork rewriting a FINALIZED block is rejected by the floor");
    assert_eq!(chain.height(), 2, "tip intact");
}
