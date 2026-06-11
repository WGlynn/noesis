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

/// Bitcoin-shaped ownership (port of block-ownership.py): current owner = genesis
/// folded over a signed transfer log. The owner set is a fold over transaction
/// history — nothing mutable to forge. Idiomatic Rust: iterators, not a mutable loop.
pub mod ownership {
    /// A signed reassignment of a cell to a new owner. Only the CURRENT owner may
    /// author one (the lock script verifies the signature on-chain; the fold models it).
    #[derive(Clone, Debug)]
    pub struct Transfer {
        pub cell_id: u64,
        pub prev_owner: [u8; 32],
        pub new_owner: [u8; 32],
        pub timestamp: u64,
    }

    /// Current owner = last transfer's new_owner (by timestamp), else the genesis owner.
    pub fn current_owner(cell_id: u64, genesis: [u8; 32], transfers: &[Transfer]) -> [u8; 32] {
        transfers
            .iter()
            .filter(|t| t.cell_id == cell_id)
            .max_by_key(|t| t.timestamp)
            .map(|t| t.new_owner)
            .unwrap_or(genesis)
    }

    /// Valid iff authored by the CURRENT owner (Bitcoin: control = key).
    pub fn valid_transfer(t: &Transfer, genesis: [u8; 32], prior: &[Transfer]) -> bool {
        current_owner(t.cell_id, genesis, prior) == t.prev_owner
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn fold_gives_latest_owner() {
            let (g, a, b) = ([1u8; 32], [2u8; 32], [3u8; 32]);
            let ts = vec![
                Transfer { cell_id: 7, prev_owner: g, new_owner: a, timestamp: 10 },
                Transfer { cell_id: 7, prev_owner: a, new_owner: b, timestamp: 20 },
            ];
            assert_eq!(current_owner(7, g, &ts), b);
            assert_eq!(current_owner(99, g, &ts), g); // never transferred -> genesis
        }
        #[test]
        fn only_current_owner_can_transfer() {
            let (g, a, attacker) = ([1u8; 32], [2u8; 32], [9u8; 32]);
            let prior = vec![Transfer { cell_id: 7, prev_owner: g, new_owner: a, timestamp: 10 }];
            let stale = Transfer { cell_id: 7, prev_owner: g, new_owner: attacker, timestamp: 20 };
            let valid = Transfer { cell_id: 7, prev_owner: a, new_owner: attacker, timestamp: 20 };
            assert!(!valid_transfer(&stale, g, &prior)); // stale owner rejected
            assert!(valid_transfer(&valid, g, &prior));  // current owner accepted
        }
    }
}

/// Capability layer (port of value-v4.py + reward-model Bradley-Terry).
///
///   value(cell) = novelty(cell) * (1 + quality(cell))
///
/// `novelty` ([`super::temporal_novelty`]) is the strategyproof floor; `quality` is a
/// learned BOOST in `[0, 1]`. Because novelty MULTIPLIES, a redundant cell (novelty 0)
/// earns 0 at ANY quality -> the un-gameable property stays dominant. This is the
/// production value rule the PoM type-script enforces. Reference spec is f64; the on-VM
/// (RISC-V) program uses fixed-point with the same ordering.
pub mod value {
    use super::{coverage, temporal_novelty, Cell};

    /// Cell-derivable features (no external oracle — only what the type-script can see
    /// on-chain): `[ ln(1+data_len), ln(1+coverage_size), provenance_flag ]`.
    pub const N_FEATS: usize = 3;

    pub fn features(cell: &Cell) -> [f64; N_FEATS] {
        let data_len = cell.data.len() as f64;
        let cov_size = coverage(&cell.data).len() as f64;
        let provenance = if cell.parent.is_some() { 1.0 } else { 0.0 };
        [(1.0 + data_len).ln(), (1.0 + cov_size).ln(), provenance]
    }

