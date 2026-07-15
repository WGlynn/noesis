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

// ============ Index-cell root-transition rule (T7 #3, single source of node::index_rule) ============
//
// The transition rule the novelty-index cell's type-script enforces: the committed seen-shingle root
// may advance old -> new ONLY as an EXACT chain of single-key SMT insertions, each proven against the
// ROLLING root. Kept here (no_std) so the on-VM index type-script and the node reference are ONE
// arithmetic (the single-source discipline of `finalization` / `commit_order` / `tx`). The node
// re-exports these and drives the drift-guard tests through its maintainer-side `NoveltyIndex`
// producer (which, being maintainer-side state, stays in the node by design).
//
// Load-bearing detail: intermediate roots are COMPUTED from each step's own sibling path
// (`root_from(key, EMPTY, siblings)` checked == rolling root, then `root_from(key, leaf, siblings)`),
// never supplied by the producer. Two structural consequences, not bookkept:
//   - duplicate insertion is impossible (the second insert of a key cannot prove non-membership
//     under the rolling root that now contains it);
//   - smuggling or omitting a key moves the computed final root off `new_root`.
pub mod index_rule {
    use crate::commit_order;
    use crate::{leaf, root_from, Hash, DEPTH};
    use alloc::vec::Vec;

    /// One insertion in the block's batch: the key and its sibling path against the rolling root at
    /// this position in the chain.
    #[derive(Clone)]
    pub struct InsertStep {
        pub key: u64,
        pub siblings: [Hash; DEPTH],
    }

    /// The transition rule the index cell's type-script enforces.
    pub fn valid_root_transition(old_root: Hash, new_root: Hash, steps: &[InsertStep]) -> bool {
        let mut root = old_root;
        for step in steps {
            if root_from(step.key, [0u8; 32], &step.siblings) != root {
                return false; // not absent under the rolling root (dup, stale, or forged)
            }
            root = root_from(step.key, leaf(step.key), &step.siblings);
        }
        root == new_root
    }

    /// A single cell's contribution to the block's index batch: its CONSENSUS-SOURCED commit
    /// coordinate ([`commit_order::Committed`]) paired with the novel-shingle insertions it makes
    /// against the rolling root. Grouping at cell granularity binds the ORDER the cells are applied
    /// in to consensus, not to producer presentation.
    #[derive(Clone)]
    pub struct CellBatch {
        pub coord: commit_order::Committed,
        pub steps: Vec<InsertStep>,
    }

    /// The index-cell transition rule WITH the commit-order invariant wired in.
    /// [`valid_root_transition`] proves the root moved correctly but TRUSTS the producer's order of
    /// steps — and order is exactly what decides first-commit-wins when two same-height cells contend
    /// for shared novel coverage (the first to insert a shared key banks it; the second can no longer
    /// prove non-membership => earns 0 for that key). This variant closes the invariant at per-cell-
    /// batch granularity: the cells must ALREADY be in canonical commit order
    /// ([`commit_order::is_canonical_order`] — height ascending, then the XOR-seeded in-block slot,
    /// NEITHER producer-arrangeable), and ONLY THEN is the flattened rolling-root transition checked.
    /// A producer-favorable reordering is REJECTED at the order gate before any root math (no silent
    /// re-sort => no probe signal), so no party can choose which of two contending cells banks the
    /// shared shingles.
    pub fn valid_ordered_root_transition(
        old_root: Hash,
        new_root: Hash,
        cells: &[CellBatch],
    ) -> bool {
        // 1. Consensus order gate: the cells must be presented in canonical commit order.
        let coords: Vec<commit_order::Committed> = cells.iter().map(|c| c.coord.clone()).collect();
        if !commit_order::is_canonical_order(&coords) {
            return false;
        }
        // 2. Rolling-root transition over the steps, flattened in that consensus-fixed order.
        let mut root = old_root;
        for cell in cells {
            for step in &cell.steps {
                if root_from(step.key, [0u8; 32], &step.siblings) != root {
                    return false; // not absent under the rolling root (dup, stale, or forged)
                }
                root = root_from(step.key, leaf(step.key), &step.siblings);
            }
        }
        root == new_root
    }

    // ---- Wire format (single home for the on-VM index-batch layout) ----
    // The index type-script DECODES; the node/producer ENCODES. Same codec discipline as
    // `commit_order::{parse,encode}_batch` and `finalization::{parse,encode}_*`. Layout:
    //   old_root (Hash, 32) ‖ new_root (Hash, 32)                    -- the transition endpoints
    //   then N cell-batch records, each:
    //     height (u64 LE, 8) ‖ secret (Hash, 32) ‖ n_steps (u32 LE, 4)   -- the consensus coord
    //     then n_steps × step: key (u64 LE, 8) ‖ siblings (DEPTH*32)       -- the rolling-root path
    // A cell may carry n_steps==0 (a fully-redundant cell earns nothing but still ORDERS); an
    // empty batch (zero cells) transitions nothing => malformed, like the empty commit/finalize group.
    pub const ROOTS_LEN: usize = 64;
    pub const CELL_HDR_LEN: usize = 8 + 32 + 4; // height + secret + n_steps
    pub const STEP_LEN: usize = 8 + DEPTH * 32; // key + one full sibling path

