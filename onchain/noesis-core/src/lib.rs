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

    // ---- Wire format (single home for the committed-batch layout) ----
    // The on-VM ordering type-script DECODES; the node/producer ENCODES. Each record is 40 bytes:
    //   height (u64 LE, 8) + secret (Hash, 32). N records, no header.
    pub const CREC_LEN: usize = 40;

    /// Decode a presented committed-batch. `None` if empty or not a whole number of records (an
    /// empty batch orders nothing ⇒ malformed, like an empty finalization group).
    pub fn parse_batch(data: &[u8]) -> Option<Vec<Committed>> {
        if data.is_empty() || data.len() % CREC_LEN != 0 {
            return None;
        }
        let mut out = Vec::with_capacity(data.len() / CREC_LEN);
        for r in data.chunks_exact(CREC_LEN) {
            let height = u64::from_le_bytes(r[0..8].try_into().unwrap());
            let mut secret = [0u8; 32];
            secret.copy_from_slice(&r[8..40]);
            out.push(Committed { height, secret });
        }
        Some(out)
    }

    /// Producer/test mirror of `parse_batch` — the other half of the single source.
    pub fn encode_batch(items: &[Committed]) -> Vec<u8> {
        let mut out = Vec::with_capacity(items.len() * CREC_LEN);
        for c in items {
            out.extend_from_slice(&c.height.to_le_bytes());
            out.extend_from_slice(&c.secret);
        }
        out
    }
}

// ============ PoM-weighted finalization in Q32.32 (ON-VM-FINALIZATION.md) ============
//
// The consensus finalize rule (`node::consensus::finalizes_hybrid`) ported to exact fixed
// point so the on-VM finalization type-script and the node reference compute bit-identically.
// SINGLE SOURCE: the node lib re-exports these (the `finalization-typescript` ELF links them),
// and the node carries the drift-guard `finalizes_fixed ≡ finalizes_hybrid` over an f64 sweep.
// The arithmetic mirrors `value_fixed`/`settlement_fixed`: saturating, divu-deterministic, and
// the threshold/quorum-floor are evaluated with ONE ceil (`bps_of_ceil`) so a boundary tie
// rounds AGAINST finalization — the fixed rule never finalizes a case the real rule rejects.
pub mod finalization {
    use alloc::vec::Vec;
    use core::convert::TryInto;

    pub const Q: u32 = 32;
    pub const ONE: u128 = 1 << Q;
    /// Basis points denominator (mirrors `consensus::BPS`). Local so the no_std core is self-contained.
    pub const BPS: u64 = 10_000;

    /// Per-dimension mix fractions in Q32.32 (sum ≈ ONE).
    #[derive(Clone, Copy, Debug)]
    pub struct MixQ {
        pub pow: u128,
        pub pos: u128,
        pub pom: u128,
    }

    /// A validator's three proof inputs in Q32.32 + the integer liveness clock.
    #[derive(Clone, Debug)]
    pub struct ValidatorQ {
        pub id: u64,
        pub pow: u128,
        pub pos: u128,
        pub pom: u128,
        pub last_heartbeat: u64,
    }

    /// Finalize parameters carried alongside the validator set in the finalization cell.
    #[derive(Clone, Copy, Debug)]
    pub struct FinalParams {
        pub horizon: u64,
        pub threshold_bps: u64,
        pub quorum_floor_bps: u64,
        pub decay_pos: bool,
    }

    /// Q32.32 product: `(a·b) >> Q`, saturating (a floored result — never wraps).
    fn mul(a: u128, b: u128) -> u128 {
        a.saturating_mul(b) >> Q
    }

    /// Linear retention in Q32.32: ONE fresh, → 0 as `elapsed → horizon`, clamped. Mirrors
    /// `consensus::retention`. `(elapsed << Q)` fits u128 for any u64 elapsed ⇒ exact integer
    /// division (deterministic on RISC-V `divu`).
    pub fn retention_q(elapsed: u64, horizon: u64) -> u128 {
        if horizon == 0 {
            return ONE;
        }
        if elapsed >= horizon {
            return 0;
        }
        ONE - (((elapsed as u128) << Q) / horizon as u128)
    }

