//! Adversarial-gaming loop (ROADMAP loop 3, the highest-leverage one) exercised at the RUNTIME
//! level: the un-gameable-v(S) property — sybil / padding resistance — must hold when the value/
//! attribution path is composed into the live node, not just in the lib's unit tests. The library
//! proves `temporal_novelty` zeroes redundant coverage in isolation; this proves the node inherits
//! it: a sybil ring spamming identical content through the real propose→validate→finalize→apply
//! path cannot multiply standing, because redundant copies earn ZERO novel coverage.

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::runtime::{finalizes, Constitution, Node};
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

#[test]
fn sybil_ring_cannot_multiply_standing_through_runtime() {
    let vs = vec![validator(0, 100.0), validator(1, 100.0)];
    let mut a = Node::new(0, vs.clone(), Constitution::default());
    let mut b = Node::new(1, vs.clone(), Constitution::default());

    // one honest contributor with novel content, then a sybil RING of 5 distinct identities all
    // submitting IDENTICAL padding content (distinct from the honest cell).
    let mut proposals = vec![(
        cell(1, b"honest", 1, b"honest unique contribution alpha beta gamma delta epsilon"),
        Committed { height: 1, secret: secret(1) },
    )];
    let sybil_ids: Vec<Vec<u8>> = (0..5u8).map(|i| format!("sybil{i}").into_bytes()).collect();
    for (i, sid) in sybil_ids.iter().enumerate() {
        proposals.push((
            cell(10 + i as u64, sid, 1, b"sybil padding ring identical zzz zzz zzz zzz"),
            Committed { height: 1, secret: secret(50 + i as u8) },
        ));
    }

    // run one honest round on both nodes.
    for (c, co) in &proposals {
        a.submit(c.clone(), co.clone());
        b.submit(c.clone(), co.clone());
    }
    let block = a.propose();
    assert!(a.validate(&block) && b.validate(&block));
    assert!(finalizes(&a.constitution, &vs, &vs, 1));
    a.apply(&block);
    b.apply(&block);
    assert_eq!(a.ledger.state_digest(), b.ledger.state_digest(), "replicas diverged");

    let pom = &a.ledger.pom;
    let honest = pom.get(&b"honest".to_vec()).copied().unwrap_or(0);
    assert!(honest > 0, "honest novel contribution earned no PoM");

    // among 5 identical-content sybils, AT MOST ONE can bank the coverage (whoever the consensus
    // shuffle orders first); the rest earn 0. Adding identities did not multiply standing.
    let nonzero = sybil_ids.iter().filter(|k| pom.get(*k).copied().unwrap_or(0) > 0).count();
    assert!(
        nonzero <= 1,
        "sybil ring multiplied standing across {nonzero} identities — un-gameable-v(S) broken at runtime"
    );

    // the whole ring's total standing is bounded by a single cell's worth — spamming K identities
    // yields no more than K=1 would.
    let sybil_total: u64 = sybil_ids.iter().map(|k| pom.get(k).copied().unwrap_or(0)).sum();
    let max_single = sybil_ids.iter().map(|k| pom.get(k).copied().unwrap_or(0)).max().unwrap_or(0);
    assert_eq!(sybil_total, max_single, "ring total exceeds one cell's coverage — sybil farming possible");
}

#[test]
fn cross_block_redundancy_earns_zero() {
    // content already committed in an earlier block earns ZERO novelty when re-submitted later —
    // a padding attacker cannot farm standing by re-posting yesterday's content.
    let vs = vec![validator(0, 100.0), validator(1, 100.0)];
    let mut n = Node::new(0, vs.clone(), Constitution::default());

    let data = b"a durable original contribution that is later copied verbatim";
    // block 1: original author posts it.
    let p1 = vec![(cell(1, b"author", 1, data), Committed { height: 1, secret: secret(7) })];
    n.submit(p1[0].0.clone(), p1[0].1.clone());
    let b1 = n.propose();
    n.apply(&b1);
    n.clear_mempool();
    let author_after_1 = n.ledger.pom.get(&b"author".to_vec()).copied().unwrap_or(0);
    assert!(author_after_1 > 0);

    // block 2: a copycat re-submits the SAME content.
    let p2 = vec![(cell(2, b"copycat", 2, data), Committed { height: 2, secret: secret(8) })];
    n.submit(p2[0].0.clone(), p2[0].1.clone());
    let b2 = n.propose();
    n.apply(&b2);
    let copycat = n.ledger.pom.get(&b"copycat".to_vec()).copied().unwrap_or(0);
    assert_eq!(copycat, 0, "copycat earned standing for re-posting already-committed content");
}

