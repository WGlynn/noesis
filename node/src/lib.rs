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
    /// TRANSFERABLE component: byte-capacity ownership. `lock.args` carry the CURRENT
    /// owner pubkey hash; the program authorizes the spend. Changes on every transfer.
    pub lock: Script,
    /// SOULBOUND component: the PoM-attestation. A shared `code_hash` identifies the
    /// canonical PoM type-script program; `type_script.args` pin the CONTRIBUTOR identity
    /// (the mind that proved the value). Set at mint, NEVER reassigned on transfer — this
    /// is what keeps consensus franchise off the transferable byte. (Noesis augmentation
    /// over CKB: a CKB cell has no soulbound attestation; here the type script carries one.)
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

/// PoM score per CONTRIBUTOR = sum of temporal-novelty value of authored cells.
///
/// Keyed by `type_script.args` (the SOULBOUND contributor identity), NOT by `lock.args`
/// (the transferable byte-owner). This is the coherence fix: when a byte is sold the lock
/// changes but the PoM-attestation stays with the mind that proved the value. Consensus
/// franchise therefore tracks soulbound standing, never bought bytes (buy storage, not
/// consensus). Resolves the POM-CONSENSUS "transferable credit" vs CRYPTOECONOMICS
/// "soulbound standing" contradiction in favor of the latter.
pub fn pom_scores(cells_in_commit_order: &[Cell]) -> HashMap<Vec<u8>, u64> {
    let vals = temporal_novelty(cells_in_commit_order);
    let mut pom: HashMap<Vec<u8>, u64> = HashMap::new();
    for (c, v) in cells_in_commit_order.iter().zip(vals) {
        *pom.entry(c.type_script.args.clone()).or_insert(0) += v;
    }
    pom
}

/// Shardability: cells partition by id; no cross-shard state dependency in the cell
/// itself (the only shared object is the committed novelty-set, distributed as a
/// commitment in production).
pub fn shard_of(cell: &Cell, n: u64) -> u64 {
    cell.id % n
}

/// SOULBOUND in the cell/UTXO model.
///
/// A cell is transferable by DEFAULT — nothing stops a spend from producing an output
/// with a different lock. There is no account to freeze. So "non-transferable" cannot be
/// a flag in the data; it is an INVARIANT the type script enforces on the consume->produce
/// transition. The PoM standing-cell's type script admits only identity-preserving
/// successors (accrue / decay / slash / burn) and REJECTS any output that reassigns the
/// owner lock or the contributor. The RISC-V program returns failure -> the tx is invalid
/// -> the cell can evolve but never change hands. Reassignment is made unrepresentable.
///
/// This is the soulbound half of the two-cell mint: the freely-tradable **capacity cell**
/// (state-bytes = money) rides the [`ownership`] fold; this **standing cell** carries the
/// franchise and cannot move. Consensus reads `Standing.contributor`, never byte-ownership.
pub mod soulbound {
    use super::Cell;