    /// `W = pow·m.pow + pos·m.pos + pom·m.pom` in Q32.32 (mirrors `consensus::base_weight`).
    pub fn base_weight_q(v: &ValidatorQ, m: MixQ) -> u128 {
        mul(v.pow, m.pow)
            .saturating_add(mul(v.pos, m.pos))
            .saturating_add(mul(v.pom, m.pom))
    }

    /// Retention-adjusted vote weight in Q32.32 (mirrors `consensus::effective_weight`).
    /// `decay_pos=false` = capital does not decay; `true` = symmetric franchise decay.
    pub fn effective_weight_q(v: &ValidatorQ, m: MixQ, now: u64, horizon: u64, decay_pos: bool) -> u128 {
        let ret = retention_q(now.saturating_sub(v.last_heartbeat), horizon);
        let pos_portion = mul(v.pos, m.pos);
        let decayable = mul(v.pow, m.pow).saturating_add(mul(v.pom, m.pom));
        if decay_pos {
            mul(decayable.saturating_add(pos_portion), ret)
        } else {
            pos_portion.saturating_add(mul(decayable, ret))
        }
    }

    /// `bps` fraction of a Q32.32 quantity, rounded UP. Used for BOTH the finalize threshold
    /// (a boundary tie rounds AGAINST finalization) AND the quorum floor (the basis is never
    /// lowered by truncation). One direction, one function.
    fn bps_of_ceil(x: u128, bps: u64) -> u128 {
        let num = x.saturating_mul(bps as u128);
        (num + (BPS as u128 - 1)) / BPS as u128
    }

    /// Does a proposal finalize, computed entirely in Q32.32? Bit-for-bit deterministic across
    /// platforms; the on-VM finalization type-script calls this exact arithmetic.
    #[allow(clippy::too_many_arguments)]
    pub fn finalizes_fixed(
        voters_for: &[ValidatorQ],
        all: &[ValidatorQ],
        m: MixQ,
        now: u64,
        horizon: u64,
        decay_pos: bool,
        threshold_bps: u64,
        quorum_floor_bps: u64,
    ) -> bool {
        let weight_for: u128 = voters_for
            .iter()
            .fold(0u128, |a, v| a.saturating_add(effective_weight_q(v, m, now, horizon, decay_pos)));
        let eff_total: u128 = all
            .iter()
            .fold(0u128, |a, v| a.saturating_add(effective_weight_q(v, m, now, horizon, decay_pos)));
        let base_total: u128 = all.iter().fold(0u128, |a, v| a.saturating_add(base_weight_q(v, m)));
        let floor = bps_of_ceil(base_total, quorum_floor_bps);
        let basis = eff_total.max(floor);
        basis > 0 && weight_for >= bps_of_ceil(basis, threshold_bps)
    }

    // ---- Wire format (single home for the finalization-cell + vote layout) ----
    // The ELF DECODES; the node/producer ENCODES; both call the same functions so the format
    // has exactly one definition. All little-endian. Q values are u128 (16B) to match the core
    // domain with no narrowing; ids/clocks are u64 (8B).
    //
    // Finalization cell data:
    //   header  (PARAMS_LEN=64): mix_pow[0..16] mix_pos[16..32] mix_pom[32..48]
    //                            horizon[48..56] threshold_bps[56..58] quorum_floor_bps[58..60]
    //                            decay_pos[60] pad[61..64]
    //   then N validator records (VREC_LEN=64 each):
    //                            id[0..8] pow[8..24] pos[24..40] pom[40..56] last_heartbeat[56..64]
    // Votes witness: u16 LE index list — the validators that voted FOR (each < N).
    pub const PARAMS_LEN: usize = 64;
    pub const VREC_LEN: usize = 64;

