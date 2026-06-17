//! Node runtime — the replicated state machine that turns the mechanism library into
//! something two participants can RUN and CONVERGE on. The library proves each rule in
//! isolation (235 unit tests); this module wires them into a node a peer can replicate.
//!
//! LEAN BY DESIGN (Bitcoin-simplicity, CONTINUE.md constraint): this is ORCHESTRATION
//! ONLY. It re-implements no mechanism. It composes four existing pieces —
//!   1. consensus-sourced ordering  (`commit_order::canonical_order`)
//!   2. the attribution gate        (`pom_scores` / temporal-novelty; the value gate is
//!                                    the v2 swap-in, see ROADMAP)
//!   3. the novelty index           (`smt::NoveltyIndex`)
//!   4. PoM-weighted finalization   (`consensus::finalizes_hybrid`)
//! into a deterministic block loop. Two honest nodes that finalize the same blocks hold
//! byte-identical ledgers — that is the convergence property the 2-node harness asserts.
//!
//! The block shape (a commit-reveal batch ordered by a consensus-sourced shuffle) is
//! derived from VibeSwap's CommitRevealAuction; the per-contributor PoM attribution it
//! settles is the same value/flow surface PsiNet's CRPC scores pairwise. The matrix that
//! governs HOW value is measured is carried as a genesis [`Constitution`]: physics-anchored
//! meta-rules near-immutable, the dimension SET verifier-gated-extensible, the WEIGHTS
//! governance-bounded (ROADMAP 2026-06-16 (c) — the completeness/weights cleavage).

use crate::commit_order::{canonical_order, is_canonical_order, Committed};
use crate::consensus::{self, Mix, Validator, NCI, TWO_THIRDS_BPS};
use crate::smt::{Hash, NoveltyIndex};
use crate::{coverage, pom_scores, tokens, Cell};
use std::collections::HashMap;

// ============ Constitution — how value is measured (the amendment frame) ============

/// Genesis-fixed parameters governing the measurement and its finalization. The
/// value-dimension matrix is governed in THREE layers (ROADMAP 2026-06-16 (c)):
///   - **physics** (near-immutable): value anchors in realized downstream flow; the
///     noise floor holds. Encoded by the gate the runtime calls, not by a tunable here.
///   - **constitutional**: the amendment rules — a dimension is admitted only if it
///     predicts realized downstream value (verifier-gated), weights stay bounded.
///   - **governance**: weights within the bounded set, fluid.
/// v1 carries only the finalization frame; the dimension-matrix amendment rules are the
/// next build (`pending`: a constitutional cell whose transitions obey the verifier gate).
#[derive(Clone, Copy, Debug)]
pub struct Constitution {
    /// PoM-weighted finalization mix (NCI as-built: 10/30/60 PoW/PoS/PoM).
    pub mix: Mix,
    /// finalization supermajority, BPS.
    pub threshold_bps: u64,
    /// minimum live weight that must participate, as BPS of total base weight.
    pub quorum_floor_bps: u64,
    /// liveness horizon for franchise decay (0 = no decay).
    pub horizon: u64,
    /// NCI as-built decays PoW+PoM only; `true` decays PoS too (the symmetric fix).
    pub decay_pos: bool,
}

impl Default for Constitution {
    fn default() -> Self {
        Constitution {
            mix: NCI,
            threshold_bps: TWO_THIRDS_BPS,
            quorum_floor_bps: 0,
            horizon: 0,
            decay_pos: false,
        }
    }
}

// ============ Ledger — the replicated state ============