    /// Decode an on-VM index batch: `(old_root, new_root, cells)`. `None` on any length that is not
    /// a well-formed header + exact cell/step records, or a zero-cell batch (transitions nothing).
    pub fn parse_index_batch(data: &[u8]) -> Option<(Hash, Hash, Vec<CellBatch>)> {
        if data.len() < ROOTS_LEN {
            return None;
        }
        let mut old_root = [0u8; 32];
        old_root.copy_from_slice(&data[0..32]);
        let mut new_root = [0u8; 32];
        new_root.copy_from_slice(&data[32..64]);
        let mut cells = Vec::new();
        let mut off = ROOTS_LEN;
        while off < data.len() {
            if off + CELL_HDR_LEN > data.len() {
                return None; // truncated cell header
            }
            let height = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
            let mut secret = [0u8; 32];
            secret.copy_from_slice(&data[off + 8..off + 40]);
            let n_steps = u32::from_le_bytes(data[off + 40..off + 44].try_into().unwrap()) as usize;
            off += CELL_HDR_LEN;
            // Capacity from REAL remaining length, never from the self-declared `n_steps` (the codec
            // discipline of the sibling parsers): a forged header claiming a huge count must NOT drive
            // a multi-TB pre-allocation that traps the VM — it must fall to a clean malformed reject.
            if n_steps > (data.len() - off) / STEP_LEN {
                return None; // more steps than the remaining bytes can hold
            }
            let mut steps = Vec::with_capacity(n_steps);
            for _ in 0..n_steps {
                if off + STEP_LEN > data.len() {
                    return None; // truncated step record
                }
                let key = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
                let mut siblings = [[0u8; 32]; DEPTH];
                for (i, sib) in siblings.iter_mut().enumerate() {
                    let b = off + 8 + i * 32;
                    sib.copy_from_slice(&data[b..b + 32]);
                }
                steps.push(InsertStep { key, siblings });
                off += STEP_LEN;
            }
            cells.push(CellBatch { coord: commit_order::Committed { height, secret }, steps });
        }
        if cells.is_empty() {
            return None; // a batch with no cells transitions nothing
        }
        Some((old_root, new_root, cells))
    }

    /// Producer/test mirror of `parse_index_batch` — the other half of the single source.
    pub fn encode_index_batch(old_root: Hash, new_root: Hash, cells: &[CellBatch]) -> Vec<u8> {
        let mut out = Vec::with_capacity(ROOTS_LEN);
        out.extend_from_slice(&old_root);
        out.extend_from_slice(&new_root);
        for cell in cells {
            out.extend_from_slice(&cell.coord.height.to_le_bytes());
            out.extend_from_slice(&cell.coord.secret);
            out.extend_from_slice(&(cell.steps.len() as u32).to_le_bytes());
            for step in &cell.steps {
                out.extend_from_slice(&step.key.to_le_bytes());
                for sib in &step.siblings {
                    out.extend_from_slice(sib);
                }
            }
        }
        out
    }
}

// ============ zk-score (Fit 2) single-source: floor, wire, digest, nullifier, verdict ============
// One home for the pieces the zk-score guest + host + parity must agree on, so the private-scoring
// path cannot drift between the prover and the ground-truth harness. The BYTE LAYOUT of the digest,
// nullifier and proof-wire lives here, matching the codec discipline of the other cross-VM wires
// (commit_order / finalization / tx). sha256 (sha2 crate, no_std) is used for the public bindings so
// the values match the guest's commitment exactly.

/// Protocol floor a private contribution must clear to earn standing. BAKED here (and thus into the
/// guest image id) so the bar is NOT a prover-chosen input — closes the `v_min=0` forgery and the
/// cross-receipt score-disclosure (a receipt proves only `value >= ZK_SCORE_V_FLOOR`; the exact score
/// cannot be binary-searched by re-proving at attacker-chosen bounds, because there is no bound to choose).
pub const ZK_SCORE_V_FLOOR: u64 = 5;

/// PUBLIC-input digest bound into the journal: sha256(domain ‖ root ‖ theta_sim_le ‖ theta_ent_le ‖
/// V_FLOOR_le). The verifier MUST recompute this from the CANONICAL corpus root + policy thetas and
/// reject any receipt whose digest differs — that comparison is what pins `root` (an otherwise
/// prover-chosen input) to the real corpus. Binding-into-a-hash only pins the value; the verifier's
/// comparison AUTHENTICATES it.
pub fn zk_score_public_digest(root: Hash, theta_sim_q16: u64, theta_ent_q16: u64) -> Hash {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"noesis-zkscore-digest-v1");
    h.update(root);
    h.update(theta_sim_q16.to_le_bytes());
    h.update(theta_ent_q16.to_le_bytes());
    h.update(ZK_SCORE_V_FLOOR.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    out
}

/// Content nullifier: sha256(domain ‖ each sorted-unique shingle key ‖ its multiplicity). It commits
/// the shingle REPRESENTATION of the content (keys + occurrence counts — the same object the score
/// consumes), NOT the raw bytes and NOT identity (that is Fit 4). Preimage-resistant, so it reveals
/// nothing about the content. Scope, stated honestly: it dedups EXACT-content (same shingle multiset)
/// resubmission only — a near-duplicate (a one-byte edit) yields a DIFFERENT nullifier, so near-dup /
/// work-splitting double-claim is NOT closed here; that needs a fuzzy / overlap-against-prior-grants
/// accumulator at the ledger layer. Multiplicities are folded in so two genuinely-distinct contents
/// that merely share a unique-key set do not collide to one nullifier.
pub fn zk_score_nullifier(data: &[u8]) -> Hash {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"noesis-zkscore-nullifier-v2");
    for (k, m) in unique_shingles(data) {
        h.update(k.to_le_bytes());
        h.update(m.to_le_bytes());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    out
}

