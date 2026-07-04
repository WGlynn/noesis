//! Portable, re-derivable PoM export layer (Tom Lindeman / Pragma idea, 2026-07-03).
//!
//! WHAT THIS IS. The seam-2 "export" side of the reduction/export primitive
//! (`internal/VERIFIABLE-REDUCTION-AND-EXPORT-LAYER.md`): it emits the PoM value
//! output — the result of the deterministic `v(S)` reduction over contributions —
//! in a **portable, re-derivable** form (JSON) so a foreign chain / L2 can consume
//! it and build on it *without trusting this node*.
//!
//! WHAT "VERIFIABLE" MEANS HERE — status discipline. The verifiability we HAVE and
//! use is **deterministic re-derivation**: a consumer re-runs the same deterministic
//! reduction on the same inputs (the cells' data in commit order) and gets the same
//! output; [`verify`] performs exactly that check. This is NOT a ZK proof and NOT a
//! succinct proof of faithful reduction — those are the open frontier (🔬) named in
//! the design note and are deliberately NOT built or claimed here.
//!
//! REUSE, NOT REINVENTION. Per-cell PoM value is produced by the EXISTING consensus
//! reduction [`crate::value_fixed::temporal_novelty_with_similarity_floor_q16`] — the
//! same integer Q16.16 path the runtime consensus attribution routes through
//! (`pom_scores_with_similarity_floor_q16`, `runtime.rs:525`). No scoring is
//! re-implemented in this module; it only serializes and hashes existing outputs.

use crate::{value_fixed, Cell};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tiny_keccak::{Hasher, Keccak};

/// Canonical near-duplicate similarity floor, Q16.16 = floor(0.95 · 2^16). This is the
/// production default carried by the genesis `Constitution` (`runtime.rs:77`). It is
/// recorded in the export so the reduction is fully re-derivable from the JSON alone.
pub const DEFAULT_THETA_SIM_Q16: u64 = 62259;

/// Canonical entropy floor, Q16.16 = floor(0.95 · 2^16). A cell whose data has normalized
/// Shannon entropy >= this is treated as incompressible NOISE and scored 0 (`semantic_floor_q16`).
/// SHIP-BLOCKER guard: without it the export would pay for random bytes in the tree that pays
/// money. This is the accidental/lazy-noise floor; the aware-adversary encoded-noise case is the
/// open learned-quality gate (`lib.rs:6883`), deliberately NOT claimed closed here.
pub const DEFAULT_THETA_ENT_Q16: u64 = 62259;

/// One contribution's portable score: its cell id + commit-order index + the PoM value
/// the deterministic reduction assigned it. `pom_value` is at the integer novelty scale
/// (post similarity-floor), identical to what feeds `pom_scores_with_similarity_floor_q16`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionScore {
    /// Cell id (`Cell::id`) — the stable per-contribution identifier.
    pub id: u64,
    /// Position in the commit-order slice the reduction ran over.
    pub index: usize,
    /// PoM value from the deterministic reduction (similarity-floored temporal novelty).
    pub pom_value: u64,
}

/// The portable PoM output. Serialize to JSON, hand to any consumer; the consumer
/// re-runs [`verify`] against the same cells to confirm faithful production.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomExport {
    /// Similarity-floor parameter the reduction used (needed to re-derive).
    pub theta_sim_q16: u64,
    /// Entropy-floor parameter the reduction used (needed to re-derive). Cells at or above this
    /// normalized entropy are scored 0 as noise (the ship-blocker guard).
    pub theta_ent_q16: u64,
    /// Per-contribution scores, in commit order.
    pub scores: Vec<ContributionScore>,
    /// Aggregate PoM value = sum of all per-contribution values. Equals the sum of the
    /// per-contributor `pom_scores_with_similarity_floor_q16` map (same reduction, regrouped).
    pub total: u64,
    /// blake2b-256 commitment over the CANONICAL INPUTS (each cell's identity + data, in
    /// commit order) plus `theta_sim_q16`. Binds the export to the exact inputs it was
    /// derived from, so a consumer re-hashing the inputs detects any input substitution.
    /// Hex-encoded for JSON portability.
    pub commitment: String,
    /// EVM-portable Merkle root over per-CONTRIBUTOR cumulative value, keccak256 with the
    /// OpenZeppelin `MerkleProof` convention (double-hash leaf + commutative pairs). This is
    /// exactly what the on-chain `PoMExportHub.PomStanding.scoresRoot` carries, so a value-
    /// routing consumer can verify a contributor's score with `verifyContributionScore`.
    /// Hex-encoded (no 0x prefix), matching `commitment`.
    pub scores_root: String,
}