    /// Franchise state carried by a soulbound standing-cell (Molecule-encoded in
    /// `cell.data` on-VM; explicit here in the reference spec).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Standing {
        /// the mind that earned this standing — invariant across EVERY valid transition.
        pub contributor: [u8; 32],
        /// accumulated novelty-value credit; append-only except via decay/slash.
        pub pom: u64,
    }

    /// The only identity-preserving operations the type script permits. Anything outside
    /// this set is, by construction, a reassignment — and gets rejected.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Op {
        /// add newly-finalized novelty value (accrual proof checked by the value layer).
        Accrue(u64),
        /// reduce capacity per the rent / decay schedule (the supply sink).
        Decay(u64),
        /// revoke on a proven refutation (dispute-window slashing).
        Slash(u64),
        /// voluntarily destroy your own standing (no successor output).
        Burn,
    }

    /// Apply an op: produce the REQUIRED successor `(cell, standing)`, or `None` for burn.
    /// The successor cell is byte-identical in lock + type_script (owner + contributor
    /// unchanged); only the carried `Standing.pom` moves. Reassignment cannot be expressed.
    pub fn apply(input: &Cell, st: &Standing, op: Op) -> Option<(Cell, Standing)> {
        let next = |pom| (input.clone(), Standing { contributor: st.contributor, pom });
        match op {
            Op::Burn => None,
            Op::Accrue(d) => Some(next(st.pom.saturating_add(d))),
            Op::Decay(d) => Some(next(st.pom.saturating_sub(d))),
            Op::Slash(d) => Some(next(st.pom.saturating_sub(d))),
        }
    }

    /// The on-chain check: a proposed transition is valid IFF the output is an
    /// identity-preserving successor of the input. Any change to `lock.args` (owner) or
    /// to the contributor is REJECTED — that rejection IS the soulbound guarantee.
    pub fn valid_transition(
        input: &Cell,
        in_st: &Standing,
        output: Option<&Cell>,
        out_st: Option<&Standing>,
    ) -> bool {
        match (output, out_st) {
            (None, None) => true, // burn: destroying your own standing is always allowed
            (Some(o), Some(os)) => {
                o.lock.args == input.lock.args                  // owner unchanged
                    && o.type_script.args == input.type_script.args // contributor (type args) unchanged
                    && os.contributor == in_st.contributor          // soulbound identity invariant
            }
            _ => false, // malformed: a standing without its cell, or vice versa
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::Script;
        use super::*;

        fn standing_cell(owner: u8, contributor: [u8; 32]) -> (Cell, Standing) {
            let c = Cell {
                id: 1,
                lock: Script { code_hash: [1u8; 32], args: vec![owner] },
                type_script: Script { code_hash: [0xB0; 32], args: contributor.to_vec() },
                parent: None,
                timestamp: 0,
                data: vec![],
            };
            (c, Standing { contributor, pom: 100 })
        }

        #[test]
        fn accrue_decay_slash_same_owner_accepted() {
            let (c, st) = standing_cell(1, [0xC1; 32]);
            let (oa, sa) = apply(&c, &st, Op::Accrue(10)).unwrap();
            assert!(valid_transition(&c, &st, Some(&oa), Some(&sa)));
            assert_eq!(sa.pom, 110);
            let (od, sd) = apply(&c, &st, Op::Decay(5)).unwrap();
            assert!(valid_transition(&c, &st, Some(&od), Some(&sd)));
            assert_eq!(sd.pom, 95);
            let (os, ss) = apply(&c, &st, Op::Slash(40)).unwrap();
            assert!(valid_transition(&c, &st, Some(&os), Some(&ss)));
            assert_eq!(ss.pom, 60);
        }

        #[test]
        fn burn_accepted() {
            let (c, st) = standing_cell(1, [0xC1; 32]);
            assert!(apply(&c, &st, Op::Burn).is_none());
            assert!(valid_transition(&c, &st, None, None));
        }

        #[test]
        fn reassign_owner_is_rejected() {
            // The soulbound proof: try to move standing to a NEW owner key.
            let (c, st) = standing_cell(1, [0xC1; 32]);
            let mut malicious = c.clone();
            malicious.lock.args = vec![9]; // adversary's key
            assert!(
                !valid_transition(&c, &st, Some(&malicious), Some(&st)),
                "reassigning the owner lock must be rejected -> non-transferable"
            );
        }

        #[test]
        fn reattribute_contributor_is_rejected() {
            // Try to steal another mind's credit by changing the contributor identity.
            let (c, st) = standing_cell(1, [0xC1; 32]);
            let mut malicious = c.clone();
            malicious.type_script.args = [0x99; 32].to_vec();
            let stolen = Standing { contributor: [0x99; 32], pom: st.pom };
            assert!(
                !valid_transition(&c, &st, Some(&malicious), Some(&stolen)),
                "changing the contributor must be rejected -> soulbound attestation"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POM_TYPE: [u8; 32] = [0xB0; 32]; // placeholder code_hash for the PoM program

    // Default mint: contributor == owner (the common case at creation). `type_script.args`
    // carry the soulbound contributor; `lock.args` the transferable owner.
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

        // ---- Adversarial-gaming loop vs the LEARNED v(S) (Roadmap Phase 1, line 41) ----

        #[test]
        fn learned_quality_preserves_the_novelty_floor() {
            // The Phase-1 proof obligation: the *learned* quality model must not be able
            // to rescue a redundant cell. Unlike the pinned-1.0 test, here quality is the
            // ACTUAL trained Bradley-Terry output. A sybil clone committed last has novelty
            // 0, and novelty MULTIPLIES, so it earns 0 at whatever quality the model assigns.
            // This is the regression guard that the capability layer cannot breach the
            // strategyproof floor.
            let mut order = honest();
            let clone = order[0].data.clone();
            order.push(cell(99, 9, 99, &clone)); // sybil clone, committed last
            let q = quality_scores(&order); // learned, not pinned
            let v = value_v4(&order, &q);
            let last = v.len() - 1;
            assert_eq!(v[last], 0.0, "learned quality cannot rescue novelty-0 redundancy");
            assert!(v[..last].iter().any(|&x| x > 0.0), "honest cells still earn value");
        }

        #[test]
        fn garbage_novelty_is_the_documented_open_gap() {
            // HONEST boundary (build-don't-claim). The strategyproof floor catches
            // REDUNDANCY (sybil/padding/collusion → 0). It does NOT catch high-entropy
            // garbage that is genuinely novel coverage but worthless: the coverage proxy
            // rewards entropy it cannot distinguish from value. A 64-byte high-entropy cell
            // committed last shares no shingles with the honest set, so it earns positive
            // novelty. This test PINS that gap: it passes today (vulnerability present) and
            // will FLIP when the learned OUTCOME-evaluator (not the coverage proxy) lands,
            // forcing this boundary to be revisited. Roadmap Phase 1 🔬 remains open here.
            let mut order = honest();
            let garbage: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            order.push(cell(77, 9, 77, &garbage));
            let q = vec![0.0; order.len()]; // even at ZERO quality, novelty alone earns value
            let v = value_v4(&order, &q);
            assert!(
                *v.last().unwrap() > 0.0,
                "KNOWN GAP: coverage-proxy v(S) cannot tell novel-garbage from novel-value; \
                 closing it needs the learned outcome-evaluator (Phase 1, line 41)"
            );
        }
    }
}

/// Synergy aggregation (port of block-value-v2.py): a SUBMODULAR outcome-value with
/// MYERSON credit, sampled Data-Shapley style. This is the inter-block aggregation of
/// §4: value flows along the provenance DAG, pivotal blocks earn more, redundant blocks
/// earn little — and crucially the cooperative machinery is *load-bearing* (synergy
/// Shapley differs from the additive win-share). Distinct from [`temporal_novelty`]
/// (commit-order floor); this is the synergy game over coverage unions.
///
/// `v(S) = |union of coverage(cells in S)|` is submodular (a redundant block adds
/// little). The Myerson restricted game `v^g(S)` sums `v` over the connected components
/// of `S` under parent edges, so disconnected coalitions cannot pool value. Exact
/// Shapley is `O(2^N)`; we estimate by permutation sampling (Data-Shapley) with a
/// deterministic PRNG so runs are reproducible (no `rand` dependency).
pub mod synergy {
    use super::{coverage, Cell, CovId};
    use std::collections::{HashMap, HashSet};

    /// SplitMix64 — tiny deterministic PRNG. Reproducible sampling without a crate dep.
    fn splitmix64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Submodular coverage value of a sub-coalition (size of the union of coverage sets).
    fn v_coverage(cells: &[Cell], idxs: &[usize]) -> u64 {
        let mut u: HashSet<CovId> = HashSet::new();
        for &i in idxs {
            u.extend(coverage(&cells[i].data));
        }
        u.len() as u64
    }

    fn find(parent: &mut HashMap<usize, usize>, mut x: usize) -> usize {
        while parent[&x] != x {
            let g = parent[&parent[&x]];
            parent.insert(x, g); // path halving
            x = g;
        }
        x
    }

    /// Connected components of `idxs` under provenance (parent) edges: a cell and its
    /// parent are joined iff BOTH are present in the coalition.
    fn components(cells: &[Cell], idxs: &[usize]) -> Vec<Vec<usize>> {
        let in_s: HashMap<u64, usize> = idxs.iter().map(|&i| (cells[i].id, i)).collect();
        let mut parent: HashMap<usize, usize> = idxs.iter().map(|&i| (i, i)).collect();
        for &i in idxs {
            if let Some(pid) = cells[i].parent {
                if let Some(&j) = in_s.get(&pid) {
                    let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                    parent.insert(ri, rj);
                }
            }
        }
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for &i in idxs {
            let r = find(&mut parent, i);
            groups.entry(r).or_default().push(i);
        }
        groups.into_values().collect()
    }

    /// Myerson restricted value: only provenance-connected sub-coalitions create value.
    fn v_graph(cells: &[Cell], idxs: &[usize]) -> u64 {
        components(cells, idxs)
            .iter()
            .map(|c| v_coverage(cells, c))
            .sum()
    }

    /// Data-Shapley permutation sampling. `restricted = true` ⇒ Myerson (graph game);
    /// `false` ⇒ plain submodular Shapley. Deterministic in `(cells, samples)`.
    pub fn sampled_value(cells: &[Cell], samples: usize, restricted: bool) -> Vec<f64> {
        let n = cells.len();
        let mut phi = vec![0.0f64; n];
        if n == 0 || samples == 0 {
            return phi;
        }
        let val = |idxs: &[usize]| -> u64 {
            if restricted {
                v_graph(cells, idxs)
            } else {
                v_coverage(cells, idxs)
            }
        };
        for t in 0..samples {
            let mut seed = 1000u64.wrapping_add(t as u64);
            let mut perm: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let r = (splitmix64(&mut seed) % (i as u64 + 1)) as usize;
                perm.swap(i, r);
            }
            let mut running: Vec<usize> = Vec::with_capacity(n);
            let mut prev = 0.0f64;
            for &b in &perm {
                running.push(b);
                let cur = val(&running) as f64;
                phi[b] += cur - prev;
                prev = cur;
            }
        }
        phi.iter().map(|x| x / samples as f64).collect()
    }

    /// Normalize a value vector into shares summing to 1 (0 vector ⇒ 0 shares).
    pub fn shares(phi: &[f64]) -> Vec<f64> {
        let tot: f64 = phi.iter().sum();
        if tot <= 0.0 {
            return vec![0.0; phi.len()];
        }
        phi.iter().map(|x| x / tot).collect()
    }

    /// Additive baseline: Copeland win-share over coverage size. This is what a naive
    /// "pairwise wins" game collapses to — the thing synergy must beat to earn its keep.
    pub fn copeland_shares(cells: &[Cell]) -> Vec<f64> {
        let cov: Vec<usize> = cells.iter().map(|c| coverage(&c.data).len()).collect();
        let wins: Vec<f64> = (0..cells.len())
            .map(|i| cov.iter().enumerate().filter(|(j, &cj)| *j != i && cov[i] > cj).count() as f64)
            .collect();
        shares(&wins)
    }

    /// L1 distance between two share vectors (the "is the cooperative game load-bearing?"
    /// statistic: synergy Shapley should diverge from additive Copeland).
    pub fn l1(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
    }

    #[cfg(test)]
    mod tests {
        use super::super::{Cell, Script};
        use super::*;

        fn cell(id: u64, parent: Option<u64>, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![] },
                parent,
                timestamp: id,
                data: data.to_vec(),
            }
        }

        // Two near-duplicate cells + one unique. Additive (cov-size) over-credits the
        // duplicates; submodular Shapley makes them SHARE their overlapping coverage.
        fn overlapping() -> Vec<Cell> {
            vec![
                cell(0, None, b"alpha-bravo-charlie-delta-echo"),
                cell(1, None, b"alpha-bravo-charlie-delta-echo-foxtrot"), // ~dup of 0
                cell(2, None, b"uniform-victor-whiskey-xray-yankee"),     // disjoint, unique
            ]
        }

        #[test]
        fn synergy_shapley_differs_from_additive_copeland() {
            let cells = overlapping();
            let sh = shares(&sampled_value(&cells, 2000, false));
            let cop = copeland_shares(&cells);
            assert!(
                l1(&sh, &cop) > 0.02,
                "submodular Shapley must diverge from additive win-share (L1={})",
                l1(&sh, &cop)
            );
            // the unique disjoint cell should out-earn each redundant near-duplicate
            assert!(sh[2] > sh[0] && sh[2] > sh[1], "pivotal/unique cell earns more than redundant ones");
        }

        #[test]
        fn redundant_cell_gets_low_shapley_marginal() {
            // cell 1 ⊇ cell 0 in coverage and a 3rd is disjoint: cell 0 is mostly redundant.
            let cells = overlapping();
            let phi = sampled_value(&cells, 2000, false);
            assert!(phi.iter().all(|&x| x >= -1e-9), "marginals are non-negative for a monotone game");
            assert!(phi[0] < phi[2], "redundant cell earns a smaller marginal than the unique one");
        }

        #[test]
        fn myerson_restricts_value_to_provenance() {
            // Same two overlapping cells. With NO edge they are separate components, so the
            // graph game double-counts their shared coverage; WITH an edge they merge into
            // one component and the overlap is counted once -> Myerson shares change.
            let disconnected = vec![
                cell(0, None, b"alpha-bravo-charlie-delta-echo"),
                cell(1, None, b"alpha-bravo-charlie-delta-echo-foxtrot"),
            ];
            let connected = vec![
                cell(0, None, b"alpha-bravo-charlie-delta-echo"),
                cell(1, Some(0), b"alpha-bravo-charlie-delta-echo-foxtrot"), // 1's parent = 0
            ];
            let my_disc = shares(&sampled_value(&disconnected, 2000, true));
            let my_conn = shares(&sampled_value(&connected, 2000, true));
            assert!(
                l1(&my_disc, &my_conn) > 0.02,
                "provenance edges change Myerson credit (L1={})",
                l1(&my_disc, &my_conn)
            );
        }

        #[test]
        fn sampling_is_deterministic() {
            let cells = overlapping();
            let a = sampled_value(&cells, 500, true);
            let b = sampled_value(&cells, 500, true);
            assert_eq!(a, b, "deterministic PRNG ⇒ identical results across runs");
        }
    }
}

