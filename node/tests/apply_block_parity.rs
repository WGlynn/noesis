//! Phase-1 replay parity — the extraction gate.
//!
//! Phase 1 of the stateless-verification engagement extracts the state transition into a single
//! pure rulebook `runtime::apply_block(state, block) -> Result<Ledger, Violation>`, and makes the
//! existing node a thin caller of it. The engagement's hard rule: *consensus behaviour must be
//! bit-identical before and after — prove it by replaying vectors through the old and new paths and
//! diffing state roots.* That is exactly this file.
//!
//! - **Old path:** `Node::apply` (now delegates to the extracted `apply_transition`).
//! - **New path:** fold the value-returning `apply_block` from an empty `Ledger`.
//!
//! The property: after EVERY block, `node.ledger.state_digest() == folded_state.state_digest()`
//! (the finalized cell sequence, novelty-index root, sorted PoM map, token-cell sequence, and the
//! cumulative-work clock — the full consensus-comparison tuple). The token-movement path is
//! additionally covered transitively: `Node::apply` and `apply_block` share `apply_transition`, and
//! the wider suite (`two_node`, `gaming`, token tests) drives token apply through `Node::apply`.

use noesis::commit_order::Committed;
use noesis::runtime::{apply_block, validate_block, Block, Constitution, Ledger, Node, Violation};
use noesis::{Cell, Script};

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

/// The same deterministic three-round proposal stream `two_node.rs` proves convergence over:
/// distinct content per cell (real temporal-novelty), with provenance edges so the attribution
/// graph is non-trivial. Reusing it means this parity test rides the exact vectors already trusted
/// for the replication guarantee.
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
fn apply_block_matches_node_apply_over_vectors() {
    let con = Constitution::default();

    // OLD path: the in-place node applier (validators empty — we exercise apply/validate, not finality).
    let mut node = Node::new(0, Vec::new(), con);
    // NEW path: fold the pure rulebook from an empty ledger.
    let mut state = Ledger::new();

    for (i, proposals) in rounds().iter().enumerate() {
        let height = i as u64 + 1;
        let block = Block::assemble(height, proposals);

        // both acceptance gates agree the block is valid.
        assert!(node.validate(&block), "Node::validate rejected a valid block at height {height}");
        assert!(
            validate_block(&state, &block).is_ok(),
            "validate_block rejected a valid block at height {height}"
        );

        // apply through both paths.
        node.apply(&block);
        state = apply_block(state, &block, &con)
            .unwrap_or_else(|v| panic!("apply_block rejected a valid block at height {height}: {v:?}"));

        // REPLAY PARITY: byte-identical state after every block.
        assert_eq!(
            node.ledger.state_digest(),
            state.state_digest(),
            "apply_block diverged from Node::apply at height {height}"
        );
    }

    // anti-theater: the run was non-trivial (an empty run would trivially "match"). The chain grew
    // to all five cells and attributed real PoM to the contributors.
    assert_eq!(state.cells.len(), 5, "not all cells were applied");
    assert_eq!(state.height, 3);
    assert!(state.pom.values().sum::<u64>() > 0, "no PoM was attributed");
    assert_eq!(node.ledger.index.root(), state.index.root(), "novelty roots diverged");
}

#[test]
fn apply_block_surfaces_typed_violations() {
    let con = Constitution::default();

    // (1) height mismatch — a height-2 block on a fresh (height-0) ledger.
    let p2 = vec![(
        cell(1, b"al", b"alice", None, 1, b"some genuinely novel content here"),
        Committed { height: 2, secret: secret(1) },
    )];
    let bad_height = Block::assemble(2, &p2);
    match validate_block(&Ledger::new(), &bad_height) {
        Err(Violation::HeightMismatch { expected, got }) => {
            assert_eq!(expected, 1);
            assert_eq!(got, 2);
        }
        other => panic!("expected HeightMismatch, got {other:?}"),
    }
    assert!(matches!(
        apply_block(Ledger::new(), &bad_height, &con),
        Err(Violation::HeightMismatch { expected: 1, got: 2 })
    ));

    // (3) empty block.
    let empty = Block::assemble(1, &[]);
    assert!(matches!(validate_block(&Ledger::new(), &empty), Err(Violation::EmptyBlock)));

    // (4) non-canonical order — a producer reorder of a valid block.
    let p1 = vec![
        (
            cell(1, b"al", b"alice", None, 1, b"first content block right here now"),
            Committed { height: 1, secret: secret(9) },
        ),
        (
            cell(2, b"bo", b"bob", None, 1, b"second content block over there today"),
            Committed { height: 1, secret: secret(3) },
        ),
    ];
    let mut nc = Block::assemble(1, &p1);
    nc.cells.swap(0, 1);
    nc.coords.swap(0, 1);
    assert!(matches!(validate_block(&Ledger::new(), &nc), Err(Violation::NonCanonicalOrder)));
    assert!(matches!(apply_block(Ledger::new(), &nc, &con), Err(Violation::NonCanonicalOrder)));
}