    fn rd_u64(b: &[u8], o: usize) -> u64 {
        u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
    }
    fn rd_u128(b: &[u8], o: usize) -> u128 {
        u128::from_le_bytes(b[o..o + 16].try_into().unwrap())
    }

    /// Decode the finalization cell: (mix, params, validators). `None` on any malformed length
    /// (short header, or a validator section that is not an exact multiple of VREC_LEN).
    pub fn parse_finalization_cell(data: &[u8]) -> Option<(MixQ, FinalParams, Vec<ValidatorQ>)> {
        if data.len() < PARAMS_LEN {
            return None;
        }
        let mix = MixQ { pow: rd_u128(data, 0), pos: rd_u128(data, 16), pom: rd_u128(data, 32) };
        let params = FinalParams {
            horizon: rd_u64(data, 48),
            threshold_bps: u16::from_le_bytes([data[56], data[57]]) as u64,
            quorum_floor_bps: u16::from_le_bytes([data[58], data[59]]) as u64,
            decay_pos: data[60] != 0,
        };
        let rest = &data[PARAMS_LEN..];
        if rest.len() % VREC_LEN != 0 {
            return None;
        }
        let mut vs = Vec::with_capacity(rest.len() / VREC_LEN);
        for r in rest.chunks_exact(VREC_LEN) {
            vs.push(ValidatorQ {
                id: rd_u64(r, 0),
                pow: rd_u128(r, 8),
                pos: rd_u128(r, 24),
                pom: rd_u128(r, 40),
                last_heartbeat: rd_u64(r, 56),
            });
        }
        Some((mix, params, vs))
    }

    /// Decode a vote witness into validator indices. `None` if the byte length is odd or any
    /// index is out of range for an `n`-validator set (a vote for a non-existent validator is
    /// malformed, not silently dropped).
    pub fn parse_votes(data: &[u8], n: usize) -> Option<Vec<usize>> {
        if data.len() % 2 != 0 {
            return None;
        }
        let mut out = Vec::with_capacity(data.len() / 2);
        for c in data.chunks_exact(2) {
            let idx = u16::from_le_bytes([c[0], c[1]]) as usize;
            if idx >= n {
                return None;
            }
            out.push(idx);
        }
        Some(out)
    }

    /// Producer/test mirror of `parse_finalization_cell` — the other half of the single source.
    pub fn encode_finalization_cell(mix: MixQ, p: &FinalParams, validators: &[ValidatorQ]) -> Vec<u8> {
        let mut out = Vec::with_capacity(PARAMS_LEN + validators.len() * VREC_LEN);
        out.extend_from_slice(&mix.pow.to_le_bytes());
        out.extend_from_slice(&mix.pos.to_le_bytes());
        out.extend_from_slice(&mix.pom.to_le_bytes());
        out.extend_from_slice(&p.horizon.to_le_bytes());
        out.extend_from_slice(&(p.threshold_bps as u16).to_le_bytes());
        out.extend_from_slice(&(p.quorum_floor_bps as u16).to_le_bytes());
        out.push(p.decay_pos as u8);
        out.extend_from_slice(&[0u8; 3]); // pad to PARAMS_LEN
        for v in validators {
            out.extend_from_slice(&v.id.to_le_bytes());
            out.extend_from_slice(&v.pow.to_le_bytes());
            out.extend_from_slice(&v.pos.to_le_bytes());
            out.extend_from_slice(&v.pom.to_le_bytes());
            out.extend_from_slice(&v.last_heartbeat.to_le_bytes());
        }
        out
    }

    /// Producer/test mirror of `parse_votes`.
    pub fn encode_votes(indices: &[u16]) -> Vec<u8> {
        let mut out = Vec::with_capacity(indices.len() * 2);
        for &i in indices {
            out.extend_from_slice(&i.to_le_bytes());
        }
        out
    }
}
