//! Proof-of-Mind value chain — core mechanism (PRIVATE, stealth).
//!
//! Nervos-shaped, serious stack:
//!   - language: Rust
//!   - execution: RISC-V via CKB-VM (Nervos); scripts below are RISC-V programs
//!     referenced by `code_hash`. Added when the VM integration lands.
//!   - state model: CKB-style **Cells** (generalized UTXO) for **shardability** —
//!     cells are independent and self-validating via their scripts, so state
//!     partitions (by cell id / owner) and validates in parallel. No global hotspots.
//!   - **lock script**: ownership — who may consume/transfer a cell (Bitcoin-shaped).
//!   - **type script**: ENCAPSULATES PoM — the state-transition rules (temporal-
//!     novelty value, strategyproofness) that run on every create/consume. PoM
//!     travels *with the cell*, not as a global oracle. That is what keeps it shardable.
//!
//! The Rust functions here are the reference spec the PoM type-script (RISC-V) enforces.

use std::collections::{HashMap, HashSet};

/// A CKB-style script: a RISC-V program (by code hash) + its arguments. VM success
/// = valid. Lock scripts gate ownership; type scripts gate state transitions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Script {
    pub code_hash: [u8; 32],
    pub args: Vec<u8>,
}

/// A Cell = one unit of chain state (generalized UTXO). Independent => shardable.
#[derive(Clone, Debug)]
pub struct Cell {
    pub id: u64,
    /// ownership: args carry the owner pubkey hash; the program authorizes the spend.
    pub lock: Script,
    /// PoM rules: a shared code_hash identifies the canonical PoM type-script program.
    pub type_script: Script,
    /// provenance edge to the cell this built on.
    pub parent: Option<u64>,
    /// commit-reveal order — the ordering that makes value strategyproof.
    pub timestamp: u64,
    /// contribution payload; coverage is derived from it (proxy for reward-model eval).
    pub data: Vec<u8>,
}

pub type CovId = u64;

/// Coverage = set of content shingles of the cell's data (proxy for the learned
/// reward-model evaluation of what the cell contributes).
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

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Temporal-novelty value over cells in COMMIT ORDER: value(cell) = coverage novel
/// vs all earlier-committed cells. Strategyproof by construction — a later sybil /
/// padding / collusion cell adds no NEW coverage, so it earns 0. This is the rule the
/// PoM type script enforces.
pub fn temporal_novelty(cells_in_commit_order: &[Cell]) -> Vec<u64> {
    let mut seen: HashSet<CovId> = HashSet::new();
    let mut out = Vec::with_capacity(cells_in_commit_order.len());
    for c in cells_in_commit_order {
        let cov = coverage(&c.data);
        let novel = cov.iter().filter(|x| !seen.contains(*x)).count() as u64;
        out.push(novel);
        seen.extend(cov);
    }
    out
}

/// PoM score per owner (lock args) = sum of temporal-novelty value of owned cells.
pub fn pom_scores(cells_in_commit_order: &[Cell]) -> HashMap<Vec<u8>, u64> {
    let vals = temporal_novelty(cells_in_commit_order);
    let mut pom: HashMap<Vec<u8>, u64> = HashMap::new();
    for (c, v) in cells_in_commit_order.iter().zip(vals) {
        *pom.entry(c.lock.args.clone()).or_insert(0) += v;
    }
    pom
}

/// Shardability: cells partition by id; no cross-shard state dependency in the cell
/// itself (the only shared object is the committed novelty-set, distributed as a
/// commitment in production).
pub fn shard_of(cell: &Cell, n: u64) -> u64 {
    cell.id % n
}

#[cfg(test)]
mod tests {
    use super::*;

    const POM_TYPE: [u8; 32] = [0xB0; 32]; // placeholder code_hash for the PoM program

    fn cell(id: u64, owner: u8, ts: u64, data: &[u8]) -> Cell {
        Cell {
            id,
            lock: Script { code_hash: [1u8; 32], args: vec![owner] },
            type_script: Script { code_hash: POM_TYPE, args: vec![] },
            parent: None,
            timestamp: ts,
            data: data.to_vec(),
        }
    }

    #[test]
    fn strategyproof_sybil_and_padding_earn_zero() {
        let a = b"alpha-bravo-charlie".to_vec();
        let dup = a.clone(); // sybil clone, committed later
        let pad = b"alph".to_vec(); // strict prefix/subset of a's shingles
        let order = vec![
            cell(0, 1, 0, &a),
            cell(1, 1, 1, b"charlie-delta-echo"), // partly novel
            cell(2, 9, 2, &dup),                  // attacker clone -> 0
            cell(3, 9, 3, &pad),                  // attacker subset -> 0
        ];
        let v = temporal_novelty(&order);
        assert!(v[0] > 0, "honest A novel");
        assert_eq!(v[2], 0, "sybil clone earns 0");
        assert_eq!(v[3], 0, "padding subset earns 0");
    }

    #[test]
    fn pom_sums_per_owner_lock() {
        let order = vec![
            cell(0, 1, 0, b"unique-content-one"),
            cell(1, 2, 1, b"totally-different-two"),
        ];
        let pom = pom_scores(&order);
        assert!(pom[&vec![1u8]] > 0);
        assert!(pom[&vec![2u8]] > 0);
    }

    #[test]
    fn cells_shard_independently() {
        assert_eq!(shard_of(&cell(7, 1, 0, b"x"), 4), 3);
    }
}