/// blake2b-256 over the canonical inputs in commit order. Domain-separated, length-prefixed
/// so no field concatenation can be ambiguous. Covers everything the reduction reads plus
/// the identity fields a consumer needs to trust attribution: id, type_script.args
/// (soulbound contributor identity), parent, timestamp, and the raw data.
fn commit_inputs(cells: &[Cell], theta_sim_q16: u64, theta_ent_q16: u64) -> [u8; 32] {
    let mut h = blake2b_ref::Blake2bBuilder::new(32)
        .personal(b"noesis-pomexport")
        .build();
    // Bind the parameters and the cardinality first.
    h.update(b"THETA");
    h.update(&theta_sim_q16.to_le_bytes());
    h.update(b"THETAENT");
    h.update(&theta_ent_q16.to_le_bytes());
    h.update(b"NCELLS");
    h.update(&(cells.len() as u64).to_le_bytes());
    for c in cells {
        h.update(b"ID");
        h.update(&c.id.to_le_bytes());
        h.update(b"CONTRIB"); // soulbound contributor identity (type_script.args)
        h.update(&(c.type_script.args.len() as u64).to_le_bytes());
        h.update(&c.type_script.args);
        h.update(b"PARENT");
        // Option<u64> -> 1 tag byte + value (0 when None), unambiguous.
        match c.parent {
            Some(p) => {
                h.update(&[1u8]);
                h.update(&p.to_le_bytes());
            }
            None => {
                h.update(&[0u8]);
                h.update(&0u64.to_le_bytes());
            }
        }
        h.update(b"TS");
        h.update(&c.timestamp.to_le_bytes());
        h.update(b"DATA");
        h.update(&(c.data.len() as u64).to_le_bytes());
        h.update(&c.data);
    }
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

fn to_hex(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

/// Build a [`PomExport`] from cells in commit order, using the canonical floors.
pub fn export(cells: &[Cell]) -> PomExport {
    export_with_thetas(cells, DEFAULT_THETA_SIM_Q16, DEFAULT_THETA_ENT_Q16)
}

/// Convenience: explicit similarity floor, canonical entropy floor.
pub fn export_with_theta(cells: &[Cell], theta_sim_q16: u64) -> PomExport {
    export_with_thetas(cells, theta_sim_q16, DEFAULT_THETA_ENT_Q16)
}

/// The reduction the export PAYS on: similarity-floored temporal novelty, THEN noise-stripped
/// by the entropy floor (ship-blocker guard). Both at RAW integer novelty scale — this is
/// `semantic_floor_q16 ∘ temporal_novelty_with_similarity_floor_q16`, i.e. `production_value_q16`
/// with quality = 0 but WITHOUT its `× (ONE + q)` rescale (which would shift the scale by 2^16).
/// Reuses the consensus value functions (`lib.rs:6506`, `:6496`); it does not reinvent scoring.
fn floored_values(cells: &[Cell], theta_sim_q16: u64, theta_ent_q16: u64) -> Vec<u64> {
    let nv = value_fixed::temporal_novelty_with_similarity_floor_q16(cells, theta_sim_q16);
    cells
        .iter()
        .zip(nv.iter())
        .map(|(c, &n)| value_fixed::semantic_floor_q16(n, &c.data, theta_ent_q16))
        .collect()
}

/// Build a [`PomExport`] with explicit similarity + entropy floors.
pub fn export_with_thetas(cells: &[Cell], theta_sim_q16: u64, theta_ent_q16: u64) -> PomExport {
    let vals = floored_values(cells, theta_sim_q16, theta_ent_q16);
    let scores: Vec<ContributionScore> = cells
        .iter()
        .zip(vals.iter())
        .enumerate()
        .map(|(index, (c, &pom_value))| ContributionScore {
            id: c.id,
            index,
            pom_value,
        })
        .collect();
    let total: u64 = vals.iter().sum();
    let commitment = to_hex(&commit_inputs(cells, theta_sim_q16, theta_ent_q16));
    let scores_root = scores_root_hex(cells, theta_sim_q16, theta_ent_q16);
    PomExport {
        theta_sim_q16,
        theta_ent_q16,
        scores,
        total,
        commitment,
        scores_root,
    }
}

/// Deterministic re-derivation check — THIS is the "as-is" verifiability.
///
/// Re-runs the same reduction on `cells` and confirms the export's per-cell values,
/// index/id mapping, aggregate total, and input commitment all match. A consumer that
/// possesses the cells needs no trust in the producer: an honest export re-derives, a
/// tampered one (any flipped value, reordered/substituted input, or wrong total) does not.
pub fn verify(export: &PomExport, cells: &[Cell]) -> bool {
    let recomputed = export_with_thetas(cells, export.theta_sim_q16, export.theta_ent_q16);
    // Full structural equality: scores (id+index+value), total, commitment, scores_root.
    recomputed == *export
}

// ============ EVM-portable per-contributor Merkle (OZ MerkleProof compatible) ============
//
// A consumer of the on-chain PoMExportHub verifies a contributor's cumulative PoM value
// against `PomStanding.scoresRoot` with a Merkle proof. The scheme MUST match OpenZeppelin
// `MerkleProof.verify`: leaf = keccak256(keccak256(abi.encode(bytes32 contributor,
// uint256 value))), commutative (sorted) pair hashing. These functions produce exactly that,
// so the Rust producer and the Solidity consumer agree byte-for-byte.

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut k = Keccak::v256();
    let mut out = [0u8; 32];
    k.update(bytes);
    k.finalize(&mut out);
    out
}

/// Contributor identity as an EVM bytes32: keccak256 of the soulbound `type_script.args`.
/// Any-length args map to a fixed 32-byte id the contract can index by.
pub fn contributor_id(args: &[u8]) -> [u8; 32] {
    keccak256(args)
}

/// OZ double-hash leaf: keccak256(keccak256(abi.encode(bytes32 contributor, uint256 value))).
/// abi.encode(bytes32, uint256) == contributor[32] || value_be[32].
fn leaf_hash(contributor: &[u8; 32], value: u64) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(contributor);
    buf[56..64].copy_from_slice(&value.to_be_bytes()); // uint256 big-endian; low 8 bytes
    keccak256(&keccak256(&buf))
}

