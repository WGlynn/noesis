//! T1 slice-4 — sync (the join), proven end-to-end over real localhost TCP.
//!
//! A fresh genesis node connects to a seed that already has a chain, pulls the seed's block log,
//! replays it, and converges to a **byte-identical `state_digest`**. This is "a network you can join"
//! demonstrated locally — the payoff the earlier slices were building toward.

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::net::{Listener, Peer};
use noesis::runtime::{Block, Constitution, Node};
use noesis::sync::{serve, sync_from};
use noesis::{Cell, Script};
use std::thread;

fn genesis() -> Node {
    let validators = vec![Validator {
        id: 0,
        pow: 0.0,
        pos: 1000.0,
        pom: 0.0,
        last_heartbeat: 0,
        staked_balance: 1000.0,
    }];
    Node::new(0, validators, Constitution::default())
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

fn committed(height: u64, s: u8) -> Committed {
    Committed { height, secret: [s; 32] }
}

fn workload() -> Vec<Vec<(Cell, Committed)>> {
    vec![
        vec![
            (cell(1, b"al", b"alice", None, 1, b"the quick brown fox jumps high"), committed(1, 11)),
            (cell(2, b"bo", b"bob", None, 1, b"lorem ipsum dolor sit amet now"), committed(1, 22)),
        ],
        vec![(cell(3, b"al", b"alice", Some(1), 2, b"fox jumps over the lazy sleeping dog"), committed(2, 33))],
        vec![(cell(4, b"ca", b"carol", None, 3, b"an entirely separate subject appears today"), committed(3, 44))],
    ]
}

/// Build a seed node with a chain, returning it and its ordered canonical block log.
fn build_seed() -> (Node, Vec<Block>) {
    let mut a = genesis();
    let mut blocks = Vec::new();
    for proposals in workload() {
        for (c, co) in &proposals {
            a.submit(c.clone(), co.clone());
        }
        let block = a.propose();
        a.apply(&block);
        blocks.push(block);
        a.clear_mempool();
    }
    (a, blocks)
}

#[test]
fn a_fresh_node_joins_a_peer_and_converges_to_identical_state() {
    let (seed, blocks) = build_seed();
    let seed_digest = seed.ledger.state_digest();
    let seed_height = seed.ledger.height;
    let n_blocks = blocks.len();

    let listener = Listener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    // seed thread: accept the joiner and serve the whole block log.
    let server = thread::spawn(move || {
        let mut peer = listener.accept().expect("accept joiner");
        serve(&mut peer, &blocks).expect("serve block log");
    });

    // joiner: a fresh genesis node pulls the log and replays it.
    let mut joiner = genesis();
    let mut peer = Peer::connect(addr).expect("connect to seed");
    let applied = sync_from(&mut peer, &mut joiner).expect("sync from seed");

    server.join().expect("seed thread");

    assert_eq!(applied, n_blocks, "joiner applied every block the seed served");
    assert_eq!(joiner.ledger.height, seed_height, "joiner reached the seed's height");
    assert_eq!(
        joiner.ledger.state_digest(),
        seed_digest,
        "a joined node must converge to the seed's byte-identical state"
    );
}

#[test]
fn joining_an_empty_seed_leaves_the_joiner_at_genesis() {
    // A seed with no blocks (genesis only) serves DONE immediately; the joiner stays at genesis and
    // still converges (both are the same empty ledger).
    let seed = genesis();
    let seed_digest = seed.ledger.state_digest();

    let listener = Listener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut peer = listener.accept().unwrap();
        serve(&mut peer, &[]).unwrap();
    });

    let mut joiner = genesis();
    let mut peer = Peer::connect(addr).unwrap();
    let applied = sync_from(&mut peer, &mut joiner).unwrap();

    server.join().unwrap();
    assert_eq!(applied, 0);
    assert_eq!(joiner.ledger.state_digest(), seed_digest, "empty-seed join still converges");
}