/// The replicated state: finalized cells in canonical commit order, the novelty index
/// over their coverage, the PoM standing each contributor has earned, and the height.
/// Two honest nodes applying the same finalized blocks hold equal [`Ledger::state_digest`].
pub struct Ledger {
    /// finalized cells, globally canonical (height ascending, then in-block shuffle slot).
    pub cells: Vec<Cell>,
    /// sparse-Merkle novelty index over inserted coverage shingles.
    pub index: NoveltyIndex,
    /// attribution output: PoM per soulbound contributor (`type_script.args`).
    pub pom: HashMap<Vec<u8>, u64>,
    /// height of the last finalized block.
    pub height: u64,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            cells: Vec::new(),
            index: NoveltyIndex::new(),
            pom: HashMap::new(),
            height: 0,
        }
    }

    /// Compact, comparable digest of replica state for convergence checks: the finalized
    /// cell-id sequence, the novelty-index root, and the sorted PoM attribution map.
    pub fn state_digest(&self) -> (Vec<u64>, Hash, Vec<(Vec<u8>, u64)>) {
        let ids: Vec<u64> = self.cells.iter().map(|c| c.id).collect();
        let mut pom: Vec<(Vec<u8>, u64)> = self.pom.iter().map(|(k, v)| (k.clone(), *v)).collect();
        pom.sort();
        (ids, self.index.root(), pom)
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Token transactions — value movement gated at the block level ============

/// Which ERC analog a [`TokenTx`] settles under (its `type_script.code_hash` standard).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenStandard {
    /// ERC-20 / sUDT — fungible supply conservation, issuer-only mint.
    Fungible,
    /// ERC-721 — id-set preservation, issuer-only new ids, no duplicates.
    Nft,
    /// ERC-1155 — per-sub-id independent conservation.
    Multi,
}

/// A token value-movement carried inside a block: consume `inputs`, produce `outputs`,
/// under one token's standard and identity (`args` = the issuer). Validity is a PURE function
/// of the tx (no oracle; the airgap is closed structurally — see the `tokens` module), and the
/// runtime calls it at the block gate so a non-conserving movement can never finalize.
///
/// NOTE the mint authority is NOT a field. An earlier shape carried a `minter: Vec<u8>`, but a
/// producer-asserted minter is self-assertion, not a check: an attacker mints any token by naming
/// itself the issuer. The minter is therefore DERIVED from the consumed inputs (see [`is_valid`]),
/// the 8th site of `[P·dont-let-attacker-choose-critical-input]`. Validation only in v1; spending
/// the inputs / persisting the outputs into a token ledger is the deploy-coupled full-tx pipeline.
#[derive(Clone)]
pub struct TokenTx {
    pub standard: TokenStandard,
    pub code_hash: [u8; 32],
    pub args: Vec<u8>,
    pub inputs: Vec<Cell>,
    pub outputs: Vec<Cell>,
}

impl TokenTx {
    /// Does this movement satisfy its standard's conservation (and mint-authority) rule?
    /// Single-sourced from the `tokens` reference analogs — the same functions the on-VM
    /// type-script port mirrors, so the block gate and the chain agree by construction.
    ///
    /// The mint authority is DERIVED, never self-declared: a mint is authorized iff the issuer
    /// SPENDS an authority cell it controls — an input of this token (same type-script identity)
    /// whose current owner (`lock.args`) is the issuer (`args`). An attacker that names itself the
    /// issuer but controls no such input gets a minter that cannot match, so any supply increase /
    /// new id is rejected; conserving transfers and burns are unaffected. Pre-deploy `lock.args`
    /// stands in for the verified owner — binding it to a checked signature is the lock-sig layer.
    pub fn is_valid(&self) -> bool {
        // a token with no issuer identity is not well-formed (also makes the non-issuer minter
        // sentinel below sound: a non-empty issuer can never equal the empty slice).
        if self.args.is_empty() {
            return false;
        }
        let issuer_controls_authority_input = self.inputs.iter().any(|c| {
            c.type_script.code_hash == self.code_hash
                && c.type_script.args == self.args
                && c.lock.args == self.args
        });
        let minter: &[u8] = if issuer_controls_authority_input { &self.args } else { &[] };
        match self.standard {
            TokenStandard::Fungible => tokens::fungible::mint_or_conserve(
                &self.inputs,
                &self.outputs,
                &self.code_hash,
                &self.args,
                minter,
            ),
            TokenStandard::Nft => tokens::nft::mint_or_conserve(
                &self.inputs,
                &self.outputs,
                &self.code_hash,
                &self.args,
                minter,
            ),
            // the starter multi analog has no issuer-mint path ⇒ pure conservation only.
            TokenStandard::Multi => {
                tokens::multi::conserves(&self.inputs, &self.outputs, &self.code_hash, &self.args)
            }
        }
    }
}

