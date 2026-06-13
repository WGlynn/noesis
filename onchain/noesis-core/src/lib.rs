//! noesis-core — the VERIFY-side cores both worlds must agree on, no_std.
//! T7 #4 first half (`T7-CROSS-CELL-SIMILARITY.md`): one home for the arithmetic the
//! node (host, std) and the type-scripts (RISC-V, no_std + alloc) share, so the
//! cross-VM-boundary determinism that T4-T6 demonstrated by duplication is now held by
//! a single source. The node carries a drift-guard test asserting these functions agree
//! with its own lib on the canonical fixtures until its lib re-exports from here
//! (single-source TODO, pinned in CONTINUE).
//!
//! Contents are verify-side only by design: SMT fold + proofs (no alloc), coverage
//! shingles (alloc), the proven intake verifier (alloc), and the Q16.16 fixed-point
//! floors (no alloc). Maintainer-side state (NoveltyIndex, flow, settlement) stays in
//! the node — the script never needs it.

#![no_std]
extern crate alloc;

use alloc::vec::Vec;

// ============ Q16.16 fixed-point floors (mirror of node value_fixed) ============

pub const Q16: u32 = 16;

/// Deterministic Q16.16 log2 (x >= 1): shift-and-square, 16 bounded iterations.
pub fn log2_q16(x: u64) -> u64 {
    debug_assert!(x >= 1);
    let ip = 63 - u64::from(x.leading_zeros());
    let mut m: u128 = ((x as u128) << 32) >> ip;
    let mut frac: u64 = 0;
    let mut i = Q16;
    while i > 0 {
        i -= 1;
        m = (m * m) >> 32;
        if m >= (2u128 << 32) {
            m >>= 1;
            frac |= 1 << i;
        }
    }
    (ip << Q16) | frac
}

/// Integer entropy floor: H/log2(min(n,256)) >= theta, division-free.
pub fn is_incompressible_q16(data: &[u8], theta_q16: u64) -> bool {
    let n = data.len() as u64;
    if n < 2 {
        return theta_q16 == 0;
    }
    let mut counts = [0u64; 256];
    let mut i = 0;
    while i < data.len() {
        counts[data[i] as usize] += 1;
        i += 1;
    }
    let mut sum_clog: i128 = 0;
    let mut b = 0;
    while b < 256 {
        if counts[b] > 0 {
            sum_clog += (counts[b] as i128) * (log2_q16(counts[b]) as i128);
        }
        b += 1;
    }
    let lhs: i128 = (n as i128) * (log2_q16(n) as i128) - sum_clog;
    let m = n.min(256);
    let rhs: i128 = ((theta_q16 as i128) * (n as i128) * (log2_q16(m) as i128)) >> Q16;
    lhs >= rhs
}

pub fn semantic_floor_q16(novelty: u64, data: &[u8], theta_q16: u64) -> u64 {
    if is_incompressible_q16(data, theta_q16) {
        0
    } else {
        novelty
    }
}

// ============ Coverage shingles (mirror of node coverage) ============

pub type CovId = u64;

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

pub fn coverage(data: &[u8]) -> Vec<CovId> {
    let mut out = Vec::new();
    if data.len() < 4 {
        if !data.is_empty() {
            out.push(fnv(data));
        }
        return out;
    }
    for w in data.windows(4) {
        out.push(fnv(w));
    }
    out
}

/// Sorted unique shingles with per-occurrence multiplicities.
pub fn unique_shingles(data: &[u8]) -> Vec<(u64, u64)> {
    let mut cov = coverage(data);
    cov.sort_unstable();
    let mut out: Vec<(u64, u64)> = Vec::new();
    for k in cov {
        match out.last_mut() {
            Some((key, m)) if *key == k => *m += 1,
            _ => out.push((k, 1)),
        }
    }
    out
}

// ============ SMT verify fold (mirror of node smt; no alloc) ============

pub const DEPTH: usize = 64;
pub type Hash = [u8; 32];