/// Flat proof wire — the guest cannot serde a `[[u8; 32]; DEPTH]` array directly, so proofs cross the
/// boundary as bytes. SINGLE SOURCE for guest + host + parity so the one wire unique to Fit 2 cannot
/// drift between the prover and the harness that validates it.
pub fn flatten_proofs(proofs: &[[Hash; DEPTH]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(proofs.len() * DEPTH * 32);
    for p in proofs {
        for h in p {
            out.extend_from_slice(h);
        }
    }
    out
}

/// Inverse of `flatten_proofs`. `None` on any length that is not `n * DEPTH * 32` (malformed witness).
pub fn unflatten_proofs(flat: &[u8], n: usize) -> Option<Vec<[Hash; DEPTH]>> {
    if flat.len() != n * DEPTH * 32 {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let base = i * DEPTH * 32;
        let mut p = [[0u8; 32]; DEPTH];
        for (j, slot) in p.iter_mut().enumerate() {
            let off = base + j * 32;
            slot.copy_from_slice(&flat[off..off + 32]);
        }
        out.push(p);
        i += 1;
    }
    Some(out)
}

/// The full zk-score verdict over PRIVATE content — the SINGLE definition guest + host + parity run.
/// Returns `None` (=> reject, `accepted = false`) for an EMPTY / sub-shingle cell (no vacuous accept)
/// OR a malformed / forged proof witness (a path proving neither polarity rejects the whole cell);
/// else `(nullifier, floored_value)`. The caller commits `(public_digest, nullifier, accepted,
/// value >= ZK_SCORE_V_FLOOR)`. `accepted` attests witness-internal consistency against the SUPPLIED
/// `root`; its trust is conditional on the verifier pinning `public_digest` to the canonical root.
pub fn zk_score_eval(
    data: &[u8],
    root: Hash,
    proofs: &[[Hash; DEPTH]],
    theta_sim_q16: u64,
    theta_ent_q16: u64,
) -> Option<(Hash, u64)> {
    if unique_shingles(data).is_empty() {
        return None; // empty / sub-shingle content is not a well-formed contribution
    }
    let value = proven_floored_novelty_q16(data, root, proofs, theta_sim_q16, theta_ent_q16)?;
    Some((zk_score_nullifier(data), value))
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

    /// Anti-concentration floor (mirrors `runtime::finality::MIN_DIM_BPS`): each fast-final dimension
    /// must independently supply ≥ 50% of its OWN dimension total.
    pub const MIN_DIM_BPS: u64 = 5000;

    /// PoW-free finality mix in Q32.32 (mirrors `runtime::finality::FINALITY_MIX = 0 : 1/3 : 2/3`).
    /// `pos + pom = ONE` exactly (the two fast-final dimensions renormalized; PoW out of finality).
    pub const FINALITY_MIX_Q: MixQ = MixQ { pow: 0, pos: 0x5555_5555, pom: 0xAAAA_AAAB };

    /// A dimension clears the anti-concentration floor. Mirrors `runtime::finality::dim_ok`: a
    /// dimension absent from the whole set can't gate (avoids /0); else the voting weight in that
    /// dimension must clear `MIN_DIM_BPS` of the dimension total. The floor rounds UP (against
    /// finalization) so the fixed gate is NEVER more permissive than the f64 one.
    fn dim_ok_q(weight_for: u128, weight_all: u128) -> bool {
        weight_all == 0 || weight_for >= bps_of_ceil(weight_all, MIN_DIM_BPS)
    }

    /// PoS+PoM finality in Q32.32 — the on-VM mirror of the live `runtime::finality::finalizes_pos_pom`
    /// (ROADMAP (mm)/(oo)). PoW is excluded by `FINALITY_MIX_Q` (probabilistic/reorgeable ⇒ a
    /// finality-safety vector); the anti-concentration floor forces BOTH the capital (PoS) and value
    /// (PoM) axes to participate, so PoM's 60% cannot unilaterally finalize (T11 capital-orthogonality).
    /// The on-VM finalization type-script will call THIS, so the live (mm) rule and its future on-VM
    /// mirror are ONE arithmetic — closing the forward parity (mm) documented. Bit-for-bit deterministic.
    pub fn finalizes_pos_pom_fixed(
        voters_for: &[ValidatorQ],
        all: &[ValidatorQ],
        now: u64,
        horizon: u64,
        decay_pos: bool,
        threshold_bps: u64,
    ) -> bool {
        // PoW out of finality + the usual 2/3 supermajority of the fast-final (PoS+PoM) set.
        if !finalizes_fixed(voters_for, all, FINALITY_MIX_Q, now, horizon, decay_pos, threshold_bps, 0) {
            return false;
        }
        // Anti-concentration over the RAW (unweighted) dimension balances, mirroring the f64 rule.
        let (mut pos_for, mut pos_all, mut pom_for, mut pom_all) = (0u128, 0u128, 0u128, 0u128);
        for v in voters_for {
            pos_for = pos_for.saturating_add(v.pos);
            pom_for = pom_for.saturating_add(v.pom);
        }
        for v in all {
            pos_all = pos_all.saturating_add(v.pos);
            pom_all = pom_all.saturating_add(v.pom);
        }
        dim_ok_q(pos_for, pos_all) && dim_ok_q(pom_for, pom_all)
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
            // A vote set is a SET: a repeated index would double-count that validator's effective
            // weight and inflate `weight_for`, forging finalization. Reject duplicates (RSAW 2026-06-13).
            if out.contains(&idx) {
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

/// Post-quantum lock-sig verifier - hash-based Lamport one-time signatures. SINGLE SOURCE for the
/// verify arithmetic the on-VM lock-script links and the node validates with (same pattern as the
/// finalization mirror). Hash-rooted: a public key is one 32-byte blake2b root, carried as a cell's
/// `lock.args`. One-time-safe for free: a cell is consumed exactly once, so its lock key signs once.
/// no_std + alloc; builds for riscv64imac. keygen/sign are key-holder (wallet) tooling; a node only
/// ever VERIFIES (they are pub here so the wallet/tests can link them - a lib does not dead-code-warn
/// pub items).
pub mod lamport {
    use alloc::vec::Vec;

    const N: usize = 256; // a 32-byte message digest = 256 bits => one keypair column per bit

    /// Domain-separated 32-byte blake2b. `tag` distinguishes secret-leaf / pk-leaf / root preimages
    /// so the three uses can never collide; the personalization separates Lamport hashes from the tx
    /// digest and the novelty-index node hash.
    fn h(tag: u8, parts: &[&[u8]]) -> [u8; 32] {
        let mut hasher = blake2b_ref::Blake2bBuilder::new(32)
            .personal(b"noesis-lamp-v1\0\0")
            .build();
        hasher.update(&[tag]);
        for p in parts {
            hasher.update(p);
        }
        let mut out = [0u8; 32];
        hasher.finalize(&mut out);
        out
    }

    fn secret_leaf(seed: &[u8; 32], i: usize, b: u8) -> [u8; 32] {
        h(0x01, &[seed, &(i as u32).to_le_bytes(), &[b]])
    }
    fn pk_leaf(preimage: &[u8; 32]) -> [u8; 32] {
        h(0x02, &[preimage])
    }
    /// Commit the full 2N public-key table in canonical (i, bit) order to one 32-byte root.
    fn root(table: &[[u8; 32]]) -> [u8; 32] {
        let flat: Vec<u8> = table.iter().flat_map(|leaf| leaf.iter().copied()).collect();
        h(0x03, &[&flat])
    }
    fn bit(msg: &[u8; 32], i: usize) -> u8 {
        (msg[i / 8] >> (i % 8)) & 1
    }

    /// Deterministic keygen from a 32-byte seed -> the public-key root (a cell's `lock.args`).
    /// Key-holder / test tooling; the node itself only ever VERIFIES.
    pub fn keygen_root(seed: &[u8; 32]) -> [u8; 32] {
        let mut table = Vec::with_capacity(2 * N);
        for i in 0..N {
            for b in 0..2u8 {
                table.push(pk_leaf(&secret_leaf(seed, i, b)));
            }
        }
        root(&table)
    }

    /// Sign a 32-byte message: per bit, reveal the preimage for that bit AND carry the SIBLING pk
    /// hash (which the verifier cannot derive without the unrevealed preimage). 256 x 64 B = 16 KiB.
    pub fn sign(seed: &[u8; 32], msg: &[u8; 32]) -> Vec<u8> {
        let mut sig = Vec::with_capacity(N * 64);
        for i in 0..N {
            let b = bit(msg, i);
            sig.extend_from_slice(&secret_leaf(seed, i, b)); // revealed preimage for the message bit
            sig.extend_from_slice(&pk_leaf(&secret_leaf(seed, i, 1 - b))); // sibling pk hash
        }
        sig
    }

    /// Verify `sig` for `msg` under the public-key `root_commit` (the finalized cell's `lock.args`).
    /// Reconstructs the 2N pk table - revealed-bit slot from `H(preimage)`, sibling slot from the
    /// carried hash - and checks it commits back to `root_commit`. Forging a DIFFERENT message needs
    /// an unrevealed preimage (a hash break); the key signs once, so one-time security holds.
    pub fn verify(root_commit: &[u8; 32], msg: &[u8; 32], sig: &[u8]) -> bool {
        if sig.len() != N * 64 {
            return false; // malformed length is not a signature
        }
        let mut table = Vec::with_capacity(2 * N);
        for i in 0..N {
            let b = bit(msg, i);
            let preimage: &[u8; 32] = sig[i * 64..i * 64 + 32].try_into().unwrap();
            let sibling: [u8; 32] = sig[i * 64 + 32..i * 64 + 64].try_into().unwrap();
            let revealed = pk_leaf(preimage);
            // place revealed in the message-bit slot, sibling in the other, table in (i,0),(i,1) order.
            let (slot0, slot1) = if b == 0 { (revealed, sibling) } else { (sibling, revealed) };
            table.push(slot0);
            table.push(slot1);
        }
        &root(&table) == root_commit
    }
}

/// Canonical transaction digest - SINGLE SOURCE for the bytes a lock-signature covers and the
/// replica-deterministic identity of a value movement. Lives here (no_std, builds riscv) so the
/// on-VM lock-script type-script recomputes the SAME digest the node signs/verifies over - paying the
/// single-source debt the node `TokenTx::digest` flagged. Injective length-prefix framing; canonical
/// input/output order; tx-domain blake2b personalization distinct from the smt + lamport hashers.
pub mod tx {
    use alloc::vec::Vec;

    /// A borrowed view of a cell's consensus identity - exactly the fields the digest commits to
    /// (the ledger identity tuple + `data`). Built by the node from its `Cell`, and on-VM from the
    /// loaded cell fields.
    pub struct CellView<'a> {
        pub id: u64,
        pub lock_code_hash: &'a [u8; 32],
        pub lock_args: &'a [u8],
        pub type_code_hash: &'a [u8; 32],
        pub type_args: &'a [u8],
        pub data: &'a [u8],
    }

    type IdentKey<'a> = (u64, &'a [u8; 32], &'a [u8], &'a [u8; 32], &'a [u8], &'a [u8]);
    fn ident_key<'a>(c: &CellView<'a>) -> IdentKey<'a> {
        (c.id, c.lock_code_hash, c.lock_args, c.type_code_hash, c.type_args, c.data)
    }

    /// Injective length-prefix framing for a variable-length field.
    fn put(buf: &mut Vec<u8>, bytes: &[u8]) {
        buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(bytes);
    }
    fn serialize_cell(buf: &mut Vec<u8>, c: &CellView) {
        buf.extend_from_slice(&c.id.to_le_bytes());
        buf.extend_from_slice(c.lock_code_hash);
        put(buf, c.lock_args);
        buf.extend_from_slice(c.type_code_hash);
        put(buf, c.type_args);
        put(buf, c.data);
    }

    /// The canonical, injective 32-byte digest of a value movement. Inputs/outputs are serialized in
    /// canonical identity order (sorted on a copy of the indices - the caller's slices are never
    /// mutated), so a re-presented tx hashes identically. `standard` is the token-standard tag byte.
    pub fn tx_digest(
        standard: u8,
        code_hash: &[u8; 32],
        args: &[u8],
        inputs: &[CellView],
        outputs: &[CellView],
    ) -> [u8; 32] {
        let mut in_order: Vec<usize> = (0..inputs.len()).collect();
        let mut out_order: Vec<usize> = (0..outputs.len()).collect();
        in_order.sort_by(|&a, &b| ident_key(&inputs[a]).cmp(&ident_key(&inputs[b])));
        out_order.sort_by(|&a, &b| ident_key(&outputs[a]).cmp(&ident_key(&outputs[b])));

        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(b"noesis-tx-v1"); // domain tag in the preimage as well as the personalization
        buf.push(standard);
        buf.extend_from_slice(code_hash);
        put(&mut buf, args);
        buf.extend_from_slice(&(inputs.len() as u64).to_le_bytes());
        for &i in &in_order {
            serialize_cell(&mut buf, &inputs[i]);
        }
        buf.extend_from_slice(&(outputs.len() as u64).to_le_bytes());
        for &i in &out_order {
            serialize_cell(&mut buf, &outputs[i]);
        }

        let mut h = blake2b_ref::Blake2bBuilder::new(32)
            .personal(b"noesis-tx-v1\0\0\0\0")
            .build();
        h.update(&buf);
        let mut out = [0u8; 32];
        h.finalize(&mut out);
        out
    }

    /// An OWNED cell identity — the on-VM lock script loads bytes it owns, then borrows them as a
    /// [`CellView`] to feed [`tx_digest`]. Same six fields the digest commits to.
    pub struct OwnedCellView {
        pub id: u64,
        pub lock_code_hash: [u8; 32],
        pub lock_args: Vec<u8>,
        pub type_code_hash: [u8; 32],
        pub type_args: Vec<u8>,
        pub data: Vec<u8>,
    }

    impl OwnedCellView {
        /// Borrow this owned record as the digest's `CellView` (zero-copy).
        pub fn view(&self) -> CellView<'_> {
            CellView {
                id: self.id,
                lock_code_hash: &self.lock_code_hash,
                lock_args: &self.lock_args,
                type_code_hash: &self.type_code_hash,
                type_args: &self.type_args,
                data: &self.data,
            }
        }
    }

    /// Serialize ONE cell's consensus identity as a self-delimited record, reusing the digest's own
    /// `serialize_cell` framing so encode and parse share ONE injective format. This is the pre-deploy
    /// model wire by which the host harness serves a cell's identity to the on-VM lock script (one
    /// record per `load_cell_data`). At deploy the same fields are sourced from real CKB cell-field
    /// syscalls (the `CELL_FIELDS_BOUND` binding) — structure now, consensus field-loading at deploy,
    /// the same boundary as the index-dep and header-`now` bindings.
    pub fn encode_cell_identity(c: &CellView) -> Vec<u8> {
        let mut buf = Vec::new();
        serialize_cell(&mut buf, c);
        buf
    }

    /// Parse exactly one record produced by [`encode_cell_identity`]. Bounds-checked end to end —
    /// returns `None` on any short read OR trailing bytes (the blob must be one whole record), never
    /// panics on attacker-supplied bytes. The inverse of `serialize_cell`.
    pub fn parse_cell_identity(data: &[u8]) -> Option<OwnedCellView> {
        fn take<'a>(d: &'a [u8], p: &mut usize, n: usize) -> Option<&'a [u8]> {
            let end = p.checked_add(n)?;
            if end > d.len() {
                return None;
            }
            let s = &d[*p..end];
            *p = end;
            Some(s)
        }
        fn take_u64(d: &[u8], p: &mut usize) -> Option<u64> {
            Some(u64::from_le_bytes(take(d, p, 8)?.try_into().ok()?))
        }
        fn take_arr32(d: &[u8], p: &mut usize) -> Option<[u8; 32]> {
            let mut a = [0u8; 32];
            a.copy_from_slice(take(d, p, 32)?);
            Some(a)
        }
        fn take_var(d: &[u8], p: &mut usize) -> Option<Vec<u8>> {
            let n = take_u64(d, p)? as usize;
            Some(take(d, p, n)?.to_vec())
        }
        let mut p = 0usize;
        let id = take_u64(data, &mut p)?;
        let lock_code_hash = take_arr32(data, &mut p)?;
        let lock_args = take_var(data, &mut p)?;
        let type_code_hash = take_arr32(data, &mut p)?;
        let type_args = take_var(data, &mut p)?;
        let cell_data = take_var(data, &mut p)?;
        if p != data.len() {
            return None; // trailing bytes ⇒ not a single clean record
        }
        Some(OwnedCellView { id, lock_code_hash, lock_args, type_code_hash, type_args, data: cell_data })
    }
}