// ============ Block — a commit-reveal batch in canonical order ============

/// A proposed block: cells paired with their consensus-sourced ordering coordinates,
/// already in canonical commit order. Assembly is presentation-independent — no producer
/// can bias which cell lands in which slot (the temporal-order strategyproofness guarantee).
#[derive(Clone)]
pub struct Block {
    pub height: u64,
    pub cells: Vec<Cell>,
    pub coords: Vec<Committed>,
    /// token value-movements settled by this block; each must conserve (see [`Node::validate`]).
    /// Empty for a pure attribution block — existing blocks are unaffected by the gate.
    pub token_txs: Vec<TokenTx>,
}

impl Block {
    /// Leader assembly: order the proposals canonically (consensus shuffle, not producer
    /// presentation) and pair each cell with its coordinate in that order.
    pub fn assemble(height: u64, proposals: &[(Cell, Committed)]) -> Block {
        let raw: Vec<Committed> = proposals.iter().map(|(_, c)| c.clone()).collect();
        let order = canonical_order(&raw);
        let cells = order.iter().map(|&i| proposals[i].0.clone()).collect();
        let coords = order.iter().map(|&i| raw[i].clone()).collect();
        Block { height, cells, coords, token_txs: Vec::new() }
    }

    /// Attach token movements to a proposed block (builder; empty by default). Each tx is
    /// gated by [`Node::validate`] — one non-conserving movement makes the whole block invalid.
    pub fn with_token_txs(mut self, txs: Vec<TokenTx>) -> Block {
        self.token_txs = txs;
        self
    }
}

// ============ Node — a single replica ============

/// One node: its validator identity is implicit in `validators`; it holds a [`Ledger`]
/// replica, the agreed validator set, the [`Constitution`], and a local mempool of
/// pre-consensus proposals.
pub struct Node {
    pub id: u64,
    pub ledger: Ledger,
    pub validators: Vec<Validator>,
    pub constitution: Constitution,
    pub mempool: Vec<(Cell, Committed)>,
}

impl Node {
    pub fn new(id: u64, validators: Vec<Validator>, constitution: Constitution) -> Self {
        Node {
            id,
            ledger: Ledger::new(),
            validators,
            constitution,
            mempool: Vec::new(),
        }
    }

    /// Gossip a proposal into the local mempool (pre-consensus).
    pub fn submit(&mut self, cell: Cell, coord: Committed) {
        self.mempool.push((cell, coord));
    }

    /// Leader move: assemble the next block from the local mempool.
    pub fn propose(&self) -> Block {
        Block::assemble(self.ledger.height + 1, &self.mempool)
    }

    /// Honest-verifier at the block level: a node votes YES iff the block passes every
    /// check it can verify against its OWN replica. (1) it extends our chain by one,
    /// (2) cells and coords align, (3) the coords are in canonical commit order (a
    /// producer-favorable reorder is rejected before any state math — no probe signal),
    /// (4) every coordinate's height matches the block (single-block batch in v1),
    /// (5) every token movement conserves under its standard (an unauthorized mint /
    /// non-conserving transfer makes the whole block invalid — value cannot be forged into
    /// a finalized block).
    pub fn validate(&self, b: &Block) -> bool {
        b.height == self.ledger.height + 1
            && b.cells.len() == b.coords.len()
            && !b.cells.is_empty()
            && is_canonical_order(&b.coords)
            && b.coords.iter().all(|c| c.height == b.height)
            && b.token_txs.iter().all(TokenTx::is_valid)
    }