/// Eigenvector value-flow over the provenance DAG + two-level recursion
/// (port of `value-flow.py`). Two pieces:
///
/// **FLOW.** A cell earns credit not only for its OWN value but for the value of what was
/// built ON it: propagate backward along parent edges with damping `d < 1` (PageRank /
/// EigenTrust style). `flow(b) = own(b) + d · Σ flow(children of b)`. Damping guarantees
/// convergence AND bounds self-referential cycles — the §8 circularity guard made
/// mechanical (a ring of self-attributing cells cannot pump its own flow unboundedly).
///
/// **RECURSION.** Split a cell's value among its INTRA-block contributors by the Shapley
/// value of a sub-game with genuine synergy. Same cooperative machinery one level down ⇒
/// the economy is two-level recursive: outcome → cells → contributors. The 2-player closed
/// form (operator = prompt, model = response) mirrors the Python; [`recurse_shares`]
/// generalizes to N contributors by reusing the [`super::synergy`] coverage game.
pub mod flow {
    use super::{coverage, Cell, CovId};
    use std::collections::{HashMap, HashSet};

    /// own-value proxy = coverage size of the cell's data (≥ 1 for non-empty) — the same
    /// proxy the learned reward model replaces in production.
    pub fn own_value(cell: &Cell) -> f64 {
        coverage(&cell.data).len().max(1) as f64
    }

