//! inc-2a — the parent-block commitment (`Block.parent_hash`), the fork-tree link topology B needs.
//!
//! Three properties: (1) the parent is BOUND into the PoW header, so a solved seal cannot be
//! re-parented onto a different predecessor (a block's work counts on exactly the fork it was mined
//! for); (2) it round-trips on the wire; (3) it is INERT-ADDITIVE — a pre-inc-2a block log with no
//! `parent_hash` field still decodes (as `None`), preserving byte-identical restart/replay.

use noesis::commit_order::Committed;
use noesis::runtime::{header_digest, Block};
use noesis::wire::{decode_block, encode_block};
use noesis::{Cell, Script};

fn sample_block() -> Block {
    let cell = Cell {
        id: 1,
        lock: Script { code_hash: [0u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
        parent: None,
        timestamp: 1,
        data: b"a distinct novel contribution about winter light".to_vec(),
    };
    Block::assemble(1, &[(cell, Committed { height: 1, secret: [9u8; 32] })])
}

#[test]
fn parent_hash_binds_the_header() {
    let b = sample_block();
    let d_none = header_digest(&b);
    let d_p1 = header_digest(&b.clone().with_parent_hash([7u8; 32]));
    let d_p2 = header_digest(&b.clone().with_parent_hash([8u8; 32]));
    assert_ne!(d_none, d_p1, "naming a parent changes the header — the seal commits it");
    assert_ne!(d_p1, d_p2, "a DIFFERENT parent yields a different header — no re-parenting under a fixed seal");
}

#[test]
fn parent_hash_round_trips_on_the_wire() {
    let b = sample_block().with_parent_hash([42u8; 32]);
    let back = decode_block(&encode_block(&b)).expect("re-inclusion decodes");
    assert_eq!(back.parent_hash, Some([42u8; 32]), "parent commitment survives encode→decode");
}

#[test]
fn a_pre_inc2a_log_without_the_field_decodes_as_none() {
    // A block log written before inc-2a has no `parent_hash` key. `#[serde(default)]` must decode it
    // as `None` (byte-identical restart/replay — the additive-field guarantee shared with
    // subblock_root/timestamp/pow).
    let b = sample_block();
    let json = String::from_utf8(encode_block(&b)).unwrap();
    let stripped = json.replace(",\"parent_hash\":null", "");
    assert!(!stripped.contains("parent_hash"), "simulated pre-inc-2a log carries no parent_hash field");
    let back = decode_block(stripped.as_bytes()).expect("a pre-inc-2a log still decodes");
    assert_eq!(back.parent_hash, None);
}