#[test]
fn paraphrase_padding_ring_cannot_multiply_standing_through_runtime() {
    // The harder sybil vector. The IDENTICAL-content ring above is already zeroed by plain
    // temporal-novelty (exact duplicates earn 0). A smarter attacker flips a few bytes so the
    // copies are NEAR-duplicates: plain temporal-novelty leaks the change-spanning shingles as
    // small residual novelty, so a ring of K near-copies banks ~K cells' worth of standing. The
    // runtime PoM gate must apply the similarity floor (Constitution.theta_sim_q16) so the ring is
    // bounded to a single cell's coverage regardless of how many near-copies are minted.
    let vs = vec![validator(0, 100.0), validator(1, 100.0)];
    let mut a = Node::new(0, vs.clone(), Constitution::default());
    let mut b = Node::new(1, vs.clone(), Constitution::default());

    let mut proposals = vec![(
        cell(1, b"honest", 1, b"honest unique contribution alpha beta gamma delta epsilon zeta eta"),
        Committed { height: 1, secret: secret(1) },
    )];
    // a long shared padding base; each sybil flips ONLY the final byte, leaving coverage overlap
    // far above the 0.95 floor — a near-duplicate, not an exact one.
    let base: &[u8] = b"sybil padding ring near-duplicate filler text, long enough that one flipped tail byte leaves coverage overlap above the floor xxxxxxxx";
    let sybil_ids: Vec<Vec<u8>> = (0..5u8).map(|i| format!("sybil{i}").into_bytes()).collect();
    for (i, sid) in sybil_ids.iter().enumerate() {
        let mut data = base.to_vec();
        let n = data.len();
        data[n - 1] = b'a' + i as u8; // single distinct tail byte → near-dup, not exact-dup
        proposals.push((
            cell(10 + i as u64, sid, 1, &data),
            Committed { height: 1, secret: secret(50 + i as u8) },
        ));
    }

    for (c, co) in &proposals {
        a.submit(c.clone(), co.clone());
        b.submit(c.clone(), co.clone());
    }
    let block = a.propose();
    assert!(a.validate(&block) && b.validate(&block));
    assert!(finalizes(&a.constitution, &vs, &vs, 1));
    a.apply(&block);
    b.apply(&block);
    assert_eq!(a.ledger.state_digest(), b.ledger.state_digest(), "replicas diverged");

    let pom = &a.ledger.pom;
    let honest = pom.get(&b"honest".to_vec()).copied().unwrap_or(0);
    assert!(honest > 0, "honest novel contribution earned no PoM");

    // at most one identity banks the padding's coverage; every near-duplicate after it is floored.
    let nonzero = sybil_ids.iter().filter(|k| pom.get(*k).copied().unwrap_or(0) > 0).count();
    assert!(
        nonzero <= 1,
        "paraphrase-padding ring multiplied standing across {nonzero} identities — near-dup floor not applied at runtime"
    );

    // the whole ring's standing is bounded by a single cell's coverage: K near-copies == 1.
    let sybil_total: u64 = sybil_ids.iter().map(|k| pom.get(k).copied().unwrap_or(0)).sum();
    let max_single = sybil_ids.iter().map(|k| pom.get(k).copied().unwrap_or(0)).max().unwrap_or(0);
    assert_eq!(
        sybil_total, max_single,
        "ring total exceeds one cell's coverage — paraphrase-padding farming possible"
    );
}