    /// parent_id → child indices, from the REAL parent linkage on the cells (self-loops
    /// dropped). Children of a cell are the cells that built on it.
    fn children_of(cells: &[Cell]) -> HashMap<u64, Vec<usize>> {
        let mut ch: HashMap<u64, Vec<usize>> = HashMap::new();
        for (i, c) in cells.iter().enumerate() {
            if let Some(p) = c.parent {
                if p != c.id {
                    ch.entry(p).or_default().push(i);
                }
            }
        }
        ch
    }

    /// `flow(b) = own(b) + d · Σ_{c built on b} flow(c)` by damped Jacobi iteration.
    /// `d < 1` ⇒ contraction ⇒ converges; a self-referential cycle stays bounded by `d`.
    /// Returns `(own, flow)`, both index-aligned with `cells`.
    pub fn value_flow(cells: &[Cell], d: f64, iters: usize) -> (Vec<f64>, Vec<f64>) {
        let own: Vec<f64> = cells.iter().map(own_value).collect();
        let mut flow = own.clone();
        if cells.is_empty() {
            return (own, flow);
        }
        let children = children_of(cells);
        let id_to_idx: HashMap<u64, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
        for _ in 0..iters {
            let mut next = own.clone();
            for (pid, kids) in &children {
                if let Some(&pi) = id_to_idx.get(pid) {
                    let s: f64 = kids.iter().map(|&k| flow[k]).sum();
                    next[pi] = own[pi] + d * s;
                }
            }
            let delta = next
                .iter()
                .zip(&flow)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f64::max);
            flow = next;
            if delta < 1e-9 {
                break;
            }
        }
        (own, flow)
    }

    fn cov_set(part: &[u8]) -> HashSet<CovId> {
        coverage(part).into_iter().collect()
    }

    /// Two-level recursion, 2-player case: split a cell's value by the Shapley value of a
    /// synergy sub-game. `v({a})=|cov(a)|`, `v({b})=|cov(b)|`, `v({a,b})=|cov(a) ∪ cov(b)|`.
    /// Shapley(2): `φ_i = ½·v({i}) + ½·(v(N) − v({other}))`. Returns `(share_a, share_b,
    /// synergy)` with `synergy = v(N) − (v(a)+v(b)) ≤ 0` (set-union coverage is sub-additive;
    /// `< 0` ⇒ redundant overlap, `0` ⇒ disjoint/independent).
    pub fn recurse_two(part_a: &[u8], part_b: &[u8]) -> (f64, f64, i64) {
        let (ca, cb) = (cov_set(part_a), cov_set(part_b));
        let v_a = ca.len() as f64;
        let v_b = cb.len() as f64;
        let v_both = ca.union(&cb).count() as f64;
        let phi_a = 0.5 * v_a + 0.5 * (v_both - v_b);
        let phi_b = 0.5 * v_b + 0.5 * (v_both - v_a);
        let tot = if phi_a + phi_b > 0.0 { phi_a + phi_b } else { 1.0 };
        let synergy = v_both as i64 - (v_a as i64 + v_b as i64);
        (phi_a / tot, phi_b / tot, synergy)
    }

    /// N-contributor intra-block recursion: Shapley shares over the submodular
    /// coverage-union sub-game among a cell's contributor payloads. Reuses
    /// [`super::synergy`] (plain submodular Shapley, sampled) one level down — literally the
    /// same machinery scoring intra-block authors that scores inter-block cells. Deterministic.
    pub fn recurse_shares(parts: &[&[u8]], samples: usize) -> Vec<f64> {
        use super::{Script};
        let cells: Vec<Cell> = parts
            .iter()
            .enumerate()
            .map(|(i, p)| Cell {
                id: i as u64,
                lock: Script { code_hash: [0u8; 32], args: vec![] },
                type_script: Script { code_hash: [0u8; 32], args: vec![] },
                parent: None, // contributors are an unordered coalition (no provenance edges)
                timestamp: i as u64,
                data: p.to_vec(),
            })
            .collect();
        super::synergy::shares(&super::synergy::sampled_value(&cells, samples, false))
    }

    #[cfg(test)]
    mod tests {
        use super::super::{Cell, Script};
        use super::*;

        fn cell(id: u64, parent: Option<u64>, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![] },
                parent,
                timestamp: id,
                data: data.to_vec(),
            }
        }

        // A root built upon by a mid cell, built upon by a leaf.
        fn chain() -> Vec<Cell> {
            vec![
                cell(0, None, b"root-alpha-bravo"),
                cell(1, Some(0), b"mid-charlie-delta"),
                cell(2, Some(1), b"leaf-echo-foxtrot"),
            ]
        }

        #[test]
        fn parent_gets_uplift_leaf_does_not() {
            let cells = chain();
            let (own, flow) = value_flow(&cells, 0.85, 200);
            assert!(flow[0] > own[0], "root earns credit for what built on it");
            assert!(flow[1] > own[1], "mid earns credit for the leaf");
            assert!((flow[2] - own[2]).abs() < 1e-9, "childless leaf -> flow == own");
        }

        #[test]
        fn flow_converges_and_damping_bounds_self_reference() {
            // A 2-cycle (a<->b) would diverge undamped; d<1 keeps it finite — the
            // self-reference guard, mechanical.
            let cyclic = vec![
                cell(0, Some(1), b"aaa-bbb-ccc"),
                cell(1, Some(0), b"ddd-eee-fff"),
            ];
            let (_own, flow) = value_flow(&cyclic, 0.85, 1000);
            assert!(flow.iter().all(|x| x.is_finite()), "damped flow stays finite on a cycle");
            assert!(flow.iter().all(|&x| x > 0.0));
        }

        #[test]
        fn higher_damping_gives_more_uplift() {
            let cells = chain();
            let (_o1, f_lo) = value_flow(&cells, 0.5, 200);
            let (_o2, f_hi) = value_flow(&cells, 0.9, 200);
            assert!(f_hi[0] > f_lo[0], "more weight on descendants -> more uplift to the root");
        }

        #[test]
        fn recurse_two_redundant_contributor_earns_less() {
            // part_b's shingles ⊆ part_a: a is pivotal, b redundant.
            let a = b"alpha-bravo-charlie-delta-echo";
            let b = b"alpha-bravo";
            let (sa, sb, syn) = recurse_two(a, b);
            assert!(sa > sb, "pivotal contributor out-earns the redundant one");
            assert!((sa + sb - 1.0).abs() < 1e-9, "shares normalize to 1");
            assert!(syn <= 0, "set-union coverage is sub-additive: synergy <= 0");
        }

        #[test]
        fn recurse_two_disjoint_is_balanced() {
            let a = b"alpha-bravo-charlie";
            let b = b"uniform-victor-whiskey";
            let (sa, sb, syn) = recurse_two(a, b);
            assert_eq!(syn, 0, "disjoint coverage -> zero synergy");
            assert!((sa - sb).abs() < 0.20, "roughly balanced for similar-size disjoint parts");
        }

        #[test]
        fn recurse_shares_generalizes_and_discounts_duplicates() {
            // contributors 0 and 2 are identical; 1 is unique. The submodular game makes the
            // duplicates SHARE their coverage, so the unique one earns >= each duplicate.
            let parts: Vec<&[u8]> = vec![
                b"alpha-bravo-charlie-delta",
                b"echo-foxtrot-golf-hotel",
                b"alpha-bravo-charlie-delta",
            ];
            let sh = recurse_shares(&parts, 2000);
            assert_eq!(sh.len(), 3);
            assert!((sh.iter().sum::<f64>() - 1.0).abs() < 1e-9, "shares normalize to 1");
            assert!(sh[1] >= sh[0] - 1e-6 && sh[1] >= sh[2] - 1e-6, "unique >= each duplicate");
            assert!((sh[0] - sh[2]).abs() < 0.05, "identical contributors earn ~equal shares");
        }
    }
}

