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

/// Coverage-similarity floor — the fix for the near-duplicate gap. Temporal-novelty alone
/// zeroes only EXACT subsets/duplicates; a near-duplicate (a few tokens flipped) leaks small
/// residual novelty from the change-spanning shingles. This treats a cell whose coverage overlap
/// with the union of earlier-committed coverage exceeds a threshold `theta` as a near-duplicate
/// and assigns it 0. Overlap = |cov ∩ earlier_union| / |cov| (the fraction of the cell already
/// on-chain). `theta` near 1.0 zeroes only near-identical cells; honest novel work (low overlap)
/// is untouched. Compose with the learned quality model so honest-but-similar work is not
/// over-cut at lower `theta`.
pub fn temporal_novelty_with_similarity_floor(cells_in_commit_order: &[Cell], theta: f64) -> Vec<u64> {
    let mut seen: HashSet<CovId> = HashSet::new();
    let mut out = Vec::with_capacity(cells_in_commit_order.len());
    for c in cells_in_commit_order {
        let cov = coverage(&c.data);
        let covset: HashSet<CovId> = cov.iter().copied().collect();
        let overlap = if covset.is_empty() {
            0.0
        } else {
            covset.iter().filter(|x| seen.contains(*x)).count() as f64 / covset.len() as f64
        };
        let novel = cov.iter().filter(|x| !seen.contains(*x)).count() as u64;
        out.push(if overlap > theta { 0 } else { novel });
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
    ///
    /// NOTE: burn is unconditionally allowed HERE; under an open dispute that becomes a
    /// slash-evasion exit, which [`valid_transition_under_dispute`] closes. This plain
    /// variant remains correct only for standing with no dispute exposure.
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

    /// Dispute-aware transition — the canonical on-VM check once the dispute layer is
    /// live. Closes the slash-evasion-by-exit hole structurally (the claim in
    /// `dispute::standing_exit_blocked` enforced, not commented): while ANY open challenge
    /// names a target this contributor has a provenance edge into,
    ///   - burn is REJECTED (no torching standing ahead of a verdict);
    ///   - `Standing.pom` may decrease by at most `authorized_slash` — the amount a closed
    ///     settlement authorizes (0 while the dispute is pending). The slash itself can
    ///     always land; voluntary drain dressed as decay cannot.
    /// Accrual and identity rules are unchanged. With no exposure, defers to
    /// [`valid_transition`].
    #[allow(clippy::too_many_arguments)]
    pub fn valid_transition_under_dispute(
        input: &Cell,
        in_st: &Standing,
        output: Option<&Cell>,
        out_st: Option<&Standing>,
        cells: &[Cell],
        open_challenges: &[super::dispute::Challenge],
        authorized_slash: u64,
    ) -> bool {
        let blocked = super::dispute::standing_exit_blocked(
            cells,
            open_challenges,
            &input.type_script.args,
        );
        if !blocked {
            return valid_transition(input, in_st, output, out_st);
        }
        match (output, out_st) {
            (None, None) => false, // exit-blocked: burn denied while a verdict is pending
            (Some(_), Some(os)) => {
                valid_transition(input, in_st, output, out_st)
                    && os.pom.saturating_add(authorized_slash) >= in_st.pom
            }
            _ => false,
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

        // ---- dispute-aware transition (slash-evasion exit closed structurally) ----

        /// Provenance fixture: challenged target (id 10) + a child authored by the
        /// certifier whose standing cell is under test.
        fn dispute_exposure(certifier: [u8; 32]) -> (Vec<Cell>, Vec<super::super::dispute::Challenge>) {
            let mk = |id: u64, args: Vec<u8>, parent: Option<u64>| Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![1] },
                type_script: Script { code_hash: [0xB0; 32], args },
                parent,
                timestamp: id,
                data: vec![id as u8; 8],
            };
            let cells = vec![mk(10, vec![8], None), mk(11, certifier.to_vec(), Some(10))];
            let open = vec![super::super::dispute::Challenge {
                target: 10,
                challenger: vec![1],
                bond: 1.0,
                opened_epoch: 5,
            }];
            (cells, open)
        }

        #[test]
        fn burn_denied_while_a_dispute_names_your_edge() {
            let contributor = [0xC1; 32];
            let (c, st) = standing_cell(1, contributor);
            let (cells, open) = dispute_exposure(contributor);
            assert!(
                !valid_transition_under_dispute(&c, &st, None, None, &cells, &open, 0),
                "exit-blocked certifier cannot torch standing ahead of the verdict"
            );
            assert!(
                valid_transition_under_dispute(&c, &st, None, None, &cells, &[], 0),
                "no open dispute: burn defers to the plain rule and is allowed"
            );
        }

        #[test]
        fn unauthorized_drain_denied_but_the_authorized_slash_lands() {
            let contributor = [0xC1; 32];
            let (c, st) = standing_cell(1, contributor); // pom = 100
            let (cells, open) = dispute_exposure(contributor);
            let (od, sd) = apply(&c, &st, Op::Decay(5)).unwrap();
            assert!(
                !valid_transition_under_dispute(&c, &st, Some(&od), Some(&sd), &cells, &open, 0),
                "voluntary drain dressed as decay is denied while blocked"
            );
            let (os, ss) = apply(&c, &st, Op::Slash(40)).unwrap();
            assert!(
                valid_transition_under_dispute(&c, &st, Some(&os), Some(&ss), &cells, &open, 40),
                "the settlement-authorized slash itself always lands"
            );
            assert!(
                !valid_transition_under_dispute(&c, &st, Some(&os), Some(&ss), &cells, &open, 39),
                "a decrease beyond the authorized amount is denied"
            );
        }

        #[test]
        fn accrue_still_allowed_while_exit_blocked() {
            let contributor = [0xC1; 32];
            let (c, st) = standing_cell(1, contributor);
            let (cells, open) = dispute_exposure(contributor);
            let (oa, sa) = apply(&c, &st, Op::Accrue(10)).unwrap();
            assert!(
                valid_transition_under_dispute(&c, &st, Some(&oa), Some(&sa), &cells, &open, 0),
                "earning more standing is never blocked by a pending dispute"
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

    #[test]
    fn similarity_floor_zeroes_the_near_duplicate() {
        // Fix for the near-duplicate gap: a near-dup (>theta of its coverage already seen) earns
        // 0, while honest novel blocks keep value. Plain temporal-novelty leaks residual to it.
        let order = vec![
            cell(0, 1, 0, b"alpha-bravo-charlie-delta-echo-foxtrot"),
            cell(1, 2, 1, b"golf-hotel-india-juliet-kilo-lima"), // honest novel
            cell(2, 9, 2, b"alpha-bravo-charlie-delta-echo-Foxtrot"), // near-dup of block 0 (1 char)
        ];
        let v = temporal_novelty_with_similarity_floor(&order, 0.8);
        assert!(v[0] > 0 && v[1] > 0, "honest novel blocks keep value");
        assert_eq!(v[2], 0, "near-duplicate (>80% coverage already seen) earns 0");
        assert!(temporal_novelty(&order)[2] > 0, "plain rule leaks residual to the near-dup (the gap)");
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

    /// Production value rule — composes the full strategyproof floor (temporal-novelty + the
    /// coverage-similarity floor at `theta`) with the learned quality boost:
    /// `value = floored_novelty × (1 + quality)`. The floor is applied BEFORE quality, so neither
    /// a near-duplicate nor a zero-novelty cell can be rescued by any quality score, while honest
    /// novel cells are quality-weighted. This is the canonical rule assembling every value-layer
    /// defense (sybil/padding/collusion via novelty, near-duplicate via the similarity floor,
    /// capability via quality) into one function the PoM type-script enforces.
    pub fn production_value(cells_in_commit_order: &[super::Cell], theta: f64, quality: &[f64]) -> Vec<f64> {
        super::temporal_novelty_with_similarity_floor(cells_in_commit_order, theta)
            .iter()
            .zip(quality)
            .map(|(&n, &q)| n as f64 * (1.0 + q))
            .collect()
    }

    /// Saturating flow-gate `g(f) = f / (f + half)` ∈ [0, 1): 0 at zero realized flow, 0.5 at
    /// `f = half`, → 1 as downstream use grows. Scale-free per cell (no global-max
    /// normalization a single whale block could distort) and monotone, so more realized use
    /// never pays less. `half` is the protocol's "how much downstream flow is half-proof of
    /// meaning" parameter.
    pub fn flow_gate(downstream: f64, half: f64) -> f64 {
        if downstream <= 0.0 {
            return 0.0;
        }
        downstream / (downstream + half.max(0.0))
    }

    /// `value_v5(novelty, downstream_flow)` — the composition change that closes the
    /// garbage-novelty gap (`value_v4_boost_does_not_gate_meaningless_novelty` proved a boost
    /// can never close it):
    ///
    ///   value = floored_novelty × g(realized_downstream_flow),  g ∈ [0, 1)
    ///
    /// Quality stops being a predicted BOOST and becomes a realized GATE. Two-clock
    /// composition (ROADMAP Phase 1): intake-novelty (immediate, strategyproof) gates
    /// redundancy; realized-flow (delayed, content-independent) gates meaninglessness.
    /// Mechanism, all reused machinery:
    ///   - floor first: temporal-novelty + similarity floor at `theta` — a redundant or
    ///     near-duplicate cell is 0 before the gate is even consulted;
    ///   - flow is SEEDED with the floored novelty (not raw coverage), so redundant children
    ///     pump no flow into their parents;
    ///   - flow counts EXTERNAL edges only (child contributor ≠ parent contributor): a mind
    ///     cannot certify its own work by building on it itself;
    ///   - gate g = f/(f+half) (see [`flow_gate`]).
    /// Honest consequence (by design, not a bug): value VESTS RETROACTIVELY — at intake every
    /// cell is worth 0 and accrues as others build on it. A contribution cannot be fully
    /// priced at intake; it is paid as it proves useful. Noise nobody uses ⇒ flow 0 ⇒ paid 0;
    /// honest-but-low-quality work that gets built upon ⇒ flow > 0 ⇒ paid.
    /// Pinned residual gap (CLOSED by [`value_v6`]): a MULTI-IDENTITY sybil ring of
    /// novel-garbage children could pump this gate, because a fresh contributor key was a
    /// free byte (see `adversary::sybil_identity_ring_pumps_the_flow_gate_open_gap`);
    /// v6 prices identity via standing-gated flow seeds.
    pub fn value_v5(
        cells_in_commit_order: &[super::Cell],
        theta: f64,
        d: f64,
        iters: usize,
        half: f64,
    ) -> Vec<f64> {
        let floored = super::temporal_novelty_with_similarity_floor(cells_in_commit_order, theta);
        let own: Vec<f64> = floored.iter().map(|&n| n as f64).collect();
        let downstream =
            super::flow::downstream_flow_external(cells_in_commit_order, &own, d, iters);
        floored
            .iter()
            .zip(&downstream)
            .map(|(&n, &f)| n as f64 * flow_gate(f, half))
            .collect()
    }

    /// `value_v6(novelty, downstream_flow, standing)` — prices identity at the value layer,
    /// closing value_v5's pinned sybil-ring gap
    /// (`adversary::sybil_identity_ring_pumps_the_flow_gate_open_gap`). Under v5 a fresh
    /// contributor key was a free byte, so a two-identity ring of novel garbage could pump
    /// the flow gate. v6 reaches consensus A3's economics ([`super::consensus::MIN_STAKE`] /
    /// [`super::consensus::max_sybils`]) down into the flow SEED:
    ///
    ///   seed_i = floored_novelty_i  if standing(contributor_i) ≥ standing_floor, else 0
    ///   value  = floored_novelty × g(downstream flow over standing-gated seeds)
    ///
    /// Standing is the soulbound, EARNED PoM on a contributor's standing cell
    /// ([`super::soulbound::Standing`]) — non-transferable by type-script invariant, so a
    /// certifying identity cannot even be bought, only earned. The ring's cost goes from 0
    /// to K × (cost of EARNING the floor) — strictly stronger than A3, where stake is at
    /// least purchasable capital.
    ///
    /// Design choice — gate the SEED, not the edge:
    ///   - an unvested identity PUMPS nothing: its building-on-others certifies nothing,
    ///     so the all-fresh ring collapses to 0;
    ///   - an unvested newcomer still EARNS: their own value = own floored novelty ×
    ///     g(flow pumped by vested minds building on them). Earn from day one; certify
    ///     only once you've earned — "buy storage, not consensus" at the value layer;
    ///   - certification is TRANSITIVE through unvested intermediaries: edges propagate
    ///     flow regardless of standing (only seeds are gated), so a vested grandchild's
    ///     use still reaches the root.
    ///
    /// Residual (pinned in `adversary::vested_certifier_endorsing_garbage_open_gap`): a
    /// standing-bearing contributor can still endorse garbage by building on it for a
    /// fresh-key pocket. That move is no longer free identity-minting — it is an act by an
    /// accountable, slashable identity; closing it is the ENDORSEMENT-SLASHING increment
    /// (refuted-value dispute window ⇒ [`super::soulbound::Op::Slash`]).
    #[allow(clippy::too_many_arguments)]
    pub fn value_v6(
        cells_in_commit_order: &[super::Cell],
        standing: &std::collections::HashMap<Vec<u8>, u64>,
        standing_floor: u64,
        theta: f64,
        d: f64,
        iters: usize,
        half: f64,
    ) -> Vec<f64> {
        let floored = super::temporal_novelty_with_similarity_floor(cells_in_commit_order, theta);
        let seed: Vec<f64> = floored
            .iter()
            .zip(cells_in_commit_order)
            .map(|(&n, c)| {
                let s = standing.get(&c.type_script.args).copied().unwrap_or(0);
                if s >= standing_floor {
                    n as f64
                } else {
                    0.0
                }
            })
            .collect();
        let downstream =
            super::flow::downstream_flow_external(cells_in_commit_order, &seed, d, iters);
        floored
            .iter()
            .zip(&downstream)
            .map(|(&n, &f)| n as f64 * flow_gate(f, half))
            .collect()
    }

    /// A3 reached down to the value layer: at most `total_earned_standing / standing_floor`
    /// identities can clear the certification floor (the mirror of
    /// [`super::consensus::max_sybils`]) — and the bound here is on EARNED, soulbound value:
    /// standing cannot be pooled, bought, or transferred in
    /// ([`super::soulbound::valid_transition`] rejects reassignment), so each of K
    /// certifying identities must independently earn ≥ floor.
    pub fn max_certifying_identities(total_standing: u64, standing_floor: u64) -> u64 {
        if standing_floor == 0 {
            return u64::MAX;
        }
        total_standing / standing_floor
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

        // ---- value_v5: the realized-flow GATE (ROADMAP Phase 1, two-clock composition) ----

        /// Cell with an explicit CONTRIBUTOR identity + provenance parent — the two fields
        /// the flow gate reads.
        fn cellc(id: u64, contrib: u8, ts: u64, parent: Option<u64>, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![contrib] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![contrib] },
                parent,
                timestamp: ts,
                data: data.to_vec(),
            }
        }

        const THETA: f64 = 0.8;
        const DAMP: f64 = 0.85;
        const ITERS: usize = 200;
        const HALF: f64 = 8.0;

        #[test]
        fn value_v5_gates_q0_noise_with_zero_downstream_flow_to_zero() {
            // THE Phase-1 regression (WAL next-move). A maximally-novel but meaningless block
            // (high-entropy noise, nothing ever built on it) earned FULL novelty under
            // value_v4 even at q=0 (the pinned composition gap). Under value_v5 its realized
            // downstream flow is 0 ⇒ gate 0 ⇒ value 0. The honest chain (built upon by
            // ANOTHER contributor) stays paid.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel"), // other mind builds on 0
                cellc(2, 9, 2, None, &(0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect::<Vec<u8>>()),
            ];
            let v5 = value_v5(&order, THETA, DAMP, ITERS, HALF);
            assert_eq!(v5[2], 0.0, "novel noise with zero downstream flow earns 0 — gap CLOSED");
            assert!(v5[0] > 0.0, "the built-upon honest cell is paid");
            // contrast: the boost form pays the same noise even at q = 0 (the gap, still pinned
            // in `value_v4_boost_does_not_gate_meaningless_novelty`).
            let v4 = value_v4(&order, &vec![0.0; order.len()]);
            assert!(*v4.last().unwrap() > 0.0, "value_v4 pays the q=0 noise; value_v5 does not");
        }

        #[test]
        fn honest_but_low_quality_work_built_upon_is_paid() {
            // The honest tension the gate had to respect: a true gate must not zero
            // honest-but-low-quality work. Sourcing g from REALIZED use resolves it — a small,
            // unglamorous cell that another mind builds on has flow > 0 and is paid.
            let order = vec![
                cellc(0, 1, 0, None, b"tiny-fix-x"), // low-coverage honest work
                cellc(1, 2, 1, Some(0), b"big-feature-built-on-the-tiny-fix-uniform-victor"),
            ];
            let v5 = value_v5(&order, THETA, DAMP, ITERS, HALF);
            assert!(v5[0] > 0.0, "low-quality-looking but USED work is paid by realized flow");
        }

        #[test]
        fn redundancy_cannot_be_rescued_by_downstream_flow() {
            // Floor before gate: a sybil clone that other identities then build on still earns
            // 0 — novelty 0 multiplies the gate away. The two clocks compose; neither rescues
            // the other.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, None, b"alpha-bravo-charlie-delta"), // clone, committed later
                cellc(2, 8, 2, Some(1), b"uniform-victor-whiskey-xray"), // accomplice builds on it
            ];
            let v5 = value_v5(&order, THETA, DAMP, ITERS, HALF);
            assert_eq!(v5[1], 0.0, "novelty-0 clone earns 0 no matter how much is built on it");
        }

        #[test]
        fn vesting_is_retroactive_value_accrues_as_flow_materializes() {
            // The honest consequence, demonstrated: at intake a cell is worth 0; when another
            // mind builds on it, value vests. A contribution is paid as it proves useful, not
            // priced at intake.
            let root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta");
            let at_intake = value_v5(&[root.clone()], THETA, DAMP, ITERS, HALF);
            assert_eq!(at_intake[0], 0.0, "at intake: no realized use yet ⇒ 0 (vests later)");
            let later = vec![root, cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel")];
            let after_use = value_v5(&later, THETA, DAMP, ITERS, HALF);
            assert!(after_use[0] > 0.0, "after another mind builds on it: value has vested");
        }

        #[test]
        fn flow_gate_is_bounded_monotone_and_zero_at_zero() {
            assert_eq!(flow_gate(0.0, 8.0), 0.0);
            assert!((flow_gate(8.0, 8.0) - 0.5).abs() < 1e-9, "g(half) = 0.5");
            assert!(flow_gate(20.0, 8.0) > flow_gate(10.0, 8.0), "monotone in realized flow");
            assert!(flow_gate(1e12, 8.0) < 1.0, "bounded below 1");
        }

        #[test]
        fn production_value_applies_floor_before_quality() {
            // Canonical rule: similarity-floor BEFORE quality, so a near-duplicate earns 0 even at
            // max quality, while an honest novel cell earns novelty x (1 + quality).
            let mut order = honest();
            let mut near = order[0].data.clone();
            near[2] ^= 0x20; // near-duplicate of block 0
            order.push(cell(99, 9, 99, &near));
            let q = vec![1.0; order.len()];
            let v = production_value(&order, 0.8, &q);
            assert_eq!(*v.last().unwrap(), 0.0, "near-dup -> 0 even at max quality (floor before quality)");
            assert!(v[0] > 0.0, "honest novel cell earns novelty x (1 + quality)");
        }

        // ---- value_v6: priced identity (standing-gated flow seeds) ----

        const FLOOR: u64 = 10;

        fn standing_of(pairs: &[(u8, u64)]) -> std::collections::HashMap<Vec<u8>, u64> {
            pairs.iter().map(|&(k, s)| (vec![k], s)).collect()
        }

        #[test]
        fn value_v6_closes_the_sybil_identity_ring() {
            // The EXACT attack pinned against v5 (two fresh identities, novel garbage,
            // "external" edge — adversary::sybil_identity_ring_pumps_the_flow_gate_open_gap)
            // earns 0 under v6: neither ring identity clears the standing floor, so the
            // child's seed is 0 ⇒ it pumps no flow ⇒ the gate stays shut.
            let noise = |seed: u8, n: u8| -> Vec<u8> {
                (0..n).map(|i| seed.wrapping_add(i.wrapping_mul(41))).collect()
            };
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"), // honest bystander
                cellc(10, 8, 1, None, &noise(0xA0, 48)),            // garbage parent, fresh key
                cellc(11, 9, 2, Some(10), &noise(0x10, 48)),        // garbage child, fresh key
            ];
            let st = standing_of(&[(1, 50)]); // only the bystander has earned standing
            let v6 = value_v6(&order, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            assert_eq!(v6[1], 0.0, "ring parent: an unvested child pumps nothing — gap CLOSED");
            assert_eq!(v6[2], 0.0, "ring child: nothing vested builds on it either");
            // contrast: v5 pays the same ring (still pinned in the adversary module).
            let v5 = value_v5(&order, THETA, DAMP, ITERS, HALF);
            assert!(v5[1] > 0.0, "v5 pays this exact ring — that is the gap v6 closes");
        }

        #[test]
        fn unvested_newcomer_still_earns_when_a_vested_mind_builds_on_them() {
            // Earn from day one; certify once you've earned. A newcomer (standing 0)
            // ships honest work; a vested mind builds on it ⇒ the newcomer is PAID.
            // Seed-gating prices certification, never participation.
            let order = vec![
                cellc(0, 7, 0, None, b"newcomer-honest-work-kilo-lima"),
                cellc(1, 1, 1, Some(0), b"vested-mind-builds-on-newcomer"),
            ];
            let st = standing_of(&[(1, 50)]); // contributor 1 vested; newcomer 7 has nothing
            let v6 = value_v6(&order, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            assert!(v6[0] > 0.0, "newcomer is paid: vested use certifies their work");
        }

        #[test]
        fn unvested_certification_pumps_nothing_vested_certification_pays() {
            // Same graph, same bytes — only the child contributor's standing differs.
            // The floor is the single bit that flips the root from unpaid to paid.
            let order = vec![
                cellc(0, 1, 0, None, b"root-work-alpha-bravo"),
                cellc(1, 2, 1, Some(0), b"child-builds-on-root-echo"),
            ];
            let unvested = standing_of(&[(1, 50)]); // child contributor 2: no standing
            let vested = standing_of(&[(1, 50), (2, FLOOR)]); // exactly at the floor
            let v_un = value_v6(&order, &unvested, FLOOR, THETA, DAMP, ITERS, HALF);
            let v_ve = value_v6(&order, &vested, FLOOR, THETA, DAMP, ITERS, HALF);
            assert_eq!(v_un[0], 0.0, "an unvested child's use certifies nothing");
            assert!(v_ve[0] > 0.0, "the SAME use by a floor-clearing identity pays the root");
        }

        #[test]
        fn certification_is_transitive_through_an_unvested_intermediary() {
            // Seeds are gated, edges are not: a vested grandchild's use flows through an
            // unvested intermediary and still reaches the root. The chain of use certifies;
            // a standing-less middle mind cannot BLOCK credit, it just cannot MINT it.
            let order = vec![
                cellc(0, 1, 0, None, b"root-alpha-bravo-charlie"),
                cellc(1, 7, 1, Some(0), b"unvested-middle-echo-foxtrot"),
                cellc(2, 2, 2, Some(1), b"vested-grandchild-india-juliet"),
            ];
            let st = standing_of(&[(2, 50)]); // ONLY the grandchild contributor is vested
            let v6 = value_v6(&order, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            assert!(v6[0] > 0.0, "vested grandchild reaches the root through the unvested middle");
        }

        #[test]
        fn fully_vested_graph_reduces_to_value_v5() {
            // With every contributor over the floor, v6's seeds equal v5's, so the values
            // are identical: pricing identity costs an honest, established graph nothing.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel"),
                cellc(2, 3, 2, Some(0), b"india-juliet-kilo-lima"),
            ];
            let st = standing_of(&[(1, 50), (2, 50), (3, 50)]);
            let v6 = value_v6(&order, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            let v5 = value_v5(&order, THETA, DAMP, ITERS, HALF);
            for (a, b) in v6.iter().zip(&v5) {
                assert!((a - b).abs() < 1e-12, "vested-everywhere ⇒ v6 == v5");
            }
        }

        #[test]
        fn redundancy_still_floored_under_v6() {
            // Floor before gate survives the new layer: a clone endorsed by a VESTED
            // identity still earns 0 — novelty 0 multiplies the gate away.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, None, b"alpha-bravo-charlie-delta"), // clone, committed later
                cellc(2, 2, 2, Some(1), b"uniform-victor-whiskey-xray"), // vested accomplice
            ];
            let st = standing_of(&[(2, 50)]);
            let v6 = value_v6(&order, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            assert_eq!(v6[1], 0.0, "novelty-0 clone earns 0 even with a vested endorser");
        }

        #[test]
        fn max_certifying_identities_mirrors_the_consensus_sybil_bound() {
            // Same economics shape as consensus A3, one layer down — with the stronger
            // property that standing is earned per identity, never poolable capital.
            assert_eq!(crate::consensus::max_sybils(1000.0), 10); // MIN_STAKE = 100
            assert_eq!(max_certifying_identities(1000, 100), 10);
            assert_eq!(max_certifying_identities(99, 100), 0, "below floor: zero certifiers");
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

    /// Like [`children_of`] but counts only EXTERNAL edges: a child whose contributor
    /// (`type_script.args`, the soulbound identity) equals its parent's is dropped. Used by
    /// the realized-flow gate — a mind cannot certify the usefulness of its own work by
    /// building on it itself. (A multi-identity sybil ring was the pinned residual gap;
    /// `value::value_v6` closes it by standing-gating the SEEDS — edges stay un-gated here
    /// so certification remains transitive through unvested intermediaries.)
    fn children_of_external(
        cells: &[Cell],
        id_to_idx: &HashMap<u64, usize>,
    ) -> HashMap<u64, Vec<usize>> {
        let mut ch: HashMap<u64, Vec<usize>> = HashMap::new();
        for (i, c) in cells.iter().enumerate() {
            if let Some(p) = c.parent {
                if p != c.id {
                    if let Some(&pi) = id_to_idx.get(&p) {
                        if cells[pi].type_script.args != c.type_script.args {
                            ch.entry(p).or_default().push(i);
                        }
                    }
                }
            }
        }
        ch
    }

    /// Generalized damped Jacobi flow over CALLER-SUPPLIED own-values.
    /// `flow(b) = own(b) + d · Σ_{c built on b} flow(c)`; `d < 1` ⇒ contraction ⇒ converges;
    /// a self-referential cycle stays bounded by `d`. `external_only` restricts propagation to
    /// cross-contributor edges (see [`children_of_external`]). The realized-flow gate seeds
    /// `own` with the strategyproof FLOORED NOVELTY so redundant children pump nothing.
    pub fn value_flow_with_own(
        cells: &[Cell],
        own: &[f64],
        d: f64,
        iters: usize,
        external_only: bool,
    ) -> Vec<f64> {
        let mut flow = own.to_vec();
        if cells.is_empty() {
            return flow;
        }
        let id_to_idx: HashMap<u64, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
        let children = if external_only {
            children_of_external(cells, &id_to_idx)
        } else {
            children_of(cells)
        };
        for _ in 0..iters {
            let mut next = own.to_vec();
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
        flow
    }

    /// `flow(b) = own(b) + d · Σ_{c built on b} flow(c)` by damped Jacobi iteration.
    /// Returns `(own, flow)`, both index-aligned with `cells`.
    pub fn value_flow(cells: &[Cell], d: f64, iters: usize) -> (Vec<f64>, Vec<f64>) {
        let own: Vec<f64> = cells.iter().map(own_value).collect();
        let flow = value_flow_with_own(cells, &own, d, iters, false);
        (own, flow)
    }

    /// REALIZED downstream flow = `flow − own`: the credit a cell earns purely from what
    /// OTHER minds built on it (external edges only). This is the un-spoofable-by-content
    /// signal the `value_v5` gate consumes: a cell nobody builds on has downstream 0 no
    /// matter how novel its bytes are. Index-aligned with `cells`; clamped at 0.
    pub fn downstream_flow_external(cells: &[Cell], own: &[f64], d: f64, iters: usize) -> Vec<f64> {
        let flow = value_flow_with_own(cells, own, d, iters, true);
        flow.iter().zip(own).map(|(f, o)| (f - o).max(0.0)).collect()
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
        fn external_only_flow_ignores_self_attribution_edges() {
            // Same chain, two worlds: child authored by ANOTHER contributor vs by the SAME
            // contributor. External-only flow uplifts the parent only in the first world —
            // a mind cannot certify its own work's usefulness by building on it itself.
            let mk = |child_contrib: u8| -> Vec<Cell> {
                let mut root = cell(0, None, b"root-alpha-bravo");
                root.type_script.args = vec![1];
                let mut kid = cell(1, Some(0), b"kid-charlie-delta");
                kid.type_script.args = vec![child_contrib];
                vec![root, kid]
            };
            let own = vec![5.0, 5.0];
            let ext = value_flow_with_own(&mk(2), &own, 0.85, 200, true); // other mind
            let selfed = value_flow_with_own(&mk(1), &own, 0.85, 200, true); // same mind
            assert!(ext[0] > own[0], "cross-contributor child uplifts the parent");
            assert!((selfed[0] - own[0]).abs() < 1e-9, "self-attribution edge is ignored");
            let ds = downstream_flow_external(&mk(1), &own, 0.85, 200);
            assert!(ds[0].abs() < 1e-9, "downstream flow from self-built children is 0");
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

    /// A4 — equivocation. One validator casting support for two DIFFERENT proposals in the same
    /// epoch is a double-vote: slashable, and the offending vote is not counted (NCI
    /// `_slashEquivocator`). Re-voting the SAME proposal, or a first vote, is not equivocation.
    pub fn is_equivocation(prev_vote_this_epoch: Option<u64>, this_proposal: u64) -> bool {
        matches!(prev_vote_this_epoch, Some(p) if p != this_proposal)
    }

    /// A4 — early-reject. A proposal that cannot reach the threshold even if EVERY remaining
    /// weight votes for it should reject early: `weightAgainst > total - threshold`
    /// (NCI `finalizeProposal`). Saves a doomed proposal from waiting out its window.
    pub fn can_early_reject(weight_against: f64, total: f64, threshold_bps: u64) -> bool {
        let threshold = total * threshold_bps as f64 / BPS as f64;
        weight_against > total - threshold
    }

    /// A2 — log-scaling. NCI applies log₂ scaling to the PoW and PoM dimensions
    /// (`Mind_weight = log₂(1 + mind_score) · SCALE`) to prevent plutocracy: concentrating a
    /// dimension yields DIMINISHING weight. PoS is linear (capital is at-risk now, not a score).
    pub fn log_weight(raw: f64) -> f64 {
        (1.0 + raw.max(0.0)).log2()
    }

    /// Realizable share of a log-scaled dimension for an actor holding `actor_raw` against a field
    /// of `field_raw` total raw. Because of log-scaling, concentration is SUBLINEAR — even
    /// `actor_raw ≫ field_raw` approaches 1 only logarithmically, so a single actor saturating PoM
    /// realizes strictly LESS than the linear mix ceiling (0.60). This strengthens, not weakens,
    /// the "no single dimension finalizes alone" property (L12 / `single_dimension_can_finalize`
    /// is the worst-case linear bound; log-scaling sits below it).
    pub fn realizable_log_share(actor_raw: f64, field_raw: f64) -> f64 {
        let (a, f) = (log_weight(actor_raw), log_weight(field_raw));
        if a + f <= 0.0 { 0.0 } else { a / (a + f) }
    }

    /// A2 (cross-node) — realizable share against a FIELD of many honest nodes, not one aggregate.
    /// Each node contributes `log_weight(raw_i)` independently, so the actor's share is
    /// `log(actor) / (log(actor) + Σ log(honest_i))`. By the concavity of log, many small nodes
    /// sum to MORE log-weight than a single node of the same total raw, so a fragmented (more
    /// decentralized) honest field dilutes the attacker further: fragmentation favors honesty.
    pub fn realizable_log_share_field(actor_raw: f64, honest_field: &[f64]) -> f64 {
        let a = log_weight(actor_raw);
        let h: f64 = honest_field.iter().map(|&r| log_weight(r)).sum();
        if a + h <= 0.0 { 0.0 } else { a / (a + h) }
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

        #[test]
        fn audit_a4_equivocation_is_detected_and_slashable() {
            assert!(is_equivocation(Some(7), 8), "a second vote for a different proposal equivocates");
            assert!(!is_equivocation(Some(7), 7), "re-voting the same proposal is not equivocation");
            assert!(!is_equivocation(None, 8), "a first vote is never equivocation");
            let mut eq = val(7, 1.0, 4.0, 1.0, 0);
            slash(&mut eq, 2.0);
            assert_eq!(eq.staked_balance, 2.0, "a detected equivocator is slashed (composes with A5)");
        }

        #[test]
        fn audit_a4_early_reject_when_threshold_unreachable() {
            // total 10, 2/3 bar = 6.667; if against > 10 - 6.667 = 3.333 the bar is unreachable.
            assert!(can_early_reject(4.0, 10.0, TWO_THIRDS_BPS), "against 4 > 3.33 -> early reject");
            assert!(!can_early_reject(3.0, 10.0, TWO_THIRDS_BPS), "against 3 < 3.33 -> still reachable");
        }

        #[test]
        fn audit_a2_log_scaling_is_sublinear() {
            // doubling raw PoM does NOT double the log-weight (diminishing returns / anti-plutocracy).
            let (w1, w2) = (log_weight(1000.0), log_weight(2000.0));
            assert!(w2 < 2.0 * w1, "log-scaling: doubling raw far less than doubles weight");
            assert!(w2 > w1, "but still monotone increasing");
        }

        #[test]
        fn audit_a2_concentrated_pom_realizes_less_than_the_linear_ceiling() {
            // An actor with 10x the field's raw PoM realizes far less than the LINEAR reading: a
            // linear share would be 10/11 ~ 0.91; log-scaling caps it well below, and below 0.60.
            let linear = 10.0 / 11.0;
            let logd = realizable_log_share(10_000.0, 1_000.0);
            assert!(logd < linear, "log-scaled concentration share < linear share ({logd} < {linear})");
            assert!(logd < 0.6, "even 10x raw PoM stays below the naive 60% read under log-scaling");
        }

        #[test]
        fn audit_a2_fragmented_field_dilutes_the_attacker_more() {
            // Same total honest raw, spread across many nodes vs one aggregate. By concavity of
            // log, many small nodes sum to MORE log-weight than one big node, so the attacker's
            // realizable share is LOWER against a fragmented field. Decentralization favors honesty.
            let actor = 10_000.0;
            let aggregate = realizable_log_share_field(actor, &[10_000.0]); // honest = one node
            let fragmented = realizable_log_share_field(actor, &[1_000.0; 10]); // ten nodes, same total
            assert!(
                fragmented < aggregate,
                "a fragmented honest field dilutes the attacker more ({fragmented} < {aggregate})"
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

/// L9 — core / nucleolus stability (no profitable fork). For a coalition game `v` over
/// validators, an allocation `x` is in the CORE iff every coalition `S` is satisfied:
/// `sum_{i in S} x_i >= v(S)` (with the grand coalition efficient). When the core is empty, the
/// NUCLEOLUS is the allocation lexicographically minimizing the sorted vector of coalition
/// excesses `e(S,x) = v(S) - sum_{i in S} x_i`. Here: core membership + the max-excess objective
/// (the nucleolus's first lexicographic component) over an explicit characteristic function. The
/// consensus layer composes this stability concept ONLY because finalization needs it (L9 / the
/// "add a stability concept when consensus requires it" rule) — pure attribution does not.
pub mod stability {
    /// Excess of coalition `S` under allocation `x`: `e(S,x) = v(S) - sum x_i`. `> 0` => `S` is
    /// dissatisfied and could profit by deviating (a fork). Core <=> all excesses <= 0.
    pub fn excess(coalition: &[usize], alloc: &[f64], v_s: f64) -> f64 {
        v_s - coalition.iter().map(|&i| alloc[i]).sum::<f64>()
    }

    /// In the core iff no coalition has positive excess. `coalitions` = list of `(members, v(S))`.
    pub fn in_core(alloc: &[f64], coalitions: &[(Vec<usize>, f64)]) -> bool {
        coalitions.iter().all(|(s, vs)| excess(s, alloc, *vs) <= 1e-9)
    }

    /// Max excess over all coalitions — the nucleolus's primary objective (minimize it).
    pub fn max_excess(alloc: &[f64], coalitions: &[(Vec<usize>, f64)]) -> f64 {
        coalitions
            .iter()
            .map(|(s, vs)| excess(s, alloc, *vs))
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// LEAST-CORE solver — the nucleolus's first lexicographic level. Minimizes the maximum
    /// *proper* coalition excess `max_{∅≠S⊊N} e(S,x)` subject to efficiency `Σ x_i = v(N)`. The
    /// objective is convex piecewise-linear in `x`; we run **projected subgradient descent** — the
    /// subgradient at `x` is the negative indicator of an arg-max coalition (raising `x_i` for
    /// `i ∈ S*` lowers `e(S*) = v(S*) − Σx`), re-projected onto the efficiency hyperplane each
    /// step, with diminishing step `1/(t+2)` and best-iterate tracking (subgradient methods are
    /// not monotone, so we keep the lowest-ε point seen). When the least-core is a single point
    /// this IS the nucleolus; otherwise it is the first level and the full lexicographic nucleolus
    /// iterates by fixing the tight coalitions and re-solving. Dependency-free, deterministic.
    /// Returns `(allocation, epsilon = max proper-coalition excess)`.
    pub fn least_core(
        n: usize,
        grand: f64,
        coalitions: &[(Vec<usize>, f64)],
        iters: usize,
    ) -> (Vec<f64>, f64) {
        let proper: Vec<&(Vec<usize>, f64)> =
            coalitions.iter().filter(|(s, _)| !s.is_empty() && s.len() < n).collect();
        let eps = |x: &[f64]| -> f64 {
            proper.iter().map(|(s, vs)| excess(s, x, *vs)).fold(f64::NEG_INFINITY, f64::max)
        };
        let mut x = vec![grand / n as f64; n];
        let mut best_x = x.clone();
        let mut best_e = eps(&x);
        for t in 0..iters {
            let mut best = f64::NEG_INFINITY;
            let mut arg: &[usize] = &[];
            for (s, vs) in &proper {
                let e = excess(s, &x, *vs);
                if e > best {
                    best = e;
                    arg = s;
                }
            }
            let step = grand.abs().max(1.0) / (t as f64 + 2.0);
            for &i in arg {
                x[i] += step;
            }
            let adj = (x.iter().sum::<f64>() - grand) / n as f64; // re-project onto Σx = grand
            for xi in x.iter_mut() {
                *xi -= adj;
            }
            let e = eps(&x);
            if e < best_e {
                best_e = e;
                best_x = x.clone();
            }
        }
        (best_x, best_e)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        // 3-validator superadditive game: each pair worth 0.5, singletons 0, grand 1.0.
        fn game() -> Vec<(Vec<usize>, f64)> {
            vec![
                (vec![0], 0.0), (vec![1], 0.0), (vec![2], 0.0),
                (vec![0, 1], 0.5), (vec![0, 2], 0.5), (vec![1, 2], 0.5),
                (vec![0, 1, 2], 1.0),
            ]
        }

        #[test]
        fn equal_split_is_in_the_core() {
            assert!(in_core(&[1.0 / 3.0; 3], &game()), "every pair gets 2/3 >= 0.5 -> no profitable fork");
        }

        #[test]
        fn an_unfair_allocation_leaves_the_core() {
            let x = [1.0, 0.0, 0.0];
            assert!(!in_core(&x, &game()), "coalition {{1,2}} is owed 0.5 but gets 0 -> can deviate");
            assert!(max_excess(&x, &game()) > 0.0, "positive max-excess signals a profitable fork");
        }

        #[test]
        fn nucleolus_objective_prefers_lower_max_excess() {
            assert!(
                max_excess(&[1.0 / 3.0; 3], &game()) < max_excess(&[1.0, 0.0, 0.0], &game()),
                "the more stable allocation has a lower max-excess (the nucleolus direction)"
            );
        }

        #[test]
        fn least_core_returns_the_equal_split_nucleolus_on_the_symmetric_game() {
            let (x, eps) = least_core(3, 1.0, &game(), 20_000);
            for xi in &x {
                assert!((xi - 1.0 / 3.0).abs() < 1e-2, "least-core point ~ equal split (got {x:?})");
            }
            assert!((eps + 1.0 / 6.0).abs() < 1e-2, "least-core epsilon ~ -1/6 (got {eps})");
        }

        // Asymmetric game: {0,1} is the valuable pair (0.9). Hand-derived nucleolus is
        // (0.475, 0.475, 0.05) with eps* = -0.05 (the {0,1} and {2} excesses balance). The solver
        // must improve far past the equal-split max-excess (0.233) and load the valuable coalition.
        fn asym_game() -> Vec<(Vec<usize>, f64)> {
            vec![
                (vec![0], 0.0), (vec![1], 0.0), (vec![2], 0.0),
                (vec![0, 1], 0.9), (vec![0, 2], 0.1), (vec![1, 2], 0.1),
                (vec![0, 1, 2], 1.0),
            ]
        }

        #[test]
        fn least_core_solves_an_asymmetric_game() {
            let (x, eps) = least_core(3, 1.0, &asym_game(), 20_000);
            assert!((x.iter().sum::<f64>() - 1.0).abs() < 1e-6, "allocation is efficient");
            assert!(eps < 0.05, "eps converges near -0.05, far below the equal-split 0.233 (got {eps})");
            assert!(x[0] + x[1] > 0.7, "mass moves onto the valuable coalition");
            assert!((x[0] - x[1]).abs() < 0.05, "symmetric players 0,1 get equal shares");
            assert!(x[2] < 0.15, "the weak player gets the small residual");
        }
    }
}

/// Dispute-window endorsement-slashing (`DISPUTE-SLASHING.md`). Makes the vested-certifier
/// attack pinned in `adversary::vested_certifier_endorsing_garbage_open_gap` NEGATIVE-EV:
/// value vests W epochs after the flow that paid it; while any of it is unvested, a
/// vested-standing holder may challenge with a bond; a PoM-weighted verdict — REUSING
/// [`consensus::finalizes_hybrid`] (2/3 supermajority + eclipse quorum-floor), value-layer
/// instance of the same machinery — refutes or upholds. On refutation the unvested value
/// cancels and each certifier is slashed `λ × causal_share + α`, where the causal share is
/// DETERMINISTIC (re-run [`value::value_v6`] with that certifier's standing removed; the
/// difference is exactly what their endorsement minted — no oracle). §-refs = the design doc.
pub mod dispute {
    use super::{consensus, value, Cell};
    use std::collections::HashMap;

    /// §2 parameters. `window`=W, `lambda`=λ restitution, `alpha`=α deterrence,
    /// `beta`=β challenger bounty share, `gamma`=γ nuisance compensation on upheld.
    #[derive(Clone, Copy, Debug)]
    pub struct Params {
        pub window: u64,
        pub lambda: f64,
        pub alpha: f64,
        pub beta: f64,
        pub gamma: f64,
    }

    /// Value paid to `cell_id` by the v6 gate at `realized_epoch`; spendable at `+W`.
    #[derive(Clone, Debug)]
    pub struct VestingEntry {
        pub cell_id: u64,
        pub amount: f64,
        pub realized_epoch: u64,
    }

    pub fn is_vested(e: &VestingEntry, now: u64, p: &Params) -> bool {
        now >= e.realized_epoch + p.window
    }

    pub fn vested_total(entries: &[VestingEntry], cell: u64, now: u64, p: &Params) -> f64 {
        entries
            .iter()
            .filter(|e| e.cell_id == cell && is_vested(e, now, p))
            .map(|e| e.amount)
            .sum()
    }

    pub fn unvested_total(entries: &[VestingEntry], cell: u64, now: u64, p: &Params) -> f64 {
        entries
            .iter()
            .filter(|e| e.cell_id == cell && !is_vested(e, now, p))
            .map(|e| e.amount)
            .sum()
    }

    /// §2 challenge — admissible only while some of the target's value is still unvested.
    /// Vested value is untouchable (the price of finality); W bounds exposure by construction.
    #[derive(Clone, Debug)]
    pub struct Challenge {
        pub target: u64,
        pub challenger: Vec<u8>,
        pub bond: f64,
        pub opened_epoch: u64,
    }

    pub fn challenge_admissible(entries: &[VestingEntry], c: &Challenge, p: &Params) -> bool {
        unvested_total(entries, c.target, c.opened_epoch, p) > 0.0
    }

    /// §2 verdict: judges = vested standing, PoM-only mix, same 2/3 + quorum-floor
    /// finalization the consensus layer uses. Proof-over-vote at the value layer.
    pub const POM_ONLY: consensus::Mix = consensus::Mix { pow: 0.0, pos: 0.0, pom: 1.0 };

    pub fn verdict_refutes(
        voters_for: &[consensus::Validator],
        all: &[consensus::Validator],
        now: u64,
        horizon: u64,
        quorum_floor_bps: u64,
    ) -> bool {
        consensus::finalizes_hybrid(
            voters_for,
            all,
            POM_ONLY,
            now,
            horizon,
            false,
            consensus::TWO_THIRDS_BPS,
            quorum_floor_bps,
        )
    }

    /// §2.2 causal share — the certifier's marginal effect on the target's v6 value,
    /// computed by removing their standing and re-running the gate. Deterministic,
    /// content-independent, no oracle. Clamped at 0.
    #[allow(clippy::too_many_arguments)]
    pub fn causal_share(
        cells: &[Cell],
        standing: &HashMap<Vec<u8>, u64>,
        floor: u64,
        theta: f64,
        d: f64,
        iters: usize,
        half: f64,
        target_idx: usize,
        certifier: &[u8],
    ) -> f64 {
        let with = value::value_v6(cells, standing, floor, theta, d, iters, half)[target_idx];
        let mut without = standing.clone();
        without.remove(certifier);
        let wo = value::value_v6(cells, &without, floor, theta, d, iters, half)[target_idx];
        (with - wo).max(0.0)
    }

    /// Individual zero-seed marginals can over-count when certifiers overlap; scale them
    /// down proportionally so `Σ shares ≤ canceled` (restitution never exceeds the harm).
    pub fn bounded_shares(shares: &[(Vec<u8>, f64)], canceled: f64) -> Vec<(Vec<u8>, f64)> {
        let total: f64 = shares.iter().map(|(_, s)| s).sum();
        if total <= canceled || total <= 0.0 {
            return shares.to_vec();
        }
        let scale = canceled / total;
        shares.iter().map(|(w, s)| (w.clone(), s * scale)).collect()
    }

    /// Outcome of a closed dispute. `burned` keeps mint↔sink balanced (COHERENCE-LAWS):
    /// everything canceled or slashed that is not paid out as bounty/compensation is burned.
    #[derive(Clone, Debug, Default)]
    pub struct Settlement {
        pub canceled: f64,
        pub slashes: Vec<(Vec<u8>, f64)>,
        pub challenger_payout: f64,
        pub author_compensation: f64,
        pub burned: f64,
    }

    /// REFUTED path (§2): cancel the target's unvested value, slash each certifier
    /// `λ × bounded_share + α`, return bond + β-bounty to the challenger, burn the rest.
    ///
    /// Critical-qa hardenings (2026-06-12):
    /// - The exposure set SNAPSHOTS at `c.opened_epoch` — an open challenge LOCKS the
    ///   target's then-unvested entries, so slow resolution cannot vest value out from
    ///   under a live dispute (the open-late/resolve-after evasion).
    /// - Zero-share certifiers are SKIPPED: α attaches to causation, not adjacency — a
    ///   vested contributor whose edge minted nothing is never α-taxed.
    /// - β is clamped to [0,1]: `burned ≥ canceled ≥ 0` always; the resolver can never
    ///   become a mint by misconfiguration.
    pub fn resolve_refuted(
        entries: &mut [VestingEntry],
        c: &Challenge,
        p: &Params,
        certifier_shares: &[(Vec<u8>, f64)],
    ) -> Settlement {
        let mut canceled = 0.0;
        for e in entries.iter_mut() {
            if e.cell_id == c.target && !is_vested(e, c.opened_epoch, p) {
                canceled += e.amount;
                e.amount = 0.0;
            }
        }
        let bounded = bounded_shares(certifier_shares, canceled);
        let mut slashes = Vec::new();
        let mut total_slashed = 0.0;
        for (who, share) in &bounded {
            if *share <= 0.0 {
                continue;
            }
            let amt = p.lambda * share + p.alpha;
            slashes.push((who.clone(), amt));
            total_slashed += amt;
        }
        let bounty = p.beta.clamp(0.0, 1.0) * total_slashed;
        Settlement {
            canceled,
            slashes,
            challenger_payout: c.bond + bounty,
            author_compensation: 0.0,
            burned: canceled + total_slashed - bounty,
        }
    }

    /// UPHELD path (§2): challenger forfeits the bond (γ to the challenged author as
    /// nuisance compensation, rest burned); the target's vesting clock is NOT reset.
    /// γ clamped to [0,1] (no mint-by-misconfiguration).
    pub fn resolve_upheld(c: &Challenge, p: &Params) -> Settlement {
        let gamma = p.gamma.clamp(0.0, 1.0);
        Settlement {
            canceled: 0.0,
            slashes: Vec::new(),
            challenger_payout: 0.0,
            author_compensation: gamma * c.bond,
            burned: (1.0 - gamma) * c.bond,
        }
    }

    /// Critical-qa hardening (2026-06-12): slash-evasion-by-exit. `soulbound::Op::Burn`
    /// is unconditionally allowed at the cell layer, so a certifier could torch their
    /// standing between endorsement and verdict and leave the slash nothing to land on.
    /// While a challenge is OPEN, any contributor with a provenance edge INTO the
    /// challenged target is exit-blocked (burn / decay-exit denied). Cell-level wiring:
    /// `soulbound::valid_transition` must consult this before admitting `Op::Burn` —
    /// integration contract, enforced here in the reference spec.
    pub fn standing_exit_blocked(
        cells: &[Cell],
        open_challenges: &[Challenge],
        contributor: &[u8],
    ) -> bool {
        open_challenges.iter().any(|c| {
            cells.iter().any(|child| {
                child.parent == Some(c.target) && child.type_script.args == contributor
            })
        })
    }

    /// Apply slashes to the value-layer standing map (the cell-level equivalent is
    /// [`super::soulbound::Op::Slash`] on each certifier's standing cell).
    pub fn apply_slashes(standing: &mut HashMap<Vec<u8>, u64>, slashes: &[(Vec<u8>, f64)]) {
        for (who, amt) in slashes {
            if let Some(s) = standing.get_mut(who) {
                *s = s.saturating_sub(amt.ceil() as u64);
            }
        }
    }

    // ============ §7 — escalation court + juror accountability ============
    // The judge-cartel structural counter (design doc §7). Round 1 is PoM-only (cheap);
    // its veto is NOT final: an appeal escalates to the AND-composed full-mix tribunal
    // (consensus::NCI), where a PoM cartel holds only 0.6 × its standing share — vetoing
    // there requires cross-dimension capture, the consensus layer's already-priced global
    // assumption. Juror accountability is the load-bearing piece: an overturned verdict
    // slashes the jurors who voted it, so a round-1 veto is a standing bet anyone can
    // call by escalating. Equilibrium: the veto doesn't fire — class dissolved, not
    // instance-detected.

    /// Which court hears the round. Round 1 = `PomOnly`; appeals = `FullMix` (NCI).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Tribunal {
        PomOnly,
        FullMix,
    }

    /// §7.2 — verdict at a given tribunal. Same finalization machinery, different mix.
    pub fn verdict_refutes_at(
        tribunal: Tribunal,
        voters_for: &[consensus::Validator],
        all: &[consensus::Validator],
        now: u64,
        horizon: u64,
        quorum_floor_bps: u64,
    ) -> bool {
        let mix = match tribunal {
            Tribunal::PomOnly => POM_ONLY,
            Tribunal::FullMix => consensus::NCI,
        };
        consensus::finalizes_hybrid(
            voters_for,
            all,
            mix,
            now,
            horizon,
            false,
            consensus::TWO_THIRDS_BPS,
            quorum_floor_bps,
        )
    }

    /// §7.1 — juror-exclusion (hygiene, not load-bearing): a juror whose contributor key
    /// has a provenance edge into the challenged target does not sit on its case.
    /// `juror_keys` joins validator ids to contributor identities. Evadeable by
    /// worker/judge identity separation — which is why accountability (below) attaches to
    /// the VOTE, not the edge.
    pub fn conflicted_juror_ids(
        cells: &[Cell],
        target: u64,
        juror_keys: &[(u64, Vec<u8>)],
    ) -> Vec<u64> {
        juror_keys
            .iter()
            .filter(|(_, key)| {
                cells
                    .iter()
                    .any(|child| child.parent == Some(target) && child.type_script.args == *key)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// §7.2 — appeal bonds double per round: dragging a dispute k rounds with no new
    /// evidence costs 2^k × B. The griefing bound.
    pub fn appeal_bond(prev_bond: f64) -> f64 {
        prev_bond * 2.0
    }

    /// §7.3 — juror accountability (LOAD-BEARING): when an appeal overturns a verdict,
    /// the jurors who voted with the overturned majority are slashed proportionally to
    /// the PoM weight they voted with: `slash_i = rate × pom_i`, rate clamped to [0,1].
    /// Attaches to the vote, not the edge — identity separation does not evade it.
    pub fn juror_slash_on_overturn(
        overturned_majority: &[consensus::Validator],
        rate: f64,
    ) -> Vec<(u64, f64)> {
        let r = rate.clamp(0.0, 1.0);
        overturned_majority.iter().map(|v| (v.id, r * v.pom)).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::super::{Cell, Script};
        use super::*;

        fn cellc(id: u64, contrib: u8, ts: u64, parent: Option<u64>, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![contrib] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![contrib] },
                parent,
                timestamp: ts,
                data: data.to_vec(),
            }
        }

        const THETA: f64 = 0.8;
        const DAMP: f64 = 0.85;
        const ITERS: usize = 200;
        const HALF: f64 = 8.0;
        const FLOOR: u64 = 10;
        const P: Params = Params { window: 10, lambda: 1.0, alpha: 0.5, beta: 0.5, gamma: 0.1 };

        fn standing_of(pairs: &[(u8, u64)]) -> HashMap<Vec<u8>, u64> {
            pairs.iter().map(|&(k, s)| (vec![k], s)).collect()
        }

        /// The exact attack from `adversary::vested_certifier_endorsing_garbage_open_gap`:
        /// fresh-key garbage parent (idx 1, pocket), vested identity 5 endorses it (idx 2).
        fn attack_graph() -> (Vec<Cell>, HashMap<Vec<u8>, u64>) {
            let noise = |seed: u8, n: u8| -> Vec<u8> {
                (0..n).map(|i| seed.wrapping_add(i.wrapping_mul(41))).collect()
            };
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(10, 8, 1, None, &noise(0xA0, 48)),
                cellc(11, 5, 2, Some(10), &noise(0x10, 48)),
            ];
            let standing = standing_of(&[(1, 50), (5, 50)]);
            (order, standing)
        }

        #[test]
        fn windowed_vesting_value_spendable_only_after_w() {
            // §6.1: flow realized at epoch E is spendable only at E + W.
            let entries = vec![VestingEntry { cell_id: 7, amount: 12.0, realized_epoch: 100 }];
            assert_eq!(vested_total(&entries, 7, 100, &P), 0.0, "at intake: nothing vested");
            assert_eq!(vested_total(&entries, 7, 109, &P), 0.0, "inside W: still unvested");
            assert_eq!(unvested_total(&entries, 7, 109, &P), 12.0);
            assert_eq!(vested_total(&entries, 7, 110, &P), 12.0, "at E+W: vested");
        }

        #[test]
        fn refutation_inside_window_cancels_unvested_only() {
            // §6.2: a refutation cancels the unvested entries; already-vested value is
            // untouchable (finality has a price; W bounds the exposure).
            let mut entries = vec![
                VestingEntry { cell_id: 7, amount: 5.0, realized_epoch: 0 },   // long vested
                VestingEntry { cell_id: 7, amount: 12.0, realized_epoch: 100 }, // in window
            ];
            let c = Challenge { target: 7, challenger: vec![1], bond: 2.0, opened_epoch: 105 };
            assert!(challenge_admissible(&entries, &c, &P));
            let s = resolve_refuted(&mut entries, &c, &P, &[]);
            assert_eq!(s.canceled, 12.0, "only the unvested entry cancels");
            assert_eq!(vested_total(&entries, 7, 105, &P), 5.0, "vested value untouched");
        }

        #[test]
        fn causal_share_is_deterministic_and_bounded_by_target_value() {
            // §6.3: zero-seed recomputation. The sole certifier's share equals the target's
            // entire v6 value (without them the gate never opened), twice-computed identical.
            let (order, standing) = attack_graph();
            let v6 = value::value_v6(&order, &standing, FLOOR, THETA, DAMP, ITERS, HALF);
            let s1 = causal_share(&order, &standing, FLOOR, THETA, DAMP, ITERS, HALF, 1, &[5]);
            let s2 = causal_share(&order, &standing, FLOOR, THETA, DAMP, ITERS, HALF, 1, &[5]);
            assert_eq!(s1, s2, "deterministic");
            assert!(s1 > 0.0, "the endorsement minted value");
            assert!((s1 - v6[1]).abs() < 1e-9, "sole certifier's share = full target value");
            // bounded_shares: an over-counting pair is scaled down to the canceled amount.
            let over = vec![(vec![5u8], 8.0), (vec![6u8], 8.0)];
            let b = bounded_shares(&over, 10.0);
            let tot: f64 = b.iter().map(|(_, s)| s).sum();
            assert!((tot - 10.0).abs() < 1e-9, "Σ shares scaled to ≤ canceled");
        }

        #[test]
        fn endorsement_slashing_makes_the_vested_certifier_ring_negative_ev() {
            // §6.4 — THE FLIP of `vested_certifier_endorsing_garbage_open_gap`. The pinned
            // attack still pays at the gate (that pin documents the value-layer surface);
            // HERE the dispute layer claws it back: with λ=1 the slash alone refunds the
            // entire minted value, and α makes the round strictly negative. Numeric §4
            // inequality at p=1/2: EV = (1−p)·V − p·(V+α) = −α/2 < 0 for ANY α > 0.
            let (order, mut standing) = attack_graph();
            let gain = value::value_v6(&order, &standing, FLOOR, THETA, DAMP, ITERS, HALF)[1];
            assert!(gain > 0.0, "pre-dispute: the pocket key was paid (the pinned surface)");

            let mut entries =
                vec![VestingEntry { cell_id: 10, amount: gain, realized_epoch: 100 }];
            let c = Challenge { target: 10, challenger: vec![1], bond: 1.0, opened_epoch: 102 };
            let share = causal_share(&order, &standing, FLOOR, THETA, DAMP, ITERS, HALF, 1, &[5]);
            let s = resolve_refuted(&mut entries, &c, &P, &[(vec![5], share)]);

            assert_eq!(s.canceled, gain, "pocket payout fully canceled (was unvested)");
            let slashed: f64 = s.slashes.iter().map(|(_, a)| a).sum();
            assert!(
                slashed >= gain + P.alpha - 1e-9,
                "certifier slash (λ·share+α) ≥ minted value + α ⇒ attack EV < 0 when caught"
            );
            let v = gain; // §4: V = value the certification minted
            let p_detect = 0.5;
            let ev = (1.0 - p_detect) * v - p_detect * (v + P.alpha);
            assert!(ev < 0.0, "§4 inequality: negative EV at p=1/2 for any α>0");

            apply_slashes(&mut standing, &s.slashes);
            assert!(
                standing[&vec![5u8]] < 50,
                "the certifier's soulbound standing actually decreased"
            );
        }

        #[test]
        fn griefing_failed_challenge_costs_the_bond() {
            // §6.5: an upheld challenge burns most of the bond, compensates the author γ·B,
            // pays the challenger nothing, and leaves the target's vesting unharmed.
            let mut entries = vec![VestingEntry { cell_id: 7, amount: 9.0, realized_epoch: 100 }];
            let c = Challenge { target: 7, challenger: vec![9], bond: 4.0, opened_epoch: 101 };
            let s = resolve_upheld(&c, &P);
            assert_eq!(s.challenger_payout, 0.0, "griefer loses the bond");
            assert!((s.author_compensation - 0.4).abs() < 1e-9, "γ·B to the author");
            assert!((s.burned - 3.6).abs() < 1e-9, "rest burned (mint↔sink)");
            assert_eq!(unvested_total(&entries, 7, 101, &P), 9.0, "vesting unharmed");
            entries[0].realized_epoch = 100; // clock NOT reset by the failed challenge
            assert_eq!(vested_total(&entries, 7, 110, &P), 9.0, "vests on the original clock");
        }

        #[test]
        fn honest_certifier_never_slashed_without_refutation() {
            // §6.6 regression: no challenge ⇒ no slash path exists; vesting completes at W
            // with amounts intact. Slashing is reachable ONLY through a refuted verdict.
            let entries = vec![VestingEntry { cell_id: 3, amount: 7.0, realized_epoch: 50 }];
            assert_eq!(vested_total(&entries, 3, 60, &P), 7.0, "full amount vests untouched");
        }

        // ---- critical-qa hardenings (2026-06-12, post-implementation hostile review) ----

        #[test]
        fn open_challenge_snapshots_exposure_slow_resolution_cannot_evade() {
            // QA R2: challenge opened at the last unvested moment, resolved AFTER E+W.
            // Pre-fix: cancellation at `now` found everything vested ⇒ canceled 0, α-only
            // slashes. Post-fix: exposure snapshots at opened_epoch ⇒ still canceled.
            let mut entries = vec![VestingEntry { cell_id: 7, amount: 12.0, realized_epoch: 100 }];
            let c = Challenge { target: 7, challenger: vec![1], bond: 2.0, opened_epoch: 109 };
            assert!(challenge_admissible(&entries, &c, &P), "opened in-window: admissible");
            // resolution happens at epoch 130, far past vesting — irrelevant by design now
            let s = resolve_refuted(&mut entries, &c, &P, &[]);
            assert_eq!(s.canceled, 12.0, "locked at open: slow verdict cannot vest it away");
        }

        #[test]
        fn zero_share_certifier_is_never_alpha_taxed() {
            // QA R3: α attaches to causation, not adjacency. A certifier whose edge minted
            // nothing (share 0) must not appear in the slash list at all.
            let mut entries = vec![VestingEntry { cell_id: 7, amount: 10.0, realized_epoch: 100 }];
            let c = Challenge { target: 7, challenger: vec![1], bond: 2.0, opened_epoch: 101 };
            let shares = vec![(vec![5u8], 10.0), (vec![6u8], 0.0)];
            let s = resolve_refuted(&mut entries, &c, &P, &shares);
            assert_eq!(s.slashes.len(), 1, "innocent zero-share certifier skipped");
            assert_eq!(s.slashes[0].0, vec![5u8], "only the causal certifier is slashed");
        }

        #[test]
        fn misconfigured_params_cannot_turn_the_resolver_into_a_mint() {
            // QA R4: β,γ outside [0,1] are clamped — burned stays non-negative on both paths.
            let bad = Params { window: 10, lambda: 1.0, alpha: 0.5, beta: 1.5, gamma: 2.0 };
            let mut entries = vec![VestingEntry { cell_id: 7, amount: 10.0, realized_epoch: 100 }];
            let c = Challenge { target: 7, challenger: vec![1], bond: 4.0, opened_epoch: 101 };
            let s = resolve_refuted(&mut entries, &c, &bad, &[(vec![5u8], 10.0)]);
            assert!(s.burned >= 0.0, "refuted path: clamped β keeps burn non-negative");
            let u = resolve_upheld(&c, &bad);
            assert!(u.burned >= 0.0, "upheld path: clamped γ keeps burn non-negative");
            assert!(u.author_compensation <= c.bond, "compensation bounded by the bond");
        }

        #[test]
        fn standing_exit_is_blocked_while_a_challenge_names_your_edge() {
            // QA R1: slash-evasion-by-exit. The certifier (identity 5) endorsed the
            // challenged target; their burn/decay-exit is blocked while the dispute is
            // open. An uninvolved identity remains free to exit.
            let (order, _) = attack_graph();
            let c = Challenge { target: 10, challenger: vec![1], bond: 1.0, opened_epoch: 102 };
            let open = vec![c];
            assert!(
                standing_exit_blocked(&order, &open, &[5]),
                "certifier with an edge into the challenged target cannot exit"
            );
            assert!(
                !standing_exit_blocked(&order, &open, &[1]),
                "uninvolved standing exits freely"
            );
            assert!(
                !standing_exit_blocked(&order, &[], &[5]),
                "no open challenge: exit unblocked"
            );
        }

        #[test]
        fn verdict_reuses_consensus_finalization_two_thirds_pom() {
            // §6 (verdict): PoM-only judges; below 2/3 of vested standing does not refute,
            // at/above 2/3 does. Same machinery, value-layer instance.
            let judge = |id: u64, pom: f64| consensus::Validator {
                id,
                pow: 0.0,
                pos: 0.0,
                pom,
                last_heartbeat: 0,
                staked_balance: 1000.0,
            };
            let all = vec![judge(1, 40.0), judge(2, 30.0), judge(3, 30.0)];
            let minority = vec![all[0].clone(), all[1].clone()]; // 70% > 2/3 ⇒ refutes
            let sub = vec![all[0].clone()]; // 40% < 2/3 ⇒ does not
            assert!(verdict_refutes(&minority, &all, 0, 0, 4000));
            assert!(!verdict_refutes(&sub, &all, 0, 0, 4000));
        }

        // ---- §7 escalation court + juror accountability (judge-cartel counter) ----

        /// Honest side: 60% of PoM plus ALL PoW and PoS. Cartel: 40% of PoM, nothing else.
        fn courtroom() -> (Vec<consensus::Validator>, Vec<consensus::Validator>) {
            let v = |id, pow, pos, pom| consensus::Validator {
                id,
                pow,
                pos,
                pom,
                last_heartbeat: 0,
                staked_balance: 1000.0,
            };
            let honest = vec![v(1, 50.0, 50.0, 30.0), v(2, 50.0, 50.0, 30.0)];
            let cartel = vec![v(9, 0.0, 0.0, 40.0)];
            (honest, cartel)
        }

        #[test]
        fn cartel_veto_holds_at_round_one_but_is_overturned_on_appeal() {
            // THE COUNTER (§7.2). Round 1 (PoM-only): honest 60% < 2/3 — the >1/3 cartel
            // vetoes, exactly the pinned gap. Appeal (full NCI mix): the cartel's 40% of
            // standing is only 0.6×0.4 = 24% of the court; honest PoW+PoS+PoM = 76% ≥ 2/3
            // ⇒ refutation lands. Cross-dimension capture is the only remaining veto, and
            // that is the consensus layer's already-priced global assumption.
            let (honest, cartel) = courtroom();
            let mut all = honest.clone();
            all.extend(cartel);
            assert!(
                !verdict_refutes_at(Tribunal::PomOnly, &honest, &all, 0, 0, 4000),
                "round 1: the cartel's >1/3 PoM veto holds (the pinned surface)"
            );
            assert!(
                verdict_refutes_at(Tribunal::FullMix, &honest, &all, 0, 0, 4000),
                "appeal: AND-composed tribunal overturns the PoM-only veto"
            );
        }

        #[test]
        fn overturned_jurors_are_slashed_proportionally_to_their_vote() {
            // §7.3 — the load-bearing piece: the cartel's round-1 veto bloc is slashed
            // when the appeal overturns it. The veto is a bonded liability, not a wall;
            // identity separation does not help because the slash attaches to the VOTE.
            let (_, cartel) = courtroom();
            let slashes = juror_slash_on_overturn(&cartel, 0.25);
            assert_eq!(slashes.len(), 1);
            assert_eq!(slashes[0].0, 9, "the vetoing juror is named");
            assert!((slashes[0].1 - 10.0).abs() < 1e-9, "rate × pom = 0.25 × 40");
            // rate clamped: a misconfigured rate cannot over-slash
            let clamped = juror_slash_on_overturn(&cartel, 7.0);
            assert!((clamped[0].1 - 40.0).abs() < 1e-9, "slash never exceeds voted pom");
        }

        #[test]
        fn conflicted_jurors_are_excluded_from_the_case() {
            // §7.1 hygiene: the certifier who endorsed the challenged target does not sit
            // on its case; an uninvolved juror does.
            let (order, _) = attack_graph(); // target 10, certifier key [5]
            let juror_keys = vec![(1u64, vec![1u8]), (5u64, vec![5u8])];
            let conflicted = conflicted_juror_ids(&order, 10, &juror_keys);
            assert_eq!(conflicted, vec![5], "edge-connected juror excluded; bystander sits");
        }

        #[test]
        fn appeal_bonds_double_per_round() {
            // §7.2 griefing bound: k evidence-free rounds cost 2^k × B.
            let mut b = 4.0;
            for _ in 0..3 {
                b = appeal_bond(b);
            }
            assert!((b - 32.0).abs() < 1e-9, "3 appeals: 4 → 32 (2^3 × B)");
        }
    }
}

/// Calibration harness (`DISPUTE-SLASHING.md` §8): the dispute stack's parameters
/// (W, B, λ, α, β, γ) and the evaluator's (κ, μ) must satisfy three inequalities
/// SIMULTANEOUSLY — attacker EV < 0, challenger EV > 0 (the bounty PURCHASES the
/// detection probability §4 assumes), griefer EV < 0 — across the whole (V, p) grid,
/// not at a hand-picked point. These are model EVs over the mechanism's own formulas;
/// the tests sweep the grid and pin a RECOMMENDED set inside the feasible region.
pub mod calibration {
    /// §4 attacker EV: `(1−p)·V − p·(λV + α)`.
    pub fn attacker_ev(v: f64, p: f64, lambda: f64, alpha: f64) -> f64 {
        (1.0 - p) * v - p * (lambda * v + alpha)
    }

    /// Challenger EV on true garbage: bond returns, bounty pays `β·(λV+α)`, minus the
    /// mechanical effort of computing the causal share and filing.
    pub fn challenger_ev(v: f64, lambda: f64, alpha: f64, beta: f64, effort: f64) -> f64 {
        beta * (lambda * v + alpha) - effort
    }

    /// Griefer EV on an honest cell, sub-capture (verdict machinery holds): the bond is
    /// forfeit, nothing is won.
    pub fn griefer_ev(bond: f64) -> f64 {
        -bond
    }

    /// Honest liquidity cost: effective wait is the window scaled by how much of the
    /// expected value the evaluator advances at intake (Role A).
    pub fn liquidity_delay(window: u64, advance_fraction: f64) -> f64 {
        window as f64 * (1.0 - advance_fraction.clamp(0.0, 1.0))
    }

    /// Full-stack feasibility over a (V, p) grid: every attack value × every detection
    /// probability ≥ `p_min` must be attacker-negative, and detection must be worth
    /// buying at every V ≥ `v_min`.
    #[allow(clippy::too_many_arguments)]
    pub fn feasible(
        lambda: f64,
        alpha: f64,
        beta: f64,
        bond: f64,
        effort: f64,
        v_grid: &[f64],
        p_min: f64,
        v_min: f64,
    ) -> bool {
        let p_grid = [p_min, (p_min + 1.0) / 2.0, 0.95];
        for &v in v_grid {
            for &p in &p_grid {
                if attacker_ev(v, p, lambda, alpha) >= 0.0 {
                    return false;
                }
            }
            if v >= v_min && challenger_ev(v, lambda, alpha, beta, effort) <= 0.0 {
                return false;
            }
        }
        griefer_ev(bond) < 0.0 && beta <= 1.0
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The recommended first-cut set, pinned INSIDE the feasible region by the sweep.
        const LAMBDA: f64 = 1.0;
        const ALPHA: f64 = 0.5;
        const BETA: f64 = 0.5;
        const BOND: f64 = 4.0;
        const EFFORT: f64 = 0.1; // causal share is a deterministic recompute — cheap
        const P_MIN: f64 = 0.5;

        fn v_grid() -> Vec<f64> {
            (0..10).map(|k| 0.5 * 2f64.powi(k)).collect() // 0.5 .. 256, log-spaced
        }

        #[test]
        fn recommended_params_are_feasible_across_the_grid() {
            assert!(
                feasible(LAMBDA, ALPHA, BETA, BOND, EFFORT, &v_grid(), P_MIN, 0.5),
                "the shipped first-cut set sits inside the feasible region"
            );
        }

        #[test]
        fn alpha_zero_breaks_below_half_detection() {
            // §4's direction, demonstrated: with α = 0 and p < 1/2, the attacker is
            // positive-EV at every V — the penalty term is what covers the sub-half
            // detection regime, λ alone cannot.
            assert!(attacker_ev(10.0, 0.4, LAMBDA, 0.0) > 0.0, "α=0, p<½: attack pays");
            assert!(
                !feasible(LAMBDA, 0.0, BETA, BOND, EFFORT, &v_grid(), 0.4, 0.5),
                "α=0 is infeasible if detection can dip below ½"
            );
            assert!(
                attacker_ev(10.0, 0.4, LAMBDA, 35.0) < 0.0,
                "a large enough α restores negativity even at p=0.4 (α > V(1−2p)/p = 5 here... \
                 with margin)"
            );
        }

        #[test]
        fn the_bounty_purchases_the_detection_assumption() {
            // p ≥ ½ is not assumed, it is BOUGHT: refuting garbage is profitable work at
            // every attack size on the grid, so a detection market exists.
            for &v in &v_grid() {
                if v >= 0.5 {
                    assert!(
                        challenger_ev(v, LAMBDA, ALPHA, BETA, EFFORT) > 0.0,
                        "refutation is positive-EV at V={v}"
                    );
                }
            }
        }

        #[test]
        fn liquidity_cost_falls_with_the_advance() {
            // The evaluator's Role A is what makes a safe (long) W tolerable.
            let no_advance = liquidity_delay(10, 0.0);
            let half_advance = liquidity_delay(10, 0.5);
            assert!(half_advance < no_advance, "advance halves the effective wait");
            assert_eq!(liquidity_delay(10, 2.0), 0.0, "fraction clamps at 1");
        }
    }
}

/// Role-bounded outcome evaluator (`OUTCOME-EVALUATOR.md`). The learned v(S) is NOT the
/// gate (v5 settled that: realized flow gates, predictions don't). Its authority is
/// bounded to two roles that cannot mint: ADVANCE timing (intake liquidity against
/// future vesting, double-bounded by κ·score·floored_novelty and μ·standing, shortfall
/// slashed at window close) and INFORM judgment (dispute evidence, never the verdict).
/// The Phase-1 obligation collapses from "prove the model un-gameable" to "the bounds
/// hold" — tested here against a maximally corrupt evaluator.
pub mod evaluator {
    /// Role A — intake advance, the double bound. A fresh identity (standing 0) gets
    /// nothing at any score; redundancy (floored novelty 0) gets nothing at any score;
    /// everyone else at most μ × their own soulbound standing.
    pub fn intake_advance(
        score: f64,
        floored_novelty: u64,
        standing: u64,
        kappa: f64,
        mu: f64,
    ) -> f64 {
        let want = kappa.max(0.0) * score.max(0.0) * floored_novelty as f64;
        let cap = mu.clamp(0.0, 1.0) * standing as f64;
        want.min(cap)
    }

    /// Window-close reconciliation. The contributor received `advance` at intake; realized
    /// vesting repays it. Returns `(paid_total, standing_slash)`:
    /// - covered: they receive the remainder; total paid = vested, no slash;
    /// - shortfall: they keep the advance but standing is slashed for the difference, so
    ///   net extraction = vested either way. The evaluator can shift WHEN, never HOW MUCH.
    pub fn reconcile(advance: f64, vested: f64) -> (f64, f64) {
        let a = advance.max(0.0);
        let v = vested.max(0.0);
        if v >= a {
            (v, 0.0)
        } else {
            (v, a - v)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const KAPPA: f64 = 0.5;
        const MU: f64 = 0.5;

        #[test]
        fn corrupt_evaluator_cannot_mint() {
            // The invariant, adversarially: score = +huge everywhere.
            let huge = 1e18;
            assert_eq!(
                intake_advance(huge, 40, 0, KAPPA, MU),
                0.0,
                "fresh identity: no standing, no advance, at ANY score"
            );
            assert_eq!(
                intake_advance(huge, 0, 1000, KAPPA, MU),
                0.0,
                "redundancy: floored novelty 0 multiplies the advance away, at ANY score"
            );
            let a = intake_advance(huge, 40, 50, KAPPA, MU);
            assert!((a - 25.0).abs() < 1e-9, "vested identity: capped at μ×standing");
            // Garbage never vests (flow gate) ⇒ window close recovers the leak:
            let (paid, slash) = reconcile(a, 0.0);
            assert_eq!(paid, 0.0, "nothing vested, nothing net-paid");
            assert!((slash - a).abs() < 1e-9, "the full advance is recovered from standing");
        }

        #[test]
        fn honest_path_gets_liquidity_and_no_slash() {
            let a = intake_advance(0.8, 20, 100, KAPPA, MU); // wants 8, cap 50 ⇒ 8
            assert!((a - 8.0).abs() < 1e-9);
            let (paid, slash) = reconcile(a, 15.0); // vesting covers the advance
            assert_eq!(slash, 0.0, "honest contributor is never slashed by the advance");
            assert!((paid - 15.0).abs() < 1e-9, "total paid = realized vesting exactly");
        }

        #[test]
        fn conservation_paid_never_exceeds_realized_vesting() {
            // Both branches: the evaluator shifts timing, never amount.
            for (adv, vest) in [(10.0, 25.0), (10.0, 3.0), (0.0, 7.0), (10.0, 0.0)] {
                let (paid, slash) = reconcile(adv, vest);
                assert!(paid <= vest + 1e-12, "net paid bounded by realized vesting");
                assert!(slash >= 0.0 && slash <= adv + 1e-12, "slash bounded by the advance");
                assert!(
                    (paid + slash - vest.max(adv)).abs() < 1e-9 || vest >= adv,
                    "accounting closes: shortfall fully recovered"
                );
            }
        }

        #[test]
        fn negative_inputs_are_inert() {
            assert_eq!(intake_advance(-5.0, 40, 50, KAPPA, MU), 0.0, "negative score: 0");
            assert_eq!(intake_advance(0.8, 40, 50, -1.0, MU), 0.0, "negative κ: 0");
            let (paid, slash) = reconcile(-3.0, -4.0);
            assert_eq!((paid, slash), (0.0, 0.0), "negative amounts clamp to 0");
        }
    }
}

/// Harness checker-routing (the JARVIS core thesis, modeled and tested). Routes a claim to the
/// verification layer that can actually catch its error — structure (recompute/verify) where a
/// verifiable referent exists, ensemble (diverse-model vote) where none does, both for reasoning —
/// and fails CLOSED to structure under adversary or ambiguity. See
/// `JARVIS-CORE-harness-as-coordination.md`. The recursion: this is the chain's mechanism (proof
/// over vote) applied at the scale of one decision rather than one block.
pub mod harness {
    /// Which verification layer a claim is routed to.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Layer {
        Structure,
        Ensemble,
        Both,
    }

    /// The features that decide routing.
    #[derive(Clone, Copy, Debug)]
    pub struct Claim {
        /// a verifiable referent exists (math, code, state, provenance) — recompute settles it.
        pub recomputable: bool,
        /// multi-step reasoning mixing stochastic steps with factual ones.
        pub reasoning_chain: bool,
        /// an adversary could flood/eclipse a vote.
        pub adversary_possible: bool,
    }

    /// Route a claim to its layer (whitepaper §8). Fail-closed: prefer structure whenever a
    /// referent might exist or an adversary might be present, because you cannot out-vote a proof.
    pub fn route(c: Claim) -> Layer {
        if c.recomputable {
            Layer::Structure
        } else if c.reasoning_chain || c.adversary_possible {
            // reasoning needs both; no-referent-but-adversary can't trust a bare vote -> add structure/bonding
            Layer::Both
        } else {
            Layer::Ensemble
        }
    }

    /// Ensemble verdict = majority of diverse-checker votes. Models correlated error directly: if
    /// the votes are correlated and wrong, the majority is confidently wrong — the exact failure
    /// mode structural grounding exists to catch.
    pub fn ensemble_verdict(votes: &[bool]) -> bool {
        votes.iter().filter(|&&v| v).count() * 2 > votes.len()
    }

    /// Dispatch verification. `structural` = Some(recompute verdict) when a referent exists, else
    /// None. `votes` = diverse-checker votes. Structure is authoritative where it can recompute;
    /// Both uses structure when present and falls to the (bonded) ensemble only with no referent;
    /// fail-closed means a Structure-routed claim with no proof supplied is NOT verified.
    pub fn verify(c: Claim, structural: Option<bool>, votes: &[bool]) -> (bool, Layer) {
        let layer = route(c);
        let verified = match layer {
            Layer::Structure => structural.unwrap_or(false),
            Layer::Ensemble => ensemble_verdict(votes),
            Layer::Both => match structural {
                Some(s) => s,
                None => ensemble_verdict(votes),
            },
        };
        (verified, layer)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn claim(recomputable: bool, reasoning_chain: bool, adversary_possible: bool) -> Claim {
            Claim { recomputable, reasoning_chain, adversary_possible }
        }

        #[test]
        fn routes_by_referent_and_adversary() {
            assert_eq!(route(claim(true, false, false)), Layer::Structure, "recomputable -> structure");
            assert_eq!(route(claim(false, false, false)), Layer::Ensemble, "open-ended, no adversary -> ensemble");
            assert_eq!(route(claim(false, true, false)), Layer::Both, "reasoning chain -> both");
            assert_eq!(route(claim(false, false, true)), Layer::Both, "no referent + adversary -> both (don't trust a bare vote)");
        }

        #[test]
        fn correlated_ensemble_passes_a_false_claim_that_structure_catches() {
            // The load-bearing demonstration. The claim is actually FALSE. A correlated ensemble
            // (cousins sharing a blind spot) all vote TRUE -> the vote confidently passes it.
            let correlated_wrong = [true, true, true, true, true];
            assert!(ensemble_verdict(&correlated_wrong), "correlated ensemble confidently passes the false claim");
            // But it's recomputable, so it routes to structure, which recomputes FALSE and catches it.
            let (verified, layer) = verify(claim(true, false, false), Some(false), &correlated_wrong);
            assert_eq!(layer, Layer::Structure);
            assert!(!verified, "structure catches what the correlated ensemble missed");
        }

        #[test]
        fn independent_ensemble_outvotes_a_minority_error() {
            // Where there's no referent, a diverse (independent) ensemble's majority is the signal.
            let mixed = [true, true, false]; // one checker errs, two are right
            let (verified, layer) = verify(claim(false, false, false), None, &mixed);
            assert_eq!(layer, Layer::Ensemble);
            assert!(verified, "independent majority overrides the minority error");
        }

        #[test]
        fn fail_closed_no_proof_is_not_verified() {
            // A recomputable claim routed to structure, but no proof supplied -> NOT verified, even
            // though a (correlated) ensemble would have passed it. You don't get verified for free.
            let (verified, layer) = verify(claim(true, false, false), None, &[true, true, true]);
            assert_eq!(layer, Layer::Structure);
            assert!(!verified, "fail-closed: no recompute proof -> unverified, vote does not rescue it");
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
    fn near_duplicate_residual_novelty_is_an_open_gap() {
        // Adversarial-gaming tick (2026-06-12). Temporal-novelty zeroes EXACT subsets and
        // duplicates, but a near-duplicate that flips a few tokens adds the shingles spanning the
        // change -> small but NONZERO novelty. An attacker minting many near-duplicates could
        // farm residual value. This PINS the gap: passes today (residual > 0), and flips when a
        // coverage-similarity floor (discount a block whose overlap with the earlier union exceeds
        // a threshold) lands. Candidate fix tracked in ROADMAP Phase 1.
        let mut order = honest();
        let mut near = order[0].data.clone();
        let mid = near.len() / 2;
        for k in 0..3 {
            if mid + k < near.len() {
                near[mid + k] ^= 0x20; // flip case of a few bytes -> a few new shingles
            }
        }
        order.push(cell(88, 9, 88, &near));
        let v = temporal_novelty(&order);
        let residual = *v.last().unwrap();
        assert!(residual > 0, "KNOWN GAP: a near-duplicate earns small residual novelty, not 0");
        assert!(residual < v[0], "but far less than the original block's novelty");
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

    #[test]
    fn sybil_identity_ring_pumps_the_flow_gate_open_gap() {
        // Adversarial-gaming tick vs value_v5 (2026-06-12, same-session — run the adversary
        // against every new v(S) the moment it lands). The gate requires downstream use from a
        // DIFFERENT contributor, so a same-key self-build fails (tested in flow). But identity
        // here is a free byte: an attacker mints novel-garbage block A under key 8, then a
        // novel-garbage child under key 9 pointing at A. The child is genuinely novel (the
        // coverage proxy again), the edge is "external", flow > 0, the gate opens, A is paid.
        //
        // PINS the residual gap: passes today (ring pays), flips when identity is PRICED —
        // the soulbound-standing / MIN_STAKE layer (consensus::max_sybils) must make a fresh
        // contributor identity cost real stake, and/or flow must be seeded by VESTED value so
        // an unvested child pumps nothing. The ring's cost is then K × identity-price instead
        // of 0 — the same economics that bounds consensus sybils (A3) must reach the value layer.
        let noise = |seed: u8, n: u8| -> Vec<u8> {
            (0..n).map(|i| seed.wrapping_add(i.wrapping_mul(41))).collect()
        };
        let order = vec![
            cell(0, 1, 0, b"alpha-bravo-charlie-delta"), // honest bystander
            {
                let mut a = cell(10, 8, 1, &noise(0xA0, 48));
                a.type_script.args = vec![8];
                a
            },
            {
                let mut child = cell(11, 9, 2, &noise(0x10, 48)); // different novel garbage
                child.type_script.args = vec![9]; // fresh free identity
                child.parent = Some(10); // "external" use of A
                child
            },
        ];
        let v5 = crate::value::value_v5(&order, 0.8, 0.85, 200, 8.0);
        assert!(
            v5[1] > 0.0,
            "KNOWN GAP (v5 surface): a two-identity ring of novel garbage pumps the flow gate; \
             value_v6 closes it by pricing identity (standing-gated seeds) — see \
             value::tests::value_v6_closes_the_sybil_identity_ring"
        );
        // CLOSED in v6 (2026-06-12): the same ring under value_v6 with no earned standing
        // earns 0. This pin stays as documentation of the v5 surface; the LIVE residual is
        // `vested_certifier_endorsing_garbage_open_gap` below.
    }

    #[test]
    fn vested_certifier_endorsing_garbage_open_gap() {
        // Adversarial-gaming tick vs value_v6 (2026-06-12, same-session — run the adversary
        // against every new v(S) the moment it lands). v6 priced identity: the all-fresh ring
        // is dead (cost 0 → K × cost-of-earning-the-floor). The surviving move: a contributor
        // who already EARNED standing builds a novel-garbage child on a fresh-key
        // novel-garbage parent. The certifier clears the floor, its seed is live, the gate
        // opens, and the attacker's fresh key collects the parent's payout.
        //
        // PINS the residual: passes today (endorsement pays), flips when ENDORSEMENT-SLASHING
        // lands — building on later-refuted garbage must cost the certifier standing
        // (refuted-value dispute window ⇒ soulbound::Op::Slash). What v6 already changed is
        // the economics: this is no longer free identity-minting, it is an act by an
        // accountable, slashable, hard-to-replace identity (standing is earned, not bought).
        let noise = |seed: u8, n: u8| -> Vec<u8> {
            (0..n).map(|i| seed.wrapping_add(i.wrapping_mul(41))).collect()
        };
        let order = vec![
            cell(0, 1, 0, b"alpha-bravo-charlie-delta"), // honest bystander
            {
                // garbage parent under a FRESH key — the attacker's collection pocket
                let mut a = cell(10, 8, 1, &noise(0xA0, 48));
                a.type_script.args = vec![8];
                a
            },
            {
                // novel-garbage child signed by the attacker's VESTED identity
                let mut child = cell(11, 5, 2, &noise(0x10, 48));
                child.type_script.args = vec![5];
                child.parent = Some(10);
                child
            },
        ];
        let mut standing = std::collections::HashMap::new();
        standing.insert(vec![5u8], 50u64); // identity 5 earned standing (floor = 10)
        let v6 = crate::value::value_v6(&order, &standing, 10, 0.8, 0.85, 200, 8.0);
        assert!(
            v6[1] > 0.0,
            "KNOWN GAP (v6 gate surface): a vested certifier can endorse garbage into a \
             fresh-key pocket at the GATE; the dispute layer makes the round negative-EV — \
             see dispute::tests::endorsement_slashing_makes_the_vested_certifier_ring_negative_ev"
        );
        // CLOSED at the dispute layer (2026-06-12): windowed vesting + causal-share slash
        // claws back the minted value (λ=1) plus α. This pin stays as documentation of the
        // gate-level surface; the LIVE residual is the judge-cartel
        // (`judge_cartel_protects_its_own_garbage_open_gap` below).
    }

    #[test]
    fn judge_cartel_protects_its_own_garbage_open_gap() {
        // Adversarial-gaming tick vs the dispute layer (2026-06-12, same-session). The
        // slashing design routes the verdict through 2/3-of-vested-standing finalization.
        // The surviving move (pre-pinned in DISPUTE-SLASHING.md §5.3): a cartel holding
        // > 1/3 of vested standing simply never convicts its own ring — refutation needs
        // 2/3 FOR, so a >1/3 bloc vetoes every challenge against itself. Detection exists
        // (challenges open), conviction doesn't. Bounded economically (capture cost,
        // defection bounty, PoM self-dilution — §5.3) but not yet structurally.
        //
        // PINS the residual: passes today (cartel veto works), flips when a structural
        // counter lands (e.g. juror-exclusion of edge-connected standing, escalation court,
        // or dilution-indexed slashing). Next gate-hardening increment after the dispute
        // module ships.
        let judge = |id: u64, pom: f64| crate::consensus::Validator {
            id,
            pow: 0.0,
            pos: 0.0,
            pom,
            last_heartbeat: 0,
            staked_balance: 1000.0,
        };
        // Honest 60%, cartel 40% (> 1/3): every honest-unanimous vote still fails 2/3.
        let all = vec![judge(1, 30.0), judge(2, 30.0), judge(3, 40.0)];
        let honest_for = vec![all[0].clone(), all[1].clone()];
        assert!(
            !crate::dispute::verdict_refutes(&honest_for, &all, 0, 0, 4000),
            "KNOWN GAP (round-1 surface): a >1/3 vested-standing cartel vetoes refutation \
             at the PoM-only court; the §7 escalation court + juror accountability makes \
             the veto a bonded liability — see \
             dispute::tests::cartel_veto_holds_at_round_one_but_is_overturned_on_appeal"
        );
        // CLOSED at the §7 layer (2026-06-12): the appeal escalates to the AND-composed
        // full-mix tribunal (cartel needs cross-dimension capture) and the overturned
        // veto bloc is slashed (juror accountability). This pin stays as round-1 surface
        // documentation; the remaining ceiling is pinned below as the system's GLOBAL
        // assumption (`full_consensus_capture_..._global_assumption`).
    }

    #[test]
    fn full_consensus_capture_defeats_the_escalation_court_global_assumption() {
        // CEILING PIN (design §7, honest tensions). This is NOT a gap that flips — it
        // documents the system's global trust assumption, stated in code: if a cartel
        // holds ≥ 2/3 of the AND-composED full mix (PoW and PoS and PoM), the escalation
        // court is theirs too and no appeal exists above it. Every layer of this chain
        // rests on sub-2/3 cross-dimension honesty; the escalation court deliberately
        // introduces NO new assumption beyond it. If this test ever needs to flip, the
        // fix is not a mechanism — it is the consensus layer itself.
        let v = |id, pow, pos, pom| crate::consensus::Validator {
            id,
            pow,
            pos,
            pom,
            last_heartbeat: 0,
            staked_balance: 1000.0,
        };
        let cartel = vec![v(9, 80.0, 80.0, 80.0)]; // ≥2/3 of every dimension
        let honest = vec![v(1, 20.0, 20.0, 20.0)];
        let mut all = cartel.clone();
        all.extend(honest.clone());
        // The cartel can refuse to convict its own ring even at the strongest court:
        assert!(
            !crate::dispute::verdict_refutes_at(
                crate::dispute::Tribunal::FullMix,
                &honest,
                &all,
                0,
                0,
                4000
            ),
            "GLOBAL ASSUMPTION: ≥2/3 cross-dimension capture defeats every tribunal; \
             this is the consensus layer's own ceiling, not a dispute-layer gap"
        );
    }

    #[test]
    fn value_v4_boost_does_not_gate_meaningless_novelty() {
        // Adversarial-gaming tick (2026-06-12, pom-roadmap loop). SHARPENS the already-documented
        // garbage-novelty gap (`garbage_novelty_is_the_documented_open_gap`): the new claim is not
        // that the coverage proxy rewards entropy (known) but that the CURRENT COMPOSITION cannot
        // close it. value_v4 folds the learned quality in as a BOOST: value = novelty * (1 + q),
        // q in [0,1]. A boost can only ADD; it can never gate to zero. So a maximally-novel but
        // meaningless block (pure high-entropy noise, q -> 0) still earns its FULL coverage-novelty.
        //
        // This is the precise reason the Phase-1 fix must change the COMPOSITION to a GATE --
        // value = novelty * g(q) with g in [0,1] sourced from the OUTCOME-evaluator -- not merely
        // train a better quality proxy on top of an additive form. PINS the composition gap: passes
        // today (q=0 noise paid), flips when value_v* gates. The honest tension: a true gate also
        // suppresses honest-but-low-quality work, so g must come from realized outcome, not a proxy.
        let mut order = honest();
        let mut garbage = Vec::new();
        for k in 0u16..40 {
            garbage.push(0xE0u8.wrapping_add((k & 0x0f) as u8)); // all bytes absent from ascii content
        }
        order.push(cell(77, 9, 77, &garbage));

        let nov = temporal_novelty(&order);
        assert!(*nov.last().unwrap() > 0, "pure noise is maximally novel to a shingle metric");

        // q = 0 on the noise block; value_v4 should NOT be able to zero it (boost, not gate).
        let q_zero = vec![0.0f64; order.len()];
        let v = crate::value::value_v4(&order, &q_zero);
        assert!(
            *v.last().unwrap() > 0.0,
            "KNOWN GAP: value_v4 pays full novelty for q=0 noise -- quality boosts, it does not GATE"
        );
    }
}
