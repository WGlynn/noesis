//! Reconciliation, first slice: two (and three) independently run devnet nodes, each with its own
//! genesis and ledger, export their contribution facts by ContentId and merge them. The design-note
//! hypothesis (*The node and the network*, 2026-10-22, §9) is that non-rival contribution facts
//! reconcile by plain union in any order, with rival claims surfaced separately. Each test pins one
//! property and is written to fail if merge dropped, duplicated, or reordered facts.

use noesis::chainspec::ChainSpec;
use noesis::commit_order::Committed;
use noesis::reconcile::{content_id, data_hash, merge, ContentId, FactRecord, FactSet};
use noesis::runtime::Node;
use noesis::{Cell, Script};

fn cell(id: u64, owner: &[u8], author: &[u8], parent: Option<u64>, ts: u64, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: author.to_vec() },
        parent,
        timestamp: ts,
        data: data.to_vec(),
    }
}

/// Boot a fresh devnet genesis and finalize one block per entry through the full pipeline
/// (submit, mine, validate, finality gate, apply). Each call is an independent chain.
fn run_chain(blocks: Vec<Vec<Cell>>) -> Node {
    let spec = ChainSpec::dev();
    let (mut node, _keys) = spec.genesis_node();
    for (i, cells) in blocks.into_iter().enumerate() {
        let h = node.ledger.height + 1;
        for (j, c) in cells.into_iter().enumerate() {
            assert!(node.submit(c, Committed { height: h, secret: [(i * 16 + j + 1) as u8; 32] }));
        }
        assert!(spec.produce_block(&mut node).is_some(), "block {h} did not finalize");
    }
    node
}

fn facts(node: &Node) -> FactSet {
    FactSet::from_cells(&node.ledger.cells).expect("every local parent was committed earlier")
}

fn root(author: &[u8], data: &[u8]) -> ContentId {
    content_id(author, &data_hash(data), None)
}

const A1: &[u8] = b"alice: the quick brown fox";
const A2: &[u8] = b"alice: the fox jumps over the lazy dog";
const B1: &[u8] = b"bob: notes on winter light";
const OV: &[u8] = b"bob: a proof that extends alice's fox";
const C1: &[u8] = b"carol: tide tables for the north shore";

/// Chain A: a1, then a2 built on a1 through the LOCAL parent id.
fn chain_a() -> FactSet {
    let a = run_chain(vec![
        vec![cell(1, b"al", b"alice", None, 1, A1)],
        vec![cell(2, b"al", b"alice", Some(1), 2, A2)],
    ]);
    facts(&a)
}

/// Chain B: b1 (same local id 1 as a1 on chain A), plus an overlap record naming a1 by ContentId,
/// which `Cell.parent` cannot express. Returns the set and the overlap's ContentId.
fn chain_b() -> (FactSet, ContentId) {
    let b = run_chain(vec![vec![cell(1, b"bo", b"bob", None, 1, B1)]]);
    let mut fb = facts(&b);
    let ov = FactRecord::new(b"bob", OV, Some(root(b"alice", A1)));
    assert!(fb.insert(ov.clone()));
    (fb, ov.cid)
}

/// Chain C: carol's own fact, then alice's a1 re-recorded at a different id, height and timestamp
/// (the same non-rival fact a second chain also holds).
fn chain_c() -> FactSet {
    let c = run_chain(vec![
        vec![cell(10, b"ca", b"carol", None, 5, C1)],
        vec![cell(11, b"zz", b"alice", None, 9, A1)],
    ]);
    facts(&c)
}

#[test]
fn two_chains_merge_in_either_order_and_the_overlap_resolves() {
    let fa = chain_a();
    let (mut fb, ov) = chain_b();
    let (a1, b1) = (root(b"alice", A1), root(b"bob", B1));
    let a2 = content_id(b"alice", &data_hash(A2), Some(&a1));

    // The local parent id resolved to a1's ContentId, and chain A is self-contained.
    assert_eq!(fa.len(), 2);
    assert_eq!(fa.get(&a2).expect("a2 present").parent, Some(a1));
    assert!(fa.dangling().is_empty());

    // In B alone the overlap is a claim awaiting its witness.
    assert_eq!(fb.dangling(), vec![ov]);

    // A record whose cid does not re-derive is refused (this is what keeps a key to one record).
    let mut forged = fb.get(&ov).unwrap().clone();
    forged.author = b"mallory".to_vec();
    assert!(!fb.insert(forged));
    assert_eq!(fb.len(), 2);

    let ab = merge(&fa, &fb);
    let ba = merge(&fb, &fa);
    assert_eq!(ab.digest(), ba.digest());
    assert_eq!(ab, ba);

    // A real union: every fact from both sides, nothing else, and the dangling reference resolved.
    assert_eq!(ab.len(), 4);
    for cid in [a1, a2, b1, ov] {
        assert!(ab.get(&cid).is_some(), "merged set lost a fact");
    }
    assert!(ab.dangling().is_empty(), "a1 is present after the merge, so the overlap must resolve");
    assert_ne!(ab.digest(), fa.digest());
    assert_ne!(ab.digest(), fb.digest());
}

