//! T1 slice-1 — persistence + wire codec round-trip.
//!
//! The property that makes "a network you can join" possible, proven locally: a node that persists
//! its finalized blocks to disk and is then **restarted from that log** reconstructs a byte-identical
//! `state_digest`. State is derived by replaying the canonical blocks (reusing `Node::apply`), not by
//! trusting a snapshot blob. This is also the exact substrate sync will use: send the block log, replay.

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::runtime::{Constitution, Node};
use noesis::wire::{decode_block, encode_block, BlockLog};
use noesis::{Cell, Script};

/// Honest genesis: one bonded PoS validator, empty ledger, default Constitution (same on every node).
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

/// A deterministic scripted workload (distinct content = real novelty, some provenance edges),
/// mirroring the shape noesisd runs.
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

#[test]
fn restart_from_block_log_reconstructs_byte_identical_state() {
    let tmp = std::env::temp_dir().join("noesis_persist_roundtrip_replay.jsonl");
    let _ = std::fs::remove_file(&tmp); // clean slate even if a prior run left one
    let log = BlockLog::new(&tmp);

    // Node A: build the chain, persisting each applied block to disk.
    let mut a = genesis();
    for proposals in workload() {
        for (c, co) in &proposals {
            a.submit(c.clone(), co.clone());
        }
        let block = a.propose();
        a.apply(&block);
        log.append(&block).expect("append to block log");
        a.clear_mempool();
    }

    // Node B: a FRESH genesis, reconstructed purely by replaying the persisted log from disk.
    let mut b = genesis();
    for block in log.load().expect("load block log") {
        b.apply(&block);
    }

    assert_eq!(
        a.ledger.state_digest(),
        b.ledger.state_digest(),
        "a node restarted from its block log must reconstruct byte-identical state"
    );
    assert_eq!(a.ledger.height, b.ledger.height, "height must match after replay");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn codec_round_trips_a_block_byte_stably() {
    let mut a = genesis();
    a.submit(cell(1, b"al", b"alice", None, 1, b"a novel first contribution"), committed(1, 7));
    let block = a.propose();

    let bytes = encode_block(&block);
    let decoded = decode_block(&bytes).expect("decode a well-formed block");
    let bytes2 = encode_block(&decoded);

    assert_eq!(
        bytes, bytes2,
        "decode(encode(block)) must re-encode identically — a faithful, lossless round trip"
    );
}

#[test]
fn a_corrupt_log_line_fails_loudly() {
    let tmp = std::env::temp_dir().join("noesis_persist_roundtrip_corrupt.jsonl");
    let _ = std::fs::remove_file(&tmp);
    std::fs::write(&tmp, b"{not valid json for a block}\n").expect("write corrupt line");

    let err = BlockLog::new(&tmp).load();
    assert!(err.is_err(), "a corrupt log must fail loudly, never silently drop a block and diverge");

    let _ = std::fs::remove_file(&tmp);
}