// ============ PoW work dimension (M2) — target math + header hasher ============
//
// The verify-side arithmetic the node (host) and a future on-VM PoW check must AGREE on, kept here
// (no_std) so the on-VM mirror is a move, not a rewrite — the same single-source discipline as `tx`
// and `lamport`. M2a-1 SCOPE: the pure math + the domain-separated hasher + the length-prefix framer
// ONLY. Nothing here is wired into `validate_block` / `block_work` yet (that is M2a-2, flag-gated).
// The RETARGET rule and EVERY numeric constant (genesis bits, block interval, Ergon params) are ⚑ M3
// (Will-gated) and are deliberately absent — asserting one would violate no-assert-from-memory.
/// PoW target arithmetic + the header-preimage hasher. Pure, integer-only, no clock / rand / float.
pub mod pow {
    use alloc::vec::Vec;

    /// Domain-separated 32-byte blake2b over a caller-assembled preimage. The node builds the block
    /// header preimage (its `Block`-specific field layout); a future on-VM check builds the same
    /// preimage from cell syscalls. The personalization keeps PoW hashes disjoint from tx/smt/lamport.
    pub fn hash(preimage: &[u8]) -> [u8; 32] {
        let mut h = blake2b_ref::Blake2bBuilder::new(32)
            .personal(b"noesis-pow-v1\0\0\0")
            .build();
        h.update(preimage);
        let mut out = [0u8; 32];
        h.finalize(&mut out);
        out
    }