/// Commutative pair hash — matches OZ `MerkleProof` (sorted siblings).
fn hash_pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 64];
    if a <= b {
        buf[..32].copy_from_slice(a);
        buf[32..].copy_from_slice(b);
    } else {
        buf[..32].copy_from_slice(b);
        buf[32..].copy_from_slice(a);
    }
    keccak256(&buf)
}

fn build_levels(leaves: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
    let mut levels = vec![leaves.to_vec()];
    while levels.last().unwrap().len() > 1 {
        let cur = levels.last().unwrap();
        let mut next = Vec::with_capacity((cur.len() + 1) / 2);
        let mut i = 0;
        while i < cur.len() {
            if i + 1 < cur.len() {
                next.push(hash_pair(&cur[i], &cur[i + 1]));
                i += 2;
            } else {
                next.push(cur[i]); // odd node promoted unchanged (merkletreejs semantics)
                i += 1;
            }
        }
        levels.push(next);
    }
    levels
}

/// Regroup the per-cell reduction output into per-CONTRIBUTOR cumulative value, keyed by the
/// soulbound contributor id and sorted by id (deterministic). Reuses the EXISTING reduction
/// (`temporal_novelty_with_similarity_floor_q16`); it does not reinvent scoring.
pub fn per_contributor(
    cells: &[Cell],
    theta_sim_q16: u64,
    theta_ent_q16: u64,
) -> Vec<([u8; 32], u64)> {
    let vals = floored_values(cells, theta_sim_q16, theta_ent_q16);
    let mut map: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    for (c, &v) in cells.iter().zip(vals.iter()) {
        *map.entry(contributor_id(&c.type_script.args)).or_insert(0) += v;
    }
    map.into_iter().collect()
}

/// Merkle root over per-contributor `(contributor, cumulativeValue)` leaves. Empty -> zero.
pub fn merkle_root(pairs: &[([u8; 32], u64)]) -> [u8; 32] {
    if pairs.is_empty() {
        return [0u8; 32];
    }
    let leaves: Vec<[u8; 32]> = pairs.iter().map(|(c, v)| leaf_hash(c, *v)).collect();
    *build_levels(&leaves).last().unwrap().first().unwrap()
}

/// Merkle proof (sibling path) for the leaf at `index`, consumed by the on-chain verify.
pub fn merkle_proof(pairs: &[([u8; 32], u64)], index: usize) -> Vec<[u8; 32]> {
    let leaves: Vec<[u8; 32]> = pairs.iter().map(|(c, v)| leaf_hash(c, *v)).collect();
    let levels = build_levels(&leaves);
    let mut proof = Vec::new();
    let mut idx = index;
    for level in &levels[..levels.len().saturating_sub(1)] {
        let sib = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
        if sib < level.len() {
            proof.push(level[sib]);
        }
        idx /= 2;
    }
    proof
}