#[test]
fn merge_is_associative_commutative_and_idempotent() {
    let fa = chain_a();
    let (fb, _) = chain_b();
    let fc = chain_c();

    let left = merge(&merge(&fa, &fb), &fc);
    let right = merge(&fa, &merge(&fb, &fc));
    assert_eq!(left, right);
    assert_eq!(left.digest(), right.digest());

    // Every order of the three sets gives the same result.
    let orders = [[&fa, &fb, &fc], [&fa, &fc, &fb], [&fb, &fa, &fc], [&fb, &fc, &fa], [&fc, &fa, &fb], [&fc, &fb, &fa]];
    for [x, y, z] in orders {
        assert_eq!(merge(&merge(x, y), z).digest(), left.digest());
    }

    // Union, not concatenation: a1 is held by A and C and appears once.
    assert_eq!(left.len(), fa.len() + fb.len() + fc.len() - 1);
    assert!(left.rival_claims().is_empty(), "a1 re-recorded by its own author is not a rival");

    // Idempotence and absorption, on non-empty sets.
    assert_eq!(fa.len(), 2);
    assert_eq!(merge(&fa, &fa), fa);
    assert_eq!(merge(&fa, &fa).digest(), fa.digest());
    assert_eq!(merge(&left, &fb), left);
}

#[test]
fn content_id_ignores_local_id_timestamp_and_lock() {
    const ROOT: &[u8] = b"alice: a shared root";
    const X: &[u8] = b"alice: the fact under test";
    // P records root then X early; Q records them later, at other ids, timestamps and locks.
    let p = run_chain(vec![
        vec![cell(1, b"al", b"alice", None, 1, ROOT)],
        vec![cell(2, b"al", b"alice", Some(1), 2, X)],
    ]);
    let q = run_chain(vec![
        vec![cell(7, b"bo", b"bob", None, 50, b"bob: something unrelated first")],
        vec![cell(99, b"zz", b"alice", None, 77, ROOT)],
        vec![cell(100, b"yy", b"alice", Some(99), 78, X)],
    ]);
    let pos = |n: &Node| n.ledger.cells.iter().position(|c| c.data == X).expect("X finalized");
    let (cp, cq) = (&p.ledger.cells[pos(&p)], &q.ledger.cells[pos(&q)]);
    assert_ne!(pos(&p), pos(&q));
    assert!(cp.id != cq.id && cp.timestamp != cq.timestamp && cp.lock != cq.lock);

    let expected = content_id(b"alice", &data_hash(X), Some(&root(b"alice", ROOT)));
    let (fp, fq) = (facts(&p), facts(&q));
    assert!(fp.get(&expected).is_some() && fq.get(&expected).is_some());
    assert_eq!(merge(&fp, &fq).len(), fp.len() + fq.len() - 2, "root and X dedup across chains");

    // And the id does commit to what it should: author, data, and parent each change it.
    assert_ne!(content_id(b"bob", &data_hash(X), Some(&root(b"alice", ROOT))), expected);
    assert_ne!(content_id(b"alice", &data_hash(b"other"), Some(&root(b"alice", ROOT))), expected);
    assert_ne!(content_id(b"alice", &data_hash(X), None), expected);
}

#[test]
fn rival_claim_is_surfaced_once_and_merge_stays_order_free() {
    const D: &[u8] = b"the same essay, word for word";
    let a = run_chain(vec![vec![cell(1, b"al", b"alice", None, 1, D)]]);
    let b = run_chain(vec![vec![
        cell(1, b"bo", b"bob", None, 1, D),
        cell(2, b"bo", b"bob", None, 1, b"bob: his own notes"),
    ]]);
    let (fa, fb) = (facts(&a), facts(&b));
    assert!(fa.rival_claims().is_empty() && fb.rival_claims().is_empty());

    let ab = merge(&fa, &fb);
    let ba = merge(&fb, &fa);
    assert_eq!(ab.digest(), ba.digest());
    assert_eq!(ab.rival_claims(), ba.rival_claims());

    // Exactly one group, with both authors; bob's unrelated notes are not in it.
    assert_eq!(ab.rival_claims(), vec![(data_hash(D), vec![b"alice".to_vec(), b"bob".to_vec()])]);
    // Surfaced, not resolved: both claims are kept.
    assert_eq!(ab.len(), 3);
    assert!(ab.get(&root(b"alice", D)).is_some() && ab.get(&root(b"bob", D)).is_some());
}
