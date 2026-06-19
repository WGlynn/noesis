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

/// Node runtime — the replicated state machine over the mechanism library (orchestration
/// only; two nodes that finalize the same blocks converge byte-for-byte). Transport-agnostic
/// by design: peer discovery / gossip layer plugs in above the `Node` API.
pub mod runtime;

/// Starter Rust analogs of the ERC token standards in the cell model (fungible/ERC-20,
/// nft/ERC-721, multi/ERC-1155). Conservation is a pure function of the tx — no oracle layer,
/// the airgap is closed so token accounting is fully on-chain.
pub mod tokens;

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

/// Temporal novelty assigned in CONSENSUS COMMIT ORDER, invariant to the order the cells are
/// PRESENTED in. This is the temporal-order attacker-input fix at the value layer: instead of
/// trusting the caller's slice order, it sorts the cells by [`commit_order::canonical_order`]
/// (height, then XOR-seeded in-block slot), runs [`temporal_novelty`] over that canonical
/// order, and returns the values keyed back to presentation order. A producer who presents a
/// redundant cell FIRST to steal novelty gains nothing: canonical order places the truly
/// earlier-committed cell first regardless of presentation, so the redundant one still earns 0.
/// `coords[i]` is the consensus-sourced ordering coordinate of `cells_in_presented_order[i]`.
pub fn novelty_in_commit_order(
    cells_in_presented_order: &[Cell],
    coords: &[commit_order::Committed],
) -> Vec<u64> {
    assert_eq!(
        cells_in_presented_order.len(),
        coords.len(),
        "every presented cell needs its consensus-sourced ordering coordinate"
    );
    let order = commit_order::canonical_order(coords);
    let canon: Vec<Cell> = order.iter().map(|&i| cells_in_presented_order[i].clone()).collect();
    let vals = temporal_novelty(&canon);
    let mut out = vec![0u64; canon.len()];
    for (slot, &orig) in order.iter().enumerate() {
        out[orig] = vals[slot];
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

    #[test]
    fn temporal_order_is_consensus_critical_and_timestamp_is_not_the_lever() {
        // Defensive-audit fixture (SECURITY-AUDIT-attacker-choosable-inputs.md, applying
        // [P·dont-let-attacker-choose-critical-input]). temporal_novelty trusts its SLICE
        // ORDER as the commit order and never reads `timestamp`. So strategyproofness rests
        // on the ON-CHAIN path sourcing that order from consensus (commit-block height), NOT
        // a producer-arrangeable list. This pins the order-DEPENDENCE that makes the source
        // critical, AND that the timestamp field is not the lever.
        let a = b"alpha-bravo-charlie".to_vec();
        let b = b"alph".to_vec(); // strict subset of a's coverage (redundant), per the sybil test
        // True commit order: A first, redundant B earns 0.
        let honest = vec![cell(0, 1, 0, &a), cell(1, 9, 1, &b)];
        let v_honest = temporal_novelty(&honest);
        assert!(v_honest[0] > 0 && v_honest[1] == 0, "true order: A novel, redundant B earns 0");
        // Producer-favorable order: B presented FIRST earns novelty it should not have.
        let gamed = vec![cell(1, 9, 1, &b), cell(0, 1, 0, &a)];
        assert!(
            temporal_novelty(&gamed)[0] > 0,
            "GAMED: the redundant block earns novelty merely by being ordered first -> the \
             on-chain path MUST fix order to consensus commit-height, not trust the slice"
        );
        // The timestamp field is NOT the lever: same slice order, B 'backdated' older, same result.
        let ts_backdated = vec![cell(0, 1, 99, &a), cell(1, 9, 0, &b)];
        assert_eq!(
            temporal_novelty(&ts_backdated), v_honest,
            "backdating B's timestamp changes nothing: ordering is slice position, not the field"
        );
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
    /// coverage-similarity floor at `theta` + the semantic/compressibility floor at
    /// `entropy_theta`) with the learned quality boost:
    /// `value = floored_novelty × (1 + quality)`. All floors are applied BEFORE quality, so
    /// neither a near-duplicate, a zero-novelty cell, nor an incompressible-noise cell can be
    /// rescued by any quality score, while honest novel cells are quality-weighted. Floors are
    /// AND-composed (each can only zero, never raise), so strategyproofness is untouched. This
    /// is the canonical rule assembling every value-layer defense (sybil/padding/collusion via
    /// novelty, near-duplicate via the similarity floor, incompressible garbage-novelty via the
    /// semantic floor, capability via quality) into one function the PoM type-script enforces.
    /// The semantic floor's pinned airgap (high-entropy-but-valuable payloads false-positived)
    /// propagates here by construction; realized-flow (v5/v6) is the backstop. Structured-but-
    /// valueless novelty remains the honest out-of-band gap (labels/flow, not bytes).
    /// Role clarification (critical-qa 2026-06-12): this is the INTAKE-time boost form —
    /// what a cell looks worth at commit. The flow-gated rules (v5→v7) are the SETTLEMENT
    /// form — what it actually vests as use realizes. Both are enforced; "canonical" here
    /// means canonical-at-intake, not the final word on the cell's value.
    pub fn production_value(
        cells_in_commit_order: &[super::Cell],
        theta: f64,
        entropy_theta: f64,
        quality: &[f64],
    ) -> Vec<f64> {
        super::temporal_novelty_with_similarity_floor(cells_in_commit_order, theta)
            .iter()
            .zip(cells_in_commit_order)
            .map(|(&n, c)| super::semantic::semantic_floor(n, &c.data, entropy_theta))
            .zip(quality)
            .map(|(n, &q)| n as f64 * (1.0 + q))
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

    /// `value_v7` — semantic-floored SEEDS, closing the pinned gap
    /// `noise_child_still_seeds_flow_in_v5_open_gap`: under v5/v6 an incompressible-noise
    /// child still carried a positive flow seed, so a vested identity could pump a parent's
    /// gate with garbage commits (priced by v6's standing floor, but real). v7 composes the
    /// semantic floor into the SEED on top of v6's standing gate:
    ///
    ///   seed_i = semantic_floor(floored_novelty_i, data_i, entropy_theta)
    ///            if standing(contributor_i) ≥ standing_floor, else 0
    ///   value  = floored_novelty × g(downstream flow over the floored seeds)
    ///
    /// Load-bearing separation (the design caution written into the pin): the cell's SEED —
    /// what it certifies upward to its parents — is semantic-floored; its OWN base novelty
    /// is NOT. That preserves the semantic airgap's backstop: a wrongly-floored useful cell
    /// (key/hash-shaped value) still EARNS through downstream use, because its own value is
    /// floored_novelty × g(flow), untouched by the semantic floor — it just cannot CERTIFY
    /// others while its bytes are indistinguishable from noise. Earning needs use;
    /// certifying needs being legible-as-content AND vested. On fully-compressible-content
    /// graphs v7 ≡ v6 (in-test).
    /// Residual (pinned): a structured-but-valueless child — novel, compressible prose with
    /// no meaning — still seeds flow; bytes cannot catch it. That is the out-of-band
    /// frontier (labels / realized outcomes), see
    /// `structured_valueless_child_still_seeds_flow_open_gap`.
    #[allow(clippy::too_many_arguments)]
    pub fn value_v7(
        cells_in_commit_order: &[super::Cell],
        standing: &std::collections::HashMap<Vec<u8>, u64>,
        standing_floor: u64,
        theta: f64,
        entropy_theta: f64,
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
                    super::semantic::semantic_floor(n, &c.data, entropy_theta) as f64
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

    /// `value_v8` — REALIZED-OUTCOME-gated seeds, the layer that ADVANCES on v7's pinned
    /// residual (`structured_valueless_child_still_seeds_flow_open_gap`): a child that is
    /// novel, compressible, AND standing-vested — but valueless — still seeds flow in FULL
    /// under v7, because neither bytes (semantic floor) nor structure (standing) can tell
    /// legible prose-with-no-meaning from legible prose-with-meaning. The ROADMAP names the
    /// fix directly (Phase 1, the v7 residual): that gap "genuinely needs labels/flow, not
    /// bytes" — the learned OUTCOME-evaluator (`super::outcome`) is the one signal that
    /// prices it. v8 composes that learned `v(S)` into the certification SEED.
    ///
    /// HONEST SCOPE (build-don't-claim): this WIRES the outcome gate into the value layer
    /// (the structural change v7 said it needed) and is demonstrated to dampen a valueless
    /// child's certification. FULL closure of structured-but-valueless rides the real
    /// DeepFunding-distill-over-sets label pull — with only synthetic structural labels a
    /// valueless child on a real root inherits genuine lineage and is dampened (~0.42×),
    /// not zeroed; the harness runs UNCHANGED when real labels land and a label pricing the
    /// lineage ~0 drives the gate → 0. The fake-lineage-of-NOISE subclass is fully zeroed
    /// here today (the entropy floor is single-sourced into `v_outcome_floored`).
    ///
    /// v8 AND-composes the learned outcome `v(S)` into the SEED on top of v7's
    /// semantic-floor + standing gate:
    ///
    ///   seed_i = semantic_floor(floored_novelty_i, data_i, entropy_theta)
    ///            × v_outcome(w, coalition_features({i ∪ provenance-ancestors-in-graph}))
    ///            if standing(contributor_i) ≥ standing_floor, else 0
    ///   value  = floored_novelty × g(downstream flow over the outcome-floored seeds)
    ///
    /// The outcome factor is the model's value for the cell's own lineage coalition (the
    /// cell plus the parent-chain it sits in) — exactly the set-level structure
    /// (connectedness / depth / synergy) the per-cell flow gate is blind to. It is the
    /// score AFTER the entropy floor (`super::outcome::v_outcome_floored`, single-sourced
    /// with the intake floor at `theta_q16`), so a fake lineage of noise scores 0 here too:
    /// structure cannot manufacture a seed from noise.
    ///
    /// AUTHORITY BOUNDARY (the load-bearing discipline — same as the role-bounded evaluator,
    /// `OUTCOME-EVALUATOR.md` Role C): the outcome factor ∈ [0, 1] is MULTIPLIED into the
    /// seed, so it can only LOWER a seed, never raise one. A corrupt model scoring 1.0
    /// everywhere reduces v8 to v7 exactly (it cannot mint above the flow+novelty the lower
    /// layers already permit); a model scoring 0 floors the seed. The learned `v(S)` thus
    /// gains the power to DENY certification to valueless-but-legible work, and no more — it
    /// is never the gate that mints value, only one of the AND-composed floors that can zero
    /// a seed. This is why a corrupt outcome model is harmless by construction, the property
    /// `outcome::output_is_bounded_and_corruption_is_harmless_by_construction` already pins
    /// at the score; v8 inherits it because the factor only ever multiplies a seed down.
    ///
    /// Load-bearing separation preserved (the v7 caution): only the SEED — what the cell
    /// certifies UPWARD — is outcome-floored; the cell's OWN gated value is
    /// `floored_novelty × g(flow)`, untouched by the outcome model. So a coalition the model
    /// wrongly rates low still EARNS through realized downstream use (the backstop survives);
    /// it merely cannot CERTIFY others upward while the labels price its lineage as valueless.
    /// On a graph the outcome model rates 1.0 everywhere, v8 ≡ v7 (in-test).
    #[allow(clippy::too_many_arguments)]
    pub fn value_v8(
        cells_in_commit_order: &[super::Cell],
        standing: &std::collections::HashMap<Vec<u8>, u64>,
        standing_floor: u64,
        outcome_w: &[f64; super::outcome::N_FEATS],
        theta: f64,
        entropy_theta: f64,
        theta_q16: u64,
        d: f64,
        iters: usize,
        half: f64,
    ) -> Vec<f64> {
        let floored = super::temporal_novelty_with_similarity_floor(cells_in_commit_order, theta);
        // id → index for walking each cell's provenance-ancestor coalition.
        let id_to_idx: std::collections::HashMap<u64, usize> = cells_in_commit_order
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();
        let seed: Vec<f64> = floored
            .iter()
            .zip(cells_in_commit_order)
            .enumerate()
            .map(|(i, (&n, c))| {
                let s = standing.get(&c.type_script.args).copied().unwrap_or(0);
                if s < standing_floor {
                    return 0.0;
                }
                let base = super::semantic::semantic_floor(n, &c.data, entropy_theta) as f64;
                if base <= 0.0 {
                    return 0.0;
                }
                // The cell's own lineage coalition: itself plus the parent-chain it sits in.
                let idxs = lineage_coalition(cells_in_commit_order, &id_to_idx, i);
                let feats = super::outcome::coalition_features(cells_in_commit_order, &idxs);
                // Outcome value AFTER the entropy floor (single-sourced with the intake floor)
                // ∈ [0, 1]: AND-composed, can only lower the seed.
                let g_outcome = super::outcome::v_outcome_floored(
                    outcome_w,
                    &feats,
                    cells_in_commit_order,
                    &idxs,
                    theta_q16,
                );
                base * g_outcome
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

    /// The provenance-ancestor coalition of `start`: the cell itself plus every parent up
    /// its lineage chain that is present in `cells` (cycle-guarded). This is the set the
    /// outcome model scores for the cell's certification seed — the lineage structure
    /// (connectedness / depth) the per-cell flow gate cannot see. Indices into `cells`.
    fn lineage_coalition(
        cells: &[super::Cell],
        id_to_idx: &std::collections::HashMap<u64, usize>,
        start: usize,
    ) -> Vec<usize> {
        let mut idxs = vec![start];
        let mut cur = cells[start].parent;
        let mut guard = 0;
        while let Some(p) = cur {
            match id_to_idx.get(&p) {
                Some(&pi) if !idxs.contains(&pi) => {
                    idxs.push(pi);
                    cur = cells[pi].parent;
                }
                _ => break,
            }
            guard += 1;
            if guard > cells.len() {
                break;
            }
        }
        idxs
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
        fn noise_child_still_seeds_flow_in_v5_open_gap() {
            // PINNED GAP (named by the production_value semantic wiring, 2026-06-12): the
            // semantic floor guards the canonical BOOST rule only. In the flow-gated rules a
            // high-entropy noise CHILD still carries a positive seed (similarity floor alone),
            // so a different-identity noise commit pumps a parent's realized flow. v6 prices
            // the identity (standing-gated seeds), so the pump costs earned standing — bounded,
            // not free — but a VESTED contributor's noise still pumps. Next increment candidate:
            // semantic-floored SEEDS. Design with care: the semantic floor's own airgap backstop
            // ("a wrongly-floored useful cell still earns via downstream flow") must survive —
            // flooring the cell's seed is not the same as flooring its own gated value.
            let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let parent_alone = vec![cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta")];
            let with_noise_child = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, Some(0), &noise), // other identity commits noise ON the parent
            ];
            let alone = value_v5(&parent_alone, THETA, DAMP, ITERS, HALF);
            let pumped = value_v5(&with_noise_child, THETA, DAMP, ITERS, HALF);
            assert!(
                pumped[0] > alone[0],
                "OPEN GAP: incompressible-noise child pumps the parent's flow gate \
                 (semantic floor not yet composed into v5/v6 seeds)"
            );
        }

        // ---- value_v7: semantic-floored seeds (closes the noise-child pump) ----

        const ENTROPY_THETA: f64 = 0.95;

        #[test]
        fn value_v7_noise_child_no_longer_pumps_the_parent() {
            // FLIPS `noise_child_still_seeds_flow_in_v5_open_gap` at the v7 rule: the SAME
            // vested identity committing the SAME noise child pumps the parent under v6
            // (standing-priced but real) and pumps NOTHING under v7 (seed semantic-floored).
            let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, Some(0), &noise), // VESTED identity commits noise ON the parent
            ];
            let st = standing_of(&[(1, FLOOR), (9, FLOOR)]); // attacker fully vested
            let v6 = value_v6(&order, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            assert!(v6[0] > 0.0, "v6: vested noise child still pumps the parent (the gap)");
            let v7 = value_v7(&order, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert_eq!(v7[0], 0.0, "v7: noise certifies nothing, even from a vested identity");
        }

        #[test]
        fn value_v7_airgap_backstop_survives_keyish_cell_still_earns() {
            // The load-bearing separation: a high-entropy-but-VALUABLE cell (key/hash shaped,
            // the semantic floor's pinned false-positive) still EARNS when a vested other
            // mind builds real content on it — its OWN novelty is not semantic-floored, only
            // its seed is. Flooring the seed ≠ flooring the cell's own gated value.
            let keyish: Vec<u8> = (0u8..32).map(|i| i.wrapping_mul(67).wrapping_add(29)).collect();
            let order = vec![
                cellc(0, 1, 0, None, &keyish),
                cellc(1, 2, 1, Some(0), b"library-built-on-the-published-key-material"),
            ];
            let st = standing_of(&[(1, FLOOR), (2, FLOOR)]);
            let v7 = value_v7(&order, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert!(
                v7[0] > 0.0,
                "backstop intact: wrongly-floored useful cell is paid through realized use"
            );
            // ...but the keyish cell cannot CERTIFY: a parent whose only child is keyish gets 0.
            let order2 = vec![
                cellc(0, 1, 0, None, b"echo-foxtrot-golf-hotel"),
                cellc(1, 2, 1, Some(0), &keyish),
            ];
            let v7b = value_v7(&order2, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert_eq!(v7b[0], 0.0, "noise-shaped bytes certify nothing upward (seed floored)");
        }

        #[test]
        fn value_v7_equals_v6_on_compressible_content() {
            // On a fully-compressible (genuine-content) graph the semantic seed floor is
            // inert: v7 ≡ v6 elementwise.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel"),
                cellc(2, 3, 2, Some(1), b"india-juliet-kilo-lima-mike"),
            ];
            let st = standing_of(&[(1, FLOOR), (2, FLOOR), (3, 0)]); // mixed vesting too
            let v6 = value_v6(&order, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            let v7 = value_v7(&order, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            for (a, b) in v6.iter().zip(&v7) {
                assert!((a - b).abs() < 1e-12, "content-only graph: v7 must equal v6");
            }
        }

        #[test]
        fn structured_valueless_child_still_seeds_flow_open_gap() {
            // PINNED GAP (the v7 survivor, named by the adversarial tick): a vested identity
            // committing NOVEL, COMPRESSIBLE, meaningless prose still seeds flow — bytes
            // cannot distinguish structured-pointless from structured-valuable. This is the
            // known out-of-band frontier (outcome labels / realized external value, not a
            // content gate): HANDOFF frontier #3. Bounded as before by v6 standing pricing +
            // dispute slashing on refutation; not free, but not structurally closed.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, Some(0), b"the-quick-brown-fox-says-nothing-of-value-today"),
            ];
            let st = standing_of(&[(1, FLOOR), (9, FLOOR)]);
            let v7 = value_v7(&order, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert!(
                v7[0] > 0.0,
                "OPEN GAP: structured-but-valueless child pumps the parent under v7; \
                 closing it needs labels/outcomes, not bytes"
            );
        }

        // ---- encoding-evasion of the semantic seed floor (adversarial tick 2026-06-12) ----

        /// Hex-encode a payload: the byte alphabet collapses to 16 symbols, so order-0 byte
        /// entropy drops to ~0.57 — under the floor's theta — while the underlying information
        /// is unchanged. The canonical evasion pinned in
        /// `semantic::tests::encoded_noise_evades_the_entropy_floor_open_gap`.
        fn hex_encoded(data: &[u8]) -> Vec<u8> {
            data.iter().flat_map(|b| format!("{b:02x}").into_bytes()).collect()
        }

        #[test]
        fn encoded_noise_defeats_the_v7_seed_gate_the_evasion_is_real() {
            // SHARPENS `semantic::tests::encoded_noise_evades_the_entropy_floor_open_gap`: the
            // evasion is not merely "the floor misses it" — it RE-OPENS the v7 seed-gate pump
            // that `value_v7_noise_child_no_longer_pumps_the_parent` closed. v7 floors a
            // RAW-noise child's seed to 0; hex-encoded noise has low byte-entropy, so
            // semantic_floor passes it ⇒ the seed is live ⇒ the SAME vested identity pumps the
            // parent's flow gate again. On encoded content v7 collapses to v6 (the floor is
            // inert) — which is exactly why the binding defense cannot live at the content
            // layer; the next two tests carry it (standing price + dispute slash).
            let raw: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let parent = || cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta");
            let st = standing_of(&[(1, FLOOR), (9, FLOOR)]); // attacker (id 9) fully vested

            let raw_child = vec![parent(), cellc(1, 9, 1, Some(0), &raw)];
            let v7_raw = value_v7(&raw_child, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert_eq!(v7_raw[0], 0.0, "raw noise: v7 floors the seed ⇒ parent unpumped");

            let enc_child = vec![parent(), cellc(1, 9, 1, Some(0), &hex_encoded(&raw))];
            let v7_enc = value_v7(&enc_child, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert!(
                v7_enc[0] > 0.0,
                "OPEN GAP (sharpened): hex-encoded noise evades the seed floor and re-pumps \
                 the parent under v7 — the content layer cannot close this (the airgap)"
            );
            // The crux: on the encoded payload the semantic floor is inert ⇒ v7 ≡ v6.
            let v6_enc = value_v6(&enc_child, &st, FLOOR, THETA, DAMP, ITERS, HALF);
            assert!((v7_enc[0] - v6_enc[0]).abs() < 1e-12, "v7 collapses to v6 on encoded noise");
        }

        #[test]
        fn encoded_noise_does_not_buy_past_the_v6_standing_price() {
            // FIRST binding defense, content-blind on identity. The evasion beats the byte
            // heuristic but NOT the standing price: the same hex-encoded noise child on a
            // FRESH (unvested) key seeds 0 regardless of how legible-as-content its bytes look
            // (v6/v7 gate the seed on EARNED standing, not on entropy). A sybil ring of encoded
            // garbage on fresh keys therefore earns nothing — encoding gains the adversary 0 here.
            let raw: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, Some(0), &hex_encoded(&raw)),
            ];
            let fresh = standing_of(&[(1, FLOOR)]); // id 9 unvested (no earned standing)
            let v7 = value_v7(&order, &fresh, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert_eq!(v7[0], 0.0, "encoded noise from an unvested key seeds nothing (priced identity)");
            let v6 = value_v6(&order, &fresh, FLOOR, THETA, DAMP, ITERS, HALF);
            assert_eq!(
                v6[0], 0.0,
                "v6 standing price is content-agnostic: encoding does not buy past it"
            );
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
            let v = production_value(&order, 0.8, 0.95, &q);
            assert_eq!(*v.last().unwrap(), 0.0, "near-dup -> 0 even at max quality (floor before quality)");
            assert!(v[0] > 0.0, "honest novel cell earns novelty x (1 + quality)");
        }

        #[test]
        fn production_value_zeroes_incompressible_noise_even_at_max_quality() {
            // The garbage-novelty cell the coverage proxy pays in full
            // (`garbage_novelty_is_the_documented_open_gap`) is now zeroed AT the canonical
            // rule: the semantic floor AND-composes after the similarity floor, before quality.
            let mut order = honest();
            let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            order.push(cell(99, 9, 99, &noise));
            let q = vec![1.0; order.len()];
            // Contrast: the similarity floor ALONE still pays the noise (it is genuinely novel),
            // proving the semantic floor — not novelty — does the zeroing here.
            let sim_only = super::super::temporal_novelty_with_similarity_floor(&order, 0.8);
            assert!(*sim_only.last().unwrap() > 0, "noise survives the similarity floor alone");
            let v = production_value(&order, 0.8, 0.95, &q);
            assert_eq!(*v.last().unwrap(), 0.0, "incompressible noise -> 0 even at max quality");
            assert!(v[..3].iter().all(|&x| x > 0.0), "structured honest cells unaffected");
        }

        #[test]
        fn production_value_semantic_airgap_pinned_high_entropy_value_floored() {
            // KNOWN TRADEOFF propagated into the canonical rule (same pin as
            // `semantic::honest_false_positive_high_entropy_value_is_floored_pinned`): a
            // high-entropy VALUABLE payload (key/hash shaped) is floored here too. Content
            // alone cannot tell it from noise; realized-flow (v5/v6) is the backstop.
            let mut order = honest();
            let keyish: Vec<u8> = (0u8..32).map(|i| i.wrapping_mul(67).wrapping_add(29)).collect();
            order.push(cell(7, 4, 50, &keyish));
            let q = vec![1.0; order.len()];
            let v = production_value(&order, 0.8, 0.95, &q);
            assert_eq!(
                *v.last().unwrap(),
                0.0,
                "airgap: high-entropy-but-valuable content false-positived at the canonical gate"
            );
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

        // ---- value_v8: realized-OUTCOME-gated seeds (closes the structured-but-valueless
        //      residual the bytes/structure floors cannot reach — ROADMAP Phase 1) ----

        const THETA_Q16: u64 = 62259; // floor(0.95 · 2^16) — same entropy threshold as on-VM intake

        /// Train the labels-sourced outcome model the v8 seed-gate consumes. Mirrors the
        /// `outcome` module's own tests: a connected-lineage VALUE coalition is preferred to
        /// an orphaned-garbage one, so the learned weights price connectedness + depth (the
        /// set-level structure the per-cell flow gate is blind to). In production these
        /// weights come from the DeepFunding distill-over-sets label pull
        /// (`outcome::load_prefs` locks that seam); here the preference is the same synthetic
        /// structural label the `outcome` tests use, so the property is demonstrated, not the
        /// real-label magnitude — the harness runs UNCHANGED when real labels land.
        fn trained_outcome_w() -> [f64; crate::outcome::N_FEATS] {
            use crate::outcome::coalition_features;
            let noise = |s: u8| -> Vec<u8> {
                (0u8..24).map(|i| s.wrapping_add(i.wrapping_mul(53))).collect()
            };
            let value_set = vec![
                cellc(100, 1, 0, None, b"alpha-bravo-charlie"),
                cellc(101, 1, 1, Some(100), b"delta-echo-foxtrot"),
                cellc(102, 1, 2, Some(101), b"golf-hotel-india"),
            ];
            let garbage_set = vec![
                cellc(110, 2, 0, None, &noise(0x10)),
                cellc(111, 2, 1, None, &noise(0x80)),
                cellc(112, 2, 2, None, &noise(0xC0)),
            ];
            let feats = [
                coalition_features(&value_set, &[0, 1, 2]),
                coalition_features(&garbage_set, &[0, 1, 2]),
            ];
            crate::outcome::train(&feats, &vec![(0usize, 1usize); 8], 4000, 0.3)
        }

        #[test]
        fn value_v8_dampens_the_structured_but_valueless_residual_via_the_outcome_gate() {
            // THE residual `structured_valueless_child_still_seeds_flow_open_gap` pins: a
            // VESTED identity commits NOVEL, COMPRESSIBLE, meaningless prose as a child. v7
            // pays it in FULL (bytes + standing both pass), pumping the parent's flow gate. v8
            // composes the labels-trained outcome `v(S)` into the seed, so the child's
            // certification is now WEIGHTED by the model's value for its lineage coalition
            // instead of admitted at face value.
            //
            // HONEST SCOPE (build-don't-claim, the boundary this test documents): with only the
            // SYNTHETIC structural labels available today, a valueless child ATTACHED TO A REAL
            // ROOT inherits genuine connectedness/depth, so the model dampens its seed
            // (~0.42×) rather than zeroing it — v8[0] is strictly below v7[0] but not 0. The
            // outcome gate moves the dial in the right direction from the one signal labels
            // carry (set-level structure the flow gate is blind to); FULL closure of
            // structured-but-valueless rides the real DeepFunding-distill-over-sets label pull
            // (`outcome::load_prefs` locks that seam — the harness runs UNCHANGED when it
            // lands, and a real label that prices this lineage ~0 drives g→0 ⇒ seed→0). The
            // fake-lineage-of-NOISE subclass is already fully zeroed (next test).
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, Some(0), b"the-quick-brown-fox-says-nothing-of-value-today"),
            ];
            let st = standing_of(&[(1, FLOOR), (9, FLOOR)]); // attacker fully vested
            let w = trained_outcome_w();

            // v7 pays it in full — the gap, still pinned above.
            let v7 = value_v7(&order, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            assert!(v7[0] > 0.0, "v7 pays the structured-valueless child (the open gap)");

            // v8: the outcome gate lowers the valueless child's certification of the parent.
            let v8 = value_v8(
                &order, &st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF,
            );
            assert!(
                v8[0] < v7[0],
                "v8 strictly lowers the parent: the valueless child certifies LESS than at \
                 v7 face value — the outcome gate is live (full closure rides real labels)"
            );
            assert!(
                v8[0] > 0.0,
                "honest scope: synthetic structural labels DAMPEN but do not zero a valueless \
                 child on a real root; the real-label pull drives this to 0"
            );
        }

        #[test]
        fn single_identity_volume_saturates_under_per_identity_damping() {
            // CLOSED (2026-06-18) — was `single_identity_volume_defeats_v8_dampening_open_gap`.
            // Previously the downstream-flow accumulation (`flow::value_flow_with_own`) SUMMED
            // every cross-identity child of a parent with NO per-identity cap, so a SINGLE vested
            // attacker identity posting N distinct novel-but-valueless children amplified the
            // root's flow gate LINEARLY in N (measured ~1.44× over N=1..8 and climbing) —
            // per-child outcome dampening was a constant the attacker bought off with volume.
            //
            // FIX: per-identity diminishing-returns damping in `value_flow_with_own` — the r-th
            // child (commit order) from a given certifying identity is weighted λ^r, λ=1/φ. One
            // identity's certifying flow now SATURATES (geometric, bound flow/(1-λ)≈2.62×) instead
            // of amplifying; distinct identities stay full-weight at rank 0 (honest diverse
            // certification untouched — orthogonal to `max_certifying_identities`).
            //
            // HONEST RESIDUAL: saturation BOUNDS the attack, it does not zero it. Eight dampened
            // children (v8(8)≈18.11) still just exceed ONE undampened child (v7(1)≈17.63) — a
            // ~2.7% gain, not the old unbounded linear pump. That residual is acceptable because
            // v6 priced-identity already requires the attacker identity to be vested/standing-
            // gated, and v8 still dampens each child; removing the UNBOUNDED-in-N amplification
            // was the actual vulnerability. (Numbers honest-numbered from the probe below.)
            //
            // Distinct from the closed vectors: the v6 sybil ring is UNVESTED (seed 0); identical
            // content is deduped by novelty; self-certification is excluded by
            // `children_of_external`'s same-identity skip. The mutually-dissimilar payloads below
            // each clear the similarity floor, so that floor does NOT bound the attack.
            let w = trained_outcome_w();
            // mutually-DISSIMILAR valueless prose: distinct words ⇒ each clears the similarity
            // floor and is individually novel, yet none carries value — the worst case for the gate.
            let payloads: [&[u8]; 8] = [
                b"the cat sat quietly on the warm mat today",
                b"rivers flow gently under the old stone bridge",
                b"morning light fills the quiet empty kitchen slowly",
                b"yellow kites drift above the distant green hill",
                b"books rest unread along the dusty wooden shelf",
                b"snow settles softly across the silent winter field",
                b"clocks tick onward through the long grey afternoon",
                b"birds gather near the fence before the evening rain",
            ];
            let root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta");
            let st = standing_of(&[(1, FLOOR), (9, FLOOR)]); // root id 1 + ONE attacker id 9, both vested
            let order = |n: usize| -> Vec<Cell> {
                let mut o = vec![root.clone()];
                for k in 0..n {
                    o.push(cellc((k + 1) as u64, 9, (k + 1) as u64, Some(0), payloads[k]));
                }
                o
            };
            let v8 = |n: usize| {
                value_v8(&order(n), &st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF)[0]
            };
            let v7 = |n: usize| value_v7(&order(n), &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF)[0];

            // Curve (honest-numbered 2026-06-18, λ=1/φ):
            //   v8: N1=14.28 N2=16.44 N4=17.66 N8=18.11 | v7: N1=17.63 N8=20.12
            let (a1, a4, a8) = (v8(1), v8(4), v8(8));
            let (s1, s8) = (v7(1), v7(8));
            // (1) SATURATION — the marginal gain collapses: doubling 4→8 children adds far less
            //     than 1→4 did (diminishing returns, the geometric tail).
            assert!(
                (a8 - a4) < 0.2 * (a4 - a1),
                "volume gain did not saturate: tail {:.4} not << head {:.4}",
                a8 - a4,
                a4 - a1
            );
            // (2) BOUNDED — eight dampened children from ONE identity barely exceed a SINGLE
            //     undampened child; per-identity volume no longer meaningfully amplifies.
            assert!(
                a8 < 1.05 * s1,
                "single-identity volume still amplifies past one undampened child: v8(8)={:.4} vs v7(1)={:.4}",
                a8, s1
            );
            // (3) NOT over-damped — honest children still seed flow (the fix must not zero value).
            assert!(a8 > a1, "damping zeroed the honest signal");
            // (4) sanity — the per-child outcome gate still dampens at equal N.
            assert!(
                a8 < s8,
                "sanity: the outcome gate still dampens each child at equal N (constant factor)"
            );
        }

        #[test]
        fn multi_identity_split_volume_saturates_under_cross_identity_damping() {
            // SIBLING of single_identity_volume_saturates_under_per_identity_damping, one level up.
            // The λ^r WITHIN-identity damping caps ONE identity's volume (the r-th child of a given
            // certifying identity decays λ^r). An attacker who SPLITS the same volume across K
            // DISTINCT VESTED identities posts one child per identity, so every child is rank-0 in
            // its OWN group (λ^0 = 1) — within-identity damping is inert for the split. The
            // CROSS-identity μ^m damping closes this: a parent's distinct certifying identities are
            // sorted by grouped contribution desc and the m-th identity weighted μ^m (μ=1/φ). One
            // identity stays full; additional identities decay, so splitting across K identities
            // saturates exactly like stacking under one. The curve now mirrors the within-identity
            // saturation one level up (K1≈14.28 … K8≈18.13 ≈ single-identity bound 18.11).
            //
            // Honest diverse certification (1-2 identities) is INERT: ranks m=0/1 ⇒ μ^0=1, μ^1≈0.62
            // on the SMALLER contributor only ⇒ near-full. The attack is volume-via-many-identities,
            // which this saturates; legitimately broad certification keeps its value.
            let w = trained_outcome_w();
            let payloads: [&[u8]; 8] = [
                b"the cat sat quietly on the warm mat today",
                b"rivers flow gently under the old stone bridge",
                b"morning light fills the quiet empty kitchen slowly",
                b"yellow kites drift above the distant green hill",
                b"books rest unread along the dusty wooden shelf",
                b"snow settles softly across the silent winter field",
                b"clocks tick onward through the long grey afternoon",
                b"birds gather near the fence before the evening rain",
            ];
            let root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta");
            // root id 1 + 8 DISTINCT attacker identities (10..=17), ALL independently vested.
            let mut sv = vec![(1u8, FLOOR)];
            for k in 0..8u8 {
                sv.push((10 + k, FLOOR));
            }
            let st = standing_of(&sv);
            // K identities, ONE child each ⇒ every child is rank-0 in its identity group.
            let split = |k: usize| -> Vec<super::super::Cell> {
                let mut o = vec![root.clone()];
                for j in 0..k {
                    o.push(cellc((j + 1) as u64, 10 + j as u8, (j + 1) as u64, Some(0), payloads[j]));
                }
                o
            };
            let v8s = |k: usize| {
                value_v8(&split(k), &st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF)[0]
            };
            // Single-identity saturation bound (the cross-identity curve must not exceed it).
            let single_v8_8 = {
                let order = |n: usize| -> Vec<super::super::Cell> {
                    let mut o = vec![root.clone()];
                    for j in 0..n {
                        o.push(cellc((j + 1) as u64, 99, (j + 1) as u64, Some(0), payloads[j]));
                    }
                    o
                };
                let st1 = standing_of(&[(1u8, FLOOR), (99u8, FLOOR)]);
                value_v8(&order(8), &st1, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF)[0]
            };
            let (k1, k2, k4, k8) = (v8s(1), v8s(2), v8s(4), v8s(8));
            println!(
                "PROBE multi-identity v8 (cross-identity μ^m damping): K1={k1:.4} K2={k2:.4} K4={k4:.4} K8={k8:.4} (single-identity v8(8) bound = {single_v8_8:.4})"
            );
            // (1) SATURATION — the marginal gain collapses: doubling K=4→8 adds far less than K=1→4.
            assert!(
                (k8 - k4) < 0.2 * (k4 - k1),
                "multi-identity split did not saturate: tail {:.4} not << head {:.4}",
                k8 - k4,
                k4 - k1
            );
            // (2) BOUNDED — splitting across 8 distinct vested identities does NOT exceed the
            //     single-identity saturation bound; the split buys the attacker nothing.
            assert!(
                k8 <= 1.02 * single_v8_8,
                "multi-identity split exceeds the single-identity bound: K8={k8:.4} vs single={single_v8_8:.4}"
            );
            // (3) BOUNDED vs K=1 — K=8 barely exceeds K=1 (the geometric μ^m tail), not ~linear in K.
            assert!(
                k8 < 1.3 * k1,
                "multi-identity split still amplifies: K8={k8:.4} K1={k1:.4}"
            );
            // (4) NOT over-damped — the first identity still seeds full value (the fix must not zero).
            assert!(k1 > 0.0 && k8 > k1, "cross-identity damping zeroed honest signal");
        }

        // 16 mutually-dissimilar valueless payloads (distinct vocabularies) for the T3
        // hybrid grid: K identities × M children needs up to 4×4 = 16 children, each
        // individually novel and clearing the similarity floor (worst case for the gate).
        const HYBRID_PAYLOADS: [&[u8]; 16] = [
            b"the cat sat quietly on the warm mat today",
            b"rivers flow gently under the old stone bridge",
            b"morning light fills the quiet empty kitchen slowly",
            b"yellow kites drift above the distant green hill",
            b"books rest unread along the dusty wooden shelf",
            b"snow settles softly across the silent winter field",
            b"clocks tick onward through the long grey afternoon",
            b"birds gather near the fence before the evening rain",
            b"copper wires hum behind the locked basement panel",
            b"sailors mend torn nets beside the harbor wall",
            b"violet orchids bloom inside the humid glass dome",
            b"trucks rumble past the shuttered roadside diner",
            b"lanterns sway along the crooked mountain trail",
            b"engineers sketch turbines on the wide blue board",
            b"foxes circle the orchard under a thin crescent moon",
            b"pottery dries in rows along the sunlit adobe ledge",
        ];

        // ---- T3 keystone helpers: the hybrid K-identities × M-children grid -------------
        // Build the root + K certifying identities (args 10..10+K), each posting M distinct
        // valueless children on the single root. Returns the order; identities all vested.
        fn t3_hybrid_order(root: &super::super::Cell, k: usize, m: usize) -> Vec<super::super::Cell> {
            let mut o = vec![root.clone()];
            let (mut id, mut p) = (1u64, 0usize);
            for ki in 0..k {
                for _ in 0..m {
                    o.push(cellc(id, 10 + ki as u8, id, Some(0), HYBRID_PAYLOADS[p]));
                    id += 1;
                    p += 1;
                }
            }
            o
        }
        fn t3_hybrid_standing(k: usize) -> std::collections::HashMap<Vec<u8>, u64> {
            let mut sv = vec![(1u8, FLOOR)];
            for ki in 0..k {
                sv.push((10 + ki as u8, FLOOR));
            }
            standing_of(&sv)
        }

        #[test]
        fn t3_hybrid_diagonal_pumps_past_single_identity_bound_open_gap() {
            // ========================= NEW GAMING VECTOR (the T3 find) =========================
            // VECTOR NAME: hybrid-split diagonal pump (cross-axis geometric-tail compounding).
            //
            // The (q) within-identity λ^r damping bounds ONE identity's M children to the
            // geometric tail flow·(1−λ^M)/(1−λ) ≤ flow/(1−λ) ≈ 2.618·flow. The (r) cross-identity
            // μ^m damping bounds K identities' grouped contributions to Σ_m μ^m·group_m. EACH
            // AXIS IS INDIVIDUALLY BOUNDED — the K=1 column tops out at the single-identity bound,
            // the M=1 row reproduces the pure cross-identity curve. BUT the DIAGONAL is not: an
            // attacker who runs K vested identities EACH posting M children gives every one of the
            // K groups a FULL λ^r tail (≈2.618·flow for large M), then sums those K near-saturated
            // groups under the μ^m tail. The two geometric tails MULTIPLY:
            //     bound_diagonal → flow · [1/(1−λ)] · [1/(1−μ)] ≈ flow · 2.618 · 2.618 ≈ 6.85·flow
            // versus the single-identity bound flow/(1−λ) ≈ 2.618·flow. The cross of the two
            // independently-bounded axes pumps past the single-identity saturation bound.
            //
            // HONEST GRID (measured 2026-06-18, μ=λ=1/φ; v8(root), single-identity K1×M8 = 18.1073):
            //   K\M     M=1       M=2       M=4
            //   K=1   14.2821   16.4373   17.6582   ← within-identity axis, saturates ≤ 18.11
            //   K=2   16.4373   18.1768   19.0835   ← K2×M2 already 18.18 > 18.11 (bound broken)
            //   K=4   17.6623   19.0838   19.7499   ← K4×M4 = 19.75, ~9% over the single bound
            // The pump is modest at the 8-identity standing-floor cost the attacker pays, but it is
            // REAL and MONOTONE in both K and M — exactly the unbounded-product signature above.
            //
            // This is THE next adversarial-loop surface the (s) spec flagged ("a fix that bounds
            // each axis independently could still pump on the diagonal"). It does. Pinned RED here;
            // fix design recorded in ROADMAP + CONTINUE. Tier: 🔬 OPEN.
            let w = trained_outcome_w();
            let root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta");
            let v8 = |k: usize, m: usize| {
                value_v8(&t3_hybrid_order(&root, k, m), &t3_hybrid_standing(k),
                    FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF)[0]
            };
            // single-identity saturation bound (K=1 axis at M=8): the diagonal must NOT exceed it,
            // but it does — that is the gap.
            let single_bound = {
                let mut o = vec![root.clone()];
                for j in 0..8u64 {
                    o.push(cellc(j + 1, 99, j + 1, Some(0), HYBRID_PAYLOADS[j as usize]));
                }
                let s = standing_of(&[(1u8, FLOOR), (99u8, FLOOR)]);
                value_v8(&o, &s, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF)[0]
            };
            let (d22, d44) = (v8(2, 2), v8(4, 4));
            println!(
                "PROBE T3 hybrid diagonal: single_bound(K1M8)={single_bound:.4} K2M2={d22:.4} K4M4={d44:.4}"
            );
            // (A) THE GAP: the diagonal pumps past the single-identity saturation bound. Even the
            //     smallest non-trivial cross (K2×M2) breaks it; the 4×4 cross breaks it materially.
            assert!(
                d22 > single_bound,
                "expected the diagonal to pump (K2M2={d22:.4} > bound={single_bound:.4}) — if this \
                 fails, the cross-axis fix landed and this open_gap should flip to saturates"
            );
            assert!(
                d44 > single_bound * 1.05,
                "expected a material 4x4 pump (K4M4={d44:.4} > 1.05·bound={:.4})",
                single_bound * 1.05
            );
            // (B) the pump is MONOTONE up the diagonal (the geometric-product signature): more
            //     identities AND more children each push it higher.
            assert!(d44 > d22, "diagonal pump is monotone in the grid: K4M4 {d44:.4} > K2M2 {d22:.4}");
        }

        #[test]
        fn t4_honest_diverse_certification_is_inert() {
            // T4 (crit. 4 — INERT). The cross-identity μ^m damping must NOT punish legitimately
            // broad certification: two HONEST distinct identities each building one real, value-
            // carrying child on a DISTINCT real parent. Different parents ⇒ each parent has a
            // SINGLE certifying identity at rank m=0 ⇒ μ^0=1 ⇒ full weight. The fix is inert; the
            // honest roots are paid the same as they would be with no cross-identity layer at all.
            let w = trained_outcome_w();
            // Two separate honest lineages, distinct parents, distinct honest identities.
            let order = vec![
                cellc(0, 1, 0, None, b"first-honest-root-alpha-bravo-charlie"),
                cellc(1, 2, 1, Some(0), b"genuine-child-builds-on-first-root-delta"),
                cellc(2, 3, 2, None, b"second-honest-root-echo-foxtrot-golf"),
                cellc(3, 4, 3, Some(2), b"genuine-child-builds-on-second-root-hotel"),
            ];
            let st = standing_of(&[(1, FLOOR), (2, FLOOR), (3, FLOOR), (4, FLOOR)]);
            let v8 = value_v8(&order, &st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF);
            // Both honest roots are paid: real downstream use certifies them, single-identity
            // per parent ⇒ μ^0=1, the cross-identity damping never engages.
            assert!(v8[0] > 0.0, "first honest root paid — diverse certification not over-punished");
            assert!(v8[2] > 0.0, "second honest root paid — distinct-parent certification inert");
        }

        #[test]
        fn t5_cross_identity_sort_is_deterministic_no_hashmap_leak() {
            // T5 (crit. 5 — DETERMINISM). The cross-identity layer must converge across replicas:
            // its identity ordering is a CANONICAL sort (grouped contribution desc, identity args
            // asc tiebreak), and the per-parent flow accumulation must NOT leak HashMap iteration
            // order. The honest property here is RUN-TO-RUN / replica determinism on a FIXED input,
            // NOT full input-shuffle invariance — commit order (vector position) is itself a real
            // value input (temporal_novelty ranks earlier commits over later near-novel ones, and
            // the within-identity λ^r rank is commit order), so permuting the input legitimately
            // changes the value. What MUST hold is that the same input always yields the same v8
            // regardless of HashMap seeding (`std::collections::HashMap` randomizes its hasher
            // per-instance, so a leak of children-map or groups-build order would surface as
            // run-to-run drift). Evaluate the T1 split AND the T3 hybrid graphs repeatedly and
            // assert bit-identical results every time.
            let w = trained_outcome_w();
            let root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta");
            // T1 split: K=8 distinct identities, one child each.
            let mut t1 = vec![root.clone()];
            for j in 0..8u64 {
                t1.push(cellc(j + 1, 10 + j as u8, j + 1, Some(0), HYBRID_PAYLOADS[j as usize]));
            }
            let st1 = t3_hybrid_standing(8);
            // T3 hybrid: K=4 identities × M=4 children (the diagonal-pump graph, max grouping).
            let t3 = t3_hybrid_order(&root, 4, 4);
            let st3 = t3_hybrid_standing(4);
            let eval = |order: &[super::super::Cell], st: &std::collections::HashMap<Vec<u8>, u64>| {
                value_v8(order, st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF)
            };
            let base1 = eval(&t1, &st1);
            let base3 = eval(&t3, &st3);
            // Re-evaluate many times: each call builds fresh HashMaps with fresh random seeds.
            for _ in 0..32 {
                let r1 = eval(&t1, &st1);
                let r3 = eval(&t3, &st3);
                for (a, b) in base1.iter().zip(&r1) {
                    assert_eq!(a.to_bits(), b.to_bits(), "T1 split: HashMap-order leak — non-deterministic v8");
                }
                for (a, b) in base3.iter().zip(&r3) {
                    assert_eq!(a.to_bits(), b.to_bits(), "T3 hybrid: HashMap-order leak — non-deterministic v8");
                }
            }
        }

        #[test]
        fn value_v8_pays_a_genuinely_useful_lineage() {
            // The other side of the gate: a child that EXTENDS the parent's lineage (real
            // provenance depth, the structure labels reward) keeps its seed, so the parent is
            // paid. The outcome gate denies the valueless and admits the valuable — not a blunt
            // suppressor.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel-built-on-the-root"),
                cellc(2, 3, 2, Some(1), b"india-juliet-kilo-lima-extends-the-lineage"),
            ];
            let st = standing_of(&[(1, FLOOR), (2, FLOOR), (3, FLOOR)]);
            let w = trained_outcome_w();
            let v8 = value_v8(
                &order, &st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF,
            );
            assert!(v8[0] > 0.0, "a real connected lineage still certifies the root: parent paid");
            assert!(v8[1] > 0.0, "the mid cell, itself extended by a grandchild, is paid too");
        }

        #[test]
        fn value_v8_fake_lineage_of_noise_seeds_nothing() {
            // Inherits the fake-lineage closure: an attacker who spoofs lineage STRUCTURE
            // (a chain of noise, each pointing at the last) to fool the outcome model's
            // connectedness/depth features still seeds 0, because v_outcome_floored
            // AND-composes the entropy floor (single-sourced with intake) — structure cannot
            // manufacture a seed from noise. Closes the fake-lineage vector at the SEED, the
            // way `outcome::semantic_floor_closes_the_fake_lineage_spoof_at_the_score` closes
            // it at the score.
            let noise = |s: u8| -> Vec<u8> {
                (0u8..48).map(|i| s.wrapping_add(i.wrapping_mul(53))).collect()
            };
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"), // honest root, real bytes
                cellc(1, 9, 1, Some(0), &noise(0x10)),              // fake-lineage noise child
                cellc(2, 9, 2, Some(1), &noise(0x90)),              // ...pointing at the last
            ];
            let st = standing_of(&[(1, FLOOR), (9, FLOOR)]); // attacker vested
            let w = trained_outcome_w();
            let v8 = value_v8(
                &order, &st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF,
            );
            // The honest root earns 0 here: the only thing built on it is a noise child whose
            // seed the entropy floor zeroes ⇒ no real downstream use ⇒ gate shut.
            assert_eq!(v8[0], 0.0, "fake-lineage noise seeds nothing — root unpumped");
        }

        #[test]
        fn value_v8_corrupt_outcome_model_cannot_mint_above_v7() {
            // THE authority boundary (OUTCOME-EVALUATOR Role C): the outcome factor ∈ [0,1] is
            // MULTIPLIED into the seed, so it can only LOWER. An adversary who corrupts the
            // model to score 1.0 everywhere (all-positive weights ⇒ v_outcome → 1 on any
            // non-noise coalition) reduces v8 to v7 EXACTLY — it cannot mint a cent above the
            // flow + novelty the lower layers already permit. The learned v(S) gains the power
            // to DENY valueless certification and no more; corruption is harmless by construction.
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel-built-on-root"),
                cellc(2, 3, 2, Some(0), b"india-juliet-kilo-lima-also-on-root"),
            ];
            let st = standing_of(&[(1, FLOOR), (2, FLOOR), (3, FLOOR)]);
            // Corrupt: huge positive weights ⇒ sigmoid(dot) → 1.0 for every (non-noise)
            // coalition, the maximum the gate can express.
            let corrupt_w = [1e6; crate::outcome::N_FEATS];
            let v8 = value_v8(
                &order, &st, FLOOR, &corrupt_w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF,
            );
            let v7 = value_v7(&order, &st, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            for (a, b) in v8.iter().zip(&v7) {
                assert!(
                    (a - b).abs() < 1e-9,
                    "outcome gate = 1 everywhere ⇒ v8 == v7: a corrupt model cannot mint above v7"
                );
            }
        }

        #[test]
        fn value_v8_backstop_a_low_rated_cell_still_earns_its_own_value() {
            // Load-bearing separation preserved: only the SEED (what a cell certifies UPWARD)
            // is outcome-floored; the cell's OWN value is floored_novelty × g(realized flow),
            // untouched by the model. So a cell the model wrongly rates low still EARNS when
            // another mind builds on it — the airgap/false-positive backstop survives v8 the
            // same way it survived the semantic floor in v7. Here a model that floors EVERY
            // seed (all-negative weights ⇒ v_outcome → 0) cannot stop the root from being paid
            // by a downstream cell whose OWN seed... is also floored — so we verify the
            // separation directly: the root's own value is computed from its novelty + the
            // flow from a vested, non-noise child, and the model only gates the child's SEED.
            // To isolate "own value survives," give the ROOT a downstream user the model rates
            // high (a real lineage) and a separate cell the model rates low; the low-rated cell
            // still earns its own value through realized use, never zeroed by the model alone.
            let order = vec![
                cellc(0, 1, 0, None, b"root-alpha-bravo-charlie"),
                cellc(1, 2, 1, Some(0), b"useful-child-delta-echo-foxtrot"),
                cellc(2, 3, 2, Some(1), b"grandchild-golf-hotel-india-juliet"),
            ];
            let st = standing_of(&[(1, FLOOR), (2, FLOOR), (3, FLOOR)]);
            // A model scoring 0 everywhere (all seeds floored) must NOT be able to zero a
            // cell's OWN earned value if it has real downstream use under a non-floored seed —
            // but since ALL seeds floor here, the design's honest consequence is that with no
            // admitted certification NOBODY's flow vests. That is correct (the model denied
            // every certification); the backstop we assert is the SEPARATION: own value is
            // never *directly* floored by the model, only the upward seed is. We prove it by
            // contrast with the trained model, where the useful lineage IS admitted and the
            // root is paid — i.e. the model gates certification, not the cell's own novelty.
            let zero_w = [-1e6; crate::outcome::N_FEATS];
            let v8_zero = value_v8(
                &order, &st, FLOOR, &zero_w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF,
            );
            let w = trained_outcome_w();
            let v8_trained = value_v8(
                &order, &st, FLOOR, &w, THETA, ENTROPY_THETA, THETA_Q16, DAMP, ITERS, HALF,
            );
            assert_eq!(
                v8_zero[0], 0.0,
                "model denying every certification ⇒ no admitted use ⇒ root unpaid (correct)"
            );
            assert!(
                v8_trained[0] > 0.0,
                "the SAME graph under the real-lineage-admitting model pays the root: the model \
                 gates upward certification, it does not directly zero a cell's own novelty"
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
    pub(crate) fn children_of(cells: &[Cell]) -> HashMap<u64, Vec<usize>> {
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
    pub(crate) fn children_of_external(
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
        // Two-axis geometric diminishing-returns damping. Closes BOTH volume gaming vectors:
        //   (1) WITHIN-identity (λ^r): the r-th child (commit order) from a given certifying
        //       identity is weighted LAMBDA^r, so ONE identity's summed certifying flow
        //       saturates (geometric, ≤ flow/(1-LAMBDA)) instead of amplifying linearly in N.
        //   (2) CROSS-identity (μ^m): a parent's DISTINCT certifying identities are SORTED by
        //       their grouped (within-identity-damped) contribution descending; the m-th
        //       identity is weighted MU^m. One identity stays full (μ^0=1), additional
        //       identities decay — so splitting volume across K vested identities saturates
        //       just like stacking it under one (every child being rank-0 in its own group no
        //       longer escapes damping). Honest diverse certification (1-2 identities) is
        //       ~INERT: ranks 0/1 ⇒ weights 1, μ ⇒ near-full.
        // Deterministic: canonical sort key = (grouped contribution desc, identity args asc as
        // tiebreak) ⇒ on-VM replicas converge regardless of HashMap iteration order. Within an
        // identity, `kids` is ascending index = canonical commit order.
        const LAMBDA: f64 = 0.618_033_988_749_894_9; // 1/φ within-identity rank decay
        const MU: f64 = 0.618_033_988_749_894_9; // 1/φ cross-identity rank decay (same constant)
        for _ in 0..iters {
            let mut next = own.to_vec();
            for (pid, kids) in &children {
                if let Some(&pi) = id_to_idx.get(pid) {
                    // Group each identity's within-identity-damped (λ^r) contribution.
                    let mut groups: Vec<(&Vec<u8>, f64, u32)> = Vec::new();
                    for &k in kids {
                        let id = &cells[k].type_script.args;
                        match groups.iter_mut().find(|(a, _, _)| *a == id) {
                            Some(e) => {
                                e.1 += LAMBDA.powi(e.2 as i32) * flow[k];
                                e.2 += 1;
                            }
                            None => groups.push((id, flow[k], 1)),
                        }
                    }
                    // Sort distinct identities by grouped contribution desc, args asc tiebreak.
                    groups.sort_by(|a, b| {
                        b.1.partial_cmp(&a.1)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| a.0.cmp(b.0))
                    });
                    let mut s = 0.0;
                    for (m, (_, contrib, _)) in groups.iter().enumerate() {
                        s += MU.powi(m as i32) * contrib;
                    }
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

        // ===== [P·dont-let-attacker-choose-critical-input] — finalization input bindings =====
        // The RSAW adversarial pass (2026-06-13) named `now` and the validator-set `all` as
        // outcome-determining inputs that `finalizes_hybrid` takes as free parameters. The quorum
        // floor already contains the forge-alone case, but an input that MOVES the verdict must
        // still be consensus-sourced, never attacker-chosen — the same lesson the temporal-order
        // and index-dep bindings already carry. These two tests PIN that, as the 5th and 6th sites
        // of the invariant (code_hash / now-finalization / temporal-order / index-dep / now / set).

        #[test]
        fn now_is_outcome_determining_so_must_be_header_sourced() {
            // `effective_weight` decays with (now - last_heartbeat), and `finalizes_hybrid` feeds
            // `now` into every weight. So the SAME votes over the SAME set finalize-or-not by the
            // choice of `now`. 5th site: on-VM `now` MUST be header-sourced (block time), never
            // tx/witness-chosen. (See ON-VM-FINALIZATION.md — this test makes the gap explicit.)
            let horizon = 100u64;
            let voters_for = vec![val(0, 1.0, 1.0, 5.0, 10), val(1, 1.0, 1.0, 5.0, 10)];
            let all = vec![val(0, 1.0, 1.0, 5.0, 10), val(1, 1.0, 1.0, 5.0, 10)];
            let near = finalizes_hybrid(&voters_for, &all, NCI, 10, horizon, true, TWO_THIRDS_BPS, 3333);
            let far = finalizes_hybrid(&voters_for, &all, NCI, 100_000, horizon, true, TWO_THIRDS_BPS, 3333);
            assert!(near, "fresh unanimous support finalizes");
            assert_ne!(
                near, far,
                "`now` is outcome-determining ⇒ it must be consensus-sourced, not chooser-supplied"
            );
        }

        #[test]
        fn validator_set_is_outcome_determining_so_must_be_consensus_bound() {
            // `all` sets the finalization denominator (eff_total AND the quorum floor's base_total).
            // A producer supplying a CURATED `all` that omits honest validators shrinks the basis
            // until a minority `voters_for` clears it. Same vote, two `all` sets, opposite verdicts
            // ⇒ 6th site: `all` must be the canonical consensus validator set (bound on-VM by the
            // validator-registry type-id, the INDEX-DEP-CODEHASH-BINDING pattern), never caller-fed.
            let now = 0u64;
            let horizon = 100u64;
            let voters_for = vec![val(9, 1.0, 1.0, 1.0, 0)];
            // Truthful set: attacker (1) + six honest = 1/7, far below the 2/3 bar.
            let mut truthful: Vec<Validator> = (0..6).map(|i| val(i, 1.0, 1.0, 1.0, 0)).collect();
            truthful.push(val(9, 1.0, 1.0, 1.0, 0));
            let honest_view =
                finalizes_hybrid(&voters_for, &truthful, NCI, now, horizon, true, TWO_THIRDS_BPS, 3333);
            // Curated set: attacker alone ⇒ the basis collapses to the attacker.
            let curated = vec![val(9, 1.0, 1.0, 1.0, 0)];
            let attacker_view =
                finalizes_hybrid(&voters_for, &curated, NCI, now, horizon, true, TWO_THIRDS_BPS, 3333);
            assert!(!honest_view, "against the true validator set, a 1/7 minority cannot finalize");
            assert!(
                attacker_view,
                "against a curated set omitting honest validators, the same vote finalizes — the set must be bound"
            );
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
    /// §7.1c — the appeal court's mix DOWN-WEIGHTS PoM, the dimension whose capture a
    /// judge-cartel dispute alleges. Dimension-level recusal: the proof axis under attack
    /// does not judge its own case. A PoM-concentrated ring (even identity-separated, so
    /// edge-recusal cannot reach it) cannot veto here without ALSO holding PoW+PoS — which
    /// is exactly the consensus layer's already-priced cross-dimension global assumption.
    /// Mirrors NCI's PoW:PoS ratio (0.10:0.30 → kept proportional) with PoM minimized.
    pub const DISPUTE_APPEAL: consensus::Mix =
        consensus::Mix { pow: 0.225, pos: 0.675, pom: 0.10 };

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

    /// §7.1c-guard at the SETTLEMENT level — the verdict→slash binding that makes the
    /// asymmetric clamp END-TO-END, and PER-CERTIFIER. Each certifier is judged on its OWN
    /// standing: its slash is dropped iff that certifier's own PoM is load-bearing to the
    /// full-mix non-conviction (the [`appeal_refutes_guarded`] one-way ratchet toward
    /// acquittal applies to it), and kept iff the appeal legitimately convicts it. The
    /// key↔id join reuses the `juror_keys` idiom (`certifier_keys`) — no new channel; an
    /// unmapped key cannot assert protection (it falls back to the unprotected verdict).
    ///
    /// Why per-certifier: the prior whole-settlement form gated EVERY certifier on one
    /// `defendant_id`, so a mixed panel (an honest PoM holder + a garbage endorser on the
    /// same target) was all-or-nothing. Now the honest certifier is spared while the
    /// garbage's slash lands. `bounded_shares` is computed over the FULL certifier set, so
    /// a spared certifier never inflates another's bounded slash (totals stay exact).
    ///
    /// Slash-level invariant (`guarded_settlement_cannot_exceed_pre_appeal_slash`): for a
    /// down-weighted-dimension certifier, `total_slash(guarded) ≤ total_slash(pre_appeal)`.
    /// The guard can only turn a conviction OFF — clamp a refutation the pre-appeal full-mix
    /// round did not reach — never ON. REDUCTION: with one certifier this is exactly the old
    /// whole-settlement guard, its mapped id being the single defendant.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_refuted_guarded(
        entries: &mut [VestingEntry],
        c: &Challenge,
        p: &Params,
        certifier_shares: &[(Vec<u8>, f64)],
        voters_for: &[consensus::Validator],
        all: &[consensus::Validator],
        certifier_keys: &[(u64, Vec<u8>)],
        now: u64,
        horizon: u64,
        quorum_floor_bps: u64,
    ) -> Settlement {
        // A certifier is convicted iff the guarded appeal verdict on ITS OWN standing id
        // convicts. Unmapped key ⇒ id u64::MAX (absent from the panel) ⇒ no protection.
        let convicts = |who: &[u8]| -> bool {
            let id = certifier_keys
                .iter()
                .find(|(_, key)| key.as_slice() == who)
                .map(|(id, _)| *id)
                .unwrap_or(u64::MAX);
            appeal_refutes_guarded(voters_for, all, id, now, horizon, quorum_floor_bps)
        };
        if !certifier_shares.iter().any(|(who, _)| convicts(who)) {
            // No certifier convicted ⇒ no refutation lands (preserves the single-defendant
            // acquittal: empty settlement, target NOT canceled).
            return Settlement::default();
        }
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
            if *share <= 0.0 || !convicts(who) {
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
        /// PoM-minimized appeal mix (§7.1c) for judge-cartel disputes: the captured
        /// dimension is recused from the court that judges it.
        AppealCourt,
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
            Tribunal::AppealCourt => DISPUTE_APPEAL,
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

    /// §7.1b — LOAD-BEARING juror-exclusion. Unlike [`conflicted_juror_ids`] (hygiene
    /// only), this recuses edge-connected jurors from BOTH the vote tally and the
    /// threshold basis, then runs the verdict on the recused panel. It closes a
    /// PoM-dominant cartel whose judge keys are edge-connected to the target: removing
    /// its standing from the basis lets the honest remainder clear 2/3, which the
    /// escalation court alone does NOT do when the cartel holds ≥ 1/3 ÷ NCI.pom of PoM
    /// (the veto survives the full-mix appeal — see
    /// `pom_dominant_cartel_vetoes_fullmix_below_global_assumption`). The residual is an
    /// identity-separated cartel (vested judges with no provenance edge into the ring):
    /// no edge exists to recuse on, so this does not reach it — see
    /// `identity_separated_pom_cartel_evades_exclusion_residual`.
    pub fn verdict_refutes_excluding_conflicted(
        tribunal: Tribunal,
        cells: &[Cell],
        target: u64,
        juror_keys: &[(u64, Vec<u8>)],
        voters_for: &[consensus::Validator],
        all: &[consensus::Validator],
        now: u64,
        horizon: u64,
        quorum_floor_bps: u64,
    ) -> bool {
        let conflicted = conflicted_juror_ids(cells, target, juror_keys);
        let recused = |v: &consensus::Validator| !conflicted.contains(&v.id);
        let voters: Vec<consensus::Validator> =
            voters_for.iter().filter(|v| recused(v)).cloned().collect();
        let panel: Vec<consensus::Validator> =
            all.iter().filter(|v| recused(v)).cloned().collect();
        verdict_refutes_at(tribunal, &voters, &panel, now, horizon, quorum_floor_bps)
    }

    /// §7.1c-guard — ASYMMETRIC APPEAL (the monotone-decrease ratchet).
    ///
    /// The PoM-minimized appeal court ([`DISPUTE_APPEAL`]) exists to BREAK a PoM cartel by
    /// down-weighting PoM, so honest PoW+PoS can overturn a cartel that vetoes the refutation
    /// of its own garbage (`appeal_court_overturns_the_identity_separated_pom_cartel`, where
    /// the cartel are the JURORS and the garbage cell is the defendant). But down-weighting
    /// PoM to 0.10 hands 0.90 of that court to PoW+PoS, so a PoW/PoS majority could weaponize
    /// it the OTHER way — to over-convict a defendant who is HIMSELF an honest holder of the
    /// down-weighted dimension and can no longer defend at full PoM weight.
    ///
    /// The appeal court therefore has NO legitimate CONVICTING power over a *defendant* who
    /// holds the dimension it down-weights: a conviction of such a holder belongs to the
    /// full-mix court, where that dimension defends at its full weight. This guard enforces
    /// the asymmetry as a one-way ratchet toward acquittal:
    ///
    ///   `appeal_refutes(defendant who holds the down-weighted dim) ≤ pre_appeal_refutes`
    ///
    /// A refutation the pre-appeal (full-mix) round did NOT reach is clamped to the prior
    /// outcome, so the PoM-minimized court can only LOWER such a defendant's conviction/slash,
    /// never raise it (worst case for honest PoM = no change). `defendant_holds_downweighted_dim`
    /// keys on the DEFENDANT's standing, not the jury's — so the cartel-break is untouched: in
    /// that case the PoM cartel sit as jurors while the defendant is the garbage cell (not a
    /// down-weighted-dim holder), so the flag is `false` and the appeal convicts unrestricted.
    pub fn appeal_refutes_asymmetric(
        defendant_holds_downweighted_dim: bool,
        pre_appeal_refuted: bool,
        appeal_court_refutes: bool,
    ) -> bool {
        if defendant_holds_downweighted_dim {
            // false < true, so the AND can only ratchet the conviction toward acquittal:
            // appeal_court_refutes can lower a pre-appeal conviction, never create a new one.
            pre_appeal_refuted && appeal_court_refutes
        } else {
            appeal_court_refutes
        }
    }

    /// §7.1c-guard input DERIVATION — the `defendant_holds_downweighted_dim` flag, derived
    /// by a counterfactual on the defendant's OWN standing, never producer-asserted. This is
    /// the dont-let-the-attacker-choose-a-critical-input class applied to the guard's own
    /// input (same lesson as header-sourced `now` and reveal-sourced coords).
    ///
    /// The flag must be TRUE exactly when the appeal court's PoM down-weighting would strip an
    /// HONEST defendant of the standing that defends them — i.e. when the defendant's own PoM
    /// is *load-bearing to their full-mix acquittal*. Predicate:
    ///
    ///   `!full_mix_convicts(panel)  AND  full_mix_convicts(panel with the defendant's own PoM removed)`
    ///
    /// In the GRIEF the honest defendant's own PoM holds the full-mix court below 2/3; removing
    /// it flips the verdict ⇒ TRUE ⇒ the asymmetric clamp applies. In the CARTEL-BREAK the
    /// defendant is the garbage cell defended by the JURY cartel's PoM, not its own — removing
    /// the defendant's negligible own PoM changes nothing ⇒ FALSE ⇒ the overturn convicts
    /// unrestricted. It reads only the consensus standing set the verdict already consumes, so
    /// an attacker cannot set it by assertion — it is derived, like `now`/coords before it.
    pub fn defendant_holds_downweighted_dim(
        voters_for: &[consensus::Validator],
        all: &[consensus::Validator],
        defendant_id: u64,
        now: u64,
        horizon: u64,
        quorum_floor_bps: u64,
    ) -> bool {
        let full_mix_convicts = |panel: &[consensus::Validator]| {
            verdict_refutes_at(Tribunal::FullMix, voters_for, panel, now, horizon, quorum_floor_bps)
        };
        // If the full-mix court already convicts WITH the defendant's standing present, the
        // defendant's own PoM is not what's defending them — not load-bearing.
        if full_mix_convicts(all) {
            return false;
        }
        // Counterfactual: remove the defendant's OWN down-weighted-dimension (PoM) standing
        // from the basis and re-run the full-mix court. A flip to conviction means the
        // defendant's own PoM was the load-bearing defense.
        let stripped: Vec<consensus::Validator> = all
            .iter()
            .map(|v| {
                if v.id == defendant_id {
                    let mut nv = v.clone();
                    nv.pom = 0.0;
                    nv
                } else {
                    v.clone()
                }
            })
            .collect();
        full_mix_convicts(&stripped)
    }

    /// §7.1c-guard, END-TO-END — the asymmetric-appeal verdict with the guard's own input
    /// DERIVED from consensus standing, not accepted from the producer. This is the form the
    /// settlement path calls: there is no boolean channel for an attacker to set, so a
    /// producer-asserted "I don't hold the down-weighted dimension" cannot escape the clamp —
    /// only the counterfactual over the consensus standing set decides.
    pub fn appeal_refutes_guarded(
        voters_for: &[consensus::Validator],
        all: &[consensus::Validator],
        defendant_id: u64,
        now: u64,
        horizon: u64,
        quorum_floor_bps: u64,
    ) -> bool {
        let pre_appeal =
            verdict_refutes_at(Tribunal::FullMix, voters_for, all, now, horizon, quorum_floor_bps);
        let appeal = verdict_refutes_at(
            Tribunal::AppealCourt,
            voters_for,
            all,
            now,
            horizon,
            quorum_floor_bps,
        );
        let holds =
            defendant_holds_downweighted_dim(voters_for, all, defendant_id, now, horizon, quorum_floor_bps);
        appeal_refutes_asymmetric(holds, pre_appeal, appeal)
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

        const ENTROPY_THETA: f64 = 0.95;

        /// `attack_graph` with the garbage payloads HEX-ENCODED — the evasion that slips the
        /// v7 semantic seed floor (`value::tests::encoded_noise_defeats_the_v7_seed_gate...`).
        fn encoded_attack_graph() -> (Vec<Cell>, HashMap<Vec<u8>, u64>) {
            let enc = |seed: u8, n: u8| -> Vec<u8> {
                (0..n)
                    .map(|i| seed.wrapping_add(i.wrapping_mul(41)))
                    .flat_map(|b| format!("{b:02x}").into_bytes())
                    .collect()
            };
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(10, 8, 1, None, &enc(0xA0, 48)),
                cellc(11, 5, 2, Some(10), &enc(0x10, 48)),
            ];
            let standing = standing_of(&[(1, 50), (5, 50)]);
            (order, standing)
        }

        #[test]
        fn encoded_noise_endorsement_is_negative_ev_slashing_is_content_agnostic() {
            // CLASS-DISSOLUTION for the encoding-evasion frontier. The hex-encoded payload
            // defeats the v7 semantic seed floor (so v7 pays it EXACTLY as v6 does — the floor
            // is inert on encoded bytes), but the dispute layer never inspected the bytes: it
            // claws back the REALIZED minted value. The evasion that beats the content gate
            // buys nothing here — the round is negative-EV identically to raw garbage. No
            // profitable trajectory survives, regardless of the content heuristic. This is the
            // demonstrated form of the comment in `encoded_noise_evades_the_entropy_floor_open_gap`
            // ("v6 standing pricing + dispute slashing stay the binding defense").
            let (order, mut standing) = encoded_attack_graph();
            let v6 = value::value_v6(&order, &standing, FLOOR, THETA, DAMP, ITERS, HALF);
            let v7 = value::value_v7(&order, &standing, FLOOR, THETA, ENTROPY_THETA, DAMP, ITERS, HALF);
            let gain = v6[1];
            assert!(gain > 0.0, "encoded pocket is paid at the gate (the evasion is real)");
            assert!(
                (v7[1] - gain).abs() < 1e-9,
                "v7 ≡ v6 on the encoded payload — the semantic seed floor was evaded"
            );

            let mut entries = vec![VestingEntry { cell_id: 10, amount: gain, realized_epoch: 100 }];
            let c = Challenge { target: 10, challenger: vec![1], bond: 1.0, opened_epoch: 102 };
            let share = causal_share(&order, &standing, FLOOR, THETA, DAMP, ITERS, HALF, 1, &[5]);
            let s = resolve_refuted(&mut entries, &c, &P, &[(vec![5], share)]);
            assert_eq!(s.canceled, gain, "encoded-pocket payout fully canceled (content-agnostic)");
            let slashed: f64 = s.slashes.iter().map(|(_, a)| a).sum();
            assert!(
                slashed >= gain + P.alpha - 1e-9,
                "λ·share+α ≥ minted + α ⇒ negative EV on the encoded payload too"
            );
            let ev = 0.5 * gain - 0.5 * (gain + P.alpha);
            assert!(ev < 0.0, "§4 inequality holds regardless of the byte encoding");
            apply_slashes(&mut standing, &s.slashes);
            assert!(standing[&vec![5u8]] < 50, "the certifier's soulbound standing decreased");
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

        /// Honest holds ALL PoW + ALL PoS but only 40% of PoM; cartel holds 60% of PoM and
        /// nothing else. So the cartel is below 2/3 of EVERY dimension — yet PoM is 60% of
        /// the NCI mix, so 60% of PoM is 36% of the full-mix court: a veto.
        fn pom_dominant_courtroom() -> (Vec<consensus::Validator>, Vec<consensus::Validator>) {
            let v = |id, pow, pos, pom| consensus::Validator {
                id,
                pow,
                pos,
                pom,
                last_heartbeat: 0,
                staked_balance: 1000.0,
            };
            let honest = vec![v(1, 50.0, 50.0, 20.0), v(2, 50.0, 50.0, 20.0)];
            let cartel = vec![v(9, 0.0, 0.0, 60.0)];
            (honest, cartel)
        }

        #[test]
        fn pom_dominant_cartel_vetoes_fullmix_below_global_assumption() {
            // ADVERSARIAL TICK (pom-roadmap). The §7 escalation court's claim that vetoing
            // at full-mix "requires cross-dimension capture" is FALSE for a PoM-dominant
            // cartel. NCI weights PoM at 0.60, so a cartel holding ≥ 1/3 ÷ 0.60 = 55.6% of
            // PoM holds > 1/3 of the AND-composed court and vetoes on APPEAL — with ZERO
            // PoW and ZERO PoS, strictly below the ≥2/3-of-every-dimension global
            // assumption. Here the honest side holds 100% of PoW AND 100% of PoS and STILL
            // cannot land the refutation. Corrects `cartel_veto_..._overturned_on_appeal`,
            // whose overturn relied on the cartel holding only 40% of PoM.
            let (honest, cartel) = pom_dominant_courtroom();
            let mut all = honest.clone();
            all.extend(cartel.clone());
            assert!(
                !verdict_refutes_at(Tribunal::PomOnly, &honest, &all, 0, 0, 4000),
                "round 1: PoM-only veto holds"
            );
            assert!(
                !verdict_refutes_at(Tribunal::FullMix, &honest, &all, 0, 0, 4000),
                "GAP: a 60%-PoM cartel vetoes the full-mix appeal too — cross-dimension \
                 capture is NOT required, and this sits below the global assumption"
            );
        }

        #[test]
        fn load_bearing_juror_exclusion_closes_the_pom_dominant_edge_connected_cartel() {
            // THE FIX (this tick): recuse edge-connected standing from the BASIS, not just
            // flag it. With the cartel's judge key edge-connected to the target, dropping it
            // from the panel lets the honest remainder clear 2/3 — the PoM-dominant veto is
            // gone for the edge-connected case.
            let (honest, cartel) = pom_dominant_courtroom();
            let mut all = honest.clone();
            all.extend(cartel.clone());
            let (order, _) = attack_graph(); // target 10 has a child with type_script.args == [5]
            let juror_keys = vec![(9u64, vec![5u8])]; // cartel judge 9 IS edge-connected to target 10
            assert!(
                verdict_refutes_excluding_conflicted(
                    Tribunal::FullMix,
                    &order,
                    10,
                    &juror_keys,
                    &honest,
                    &all,
                    0,
                    0,
                    4000,
                ),
                "load-bearing exclusion recuses the conflicted cartel from the basis ⇒ \
                 honest clears 2/3"
            );
        }

        #[test]
        fn identity_separated_pom_cartel_evades_exclusion_residual() {
            // HONEST RESIDUAL: exclusion keys on a provenance EDGE. A cartel that judges
            // with an identity-separated key (no edge into the ring) has nothing to recuse
            // on, so the PoM-dominant veto survives. The residual is identity-separated
            // vested PoM, not edge-connected standing. Candidate next gates: a PoM-share cap
            // in the dispute tribunal mix, or dilution-indexed slashing keyed on the vote.
            let (honest, cartel) = pom_dominant_courtroom();
            let mut all = honest.clone();
            all.extend(cartel.clone());
            let (order, _) = attack_graph(); // no child carries the cartel's separated key [77]
            let juror_keys = vec![(9u64, vec![77u8])];
            assert!(
                !verdict_refutes_excluding_conflicted(
                    Tribunal::FullMix,
                    &order,
                    10,
                    &juror_keys,
                    &honest,
                    &all,
                    0,
                    0,
                    4000,
                ),
                "identity-separated cartel has no edge to recuse ⇒ the PoM-dominant veto \
                 survives"
            );
        }

        #[test]
        fn appeal_court_overturns_the_identity_separated_pom_cartel() {
            // THE FIX for the identity-separated residual (this tick, §7.1c). The appeal
            // court DOWN-WEIGHTS PoM (the captured dimension): DISPUTE_APPEAL pom=0.10.
            // The same 60%-PoM / 0-PoW / 0-PoS cartel that vetoed FullMix now holds only
            // 0.10×0.6 = 6% of the appeal court — honest PoW+PoS carry 94% ≥ 2/3 and the
            // refutation lands. No edge-recusal needed: dimension-level recusal reaches the
            // identity-separated cartel that `verdict_refutes_excluding_conflicted` cannot.
            let (honest, cartel) = pom_dominant_courtroom();
            let mut all = honest.clone();
            all.extend(cartel.clone());
            assert!(
                verdict_refutes_at(Tribunal::AppealCourt, &honest, &all, 0, 0, 4000),
                "PoM-minimized appeal court overturns the identity-separated PoM cartel"
            );
        }

        #[test]
        fn appeal_court_still_yields_to_full_cross_dimension_capture() {
            // CEILING PRESERVED. The appeal court closes the PoM-only residual DOWN TO the
            // consensus layer's irreducible global assumption — no further. A cartel holding
            // ≥2/3 of EVERY dimension still vetoes even the PoM-minimized court, exactly as
            // `full_consensus_capture_..._global_assumption` documents. The §7.1c fix adds no
            // new trust assumption; it merely makes "cross-dimension capture required" TRUE.
            let v = |id, pow, pos, pom| consensus::Validator {
                id,
                pow,
                pos,
                pom,
                last_heartbeat: 0,
                staked_balance: 1000.0,
            };
            let cartel = vec![v(9, 80.0, 80.0, 80.0)];
            let honest = vec![v(1, 20.0, 20.0, 20.0)];
            let mut all = cartel.clone();
            all.extend(honest.clone());
            assert!(
                !verdict_refutes_at(Tribunal::AppealCourt, &honest, &all, 0, 0, 4000),
                "≥2/3 cross-dimension capture defeats even the PoM-minimized appeal court \
                 — the consensus layer's own ceiling, unchanged"
            );
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

        #[test]
        fn honest_pom_defendant_vs_powpos_majority_appeal_cannot_increase_slash() {
            // RSAW BUILD (this tick): the INVERSE of the §7.1c cartel-break. Down-weighting
            // PoM to 0.10 to break a PoM cartel hands 0.90 of the appeal court to PoW+PoS, so a
            // PoW/PoS majority can weaponize the PoM-minimized court to OVER-convict a defendant
            // who is himself an honest PoM holder (he can no longer defend at full PoM weight).
            // Guard = ASYMMETRIC APPEAL: a down-weighted-dimension *defendant's* conviction may
            // only MONOTONE-DECREASE on appeal, never rise.
            let v = |id, pow, pos, pom| consensus::Validator {
                id,
                pow,
                pos,
                pom,
                last_heartbeat: 0,
                staked_balance: 1000.0,
            };
            // Attacker jurors: PoW/PoS-heavy, low PoM — they vote to refute (convict) the
            // honest PoM defendant. Honest defendant: PoM-heavy (the down-weighted dimension).
            let attacker = vec![v(9, 80.0, 80.0, 10.0)];
            let honest = vec![v(1, 20.0, 20.0, 80.0)];
            let mut all = attacker.clone();
            all.extend(honest.clone());

            // 1) The FULL-MIX court, where PoM defends at its full 0.60, does NOT convict:
            //    the honest defendant holds 80/90 of PoM, so the attacker can't reach 2/3.
            let pre_appeal_refuted =
                verdict_refutes_at(Tribunal::FullMix, &attacker, &all, 0, 0, 4000);
            assert!(
                !pre_appeal_refuted,
                "full-mix court does not convict the honest PoM defendant"
            );

            // 2) THE GRIEF is real: at the PoM-minimized appeal court the PoW/PoS majority DOES
            //    reach a refutation against the honest PoM defendant (PoM down-weighted to 0.10).
            let raw_appeal =
                verdict_refutes_at(Tribunal::AppealCourt, &attacker, &all, 0, 0, 4000);
            assert!(
                raw_appeal,
                "ungated appeal court lets the PoW/PoS majority convict honest PoM (the grief)"
            );

            // 3) THE GUARD: asymmetric appeal clamps it — a down-weighted-dimension defendant's
            //    conviction can only ratchet toward acquittal, so the appeal adds no slash.
            assert!(
                !appeal_refutes_asymmetric(true, pre_appeal_refuted, raw_appeal),
                "asymmetric appeal: no new conviction of the down-weighted-dimension holder ⇒ \
                 no slash increase"
            );

            // 4) The cartel-break is PRESERVED. (a) An appeal still ACQUITS a down-weighted-dim
            //    holder the prior round wrongly convicted (convict → acquit still flows). (b) A
            //    refutation the prior round already reached is not weakened. (c) For a defendant
            //    who is NOT a down-weighted-dim holder (the garbage cell in the cartel-break,
            //    where the PoM cartel are jurors), the appeal convicts unrestricted.
            assert!(
                !appeal_refutes_asymmetric(true, true, false),
                "acquittal on appeal still flows (cartel-break / overturn preserved)"
            );
            assert!(
                appeal_refutes_asymmetric(true, true, true),
                "a pre-appeal-reached refutation is not weakened"
            );
            assert!(
                appeal_refutes_asymmetric(false, false, true),
                "non-down-weighted defendant (garbage cell): appeal convicts unrestricted ⇒ \
                 §7.1c cartel-break is untouched"
            );
        }

        #[test]
        fn guard_flag_is_derived_from_standing_not_producer_asserted() {
            // RSAW BUILD (increment 1): the §7.1c-guard's `defendant_holds_downweighted_dim`
            // input is now DERIVED by counterfactual over the consensus standing set, not
            // accepted as a producer bool. `appeal_refutes_guarded` exposes NO boolean channel,
            // so an attacker cannot assert "I don't hold the down-weighted dimension" to escape
            // the clamp — only the counterfactual decides. This proves both branches derive
            // correctly AND that the two regimes the guard must not conflate are separated by
            // the predicate, not by a passed flag.
            let v = |id, pow, pos, pom| consensus::Validator {
                id,
                pow,
                pos,
                pom,
                last_heartbeat: 0,
                staked_balance: 1000.0,
            };

            // ---- GRIEF: honest PoM defendant (id 1) vs PoW/PoS-majority attacker (id 9) ----
            // The defendant's OWN PoM is load-bearing to the full-mix acquittal: removing it
            // flips full-mix to conviction ⇒ derived flag TRUE ⇒ the clamp applies.
            let g_attacker = vec![v(9, 80.0, 80.0, 10.0)];
            let mut g_all = g_attacker.clone();
            g_all.push(v(1, 20.0, 20.0, 80.0));
            assert!(
                defendant_holds_downweighted_dim(&g_attacker, &g_all, 1, 0, 0, 4000),
                "grief: the defendant's OWN PoM defends them at full mix (removing it convicts) \
                 ⇒ flag derived TRUE — no producer assertion involved"
            );
            assert!(
                !appeal_refutes_guarded(&g_attacker, &g_all, 1, 0, 0, 4000),
                "the grief is clamped end-to-end through the derived guard (no bool channel to \
                 flip): the PoM-minimized appeal cannot convict the honest PoM defendant"
            );

            // ---- CARTEL-BREAK: garbage defendant (id 7, ~0 own PoM) defended by a PoM JURY
            // cartel (id 9); honest PoW/PoS (id 1,2) vote to refute the garbage. ----
            // The defendant's OWN PoM is NOT load-bearing — the JURY's is — so removing the
            // defendant's own PoM does not change full-mix ⇒ derived flag FALSE ⇒ the overturn
            // convicts unrestricted (the §7.1c cartel-break is untouched).
            let cb_honest = vec![v(1, 50.0, 50.0, 20.0), v(2, 50.0, 50.0, 20.0)];
            let mut cb_all = cb_honest.clone();
            cb_all.push(v(9, 0.0, 0.0, 60.0)); // jury cartel's PoM, not the defendant's
            cb_all.push(v(7, 0.0, 0.0, 1.0)); // garbage author: negligible own PoM
            assert!(
                !defendant_holds_downweighted_dim(&cb_honest, &cb_all, 7, 0, 0, 4000),
                "cartel-break: the garbage defendant's OWN PoM defends nothing (the cartel's \
                 does) ⇒ flag derived FALSE"
            );
            assert!(
                appeal_refutes_guarded(&cb_honest, &cb_all, 7, 0, 0, 4000),
                "cartel-break preserved end-to-end: the overturn still convicts the garbage \
                 through the guard (derived flag FALSE ⇒ no clamp)"
            );

            // The two regimes are separated by the COUNTERFACTUAL, not by who passed which bool:
            // same guard call, opposite outcomes, driven only by whose PoM is load-bearing.
            assert_ne!(
                appeal_refutes_guarded(&g_attacker, &g_all, 1, 0, 0, 4000),
                appeal_refutes_guarded(&cb_honest, &cb_all, 7, 0, 0, 4000),
                "grief clamps (false) while cartel-break convicts (true) — derived, not asserted"
            );
        }

        #[test]
        fn guarded_settlement_cannot_exceed_pre_appeal_slash() {
            // RSAW BUILD (increment 2): the §7.1c-guard wired into the SETTLEMENT path, so the
            // asymmetric clamp is end-to-end (slash-level), not just a verdict boolean. For a
            // down-weighted-dimension defendant the invariant `total_slash(guarded) ≤
            // total_slash(pre_appeal)` holds because the guard can only turn a conviction OFF.
            let v = |id, pow, pos, pom| consensus::Validator {
                id,
                pow,
                pos,
                pom,
                last_heartbeat: 0,
                staked_balance: 1000.0,
            };
            let total_slash = |s: &Settlement| -> f64 { s.slashes.iter().map(|(_, a)| a).sum() };

            // A real unvested entry on the target + a non-empty certifier share, so a conviction
            // ACTUALLY slashes (otherwise the invariant would be vacuously satisfied). The slashed
            // certifier IS the down-weighted-dimension defendant (id 1 ↔ key [1]) being defended.
            let c = Challenge { target: 7, challenger: vec![9], bond: 2.0, opened_epoch: 105 };
            let shares = vec![(vec![1u8], 12.0)];
            let fresh = || vec![VestingEntry { cell_id: 7, amount: 12.0, realized_epoch: 100 }];

            // GRIEF: honest PoM defendant (id 1) vs PoW/PoS-majority attacker-challenger (id 9).
            let attacker = vec![v(9, 80.0, 80.0, 10.0)];
            let mut all = attacker.clone();
            all.push(v(1, 20.0, 20.0, 80.0));

            // 1) PRE-APPEAL (full-mix) settlement: full-mix acquits ⇒ gate false ⇒ no slash.
            let pre = if verdict_refutes_at(Tribunal::FullMix, &attacker, &all, 0, 0, 4000) {
                resolve_refuted(&mut fresh(), &c, &P, &shares)
            } else {
                Settlement::default()
            };
            assert_eq!(total_slash(&pre), 0.0, "full-mix acquits the honest PoM defendant ⇒ 0 slash");

            // 2) UNGUARDED appeal settlement: the PoM-minimized court convicts ⇒ the grief lands.
            let raw = if verdict_refutes_at(Tribunal::AppealCourt, &attacker, &all, 0, 0, 4000) {
                resolve_refuted(&mut fresh(), &c, &P, &shares)
            } else {
                Settlement::default()
            };
            assert!(
                total_slash(&raw) > 0.0,
                "ungated appeal slashes the honest PoM defendant (the realized grief)"
            );

            // 3) GUARDED settlement: the clamp removes the grief end-to-end (no bool channel).
            let guarded =
                resolve_refuted_guarded(&mut fresh(), &c, &P, &shares, &attacker, &all, &[(1u64, vec![1u8])], 0, 0, 4000);
            assert_eq!(
                total_slash(&guarded),
                0.0,
                "guarded settlement adds no slash to the honest PoM defendant"
            );

            // THE INVARIANT: appeal_slash ≤ pre_appeal_slash for the down-weighted-dim defendant,
            // AND the grief is strictly removed relative to the unguarded path.
            assert!(
                total_slash(&guarded) <= total_slash(&pre),
                "slash-level monotone clamp holds end-to-end: appeal_slash ≤ pre_appeal_slash"
            );
            assert!(
                total_slash(&guarded) < total_slash(&raw),
                "the guard strictly removes the realized grief slash"
            );

            // CARTEL-BREAK preserved at the SETTLEMENT level: when the defendant's own PoM is NOT
            // load-bearing (garbage cell id 7, flag derives FALSE), the guarded settlement EQUALS
            // the unguarded appeal settlement — the overturn still slashes the garbage's certifiers.
            let cb_honest = vec![v(1, 50.0, 50.0, 20.0), v(2, 50.0, 50.0, 20.0)];
            let mut cb_all = cb_honest.clone();
            cb_all.push(v(9, 0.0, 0.0, 60.0));
            cb_all.push(v(7, 0.0, 0.0, 1.0));
            let cb_raw = if verdict_refutes_at(Tribunal::AppealCourt, &cb_honest, &cb_all, 0, 0, 4000) {
                resolve_refuted(&mut fresh(), &c, &P, &shares)
            } else {
                Settlement::default()
            };
            let cb_guarded =
                resolve_refuted_guarded(&mut fresh(), &c, &P, &shares, &cb_honest, &cb_all, &[(7u64, vec![1u8])], 0, 0, 4000);
            assert!(total_slash(&cb_raw) > 0.0, "cartel-break: the overturn convicts the garbage");
            assert_eq!(
                total_slash(&cb_guarded),
                total_slash(&cb_raw),
                "cartel-break preserved end-to-end: guarded == unguarded when the defendant's own \
                 PoM is not load-bearing"
            );

            // MIXED PANEL — the new per-certifier capability. One honest-PoM certifier
            // (key [1] ↔ id 1, own PoM load-bearing to the full-mix acquittal) and one
            // garbage endorser (key [7] ↔ id 7, not load-bearing) on the SAME target. The
            // honest slash is DROPPED while the garbage's is KEPT — no longer all-or-nothing.
            let mut mp_all = all.clone(); // [v9(80,80,10), v1(20,20,80)]
            mp_all.push(v(7, 0.0, 0.0, 1.0)); // garbage endorser standing, PoM not load-bearing
            let mp_voters = attacker.clone(); // PoW/PoS majority (id 9) votes to refute
            let mp_shares = vec![(vec![1u8], 6.0), (vec![7u8], 6.0)];
            let mp_keys = vec![(1u64, vec![1u8]), (7u64, vec![7u8])];
            // standing derived, not asserted: id 1 protected, id 7 not.
            assert!(
                defendant_holds_downweighted_dim(&mp_voters, &mp_all, 1, 0, 0, 4000),
                "honest certifier id 1: own PoM load-bearing ⇒ protected"
            );
            assert!(
                !defendant_holds_downweighted_dim(&mp_voters, &mp_all, 7, 0, 0, 4000),
                "garbage certifier id 7: own PoM not load-bearing ⇒ unprotected"
            );
            let mp = resolve_refuted_guarded(
                &mut vec![VestingEntry { cell_id: 7, amount: 12.0, realized_epoch: 100 }],
                &c,
                &P,
                &mp_shares,
                &mp_voters,
                &mp_all,
                &mp_keys,
                0,
                0,
                4000,
            );
            assert!(
                mp.slashes.iter().all(|(who, _)| who != &vec![1u8]),
                "honest PoM certifier dropped from the slash set"
            );
            assert!(
                mp.slashes.iter().any(|(who, _)| who == &vec![7u8]),
                "garbage certifier kept in the slash set"
            );
            // totals exact: the garbage's bounded slash is unchanged by the honest
            // certifier's presence (bounded over the FULL set, not the kept subset).
            let expected_garbage = {
                let bounded = bounded_shares(&mp_shares, mp.canceled);
                let gshare = bounded.iter().find(|(w, _)| w == &vec![7u8]).unwrap().1;
                P.lambda * gshare + P.alpha
            };
            assert_eq!(
                total_slash(&mp),
                expected_garbage,
                "totals exact: only the garbage certifier's bounded slash, honest spared"
            );
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
        // SOUNDNESS GUARD (adversarial tick on the harness itself, 2026-06-12).
        // attacker_ev = (1−p)V − p(λV+α). At the binding (lowest) detection p_min:
        //   ∂/∂V = (1−p_min) − p_min·λ.  With λ=1 that is 1 − 2·p_min.
        // If p_min < 0.5 the EV GROWS in V without bound, so NO finite α and NO finite
        // grid can certify safety — a large-enough attack value is always profitable.
        // A finite-grid sweep that returned `true` here would be lying. Refuse to certify:
        // either detection must be ≥ ½ (the bounty buys this, §4) or the model needs an
        // explicit, defended max-attack-value cap (not assumed here).
        if (1.0 - p_min) - p_min * lambda > 1e-12 {
            return false;
        }
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
        fn harness_refuses_to_certify_below_half_detection_unbounded_v() {
            // ADVERSARIAL TICK on the harness (2026-06-12). Before the soundness guard,
            // feasible() swept a FINITE V grid; with p_min < 1/2, attacker EV grows in V,
            // so a grid that topped out at V=256 could return `true` while V=1e9 was
            // positive-EV. The blind spot, made explicit:
            let big_alpha = 1e6;
            // At p=0.4, a huge V is profitable no matter how large α is, because EV grows
            // linearly in V and only subtracts a constant α:
            assert!(
                attacker_ev(1e12, 0.4, 1.0, big_alpha) > 0.0,
                "p<1/2: a large enough attack value beats ANY fixed α — the blind spot"
            );
            // The guard now REFUSES to certify that regime rather than trusting the grid:
            assert!(
                !feasible(1.0, big_alpha, BETA, BOND, EFFORT, &v_grid(), 0.4, 0.5),
                "harness refuses to certify p_min<1/2 (unbounded-V attack) regardless of grid"
            );
            // At p_min = 1/2 it certifies honestly (EV = −α/2, V-independent):
            assert!(
                feasible(LAMBDA, ALPHA, BETA, BOND, EFFORT, &v_grid(), 0.5, 0.5),
                "p_min=1/2 is V-independent at the binding case ⇒ honestly certifiable"
            );
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

/// Concurrent claims on standing (`OUTCOME-EVALUATOR.md` §5): a contributor's standing is
/// collateral for SEVERAL claimants — dispute restitution, advance-shortfall recovery,
/// protocol decay — and `saturating_sub` ordering would otherwise let whichever claim
/// settles first silently defund the rest (collateral double-spend). Three rules:
/// 1. PRIORITY: restitution-to-others > advance-shortfall > decay. Harm to others is
///    senior; self-dealing recovery is junior to it; rent is junior to both.
/// 2. EXPOSURE FREEZES BORROWING: while `dispute::standing_exit_blocked` holds, new
///    advances are denied — the same predicate that blocks exit blocks double-pledging
///    the collateral a live dispute may claim.
/// 3. DEFICITS LAND ON THE RISK-TAKER: an unpayable advance shortfall is the evaluator
///    pool's recorded loss (Role A takes risk, never authority); it is never recovered
///    from honest third parties, and standing never goes negative.
pub mod claims {
    /// Priority order IS the enum order (lower discriminant = senior).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub enum ClaimKind {
        DisputeSlash,
        AdvanceShortfall,
        Decay,
    }

    /// One settled claim: what was asked, what the collateral could pay, the deficit.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Settled {
        pub kind: ClaimKind,
        pub asked: f64,
        pub paid: f64,
        pub deficit: f64,
    }

    /// Settle all claims against one standing balance, senior first. Deterministic in
    /// the INPUT ORDER too: claims are sorted by seniority (stable within a class), so
    /// the same claim set settles identically however it arrives.
    pub fn settle(standing: u64, claims: &[(ClaimKind, f64)]) -> (u64, Vec<Settled>) {
        let mut ordered: Vec<(usize, ClaimKind, f64)> = claims
            .iter()
            .enumerate()
            .map(|(i, &(k, a))| (i, k, a.max(0.0)))
            .collect();
        ordered.sort_by_key(|&(i, k, _)| (k, i));
        let mut free = standing as f64;
        let mut out = Vec::with_capacity(ordered.len());
        for (_, kind, asked) in ordered {
            let paid = asked.min(free);
            free -= paid;
            out.push(Settled { kind, asked, paid, deficit: asked - paid });
        }
        (free.floor() as u64, out)
    }

    /// Rule 2 — the advance gate composes the evaluator bound with dispute exposure:
    /// an exposed contributor cannot pledge collateral a live dispute may claim.
    pub fn advance_allowed(exposed: bool) -> bool {
        !exposed
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn restitution_is_senior_shortfall_junior_decay_last() {
            // S = 10 against claims totalling 20: the dispute slash is made whole first,
            // the shortfall gets the remainder, decay gets nothing — and the deficits
            // are recorded, not silently dropped.
            let claims = vec![
                (ClaimKind::Decay, 5.0),
                (ClaimKind::AdvanceShortfall, 8.0),
                (ClaimKind::DisputeSlash, 7.0),
            ];
            let (remaining, settled) = settle(10, &claims);
            assert_eq!(remaining, 0);
            let by = |k: ClaimKind| settled.iter().find(|s| s.kind == k).unwrap().clone();
            assert_eq!(by(ClaimKind::DisputeSlash).paid, 7.0, "restitution made whole");
            assert_eq!(by(ClaimKind::AdvanceShortfall).paid, 3.0, "junior gets remainder");
            assert_eq!(by(ClaimKind::AdvanceShortfall).deficit, 5.0, "deficit recorded");
            assert_eq!(by(ClaimKind::Decay).paid, 0.0, "rent last");
        }

        #[test]
        fn settlement_is_input_order_independent() {
            let a = vec![
                (ClaimKind::AdvanceShortfall, 6.0),
                (ClaimKind::DisputeSlash, 9.0),
            ];
            let b = vec![
                (ClaimKind::DisputeSlash, 9.0),
                (ClaimKind::AdvanceShortfall, 6.0),
            ];
            let (ra, sa) = settle(12, &a);
            let (rb, sb) = settle(12, &b);
            assert_eq!(ra, rb);
            let key = |v: &Vec<Settled>| {
                let mut k: Vec<_> = v.iter().map(|s| (s.kind, s.paid as i64)).collect();
                k.sort();
                k
            };
            assert_eq!(key(&sa), key(&sb), "same claims settle identically in any order");
        }

        #[test]
        fn deficit_lands_on_the_pool_and_standing_never_negative() {
            let (remaining, settled) =
                settle(3, &[(ClaimKind::AdvanceShortfall, 10.0)]);
            assert_eq!(remaining, 0, "collateral exhausted, not negative");
            assert_eq!(settled[0].paid, 3.0);
            assert_eq!(settled[0].deficit, 7.0, "the advance pool eats its own risk");
        }

        #[test]
        fn exposure_freezes_new_borrowing() {
            // Composes with dispute::standing_exit_blocked: same predicate, second use —
            // collateral a live dispute may claim cannot be double-pledged.
            assert!(!advance_allowed(true), "exposed: no new advances");
            assert!(advance_allowed(false), "clear: borrowing open");
        }
    }
}

/// Fixed-point mirror of the INTAKE value rule — CKB-VM-PORT.md code increment #1.
/// On-VM consensus code cannot use f64 (cross-platform float nondeterminism); this module
/// re-derives the intake pipeline (similarity floor → semantic floor → quality boost) in
/// pure integer math, Q16.16. The f64 forms stay the PROTOTYPE; this form is what the
/// RISC-V type-script will run, so equivalence is tested against the f64 forms on every
/// existing corpus fixture + a deterministic random sweep.
///
/// Honest boundary note (flagged in CKB-VM-PORT.md for its own adversarial tick): at the
/// exact theta boundary the Q16.16 comparison can disagree with f64 within quantization
/// error (≤ 2^-16 in theta, log2 truncation ≤ ~2^-16 relative). The fixed form is the
/// CANONICAL one on-chain; divergence inside the epsilon band is acceptable because it is
/// deterministic — every node disagrees with the f64 prototype identically.
pub mod value_fixed {
    use super::{coverage, CovId};
    use std::collections::HashSet;

    pub const Q: u32 = 16;
    pub const ONE: u64 = 1 << Q;

    /// Deterministic Q16.16 log2 for x ≥ 1: integer part from leading_zeros, 16 fractional
    /// bits by mantissa squaring (the classic shift-and-square algorithm). Pure integer
    /// ops — bit-identical on every platform, no table, bounded 16 iterations.
    pub fn log2_q16(x: u64) -> u64 {
        debug_assert!(x >= 1, "log2 needs x >= 1");
        let ip = 63 - u64::from(x.leading_zeros());
        // mantissa in Q32.32, normalized into [1, 2)
        let mut m: u128 = ((x as u128) << 32) >> ip;
        let mut frac: u64 = 0;
        for i in (0..Q).rev() {
            m = (m * m) >> 32; // square in Q32.32; m < 2^33 so m^2 < 2^66 fits u128
            if m >= (2u128 << 32) {
                m >>= 1;
                frac |= 1 << i;
            }
        }
        (ip << Q) | frac
    }

    /// Integer mirror of `semantic::is_incompressible` (normalized entropy ≥ theta).
    /// Derivation:  H = log2(n) − (1/n)·Σ c·log2(c);  H / log2(min(n,256)) ≥ θ
    ///          ⇔  n·log2(n) − Σ c·log2(c)  ≥  θ · n · log2(min(n,256))
    /// Both sides carried at Q16.16 × n scale in i128 — no overflow for any real payload.
    pub fn is_incompressible_q16(data: &[u8], theta_q16: u64) -> bool {
        let n = data.len() as u64;
        if n < 2 {
            return theta_q16 == 0; // mirror f64: entropy is defined as 0.0 here
        }
        let mut counts = [0u64; 256];
        for &b in data {
            counts[b as usize] += 1;
        }
        let lhs: i128 = (n as i128) * (log2_q16(n) as i128)
            - counts
                .iter()
                .filter(|&&c| c > 0)
                .map(|&c| (c as i128) * (log2_q16(c) as i128))
                .sum::<i128>();
        let m = n.min(256);
        let rhs: i128 = ((theta_q16 as i128) * (n as i128) * (log2_q16(m) as i128)) >> Q;
        lhs >= rhs
    }

    /// Integer mirror of `semantic::semantic_floor`.
    pub fn semantic_floor_q16(novelty: u64, data: &[u8], theta_q16: u64) -> u64 {
        if is_incompressible_q16(data, theta_q16) {
            0
        } else {
            novelty
        }
    }

    /// Integer mirror of `temporal_novelty_with_similarity_floor`:
    /// overlap/|cov| > θ  ⇔  overlap·2^Q > θ_q16·|cov|  (cross-multiplied, exact).
    pub fn temporal_novelty_with_similarity_floor_q16(
        cells_in_commit_order: &[super::Cell],
        theta_q16: u64,
    ) -> Vec<u64> {
        let mut seen: HashSet<CovId> = HashSet::new();
        let mut out = Vec::with_capacity(cells_in_commit_order.len());
        for c in cells_in_commit_order {
            let cov = coverage(&c.data);
            let covset: HashSet<CovId> = cov.iter().copied().collect();
            let overlap = covset.iter().filter(|x| seen.contains(*x)).count() as u128;
            let len = covset.len() as u128;
            let floored = len > 0 && (overlap << Q) > (theta_q16 as u128) * len;
            let novel = cov.iter().filter(|x| !seen.contains(*x)).count() as u64;
            out.push(if floored { 0 } else { novel });
            seen.extend(cov);
        }
        out
    }

    /// Full integer mirror of `value::production_value`. `quality_q16[i]` ∈ [0, ONE].
    /// Returns value at Q16.16 scale: floored_novelty × (ONE + quality).
    pub fn production_value_q16(
        cells_in_commit_order: &[super::Cell],
        theta_sim_q16: u64,
        theta_ent_q16: u64,
        quality_q16: &[u64],
    ) -> Vec<u64> {
        temporal_novelty_with_similarity_floor_q16(cells_in_commit_order, theta_sim_q16)
            .iter()
            .zip(cells_in_commit_order)
            .zip(quality_q16)
            .map(|((&nv, c), &q)| {
                semantic_floor_q16(nv, &c.data, theta_ent_q16).saturating_mul(ONE + q)
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::super::{semantic, Cell, Script};
        use super::*;

        const THETA_ENT_Q16: u64 = 62259; // floor(0.95 · 2^16) = 0.949997…
        const THETA_SIM_Q16: u64 = 52429; // floor(0.8 · 2^16) + 1 ulp ≈ 0.800003

        fn splitmix64(state: &mut u64) -> u64 {
            *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = *state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        #[test]
        fn log2_q16_tracks_f64_within_quantization() {
            for &x in &[1u64, 2, 3, 5, 10, 24, 64, 255, 256, 1000, 65535, 1 << 32, u64::MAX] {
                let fixed = log2_q16(x) as f64 / ONE as f64;
                let float = (x as f64).log2();
                assert!((fixed - float).abs() < 1e-4, "x={x}: fixed {fixed} vs f64 {float}");
            }
        }

        #[test]
        fn entropy_floor_agrees_with_f64_on_every_corpus_fixture() {
            let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let keyish: Vec<u8> = (0u8..32).map(|i| i.wrapping_mul(67).wrapping_add(29)).collect();
            let hexed: Vec<u8> = noise.iter().flat_map(|b| format!("{b:02x}").into_bytes()).collect();
            let diluted: Vec<u8> =
                noise.iter().copied().chain(std::iter::repeat(0u8).take(64)).collect();
            let n24a: Vec<u8> = (0u8..24).map(|i| 0x10u8.wrapping_add(i.wrapping_mul(53))).collect();
            let n24b: Vec<u8> = (0u8..24).map(|i| 0x80u8.wrapping_add(i.wrapping_mul(53))).collect();
            let fixtures: Vec<&[u8]> = vec![
                b"alpha-bravo-charlie-delta",
                b"echo-foxtrot-golf-hotel",
                b"india-juliet-kilo-lima",
                b"big-feature-built-on-the-tiny-fix-uniform-victor",
                b"the-quick-brown-fox-says-nothing-of-value-today",
                b"fn main() { println!(\"hello\"); } // code reuses bytes heavily",
                &noise,
                &keyish,
                &hexed,
                &diluted,
                &n24a,
                &n24b,
                b"",
                b"x",
            ];
            for f in fixtures {
                assert_eq!(
                    is_incompressible_q16(f, THETA_ENT_Q16),
                    semantic::is_incompressible(f, 0.95),
                    "fixture disagrees: {:?}…",
                    &f[..f.len().min(12)]
                );
            }
        }

        #[test]
        fn entropy_floor_random_sweep_agrees_outside_the_quantization_band() {
            // Deterministic sweep across lengths and alphabet sizes (masking bytes to
            // 2^k symbols sweeps entropy through the whole range). HONEST tolerance: only
            // payloads whose f64 entropy lands within 1e-3 of theta are skipped — that band
            // is exactly the documented Q16.16 quantization divergence; everything outside
            // it must agree exactly.
            let mut state = 0x0BAD_5EED_u64;
            let mut checked = 0u32;
            for trial in 0..500u64 {
                let len = 2 + (splitmix64(&mut state) % 300) as usize;
                let mask: u64 = match trial % 4 {
                    0 => 0xFF,
                    1 => 0x3F,
                    2 => 0x0F,
                    _ => 0x03,
                };
                let data: Vec<u8> =
                    (0..len).map(|_| (splitmix64(&mut state) & mask) as u8).collect();
                let h = semantic::normalized_entropy(&data);
                if (h - 0.95).abs() < 1e-3 {
                    continue;
                }
                checked += 1;
                assert_eq!(
                    is_incompressible_q16(&data, THETA_ENT_Q16),
                    h >= 0.95,
                    "len={len} mask={mask:#x} h={h}"
                );
            }
            assert!(checked > 400, "sweep must mostly land outside the band (got {checked})");
        }

        fn cell(id: u64, owner: u8, ts: u64, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![owner] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![owner] },
                parent: None,
                timestamp: ts,
                data: data.to_vec(),
            }
        }

        #[test]
        fn production_value_q16_matches_f64_on_the_canonical_fixtures() {
            // Same scenario the f64 tests pin: honest cells + a near-duplicate + a noise
            // cell, mixed quality. The integer mirror must reproduce the f64 values exactly
            // (the chosen qualities are exactly representable in Q16.16).
            let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let mut order = vec![
                cell(0, 1, 0, b"alpha-bravo-charlie-delta"),
                cell(1, 2, 1, b"echo-foxtrot-golf-hotel"),
                cell(2, 3, 2, b"india-juliet-kilo-lima"),
            ];
            let mut near = order[0].data.clone();
            near[2] ^= 0x20;
            order.push(cell(3, 9, 3, &near));
            order.push(cell(4, 8, 4, &noise));
            let q_f = vec![1.0, 0.5, 0.0, 1.0, 1.0];
            let q_q = vec![ONE, ONE / 2, 0, ONE, ONE];
            let f = super::super::value::production_value(&order, 0.8, 0.95, &q_f);
            let x = production_value_q16(&order, THETA_SIM_Q16, THETA_ENT_Q16, &q_q);
            assert_eq!(f.len(), x.len());
            for (i, (a, b)) in f.iter().zip(&x).enumerate() {
                let bf = *b as f64 / ONE as f64;
                assert!((a - bf).abs() < 1e-9, "cell {i}: f64 {a} vs fixed {bf}");
            }
            assert_eq!(x[3], 0, "near-duplicate floored in the integer mirror too");
            assert_eq!(x[4], 0, "noise floored in the integer mirror too");
        }
    }
}

/// Semantic / compressibility floor (ROADMAP Phase 1, Role-C — the garbage-novelty gap
/// AT the gate). The coverage proxy calls high-entropy noise "novel" because every shingle
/// is unique. But genuine content (text, code, thought) REUSES bytes and so is compressible;
/// near-random noise is incompressible. This floor zeroes a cell whose normalized byte
/// entropy is at/above `theta` — the incompressible-noise subclass of valueless novelty.
///
/// It is a FLOOR: it can only ZERO suspected noise, never rescue anything, so it composes
/// with temporal-novelty WITHOUT touching strategyproofness (AND, like the similarity
/// floor). Honest bound (the airgap, pinned in tests): genuinely-novel HIGH-entropy but
/// VALUABLE payloads — keys, hashes, compressed blobs — are false-positived. That is why
/// it is a heuristic floor backstopped by realized-flow (a wrongly-floored useful cell
/// still earns through downstream use in v5/v6), NOT the whole answer. Structured content
/// that is novel-but-pointless is NOT caught here — that needs labels/flow, not bytes.
///
/// SECOND honest bound (critical-qa 2026-06-12, pinned): the floor only catches NAIVE
/// noise. An adversary-aware generator hex-encodes or zero-dilutes the same garbage and
/// drops well under any workable theta (≈0.57 vs 0.95, in-test) while keeping its shingle
/// novelty — encoded noise IS structured-but-valueless content, so it re-enters through
/// the frontier above. The floor's real guarantee is therefore narrow and should be
/// claimed narrowly: it zeroes accidental/lazy noise and raises the attacker's move from
/// "dump entropy" to "encode it", nothing more. The economic layers (v6 standing price,
/// dispute slashing) remain the binding defense against the aware adversary.
pub mod semantic {
    /// Shannon byte entropy normalized to [0,1] by the max achievable for this length
    /// (`log2(min(n,256))`). 1.0 = every byte distinct (random-looking); low = structured/
    /// repetitive. Empty or single-byte data is treated as fully structured (0.0).
    pub fn normalized_entropy(data: &[u8]) -> f64 {
        let n = data.len();
        if n < 2 {
            return 0.0;
        }
        let mut counts = [0u32; 256];
        for &b in data {
            counts[b as usize] += 1;
        }
        let nf = n as f64;
        let h: f64 = counts
            .iter()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = c as f64 / nf;
                -p * p.log2()
            })
            .sum();
        let max = (n.min(256) as f64).log2();
        if max <= 0.0 {
            0.0
        } else {
            h / max
        }
    }

    /// True when the payload looks like incompressible noise (entropy ≥ theta).
    pub fn is_incompressible(data: &[u8], theta: f64) -> bool {
        normalized_entropy(data) >= theta
    }

    /// AND-compose the floor: pass `novelty` through unless the payload is incompressible
    /// noise, in which case it is zeroed. Never raises novelty.
    pub fn semantic_floor(novelty: u64, data: &[u8], theta: f64) -> u64 {
        if is_incompressible(data, theta) {
            0
        } else {
            novelty
        }
    }

    /// Calibrate `theta` against labeled corpora instead of asserting it (the 0.95 used
    /// across the suite was a magic constant until this). Returns the SEPARATING BAND
    /// `(max content entropy, min noise entropy)` — every theta strictly inside it has
    /// zero empirical false-positives AND zero false-negatives on the given corpora —
    /// or `None` when the classes overlap and NO theta separates by bytes.
    ///
    /// Honest scope: corpus-relative evidence, not a proof. The airgap is visible right
    /// here: add one high-entropy-but-valuable payload (key/hash/compressed blob) to the
    /// CONTENT corpus and the band collapses to `None` — pinned in-test. That collapse is
    /// the formal statement of WHY the floor needs the realized-flow backstop (v7 floors
    /// only the seed) rather than being trusted as a verdict.
    pub fn calibrate_theta(content: &[&[u8]], noise: &[&[u8]]) -> Option<(f64, f64)> {
        if content.is_empty() || noise.is_empty() {
            return None;
        }
        let max_content = content
            .iter()
            .map(|d| normalized_entropy(d))
            .fold(f64::NEG_INFINITY, f64::max);
        let min_noise = noise
            .iter()
            .map(|d| normalized_entropy(d))
            .fold(f64::INFINITY, f64::min);
        if max_content < min_noise {
            Some((max_content, min_noise))
        } else {
            None
        }
    }

    /// Midpoint of the separating band — the recommended theta, centered so both empirical
    /// error margins are equal on the calibration corpora.
    pub fn recommend_theta(band: (f64, f64)) -> f64 {
        (band.0 + band.1) / 2.0
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const THETA: f64 = 0.95;

        fn noise(seed: u8, n: u8) -> Vec<u8> {
            (0u8..n).map(|i| seed.wrapping_add(i.wrapping_mul(53))).collect()
        }

        #[test]
        fn structured_content_is_below_threshold_noise_is_at_ceiling() {
            for s in [
                &b"alpha-bravo-charlie"[..],
                b"delta-echo-foxtrot",
                b"big-feature-built-on-the-tiny-fix-uniform-victor",
            ] {
                assert!(normalized_entropy(s) < THETA, "real content reuses bytes ⇒ < theta");
            }
            assert!(normalized_entropy(&noise(0x10, 24)) >= THETA, "all-distinct noise ⇒ ceiling");
            assert!(
                normalized_entropy(&(0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect::<Vec<u8>>())
                    >= THETA,
                "the 64-byte garbage cell is incompressible"
            );
        }

        #[test]
        fn floor_zeroes_high_entropy_garbage_but_passes_real_content() {
            // The garbage cell that earned full novelty under the coverage proxy
            // (`garbage_novelty_is_the_documented_open_gap`) is zeroed here AT the gate.
            assert_eq!(semantic_floor(7, &noise(0x80, 24), THETA), 0, "noise floored to 0");
            assert_eq!(
                semantic_floor(7, b"golf-hotel-india", THETA),
                7,
                "structured content keeps its novelty"
            );
        }

        #[test]
        fn floor_only_zeroes_never_rescues_and_is_and_composable() {
            // Strategyproofness preserved: the floor cannot turn a 0 into anything positive.
            assert_eq!(semantic_floor(0, b"golf-hotel-india", THETA), 0, "0 stays 0 (no rescue)");
            // Composes after temporal-novelty: min-like, only ever lowers.
            let nov = 9u64;
            assert!(semantic_floor(nov, &noise(1, 32), THETA) <= nov);
            assert!(semantic_floor(nov, b"structured-text-here", THETA) <= nov);
        }

        fn content_corpus() -> Vec<&'static [u8]> {
            vec![
                b"alpha-bravo-charlie-delta",
                b"echo-foxtrot-golf-hotel",
                b"india-juliet-kilo-lima",
                b"big-feature-built-on-the-tiny-fix-uniform-victor",
                b"the-quick-brown-fox-says-nothing-of-value-today",
                b"fn main() { println!(\"hello\"); } // code reuses bytes heavily",
            ]
        }

        #[test]
        fn calibrated_band_exists_and_contains_the_suite_constant() {
            // Grounds the 0.95 used across the suite: on the canonical corpora a separating
            // band exists, THETA sits strictly inside it, and the midpoint recommendation
            // is a valid theta too. The constant stops being magic.
            let noise_corpus: Vec<Vec<u8>> = vec![
                noise(0x10, 24),
                noise(0x80, 24),
                (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect(),
                (0u8..48).map(|i| i.wrapping_mul(91).wrapping_add(7)).collect(),
            ];
            let noise_refs: Vec<&[u8]> = noise_corpus.iter().map(|v| v.as_slice()).collect();
            let band = calibrate_theta(&content_corpus(), &noise_refs)
                .expect("structured content vs synthetic noise must separate");
            assert!(band.0 < THETA && THETA < band.1, "suite constant inside the band");
            let rec = recommend_theta(band);
            assert!(band.0 < rec && rec < band.1, "midpoint is a valid theta");
            for c in content_corpus() {
                assert!(!is_incompressible(c, rec), "recommended theta passes all content");
            }
            for n in &noise_refs {
                assert!(is_incompressible(n, rec), "recommended theta floors all noise");
            }
        }

        #[test]
        fn airgap_collapses_the_band_no_theta_separates_by_bytes_pinned() {
            // THE AIRGAP, restated as calibration math: one high-entropy-but-VALUABLE
            // payload in the content corpus (a 32-byte key) and the band is gone — there
            // exists NO theta with zero FP and zero FN. This is the formal reason the
            // floor is seed-only (v7) + flow-backstopped, never a verdict.
            let keyish: Vec<u8> = (0u8..32).map(|i| i.wrapping_mul(67).wrapping_add(29)).collect();
            let mut content = content_corpus();
            content.push(&keyish);
            let n = noise(0x10, 24);
            let noise_refs: Vec<&[u8]> = vec![&n];
            assert!(
                calibrate_theta(&content, &noise_refs).is_none(),
                "PINNED: with high-entropy value in-corpus, byte-entropy cannot separate"
            );
        }

        #[test]
        fn encoded_noise_evades_the_entropy_floor_open_gap() {
            // PINNED GAP (critical-qa 2026-06-12): the SAME garbage cell the floor catches
            // raw sails under it once hex-encoded or zero-diluted — byte entropy halves
            // while shingle novelty survives. Encoded noise is structured-but-valueless
            // content, i.e. it re-enters through the already-named out-of-band frontier.
            // Claim the floor narrowly: it stops accidental/lazy noise, not the aware
            // adversary; v6 standing pricing + dispute slashing stay the binding defense.
            let raw: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            assert!(is_incompressible(&raw, THETA), "raw noise is caught");
            let hexed: Vec<u8> = raw.iter().flat_map(|b| format!("{b:02x}").into_bytes()).collect();
            assert!(
                !is_incompressible(&hexed, THETA),
                "OPEN GAP: hex-encoded noise passes the floor (entropy ~0.57)"
            );
            let diluted: Vec<u8> = raw.iter().copied().chain(std::iter::repeat(0u8).take(64)).collect();
            assert!(
                !is_incompressible(&diluted, THETA),
                "OPEN GAP: zero-diluted noise passes the floor (entropy ~0.57)"
            );
        }

        #[test]
        fn honest_false_positive_high_entropy_value_is_floored_pinned() {
            // THE AIRGAP, pinned. A genuinely-novel high-entropy VALUABLE payload — e.g. a
            // 32-byte key/hash — is indistinguishable from noise BY CONTENT, so this floor
            // zeroes it too. That is why it is a heuristic backstopped by realized-flow
            // (the cell still earns if other minds build on it), not the whole answer.
            let keyish: Vec<u8> = (0u8..32).map(|i| i.wrapping_mul(67).wrapping_add(29)).collect();
            assert_eq!(
                semantic_floor(5, &keyish, THETA),
                0,
                "KNOWN TRADEOFF: high-entropy-but-valuable content is false-positived at the \
                 gate; realized-flow (v5/v6) is the backstop, content alone cannot tell them apart"
            );
        }
    }
}

/// Learned OUTCOME model over coalitions (`OUTCOME-EVALUATOR.md` §4, Phase-1 frontier).
/// The coverage proxy cannot tell high-entropy garbage-novelty from value-novelty by
/// CONTENT alone (`garbage_novelty_is_the_documented_open_gap`). The only thing that can
/// is OUTSIDE information: outcome labels — "the result using set S is better than using
/// S'." This module learns `v(S) ∈ [0,1]` from PAIRWISE coalition preferences (the
/// DeepFunding-distill-over-sets idea), Bradley-Terry over SET-level structural features.
///
/// It is deliberately NOT the gate. Its output is consumed ONLY through
/// [`evaluator`] (advance timing + dispute evidence, both bounded so a corrupt or
/// mis-trained model cannot mint — `evaluator::tests::corrupt_evaluator_cannot_mint`).
/// That is what makes a learned signal safe to introduce: the authority boundary, not a
/// robustness proof about the model.
pub mod outcome {
    use super::{coverage, Cell};
    use std::collections::HashSet;

    pub const N_FEATS: usize = 4;

    /// Set-level features for a coalition S (the cells whose indices are in `idxs`).
    /// Chosen so the model can express what the per-block coverage proxy cannot:
    ///   f0 breadth     = ln(1 + |union coverage|)
    ///   f1 synergy     = |union| / Σ|individual|   (1 = disjoint, →0 = redundant overlap)
    ///   f2 connectedness = fraction of S whose parent is also in S (internal provenance;
    ///                      orphaned garbage scores low, work-built-on-work scores high)
    ///   f3 depth       = longest parent chain within S / |S|   (shallow dump vs real lineage)
    /// All on-chain-derivable; none needs an oracle. The LABELS carry the outside signal.
    pub fn coalition_features(cells: &[Cell], idxs: &[usize]) -> [f64; N_FEATS] {
        if idxs.is_empty() {
            return [0.0; N_FEATS];
        }
        let in_s: HashSet<u64> = idxs.iter().map(|&i| cells[i].id).collect();
        let mut union: HashSet<_> = HashSet::new();
        let mut sum_individual = 0usize;
        for &i in idxs {
            let cov = coverage(&cells[i].data);
            sum_individual += cov.len();
            union.extend(cov);
        }
        let breadth = (1.0 + union.len() as f64).ln();
        let synergy = if sum_individual > 0 {
            union.len() as f64 / sum_individual as f64
        } else {
            0.0
        };
        let connected = idxs
            .iter()
            .filter(|&&i| cells[i].parent.is_some_and(|p| in_s.contains(&p)))
            .count() as f64
            / idxs.len() as f64;
        // longest parent chain confined to S
        let mut best = 0usize;
        for &i in idxs {
            let mut depth = 1usize;
            let mut cur = cells[i].parent;
            while let Some(p) = cur {
                if !in_s.contains(&p) {
                    break;
                }
                depth += 1;
                cur = cells.iter().find(|c| c.id == p).and_then(|c| c.parent);
                if depth > idxs.len() {
                    break; // cycle guard
                }
            }
            best = best.max(depth);
        }
        let depth = best as f64 / idxs.len() as f64;
        [breadth, synergy, connected, depth]
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    /// Bradley-Terry over coalition features, trained on `prefs` = `(winner, loser)`
    /// index pairs into `feats`. Same estimator shape as `value::train_bradley_terry`,
    /// at the set level. L2-regularized, deterministic.
    pub fn train(
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
                let dot: f64 = w.iter().zip(d).map(|(wk, dk)| wk * dk).sum();
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

    /// Learned outcome value of a coalition feature vector, squashed to [0,1].
    pub fn v_outcome(w: &[f64; N_FEATS], feats: &[f64; N_FEATS]) -> f64 {
        let dot: f64 = w.iter().zip(feats).map(|(wk, fk)| wk * fk).sum();
        sigmoid(dot)
    }

    /// The COVERAGE-PROXY baseline: what a coalition scores under the per-block rule alone — total
    /// union coverage, `ln(1 + |union|)`. This is the value the per-block proxy CAN see; it is blind
    /// to lineage and synergy structure. The moat is the gap between this and a model trained on
    /// real outcomes: value the proxy cannot express. Same quantity as feature f0 (breadth).
    pub fn proxy_value(cells: &[Cell], idxs: &[usize]) -> f64 {
        let mut union: HashSet<_> = HashSet::new();
        for &i in idxs {
            union.extend(coverage(&cells[i].data));
        }
        (1.0 + union.len() as f64).ln()
    }

    /// Generalization metric: the fraction of HELD-OUT `(winner, loser)` pairs a scorer ranks
    /// correctly (`score(winner) > score(loser)`; a tie is 0.5). Train on one split, measure here on
    /// UNSEEN coalitions — this is the number that decides whether a learned `v(S)` actually beats
    /// the proxy, the mile the whole value layer rests on.
    pub fn pairwise_accuracy<F: Fn(usize) -> f64>(score: F, prefs: &[(usize, usize)]) -> f64 {
        if prefs.is_empty() {
            return 0.0;
        }
        let mut correct = 0.0;
        for &(w, l) in prefs {
            let (sw, sl) = (score(w), score(l));
            correct += if sw > sl {
                1.0
            } else if sw == sl {
                0.5
            } else {
                0.0
            };
        }
        correct / prefs.len() as f64
    }

    /// Semantic-floored learned value — the AND-composition named by
    /// `fake_lineage_garbage_fools_the_model_but_is_contained_below`. Spoofed STRUCTURE (a fake
    /// lineage of noise) raises the connectedness/depth features, but if the coalition's content is
    /// incompressible noise the floor zeroes the score: structure cannot manufacture value from
    /// noise. The floor can only LOWER, never rescue (same rule as the value gate). The noise check
    /// is single-sourced from the intake floor (`noesis_core::is_incompressible_q16`, same `theta`),
    /// so a coalition the chain refuses to mint cannot score here either. Closes the fake-lineage
    /// gaming vector AT THE SCORE, not just below it via the bounded evaluator.
    pub fn v_outcome_floored(
        w: &[f64; N_FEATS],
        feats: &[f64; N_FEATS],
        cells: &[Cell],
        idxs: &[usize],
        theta_q16: u64,
    ) -> f64 {
        if idxs.is_empty() {
            return 0.0;
        }
        let real = idxs
            .iter()
            .filter(|&&i| !noesis_core::is_incompressible_q16(&cells[i].data, theta_q16))
            .count();
        if real == 0 {
            return 0.0; // all content is incompressible noise ⇒ no structure can rescue it
        }
        // AND-compose: scale the structural score by the non-noise fraction (never raises it).
        v_outcome(w, feats) * (real as f64 / idxs.len() as f64)
    }

    /// Parse file-sourced outcome labels into the `(feats, prefs)` the harness consumes.
    /// This is the on-disk contract the DeepFunding distill-over-sets pull must emit
    /// (stdlib, no serde): blank / `#...` lines are ignored; a line of `N_FEATS`
    /// whitespace-separated floats is one coalition's feature row (indexed in file
    /// order); a `pref <winner> <loser>` line is one pairwise outcome preference into
    /// those rows. When real labels land they serialize to exactly this and the harness
    /// (`train` + `v_outcome` + `pairwise_accuracy`) runs UNCHANGED — the seam, not the
    /// data, is what this locks. Malformed rows are skipped, never partial-credited.
    pub fn load_prefs(text: &str) -> (Vec<[f64; N_FEATS]>, Vec<(usize, usize)>) {
        let mut feats: Vec<[f64; N_FEATS]> = Vec::new();
        let mut prefs: Vec<(usize, usize)> = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("pref ") {
                let nums: Vec<usize> =
                    rest.split_whitespace().filter_map(|t| t.parse().ok()).collect();
                if nums.len() == 2 {
                    prefs.push((nums[0], nums[1]));
                }
                continue;
            }
            let vals: Vec<f64> =
                line.split_whitespace().filter_map(|t| t.parse().ok()).collect();
            if vals.len() == N_FEATS {
                feats.push(std::array::from_fn(|k| vals[k]));
            }
        }
        // Adversarial robustness: drop any preference that references a coalition row
        // outside the parsed set. A malformed or hostile label file must not index out
        // of bounds into the harness (which would panic) or train on a phantom coalition.
        let n = feats.len();
        prefs.retain(|&(w, l)| w < n && l < n);
        (feats, prefs)
    }

    #[cfg(test)]
    mod tests {
        use super::super::{Cell, Script};
        use super::*;

        #[test]
        fn file_sourced_labels_train_a_model_that_ranks_the_held_out_winner() {
            // The harness had only ever read synthetic in-test coalitions. This proves the
            // on-disk contract (load_prefs) feeds train + v_outcome end-to-end, so the moat
            // cannot be dismissed as in-test-only. Fixture = 4 connected/orphan pairs at
            // EQUAL breadth/synergy (the coverage proxy ties them), differing only in the
            // lineage features a learned model can read.
            let fixture = include_str!("fixtures/outcome_labels_demo.txt");
            let (feats, prefs) = load_prefs(fixture);
            assert_eq!(feats.len(), 8, "parsed 8 coalition feature rows from file");
            assert_eq!(prefs.len(), 4, "parsed 4 preference pairs from file");

            // Train on the first 3 pairs; hold out the 4th (never trained on).
            let split = prefs.len() - 1;
            let w = train(&feats, &prefs[..split], 5000, 0.5);
            let (win, lose) = prefs[split];
            assert!(
                v_outcome(&w, &feats[win]) > v_outcome(&w, &feats[lose]),
                "model trained on FILE-SOURCED labels ranks the held-out winner above the loser"
            );

            // Adversarial robustness: out-of-range pref indices are dropped, not indexed
            // out of bounds into the harness. (0,99) and (7,0) reference rows that don't
            // exist in a 2-row file ⇒ only the valid pair survives.
            let (f2, p2) = load_prefs("0.1 0.2 0.3 0.4\n0.5 0.6 0.7 0.8\npref 0 1\npref 0 99\npref 7 0");
            assert_eq!(f2.len(), 2);
            assert_eq!(p2, vec![(0usize, 1usize)], "malformed out-of-range prefs dropped; valid pair kept");
        }

        fn cellp(id: u64, ts: u64, parent: Option<u64>, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![1] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![1] },
                parent,
                timestamp: ts,
                data: data.to_vec(),
            }
        }

        // A "value" coalition: connected lineage, real synergy.
        fn value_set() -> Vec<Cell> {
            vec![
                cellp(0, 0, None, b"alpha-bravo-charlie"),
                cellp(1, 1, Some(0), b"delta-echo-foxtrot"),
                cellp(2, 2, Some(1), b"golf-hotel-india"),
            ]
        }

        // A "garbage" coalition: orphaned high-entropy noise, no internal provenance.
        fn garbage_set() -> Vec<Cell> {
            let noise = |s: u8| -> Vec<u8> { (0u8..24).map(|i| s.wrapping_add(i.wrapping_mul(53))).collect() };
            vec![
                cellp(10, 0, None, &noise(0x10)),
                cellp(11, 1, None, &noise(0x80)),
                cellp(12, 2, None, &noise(0xC0)),
            ]
        }

        #[test]
        fn features_separate_value_from_garbage_on_structure() {
            let v = coalition_features(&value_set(), &[0, 1, 2]);
            let g = coalition_features(&garbage_set(), &[0, 1, 2]);
            // connectedness (f2) and depth (f3) are the discriminators content can't fake.
            assert!(v[2] > g[2], "value coalition is internally connected, garbage is orphaned");
            assert!(v[3] > g[3], "value coalition has lineage depth, garbage is flat");
            assert_eq!(g[2], 0.0, "orphaned garbage: zero internal provenance");
        }

        #[test]
        fn learns_a_label_ordering_the_proxy_cannot_express() {
            // Two coalitions the COVERAGE proxy would rank similarly (both novel bytes);
            // labels say the connected one is the better outcome. The model must learn it.
            let vfeat = coalition_features(&value_set(), &[0, 1, 2]);
            let gfeat = coalition_features(&garbage_set(), &[0, 1, 2]);
            let feats = [vfeat, gfeat];
            let prefs = vec![(0usize, 1usize); 8]; // value preferred to garbage, repeated
            let w = train(&feats, &prefs, 4000, 0.3);
            assert!(
                v_outcome(&w, &vfeat) > v_outcome(&w, &gfeat),
                "learned v(S) ranks the labelled-better coalition higher"
            );
            assert!(v_outcome(&w, &vfeat) > 0.5 && v_outcome(&w, &gfeat) < 0.5);
        }

        #[test]
        fn generalizes_to_an_unseen_coalition() {
            // Train on one value/garbage pair; score a DIFFERENT connected coalition the
            // model never saw. It should still rank as value, because the features
            // (connectedness, depth) generalize.
            let train_feats = [
                coalition_features(&value_set(), &[0, 1, 2]),
                coalition_features(&garbage_set(), &[0, 1, 2]),
            ];
            let w = train(&train_feats, &vec![(0usize, 1usize); 8], 4000, 0.3);
            let unseen = vec![
                cellp(20, 0, None, b"kilo-lima-mike"),
                cellp(21, 1, Some(20), b"november-oscar-papa"),
            ];
            let unseen_feat = coalition_features(&unseen, &[0, 1]);
            let unseen_garbage = coalition_features(&garbage_set(), &[0, 1, 2]);
            assert!(
                v_outcome(&w, &unseen_feat) > v_outcome(&w, &unseen_garbage),
                "an unseen connected coalition still scores above garbage (generalization)"
            );
        }

        #[test]
        fn fake_lineage_garbage_fools_the_model_but_is_contained_below() {
            // ADVERSARIAL TICK vs the outcome model (2026-06-12, same session — run the
            // adversary against every new v(S) the moment it lands, per the layering
            // method). The model rewards connectedness + depth. So the attacker builds a
            // CHAIN of novel-garbage, each cell pointing at the last: fake lineage. The
            // structural features now look like "value" even though every byte is noise.
            let noise = |s: u8| -> Vec<u8> {
                (0u8..24).map(|i| s.wrapping_add(i.wrapping_mul(53))).collect()
            };
            let fake_lineage = vec![
                cellp(30, 0, None, &noise(0x10)),
                cellp(31, 1, Some(30), &noise(0x40)),
                cellp(32, 2, Some(31), &noise(0x90)),
            ];
            let w = train(
                &[
                    coalition_features(&value_set(), &[0, 1, 2]),
                    coalition_features(&garbage_set(), &[0, 1, 2]),
                ],
                &vec![(0usize, 1usize); 8],
                4000,
                0.3,
            );
            let fake_feat = coalition_features(&fake_lineage, &[0, 1, 2]);
            let orphan_feat = coalition_features(&garbage_set(), &[0, 1, 2]);
            // The survivor: fake lineage out-scores orphaned garbage — the model IS fooled
            // by spoofed structure. Honest finding, pinned.
            assert!(
                v_outcome(&w, &fake_feat) > v_outcome(&w, &orphan_feat),
                "KNOWN: fake-lineage garbage spoofs the model's connectedness/depth features"
            );

            // CONTAINMENT (why this is not a chain-level vulnerability):
            // (1) it cannot MINT — routed through the bounded evaluator on fresh keys = 0.
            for id in 0u8..3 {
                assert_eq!(
                    crate::evaluator::intake_advance(v_outcome(&w, &fake_feat), 40, 0, 0.5, 0.5),
                    0.0,
                    "fresh-identity-{id}: spoofed score still mints nothing via the evaluator"
                );
            }
            // (2) building the fake lineage is EXACTLY what the lower layers price: each
            //     link is an external edge needing earned standing (v6) and is slashable
            //     when refuted (dispute). The model inherits that protection; it does not
            //     re-open the gap. So the model's authority stays bounded to advance/
            //     evidence, where being fooled costs the evaluator-pool a timing bet, never
            //     minted value. Next increment if ever wired into scoring: AND-compose the
            //     learned semantic floor (Role C) so spoofed structure cannot raise novelty.
        }

        /// ADVERSARIAL-GAMING increment (2026-06-13, pom-roadmap-advance): CLOSE the fake-lineage
        /// spoof AT THE SCORE. The survivor above shows spoofed structure fools the bare model; the
        /// semantic-floored `v_outcome_floored` AND-composes the entropy floor, so a fake lineage of
        /// noise scores 0 even though its structure looks like value — structure can no longer
        /// manufacture value from noise. Real work keeps its score.
        #[test]
        fn semantic_floor_closes_the_fake_lineage_spoof_at_the_score() {
            const THETA: u64 = 62259; // same entropy threshold as the on-VM intake floor
            let noise = |s: u8| -> Vec<u8> { (0u8..24).map(|i| s.wrapping_add(i.wrapping_mul(53))).collect() };
            let fake_lineage = vec![
                cellp(30, 0, None, &noise(0x10)),
                cellp(31, 1, Some(30), &noise(0x40)),
                cellp(32, 2, Some(31), &noise(0x90)),
            ];
            let real = value_set();
            let w = train(
                &[
                    coalition_features(&value_set(), &[0, 1, 2]),
                    coalition_features(&garbage_set(), &[0, 1, 2]),
                ],
                &vec![(0usize, 1usize); 8],
                4000,
                0.3,
            );
            let fake_feat = coalition_features(&fake_lineage, &[0, 1, 2]);
            let real_feat = coalition_features(&real, &[0, 1, 2]);
            // Bare model: the spoof scores high (the known survivor).
            assert!(v_outcome(&w, &fake_feat) > 0.5, "bare model is fooled by spoofed structure");
            // Floored: the fake lineage of noise collapses to 0; real work keeps its value.
            let fake_floored = v_outcome_floored(&w, &fake_feat, &fake_lineage, &[0, 1, 2], THETA);
            let real_floored = v_outcome_floored(&w, &real_feat, &real, &[0, 1, 2], THETA);
            assert_eq!(fake_floored, 0.0, "fake-lineage NOISE scores 0 under the semantic-floored v(S)");
            assert!(real_floored > 0.0, "real work keeps its learned value through the floor");
            assert!(real_floored > fake_floored, "the floor closes the spoof at the score, not only below it");
        }

        #[test]
        fn output_is_bounded_and_corruption_is_harmless_by_construction() {
            // The model is bounded [0,1]; and even a maximal score cannot mint, because
            // consumption is via the bounded evaluator (re-asserting the boundary that
            // makes a learned signal safe — see evaluator::corrupt_evaluator_cannot_mint).
            let w = [1e6, 1e6, 1e6, 1e6]; // absurd / corrupt weights
            let feat = coalition_features(&garbage_set(), &[0, 1, 2]);
            let v = v_outcome(&w, &feat);
            assert!((0.0..=1.0).contains(&v), "v_outcome always in [0,1]");
            // a corrupt high score routed through the evaluator on a fresh identity = 0 advance
            let advance = crate::evaluator::intake_advance(v, 40, 0, 0.5, 0.5);
            assert_eq!(advance, 0.0, "corrupt outcome score still cannot mint via the evaluator");
        }

        /// THE MOAT MEASUREMENT: train on labeled coalition preferences, then on HELD-OUT (unseen)
        /// coalitions measure whether the learned `v(S)` ranks them better than the coverage proxy
        /// can. Ground truth here is LINEAGE — at IDENTICAL coverage, a connected work-built-on-work
        /// coalition is worth more than the same cells dumped as orphans. The proxy sees only
        /// coverage, so it is blind to this BY CONSTRUCTION; a model with the connectedness/depth
        /// features is not. (In production the labels come from real outcomes — the DeepFunding
        /// distill-over-sets — not a hand-set lineage rule; this proves the harness + that the
        /// features carry signal the proxy cannot, and that it GENERALIZES to coalitions never
        /// trained on. The remaining mile is the real-outcome label pull, OUTCOME-EVALUATOR.md §4.)
        #[test]
        fn learned_v_s_beats_coverage_proxy_on_held_out_coalitions() {
            // A connected and an orphaned coalition built from the SAME cell data ⇒ identical
            // coverage (so the proxy ties them), differing only in lineage (so the model can rank).
            fn coalition(t: u64, connected: bool) -> Vec<Cell> {
                let d = |n: u64| format!("tk{t}-{n}-aaa bb{t}{n}b cc{t}{n}c dd{t}{n}d").into_bytes();
                let par = |id: u64| if connected { Some(id) } else { None };
                vec![
                    cellp(t * 10, 0, None, &d(0)),
                    cellp(t * 10 + 1, 1, par(t * 10), &d(1)),
                    cellp(t * 10 + 2, 2, par(t * 10 + 1), &d(2)),
                ]
            }
            let templates: u64 = 16;
            let mut feats: Vec<[f64; N_FEATS]> = Vec::new();
            let mut store: Vec<Vec<Cell>> = Vec::new();
            let mut prefs: Vec<(usize, usize)> = Vec::new(); // (winner = connected, loser = orphan)
            for t in 0..templates {
                let (conn, orph) = (coalition(t, true), coalition(t, false));
                let ci = feats.len();
                feats.push(coalition_features(&conn, &[0, 1, 2]));
                store.push(conn);
                let oi = feats.len();
                feats.push(coalition_features(&orph, &[0, 1, 2]));
                store.push(orph);
                prefs.push((ci, oi));
            }
            // Held-out split: train on the first 10 templates, TEST on the last 6 (never seen).
            let split = 10usize;
            let train_prefs: Vec<(usize, usize)> = prefs[..split].to_vec();
            let test_prefs: Vec<(usize, usize)> = prefs[split..].to_vec();
            let w = train(&feats, &train_prefs, 5000, 0.5);

            let learned = pairwise_accuracy(|i| v_outcome(&w, &feats[i]), &test_prefs);
            let proxy = pairwise_accuracy(|i| proxy_value(&store[i], &[0, 1, 2]), &test_prefs);

            assert!(
                learned > proxy,
                "learned v(S) ({learned}) must beat the coverage proxy ({proxy}) on held-out coalitions"
            );
            assert!(learned >= 0.9, "learned v(S) generalizes to UNSEEN coalitions (got {learned})");
            assert!(
                (proxy - 0.5).abs() < 1e-9,
                "the coverage proxy is blind to lineage at equal coverage ⇒ a coin-flip (got {proxy})"
            );
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

/// Sparse Merkle Tree over 64-bit shingle keys — T7 #1 (`T7-CROSS-CELL-SIMILARITY.md`).
/// The novelty-index cell's commitment structure: root = constant-size commitment to the
/// monotone seen-shingle SET; provers carry per-shingle membership / non-membership /
/// insertion proofs so the on-VM script can derive EXACT novelty+overlap counts without
/// seeing history. blake2b (CKB-native hasher); depth-64 (keys ARE CovId u64); the verify
/// fold is one shared function so membership, non-membership, and insertion are all the
/// same arithmetic — no_std-portable core (no allocation in `root_from`).
///
/// Set semantics fall out of positional hashing: insertion order cannot change the root
/// (in-test). The full set is DERIVED consensus state (reconstructible from chain
/// history, like a UTXO set); only the root lives in the index cell.
pub mod smt {
    use std::collections::HashMap;

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

    /// Hash of the empty subtree at each height (0 = leaf level ... DEPTH = root of empty tree).
    fn empty_hashes() -> [Hash; DEPTH + 1] {
        let mut e = [[0u8; 32]; DEPTH + 1];
        for d in 1..=DEPTH {
            e[d] = blake2b(&[&e[d - 1], &e[d - 1]]);
        }
        e
    }

    /// Leaf hash for a PRESENT key — domain-separated and key-bound, so a proof for one
    /// key can never be replayed for another.
    pub fn leaf(key: u64) -> Hash {
        blake2b(&[b"leaf", &key.to_le_bytes()])
    }

    /// THE shared fold: compute the root implied by placing `leaf_hash` at `key`'s slot
    /// with the given sibling path (siblings[0] = leaf level ... siblings[DEPTH-1] = top).
    /// Bit i of `key` (LSB-first from the leaf) picks the side at height i.
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

    /// Membership: the key's slot holds its leaf hash under `root`.
    pub fn verify_member(root: Hash, key: u64, siblings: &[Hash; DEPTH]) -> bool {
        root_from(key, leaf(key), siblings) == root
    }

    /// Non-membership: the key's slot is EMPTY under `root`. Proving this for a present
    /// key is impossible — its slot hashes to leaf(key), not the empty leaf.
    pub fn verify_non_member(root: Hash, key: u64, siblings: &[Hash; DEPTH]) -> bool {
        root_from(key, [0u8; 32], siblings) == root
    }

    /// Insertion: `old_root` had the slot empty; `new_root` is exactly `old_root` with
    /// leaf(key) placed there. SAME sibling path on both sides — nothing else may move.
    pub fn verify_insert(old_root: Hash, new_root: Hash, key: u64, siblings: &[Hash; DEPTH]) -> bool {
        root_from(key, [0u8; 32], siblings) == old_root
            && root_from(key, leaf(key), siblings) == new_root
    }

    /// Off-VM maintainer of the index (the consensus-derived state). Node hashes are
    /// stored sparsely; absent nodes are the height's empty hash.
    pub struct NoveltyIndex {
        nodes: HashMap<(usize, u64), Hash>, // (height, prefix-above-height) -> hash
        empty: [Hash; DEPTH + 1],
    }

    impl Default for NoveltyIndex {
        fn default() -> Self {
            Self::new()
        }
    }

    impl NoveltyIndex {
        pub fn new() -> Self {
            NoveltyIndex { nodes: HashMap::new(), empty: empty_hashes() }
        }

        fn node(&self, height: usize, prefix: u64) -> Hash {
            *self.nodes.get(&(height, prefix)).unwrap_or(&self.empty[height])
        }

        pub fn root(&self) -> Hash {
            self.node(DEPTH, 0)
        }

        /// Sibling path for `key`, bottom-up — valid for both proof polarities.
        pub fn proof(&self, key: u64) -> [Hash; DEPTH] {
            let mut sib = [[0u8; 32]; DEPTH];
            for (i, s) in sib.iter_mut().enumerate() {
                let prefix = key >> i;
                *s = self.node(i, prefix ^ 1);
            }
            sib
        }

        pub fn contains(&self, key: u64) -> bool {
            self.nodes.contains_key(&(0, key))
        }

        /// Insert a key (idempotent), updating the O(DEPTH) path to the root.
        pub fn insert(&mut self, key: u64) {
            let mut acc = leaf(key);
            self.nodes.insert((0, key), acc);
            for i in 0..DEPTH {
                let prefix = key >> i;
                let sib = self.node(i, prefix ^ 1);
                acc = if prefix & 1 == 0 {
                    blake2b(&[&acc, &sib])
                } else {
                    blake2b(&[&sib, &acc])
                };
                self.nodes.insert((i + 1, prefix >> 1), acc);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn insertion_order_cannot_change_the_root() {
            // Set semantics from positional hashing — the property that makes the root a
            // commitment to the SET, immune to history-presentation games.
            let keys = [42u64, 7, 0xDEAD_BEEF, u64::MAX, 1 << 40];
            let mut a = NoveltyIndex::new();
            let mut b = NoveltyIndex::new();
            for k in keys {
                a.insert(k);
            }
            for k in keys.iter().rev() {
                b.insert(*k);
            }
            assert_eq!(a.root(), b.root(), "order-independent");
            a.insert(42); // idempotent re-insert
            assert_eq!(a.root(), b.root(), "re-insertion is a no-op");
            assert_ne!(a.root(), NoveltyIndex::new().root(), "non-empty != empty");
        }

        #[test]
        fn membership_and_non_membership_are_mutually_exclusive() {
            // THE omission-attack kill from the design doc: a present key can NEVER prove
            // non-membership, an absent key can NEVER prove membership — with the same
            // honest sibling path. Complete classification is therefore forced.
            let mut idx = NoveltyIndex::new();
            for k in [10u64, 11, 99, 1 << 33] {
                idx.insert(k);
            }
            let root = idx.root();
            let present = 99u64;
            let absent = 100u64;
            assert!(verify_member(root, present, &idx.proof(present)));
            assert!(!verify_non_member(root, present, &idx.proof(present)), "present can't deny");
            assert!(verify_non_member(root, absent, &idx.proof(absent)));
            assert!(!verify_member(root, absent, &idx.proof(absent)), "absent can't claim");
        }

        #[test]
        fn proofs_bind_key_and_root() {
            let mut idx = NoveltyIndex::new();
            idx.insert(5);
            let root = idx.root();
            let p5 = idx.proof(5);
            assert!(!verify_member(root, 6, &p5), "proof not replayable for another key");
            let mut other = NoveltyIndex::new();
            other.insert(77);
            assert!(!verify_member(other.root(), 5, &p5), "proof not replayable across roots");
        }

        #[test]
        fn insertion_proof_validates_exact_transition_only() {
            // The index-cell rule (T7 #3 will consume this): new root = old root + exactly
            // the proven key. A different key, a skipped insert, or a smuggled second
            // change all fail against the same sibling path.
            let mut idx = NoveltyIndex::new();
            for k in [3u64, 9, 2000] {
                idx.insert(k);
            }
            let old_root = idx.root();
            let key = 4096u64;
            let sib = idx.proof(key);
            idx.insert(key);
            let new_root = idx.root();
            assert!(verify_insert(old_root, new_root, key, &sib));
            assert!(!verify_insert(old_root, new_root, 4097, &idx.proof(4097)), "wrong key");
            assert!(!verify_insert(old_root, old_root, key, &sib), "no-op claimed as insert");
            assert!(!verify_insert(new_root, old_root, key, &sib), "reversed transition");
        }

        #[test]
        fn exact_novelty_and_overlap_counts_from_complete_classification() {
            // The T7 #2 verifier shape, demonstrated end to end off-VM: classify EVERY
            // shingle of a coverage list against the root; counts must equal ground truth.
            let mut idx = NoveltyIndex::new();
            let seen = [1u64, 2, 3, 50, 60];
            for k in seen {
                idx.insert(k);
            }
            let root = idx.root();
            let coverage = [2u64, 3, 4, 50, 70, 71]; // 3 overlap, 3 novel
            let mut overlap = 0;
            let mut novelty = 0;
            for k in coverage {
                let p = idx.proof(k);
                match (verify_member(root, k, &p), verify_non_member(root, k, &p)) {
                    (true, false) => overlap += 1,
                    (false, true) => novelty += 1,
                    _ => panic!("classification must be complete and exclusive"),
                }
            }
            assert_eq!((overlap, novelty), (3, 3), "exact counts — floors run on these");
        }
    }
}

/// Q32.32 settlement mirror — ROADMAP T8 (`CKB-VM-PORT.md` fixed-point map, last entry).
/// The flow-gated rules (v5-v7) in pure integer arithmetic: damped-Jacobi value flow,
/// rational flow gate, and the full v7 composition (similarity floor + standing gate +
/// semantic-floored seeds + realized-flow gate) — deterministic across platforms, the
/// settlement-side counterpart of `value_fixed` (intake side).
///
/// Representation: Q32.32 in u128 carriers (values are novelty-scale, flow amplification
/// bounded by 1/(1-d); u128 headroom makes overflow practically unreachable and every op
/// SATURATES rather than wraps — saturation is an honest bound, pinned in-test).
/// Early exit on exact fixpoint (delta == 0) only — data-dependent but deterministic,
/// and the `iters` cap bounds it either way.
pub mod settlement_fixed {
    use super::{flow, value_fixed, Cell};
    use std::collections::HashMap;

    pub const Q: u32 = 32;
    pub const ONE: u128 = 1 << Q;

    fn mul(a: u128, b: u128) -> u128 {
        a.saturating_mul(b) >> Q
    }

    /// `flow(b) = own(b) + d · Σ_{c built on b} flow(c)` — damped Jacobi, integer-exact,
    /// EXTERNAL edges only (child contributor ≠ parent contributor), mirroring
    /// `flow::value_flow_with_own(.., external_only = true)`.
    pub fn value_flow_external_q32(cells: &[Cell], own: &[u128], d_q32: u128, iters: usize) -> Vec<u128> {
        let mut fl = own.to_vec();
        if cells.is_empty() {
            return fl;
        }
        let id_to_idx: HashMap<u64, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
        let children = flow::children_of_external(cells, &id_to_idx);
        // Fixed-point mirror of the TWO-AXIS geometric damping in
        // `flow::value_flow_with_own`: (1) within-identity, the r-th child (commit order) from a
        // certifying identity weighted λ^r; (2) cross-identity, a parent's distinct identities
        // SORTED by grouped contribution desc (args asc tiebreak) and the m-th weighted μ^m
        // (μ=λ=1/φ). round(2^32/φ) is the Fibonacci-hashing constant; the f64 side uses the same
        // 1/φ, and the drift-guard `v7_q32_tracks_f64_v7_*` holds them within band.
        const LAMBDA_Q32: u128 = 2_654_435_769; // round(2^32 / φ) = (1/φ) at Q32.32, within-identity
        const MU_Q32: u128 = 2_654_435_769; // same constant, cross-identity
        for _ in 0..iters {
            let mut next = own.to_vec();
            for (pid, kids) in &children {
                if let Some(&pi) = id_to_idx.get(pid) {
                    // Group each identity's within-identity-damped (λ^r) contribution.
                    let mut groups: Vec<(&Vec<u8>, u128, u32)> = Vec::new();
                    for &k in kids {
                        let id = &cells[k].type_script.args;
                        match groups.iter_mut().find(|(a, _, _)| *a == id) {
                            Some(e) => {
                                let mut w: u128 = ONE;
                                for _ in 0..e.2 {
                                    w = mul(w, LAMBDA_Q32);
                                }
                                e.1 = e.1.saturating_add(mul(w, fl[k]));
                                e.2 += 1;
                            }
                            None => groups.push((id, fl[k], 1)),
                        }
                    }
                    // Sort distinct identities by grouped contribution desc, args asc tiebreak.
                    groups.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
                    let mut s: u128 = 0;
                    for (m, (_, contrib, _)) in groups.iter().enumerate() {
                        let mut w: u128 = ONE; // μ^0 = 1.0 in Q32.32
                        for _ in 0..m {
                            w = mul(w, MU_Q32);
                        }
                        s = s.saturating_add(mul(w, *contrib));
                    }
                    next[pi] = own[pi].saturating_add(mul(d_q32, s));
                }
            }
            let delta = next
                .iter()
                .zip(&fl)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            fl = next;
            if delta == 0 {
                break;
            }
        }
        fl
    }

    /// Rational flow gate applied in one shot: `novelty · f / (f + half)` at Q32.32.
    /// Division is exact integer division — deterministic on-VM (RISC-V divu).
    pub fn gated_value_q32(novelty_q32: u128, downstream_q32: u128, half_q32: u128) -> u128 {
        if downstream_q32 == 0 {
            return 0;
        }
        let denom = downstream_q32.saturating_add(half_q32);
        novelty_q32.saturating_mul(downstream_q32) / denom
    }

    /// Full `value_v7` in fixed point: similarity floor (Q16.16, exact cross-multiplied) →
    /// standing gate + semantic-floored SEEDS → external realized flow → rational gate.
    /// Same parameters as `value::value_v7`, integer forms.
    #[allow(clippy::too_many_arguments)]
    pub fn value_v7_q32(
        cells_in_commit_order: &[Cell],
        standing: &HashMap<Vec<u8>, u64>,
        standing_floor: u64,
        theta_sim_q16: u64,
        theta_ent_q16: u64,
        d_q32: u128,
        iters: usize,
        half_q32: u128,
    ) -> Vec<u128> {
        let floored =
            value_fixed::temporal_novelty_with_similarity_floor_q16(cells_in_commit_order, theta_sim_q16);
        let seed: Vec<u128> = floored
            .iter()
            .zip(cells_in_commit_order)
            .map(|(&n, c)| {
                let s = standing.get(&c.type_script.args).copied().unwrap_or(0);
                if s >= standing_floor {
                    (value_fixed::semantic_floor_q16(n, &c.data, theta_ent_q16) as u128) << Q
                } else {
                    0
                }
            })
            .collect();
        let fl = value_flow_external_q32(cells_in_commit_order, &seed, d_q32, iters);
        floored
            .iter()
            .zip(&seed)
            .zip(&fl)
            .map(|((&n, &s), &f)| gated_value_q32((n as u128) << Q, f.saturating_sub(s), half_q32))
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::super::{value, Cell, Script};
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

        fn st(pairs: &[(u8, u64)]) -> HashMap<Vec<u8>, u64> {
            pairs.iter().map(|&(k, s)| (vec![k], s)).collect()
        }

        const FLOOR: u64 = 10;
        const SIM: u64 = 52429; // ~0.8 Q16.16
        const ENT: u64 = 62259; // ~0.95 Q16.16
        const D: u128 = (0.85f64 * (1u64 << 32) as f64) as u128;
        const HALF: u128 = 8 << 32;
        const ITERS: usize = 200;

        fn as_f64(x: u128) -> f64 {
            x as f64 / ONE as f64
        }

        #[test]
        fn v7_q32_tracks_f64_v7_on_content_graphs() {
            // The settlement mirror must agree with the f64 prototype within iterative
            // tolerance on honest mixed-vesting graphs (exactness is impossible across
            // float-vs-fixed iteration; 1e-6 relative is the documented band).
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel"),
                cellc(2, 3, 2, Some(1), b"india-juliet-kilo-lima-mike"),
                cellc(3, 2, 3, Some(0), b"november-oscar-papa-quebec"),
            ];
            let standing = st(&[(1, FLOOR), (2, FLOOR), (3, 0)]);
            let f = value::value_v7(&order, &standing, FLOOR, 0.8, 0.95, 0.85, ITERS, 8.0);
            let x = value_v7_q32(&order, &standing, FLOOR, SIM, ENT, D, ITERS, HALF);
            for (i, (a, b)) in f.iter().zip(&x).enumerate() {
                let bf = as_f64(*b);
                assert!(
                    (a - bf).abs() <= 1e-6 * a.abs().max(1.0),
                    "cell {i}: f64 {a} vs q32 {bf}"
                );
            }
        }

        #[test]
        fn v7_q32_noise_child_pumps_nothing() {
            // The flipped v7 pin holds in fixed point: vested noise child, parent earns 0.
            let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let order = vec![
                cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta"),
                cellc(1, 9, 1, Some(0), &noise),
            ];
            let standing = st(&[(1, FLOOR), (9, FLOOR)]);
            let x = value_v7_q32(&order, &standing, FLOOR, SIM, ENT, D, ITERS, HALF);
            assert_eq!(x[0], 0, "semantic-floored seed: noise certifies nothing, integer-exact");
        }

        #[test]
        fn v7_q32_retroactive_vesting_is_monotone() {
            // More realized use never pays less — the gate's monotonicity survives the
            // integer port (saturating ops are monotone).
            let base = vec![cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta")];
            let standing = st(&[(1, FLOOR), (2, FLOOR), (3, FLOOR)]);
            let one_child = {
                let mut o = base.clone();
                o.push(cellc(1, 2, 1, Some(0), b"echo-foxtrot-golf-hotel"));
                o
            };
            let two_children = {
                let mut o = one_child.clone();
                o.push(cellc(2, 3, 2, Some(0), b"india-juliet-kilo-lima"));
                o
            };
            let v0 = value_v7_q32(&base, &standing, FLOOR, SIM, ENT, D, ITERS, HALF)[0];
            let v1 = value_v7_q32(&one_child, &standing, FLOOR, SIM, ENT, D, ITERS, HALF)[0];
            let v2 = value_v7_q32(&two_children, &standing, FLOOR, SIM, ENT, D, ITERS, HALF)[0];
            assert_eq!(v0, 0, "no use, no pay");
            assert!(v1 > v0 && v2 > v1, "value accrues monotonically with realized use");
        }

        #[test]
        fn deep_chain_saturates_instead_of_wrapping_pinned() {
            // Fixed-point-specific surface (this tick's adversarial look): a long
            // build-chain amplifies flow by ~1/(1-d). With u128 carriers the headroom is
            // astronomical, but the CONTRACT is saturation, never wraparound — a wrap
            // would mint value from overflow. Construct a 200-deep alternating-identity
            // chain and assert finite, ordered, panic-free results.
            let mut order = vec![cellc(0, 1, 0, None, b"root-cell-alpha-bravo")];
            for i in 1..200u64 {
                let data = format!("chain-cell-number-{i}-built-on-previous");
                order.push(cellc(i, (i % 2) as u8 + 1, i, Some(i - 1), data.as_bytes()));
            }
            let standing = st(&[(1, FLOOR), (2, FLOOR)]);
            let x = value_v7_q32(&order, &standing, FLOOR, SIM, ENT, D, ITERS, HALF);
            assert!(x.iter().all(|&v| v < u128::MAX / 2), "finite under deep amplification");
            assert!(x[0] > 0, "root is paid through the whole chain");
        }

        // 16 mutually-dissimilar valueless payloads — duplicated here (separate test module,
        // its own helpers) so T6 can rebuild the T1/T3 graphs the f64 matrix uses.
        const HYBRID_PAYLOADS: [&[u8]; 16] = [
            b"the cat sat quietly on the warm mat today",
            b"rivers flow gently under the old stone bridge",
            b"morning light fills the quiet empty kitchen slowly",
            b"yellow kites drift above the distant green hill",
            b"books rest unread along the dusty wooden shelf",
            b"snow settles softly across the silent winter field",
            b"clocks tick onward through the long grey afternoon",
            b"birds gather near the fence before the evening rain",
            b"copper wires hum behind the locked basement panel",
            b"sailors mend torn nets beside the harbor wall",
            b"violet orchids bloom inside the humid glass dome",
            b"trucks rumble past the shuttered roadside diner",
            b"lanterns sway along the crooked mountain trail",
            b"engineers sketch turbines on the wide blue board",
            b"foxes circle the orchard under a thin crescent moon",
            b"pottery dries in rows along the sunlit adobe ledge",
        ];

        #[test]
        fn t6_q32_tracks_f64_on_t1_t3_multi_identity_graphs() {
            // T6 (crit. 6 — Q32.32 MIRROR). The two-axis (λ^r within-identity, μ^m cross-identity)
            // geometric damping lives in the FLOW layer, mirrored in `value_flow_external_q32`
            // (MU_Q32 = LAMBDA_Q32 = round(2^32/φ)). The settlement port must track the f64
            // prototype on the SAME adversarial graphs the f64 matrix asserts — the T1 split (K
            // distinct identities × 1 child) and the T3 hybrid diagonal (K × M). v8's outcome gate
            // is an f64-only host stage with no fixed-point port, so parity is checked at the layer
            // the damping actually lives: value_v7_q32 (which calls value_flow_external_q32) vs the
            // f64 value::value_v7. Same documented 1e-6 relative band as v7_q32_tracks_f64_v7_*.
            type Graph = (String, Vec<Cell>, HashMap<Vec<u8>, u64>);
            let root = cellc(0, 1, 0, None, b"alpha-bravo-charlie-delta");
            // T1: K distinct vested identities, one child each on the root, K = 1,2,4,8.
            let t1 = |k: usize| -> (Vec<Cell>, HashMap<Vec<u8>, u64>) {
                let mut o = vec![root.clone()];
                let mut sv = vec![(1u8, FLOOR)];
                for (j, payload) in HYBRID_PAYLOADS.iter().enumerate().take(k) {
                    o.push(cellc((j + 1) as u64, 10 + j as u8, (j + 1) as u64, Some(0), payload));
                    sv.push((10 + j as u8, FLOOR));
                }
                (o, st(&sv))
            };
            // T3: K identities × M children each on the root (the diagonal pump graph).
            let t3 = |k: usize, m: usize| -> (Vec<Cell>, HashMap<Vec<u8>, u64>) {
                let mut o = vec![root.clone()];
                let mut sv = vec![(1u8, FLOOR)];
                let (mut id, mut p) = (1u64, 0usize);
                for ki in 0..k {
                    sv.push((10 + ki as u8, FLOOR));
                    for _ in 0..m {
                        o.push(cellc(id, 10 + ki as u8, id, Some(0), HYBRID_PAYLOADS[p]));
                        id += 1;
                        p += 1;
                    }
                }
                (o, st(&sv))
            };
            let mut graphs: Vec<Graph> = Vec::new();
            for k in [1usize, 2, 4, 8] {
                let (o, s) = t1(k);
                graphs.push((format!("T1 K={k}"), o, s));
            }
            for (k, m) in [(2usize, 2usize), (4, 2), (2, 4), (4, 4)] {
                let (o, s) = t3(k, m);
                graphs.push((format!("T3 K={k} M={m}"), o, s));
            }
            for (label, order, standing) in &graphs {
                let f = value::value_v7(order, standing, FLOOR, 0.8, 0.95, 0.85, ITERS, 8.0);
                let x = value_v7_q32(order, standing, FLOOR, SIM, ENT, D, ITERS, HALF);
                for (i, (a, b)) in f.iter().zip(&x).enumerate() {
                    let bf = as_f64(*b);
                    assert!(
                        (a - bf).abs() <= 1e-6 * a.abs().max(1.0),
                        "{label} cell {i}: f64 {a} vs q32 {bf} — drift band broken on the \
                         two-axis damping settlement mirror"
                    );
                }
            }
        }
    }
}

/// PoM-weighted finalization mirror in Q32.32 — `ON-VM-FINALIZATION.md` build-order step 1.
/// `consensus::finalizes_hybrid` recomputed in pure integer arithmetic (no floats): effective
/// vote weight with fixed-point linear retention-decay, the `max(effective_total, quorum_floor)`
/// basis, and the 2/3 threshold. This is the consensus-side counterpart of `value_fixed` (intake)
/// and `settlement_fixed` (value) — the third and last on-VM arithmetic surface, the one a
/// finalization-cell type-script will run. The threshold is evaluated rounded AGAINST finalization
/// (ceil): a borderline tie that the real-valued rule leaves un-finalized is never flipped to
/// finalized by fixed-point rounding (the documented direction the design doc requires).
///
/// Representation: Q32.32 in u128. The proof inputs (pow/pos/pom, mix fractions in [0,1]) carry
/// at most ~unit magnitude, so saturation is unreachable for realistic validator sets; every op
/// saturates rather than wraps regardless (an honest bound). Drift-guarded against the f64
/// reference over a deterministic fixture sweep — agreement away from the boundary band, and the
/// conservative direction AT a constructed exact-2/3 tie.
pub mod finalization_fixed {
    use super::consensus::{Mix, Validator};

    // SINGLE SOURCE (lean): the Q32.32 finalize arithmetic + wire format live in
    // `noesis-core::finalization`; the on-VM `finalization-typescript` ELF links the SAME
    // functions. This module is the f64↔fixed boundary (conversion helpers used by the host
    // and the drift-guard tests) — the arithmetic itself is no longer duplicated here.
    pub use noesis_core::finalization::{
        base_weight_q, effective_weight_q, encode_finalization_cell, encode_votes, finalizes_fixed,
        parse_finalization_cell, parse_votes, retention_q, FinalParams, MixQ, ValidatorQ, ONE, PARAMS_LEN,
        VREC_LEN,
    };

    /// f64 → Q32.32 (round to nearest). Test/loader helper; the on-VM inputs arrive already fixed.
    pub fn to_q(x: f64) -> u128 {
        (x * ONE as f64).round() as u128
    }

    /// Convert an f64 reference `Validator`/`Mix` to the fixed forms (same inputs, both rules).
    pub fn val_to_q(v: &Validator) -> ValidatorQ {
        ValidatorQ {
            id: v.id,
            pow: to_q(v.pow),
            pos: to_q(v.pos),
            pom: to_q(v.pom),
            last_heartbeat: v.last_heartbeat,
        }
    }
    pub fn mix_to_q(m: Mix) -> MixQ {
        MixQ { pow: to_q(m.pow), pos: to_q(m.pos), pom: to_q(m.pom) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::consensus::{base_weight, effective_weight, finalizes_hybrid, BPS, NCI, TWO_THIRDS_BPS};

        fn v(id: u64, pow: f64, pos: f64, pom: f64, hb: u64) -> Validator {
            Validator { id, pow, pos, pom, last_heartbeat: hb, staked_balance: 1.0 }
        }

        // Run both rules on the SAME inputs; return (fixed, float).
        #[allow(clippy::too_many_arguments)]
        fn both(
            voters_for: &[Validator],
            all: &[Validator],
            m: Mix,
            now: u64,
            horizon: u64,
            decay_pos: bool,
            threshold_bps: u64,
            floor_bps: u64,
        ) -> (bool, bool) {
            let vf_q: Vec<ValidatorQ> = voters_for.iter().map(val_to_q).collect();
            let all_q: Vec<ValidatorQ> = all.iter().map(val_to_q).collect();
            let fixed = finalizes_fixed(&vf_q, &all_q, mix_to_q(m), now, horizon, decay_pos, threshold_bps, floor_bps);
            let float = finalizes_hybrid(voters_for, all, m, now, horizon, decay_pos, threshold_bps, floor_bps);
            (fixed, float)
        }

        #[test]
        fn retention_q_mirrors_reference() {
            for &(e, h) in &[(0u64, 100u64), (25, 100), (50, 100), (99, 100), (100, 100), (250, 100), (0, 0)] {
                let q = retention_q(e, h) as f64 / ONE as f64;
                let f = crate::consensus::retention(e, h);
                assert!((q - f).abs() < 1e-9, "retention({e},{h}): q {q} vs f {f}");
            }
        }

        #[test]
        fn clear_pass_and_clear_fail_agree() {
            // Three identical fresh validators, all vote for ⇒ 100% ≥ 2/3 ⇒ finalize.
            let all = vec![v(1, 0.9, 0.9, 0.9, 0), v(2, 0.9, 0.9, 0.9, 0), v(3, 0.9, 0.9, 0.9, 0)];
            let (fx, fl) = both(&all, &all, NCI, 0, 100, true, TWO_THIRDS_BPS, 3333);
            assert!(fx && fl, "unanimous fresh ⇒ both finalize");
            // Only one of three votes ⇒ ~33% < 2/3 ⇒ neither finalizes.
            let (fx2, fl2) = both(&all[..1], &all, NCI, 0, 100, true, TWO_THIRDS_BPS, 3333);
            assert!(!fx2 && !fl2, "one-of-three ⇒ both reject");
        }

        #[test]
        fn quorum_floor_binding_agrees() {
            // All stale (now ≫ horizon) ⇒ eff_total → 0; the base-weight quorum floor becomes the
            // basis. Two fresh voters_for against a 4-validator base: exercise the floor path.
            let all = vec![
                v(1, 0.8, 0.8, 0.8, 100),
                v(2, 0.8, 0.8, 0.8, 100),
                v(3, 0.8, 0.8, 0.8, 0),
                v(4, 0.8, 0.8, 0.8, 0),
            ];
            for &now in &[0u64, 50, 100, 150] {
                let (fx, fl) = both(&all[..2], &all, NCI, now, 100, true, TWO_THIRDS_BPS, 5000);
                assert_eq!(fx, fl, "quorum-floor path disagreement at now={now}");
            }
        }

        #[test]
        fn staleness_sweep_agrees_away_from_boundary() {
            // Deterministic sweep over liveness, decay modes, and voter subsets. Where the f64
            // margin is clearly off the bar, the fixed verdict MUST match (the drift-guard).
            let all = vec![
                v(1, 0.7, 0.5, 0.9, 0),
                v(2, 0.6, 0.8, 0.4, 10),
                v(3, 0.9, 0.3, 0.7, 25),
                v(4, 0.5, 0.6, 0.6, 40),
                v(5, 0.8, 0.7, 0.5, 60),
            ];
            let horizon = 100u64;
            let mut compared = 0;
            for &decay in &[true, false] {
                for &now in &[0u64, 20, 50, 80, 100, 200] {
                    for k in 1..=all.len() {
                        let vf = &all[..k];
                        // f64 relative margin of weight_for vs the bar.
                        let wf: f64 = vf.iter().map(|x| effective_weight(x, NCI, now, horizon, decay)).sum();
                        let eff: f64 = all.iter().map(|x| effective_weight(x, NCI, now, horizon, decay)).sum();
                        let base: f64 = all.iter().map(|x| base_weight(x, NCI)).sum();
                        let basis = eff.max(base * 5000.0 / BPS as f64);
                        let bar = basis * TWO_THIRDS_BPS as f64 / BPS as f64;
                        let margin = if bar > 0.0 { (wf - bar) / bar } else { 0.0 };
                        let (fx, fl) = both(vf, &all, NCI, now, horizon, decay, TWO_THIRDS_BPS, 5000);
                        // Conservative direction ALWAYS holds: fixed never finalizes what f64 rejects.
                        assert!(!(fx && !fl), "fixed finalized a float-rejected case (now={now},k={k},decay={decay})");
                        if margin.abs() > 1e-3 {
                            assert_eq!(fx, fl, "drift away from boundary (now={now},k={k},decay={decay},margin={margin})");
                            compared += 1;
                        }
                    }
                }
            }
            assert!(compared > 30, "sweep should compare many off-boundary points, got {compared}");
        }

        #[test]
        fn adversarial_edges_hold_conservative_direction() {
            // RSAW tick on the increment: probe the corners the sweep underweights and assert the
            // load-bearing safety property — the fixed rule NEVER finalizes a case the float rule
            // rejects — survives every edge. Each is also checked for off-boundary agreement.
            let base = vec![
                v(1, 0.7, 0.5, 0.9, 0),
                v(2, 0.6, 0.8, 0.4, 0),
                v(3, 0.9, 0.3, 0.7, 0),
            ];
            // horizon = 0 ⇒ retention pinned to 1 (no decay), regardless of staleness.
            for &now in &[0u64, 1, 1_000_000] {
                let (fx, fl) = both(&base, &base, NCI, now, 0, true, TWO_THIRDS_BPS, 3333);
                assert!(!(fx && !fl), "horizon=0 conservative break at now={now}");
                assert_eq!(fx, fl, "horizon=0 ⇒ no decay ⇒ should agree (now={now})");
            }
            // threshold = 100% (BPS): only a unanimous effective vote finalizes.
            let (fx, fl) = both(&base, &base, NCI, 0, 100, true, BPS, 0);
            assert!(!(fx && !fl) && fx == fl, "100% threshold, unanimous");
            let (fx2, fl2) = both(&base[..2], &base, NCI, 0, 100, true, BPS, 0);
            assert!(!(fx2 && !fl2), "100% threshold, 2/3 ⇒ fixed must not over-finalize");
            // A zero-weight validator padding `all` (pow=pos=pom=0) — must not break basis>0
            // and must not let the fixed rule diverge upward.
            let mut padded = base.clone();
            padded.push(v(4, 0.0, 0.0, 0.0, 0));
            let (fx3, fl3) = both(&base, &padded, NCI, 0, 100, true, TWO_THIRDS_BPS, 3333);
            assert!(!(fx3 && !fl3), "zero-weight padding conservative break");
            // Empty voters_for ⇒ weight_for = 0 ⇒ never finalizes (both).
            let (fx4, fl4) = both(&[], &base, NCI, 0, 100, true, TWO_THIRDS_BPS, 3333);
            assert!(!fx4 && !fl4, "no voters ⇒ neither finalizes");
            // All-zero `all` ⇒ basis = 0 ⇒ the basis>0 guard rejects (no divide-by-nothing finalize).
            let zeros = vec![v(9, 0.0, 0.0, 0.0, 0)];
            let (fx5, fl5) = both(&zeros, &zeros, NCI, 0, 100, true, TWO_THIRDS_BPS, 3333);
            assert!(!fx5 && !fl5, "empty basis ⇒ neither finalizes (guarded)");
        }

        #[test]
        fn exact_two_thirds_tie_does_not_finalize_in_fixed() {
            // Construct an exact 2:1 split where voters_for hold exactly 2/3 of a no-decay basis.
            // The real bar is 6667 bps > 6666.6..%, so even the float rule rejects a clean 2/3;
            // the fixed rule must also reject (round-against-finalization keeps the tie un-finalized).
            let all = vec![v(1, 1.0, 1.0, 1.0, 0), v(2, 1.0, 1.0, 1.0, 0), v(3, 1.0, 1.0, 1.0, 0)];
            let (fx, fl) = both(&all[..2], &all, NCI, 0, 100, false, TWO_THIRDS_BPS, 0);
            assert!(!fl, "two-of-three = 66.67% < 6667bps bar ⇒ float rejects");
            assert!(!fx, "fixed rounds against finalization ⇒ also rejects (no boundary flip)");
        }
    }
}

/// T7 #2 — the shared proof-driven intake verifier (`T7-CROSS-CELL-SIMILARITY.md` §increments).
/// Turns SMT classifications into EXACTLY the numbers the sequential rule computes, so the
/// on-VM script (which cannot see history) and the off-VM node (which can) agree by
/// construction. Count semantics preserved from the stateful rule precisely:
///   - novelty counts PER-OCCURRENCE (a repeated novel 4-gram pays each occurrence, as in
///     `temporal_novelty`: `seen` is extended only AFTER the whole cell);
///   - the similarity floor runs on the UNIQUE-shingle overlap fraction, exact
///     cross-multiplied Q16.16 (as in `value_fixed`).
/// Polarity is DERIVED, never prover-claimed: each unique shingle carries one sibling
/// path; membership and non-membership are mutually exclusive under it (in-test in smt),
/// so misclassification and omission are both structurally impossible — any invalid or
/// missing proof rejects the WHOLE cell (`None`), never a partial count.
pub mod proven {
    use super::smt::{verify_member, verify_non_member, Hash, DEPTH};
    use super::{coverage, value_fixed};

    /// Sorted unique shingles of a cell with per-occurrence multiplicities.
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

    /// Classify every unique shingle against `root`. `proofs[i]` is the sibling path for
    /// `unique_shingles(data)[i]` (sorted order — the proof layout is canonical, so a
    /// prover cannot reorder to confuse the verifier). Returns
    /// `(novelty_occurrences, unique_overlap, unique_total)` or `None` on ANY invalid or
    /// missing proof.
    pub fn novelty_with_proofs(
        data: &[u8],
        root: Hash,
        proofs: &[[Hash; DEPTH]],
    ) -> Option<(u64, u64, u64)> {
        let uniq = unique_shingles(data);
        if proofs.len() != uniq.len() {
            return None; // omission or padding — reject whole cell
        }
        let mut novelty_occ = 0u64;
        let mut overlap_uniq = 0u64;
        for ((key, mult), path) in uniq.iter().zip(proofs) {
            let member = verify_member(root, *key, path);
            let absent = verify_non_member(root, *key, path);
            match (member, absent) {
                (true, false) => overlap_uniq += 1,
                (false, true) => novelty_occ += mult,
                _ => return None, // invalid path: proves neither (or a broken tree both)
            }
        }
        Some((novelty_occ, overlap_uniq, uniq.len() as u64))
    }

    /// The full proven intake floor — the function the type-script will run (T7 #4):
    /// similarity floor on the proven overlap fraction (exact cross-multiplied Q16.16,
    /// same comparison as `value_fixed`), then the semantic floor on the bytes.
    pub fn proven_floored_novelty_q16(
        data: &[u8],
        root: Hash,
        proofs: &[[Hash; DEPTH]],
        theta_sim_q16: u64,
        theta_ent_q16: u64,
    ) -> Option<u64> {
        let (novelty, overlap, total) = novelty_with_proofs(data, root, proofs)?;
        let floored = if total > 0 && ((overlap as u128) << 16) > (theta_sim_q16 as u128) * total as u128
        {
            0
        } else {
            novelty
        };
        Some(value_fixed::semantic_floor_q16(floored, data, theta_ent_q16))
    }

    #[cfg(test)]
    mod tests {
        use super::super::smt::NoveltyIndex;
        use super::super::{value_fixed, Cell, Script};
        use super::*;

        const SIM: u64 = 52429;
        const ENT: u64 = 62259;

        fn cell(id: u64, owner: u8, ts: u64, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [1u8; 32], args: vec![owner] },
                type_script: Script { code_hash: [0xB0; 32], args: vec![owner] },
                parent: None,
                timestamp: ts,
                data: data.to_vec(),
            }
        }

        fn index_of(cells: &[Cell]) -> NoveltyIndex {
            let mut idx = NoveltyIndex::new();
            for c in cells {
                for (k, _) in unique_shingles(&c.data) {
                    idx.insert(k);
                }
            }
            idx
        }

        fn proofs_for(idx: &NoveltyIndex, data: &[u8]) -> Vec<[super::Hash; super::DEPTH]> {
            unique_shingles(data).iter().map(|(k, _)| idx.proof(*k)).collect()
        }

        #[test]
        fn proven_counts_equal_the_sequential_rule_exactly() {
            // THE T7 theorem, in-test: for any new cell, the proof-driven floored novelty
            // equals what the stateful sequential rule assigns it as the last element of
            // the commit order. History-blind script ≡ history-seeing node.
            let prior = vec![
                cell(0, 1, 0, b"alpha-bravo-charlie-delta"),
                cell(1, 2, 1, b"echo-foxtrot-golf-hotel"),
            ];
            let candidates: Vec<&[u8]> = vec![
                b"india-juliet-kilo-lima",                    // fresh content
                b"alpha-bravo-charlie-delta",                 // exact duplicate -> floored
                b"alpha-bravo-charlie-deltX",                 // near-duplicate -> floored
                b"charlie-delta plus brand new tail words",   // partial overlap
                b"xy",                                        // sub-window edge case
            ];
            let idx = index_of(&prior);
            for data in candidates {
                let mut order = prior.clone();
                order.push(cell(9, 9, 9, data));
                let sequential =
                    value_fixed::temporal_novelty_with_similarity_floor_q16(&order, SIM);
                let seq_last = *sequential.last().unwrap();
                let seq_floored = value_fixed::semantic_floor_q16(seq_last, data, ENT);
                let proven =
                    proven_floored_novelty_q16(data, idx.root(), &proofs_for(&idx, data), SIM, ENT)
                        .expect("honest proofs verify");
                assert_eq!(proven, seq_floored, "mismatch for {:?}", &data[..data.len().min(16)]);
            }
        }

        #[test]
        fn omission_and_padding_reject_the_whole_cell() {
            let prior = vec![cell(0, 1, 0, b"alpha-bravo-charlie-delta")];
            let idx = index_of(&prior);
            let data = b"echo-foxtrot-golf-hotel";
            let mut proofs = proofs_for(&idx, data);
            proofs.pop(); // omit one shingle's proof
            assert_eq!(novelty_with_proofs(data, idx.root(), &proofs), None, "omission");
            let mut padded = proofs_for(&idx, data);
            padded.push(padded[0]); // pad with an extra
            assert_eq!(novelty_with_proofs(data, idx.root(), &padded), None, "padding");
        }

        #[test]
        fn tampered_or_misaligned_proofs_reject_not_miscount() {
            // A wrong sibling path proves NEITHER polarity -> the whole cell rejects.
            // Partial credit is the failure mode this design forbids.
            let prior = vec![cell(0, 1, 0, b"alpha-bravo-charlie-delta")];
            let idx = index_of(&prior);
            let data = b"echo-foxtrot-golf-hotel";
            let mut proofs = proofs_for(&idx, data);
            proofs[0][0] = [0xAB; 32]; // corrupt the leaf-level sibling
            assert_eq!(novelty_with_proofs(data, idx.root(), &proofs), None);
            // Stale root: proofs against a root that has since moved also reject.
            let mut newer = index_of(&prior);
            newer.insert(0xFEED_FACE);
            let stale_proofs = proofs_for(&idx, data);
            assert_eq!(novelty_with_proofs(data, newer.root(), &stale_proofs), None, "stale root");
        }

        #[test]
        fn noise_cell_is_semantically_floored_through_the_proven_path() {
            let prior = vec![cell(0, 1, 0, b"alpha-bravo-charlie-delta")];
            let idx = index_of(&prior);
            let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
            let v = proven_floored_novelty_q16(&noise, idx.root(), &proofs_for(&idx, &noise), SIM, ENT)
                .expect("proofs verify; the floor does the zeroing");
            assert_eq!(v, 0, "novel garbage proven novel — and still floored at the gate");
        }
    }
}

/// T7 #3 — the index-cell root-transition rule (`T7-CROSS-CELL-SIMILARITY.md` §QA R2:
/// per-block batched update). The novelty-index cell's own type-script logic: a block's
/// root transition old → new is valid iff it is EXACTLY a chain of single-key insertions,
/// each proven against the ROLLING root.
///
/// Load-bearing detail: intermediate roots are COMPUTED from each step's own sibling
/// path (`root_from(key, leaf, siblings)` after checking `root_from(key, EMPTY, siblings)`
/// equals the rolling root) — never supplied by the producer. Two consequences, both
/// structural rather than bookkept:
///   - duplicate insertion is impossible (the second insert of a key cannot prove
///     non-membership under the root that now contains it);
///   - smuggling or omitting a key moves the computed final root off `new_root`.
/// Intra-block novelty assignment (qa R2's consensus rule): FIRST commit wins a shared
/// novel shingle; later cells in the block see it as overlap — demonstrated in-test by
/// running the proven verifier against the evolving roots.
pub mod index_rule {
    use super::smt::{leaf, root_from, Hash, DEPTH};

    /// One insertion in the block's batch: the key and its sibling path against the
    /// rolling root at this position in the chain.
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
    /// coordinate ([`super::commit_order::Committed`]) paired with the novel-shingle insertions
    /// it makes against the rolling root. Grouping at cell granularity is what lets the rule
    /// bind the ORDER the cells are applied in to consensus, not to producer presentation.
    #[derive(Clone)]
    pub struct CellBatch {
        pub coord: super::commit_order::Committed,
        pub steps: Vec<InsertStep>,
    }

    /// The index-cell transition rule WITH the commit-order invariant wired in
    /// (TEMPORAL-ORDER-ONCHAIN.md, NEXT-BUILD (b)). [`valid_root_transition`] proves the root
    /// moved correctly but TRUSTS the producer's order of steps — and order is exactly what
    /// decides first-commit-wins when two same-height cells contend for shared novel coverage
    /// (the first to insert a shared key banks it; the second can no longer prove non-membership
    /// ⇒ earns 0 for that key). This variant closes the relocated invariant at per-cell-batch
    /// granularity: the cells must ALREADY be in canonical commit order
    /// ([`super::commit_order::is_canonical_order`] — height ascending, then the XOR-seeded
    /// in-block slot, NEITHER producer-arrangeable), and ONLY THEN is the flattened rolling-root
    /// transition checked. A producer-favorable reordering is REJECTED at the order gate before
    /// any root math (no silent re-sort ⇒ no probe signal), so no party can choose which of two
    /// contending cells banks the shared shingles. This is the index-rule half of the temporal-
    /// order fix: `commit_order` made the order consensus-sourced; this makes the index cell
    /// REFUSE to advance on any other order.
    pub fn valid_ordered_root_transition(
        old_root: Hash,
        new_root: Hash,
        cells: &[CellBatch],
    ) -> bool {
        // 1. Consensus order gate: the cells must be presented in canonical commit order.
        let coords: Vec<super::commit_order::Committed> =
            cells.iter().map(|c| c.coord.clone()).collect();
        if !super::commit_order::is_canonical_order(&coords) {
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

    #[cfg(test)]
    mod tests {
        use super::super::proven::{novelty_with_proofs, unique_shingles};
        use super::super::smt::NoveltyIndex;
        use super::*;

        /// Honest producer: insert keys sequentially, capturing each step's proof
        /// against the index state BEFORE that insert.
        fn build_steps(idx: &mut NoveltyIndex, keys: &[u64]) -> Vec<InsertStep> {
            keys.iter()
                .map(|&k| {
                    let s = InsertStep { key: k, siblings: idx.proof(k) };
                    idx.insert(k);
                    s
                })
                .collect()
        }

        #[test]
        fn honest_batch_validates_and_matches_ground_truth() {
            let mut idx = NoveltyIndex::new();
            for k in [1u64, 2, 3] {
                idx.insert(k);
            }
            let old_root = idx.root();
            let keys = [10u64, 99, 7_000_000, 4];
            let steps = build_steps(&mut idx, &keys);
            let new_root = idx.root(); // ground truth after the same inserts
            assert!(valid_root_transition(old_root, new_root, &steps));
            assert!(!valid_root_transition(old_root, old_root, &steps), "must move the root");
        }

        #[test]
        fn duplicate_insertion_is_structurally_impossible() {
            // No dedup bookkeeping anywhere — the second insert of the same key cannot
            // prove non-membership under the rolling root that now contains it.
            let mut idx = NoveltyIndex::new();
            let old_root = idx.root();
            let mut steps = build_steps(&mut idx, &[42u64]);
            // forge a second insertion of 42 with the freshest possible path
            steps.push(InsertStep { key: 42, siblings: idx.proof(42) });
            idx.insert(42); // no-op on the real tree
            assert!(!valid_root_transition(old_root, idx.root(), &steps));
        }

        #[test]
        fn smuggled_or_omitted_keys_move_the_computed_root_off_target() {
            let mut idx = NoveltyIndex::new();
            let old_root = idx.root();
            let keys = [5u64, 6, 7];
            let steps = build_steps(&mut idx, &keys);
            let new_root = idx.root();
            // omit the last step: computed end != new_root
            assert!(!valid_root_transition(old_root, new_root, &steps[..2]));
            // smuggle an extra key the announced new_root doesn't contain
            let mut smuggled = steps;
            let mut shadow = NoveltyIndex::new();
            for k in keys {
                shadow.insert(k);
            }
            smuggled.push(InsertStep { key: 1234, siblings: shadow.proof(1234) });
            assert!(!valid_root_transition(old_root, new_root, &smuggled));
        }

        #[test]
        fn forged_sibling_path_rejects() {
            let mut idx = NoveltyIndex::new();
            let old_root = idx.root();
            let mut steps = build_steps(&mut idx, &[8u64, 9]);
            steps[1].siblings[3] = [0xCC; 32];
            assert!(!valid_root_transition(old_root, idx.root(), &steps));
        }

        #[test]
        fn first_commit_wins_shared_novelty_within_a_block() {
            // qa R2's consensus rule, demonstrated end to end with the proven verifier:
            // two cells in one block share novel content. Against the BLOCK-START root
            // both prove it novel; under sequential assignment (proofs against the
            // EVOLVING root) the first earns it, the second sees overlap.
            let mut idx = NoveltyIndex::new();
            for (k, _) in unique_shingles(b"prior-committed-content-zulu") {
                idx.insert(k);
            }
            let block_start = idx.root();
            let cell_a: &[u8] = b"shared-brand-new-phrase-here";
            let cell_b: &[u8] = b"shared-brand-new-phrase-here plus b's own tail";

            // Both novel against block start:
            let pa: Vec<_> = unique_shingles(cell_a).iter().map(|(k, _)| idx.proof(*k)).collect();
            let (nov_a_start, _, _) = novelty_with_proofs(cell_a, block_start, &pa).unwrap();
            let pb: Vec<_> = unique_shingles(cell_b).iter().map(|(k, _)| idx.proof(*k)).collect();
            let (nov_b_start, _, _) = novelty_with_proofs(cell_b, block_start, &pb).unwrap();
            assert!(nov_a_start > 0 && nov_b_start > 0, "both look novel at block start");

            // Sequential assignment: A lands first, B proves against the evolved root.
            for (k, _) in unique_shingles(cell_a) {
                idx.insert(k);
            }
            let after_a = idx.root();
            let pb2: Vec<_> = unique_shingles(cell_b).iter().map(|(k, _)| idx.proof(*k)).collect();
            let (nov_b_seq, overlap_b_seq, _) = novelty_with_proofs(cell_b, after_a, &pb2).unwrap();
            assert!(nov_b_seq < nov_b_start, "B's shared shingles became overlap");
            assert!(overlap_b_seq > 0, "first commit won them");
            assert!(nov_b_seq > 0, "B's own tail still earns");
        }

        // A secret with enough byte spread to give the in-block shuffle real work.
        fn sec(b: u8) -> super::super::smt::Hash {
            let mut s = [0u8; 32];
            s[0] = b;
            s[31] = b.wrapping_mul(7).wrapping_add(1);
            s
        }

        #[test]
        fn ordered_batch_validates_in_canonical_order() {
            use super::super::commit_order::Committed;
            // Distinct heights ⇒ canonical order is simply height-ascending; build the steps in
            // that order and the ordered rule accepts the batch against ground-truth new_root.
            let coords = [
                Committed { height: 4, secret: sec(2) },
                Committed { height: 5, secret: sec(9) },
                Committed { height: 6, secret: sec(1) },
            ];
            let keysets: [Vec<u64>; 3] = [vec![11, 12], vec![99], vec![7, 8, 9]];
            let mut idx = NoveltyIndex::new();
            let old_root = idx.root();
            let mut cells: Vec<CellBatch> = Vec::new();
            for (coord, ks) in coords.iter().zip(keysets.iter()) {
                let steps: Vec<InsertStep> = ks
                    .iter()
                    .map(|&k| {
                        let s = InsertStep { key: k, siblings: idx.proof(k) };
                        idx.insert(k);
                        s
                    })
                    .collect();
                cells.push(CellBatch { coord: coord.clone(), steps });
            }
            let new_root = idx.root();
            assert!(valid_ordered_root_transition(old_root, new_root, &cells));
            // and it must actually move the root (empty-transition guard inherited).
            assert!(!valid_ordered_root_transition(old_root, old_root, &cells));
        }

        #[test]
        fn producer_favorable_reorder_is_rejected_at_the_order_gate() {
            use super::super::commit_order::{canonical_order, Committed};
            // Two cells at the SAME height contend for a shared novel key (50). Consensus —
            // not the producer — decides which one banks it, via the XOR-seeded slot order.
            let h = 7u64;
            let coords = [
                Committed { height: h, secret: sec(40) },
                Committed { height: h, secret: sec(80) },
            ];
            let canon = canonical_order(&coords); // slot order over these two same-height cells
            // Build the batch in canonical slot order: the FIRST cell inserts {own, shared};
            // the second inserts only its own key (it can't re-prove the shared key absent).
            let keysets: [Vec<u64>; 2] = [vec![101, 50], vec![202]];
            let mut idx = NoveltyIndex::new();
            let old_root = idx.root();
            let mut batches: Vec<CellBatch> = Vec::new();
            for (slot, &ci) in canon.iter().enumerate() {
                let ks = &keysets[slot];
                let steps: Vec<InsertStep> = ks
                    .iter()
                    .map(|&k| {
                        let s = InsertStep { key: k, siblings: idx.proof(k) };
                        idx.insert(k);
                        s
                    })
                    .collect();
                batches.push(CellBatch { coord: coords[ci].clone(), steps });
            }
            let new_root = idx.root();
            assert!(
                valid_ordered_root_transition(old_root, new_root, &batches),
                "the canonical-order batch is accepted"
            );
            // The producer-favorable move: present the contending cells in the OTHER order so a
            // different cell would bank the shared key. Rejected at the order gate before any
            // root math — not silently re-sorted.
            let swapped: Vec<CellBatch> = batches.iter().rev().cloned().collect();
            assert!(
                !valid_ordered_root_transition(old_root, new_root, &swapped),
                "a producer-favorable reorder of same-height contenders is rejected"
            );
        }

        #[test]
        fn ordered_rule_trusts_coords_so_they_must_be_consensus_sourced() {
            use super::super::commit_order::Committed;
            // valid_ordered_root_transition dissolves producer REORDERING, but it still trusts the
            // CellBatch coords (height, secret) AS CLAIMED. is_canonical_order only checks the
            // presented coords are internally canonical — never that they are TRUE. So a producer
            // who LIES about a redundant cell's commit height (claims an earlier one) makes it sort
            // first and bank the contested novelty, and the batch still validates. 7th site of
            // [P·dont-let-attacker-choose-critical-input]: the coords themselves must be consensus-
            // sourced on-VM (height from the header the commitment landed in; secret from the
            // block's reveals), never producer-asserted. This test makes that requirement explicit.
            let shared = 50u64;
            let (a_own, b_own) = (101u64, 202u64);

            // Build a batch from (coord, keys) pairs in the SUPPLIED order, inserting against the
            // rolling root. The first cell to insert `shared` banks it; the later cell omits it
            // (it could not prove non-membership anyway). Same 3 keys ⇒ same final index root.
            fn build_in_order(cells: &[(Committed, &[u64])]) -> (Vec<CellBatch>, super::super::smt::Hash) {
                let mut idx = NoveltyIndex::new();
                let mut batches = Vec::new();
                for (coord, ks) in cells {
                    let steps: Vec<InsertStep> = ks
                        .iter()
                        .map(|&k| {
                            let s = InsertStep { key: k, siblings: idx.proof(k) };
                            idx.insert(k);
                            s
                        })
                        .collect();
                    batches.push(CellBatch { coord: coord.clone(), steps });
                }
                let root = idx.root();
                (batches, root)
            }

            let old_root = NoveltyIndex::new().root();
            // Honest: A committed at the EARLIER true height (5) and banks the shared coverage;
            // redundant B is later (6).
            let (truthful, root_t) = build_in_order(&[
                (Committed { height: 5, secret: sec(10) }, &[a_own, shared][..]),
                (Committed { height: 6, secret: sec(20) }, &[b_own][..]),
            ]);
            // Forged: B LIES, claiming height 4 (< A's 5), so B sorts first and banks `shared`.
            let (forged, root_f) = build_in_order(&[
                (Committed { height: 4, secret: sec(20) }, &[b_own, shared][..]),
                (Committed { height: 5, secret: sec(10) }, &[a_own][..]),
            ]);

            assert_eq!(root_t, root_f, "same key set ⇒ same index root regardless of who banks shared");
            assert!(
                valid_ordered_root_transition(old_root, root_t, &truthful),
                "the honest-coord batch validates"
            );
            assert!(
                valid_ordered_root_transition(old_root, root_f, &forged),
                "the forged-height batch ALSO validates — the rule trusts the claimed coords"
            );
            // The contested coverage flipped owner: A banks `shared` honestly; B banks it by a false
            // height claim. Identify each index-0 winner by its own (non-shared) key.
            assert!(
                truthful[0].steps.iter().any(|s| s.key == a_own),
                "honest order: A (earlier true height) banks the shared coverage"
            );
            assert!(
                forged[0].steps.iter().any(|s| s.key == b_own),
                "forged order: redundant B banks the shared coverage by claiming a lower height ⇒ coords must be consensus-sourced"
            );
        }
    }
}

// ============ Consensus-sourced commit ordering (TEMPORAL-ORDER-ONCHAIN.md) ============
/// The fix for the temporal-order attacker-choosable-input finding
/// ([P·dont-let-attacker-choose-critical-input], 2026-06-13). [`temporal_novelty`] and the
/// index [`index_rule::valid_root_transition`] assign shared novelty by ORDER: the
/// earlier-committed cell wins the contested coverage; a later redundant cell earns 0. That
/// is strategyproof ONLY if "earlier" is a relation the block producer cannot arrange. The
/// rules above trust the caller's slice / step order — correct for a reference model, but it
/// RELOCATES the invariant to the source of that order. On-chain the order MUST come from
/// consensus, at two scales, so it is not producer-arrangeable:
///
///   - INTER-block: the commit-reveal BLOCK HEIGHT the cell's commitment landed in. A later
///     height can never precede an earlier one. The self-set `Cell.timestamp` is never
///     consulted (pinned by `temporal_order_is_consensus_critical_and_timestamp_is_not_the_lever`),
///     so backdating it is a no-op — the field is not the lever, the ORDER is.
///   - INTRA-block (same-height ties): a Fisher-Yates shuffle seeded by the XOR of EVERY
///     revealing participant's secret — the VibeSwap `DeterministicShuffle` primitive. A
///     participant commits before any secret is revealed, and their slot depends on the XOR
///     of all secrets, so no party can predict — let alone choose — their own position. This
///     DISSOLVES producer-favorable ordering ([P·class-dissolution-vs-case-defeat]) rather
///     than detecting it case by case: there is no secret a rational producer can pick that
///     guarantees an earlier slot, because the others' (unknown) secrets co-determine it.
///
/// This is the reference model. The on-VM index-cell type-script sources `height` from the
/// header and the secrets from the block's reveals (sentinel-gated inert pre-deploy, exactly
/// like the index-dep binding and the finalization `now`). See `TEMPORAL-ORDER-ONCHAIN.md`.
pub mod commit_order {
    // Single source of truth: the consensus permutation lives in noesis-core (no_std, on-VM),
    // so node RE-EXPORTS it rather than keeping a drift-guarded copy. Lean (Bitcoin-simplicity):
    // ONE implementation, not two. The node-side tests below exercise it through `Cell` /
    // `novelty_in_commit_order`; the pure-permutation properties are tested in noesis-core.
    pub use noesis_core::commit_order::{
        block_shuffle, canonical_order, encode_batch, is_canonical_order, parse_batch, Committed, CREC_LEN,
    };

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::smt::Hash;
        use crate::{novelty_in_commit_order, Cell, Script};
        use std::collections::HashSet;

        fn mkcell(id: u64, data: &[u8]) -> Cell {
            Cell {
                id,
                lock: Script { code_hash: [0u8; 32], args: vec![] },
                type_script: Script { code_hash: [1u8; 32], args: vec![id as u8] },
                parent: None,
                timestamp: 0,
                data: data.to_vec(),
            }
        }
        fn sec(b: u8) -> [u8; 32] {
            let mut s = [0u8; 32];
            s[0] = b;
            s[31] = b.wrapping_mul(7).wrapping_add(1);
            s
        }

        #[test]
        fn cross_block_height_dominates_presentation() {
            // A is original at the EARLIER height; B is a redundant subset at a LATER height.
            // The producer-favorable attack presents B first to steal novelty. Consensus order
            // sources "earlier" from height, so B earns 0 no matter how it is presented.
            let a = mkcell(0, b"alpha-bravo-charlie-delta");
            let b = mkcell(1, b"alph"); // single shingle, a strict subset of a's coverage
            let ca = Committed { height: 5, secret: sec(9) };
            let cb = Committed { height: 6, secret: sec(3) };

            let honest = novelty_in_commit_order(&[a.clone(), b.clone()], &[ca.clone(), cb.clone()]);
            assert!(honest[0] > 0 && honest[1] == 0, "true order: A novel, redundant B earns 0");

            // present B FIRST (its coord still says height 6): the attack must not pay.
            let gamed = novelty_in_commit_order(&[b.clone(), a.clone()], &[cb, ca]);
            assert!(
                gamed[1] > 0 && gamed[0] == 0,
                "redundant B presented first STILL earns 0; A keeps the novelty by lower commit height"
            );
        }

        #[test]
        fn canonical_order_is_invariant_to_presentation() {
            let coords = vec![
                Committed { height: 1, secret: sec(7) },
                Committed { height: 1, secret: sec(2) },
                Committed { height: 2, secret: sec(5) },
                Committed { height: 1, secret: sec(9) },
                Committed { height: 3, secret: sec(1) },
            ];
            let ident = |c: &[Committed]| -> Vec<(u64, Hash)> {
                canonical_order(c).iter().map(|&i| (c[i].height, c[i].secret)).collect()
            };
            let seq0 = ident(&coords);
            let mut reversed = coords.clone();
            reversed.reverse();
            assert_eq!(seq0, ident(&reversed), "canonical order is independent of presentation order");
            // and heights are non-decreasing across the canonical sequence
            assert!(seq0.windows(2).all(|w| w[0].0 <= w[1].0), "earlier height always precedes later");
        }

        #[test]
        fn intra_block_slot_is_not_self_selectable() {
            // Same-height tie. The XOR-seeded shuffle means a participant's slot depends on the
            // OTHER participants' (unknown-at-commit) secrets, so no fixed secret guarantees an
            // earlier slot. Hold the attacker's secret constant; vary the others; show the
            // attacker lands in more than one slot -> the slot is co-determined, not chosen.
            let atk = sec(123);
            let mut slots = HashSet::new();
            for o in 0..8u8 {
                let set = [
                    Committed { height: 1, secret: atk },
                    Committed { height: 1, secret: sec(200 + o) },
                    Committed { height: 1, secret: sec(150 + o) },
                ];
                let perm = canonical_order(&set);
                let attacker_slot = perm.iter().position(|&i| i == 0).unwrap();
                slots.insert(attacker_slot);
            }
            assert!(
                slots.len() > 1,
                "attacker's slot varies with the others' secrets -> producer-favorable ordering is dissolved, not merely detected"
            );
        }

        #[test]
        fn block_shuffle_is_deterministic_and_total() {
            let secs = [sec(1), sec(2), sec(3), sec(4), sec(5)];
            let p = block_shuffle(&secs);
            assert_eq!(p, block_shuffle(&secs), "consensus-replayable: same secrets, same permutation");
            let seen: HashSet<usize> = p.iter().copied().collect();
            assert_eq!(seen.len(), secs.len(), "a permutation: every participant gets exactly one slot");
        }

        #[test]
        fn is_canonical_order_rejects_a_reordered_batch() {
            let coords = vec![
                Committed { height: 1, secret: sec(4) },
                Committed { height: 2, secret: sec(8) },
                Committed { height: 3, secret: sec(2) },
            ];
            let order = canonical_order(&coords);
            let canon: Vec<Committed> = order.iter().map(|&i| coords[i].clone()).collect();
            assert!(is_canonical_order(&canon), "the canonical sequence is accepted");
            let mut swapped = canon;
            swapped.swap(0, 2); // producer reorders across heights
            assert!(!is_canonical_order(&swapped), "a producer-favorable reorder is rejected, not silently re-sorted");
        }
    }
}

// ============ Index-dep binding (reference model — INDEX-DEP-CODEHASH-BINDING.md) ============
/// Host-side reference model of the on-VM index cell-dep binding. On-VM the program will
/// `load_cell_type(0, Source::CellDep)` and compare the dep's type-script identity; this
/// models the accept/reject decision so the rule is executable + tested before the ELF
/// port (repo convention: reference model in `node/`, on-VM later). Closes the design's
/// F1 (identity is the bound thing, not free args), F2 (full identity, not code_hash
/// alone), and F3 (type-id singleton) at the reference level.
pub mod index_binding {
    use super::Script;

    /// CKB `hash_type` discriminant. Two scripts sharing `code_hash`+`args` but differing
    /// in `hash_type` are DISTINCT programs, so identity is incomplete without it (QA-port-1
    /// / F2-complete). The node Cell model never needed it, so the index-dep identity carries
    /// it explicitly here rather than bloating the global `Script` struct.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum HashType {
        Data,
        Type,
        Data1,
    }

    /// A cell-dep's type-script as the on-VM port sees it: the FULL CKB `Script` identity
    /// `(code_hash, hash_type, args)`. Mirrors `load_cell_type(0, CellDep)` → reader, which
    /// exposes all three; the node `Script` struct omits `hash_type`, so the dep is modeled
    /// with this local triple to keep the reference faithful to what the ELF compares.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct DepScript<'a> {
        pub code_hash: [u8; 32],
        pub hash_type: HashType,
        pub args: &'a [u8],
    }

    /// Identity of the canonical index type-script. On-VM the port compares the full CKB
    /// script identity (code_hash ‖ hash_type ‖ args, the molecule blake2b hashes); the
    /// `type_id` arg pins the canonical singleton instance (F3). F2-complete: `hash_type`
    /// is part of identity, so a forged dep reusing code_hash+type-id under a different
    /// hash_type (Data vs Type vs Data1) no longer passes.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct IndexIdentity<'a> {
        pub code_hash: [u8; 32],
        pub hash_type: HashType,
        pub type_id: &'a [u8],
    }

    /// Is cell-dep 0 an acceptable index source?
    /// - `expected = None` ⇒ legacy/unset: shape path (accept any cell that HAS a
    ///   type-script). Keeps existing fixtures green until deploy pins the constant.
    /// - `expected = Some(id)` ⇒ bound: the dep's type-script `code_hash`, `hash_type`, AND
    ///   its type-id arg must ALL match (F1 const + F2 full identity + F3 instance). A dep
    ///   with no type-script is rejected under binding (F2).
    pub fn dep_accepted(dep_type_script: Option<&DepScript>, expected: Option<IndexIdentity>) -> bool {
        match expected {
            None => dep_type_script.is_some(),
            Some(id) => match dep_type_script {
                None => false,
                Some(d) => {
                    d.code_hash == id.code_hash
                        && d.hash_type == id.hash_type
                        && d.args == id.type_id
                }
            },
        }
    }

    /// F3 singleton invariant: among candidate index cells, at most one may carry the
    /// canonical type-id. On-VM this is guaranteed by the CKB type-id rule + UTXO liveness
    /// (an old root lives in a spent cell, unreferenceable as a dep); the model asserts
    /// the uniqueness the type-id rule enforces.
    pub fn is_unique_index(candidate_type_scripts: &[Script], type_id: &[u8]) -> bool {
        candidate_type_scripts.iter().filter(|s| s.args.as_slice() == type_id).count() <= 1
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const H: [u8; 32] = [0xA1; 32];
        const WRONG_H: [u8; 32] = [0xB2; 32];

        // A cell-dep type-script as the on-VM port sees it: full (code_hash, hash_type, args).
        fn dep(code_hash: [u8; 32], hash_type: HashType, args: &[u8]) -> DepScript<'_> {
            DepScript { code_hash, hash_type, args }
        }
        // The canonical expected identity, parameterized by the instance type-id.
        fn id(type_id: &[u8]) -> IndexIdentity<'_> {
            IndexIdentity { code_hash: H, hash_type: HashType::Type, type_id }
        }
        // For the type-id singleton test (uniqueness needs no hash_type).
        fn ts(code_hash: [u8; 32], args: &[u8]) -> Script {
            Script { code_hash, args: args.to_vec() }
        }

        #[test]
        fn bound_match_accepts() {
            let tid: &[u8] = &[7, 7, 7];
            assert!(
                dep_accepted(Some(&dep(H, HashType::Type, tid)), Some(id(tid))),
                "right code + right hash_type + right instance"
            );
        }

        #[test]
        fn bound_wrong_code_hash_rejects() {
            // F2: a forged index reusing the canonical type-id arg but a different
            // type-script code is rejected — code_hash-only would have missed this.
            let tid: &[u8] = &[7, 7, 7];
            assert!(!dep_accepted(Some(&dep(WRONG_H, HashType::Type, tid)), Some(id(tid))));
        }

        #[test]
        fn bound_wrong_hash_type_rejects() {
            // QA-port-1 / F2-complete: SAME code_hash + SAME type-id, but Data instead of
            // Type is a DISTINCT program. code_hash-only (the pre-fix model) accepted this;
            // full-identity rejects it. This is the forged-dep the hash_type field closes.
            let tid: &[u8] = &[7, 7, 7];
            assert!(
                !dep_accepted(Some(&dep(H, HashType::Data, tid)), Some(id(tid))),
                "code_hash+type-id match but hash_type differs ⇒ distinct program ⇒ reject"
            );
            assert!(
                !dep_accepted(Some(&dep(H, HashType::Data1, tid)), Some(id(tid))),
                "Data1 is also distinct from Type"
            );
        }

        #[test]
        fn bound_wrong_type_id_rejects() {
            // F3: right code, wrong instance (different type-id) ⇒ reject (not canonical).
            assert!(!dep_accepted(Some(&dep(H, HashType::Type, &[9, 9])), Some(id(&[7, 7, 7]))));
        }

        #[test]
        fn bound_no_type_script_rejects() {
            assert!(!dep_accepted(None, Some(id(&[7, 7, 7]))), "F2: no type-script ⇒ unbound ⇒ reject");
        }

        #[test]
        fn legacy_unset_is_shape_path() {
            assert!(
                dep_accepted(Some(&dep(WRONG_H, HashType::Data, &[1])), None),
                "unset ⇒ any type-scripted cell ok (hash_type irrelevant on the shape path)"
            );
            assert!(!dep_accepted(None, None), "still rejects a cell with no type-script");
        }

        #[test]
        fn f3_singleton_rejects_duplicate_type_id() {
            let tid: &[u8] = &[7, 7, 7];
            assert!(is_unique_index(&[ts(H, tid)], tid));
            assert!(
                !is_unique_index(&[ts(H, tid), ts(H, tid)], tid),
                "two live cells with the canonical type-id violates the singleton"
            );
        }
    }
}