/// The scores root the on-chain `PomStanding.scoresRoot` carries, hex-encoded (no 0x).
pub fn scores_root_hex(cells: &[Cell], theta_sim_q16: u64, theta_ent_q16: u64) -> String {
    to_hex(&merkle_root(&per_contributor(cells, theta_sim_q16, theta_ent_q16)))
}

// ============ Delta-priced meta-block payout tree (MindCoin subsidy routing) ============
//
// `scores_root` above carries LIFETIME cumulative value (what a consumer reads). The MINT, by
// contrast, is delta-priced: each finalized meta-block pays only for the NEW information in it
// (paying pro-rata against lifetime score would pay perpetual rent to old contributions, breaking
// the value-router soul). This section builds the `PomStanding.payoutRoot` the on-chain
// `PoMExportHub.claimContributorReward` verifies:
//     leaf = keccak256(keccak256(abi.encode(bytes32 contributor, address payTo, uint256 amount)))
// and mirrors the on-chain Bitcoin-form schedule so a proposer's payout root and a challenger's
// re-derivation agree byte-for-byte. This is the last cross-language tie of the export layer.

/// Bitcoin-form subsidy at meta-block 0, in wei (3.125 MIND). Mirrors `PoMExportHub.INITIAL_SUBSIDY`.
pub const INITIAL_SUBSIDY_WEI: u128 = 3_125_000_000_000_000_000;
/// Halving interval in meta-blocks. Mirrors `PoMExportHub.HALVING_INTERVAL`.
pub const HALVING_INTERVAL: u64 = 210_000;
/// Proposer cut / security tranche, bps. Mirror `PoMExportHub` `proposerCutBps` / `trancheBps` defaults.
pub const PROPOSER_CUT_BPS: u128 = 600;
pub const TRANCHE_BPS: u128 = 300;

/// Bitcoin-form meta-block subsidy in wei. Mirrors `PoMExportHub.metaBlockSubsidy`: 3.125 MIND,
/// halving every 210,000 meta-blocks, 0 at epoch >= 64.
pub fn meta_block_subsidy_wei(nonce: u64) -> u128 {
    let epoch = nonce / HALVING_INTERVAL;
    if epoch >= 64 {
        return 0;
    }
    INITIAL_SUBSIDY_WEI >> epoch
}

/// The 91% contributor pool for a meta-block, in wei. Mirrors `PoMExportHub._finalize` EXACTLY:
/// `pool = subsidy - floor(subsidy*600/10000) - floor(subsidy*300/10000)`, so the division
/// remainders accrue to the pool. Computing `floor(subsidy*9100/10000)` instead could exceed the
/// on-chain `blockPool` by a wei and make an honest claim revert with `ClaimExceedsPool`.
/// (Terminal-edge caveat: on-chain the subsidy is also clamped to `MAX_SUPPLY - emissionCommitted`.
/// This helper ignores that clamp, which only bites in the final wei of the 1,312,500 cap — far
/// beyond v1; a proposer near the cap must clamp to on-chain headroom.)
pub fn meta_block_pool_wei(nonce: u64) -> u128 {
    let subsidy = meta_block_subsidy_wei(nonce);
    let proposer_cut = subsidy * PROPOSER_CUT_BPS / 10_000;
    let tranche = subsidy * TRANCHE_BPS / 10_000;
    subsidy - proposer_cut - tranche
}

/// One delta-priced payout. The on-chain claim leaf is
/// `keccak256(keccak256(abi.encode(bytes32 contributor, address payTo, uint256 amount)))`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayoutEntry {
    /// Soulbound contributor id (`contributor_id(type_script.args)`), the on-chain claim key.
    pub contributor: [u8; 32],
    /// Registered payout address the reward mints to.
    pub pay_to: [u8; 20],
    /// This meta-block's share, in wei of MIND.
    pub amount: u128,
}

/// OZ double-hash leaf over `abi.encode(bytes32 contributor, address payTo, uint256 amount)`.
/// abi.encode packs each argument to a 32-byte word: contributor[32] || (12 zero || payTo[20]) ||
/// (16 zero || amount_u128_be[16]). The amount word is a full uint256 on the Solidity side; here it
/// is a u128 written into the low 16 bytes with the high 16 zero, which is byte-identical for any
/// value < 2^128 (all reachable amounts). Matches `PoMExportHub.claimContributorReward`'s leaf.
fn payout_leaf_hash(contributor: &[u8; 32], pay_to: &[u8; 20], amount: u128) -> [u8; 32] {
    let mut buf = [0u8; 96];
    buf[..32].copy_from_slice(contributor);
    buf[44..64].copy_from_slice(pay_to); // address right-aligned in its 32-byte word
    buf[80..96].copy_from_slice(&amount.to_be_bytes()); // uint256 big-endian; u128 in the low 16 bytes
    keccak256(&keccak256(&buf))
}