    /// Injective length-prefix framing (mirrors `tx::put`) so a caller assembles an unambiguous
    /// header preimage from variable-length fields — one home for the framing both worlds use.
    pub fn put(buf: &mut Vec<u8>, bytes: &[u8]) {
        buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(bytes);
    }

    /// Decode a Bitcoin-style compact target ("nBits", `(exponent << 24) | mantissa24`) into a
    /// 256-bit big-endian target, with STRICT validation: the sign bit is rejected (no negative
    /// target), a zero mantissa is rejected (unmeetable), and any encoding whose non-zero bytes would
    /// overflow 32 bytes is rejected. Returns `None` on any malformed input — never panics.
    pub fn compact_to_target(bits: u32) -> Option<[u8; 32]> {
        let exponent = (bits >> 24) as usize;
        let mantissa = bits & 0x00ff_ffff;
        if mantissa & 0x0080_0000 != 0 {
            return None; // sign bit set ⇒ "negative" ⇒ reject
        }
        if mantissa == 0 {
            return None; // zero target ⇒ unmeetable ⇒ reject
        }
        let mut target = [0u8; 32];
        // most-significant mantissa byte first
        let mant_bytes = [
            ((mantissa >> 16) & 0xff) as u8,
            ((mantissa >> 8) & 0xff) as u8,
            (mantissa & 0xff) as u8,
        ];
        if exponent <= 3 {
            // Bitcoin defines exp<=3 as target = mantissa >> 8*(3-exponent): keep the top `exponent`
            // mantissa bytes, placed at the very low end. exp==0 keeps nothing ⇒ target 0 (rejected below).
            let keep = exponent.min(3);
            for k in 0..keep {
                target[31 - (keep - 1 - k)] = mant_bytes[k];
            }
        } else {
            // shift left by (exponent - 3) bytes: the mantissa's low byte sits at index 31-(exp-3).
            let shift = exponent - 3;
            for (k, &b) in mant_bytes.iter().rev().enumerate() {
                // k = 0 is the mantissa LSB (index 31-shift); k grows toward the MSB (index decreases).
                let idx = 31isize - shift as isize - k as isize;
                if b != 0 && idx < 0 {
                    return None; // a non-zero byte overflows the 32-byte target
                }
                if (0..32).contains(&idx) {
                    target[idx as usize] = b;
                }
            }
        }
        if target == [0u8; 32] {
            return None; // degenerate (e.g. exponent 0) ⇒ unmeetable zero target ⇒ reject
        }
        Some(target)
    }

