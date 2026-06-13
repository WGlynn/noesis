//! Drift guard — T7 #4 first half. noesis-core (the no_std crate the type-scripts link)
//! must agree with the node lib's own implementations on every canonical fixture, until
//! the lib re-exports from the core (single-source TODO). If either side drifts, this
//! fails before the divergence can reach a VM boundary.

use noesis::{proven, semantic, smt, value_fixed};

const ENT: u64 = 62259;
const SIM: u64 = 52429;

fn fixtures() -> Vec<Vec<u8>> {
    let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
    let keyish: Vec<u8> = (0u8..32).map(|i| i.wrapping_mul(67).wrapping_add(29)).collect();
    let hexed: Vec<u8> = noise.iter().flat_map(|b| format!("{b:02x}").into_bytes()).collect();
    vec![
        b"alpha-bravo-charlie-delta".to_vec(),
        b"echo-foxtrot-golf-hotel".to_vec(),
        b"the-quick-brown-fox-says-nothing-of-value-today".to_vec(),
        noise,
        keyish,
        hexed,
        b"xy".to_vec(),
        b"".to_vec(),
    ]
}

#[test]
fn entropy_floor_agrees() {
    for f in fixtures() {
        assert_eq!(
            noesis_core::is_incompressible_q16(&f, ENT),
            value_fixed::is_incompressible_q16(&f, ENT),
            "fixed-point entropy drift"
        );
        assert_eq!(
            noesis_core::is_incompressible_q16(&f, ENT),
            semantic::is_incompressible(&f, 0.95),
            "core vs f64 prototype (outside quantization band fixtures)"
        );
    }
}

#[test]
fn shingles_and_smt_agree() {
    for f in fixtures() {
        assert_eq!(
            noesis_core::unique_shingles(&f),
            proven::unique_shingles(&f),
            "coverage/shingle drift"
        );
    }
    // SMT: same keys -> same root and interchangeable proofs.
    let mut idx = smt::NoveltyIndex::new();
    for k in [1u64, 99, 0xDEAD_BEEF, u64::MAX] {
        idx.insert(k);
    }
    let root = idx.root();
    assert_eq!(noesis_core::root_from(99, noesis_core::leaf(99), &idx.proof(99)), root,
        "core fold reproduces the node root (hash + layout identical)");
    assert!(noesis_core::verify_member(root, 99, &idx.proof(99)));
    assert!(noesis_core::verify_non_member(root, 100, &idx.proof(100)));
}

#[test]
fn proven_verifier_agrees() {
    let mut idx = smt::NoveltyIndex::new();
    for (k, _) in proven::unique_shingles(b"prior-committed-content-zulu") {
        idx.insert(k);
    }
    let root = idx.root();
    for f in fixtures() {
        let proofs: Vec<_> = proven::unique_shingles(&f).iter().map(|(k, _)| idx.proof(*k)).collect();
        assert_eq!(
            noesis_core::proven_floored_novelty_q16(&f, root, &proofs, SIM, ENT),
            proven::proven_floored_novelty_q16(&f, root, &proofs, SIM, ENT),
            "proven-path drift"
        );
    }
}

#[test]
fn commit_order_agrees() {
    // The on-VM ordering port must compute the IDENTICAL consensus permutation the node lib does,
    // or the type-script would reject batches the node accepts (or vice versa). Drift-guard the
    // canonical commit order across a mixed multi-height set with same-height ties (XOR shuffle).
    use noesis::commit_order as n;
    use noesis_core::commit_order as c;
    let sec = |b: u8| {
        let mut s = [0u8; 32];
        s[0] = b;
        s[31] = b.wrapping_mul(7).wrapping_add(1);
        s
    };
    let coords: [(u64, u8); 6] = [(1, 7), (1, 2), (2, 5), (1, 9), (3, 1), (2, 6)];
    let nv: Vec<n::Committed> =
        coords.iter().map(|&(h, b)| n::Committed { height: h, secret: sec(b) }).collect();
    let cv: Vec<c::Committed> =
        coords.iter().map(|&(h, b)| c::Committed { height: h, secret: sec(b) }).collect();
    assert_eq!(
        n::canonical_order(&nv),
        c::canonical_order(&cv),
        "node and on-VM core compute the identical canonical commit order"
    );
    // is_canonical_order agrees on the canonical sequence AND on a producer reordering of it.
    let order = c::canonical_order(&cv);
    let canon_n: Vec<n::Committed> = order.iter().map(|&i| nv[i].clone()).collect();
    let canon_c: Vec<c::Committed> = order.iter().map(|&i| cv[i].clone()).collect();
    assert!(n::is_canonical_order(&canon_n) && c::is_canonical_order(&canon_c), "canonical accepted by both");
    let mut rev_n = canon_n.clone();
    rev_n.reverse();
    let mut rev_c = canon_c.clone();
    rev_c.reverse();
    assert_eq!(
        n::is_canonical_order(&rev_n),
        c::is_canonical_order(&rev_c),
        "node and core agree on the reordered presentation verdict"
    );
}