fn blake2b(parts: &[&[u8]]) -> Hash {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-smt-v1\0\0\0").build();
    for p in parts {
        h.update(p);
    }
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

pub fn leaf(key: u64) -> Hash {
    blake2b(&[b"leaf", &key.to_le_bytes()])
}

pub fn root_from(key: u64, leaf_hash: Hash, siblings: &[Hash; DEPTH]) -> Hash {
    let mut acc = leaf_hash;
    for (i, sib) in siblings.iter().enumerate() {
        acc = if (key >> i) & 1 == 0 {
            blake2b(&[&acc, sib])
        } else {
            blake2b(&[sib, &acc])
        };
    }
    acc
}

pub fn verify_member(root: Hash, key: u64, siblings: &[Hash; DEPTH]) -> bool {
    root_from(key, leaf(key), siblings) == root
}

pub fn verify_non_member(root: Hash, key: u64, siblings: &[Hash; DEPTH]) -> bool {
    root_from(key, [0u8; 32], siblings) == root
}

pub fn verify_insert(old_root: Hash, new_root: Hash, key: u64, siblings: &[Hash; DEPTH]) -> bool {
    root_from(key, [0u8; 32], siblings) == old_root
        && root_from(key, leaf(key), siblings) == new_root
}

// ============ Proven intake verifier (mirror of node proven) ============

/// One shingle's classification under `root` — the STREAMING primitive (the on-VM
/// program verifies proof-by-proof from the witness without materializing them all).
pub enum Class {
    Member,
    Absent,
}

pub fn classify(root: Hash, key: u64, path: &[Hash; DEPTH]) -> Option<Class> {
    match (verify_member(root, key, path), verify_non_member(root, key, path)) {
        (true, false) => Some(Class::Member),
        (false, true) => Some(Class::Absent),
        _ => None,
    }
}

/// The floor arithmetic on proven counts — kept here so script and node share ONE copy.
pub fn floored_from_counts(
    novelty_occ: u64,
    overlap_uniq: u64,
    unique_total: u64,
    data: &[u8],
    theta_sim_q16: u64,
    theta_ent_q16: u64,
) -> u64 {
    let floored = if unique_total > 0
        && ((overlap_uniq as u128) << 16) > (theta_sim_q16 as u128) * unique_total as u128
    {
        0
    } else {
        novelty_occ
    };
    semantic_floor_q16(floored, data, theta_ent_q16)
}

/// Classify every unique shingle against `root`; counts or whole-cell rejection.
/// Implemented ON TOP of `classify` so batch and streaming are one source.
pub fn novelty_with_proofs(data: &[u8], root: Hash, proofs: &[[Hash; DEPTH]]) -> Option<(u64, u64, u64)> {
    let uniq = unique_shingles(data);
    if proofs.len() != uniq.len() {
        return None;
    }
    let mut novelty_occ = 0u64;
    let mut overlap_uniq = 0u64;
    for ((key, mult), path) in uniq.iter().zip(proofs) {
        match classify(root, *key, path)? {
            Class::Member => overlap_uniq += 1,
            Class::Absent => novelty_occ += mult,
        }
    }
    Some((novelty_occ, overlap_uniq, uniq.len() as u64))
}

/// The full proven intake floor — what the type-script runs (T7 #4 second half).
pub fn proven_floored_novelty_q16(
    data: &[u8],
    root: Hash,
    proofs: &[[Hash; DEPTH]],
    theta_sim_q16: u64,
    theta_ent_q16: u64,
) -> Option<u64> {
    let (novelty, overlap, total) = novelty_with_proofs(data, root, proofs)?;
    Some(floored_from_counts(novelty, overlap, total, data, theta_sim_q16, theta_ent_q16))
}

// ============ Consensus-sourced commit ordering (on-VM port of node::commit_order) ============
// The no_std half of the temporal-order fix, so the index-cell type-script can verify ordering
// ON-VM. Logic is bit-identical to node::commit_order (drift-guarded in node/tests). On-VM the
// CALLER must source `height` from the commitment's block header and `secret` from the block's
// reveals (never the producer's claim) — that sourcing is the deploy-coupled, sentinel-gated part;
// these functions are the consensus-replayable permutation the sourcing feeds.
pub mod commit_order {
    use super::Hash;
    use alloc::vec::Vec;

    /// A committed cell with its CONSENSUS-SOURCED ordering coordinates: `height` = commit-reveal
    /// block height (header on-chain), `secret` = the participant's revealed commit-reveal secret.
    #[derive(Clone, Debug)]
    pub struct Committed {
        pub height: u64,
        pub secret: Hash,
    }

    fn seed_from_xor(xor: &Hash) -> u64 {
        xor.chunks_exact(8)
            .fold(0u64, |acc, c| acc ^ u64::from_be_bytes(c.try_into().unwrap()))
    }

    /// splitmix64 — deterministic, consensus-replayable 64-bit PRNG.
    fn splitmix64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Canonical intra-block permutation: Fisher-Yates over participants, seeded by the XOR of all
    /// `secrets`. Returns original indices in winning-first slot order. Presentation-independent.
    pub fn block_shuffle(secrets: &[Hash]) -> Vec<usize> {
        let mut base: Vec<usize> = (0..secrets.len()).collect();
        base.sort_by(|&a, &b| secrets[a].cmp(&secrets[b]));
        let mut xor = [0u8; 32];
        for s in secrets {
            for (x, b) in xor.iter_mut().zip(s.iter()) {
                *x ^= b;
            }
        }
        let mut state = seed_from_xor(&xor);
        for i in (1..base.len()).rev() {
            let j = (splitmix64(&mut state) % (i as u64 + 1)) as usize;
            base.swap(i, j);
        }
        base
    }

    /// Canonical commit order over cells that may span blocks: (height ascending, then in-block
    /// shuffle slot). Returns indices into `items` in canonical order.
    pub fn canonical_order(items: &[Committed]) -> Vec<usize> {
        if items.is_empty() {
            return Vec::new();
        }
        let mut heights: Vec<u64> = items.iter().map(|c| c.height).collect();
        heights.sort_unstable();
        heights.dedup();
        let mut order: Vec<usize> = Vec::with_capacity(items.len());
        for h in heights {
            let idxs: Vec<usize> = items
                .iter()
                .enumerate()
                .filter(|(_, c)| c.height == h)
                .map(|(i, _)| i)
                .collect();
            let secrets: Vec<Hash> = idxs.iter().map(|&i| items[i].secret).collect();
            for &slot in block_shuffle(&secrets).iter() {
                order.push(idxs[slot]);
            }
        }
        order
    }

    /// Does `presented` already equal the canonical commit order? The on-VM type-script ASSERTS
    /// this and REJECTS a non-canonical batch (denying any probe signal).
    pub fn is_canonical_order(presented: &[Committed]) -> bool {
        canonical_order(presented)
            .iter()
            .enumerate()
            .all(|(pos, &idx)| pos == idx)
    }
}
