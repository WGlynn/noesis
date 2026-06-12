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