/// Compute one meta-block's delta-priced payout entries.
///
/// `prev` / `curr` are per-CONTRIBUTOR CUMULATIVE PoM value (as [`per_contributor`] returns) BEFORE
/// and AFTER this meta-block; `pool` is the 91% wei pool ([`meta_block_pool_wei`]); `registrations`
/// maps a contributor id to its payout address. Each contributor's DELTA (new information this block)
/// is `curr - prev`, clamped at 0 (a theta change could shrink a cumulative; v1 pins theta so it
/// never does). The pool splits pro-rata by delta with FLOOR division, so the amounts always sum to
/// `<= pool` — the on-chain `blockClaimed <= blockPool` guard therefore never trips on an honest
/// root, and the floor dust is simply never minted (the lost-coins analog). Only REGISTERED
/// contributors with a positive amount get a leaf; an unregistered contributor's share is left
/// unminted (the v1 "accrue-unclaimable-until-registered" default) yet its delta STILL counts in the
/// denominator, so no rival is over-paid. Deterministic: sorted by contributor id.
pub fn payout_entries(
    prev: &[([u8; 32], u64)],
    curr: &[([u8; 32], u64)],
    registrations: &BTreeMap<[u8; 32], [u8; 20]>,
    pool: u128,
) -> Vec<PayoutEntry> {
    let prev_map: BTreeMap<[u8; 32], u64> = prev.iter().copied().collect();
    let mut total_delta: u128 = 0;
    for (id, cum) in curr {
        let p = prev_map.get(id).copied().unwrap_or(0);
        total_delta += u128::from(cum.saturating_sub(p));
    }
    if total_delta == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (id, cum) in curr {
        let p = prev_map.get(id).copied().unwrap_or(0);
        let delta = u128::from(cum.saturating_sub(p));
        if delta == 0 {
            continue;
        }
        if let Some(pay_to) = registrations.get(id) {
            let amount = pool * delta / total_delta; // floor; sum over i <= pool
            if amount > 0 {
                out.push(PayoutEntry { contributor: *id, pay_to: *pay_to, amount });
            }
        }
    }
    out.sort_by(|a, b| a.contributor.cmp(&b.contributor));
    out
}

/// Merkle root over payout entries (OZ `MerkleProof`-compatible). Empty -> zero.
pub fn payout_root(entries: &[PayoutEntry]) -> [u8; 32] {
    if entries.is_empty() {
        return [0u8; 32];
    }
    let leaves: Vec<[u8; 32]> = entries
        .iter()
        .map(|e| payout_leaf_hash(&e.contributor, &e.pay_to, e.amount))
        .collect();
    *build_levels(&leaves).last().unwrap().first().unwrap()
}

/// Merkle proof (sibling path) for the payout entry at `index`, consumed on-chain by
/// `claimContributorReward`. Entries must be in the same order [`payout_entries`] returns.
pub fn payout_proof(entries: &[PayoutEntry], index: usize) -> Vec<[u8; 32]> {
    let leaves: Vec<[u8; 32]> = entries
        .iter()
        .map(|e| payout_leaf_hash(&e.contributor, &e.pay_to, e.amount))
        .collect();
    let levels = build_levels(&leaves);
    let mut proof = Vec::new();
    let mut idx = index;
    for level in &levels[..levels.len().saturating_sub(1)] {
        let sib = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
        if sib < level.len() {
            proof.push(level[sib]);
        }
        idx /= 2;
    }
    proof
}

/// Hex of [`payout_root`] (no 0x), matching the `commitment` / `scores_root` encoding. This is the
/// value the proposer puts in `PomStanding.payoutRoot`.
pub fn payout_root_hex(entries: &[PayoutEntry]) -> String {
    to_hex(&payout_root(entries))
}