/// PoM-weighted consensus — finalization, retention-decay, and AND-vs-OR composition made
/// concrete and TESTED (build-don't-claim). The docs mark consensus "designed, not built";
/// this is the reference model with this session's findings baked in as regression tests:
///   - the **2/3 supermajority** keeps any single dimension (PoM 60%) below the finalize bar
///     (COHERENCE-LAWS L12 — no single dimension finalizes alone; capture needs a coalition);
///   - NCI's **non-decaying PoS** drifts the effective mix toward capital under staleness
///     (POM-CONSENSUS retention-decay surface), and **symmetric franchise-decay** fixes it;
///   - a **base-weight** threshold halts under low participation; an **effective-weight**
///     threshold does not (the paired fix).
/// Models VOTE-WEIGHT only; the staked balance is a separate field decay never touches.
pub mod consensus {
    /// Per-dimension mix weights (fractions of combined weight; sum = 1). NCI = 0.10/0.30/0.60.
    #[derive(Clone, Copy, Debug)]
    pub struct Mix {
        pub pow: f64,
        pub pos: f64,
        pub pom: f64,
    }

    /// NCI as-built (`NakamotoConsensusInfinity.sol`: POW/POS/POM = 1000/3000/6000 BPS).
    pub const NCI: Mix = Mix { pow: 0.10, pos: 0.30, pom: 0.60 };
    /// 2/3 supermajority in BPS (NCI `FINALIZATION_THRESHOLD_BPS`).
    pub const TWO_THIRDS_BPS: u64 = 6667;
    pub const BPS: u64 = 10_000;

    /// A validator's three proof inputs (base, pre-mix) + liveness + an untouchable stake.
    #[derive(Clone, Debug)]
    pub struct Validator {
        pub id: u64,
        pub pow: f64,
        pub pos: f64,
        pub pom: f64,
        pub last_heartbeat: u64,
        /// staked capital — franchise decay NEVER reduces this (decay the vote, not the balance).
        pub staked_balance: f64,
    }

    /// Combined base weight `W = pow·m.pow + pos·m.pos + pom·m.pom`.
    pub fn base_weight(v: &Validator, m: Mix) -> f64 {
        v.pow * m.pow + v.pos * m.pos + v.pom * m.pom
    }

    /// Linear retention: 1.0 fresh, → 0.0 as `elapsed → horizon`, clamped.
    pub fn retention(elapsed: u64, horizon: u64) -> f64 {
        if horizon == 0 {
            return 1.0;
        }
        (1.0 - elapsed as f64 / horizon as f64).clamp(0.0, 1.0)
    }