    /// Encode a 256-bit big-endian target back into a Bitcoin-style compact target ("nBits") — the
    /// strict inverse of [`compact_to_target`], needed by the M3 retarget controller to emit a
    /// candidate block's `bits` from a computed target. Produces the CANONICAL compact (no redundant
    /// leading-zero mantissa byte; the sign bit is never set): for any canonical `bits`,
    /// `target_to_compact(compact_to_target(bits).unwrap()) == Some(bits)` (round-trip). A zero target
    /// is rejected (`None`) — unmeetable, mirroring `compact_to_target`'s zero rejection. Total,
    /// integer-only, never panics (the transition-purity contract).
    ///
    /// The compact format carries only a 3-byte mantissa, so encoding a target with significant bytes
    /// BELOW the top three truncates toward zero — the standard Bitcoin `GetCompact` behavior. The
    /// round-trip identity therefore holds on the canonical compact space (every `bits` accepted by
    /// `compact_to_target`), not on the full 2^256 target space; the M3 controller always re-encodes a
    /// target it just derived, so it lives on that canonical space by construction. No numeric constant
    /// (genesis bits, interval, retarget params) is introduced here — those stay ⚑ M3.
    pub fn target_to_compact(target: &[u8; 32]) -> Option<u32> {
        // index of the most-significant non-zero byte; all-zero ⇒ None (unmeetable), matches the decoder.
        let msb = target.iter().position(|&b| b != 0)?;
        let mut size = (32 - msb) as u32; // number of significant bytes (Bitcoin's nSize / exponent)
        // top three significant bytes, big-endian, as a 24-bit mantissa (bytes past the end ⇒ 0). This
        // one expression is correct for every size: for size < 3 the low significant byte(s) land in the
        // high part of the mantissa exactly as Bitcoin's `nCompact = low << 8*(3-nSize)` prescribes.
        let b0 = target[msb] as u32;
        let b1 = if msb + 1 < 32 { target[msb + 1] as u32 } else { 0 };
        let b2 = if msb + 2 < 32 { target[msb + 2] as u32 } else { 0 };
        let mut mantissa = (b0 << 16) | (b1 << 8) | b2;
        // the mantissa's top bit must never read as the sign bit: shift down one byte and grow the
        // exponent (this is why e.g. the max target 0x1d00ffff carries a leading-zero mantissa byte).
        if mantissa & 0x0080_0000 != 0 {
            mantissa >>= 8;
            size += 1;
        }
        // size is at most 33 (all-ones target after the sign-shift); the resulting `bits` always decodes
        // without the decoder's overflow rejection because the shifted mantissa's top byte is zero.
        Some((size << 24) | mantissa)
    }

    /// Chainwork of a target: `floor((2^256 - 1) / (target + 1))`, saturated to `u64`. Every valid
    /// block advances the monotone work-clock by >= 1: the all-ones (easiest) target returns the
    /// minimum 1 via the `target + 1` overflow branch, and the quotient is >= 1 for every other
    /// representable target. Monotone NON-INCREASING in target — a smaller (harder) target yields AT
    /// LEAST as much work; strictly more only across sufficiently separated targets, since floor
    /// division and u64 saturation tie adjacent and extreme targets. That weak monotonicity is exactly
    /// right for a cumulative work-clock (and for equal-difficulty blocks contributing equal weight).
    /// Pure integer 256-bit long division — no float, no clock, no rand (the transition-purity
    /// contract, `runtime::apply_transition`). This is the value `block_work` returns under enforcement
    /// (M2a-2), by which point `validate_block` has already proven the seal.
    pub fn work_from_target(target: &[u8; 32]) -> u64 {
        // divisor = target + 1 (256-bit, big-endian). If target == 2^256-1, target+1 overflows ⇒ the
        // dividend (2^256-1) < divisor (2^256) ⇒ quotient 0 ⇒ clamp to the minimum work of 1.
        let mut divisor = *target;
        let mut carry = 1u16;
        let mut i = 32;
        while i > 0 && carry > 0 {
            i -= 1;
            let s = divisor[i] as u16 + carry;
            divisor[i] = (s & 0xff) as u8;
            carry = s >> 8;
        }
        if carry != 0 {
            return 1; // target + 1 == 2^256 ⇒ quotient 0 ⇒ minimum work
        }
        let dividend = [0xffu8; 32]; // 2^256 - 1
        let mut rem = [0u8; 32];
        let mut q: u64 = 0;
        for bit in 0..256usize {
            // rem <<= 1
            let mut c = 0u8;
            let mut j = 32;
            while j > 0 {
                j -= 1;
                let v = ((rem[j] as u16) << 1) | c as u16;
                rem[j] = (v & 0xff) as u8;
                c = (v >> 8) as u8;
            }
            // bring in the next dividend bit, MSB-first
            let byte = bit / 8;
            let off = 7 - (bit % 8);
            rem[31] |= (dividend[byte] >> off) & 1;
            let ge = cmp256(&rem, &divisor) != core::cmp::Ordering::Less;
            if ge {
                sub256(&mut rem, &divisor);
            }
            if q > (u64::MAX >> 1) {
                return u64::MAX; // any further growth stays saturated
            }
            q = (q << 1) | ge as u64;
        }
        // Defensive floor only: for every representable target the loop already yields q >= 1 (the
        // sole q == 0 case is target == all-ones, handled by the carry-overflow `return 1` above), so
        // `.max(1)` never fires today — it guards a future change to the division, it is not the
        // all-ones guarantee (that is the overflow branch). See pow_arithmetic anti-theater note.
        q.max(1)
    }