/// Re-derive the payout entries and confirm a claimed set matches — the payout-side analog of
/// [`verify`]. A challenger holding the prev+curr per-contributor snapshots, the pool, and the
/// registration map re-runs this; any inflated amount, reassigned `payTo`, or injected leaf fails
/// it. Kept separate from [`verify`] because a payout is a function of CROSS-block state (a delta
/// plus registrations plus the schedule), which a single-snapshot [`PomExport`] does not carry.
pub fn verify_payout(
    claimed: &[PayoutEntry],
    prev: &[([u8; 32], u64)],
    curr: &[([u8; 32], u64)],
    registrations: &BTreeMap<[u8; 32], [u8; 20]>,
    pool: u128,
) -> bool {
    payout_entries(prev, curr, registrations, pool) == *claimed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Script;

    const POM_TYPE: [u8; 32] = [0xB0; 32];

    // Mirror the crate test helper (`lib.rs:699`): contributor == owner at mint.
    fn cell(id: u64, owner: u8, ts: u64, data: &[u8]) -> Cell {
        Cell {
            id,
            lock: Script { code_hash: [1u8; 32], args: vec![owner] },
            type_script: Script { code_hash: POM_TYPE, args: vec![owner] },
            parent: None,
            timestamp: ts,
            data: data.to_vec(),
        }
    }

    // A commit-order slice with an honest novel cell, a partly-novel cell, and a sybil
    // clone that must score 0 under the existing reduction — so `total` is load-bearing.
    fn sample_cells() -> Vec<Cell> {
        let a = b"alpha-bravo-charlie-delta-echo".to_vec();
        vec![
            cell(0, 1, 0, &a),
            cell(1, 2, 1, b"golf-hotel-india-juliet-kilo"),
            cell(2, 9, 2, &a.clone()), // sybil clone -> 0 novelty
        ]
    }

    #[test]
    fn export_is_deterministic() {
        let cells = sample_cells();
        let e1 = export(&cells);
        let e2 = export(&cells);
        assert_eq!(e1, e2, "export must be a pure function of the cells");
        // Sanity: the reduction actually ran (honest cells scored, sybil clone did not).
        assert_eq!(e1.scores.len(), 3);
        assert!(e1.scores[0].pom_value > 0, "honest cell 0 novel");
        assert!(e1.scores[1].pom_value > 0, "honest cell 1 novel");
        assert_eq!(e1.scores[2].pom_value, 0, "sybil clone earns 0");
        assert_eq!(
            e1.total,
            e1.scores.iter().map(|s| s.pom_value).sum::<u64>(),
            "total is the aggregate of per-cell values"
        );
    }

    #[test]
    fn verify_accepts_honest_export() {
        let cells = sample_cells();
        let e = export(&cells);
        assert!(verify(&e, &cells), "an honestly produced export re-derives");
    }

    #[test]
    fn verify_rejects_tampered_value() {
        let cells = sample_cells();
        let mut e = export(&cells);
        // Inflate the sybil clone's score — the classic lie the consumer must catch.
        e.scores[2].pom_value += 1;
        e.total += 1; // even a self-consistent forge must fail: re-derivation disagrees.
        assert!(!verify(&e, &cells), "a flipped value must be rejected");
    }

    #[test]
    fn verify_rejects_tampered_commitment() {
        let cells = sample_cells();
        let mut e = export(&cells);
        // Swap the input-binding commitment; the re-hash of the real inputs won't match.
        e.commitment = "00".repeat(32);
        assert!(!verify(&e, &cells), "a broken input commitment must be rejected");
    }

    #[test]
    fn json_round_trips_and_reverifies() {
        let cells = sample_cells();
        let e = export(&cells);
        let json = serde_json::to_string(&e).expect("serialize");
        let back: PomExport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back, "JSON round-trip preserves the export");
        assert!(
            verify(&back, &cells),
            "the deserialized export still re-derives against the cells"
        );
    }

    #[test]
    fn ship_blocker_entropy_floor_zeros_noise() {
        // A near-uniform byte payload (all 256 values once) is incompressible: normalized
        // entropy = 1.0 >= theta_ent, so the entropy floor scores it 0 even though every
        // shingle is novel. WITHOUT this floor it would earn maximal reward in the tree that
        // pays money — this is the ship blocker the design caught.
        let noise: Vec<u8> = (0u8..=255).collect();
        let e = export(&[cell(0, 1, 0, &noise)]);
        assert_eq!(e.scores[0].pom_value, 0, "incompressible noise earns 0");
        assert_eq!(e.total, 0, "a noise-only export pays nothing");
        // Sanity: real (compressible) contribution still earns.
        assert!(export(&sample_cells()).total > 0, "compressible content still earns");
    }

    // ---- EVM-portable Merkle conformance ----

    /// Pinned root for the shared cross-language vector below. The Solidity
    /// PoMExportHub test asserts the SAME root over the SAME vector, so a change to the
    /// leaf/pair scheme on either side breaks both here and there.
    const POM_MERKLE_VECTOR_ROOT: &str =
        "daf99dca546152568a24c92ce244b1cdc50a8d893b491485e740811609d38bc0";

    /// Fold a leaf + proof to a root the OZ `MerkleProof.verify` way (commutative).
    fn oz_verify(mut node: [u8; 32], proof: &[[u8; 32]], root: [u8; 32]) -> bool {
        for p in proof {
            node = hash_pair(&node, p);
        }
        node == root
    }

    #[test]
    fn per_contributor_regroups_and_sums() {
        let cells = sample_cells();
        let pc = per_contributor(&cells, DEFAULT_THETA_SIM_Q16, DEFAULT_THETA_ENT_Q16);
        assert_eq!(pc.len(), 3, "owners 1, 2, 9 = three contributors");
        let get = |owner: u8| -> u64 {
            let id = contributor_id(&[owner]);
            pc.iter().find(|(c, _)| *c == id).map(|(_, v)| *v).unwrap()
        };
        assert!(get(1) > 0, "contributor 1 earned");
        assert!(get(2) > 0, "contributor 2 earned");
        assert_eq!(get(9), 0, "sybil-clone contributor earns 0");

        let r1 = scores_root_hex(&cells, DEFAULT_THETA_SIM_Q16, DEFAULT_THETA_ENT_Q16);
        let r2 = scores_root_hex(&cells, DEFAULT_THETA_SIM_Q16, DEFAULT_THETA_ENT_Q16);
        assert_eq!(r1, r2, "scores root is deterministic");
        assert_ne!(r1, "0".repeat(64), "non-empty tree has a non-zero root");
    }

    #[test]
    fn export_carries_scores_root() {
        let cells = sample_cells();
        let e = export(&cells);
        assert_eq!(e.scores_root.len(), 64, "hex-encoded 32-byte root");
        assert_eq!(
            e.scores_root,
            scores_root_hex(&cells, DEFAULT_THETA_SIM_Q16, DEFAULT_THETA_ENT_Q16)
        );
    }

    // ---- Delta-priced payout tree (MindCoin subsidy routing) ----

    /// Pinned payout root for the cross-language vector in `payout_conformance_vector`. The Solidity
    /// PoMExportHub test (`test_PayoutRoot_MatchesRustVector`) asserts the SAME root over the SAME
    /// (contributor, payTo, amount) leaves — either side drifting the leaf/pair scheme breaks both.
    const PAYOUT_VECTOR_ROOT: &str =
        "c6abf3071c75118de31c207fec9f98a7198f97403165a0b45dd20b99b315536e";

    #[test]
    fn meta_block_pool_mirrors_schedule() {
        // 3.125 MIND at genesis; 91% pool via remainder-routed split (matches the Solidity constants).
        assert_eq!(meta_block_subsidy_wei(0), 3_125_000_000_000_000_000);
        assert_eq!(meta_block_subsidy_wei(HALVING_INTERVAL), 3_125_000_000_000_000_000 / 2);
        assert_eq!(meta_block_subsidy_wei(62 * HALVING_INTERVAL), 0);
        assert_eq!(meta_block_subsidy_wei(64 * HALVING_INTERVAL), 0);
        // pool = subsidy - 6% - 3% (each floored). At genesis the subsidy divides evenly -> exactly 91%.
        assert_eq!(meta_block_pool_wei(0), 2_843_750_000_000_000_000);
        assert_eq!(meta_block_pool_wei(0), 3_125_000_000_000_000_000 * 91 / 100);
    }

    #[test]
    fn payout_conformance_vector() {
        let alice = contributor_id(b"alice");
        let bob = contributor_id(b"bob");
        let mut pay_a = [0u8; 20];
        pay_a[19] = 0xA1; // address(uint160(0xA1))
        let mut pay_b = [0u8; 20];
        pay_b[19] = 0xB2; // address(uint160(0xB2))

        let prev: Vec<([u8; 32], u64)> = vec![]; // genesis: nothing before block 0
        let curr = vec![(alice, 1000u64), (bob, 500u64)];
        let mut regs: BTreeMap<[u8; 32], [u8; 20]> = BTreeMap::new();
        regs.insert(alice, pay_a);
        regs.insert(bob, pay_b);

        let pool = meta_block_pool_wei(0);
        assert_eq!(pool, 2_843_750_000_000_000_000u128);

        let entries = payout_entries(&prev, &curr, &regs, pool);
        assert_eq!(entries.len(), 2, "two registered contributors");

        // Amounts: floor(pool * delta / total_delta), total_delta = 1500.
        let amt = |id: [u8; 32]| entries.iter().find(|e| e.contributor == id).unwrap().amount;
        assert_eq!(amt(alice), 1_895_833_333_333_333_333u128, "alice = pool*1000/1500 floor");
        assert_eq!(amt(bob), 947_916_666_666_666_666u128, "bob = pool*500/1500 floor");
        let sum: u128 = entries.iter().map(|e| e.amount).sum();
        assert!(sum <= pool, "solvent by construction: payouts never exceed the pool");

        let root = payout_root(&entries);
        // Self-consistent + OZ-style: every leaf's proof folds back to the root.
        for i in 0..entries.len() {
            let leaf = payout_leaf_hash(&entries[i].contributor, &entries[i].pay_to, entries[i].amount);
            let proof = payout_proof(&entries, i);
            assert!(oz_verify(leaf, &proof, root), "payout leaf {i} proof must verify");
        }
        // Cross-language pin.
        assert_eq!(to_hex(&root), PAYOUT_VECTOR_ROOT, "Rust payout root must equal the pinned Solidity vector");
    }

    #[test]
    fn payout_verify_accepts_honest_rejects_tampered() {
        let alice = contributor_id(b"alice");
        let bob = contributor_id(b"bob");
        let mut pay_a = [0u8; 20];
        pay_a[19] = 0xA1;
        let mut pay_b = [0u8; 20];
        pay_b[19] = 0xB2;
        let prev: Vec<([u8; 32], u64)> = vec![];
        let curr = vec![(alice, 1000u64), (bob, 500u64)];
        let mut regs: BTreeMap<[u8; 32], [u8; 20]> = BTreeMap::new();
        regs.insert(alice, pay_a);
        regs.insert(bob, pay_b);
        let pool = meta_block_pool_wei(0);

        let honest = payout_entries(&prev, &curr, &regs, pool);
        assert!(verify_payout(&honest, &prev, &curr, &regs, pool), "honest payout re-derives");

        // Inflate one amount -> re-derivation disagrees.
        let mut inflated = honest.clone();
        inflated[0].amount += 1;
        assert!(!verify_payout(&inflated, &prev, &curr, &regs, pool), "inflated amount rejected");

        // Reassign a payTo (theft) -> rejected.
        let mut stolen = honest.clone();
        stolen[0].pay_to[19] = 0xFF;
        assert!(!verify_payout(&stolen, &prev, &curr, &regs, pool), "reassigned payTo rejected");
    }

    #[test]
    fn payout_is_delta_priced_and_dilutes_unregistered() {
        let alice = contributor_id(b"alice");
        let bob = contributor_id(b"bob");
        let carol = contributor_id(b"carol"); // never registers -> accrue-unclaimable
        let mut pay_a = [0u8; 20];
        pay_a[19] = 0xA1;
        let mut regs: BTreeMap<[u8; 32], [u8; 20]> = BTreeMap::new();
        regs.insert(alice, pay_a); // only alice registered

        // Block 1 delta: alice +1000, carol +1000 (bob unchanged). Only alice gets a leaf; carol's
        // delta still dilutes the denominator (its share is unminted, not handed to alice).
        let prev = vec![(alice, 500u64), (bob, 500u64)];
        let curr = vec![(alice, 1500u64), (bob, 500u64), (carol, 1000u64)];
        let pool = 3000u128;
        let entries = payout_entries(&prev, &curr, &regs, pool);
        assert_eq!(entries.len(), 1, "only the registered contributor is paid");
        assert_eq!(entries[0].contributor, alice);
        // total_delta = 1000 (alice) + 0 (bob) + 1000 (carol) = 2000; alice = pool*1000/2000 = 1500.
        assert_eq!(entries[0].amount, 1500u128, "delta-priced; carol's 1000 dilutes but is not paid to alice");

        // Pays for NEW info only: a block that adds no delta for alice pays her nothing.
        let entries2 = payout_entries(&curr, &curr, &regs, pool);
        assert!(entries2.is_empty(), "no new information => no payout");
    }

    #[test]
    fn merkle_conformance_vector() {
        // Same vector the Solidity test uses: contributor = keccak256(name), plus a value.
        let pairs = vec![
            (contributor_id(b"alice"), 1000u64),
            (contributor_id(b"bob"), 500u64),
        ];
        let root = merkle_root(&pairs);

        // Self-consistent + OZ-style: every leaf's proof folds back to the root.
        for i in 0..pairs.len() {
            let leaf = leaf_hash(&pairs[i].0, pairs[i].1);
            let proof = merkle_proof(&pairs, i);
            assert!(oz_verify(leaf, &proof, root), "leaf {i} proof must verify");
        }

        // Cross-language pin.
        assert_eq!(
            to_hex(&root),
            POM_MERKLE_VECTOR_ROOT,
            "Rust merkle root must equal the pinned Solidity vector root"
        );
    }
}