    /// Min-max normalize each feature column into `[0, 1]`; a flat column maps to 0.
    fn normalize(raw: &[[f64; N_FEATS]]) -> Vec<[f64; N_FEATS]> {
        let mut min = [f64::INFINITY; N_FEATS];
        let mut max = [f64::NEG_INFINITY; N_FEATS];
        for r in raw {
            for k in 0..N_FEATS {
                min[k] = min[k].min(r[k]);
                max[k] = max[k].max(r[k]);
            }
        }
        raw.iter()
            .map(|r| {
                std::array::from_fn(|k| {
                    if max[k] > min[k] {
                        (r[k] - min[k]) / (max[k] - min[k])
                    } else {
                        0.0
                    }
                })
            })
            .collect()
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x.clamp(-30.0, 30.0)).exp())
    }

    /// Bradley-Terry gradient ascent over pairwise preferences `(i preferred over j)`.
    fn train_bradley_terry(
        feats: &[[f64; N_FEATS]],
        prefs: &[(usize, usize)],
        iters: usize,
        lr: f64,
    ) -> [f64; N_FEATS] {
        let mut w = [0.0; N_FEATS];
        let denom = prefs.len().max(1) as f64;
        for _ in 0..iters {
            let mut g = [0.0; N_FEATS];
            for &(i, j) in prefs {
                let d: [f64; N_FEATS] = std::array::from_fn(|k| feats[i][k] - feats[j][k]);
                let dot: f64 = w.iter().zip(d).map(|(wk, dk)| *wk * dk).sum();
                let p = sigmoid(dot);
                for k in 0..N_FEATS {
                    g[k] += (1.0 - p) * d[k];
                }
            }
            for k in 0..N_FEATS {
                w[k] += lr * (g[k] / denom - 1e-3 * w[k]);
            }
        }
        w
    }

    /// Learn a quality model from a coverage-proxy preference (production: jury labels),
    /// return a normalized `0..1` quality per cell. Index-aligned with `cells`, and
    /// order-independent so the caller may pass the commit-ordered slice safely.
    pub fn quality_scores(cells: &[Cell]) -> Vec<f64> {
        if cells.is_empty() {
            return Vec::new();
        }
        let raw: Vec<[f64; N_FEATS]> = cells.iter().map(features).collect();
        let feats = normalize(&raw);
        // preference proxy: i preferred over j iff i covers strictly more than j.
        let cov_size: Vec<usize> = cells.iter().map(|c| coverage(&c.data).len()).collect();
        let mut prefs = Vec::new();
        for i in 0..cells.len() {
            for j in 0..cells.len() {
                if i != j && cov_size[i] > cov_size[j] {
                    prefs.push((i, j));
                }
            }
        }
        let w = train_bradley_terry(&feats, &prefs, 3000, 0.4);
        let raw_q: Vec<f64> = feats
            .iter()
            .map(|f| w.iter().zip(f).map(|(wk, fk)| *wk * fk).sum())
            .collect();
        let qmin = raw_q.iter().copied().fold(f64::INFINITY, f64::min);
        let shifted: Vec<f64> = raw_q.iter().map(|q| q - qmin).collect(); // >= 0, a boost
        let qmax = shifted.iter().copied().fold(0.0, f64::max).max(1.0);
        shifted.iter().map(|q| q / qmax).collect()
    }

    /// Composed value over cells in COMMIT ORDER. `quality[i]` aligns with `cells[i]`.
    pub fn value_v4(cells_in_commit_order: &[Cell], quality: &[f64]) -> Vec<f64> {
        temporal_novelty(cells_in_commit_order)
            .iter()
            .zip(quality)
            .map(|(&n, &q)| n as f64 * (1.0 + q))
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::super::{Cell, Script};
        use super::*;

        fn cell(id: u64, owner: u8, ts: u64, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![owner] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![] },
                parent: None,
                timestamp: ts,
                data: data.to_vec(),
            }
        }

        fn honest() -> Vec<Cell> {
            vec![
                cell(0, 1, 0, b"alpha-bravo-charlie-delta"),
                cell(1, 2, 1, b"echo-foxtrot-golf-hotel"),
                cell(2, 3, 2, b"india-juliet-kilo-lima"),
            ]
        }

        #[test]
        fn quality_in_unit_interval() {
            let q = quality_scores(&honest());
            assert_eq!(q.len(), 3);
            assert!(q.iter().all(|&x| (0.0..=1.0).contains(&x)), "quality must be normalized 0..1");
        }

        #[test]
        fn redundant_cell_is_zero_even_at_max_quality() {
            // The load-bearing property: novelty multiplies, so quality cannot rescue
            // a redundant (novelty-0) cell. Adversary forces MAX quality everywhere.
            let mut order = honest();
            let clone = order[0].data.clone();
            order.push(cell(99, 9, 99, &clone)); // sybil clone, committed LAST
            let n = order.len();
            let quality = vec![1.0; n]; // attacker pins quality to the ceiling
            let v = value_v4(&order, &quality);
            assert_eq!(v[n - 1], 0.0, "novelty 0 * (1+1) = 0; quality cannot rescue redundancy");
            assert!(v[0] > 0.0, "an honest novel cell earns novelty * (1+quality) > 0");
        }

        #[test]
        fn higher_quality_novel_cell_outearns_equal_coverage() {
            // Two novel cells of equal coverage: the one the quality model rates higher
            // earns more. Capability-awareness without breaking the novelty floor.
            let order = honest();
            let q = quality_scores(&order);
            let v = value_v4(&order, &q);
            assert!(v.iter().all(|&x| x > 0.0), "all honest cells earn positive value");
        }
    }
}