    /// A node's vote on a proposal is its honest local validation.
    pub fn vote(&self, b: &Block) -> bool {
        self.validate(b)
    }

    /// Apply a finalized block. DETERMINISTIC state transition: append the canonical-ordered
    /// cells, insert their coverage into the novelty index (idempotent — mirrors the on-chain
    /// index rule), advance the height, and recompute PoM attribution over the full chain.
    /// Identical inputs ⇒ identical (cells, index.root(), pom) ⇒ replicas converge.
    pub fn apply(&mut self, b: &Block) {
        for cell in &b.cells {
            for key in coverage(&cell.data) {
                self.ledger.index.insert(key);
            }
            self.ledger.cells.push(cell.clone());
        }
        self.ledger.height = b.height;
        self.ledger.pom = pom_scores(&self.ledger.cells);
    }

    /// Drop mempool entries (called after a block finalizes; v1 clears the whole pool since
    /// the round's proposals are all included).
    pub fn clear_mempool(&mut self) {
        self.mempool.clear();
    }
}

// ============ Finalization decision ============

/// Does a block finalize under the constitution, given the validators that voted for it?
/// Thin wrapper over `consensus::finalizes_hybrid` so the runtime reads the rule from the
/// constitution rather than threading eight parameters at every call site.
pub fn finalizes(c: &Constitution, voters_for: &[Validator], all: &[Validator], now: u64) -> bool {
    consensus::finalizes_hybrid(
        voters_for,
        all,
        c.mix,
        now,
        c.horizon,
        c.decay_pos,
        c.threshold_bps,
        c.quorum_floor_bps,
    )
}

// ============ PoS+PoM finality gadget (T3 — PoW out of finality) ============
//
// Research T3 (RESEARCH-NETWORK-CONSENSUS.md) found a latent bug: the face-value 3-way sum counts
// PoW weight as final the instant it is mined, but PoW finality is probabilistic/reorgeable — so
// PoW lag becomes a finality-SAFETY vector. The production pattern (Casper-FFG / GRANDPA / Babylon /
// Decred) keeps the probabilistic layer OUT of the immediate finality weight. This module is the
// runtime-level fix; the 235-test core `consensus::finalizes_hybrid` rule is left intact and reused.
pub mod finality {
    use crate::consensus::{self, Mix, Validator};

    /// Finality mix — PoW REMOVED (it secures production / ordering / sybil-cost, never finality).
    /// PoS:PoM = 30:60 renormalized so the set sums to 1, so the 2/3 bar is 2/3 OF THE FAST-FINAL
    /// SET (PoS+PoM), not of a mixed-confidence global total.
    pub const FINALITY_MIX: Mix = Mix { pow: 0.0, pos: 1.0 / 3.0, pom: 2.0 / 3.0 };

    /// Anti-concentration floor (T3 + T11): each fast-final DIMENSION must independently supply at
    /// least this fraction (BPS) of its OWN dimension total for a checkpoint to finalize. This forces
    /// BOTH the objective capital axis (PoS) AND the subjective value axis (PoM) to participate, so
    /// PoM's 60% cannot unilaterally finalize — capital-orthogonality (T11) enforced in code. A
    /// CONSTITUTIONAL constant (physics/constitutional layer of the value-matrix governance, not
    /// governance-tunable).
    pub const MIN_DIM_BPS: u64 = 5000;

    fn dim_ok(weight_for: f64, weight_all: f64) -> bool {
        // a dimension absent from the whole set can't gate (avoids div-by-zero); otherwise the
        // voting weight in that dimension must clear MIN_DIM_BPS of the dimension's total.
        weight_all <= 0.0 || weight_for >= weight_all * MIN_DIM_BPS as f64 / 10_000.0
    }