    /// Retention-adjusted vote weight. `decay_pos=false` = NCI as-built (PoS/capital does not
    /// decay; only PoW+PoM portions fade). `decay_pos=true` = the symmetric-decay fix.
    pub fn effective_weight(v: &Validator, m: Mix, now: u64, horizon: u64, decay_pos: bool) -> f64 {
        let ret = retention(now.saturating_sub(v.last_heartbeat), horizon);
        let pos_portion = v.pos * m.pos;
        let decayable = v.pow * m.pow + v.pom * m.pom;
        if decay_pos {
            (decayable + pos_portion) * ret
        } else {
            pos_portion + decayable * ret
        }
    }

    /// Where the 2/3 bar is measured: against total BASE weight (NCI as-built) or total
    /// EFFECTIVE (decayed) weight (the liveness fix).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ThresholdBasis {
        Base,
        Effective,
    }

    /// Does a proposal finalize? `weightFor` (effective) must reach `threshold_bps` of the basis.
    #[allow(clippy::too_many_arguments)]
    pub fn finalizes(
        voters_for: &[Validator],
        all: &[Validator],
        m: Mix,
        now: u64,
        horizon: u64,
        decay_pos: bool,
        threshold_bps: u64,
        basis: ThresholdBasis,
    ) -> bool {
        let weight_for: f64 = voters_for
            .iter()
            .map(|v| effective_weight(v, m, now, horizon, decay_pos))
            .sum();
        let basis_total: f64 = match basis {
            ThresholdBasis::Base => all.iter().map(|v| base_weight(v, m)).sum(),
            ThresholdBasis::Effective => all
                .iter()
                .map(|v| effective_weight(v, m, now, horizon, decay_pos))
                .sum(),
        };
        basis_total > 0.0 && weight_for >= basis_total * threshold_bps as f64 / BPS as f64
    }

    /// The L12 question: can an actor owning 100% of ONE dimension finalize alone? True iff
    /// that dimension's mix fraction reaches the threshold. PoM 0.60 vs 2/3 → false.
    pub fn single_dimension_can_finalize(dim_fraction: f64, threshold_bps: u64) -> bool {
        dim_fraction >= threshold_bps as f64 / BPS as f64
    }

    /// Effective mix shares `(pow, pos, pom)` of total effective weight — to observe drift.
    pub fn effective_mix(
        all: &[Validator],
        m: Mix,
        now: u64,
        horizon: u64,
        decay_pos: bool,
    ) -> (f64, f64, f64) {
        let (mut pw, mut ps, mut pm) = (0.0, 0.0, 0.0);
        for v in all {
            let ret = retention(now.saturating_sub(v.last_heartbeat), horizon);
            let (rpw, rpm) = (v.pow * m.pow * ret, v.pom * m.pom * ret);
            let rps = if decay_pos { v.pos * m.pos * ret } else { v.pos * m.pos };
            pw += rpw;
            ps += rps;
            pm += rpm;
        }
        let tot = pw + ps + pm;
        if tot <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        (pw / tot, ps / tot, pm / tot)
    }

    /// NCI `MIN_STAKE` (100e18; units here). Registration floor — the basis of sybil economics.
    pub const MIN_STAKE: f64 = 100.0;

    /// A3 — sybil economics. A validator below `MIN_STAKE` is ineligible (its weight does not
    /// count); splitting one identity into K validators costs `K × MIN_STAKE`. Vote-weight is
    /// additive, but the stake floor makes splitting non-free and caps K at
    /// `floor(total_capital / MIN_STAKE)`.
    pub fn eligible(v: &Validator) -> bool {
        v.staked_balance >= MIN_STAKE
    }
    pub fn registration_cost(num_validators: usize) -> f64 {
        num_validators as f64 * MIN_STAKE
    }
    pub fn max_sybils(total_capital: f64) -> u64 {
        (total_capital / MIN_STAKE).floor() as u64
    }

    /// A5 — slashability is orthogonal to retention. A proven-bad validator is slashed (capital
    /// reduced + proof inputs revoked) REGARDLESS of how stale its franchise is. Decay fades
    /// influence; it must never be an exit from accountability.
    pub fn slash(v: &mut Validator, amount: f64) {
        v.staked_balance = (v.staked_balance - amount).max(0.0);
        v.pow = 0.0;
        v.pos = 0.0;
        v.pom = 0.0;
    }

    /// A1 — hybrid finalization basis = `max(effective_total, quorum_floor)`, where the floor is
    /// `quorum_floor_bps` of the BASE total. The floor stops an eclipse attacker from shrinking
    /// the denominator below real honest participation, while the effective term still closes the
    /// staleness liveness-halt. Promotes the self-audit's test-local fix to a real consensus param.
    #[allow(clippy::too_many_arguments)]
    pub fn finalizes_hybrid(
        voters_for: &[Validator],
        all: &[Validator],
        m: Mix,
        now: u64,
        horizon: u64,
        decay_pos: bool,
        threshold_bps: u64,
        quorum_floor_bps: u64,
    ) -> bool {
        let weight_for: f64 = voters_for
            .iter()
            .map(|v| effective_weight(v, m, now, horizon, decay_pos))
            .sum();
        let eff_total: f64 = all.iter().map(|v| effective_weight(v, m, now, horizon, decay_pos)).sum();
        let base_total: f64 = all.iter().map(|v| base_weight(v, m)).sum();
        let floor = base_total * quorum_floor_bps as f64 / BPS as f64;
        let basis = eff_total.max(floor);
        basis > 0.0 && weight_for >= basis * threshold_bps as f64 / BPS as f64
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn val(id: u64, pow: f64, pos: f64, pom: f64, hb: u64) -> Validator {
            Validator { id, pow, pos, pom, last_heartbeat: hb, staked_balance: pos }
        }
        fn cohort() -> Vec<Validator> {
            vec![val(0, 1.0, 1.0, 1.0, 0), val(1, 1.0, 1.0, 1.0, 0), val(2, 1.0, 1.0, 1.0, 0)]
        }

        #[test]
        fn single_dimension_cannot_finalize_under_two_thirds() {
            // L12, verified: PoM is 60% of the mix; the 2/3 bar (66.67%) sits above it.
            assert!(!single_dimension_can_finalize(NCI.pom, TWO_THIRDS_BPS), "PoM 60% < 66.67% bar");
            assert!(!single_dimension_can_finalize(NCI.pos, TWO_THIRDS_BPS));
            assert!(!single_dimension_can_finalize(NCI.pow, TWO_THIRDS_BPS));
        }

        #[test]
        fn single_dimension_would_finalize_under_simple_majority() {
            // Why the supermajority matters: a 50% bar lets PoM (60%) capture alone (OR).
            assert!(single_dimension_can_finalize(NCI.pom, 5000), "PoM 60% > 50% -> OR-capture");
        }

        #[test]
        fn capture_needs_pom_plus_a_second_dimension() {
            // AND-at-the-margin: PoM (0.60) + just over 6.67% of a second dimension clears 2/3.
            assert!(!single_dimension_can_finalize(0.60, TWO_THIRDS_BPS));
            assert!(single_dimension_can_finalize(0.60 + 0.07, TWO_THIRDS_BPS), "PoM + >6.67% of a 2nd dim");
        }

        #[test]
        fn nci_as_built_drifts_toward_capital_under_staleness() {
            // decay_pos=false: stale validators keep full PoS but lose PoW+PoM -> PoS's
            // effective share rises above its base 0.30 (the drift surface).
            let (_pw, ps, _pm) = effective_mix(&cohort(), NCI, 60, 100, false);
            assert!(ps > NCI.pos + 1e-6, "capital's effective share drifts up under staleness: {ps} > 0.30");
        }

        #[test]
        fn symmetric_decay_preserves_composition() {
            // decay_pos=true (Will's fix): all three fade together -> effective mix == base mix.
            let (pw, ps, pm) = effective_mix(&cohort(), NCI, 60, 100, true);
            assert!(
                (pw - NCI.pow).abs() < 1e-9 && (ps - NCI.pos).abs() < 1e-9 && (pm - NCI.pom).abs() < 1e-9,
                "symmetric decay keeps the effective mix at 0.10/0.30/0.60"
            );
        }

        #[test]
        fn base_threshold_halts_under_staleness() {
            // Unanimous support, but everyone is stale; a BASE-weight threshold is unreachable
            // because weightFor (decayed) < 2/3 of base (undecayed) -> liveness halt.
            let all = cohort();
            assert!(
                !finalizes(&all, &all, NCI, 90, 100, true, TWO_THIRDS_BPS, ThresholdBasis::Base),
                "base-weight threshold halts under low effective participation"
            );
        }

        #[test]
        fn effective_threshold_avoids_halt() {
            // Same stale-but-unanimous set; measuring against EFFECTIVE weight lets the present
            // validators finalize (weightFor == basis_total -> 100% >= 66.67%).
            let all = cohort();
            assert!(
                finalizes(&all, &all, NCI, 90, 100, true, TWO_THIRDS_BPS, ThresholdBasis::Effective),
                "effective-weight threshold finalizes on unanimous present support"
            );
        }

        #[test]
        fn decay_touches_vote_weight_not_the_staked_balance() {
            // The franchise decays; the capital does not (decay the vote, not the balance).
            let v = val(7, 1.0, 5.0, 1.0, 0);
            assert!(effective_weight(&v, NCI, 100, 100, true).abs() < 1e-9, "fully-stale franchise -> 0");
            assert_eq!(v.staked_balance, 5.0, "staked balance untouched by franchise decay");
        }

        #[test]
        fn honest_supermajority_finalizes_when_fresh() {
            let all = cohort();
            assert!(finalizes(&all, &all, NCI, 0, 100, false, TWO_THIRDS_BPS, ThresholdBasis::Base));
            assert!(finalizes(&all, &all, NCI, 0, 100, true, TWO_THIRDS_BPS, ThresholdBasis::Effective));
        }

        #[test]
        fn audit_a3_sybil_splitting_is_bounded_by_min_stake() {
            // Splitting fixed capital into K validators costs K×MIN_STAKE and each must clear
            // MIN_STAKE to be eligible -> K is capped at floor(capital / MIN_STAKE).
            assert_eq!(max_sybils(350.0), 3, "350 capital / 100 MIN_STAKE -> at most 3 eligible identities");
            assert_eq!(registration_cost(3), 300.0);
            let under = val(0, 0.0, 50.0, 0.0, 0); // staked 50 < MIN_STAKE
            assert!(!eligible(&under), "a validator below MIN_STAKE is ineligible");
            let ok = Validator { staked_balance: 100.0, ..under };
            assert!(eligible(&ok));
        }

        #[test]
        fn audit_a5_stale_validator_is_still_slashable() {
            // Decay fades the franchise but is NOT an accountability exit: a fully-stale
            // validator (effective weight ~0) can still be slashed for a proven offense.
            let mut v = val(0, 1.0, 5.0, 1.0, 0);
            assert!(effective_weight(&v, NCI, 100, 100, true).abs() < 1e-9, "stale -> franchise ~0");
            slash(&mut v, 3.0);
            assert_eq!(v.staked_balance, 2.0, "stale validator still loses capital when slashed");
            assert_eq!(base_weight(&v, NCI), 0.0, "slashing revokes proof inputs regardless of liveness");
        }

        #[test]
        fn quorum_floor_param_closes_eclipse_and_keeps_liveness() {
            // The real-param hybrid: under eclipse the floor blocks the lone attacker; with honest
            // participation the floor is not binding and the set finalizes.
            let attacker = val(0, 0.0, 10.0, 0.0, 100);
            let all = vec![attacker.clone(), val(1, 0.0, 0.0, 10.0, 0), val(2, 0.0, 0.0, 10.0, 0)];
            assert!(
                !finalizes_hybrid(&[attacker.clone()], &all, NCI, 100, 100, true, TWO_THIRDS_BPS, 5000),
                "quorum floor blocks the eclipse attacker (basis cannot shrink below the floor)"
            );
            let fresh = vec![val(0, 1.0, 1.0, 1.0, 0), val(1, 1.0, 1.0, 1.0, 0), val(2, 1.0, 1.0, 1.0, 0)];
            assert!(
                finalizes_hybrid(&fresh, &fresh, NCI, 0, 100, true, TWO_THIRDS_BPS, 5000),
                "honest unanimous fresh set finalizes under the hybrid"
            );
        }

        // ===================== ADVERSARIAL SELF-AUDIT (RSAW) =====================
        // Attacking this module's own claims. Findings (honest, build-don't-claim):
        //  A1 [TESTED below] The effective-weight threshold that fixes the liveness halt
        //     OPENS AN ECLIPSE SURFACE: shrink the denominator (make honest validators look
        //     stale/absent) and an attacker reaches 2/3 of a shrunken basis ALONE. The base-
        //     weight threshold is eclipse-resistant but halts; neither is free. The real fix
        //     is a hybrid: effective-weight bar PLUS an absolute present-quorum floor so the
        //     denominator cannot be shrunk below a minimum honest participation.
        //  A2 [GAP] `single_dimension_can_finalize` treats a dimension's mix fraction (0.60)
        //     as its realizable share. That is the SATURATION ceiling; the actual share also
        //     depends on cross-node distribution and NCI's log-scaling on PoW+PoM. 0.60 is the
        //     worst case for the L12 claim (correct to test), not the general case.
        //  A3 [GAP] Sybil economics not modeled here: weight is additive across validators, so
        //     splitting is neutral in this model, but real NCI has MIN_STAKE / MAX_VALIDATORS /
        //     registration cost that this reference omits. The value layer's anti-sybil lives in
        //     `temporal_novelty` + the `adversary` module, not here.
        //  A4 [GAP] Lifecycle omitted: equivocation slashing, the early-reject branch
        //     (weightAgainst > total - threshold), and proposal expiry are not modeled.
        //  A5 [OPEN] Decay must not remove SLASHABILITY: a stale validator's franchise → 0 but
        //     they must stay slashable, or staleness becomes a griefing exit. Not modeled.

        #[test]
        fn audit_a1_effective_threshold_opens_an_eclipse_surface() {
            // attacker = pure capital, fresh; honest = PoM, eclipsed (made to look stale).
            let attacker = val(0, 0.0, 10.0, 0.0, 100);
            let mut all = vec![attacker.clone()];
            all.push(val(1, 0.0, 0.0, 10.0, 0));
            all.push(val(2, 0.0, 0.0, 10.0, 0));
            let (now, horizon) = (100, 100); // honest fully stale (eclipsed); attacker fresh
            let base = finalizes(&[attacker.clone()], &all, NCI, now, horizon, true, TWO_THIRDS_BPS, ThresholdBasis::Base);
            let eff = finalizes(&[attacker.clone()], &all, NCI, now, horizon, true, TWO_THIRDS_BPS, ThresholdBasis::Effective);
            assert!(!base, "base-weight threshold is eclipse-RESISTANT: attacker alone cannot finalize");
            assert!(eff, "effective-weight threshold is eclipse-AMPLIFYING: attacker finalizes vs a shrunken basis");
        }

        #[test]
        fn audit_a1_quorum_floor_closes_the_eclipse() {
            // The hybrid mitigation: require weight_for to clear 2/3 of MAX(effective_total,
            // quorum_floor). With a floor at the honest base, the shrunken denominator cannot
            // be exploited and the lone attacker fails again.
            let attacker = val(0, 0.0, 10.0, 0.0, 100);
            let all = vec![attacker.clone(), val(1, 0.0, 0.0, 10.0, 0), val(2, 0.0, 0.0, 10.0, 0)];
            let (now, horizon) = (100, 100);
            let eff_total: f64 = all.iter().map(|v| effective_weight(v, NCI, now, horizon, true)).sum();
            let quorum_floor: f64 = all.iter().map(|v| base_weight(v, NCI)).sum::<f64>() * 0.5; // present-quorum
            let basis = eff_total.max(quorum_floor);
            let weight_for = effective_weight(&attacker, NCI, now, horizon, true);
            let finalizes_with_floor = weight_for >= basis * TWO_THIRDS_BPS as f64 / BPS as f64;
            assert!(!finalizes_with_floor, "a present-quorum floor restores eclipse-resistance under the live bar");
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

    // --- more adversarial rounds on the value/synergy layer (2026-06-11) ---

    #[test]
    fn provenance_forgery_earns_no_synergy_credit() {
        // An attacker block forges a parent edge to a high-coverage block to ride its Myerson
        // component and steal credit, but adds NO new coverage (copies a subset). Submodular
        // union over the connected component => the forger's marginal is ~0.
        let rich = b"alpha-bravo-charlie-delta-echo-foxtrot-golf-hotel";
        let honest = cell(0, 1, 0, rich);
        let mut forger = cell(1, 9, 1, &rich[..12].to_vec()); // subset content (coverage ⊆ rich)
        forger.parent = Some(0); // FORGED provenance edge to the rich block
        let cells = vec![honest, forger];
        let phi = crate::synergy::sampled_value(&cells, 3000, true); // Myerson (graph-restricted)
        assert!(
            phi[1] < phi[0] * 0.25,
            "provenance-forgery with no new coverage earns negligible Myerson credit (forger {} vs honest {})",
            phi[1], phi[0]
        );
    }

    #[test]
    fn quality_boost_cannot_exceed_2x_novelty() {
        // value = novelty * (1 + quality), quality in [0,1] => at most 2x novelty. An attacker
        // maximizing the quality model can never escape the strategyproof novelty floor: a
        // novelty-0 cell stays 0, and a novel cell is bounded at 2x.
        let order = honest();
        let nov = temporal_novelty(&order);
        let max_quality = vec![1.0f64; order.len()];
        let v = crate::value::value_v4(&order, &max_quality);
        for i in 0..order.len() {
            assert!(v[i] <= nov[i] as f64 * 2.0 + 1e-9, "quality boost is bounded by 2x novelty");
            assert!(v[i] >= nov[i] as f64, "value is at least the novelty floor");
        }
    }
}
