//! Two-node convergence — the first END-TO-END run of the Noesis state machine across
//! more than one replica. The unit suite proves each rule in isolation; this proves the
//! pieces COMPOSE into a node two participants can run and agree on.
//!
//! The property asserted is deterministic state-machine replication: two nodes that
//! finalize the same blocks hold byte-identical ledgers (cell sequence, novelty-index
//! root, and PoM attribution map) after EVERY block. This is the in-process milestone
//! beneath any real transport — peer discovery / gossip swaps in above the `Node` API
//! without touching the convergence guarantee proven here.

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::runtime::{finalizes, Block, Constitution, Node};
use noesis::{Cell, Script};

fn validator(id: u64, pom: f64) -> Validator {
    Validator {
        id,
        pow: 0.0,
        pos: 0.0,
        pom,
        last_heartbeat: 0,
        staked_balance: 0.0,
    }
}

fn cell(id: u64, owner: &[u8], contributor: &[u8], parent: Option<u64>, ts: u64, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent,
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

/// A deterministic three-round proposal stream. Distinct content per cell so each earns
/// real temporal-novelty; some cells build on earlier ones (provenance edges) so the
/// attribution graph is non-trivial.
fn rounds() -> Vec<Vec<(Cell, Committed)>> {
    vec![
        vec![
            (
                cell(1, b"al", b"alice", None, 1, b"the quick brown fox jumps high"),
                Committed { height: 1, secret: secret(11) },
            ),
            (
                cell(2, b"bo", b"bob", None, 1, b"lorem ipsum dolor sit amet now"),
                Committed { height: 1, secret: secret(22) },
            ),
        ],
        vec![(
            cell(3, b"al", b"alice", Some(1), 2, b"fox jumps over the lazy sleeping dog"),
            Committed { height: 2, secret: secret(33) },
        )],
        vec![
            (
                cell(4, b"ca", b"carol", None, 3, b"entirely separate subject matter here today"),
                Committed { height: 3, secret: secret(44) },
            ),
            (
                cell(5, b"bo", b"bob", Some(2), 3, b"ipsum dolor greatly expanded with extra material"),
                Committed { height: 3, secret: secret(55) },
            ),
        ],
    ]
}

#[test]
fn two_nodes_converge_over_rounds() {
    let validators = vec![validator(0, 100.0), validator(1, 100.0)];
    let mut a = new_node(0, &validators);
    let mut b = new_node(1, &validators);

    for (i, proposals) in rounds().iter().enumerate() {
        let height = i as u64 + 1;

        // gossip: both nodes receive the round's proposals into their mempools.
        for (c, co) in proposals {
            a.submit(c.clone(), co.clone());
            b.submit(c.clone(), co.clone());
        }

        // leader (a) proposes; the block is broadcast to both.
        let block = a.propose();

        // each node INDEPENDENTLY validates against its own replica and votes.
        assert!(a.validate(&block), "node a rejected a valid block at height {height}");
        assert!(b.validate(&block), "node b rejected a valid block at height {height}");
        let voters_for = vec![validators[0].clone(), validators[1].clone()];

        // PoM-weighted finalization (2/3 of effective weight; both honest validators vote).
        assert!(
            finalizes(&a.constitution, &voters_for, &validators, height),
            "block at height {height} failed to finalize"
        );

        // both apply the finalized block, then clear the consumed mempool.
        a.apply(&block);
        b.apply(&block);
        a.clear_mempool();
        b.clear_mempool();

        // CONVERGENCE: identical state after every block.
        assert_eq!(
            a.ledger.state_digest(),
            b.ledger.state_digest(),
            "replicas diverged at height {height}"
        );
    }

    // the run was non-trivial: the chain grew and attributed PoM to contributors.
    assert_eq!(a.ledger.cells.len(), 5, "not all cells were finalized");
    assert_eq!(a.ledger.height, 3);
    assert!(a.ledger.pom.values().sum::<u64>() > 0, "no PoM was attributed");
    // alice and bob both contributed across rounds; both should hold standing.
    assert!(a.ledger.pom.contains_key(&b"alice".to_vec()));
    assert!(a.ledger.pom.contains_key(&b"bob".to_vec()));
    // final cross-check on the consensus-critical root.
    assert_eq!(a.ledger.index.root(), b.ledger.index.root());
}

#[test]
fn block_assembly_is_presentation_independent() {
    // The consensus shuffle decides order, not the producer. The same proposals presented
    // in any order must assemble to the SAME canonical block — otherwise a producer could
    // bias which of two contending cells banks shared novelty.
    let p1 = vec![
        (cell(1, b"al", b"alice", None, 1, b"aaaa bbbb cccc dddd"), Committed { height: 1, secret: secret(7) }),
        (cell(2, b"bo", b"bob", None, 1, b"eeee ffff gggg hhhh"), Committed { height: 1, secret: secret(2) }),
        (cell(3, b"ca", b"carol", None, 1, b"iiii jjjj kkkk llll"), Committed { height: 1, secret: secret(5) }),
    ];
    let mut p2 = p1.clone();
    p2.reverse();

    let b1 = Block::assemble(1, &p1);
    let b2 = Block::assemble(1, &p2);

    let ids1: Vec<u64> = b1.cells.iter().map(|c| c.id).collect();
    let ids2: Vec<u64> = b2.cells.iter().map(|c| c.id).collect();
    assert_eq!(ids1, ids2, "block assembly depended on producer presentation order");
}

#[test]
fn non_canonical_block_is_rejected() {
    // A producer-favorable reorder must be refused at the order gate, before any state math.
    let validators = vec![validator(0, 100.0), validator(1, 100.0)];
    let node = new_node(0, &validators);

    let proposals = vec![
        (cell(1, b"al", b"alice", None, 1, b"first content block here"), Committed { height: 1, secret: secret(9) }),
        (cell(2, b"bo", b"bob", None, 1, b"second content block there"), Committed { height: 1, secret: secret(3) }),
    ];
    let mut block = Block::assemble(1, &proposals);
    // deliberately break canonical order (a 2-permutation has only the canonical one and its swap).
    block.cells.swap(0, 1);
    block.coords.swap(0, 1);

    assert!(!node.validate(&block), "node accepted a non-canonical (producer-reordered) block");
}