    /// Finalize a checkpoint on PoS+PoM only, with the anti-concentration rule. PoW is excluded by
    /// the mix; `now`/`horizon`/`decay_pos`/`threshold_bps` carry the usual liveness/threshold
    /// semantics. Returns true iff (1) PoS+PoM voting weight ≥ 2/3 of the fast-final set AND (2)
    /// each of PoS and PoM independently clears the anti-concentration floor.
    pub fn finalizes_pos_pom(
        voters_for: &[Validator],
        all: &[Validator],
        now: u64,
        horizon: u64,
        decay_pos: bool,
        threshold_bps: u64,
    ) -> bool {
        if !consensus::finalizes_hybrid(
            voters_for,
            all,
            FINALITY_MIX,
            now,
            horizon,
            decay_pos,
            threshold_bps,
            0,
        ) {
            return false;
        }
        let pos_for: f64 = voters_for.iter().map(|v| v.pos).sum();
        let pos_all: f64 = all.iter().map(|v| v.pos).sum();
        let pom_for: f64 = voters_for.iter().map(|v| v.pom).sum();
        let pom_all: f64 = all.iter().map(|v| v.pom).sum();
        dim_ok(pos_for, pos_all) && dim_ok(pom_for, pom_all)
    }
}

#[cfg(test)]
mod tests {
    use super::finality::{finalizes_pos_pom, MIN_DIM_BPS, FINALITY_MIX};
    use super::{Block, Constitution, Node, TokenStandard, TokenTx};
    use crate::commit_order::Committed;
    use crate::consensus::{Validator, TWO_THIRDS_BPS};
    use crate::tokens::fungible;
    use crate::{Cell, Script};

    fn v(id: u64, pow: f64, pos: f64, pom: f64) -> Validator {
        Validator { id, pow, pos, pom, last_heartbeat: 0, staked_balance: 0.0 }
    }

    // ---- block-level token-conservation gate (gap #4) ----

    fn ft_cell(issuer: &[u8], owner: &[u8], amt: u128) -> Cell {
        Cell {
            id: 0,
            lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
            type_script: Script { code_hash: [20u8; 32], args: issuer.to_vec() },
            parent: None,
            timestamp: 0,
            data: fungible::encode(amt),
        }
    }

    /// A fresh node and a structurally-valid one-cell carrier block at height 1, so the only
    /// thing a token tx can flip in `validate` is the conservation gate itself.
    fn node_and_carrier_block() -> (Node, Block) {
        let node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        let c = Cell {
            id: 1,
            lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
            type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
            parent: None,
            timestamp: 1,
            data: b"the quick brown fox jumps over".to_vec(),
        };
        let block = Block::assemble(1, &[(c, Committed { height: 1, secret: [11u8; 32] })]);
        (node, block)
    }