    fn cmp256(a: &[u8; 32], b: &[u8; 32]) -> core::cmp::Ordering {
        for i in 0..32 {
            match a[i].cmp(&b[i]) {
                core::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
        core::cmp::Ordering::Equal
    }

    fn sub256(a: &mut [u8; 32], b: &[u8; 32]) {
        let mut borrow = 0i16;
        let mut i = 32;
        while i > 0 {
            i -= 1;
            let d = a[i] as i16 - b[i] as i16 - borrow;
            if d < 0 {
                a[i] = (d + 256) as u8;
                borrow = 1;
            } else {
                a[i] = d as u8;
                borrow = 0;
            }
        }
    }

    // ============ ASERT difficulty retarget (inc-M3-1, DESIGN-M3 §3) ============

    /// Parameters for the [`next_target`] ASERT retarget. All caller-supplied — this module hard-codes NO
    /// economic number (interval, half-life, floor are ⚑ M3, ratified at wiring). `ideal_interval` and
    /// `half_life` are in the SAME time unit as the `observed` elapsed passed to [`next_target`].
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct RetargetParams {
        /// Target time per block (the schedule slope): `expected_elapsed = ideal_interval · height_delta`.
        pub ideal_interval: u64,
        /// ASERT time constant τ: the elapsed-time surplus/deficit that moves difficulty by exactly one
        /// doubling (2× target ⇒ ½ difficulty) or halving. Must be ≥ 1 (τ == 0 ⇒ [`next_target`] `None`).
        pub half_life: u64,
        /// Minimum difficulty = maximum (easiest) target, as compact bits. The retarget never returns a
        /// target easier than this (the death-spiral / never-halt floor). Must be a valid compact.
        pub pow_limit_bits: u32,
    }

    /// ASERT difficulty retarget — negative-solvetime-tolerant, integer-only, float-free (the
    /// transition-purity contract). Given the anchor block's compact target `anchor_bits`, the block-count
    /// since the anchor `height_delta`, and the OBSERVED elapsed time since the anchor, returns the next
    /// block's compact target.
    ///
    /// The exponent decomposes as `observed − (ideal_interval · height_delta)`: the **expected** (schedule)
    /// half is pure height + a constant (NO clock); only the **observed** half needs a timestamp.
    /// `observed == None` is the Phase-1 seam — no clock signal ⇒ treat the block as exactly on schedule
    /// (exponent 0) ⇒ target UNCHANGED. So the controller is live and testable while the timestamp source
    /// stays ⚑ M3 (the same "right interface, degenerate constant" discipline as `pow_enforced` default-false).
    ///
    /// Faster-than-schedule (`observed < expected`) ⇒ smaller (HARDER) target; slower ⇒ larger (EASIER),
    /// clamped to `pow_limit_bits` (never easier than the floor) and to a minimum target of 1 (never the
    /// unmeetable zero). Exact at whole half-lives: `observed = expected ± k·half_life ⇒ target = anchor·2^±k`
    /// (the cubic `2^frac` approximation only rounds the fractional part). Returns `None` on a malformed
    /// anchor/floor compact or a zero `half_life`; never panics.
    pub fn next_target(
        anchor_bits: u32,
        height_delta: u64,
        observed: Option<u64>,
        params: RetargetParams,
    ) -> Option<u32> {
        let anchor = compact_to_target(anchor_bits)?;
        let pow_limit = compact_to_target(params.pow_limit_bits)?;
        if params.half_life == 0 {
            return None; // τ == 0 ⇒ undefined (division) ⇒ reject, never divide-by-zero
        }
        // No clock signal ⇒ exactly-on-schedule seam ⇒ anchor unchanged (still floor-clamped + re-encoded
        // so the result is always a canonical compact).
        let observed = match observed {
            None => return clamp_and_encode(anchor, &pow_limit),
            Some(t) => t,
        };
        let expected = params.ideal_interval.saturating_mul(height_delta);
        // signed exponent in 16.16 fixed point: ((observed − expected) << 16) / half_life. i128 holds any
        // u64 difference shifted left 16 with room to spare.
        let (neg, diff) = if observed >= expected {
            (false, observed - expected)
        } else {
            (true, expected - observed)
        };
        let magnitude = ((diff as i128) << 16) / (params.half_life as i128);
        let exponent = if neg { -magnitude } else { magnitude };
        // decompose: arithmetic >> 16 floors toward −∞, and `& 0xffff` recovers the positive fractional
        // remainder in [0, 65536) — the standard two's-complement ASERT identity (holds for negatives).
        let shifts = exponent >> 16;
        let frac = (exponent & 0xffff) as u128;
        // 2^frac ≈ factor / 65536, factor ∈ [65536, 131072): single-sourced cubic (see `pow2_frac_q16`).
        let factor = pow2_frac_q16(frac);
        // next = anchor · factor, then a net bit-shift of (shifts − 16) (the −16 undoes the factor's ×65536).
        let mut t = mul_small256(&anchor, factor as u64);
        let net = shifts - 16;
        if net >= 0 {
            shl256_sat(&mut t, net.min(256) as u32); // ≥256 ⇒ overflow ⇒ saturates to the easiest target
        } else {
            shr256(&mut t, (-net).min(256) as u32); // ≤ −256 ⇒ underflows to 0 ⇒ pinned to 1 below
        }
        clamp_and_encode(t, &pow_limit)
    }

    /// `2^(frac/65536) · 65536` — the fractional-part power-of-two, as a Q16.16 factor in
    /// `[65536, 131072)`. BCH aserti3-2d cubic approximation: integer-only, float-free, replica-identical.
    /// `frac` must be in `[0, 65536)`. Single-sourced so both the ASERT retarget ([`next_target`]) and the
    /// Moore's-law issuance decay ([`moore_decay_q32`]) share ONE exponential (no duplicated magic constants
    /// to drift apart).
    pub fn pow2_frac_q16(frac: u128) -> u128 {
        65536u128
            + ((195_766_423_245_605u128 * frac
                + 971_821_376u128 * frac * frac
                + 5_127u128 * frac * frac * frac
                + (1u128 << 47))
                >> 48)
    }

    /// Moore's-law JUL-issuance decay multiplier `2^(−elapsed / halflife)`, returned as a **Q32 fraction**
    /// (× 2^32) in `[0, 2^32]`. This is the coefficient that holds JUL's ENERGY peg: a flat (no-decay)
    /// reward pegs to *hashes*, but hashes-per-joule rises as hardware improves, so an undecayed reward
    /// silently inflates JUL against energy. Halving JUL-per-hash every hardware-efficiency doubling period
    /// keeps JUL-per-*joule* constant (`DECISIONS-M3-money-2026-07-15.md` §1).
    ///
    /// `halflife` = the hardware-efficiency doubling period, in the SAME calendar time unit as `elapsed`
    /// (seconds since genesis, from the attested wall-clock — Moore's law is calendar-based, NOT
    /// work-based). It equals `ln 2 / a_estim`; expressing the governable parameter as an integer period
    /// (rather than a fixed-point rate) keeps this exact and float-free.
    ///
    /// **INERT SEAM:** `halflife == 0` ⇒ returns exactly `2^32` (identity multiplier, decay OFF) so issuance
    /// is byte-identical until a nonzero period is governed in — the same default-off discipline as
    /// `pow_enforced`. Total and panic-free: after ~32 doubling periods the multiplier saturates to 0
    /// (issuance-per-hash → 0 over decades), and it never exceeds `2^32` (decay is always ≤ 1).
    pub fn moore_decay_q32(elapsed: u64, halflife: u64) -> u64 {
        const Q32: i128 = 1 << 32;
        if halflife == 0 {
            return Q32 as u64; // inert: no decay, identity multiplier
        }
        // decay · 2^32 = 2^(32 − elapsed/halflife). Build the exponent E in Q16.16 (signed).
        let e_q16: i128 = (32i128 << 16) - (((elapsed as i128) << 16) / halflife as i128);
        let shifts = e_q16 >> 16; // arithmetic floor toward −∞
        let frac = (e_q16 & 0xffff) as u128; // positive remainder in [0, 65536), holds for negatives
        let mut val: i128 = pow2_frac_q16(frac) as i128; // ∈ [2^16, 2^17)
        let net = shifts - 16; // undo the factor's ×2^16
        if net >= 0 {
            // decay ≤ 1 ⇒ 2^E ≤ 2^32; this branch is only reached near elapsed≈0. Cap at 2^32.
            if net >= 64 {
                return Q32 as u64;
            }
            val <<= net;
            if val > Q32 {
                val = Q32;
            }
        } else {
            val >>= (-net).min(127) as u32; // enough doublings ⇒ floors to 0 (fail-closed)
        }
        val as u64
    }

    /// Clamp a target to `[1, pow_limit]` (never easier than the floor, never the unmeetable zero) and
    /// re-encode to a canonical compact.
    fn clamp_and_encode(mut t: [u8; 32], pow_limit: &[u8; 32]) -> Option<u32> {
        if cmp256(&t, pow_limit) == core::cmp::Ordering::Greater {
            t = *pow_limit; // too easy ⇒ pin to the floor (max target)
        }
        if t == [0u8; 32] {
            t[31] = 1; // never the unmeetable zero target — pin to the maximally-hard target of 1
        }
        target_to_compact(&t)
    }

    /// `target · factor` (factor ≤ ~2^17), big-endian 256-bit, saturating to all-ones on overflow (an
    /// overflow means "easier than any representable target" ⇒ the caller's floor-clamp handles it).
    fn mul_small256(target: &[u8; 32], factor: u64) -> [u8; 32] {
        let mut out = [0u8; 32];
        let mut carry: u128 = 0;
        let mut i = 32;
        while i > 0 {
            i -= 1;
            let prod = target[i] as u128 * factor as u128 + carry;
            out[i] = (prod & 0xff) as u8;
            carry = prod >> 8;
        }
        if carry != 0 {
            return [0xffu8; 32]; // overflowed 256 bits ⇒ saturate to the easiest target
        }
        out
    }

    /// One-bit left shift of a big-endian 256-bit value; returns `true` if a set bit fell off the top.
    fn shl1_256(t: &mut [u8; 32]) -> bool {
        let mut carry = 0u16;
        let mut i = 32;
        while i > 0 {
            i -= 1;
            let v = ((t[i] as u16) << 1) | carry;
            t[i] = (v & 0xff) as u8;
            carry = v >> 8;
        }
        carry != 0
    }

    /// One-bit right shift of a big-endian 256-bit value (toward zero).
    fn shr1_256(t: &mut [u8; 32]) {
        let mut carry = 0u8; // the LSB of the higher byte drops into the top of the current byte
        for byte in t.iter_mut() {
            let next = *byte & 1;
            *byte = (*byte >> 1) | (carry << 7);
            carry = next;
        }
    }

    /// `target <<= bits`, saturating to all-ones the moment a set bit would fall off the top.
    fn shl256_sat(target: &mut [u8; 32], bits: u32) {
        for _ in 0..bits.min(256) {
            if shl1_256(target) {
                *target = [0xffu8; 32];
                return;
            }
        }
    }

    /// `target >>= bits`, big-endian 256-bit (toward zero; `bits ≥ 256 ⇒ 0`).
    fn shr256(target: &mut [u8; 32], bits: u32) {
        for _ in 0..bits.min(256) {
            shr1_256(target);
        }
    }
}