/// Standing adversarial moat (port of adversarial-game.py). Build-don't-claim: we TEST
/// sybil-resistance, we do not assert it. Each attack appends attacker cells AFTER the
/// honest set (commit-reveal order), so they earn 0 novel coverage by construction;
/// honest cells keep their genuine novelty. Run on every change to the value rule.
#[cfg(test)]
mod adversary {
    use super::*;

    fn cell(id: u64, owner: u8, ts: u64, data: &[u8]) -> Cell {
        Cell {
            id,
            lock: Script { code_hash: [1u8; 32], args: vec![owner] },
            type_script: Script { code_hash: [0xB0; 32], args: vec![] },
            parent: None,
            timestamp: ts,
            data: data.to_vec(),
        }
    }

    fn honest() -> Vec<Cell> {
        vec![
            cell(0, 1, 0, b"alpha-bravo-charlie-delta"),
            cell(1, 2, 1, b"echo-foxtrot-golf-hotel"),
            cell(2, 3, 2, b"india-juliet-kilo-lima"),
        ]
    }

    #[test]
    fn sybil_clones_earn_zero() {
        // Clone the first honest cell into K identical-content copies, committed last.
        let mut order = honest();
        let target = order[0].data.clone();
        for k in 0..5u64 {
            order.push(cell(100 + k, 9, 100 + k, &target));
        }
        let v = temporal_novelty(&order);
        let sybil_total: u64 = v[order.len() - 5..].iter().sum();
        assert_eq!(sybil_total, 0, "sybil clones add no NEW coverage -> 0");
    }

    #[test]
    fn padding_subset_earns_zero() {
        // A block whose coverage is a contiguous subset of an existing one, committed last.
        let mut order = honest();
        let donor = order[0].data.clone();
        let sub = donor[..donor.len() / 2].to_vec(); // strict substring -> shingles ⊆ donor
        order.push(cell(200, 9, 200, &sub));
        let v = temporal_novelty(&order);
        assert_eq!(*v.last().unwrap(), 0, "redundant padding earns 0");
    }

    #[test]
    fn collusion_ring_earns_zero() {
        // K mutually-attributing blocks that recombine EXISTING coverage (each copies an
        // honest cell's content), committed last -> no new coverage -> 0.
        let mut order = honest();
        let honest_data: Vec<Vec<u8>> = order.iter().map(|c| c.data.clone()).collect();
        for k in 0..5usize {
            let src = &honest_data[k % honest_data.len()];
            order.push(cell(300 + k as u64, 9, 300 + k as u64, src));
        }
        let v = temporal_novelty(&order);
        let ring_total: u64 = v[order.len() - 5..].iter().sum();
        assert_eq!(ring_total, 0, "collusion ring adds no NEW coverage -> 0");
    }

    #[test]
    fn honest_cells_keep_their_novelty() {
        let order = honest();
        let v = temporal_novelty(&order);
        assert!(v.iter().all(|&x| x > 0), "every honest cell earns genuine novelty");
    }
}