    #[test]
    fn block_with_unauthorized_mint_is_rejected() {
        let (node, block) = node_and_carrier_block();
        // carrier block alone (no token movement) is valid.
        assert!(node.validate(&block));
        // alice (NOT the issuer "USD") tries to inflate supply 10 -> 11 from a cell she owns.
        let inflate = TokenTx {
            standard: TokenStandard::Fungible,
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"alice", 10)],
            outputs: vec![ft_cell(b"USD", b"alice", 11)],
        };
        assert!(!inflate.is_valid(), "non-issuer inflation must be invalid");
        let bad = block.with_token_txs(vec![inflate]);
        assert!(
            !node.validate(&bad),
            "honest node finalized a block carrying a non-conserving token tx"
        );
    }

    #[test]
    fn block_with_conserving_transfer_validates() {
        let (node, block) = node_and_carrier_block();
        // 10 -> 7 + 3: a pure split, conserves supply (no mint authority needed).
        let split = TokenTx {
            standard: TokenStandard::Fungible,
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"alice", 10)],
            outputs: vec![ft_cell(b"USD", b"alice", 7), ft_cell(b"USD", b"bob", 3)],
        };
        assert!(split.is_valid());
        assert!(node.validate(&block.with_token_txs(vec![split])));
    }

    // ---- mint authority is DERIVED, not self-declared (8th attacker-input site) ----

    #[test]
    fn mint_authority_cannot_be_self_declared() {
        let (node, block) = node_and_carrier_block();
        let code = [20u8; 32];
        // mallory mints 1000 USD from nothing, controlling no USD authority cell.
        let forge = TokenTx {
            standard: TokenStandard::Fungible,
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![],
            outputs: vec![ft_cell(b"USD", b"mallory", 1000)],
        };
        // vector premise: the raw primitive WOULD authorize the mint if handed a self-declared
        // minter equal to the issuer — i.e. the danger is real if the runtime trusted a free field.
        assert!(
            fungible::mint_or_conserve(&forge.inputs, &forge.outputs, &code, b"USD", b"USD"),
            "premise: the primitive trusts whatever minter it is handed"
        );
        // but the runtime DERIVES the minter from issuer-controlled inputs, so the forge fails:
        // there is no self-declared minter channel to set.
        assert!(
            !forge.is_valid(),
            "self-declared mint authority accepted — 8th attacker-input site open"
        );
        assert!(!node.validate(&block.with_token_txs(vec![forge])));
    }

    #[test]
    fn issuer_mints_by_spending_its_authority_cell() {
        let (node, block) = node_and_carrier_block();
        let code = [20u8; 32];
        // the issuer USD spends an authority cell it controls (a USD-token cell it OWNS:
        // type_script.args == lock.args == "USD"), and mints 1000 to alice.
        let mint = TokenTx {
            standard: TokenStandard::Fungible,
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"USD", 0)],
            outputs: vec![ft_cell(b"USD", b"alice", 1000)],
        };
        assert!(mint.is_valid(), "issuer spending its own authority cell cannot mint");
        assert!(node.validate(&block.with_token_txs(vec![mint])));
    }

    #[test]
    fn pos_pom_both_dims_finalize() {
        // two validators each carrying capital (pos) AND contribution (pom); both vote.
        let all = vec![v(0, 0.0, 100.0, 100.0), v(1, 0.0, 100.0, 100.0)];
        assert!(finalizes_pos_pom(&all, &all, 1, 0, false, TWO_THIRDS_BPS));
    }

    #[test]
    fn pom_alone_cannot_finalize_anti_concentration() {
        // a PoM "whale" (no capital) + a capital holder (no PoM). The whale alone clears 2/3 of the
        // PoS+PoM SET, but contributes ZERO to the capital axis ⇒ anti-concentration must reject it.
        let whale = v(0, 0.0, 0.0, 200.0);
        let capital = v(1, 0.0, 100.0, 0.0);
        let all = vec![whale.clone(), capital.clone()];
        let pom_only = vec![whale];
        assert!(
            !finalizes_pos_pom(&pom_only, &all, 1, 0, false, TWO_THIRDS_BPS),
            "PoM unilaterally finalized — capital-orthogonality not enforced"
        );
        // but both axes participating DOES finalize.
        assert!(finalizes_pos_pom(&all, &all, 1, 0, false, TWO_THIRDS_BPS));
    }

    #[test]
    fn pow_is_excluded_from_finality() {
        // a PoW giant with no stake/contribution must not help finalize — PoW is out of the gadget.
        let pow_giant = v(0, 1_000_000.0, 0.0, 0.0);
        let pos = v(1, 0.0, 100.0, 0.0);
        let pom = v(2, 0.0, 0.0, 100.0);
        let all = vec![pow_giant.clone(), pos.clone(), pom.clone()];
        // PoW giant alone: FINALITY_MIX zeroes pow ⇒ contributes nothing ⇒ cannot finalize.
        assert!(!finalizes_pos_pom(&[pow_giant], &all, 1, 0, false, TWO_THIRDS_BPS));
        // the two fast-final dimensions together finalize, PoW irrelevant.
        assert!(finalizes_pos_pom(&[pos, pom], &all, 1, 0, false, TWO_THIRDS_BPS));
        // the mix really does exclude PoW.
        assert_eq!(FINALITY_MIX.pow, 0.0);
        assert_eq!(MIN_DIM_BPS, 5000);
    }
}
