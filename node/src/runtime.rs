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
use crate::consensus::{Mix, Validator, NCI, TWO_THIRDS_BPS};
use crate::smt::{Hash, NoveltyIndex};
use crate::{coverage, pom_scores, tokens, Cell, Script};
use std::collections::{HashMap, HashSet};

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
    /// finalized TOKEN cells — the value UTXO set, kept SEPARATE from `cells` so value movement
    /// never touches the novelty index or PoM attribution (both fold over `cells` only, so they
    /// stay token-blind). Token-tx input existence resolves HERE; [`Node::apply`] retires consumed
    /// inputs from and appends produced outputs to this set, enabling multi-hop token flow while
    /// preserving cross-block single-use. Issuance authority cells are seeded into it.
    pub token_cells: Vec<Cell>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            cells: Vec::new(),
            index: NoveltyIndex::new(),
            pom: HashMap::new(),
            height: 0,
            token_cells: Vec::new(),
        }
    }

    /// Compact, comparable digest of replica state for convergence checks: the finalized
    /// cell-id sequence, the novelty-index root, the sorted PoM attribution map, and the
    /// finalized token-cell id sequence (deterministic apply order ⇒ replicas converge on
    /// token state too, not just attribution).
    pub fn state_digest(&self) -> (Vec<u64>, Hash, Vec<(Vec<u8>, u64)>, Vec<u64>) {
        let ids: Vec<u64> = self.cells.iter().map(|c| c.id).collect();
        let mut pom: Vec<(Vec<u8>, u64)> = self.pom.iter().map(|(k, v)| (k.clone(), *v)).collect();
        pom.sort();
        let token_ids: Vec<u64> = self.token_cells.iter().map(|c| c.id).collect();
        (ids, self.index.root(), pom, token_ids)
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
    /// Per-input authorization, positionally aligned with `inputs`: `auths[i]` is the spend proof for
    /// `inputs[i]`. At deploy each is a signature over [`Self::digest`] by that input's owner key
    /// (`lock.args`); pre-deploy it is the empty sentinel. Carried ON the tx (not a validate-time
    /// param) because the signature is committed content every validator must re-check identically.
    /// A SHORT/EMPTY `auths` treats the missing entries as the empty sentinel ⇒ inert (existing flows
    /// unchanged). See [`Self::spend_is_authorized`].
    pub auths: Vec<Vec<u8>>,
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
        let minter: &[u8] = if issuer_controls_authority_input {
            &self.args
        } else {
            &[]
        };
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

    /// Ledger-aware validity: the pure [`is_valid`](Self::is_valid) rule AND a check that every
    /// consumed input actually EXISTS as a live finalized cell. This closes the input-authenticity
    /// residual the (f)/(g) derived-minter fix named: `is_valid` derives mint authority from a
    /// consumed authority input, but on its own it cannot tell a real authority cell from one an
    /// attacker FABRICATED (right shape, `lock.args` == issuer) and dropped into `inputs`. Requiring
    /// each input to match a cell in the live set by identity (id + lock + type_script) means a
    /// fabricated cell — never finalized into the ledger — can never enter `inputs`, so the
    /// fabrication path to a mint is gone. The same existence requirement also stops a conserving
    /// transfer from spending phantom inputs (conjuring balance out of fake cells).
    ///
    /// HONEST SCOPE (reference layer): this proves EXISTENCE, not yet CONTROL or single-use. Binding
    /// the spend to a checked lock-signature (the owner actually authorized it) and removing the
    /// consumed cell so it cannot be spent twice are the deploy-coupled lock-sig + UTXO-retirement
    /// layers — the same "structure now, crypto-enforcement at deploy" boundary as the index-dep and
    /// header-`now` bindings. `lock` equality here stands in for the verified owner.
    /// AMOUNT BINDING (closes the value-forgery hole a critical-qa pass found): the input must
    /// match the finalized cell on `data` TOO, not just `(id, lock, type_script)`. `data` carries
    /// the fungible amount / NFT id, and `is_valid`'s conservation trusts the PRODUCER-supplied
    /// input amount. Without binding `data`, an attacker controlling ONE live cell of identity
    /// `(id, lock, type_script)` could present an input with that identity but an INFLATED amount
    /// and conserve the lie into a finalized block (spend 1000 while owning 6). Binding `data`
    /// forces every consumed input to equal a real finalized cell byte-for-byte, so the amount can
    /// no longer be forged. (Spending ANOTHER owner's real cell is the separate, still-open lock-sig
    /// gap above — orthogonal to amount forgery.)
    pub fn is_valid_in_ledger(&self, live: &[Cell]) -> bool {
        let input_is_live = |inp: &Cell| {
            live.iter().any(|c| {
                c.id == inp.id
                    && c.lock == inp.lock
                    && c.type_script == inp.type_script
                    && c.data == inp.data
            })
        };
        if !self.inputs.iter().all(input_is_live) || !self.is_valid() {
            return false;
        }
        // EXISTENCE→CONTROL: each consumed input must be authorized by its owner, not merely exist.
        // Empty `auth` is the pre-deploy inert path (authorized while CONTROL_BINDING_ACTIVE is off),
        // so honest empty-auth flows are unchanged; a PRESENTED `auth` is verified for real against the
        // input's `lock.args` (post-quantum Lamport). The message is the canonical [`Self::digest`].
        let tx_digest = self.digest();
        self.inputs.iter().enumerate().all(|(i, input)| {
            let auth = self.auths.get(i).map(Vec::as_slice).unwrap_or(&[]);
            Self::spend_is_authorized(input, auth, &tx_digest)
        })
    }

    /// Canonical, injective digest of this value-movement — the deterministic bytes a future
    /// lock-signature will cover (`internal/DESIGN-locksig-binding.md`). It is the deploy-independent
    /// prerequisite of the existence→control mile: it earns its place TODAY on two properties, before
    /// any signature exists.
    ///
    /// 1. REPLICA DETERMINISM — every honest node computes the same 32 bytes for the same logical
    ///    movement regardless of `inputs`/`outputs` array order. The signed content is the LOGICAL
    ///    movement, not its presentation; we canonicalize order so a re-presented tx is the same tx.
    /// 2. AUTHORIZATION SURFACE — it commits to exactly what [`Self::is_valid_in_ledger`] +
    ///    [`Self::is_valid`] check, so a signature over it authorizes precisely the movement the
    ///    validator will accept (no more, no less).
    ///
    /// CELL IDENTITY = `(id, lock, type_script, data)` — the SAME tuple `is_valid_in_ledger` keys
    /// existence on (lines above). `parent`/`timestamp` are DELIBERATELY excluded, not overlooked:
    /// the ledger treats two cells equal under that tuple as the same live cell, so the digest must
    /// not distinguish them either — committing to `parent`/`timestamp` would diverge the SIGNED
    /// identity from the VALIDATED identity (token cells carry `parent: None`/`timestamp: 0` anyway).
    ///
    /// INJECTIVITY — every variable-length field is length-prefixed via `put` so there is no
    /// field-boundary ambiguity (`args=[1],data=[2,3]` can never serialize like `args=[1,2],data=[3]`);
    /// fixed-32 hashes are emitted raw. The hasher's personalization is domain-separated from the smt
    /// node hasher so a tx digest can never collide with a novelty-index node hash.
    ///
    /// SINGLE SOURCE ((qq), debt paid): the serializer + tx-domain hasher live in `noesis_core::tx`
    /// (no_std, builds riscv) so the on-VM lock-script type-script recomputes the SAME digest — this
    /// builds borrowed `CellView`s over the tx's cells and delegates. Byte-identical to the prior
    /// in-line form (the digest/signing tests are the regression proof).
    pub(crate) fn digest(&self) -> [u8; 32] {
        fn view(c: &Cell) -> noesis_core::tx::CellView<'_> {
            noesis_core::tx::CellView {
                id: c.id,
                lock_code_hash: &c.lock.code_hash,
                lock_args: &c.lock.args,
                type_code_hash: &c.type_script.code_hash,
                type_args: &c.type_script.args,
                data: &c.data,
            }
        }
        let inputs: Vec<noesis_core::tx::CellView<'_>> = self.inputs.iter().map(view).collect();
        let outputs: Vec<noesis_core::tx::CellView<'_>> = self.outputs.iter().map(view).collect();
        noesis_core::tx::tx_digest(self.standard as u8, &self.code_hash, &self.args, &inputs, &outputs)
    }

    /// Does the presenter of `input` prove CONTROL of it (hold the spending key), as opposed to merely
    /// naming a real cell's identity? This is the existence→control closure: [`Self::is_valid_in_ledger`]
    /// proves a consumed input EXISTS; this proves the spender may move it. `tx_digest` is the canonical
    /// message a future owner-signature covers ([`Self::digest`]); `auth` is that signature, an opaque blob.
    ///
    /// The verifier is now LINKED — a post-quantum, hash-based Lamport one-time signature (see the
    /// [`lamport`] module; Will chose PQ 2026-06-22). A PRESENTED `auth` is verified FOR REAL against
    /// the finalized cell's `lock.args`, closing existence→control cryptographically: a real cell can
    /// be NAMED by anyone, but only the holder of the lock's one-time key can MOVE it.
    ///   - an ABSENT (empty) `auth` is the honest pre-deploy case; `CONTROL_BINDING_ACTIVE` gates
    ///     whether it is still tolerated (pre-deploy YES ⇒ existing empty-auth flows unchanged; at
    ///     deploy the flag flips and every spend must present a signature), and
    ///   - a PRESENTED (non-empty) `auth` is verified: accepted iff it is a valid Lamport signature
    ///     over `tx_digest` under the input's `lock.args` public-key root, rejected otherwise (a
    ///     garbage blob or a wrong-key/wrong-message signature is not an authorization).
    ///
    /// The owner public key is sourced from the FINALIZED cell's `lock.args`, never producer-asserted
    /// ([P·dont-let-attacker-choose-critical-input]). WIRED into [`Self::is_valid_in_ledger`]: called
    /// once per consumed input with that input's positional `auth`.
    pub(crate) fn spend_is_authorized(input: &Cell, auth: &[u8], tx_digest: &[u8; 32]) -> bool {
        // Explicit deploy flag — never an overloaded sentinel (the QA-port-2 lesson). Pre-deploy a
        // cell may carry no control proof; at deploy an empty auth no longer authorizes.
        const CONTROL_BINDING_ACTIVE: bool = false;
        if auth.is_empty() {
            return !CONTROL_BINDING_ACTIVE;
        }
        Self::verify_sig(&input.lock.args, tx_digest, auth)
    }

    /// Post-quantum lock-signature check: the owner key is a 32-byte Lamport public-key root carried
    /// as the finalized cell's `lock.args`. A `lock.args` that is not exactly a 32-byte root cannot
    /// authorize a spend (rejected, never panics on a malformed owner field).
    fn verify_sig(owner_pubkey: &[u8], tx_digest: &[u8; 32], auth: &[u8]) -> bool {
        match <&[u8; 32]>::try_from(owner_pubkey) {
            Ok(root) => lamport::verify(root, tx_digest, auth),
            Err(_) => false,
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
        Block {
            height,
            cells,
            coords,
            token_txs: Vec::new(),
        }
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
    /// (5) every token movement conserves under its standard and spends only live inputs (an
    /// unauthorized mint / non-conserving transfer / phantom input makes the whole block invalid
    /// — value cannot be forged into a finalized block), AND (6) no input is spent twice within
    /// the block (within-block double-spend; the cross-block half is enforced by [`apply`] retiring
    /// consumed inputs from the live set).
    pub fn validate(&self, b: &Block) -> bool {
        b.height == self.ledger.height + 1
            && b.cells.len() == b.coords.len()
            && !b.cells.is_empty()
            && is_canonical_order(&b.coords)
            && b.coords.iter().all(|c| c.height == b.height)
            && self.token_txs_conserve_and_single_use(b)
    }

    /// Checks (5)+(6) for a block's token movements: each tx is ledger-valid (conserves AND spends
    /// only live inputs — [`TokenTx::is_valid_in_ledger`]), AND no input identity is consumed more
    /// than once across the whole block. (6) closes the WITHIN-BLOCK double-spend the (h) existence
    /// fix left open: `is_valid_in_ledger` proves each input EXISTS, but two txs (or two inputs of
    /// one tx) could each consume the SAME live authority/value cell — minting or transferring off
    /// one cell twice. We fold a consumed-identity set across `token_txs` in canonical order and
    /// reject the first reuse. The identity is the SAME consensus-derived tuple the existence check
    /// keys on — `(id, lock, type_script)` — not a producer-asserted nullifier, so it stays in the
    /// `[P·dont-let-attacker-choose-critical-input]` class. The crypto nullifier / on-VM UTXO-set
    /// retirement is the deploy-coupled layer; this reference set models both block scopes.
    fn token_txs_conserve_and_single_use(&self, b: &Block) -> bool {
        let mut consumed: HashSet<(u64, Script, Script)> = HashSet::new();
        for tx in &b.token_txs {
            if !tx.is_valid_in_ledger(&self.ledger.token_cells) {
                return false;
            }
            for inp in &tx.inputs {
                let identity = (inp.id, inp.lock.clone(), inp.type_script.clone());
                // insert returns false iff this identity was already consumed in this block.
                if !consumed.insert(identity) {
                    return false;
                }
            }
        }
        true
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
        // TOKEN STATE TRANSITION (over the SEPARATE `token_cells` set, never `cells`):
        //   (a) CROSS-BLOCK single-use — retire each consumed token input, so a later block's
        //       `is_valid_in_ledger` existence check fails for an already-spent cell (the same real
        //       authority/value cell cannot be respent across blocks — UTXO retirement); then
        //   (b) PERSIST OUTPUTS — append each tx's produced outputs, so a recipient can spend them
        //       in a LATER block (multi-hop A→B→C flow). Without (b) every output was unspendable.
        // Identity is the consensus-derived tuple the existence check keys on (`id + lock +
        // type_script`), never producer-asserted. Retire BEFORE appending so a within-block reuse of
        // a just-produced output can't be conjured (validation already snapshots the pre-block set;
        // within-block chaining is intentionally out of scope in v1). `cells`/index/`pom` are left
        // untouched here ⇒ value movement does not perturb attribution. (The crypto nullifier set +
        // on-VM UTXO-set retirement are the deploy-coupled layer; this is the reference model.)
        for tx in &b.token_txs {
            for inp in &tx.inputs {
                self.ledger
                    .token_cells
                    .retain(|c| !(c.id == inp.id && c.lock == inp.lock && c.type_script == inp.type_script));
            }
        }
        for tx in &b.token_txs {
            for out in &tx.outputs {
                self.ledger.token_cells.push(out.clone());
            }
        }
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

// ============ Lock-sig verifier — post-quantum (hash-based Lamport) ============
//
// ROADMAP lock-sig DEPLOY half (verifier, (nn)) → on-VM port ((pp)). Will 2026-06-22 chose PQ. Lamport
// one-time signatures are the structurally-correct PQ choice: HASH-BASED (no lattice, no external
// crate beyond the in-tree blake2b; post-quantum hash-rooted), ONE-TIME-safe for FREE (a cell is
// consumed exactly once ⇒ its lock key signs exactly once; the UTXO/cell model IS Lamport's safety
// precondition, a SubstrateGeometryMatch), HASH-ROOTED key (a 32-byte blake2b root fits `lock.args`).
//
// SINGLE SOURCE (lean, same pattern as `finalization_fixed`): the verify arithmetic + keygen/sign live
// in `noesis-core::lamport` so the on-VM lock-script type-script and the node validate with ONE
// implementation. The node only ever VERIFIES; keygen/sign are key-holder (wallet) tooling exposed for
// tests. 🔬 the 16 KiB one-time signature is Lamport's known size tradeoff (Winternitz/SPHINCS+
// compression is the deferred optimization).
pub(crate) use noesis_core::lamport;

// ============ Finalization decision ============

/// Does a block finalize under the constitution, given the validators that voted for it?
///
/// T3 WIRING (ROADMAP decision #3, LOCKED 2026-06-20): the live finalization decision routes
/// through [`finality::finalizes_pos_pom`] — PoS+PoM only, PoW EXCLUDED from finality — not the
/// PoW-inclusive `c.mix`. PoW's finality is probabilistic/reorgeable, so counting freshly-mined
/// PoW weight as final is a safety vector (RESEARCH-NETWORK-CONSENSUS.md / Casper-FFG-class
/// pattern); PoW still secures production/ordering/sybil-cost via the constitution mix elsewhere.
/// The anti-concentration floor additionally forces BOTH the capital (PoS) and value (PoM) axes to
/// participate, so PoM's 60% cannot unilaterally finalize (T11 capital-orthogonality, in code).
///
/// `finalizes_pos_pom` REUSES `consensus::finalizes_hybrid` internally (with the PoW-free
/// `FINALITY_MIX`), so the 235-test core rule is intact; the `c.mix`/`c.quorum_floor_bps` fields
/// govern the production/ordering path, not this fast-final gate. Forward parity: when the on-VM
/// finalization mirror (🟡 `ON-VM-FINALIZATION.md`) is built, it must mirror THIS rule (PoW-out +
/// anti-concentration), not bare `finalizes_hybrid`.
pub fn finalizes(c: &Constitution, voters_for: &[Validator], all: &[Validator], now: u64) -> bool {
    finality::finalizes_pos_pom(voters_for, all, now, c.horizon, c.decay_pos, c.threshold_bps)
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
    pub const FINALITY_MIX: Mix = Mix {
        pow: 0.0,
        pos: 1.0 / 3.0,
        pom: 2.0 / 3.0,
    };

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
    use super::finality::{finalizes_pos_pom, FINALITY_MIX, MIN_DIM_BPS};
    use super::{Block, Constitution, Node, TokenStandard, TokenTx};
    use crate::commit_order::Committed;
    use crate::consensus::{Validator, TWO_THIRDS_BPS};
    use crate::tokens::fungible;
    use crate::{Cell, Script};

    fn v(id: u64, pow: f64, pos: f64, pom: f64) -> Validator {
        Validator {
            id,
            pow,
            pos,
            pom,
            last_heartbeat: 0,
            staked_balance: 0.0,
        }
    }

    // ---- block-level token-conservation gate (gap #4) ----

    fn ft_cell(issuer: &[u8], owner: &[u8], amt: u128) -> Cell {
        Cell {
            id: 0,
            lock: Script {
                code_hash: [0u8; 32],
                args: owner.to_vec(),
            },
            type_script: Script {
                code_hash: [20u8; 32],
                args: issuer.to_vec(),
            },
            parent: None,
            timestamp: 0,
            data: fungible::encode(amt),
        }
    }

    // ---- canonical tx-digest serializer (deploy-independent grain of the lock-sig mile) ----

    /// A token tx with UNSORTED inputs/outputs (ids 3,1 and 2,4) and distinct content, so the
    /// presentation-invariance and value-sensitivity properties are non-vacuous.
    fn sample_tx() -> TokenTx {
        let cell = |id: u64, owner: &[u8], data: Vec<u8>| Cell {
            id,
            lock: Script {
                code_hash: [0u8; 32],
                args: owner.to_vec(),
            },
            type_script: Script {
                code_hash: [20u8; 32],
                args: vec![7],
            },
            parent: None,
            timestamp: 0,
            data,
        };
        TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: vec![7],
            inputs: vec![cell(3, b"alice", vec![1, 2]), cell(1, b"bob", vec![3])],
            outputs: vec![cell(2, b"carol", vec![4]), cell(4, b"dave", vec![5, 6])],
        }
    }

    #[test]
    fn tx_digest_is_deterministic() {
        let tx = sample_tx();
        let d = tx.digest();
        for _ in 0..16 {
            assert_eq!(tx.digest(), d, "same tx must hash bit-identically every time");
        }
        assert_eq!(tx.clone().digest(), d, "a clone hashes identically");
    }

    #[test]
    fn tx_digest_is_invariant_to_input_output_presentation() {
        let tx = sample_tx();
        let mut shuffled = tx.clone();
        shuffled.inputs.reverse();
        shuffled.outputs.reverse();
        assert_eq!(
            tx.digest(),
            shuffled.digest(),
            "canonical sort => presentation order of inputs/outputs does not change the digest"
        );
    }

    #[test]
    fn tx_digest_changes_iff_value_changes() {
        let base = sample_tx();
        let d = base.digest();
        assert_eq!(base.clone().digest(), d, "a no-op clone must not change the digest");
        // each load-bearing field, flipped one at a time, must move the digest:
        let mut t = base.clone();
        t.inputs[0].data = vec![9, 9, 9];
        assert_ne!(t.digest(), d, "an input's data is signed");
        let mut t = base.clone();
        t.inputs[0].lock.args = vec![42];
        assert_ne!(t.digest(), d, "an input's owner (lock.args) is signed");
        let mut t = base.clone();
        t.outputs[0].data = vec![1];
        assert_ne!(t.digest(), d, "an output's data is signed");
        let mut t = base.clone();
        t.args = vec![100];
        assert_ne!(t.digest(), d, "the tx issuer (args) is signed");
        let mut t = base.clone();
        t.standard = TokenStandard::Nft;
        assert_ne!(t.digest(), d, "the token standard is signed");
        let mut t = base.clone();
        t.inputs[0].id = 999;
        assert_ne!(t.digest(), d, "an input's id is signed");
    }

    #[test]
    fn tx_digest_no_field_boundary_collision() {
        // type_script.args and data are ADJACENT in serialize_cell. Without length-prefixing,
        // (ts_args=[1], data=[2,3]) and (ts_args=[1,2], data=[3]) both serialize to the run
        // [1,2,3] and collide. Length prefixes make them distinct — this proves `put` is
        // load-bearing (break-on-purpose: stripping the prefix reds THIS test).
        let mk = |ts_args: Vec<u8>, data: Vec<u8>| TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [5u8; 32],
            args: vec![9],
            inputs: vec![],
            outputs: vec![Cell {
                id: 1,
                lock: Script {
                    code_hash: [0u8; 32],
                    args: vec![],
                },
                type_script: Script {
                    code_hash: [5u8; 32],
                    args: ts_args,
                },
                parent: None,
                timestamp: 0,
                data,
            }],
        };
        let a = mk(vec![1], vec![2, 3]);
        let b = mk(vec![1, 2], vec![3]);
        assert_ne!(
            a.digest(),
            b.digest(),
            "length-prefixing is load-bearing: differing field splits must not collide"
        );
    }

    #[test]
    fn spend_authorization_inert_pre_deploy_but_rejects_presented_unverifiable_auth() {
        // existence->control SHAPE (DESIGN-locksig-binding.md step 2), sentinel-gated inert.
        let tx = sample_tx();
        let d = tx.digest();
        let input = &tx.inputs[0];
        // absent (sentinel) auth = the honest pre-deploy case => authorized (inert; flows unchanged).
        assert!(
            TokenTx::spend_is_authorized(input, &[], &d),
            "an absent (sentinel) auth is inert-authorized pre-deploy"
        );
        // a PRESENTED garbage auth fails REAL verification (a 3-byte blob is not a valid Lamport sig).
        assert!(
            !TokenTx::spend_is_authorized(input, &[1, 2, 3], &d),
            "a malformed presented signature is not an authorization"
        );
    }

    #[test]
    fn existence_binds_amount_no_value_forgery_from_an_inflated_input() {
        // CLOSES a value-forgery hole a critical-qa pass found: `is_valid_in_ledger` once keyed
        // existence on `(id, lock, type_script)` only, NOT `data`. `data` carries the fungible
        // amount, and `is_valid`'s conservation trusts the producer-supplied input amount — so an
        // attacker owning ONE small live cell could present an input with the same identity but an
        // INFLATED amount and conserve the lie into a finalized block.
        let (mut node, block) = node_and_carrier_block();
        // alice REALLY owns just 6 USD (a finalized live token cell).
        node.ledger.token_cells.push(ft_cell(b"USD", b"alice", 6));
        // she presents an input with the SAME identity (id 0, lock alice, type USD) but a
        // LIED-ABOUT amount of 1000, transferring 1000 to bob (1000->1000 conserves).
        let inflate = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"alice", 1000)],
            outputs: vec![ft_cell(b"USD", b"bob", 1000)],
        };
        assert!(
            !node.validate(&block.clone().with_token_txs(vec![inflate])),
            "VALUE FORGERY: spent 1000 while owning 6 — existence must bind the amount (data)"
        );
        // and the HONEST spend of the real 6-cell still validates — binding data rejects only the lie.
        let honest = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"alice", 6)],
            outputs: vec![ft_cell(b"USD", b"bob", 6)],
        };
        assert!(
            node.validate(&block.with_token_txs(vec![honest])),
            "binding data over-rejected: alice's real 6-cell spend must still validate"
        );
    }

    #[test]
    fn ledger_spend_path_consults_authorization_gate() {
        // The existence->control gate is WIRED into `is_valid_in_ledger` (DESIGN-locksig-binding.md
        // step 3), not merely unit-tested in isolation: an honest spend with an EMPTY (sentinel) auth
        // is inert-authorized and validates; the SAME spend carrying a PRESENTED, unverifiable auth is
        // rejected pre-deploy. This pins the call-site LIVE through the real block-validation path.
        let (mut node, block) = node_and_carrier_block();
        node.ledger.token_cells.push(ft_cell(b"USD", b"alice", 6));
        let mk = |auths: Vec<Vec<u8>>| TokenTx {
            standard: TokenStandard::Fungible,
            auths,
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"alice", 6)],
            outputs: vec![ft_cell(b"USD", b"bob", 6)],
        };
        // sentinel (empty auths) => inert-authorized => the honest spend validates (flows unchanged).
        assert!(
            node.validate(&block.clone().with_token_txs(vec![mk(vec![])])),
            "sentinel-auth spend must validate: the inert path leaves existing flows unchanged"
        );
        // a presented garbage auth on the real input => fails real verification THROUGH the wired path.
        assert!(
            !node.validate(&block.with_token_txs(vec![mk(vec![vec![9, 9, 9]])])),
            "WIRED gate: a malformed presented signature must be rejected by is_valid_in_ledger"
        );
    }

    #[test]
    fn lamport_pq_signature_roundtrips_and_rejects_forgery() {
        // The post-quantum (hash-based) one-time verifier in isolation.
        let seed = [7u8; 32];
        let root = super::lamport::keygen_root(&seed);
        let msg = [42u8; 32];
        let sig = super::lamport::sign(&seed, &msg);
        assert!(super::lamport::verify(&root, &msg, &sig), "honest one-time signature verifies");
        // WRONG MESSAGE — a signature over `msg` must not transfer to a different digest (no replay).
        let other_msg = [43u8; 32];
        assert!(!super::lamport::verify(&root, &other_msg, &sig), "sig does not transfer to another message");
        // WRONG KEY — a signature under one key must not verify under a different owner's root.
        let other_root = super::lamport::keygen_root(&[8u8; 32]);
        assert!(!super::lamport::verify(&other_root, &msg, &sig), "sig under one key fails under another");
        // TAMPERED — flipping any signature byte breaks it.
        let mut tampered = sig.clone();
        tampered[100] ^= 0xff;
        assert!(!super::lamport::verify(&root, &msg, &tampered), "a tampered signature is rejected");
        // MALFORMED LENGTH — a truncated blob is not a signature.
        assert!(!super::lamport::verify(&root, &msg, &sig[..sig.len() - 1]), "a wrong-length blob is rejected");
    }

    #[test]
    fn spend_path_authorizes_a_valid_pq_signature_and_rejects_a_wrong_key() {
        // existence→CONTROL closed CRYPTOGRAPHICALLY through the live block-validation path: a real
        // finalized cell can be NAMED by anyone (existence), but only the holder of the lock's one-time
        // key can MOVE it. Closes the (o) orthogonal residual — "spending another owner's real cell
        // still validates" — with a post-quantum signature.
        let seed = [3u8; 32];
        let root = super::lamport::keygen_root(&seed); // alice's PQ public key = her cell's lock.args
        let (mut node, block) = node_and_carrier_block();
        let alice_cell = ft_cell(b"USD", &root, 6);
        node.ledger.token_cells.push(alice_cell.clone());

        let unsigned = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![alice_cell.clone()],
            outputs: vec![ft_cell(b"USD", b"bob", 6)],
        };
        let digest = unsigned.digest(); // the bytes the owner signs (auth-independent — auths are not in the digest)
        let good = super::lamport::sign(&seed, &digest);
        let wrong_key = super::lamport::sign(&[9u8; 32], &digest); // a different key over the SAME movement

        assert!(
            node.validate(&block.clone().with_token_txs(vec![TokenTx { auths: vec![good], ..unsigned.clone() }])),
            "a valid PQ owner-signature authorizes the spend through is_valid_in_ledger"
        );
        assert!(
            !node.validate(&block.with_token_txs(vec![TokenTx { auths: vec![wrong_key], ..unsigned }])),
            "a signature under a DIFFERENT key cannot move alice's cell — existence != control"
        );
    }

    /// A fresh node and a structurally-valid one-cell carrier block at height 1, so the only
    /// thing a token tx can flip in `validate` is the conservation gate itself.
    fn node_and_carrier_block() -> (Node, Block) {
        let node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        let c = Cell {
            id: 1,
            lock: Script {
                code_hash: [0u8; 32],
                args: b"al".to_vec(),
            },
            type_script: Script {
                code_hash: [1u8; 32],
                args: b"alice".to_vec(),
            },
            parent: None,
            timestamp: 1,
            data: b"the quick brown fox jumps over".to_vec(),
        };
        let block = Block::assemble(
            1,
            &[(
                c,
                Committed {
                    height: 1,
                    secret: [11u8; 32],
                },
            )],
        );
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
            auths: vec![],
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
        let (mut node, block) = node_and_carrier_block();
        // the spent input must be a REAL live cell — a transfer cannot conserve from a phantom.
        node.ledger.token_cells.push(ft_cell(b"USD", b"alice", 10));
        // 10 -> 7 + 3: a pure split, conserves supply (no mint authority needed).
        let split = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
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
            auths: vec![],
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
        let (mut node, block) = node_and_carrier_block();
        let code = [20u8; 32];
        // the authority cell the issuer spends must be a REAL live cell it controls.
        node.ledger.token_cells.push(ft_cell(b"USD", b"USD", 0));
        // the issuer USD spends an authority cell it controls (a USD-token cell it OWNS:
        // type_script.args == lock.args == "USD"), and mints 1000 to alice.
        let mint = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"USD", 0)],
            outputs: vec![ft_cell(b"USD", b"alice", 1000)],
        };
        assert!(
            mint.is_valid(),
            "issuer spending its own authority cell cannot mint"
        );
        assert!(node.validate(&block.with_token_txs(vec![mint])));
    }

    #[test]
    fn fabricated_authority_cell_is_rejected_by_ledger_existence() {
        // CLOSES the residual the prior tick pinned. The (f)/(g) derived-minter fix relocated
        // mint-authority trust to the AUTHENTICITY of a consumed authority input, but the pure gate
        // alone cannot tell a real authority cell from one an attacker FABRICATED (right shape,
        // owner == issuer) and dropped into `inputs`. The ledger-existence check closes it: an input
        // that was never finalized into the live set can never enter `inputs`.
        let (mut node, block) = node_and_carrier_block();
        let code = [20u8; 32];
        // mallory fabricates a USD authority cell (owner == issuer == "USD") she does NOT control,
        // then mints 1000 to herself.
        let fabricated_authority = ft_cell(b"USD", b"USD", 0);
        let mint = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![fabricated_authority.clone()],
            outputs: vec![ft_cell(b"USD", b"mallory", 1000)],
        };
        // the PURE gate still accepts it — the residual lives exactly here, one layer down ...
        assert!(
            mint.is_valid(),
            "pure gate alone cannot distinguish a fabricated authority input from a real one"
        );
        // ... and the LEDGER-AWARE gate rejects it: no such cell was ever finalized.
        assert!(
            !mint.is_valid_in_ledger(&node.ledger.token_cells),
            "fabricated authority input passed the ledger-existence check"
        );
        assert!(
            !node.validate(&block.clone().with_token_txs(vec![mint.clone()])),
            "honest node finalized a mint backed by a fabricated authority cell"
        );
        // and the legitimate path is preserved: once the issuer REALLY controls a finalized
        // authority cell, the identical mint validates — existence was the whole difference.
        node.ledger.token_cells.push(fabricated_authority);
        assert!(node.validate(&block.with_token_txs(vec![mint])));
    }

    // ---- input single-use: no double-spend within OR across blocks (the (i) closure) ----
    // The (h) existence fix proved every consumed input EXISTS as a live finalized cell, but
    // `apply` only APPENDED cells and never RETIRED a consumed one, so a REAL authority cell
    // passed existence and could be spent repeatedly. These pin both scopes.

    #[test]
    fn same_authority_spent_twice_in_one_block_is_rejected() {
        let (mut node, block) = node_and_carrier_block();
        let code = [20u8; 32];
        // exactly ONE real authority cell exists.
        node.ledger.token_cells.push(ft_cell(b"USD", b"USD", 0));
        let mint_to = |to: &'static [u8]| TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"USD", 0)],
            outputs: vec![ft_cell(b"USD", to, 1000)],
        };
        let m1 = mint_to(b"alice");
        let m2 = mint_to(b"bob");
        // each tx ALONE is ledger-valid — the authority cell really exists ...
        assert!(m1.is_valid_in_ledger(&node.ledger.token_cells));
        assert!(m2.is_valid_in_ledger(&node.ledger.token_cells));
        // ... but spending that SAME authority identity twice in one block is a double-spend.
        assert!(
            !node.validate(&block.with_token_txs(vec![m1, m2])),
            "the same authority cell was spent twice in one block (within-block double-spend)"
        );
    }

    #[test]
    fn spend_then_respend_across_blocks_is_rejected() {
        let (mut node, block1) = node_and_carrier_block();
        let code = [20u8; 32];
        node.ledger.token_cells.push(ft_cell(b"USD", b"USD", 0));
        let mint = || TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"USD", 0)],
            outputs: vec![ft_cell(b"USD", b"alice", 1000)],
        };
        // block 1 spends the authority cell — valid — then is applied, retiring it.
        let b1 = block1.with_token_txs(vec![mint()]);
        assert!(
            node.validate(&b1),
            "first spend of a live authority cell must be valid"
        );
        node.apply(&b1);

        // a height-2 carrier block re-spending the SAME authority cell: it was retired by apply,
        // so the existence check now fails — no respend across blocks.
        let c2 = Cell {
            id: 2,
            lock: Script {
                code_hash: [0u8; 32],
                args: b"al".to_vec(),
            },
            type_script: Script {
                code_hash: [1u8; 32],
                args: b"alice".to_vec(),
            },
            parent: None,
            timestamp: 2,
            data: b"a distinct second-block attribution payload".to_vec(),
        };
        let carrier2 = Block::assemble(
            2,
            &[(
                c2,
                Committed {
                    height: 2,
                    secret: [22u8; 32],
                },
            )],
        );
        let b2 = carrier2.with_token_txs(vec![mint()]);
        assert!(
            !node.validate(&b2),
            "an already-spent authority cell was respent in a later block (cross-block double-spend)"
        );
    }

    #[test]
    fn distinct_inputs_each_spent_once_still_validate() {
        // single-use rejects REUSE of one identity, never DISTINCT inputs. A block carrying a USD
        // mint (spends the USD authority cell) AND a conserving EUR transfer (spends alice's EUR
        // cell) consumes two different identities — both must validate (no over-rejection).
        let (mut node, block) = node_and_carrier_block();
        let code = [20u8; 32];
        node.ledger.token_cells.push(ft_cell(b"USD", b"USD", 0));
        node.ledger.token_cells.push(ft_cell(b"EUR", b"alice", 10));
        let usd_mint = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"USD", 0)],
            outputs: vec![ft_cell(b"USD", b"alice", 1000)],
        };
        let eur_xfer = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"EUR".to_vec(),
            inputs: vec![ft_cell(b"EUR", b"alice", 10)],
            outputs: vec![ft_cell(b"EUR", b"alice", 4), ft_cell(b"EUR", b"bob", 6)],
        };
        assert!(usd_mint.is_valid() && eur_xfer.is_valid());
        assert!(
            node.validate(&block.with_token_txs(vec![usd_mint, eur_xfer])),
            "two txs spending DISTINCT inputs must both validate — single-use rejects reuse, not distinctness"
        );
    }

    #[test]
    fn existence_and_single_use_compose() {
        // the (h) existence gate and the (i) single-use gate are BOTH live in `validate` and
        // independent: neither masks the other.
        let (mut node, block) = node_and_carrier_block();
        let code = [20u8; 32];
        node.ledger.token_cells.push(ft_cell(b"USD", b"USD", 0)); // the ONE real authority cell
        let real_spend = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"USD", 0)],
            outputs: vec![ft_cell(b"USD", b"alice", 1000)],
        };
        // a fabricated EUR authority cell that was never finalized into the ledger.
        let fabricated = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: code,
            args: b"EUR".to_vec(),
            inputs: vec![ft_cell(b"EUR", b"EUR", 0)],
            outputs: vec![ft_cell(b"EUR", b"mallory", 1000)],
        };
        // real spend alongside a fabricated input ⇒ rejected by EXISTENCE (h).
        assert!(
            !node.validate(&block.clone().with_token_txs(vec![real_spend.clone(), fabricated])),
            "a fabricated input rode along beside a real spend (existence gate)"
        );
        // the SAME real authority spent twice ⇒ rejected by SINGLE-USE (i).
        assert!(
            !node.validate(&block.with_token_txs(vec![real_spend.clone(), real_spend])),
            "a real authority cell was double-spent (single-use gate)"
        );
    }

    // ---- token-state persistence: outputs become spendable, enabling multi-hop flow ----
    // Before this tick `apply` retired consumed inputs but NEVER persisted `tx.outputs`, so every
    // output was a dead end — a recipient could not spend what it received. Outputs now land in the
    // SEPARATE `token_cells` set; existence resolves there; `cells`/index/`pom` stay token-blind.

    /// A non-empty carrier block at a given height (needed because `validate` rejects an empty
    /// block); its single attribution cell is token-blind, so it never collides with token cells.
    fn carrier_at(height: u64, id: u64, secret: u8, payload: &[u8]) -> Block {
        let c = Cell {
            id,
            lock: Script {
                code_hash: [0u8; 32],
                args: b"al".to_vec(),
            },
            type_script: Script {
                code_hash: [1u8; 32],
                args: b"alice".to_vec(),
            },
            parent: None,
            timestamp: height,
            data: payload.to_vec(),
        };
        Block::assemble(
            height,
            &[(
                c,
                Committed {
                    height,
                    secret: [secret; 32],
                },
            )],
        )
    }

    #[test]
    fn multi_hop_token_flow_across_blocks() {
        // A→B→C: a value cell owned by alice moves to bob, then bob's RECEIVED cell moves to carol
        // in a later block. The second hop is only possible because `apply` persisted bob's output.
        let node = &mut Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        node.ledger.token_cells.push(ft_cell(b"USD", b"alice", 10)); // alice holds 10 USD
        let xfer = |from: &'static [u8], to: &'static [u8]| TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", from, 10)],
            outputs: vec![ft_cell(b"USD", to, 10)],
        };

        // hop 1: alice → bob.
        let b1 = carrier_at(1, 1, 11, b"first hop attribution payload")
            .with_token_txs(vec![xfer(b"alice", b"bob")]);
        assert!(node.validate(&b1), "alice's live cell must spend");
        node.apply(&b1);
        let owners: Vec<Vec<u8>> = node
            .ledger
            .token_cells
            .iter()
            .map(|c| c.lock.args.clone())
            .collect();
        assert_eq!(
            owners,
            vec![b"bob".to_vec()],
            "after hop 1 only bob holds the value (alice retired, bob persisted)"
        );

        // hop 2: bob → carol — spends the cell bob RECEIVED in block 1 (impossible before persistence).
        let b2 = carrier_at(2, 2, 22, b"second hop attribution payload")
            .with_token_txs(vec![xfer(b"bob", b"carol")]);
        assert!(
            node.validate(&b2),
            "bob must be able to spend the output it received in an earlier block"
        );
        node.apply(&b2);
        let owners: Vec<Vec<u8>> = node
            .ledger
            .token_cells
            .iter()
            .map(|c| c.lock.args.clone())
            .collect();
        assert_eq!(
            owners,
            vec![b"carol".to_vec()],
            "after hop 2 only carol holds the value — full A→B→C flow"
        );
    }

    #[test]
    fn output_is_unspendable_until_its_producing_block_is_applied() {
        // The exact bug the persistence change fixes: bob's output cannot be spent until the block
        // that produces it is applied. Rejected pre-apply (it isn't in `token_cells`), accepted after.
        let node = &mut Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        node.ledger.token_cells.push(ft_cell(b"USD", b"alice", 10));
        let alice_to_bob = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"alice", 10)],
            outputs: vec![ft_cell(b"USD", b"bob", 10)],
        };
        let bob_to_carol = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"bob", 10)],
            outputs: vec![ft_cell(b"USD", b"carol", 10)],
        };
        // bob's cell does not exist yet ⇒ spending it is rejected by the existence gate.
        let premature =
            carrier_at(1, 1, 11, b"premature spend payload").with_token_txs(vec![bob_to_carol.clone()]);
        assert!(
            !node.validate(&premature),
            "spent an output that was never persisted"
        );
        // apply the block that PRODUCES bob's cell ...
        let produce = carrier_at(1, 1, 11, b"produce bob payload").with_token_txs(vec![alice_to_bob]);
        assert!(node.validate(&produce));
        node.apply(&produce);
        // ... and now the identical bob→carol spend validates at the next height.
        let now_ok = carrier_at(2, 2, 22, b"now spendable payload").with_token_txs(vec![bob_to_carol]);
        assert!(
            node.validate(&now_ok),
            "persisted output still unspendable — persistence did not take"
        );
    }

    #[test]
    fn token_movement_leaves_attribution_unchanged() {
        // Token state is SEPARATE: moving value must not perturb the attribution fold (`cells`,
        // novelty index, `pom`). Same carrier cell with vs without a token tx ⇒ identical attribution
        // digest, different token digest. This is the whole reason `token_cells` is its own set.
        let carrier = carrier_at(1, 1, 11, b"identical attribution payload across both runs");

        // run A: carrier only, no token movement.
        let mut a = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        a.apply(&carrier);

        // run B: same carrier, but the block also carries a conserving transfer.
        let mut b = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        b.ledger.token_cells.push(ft_cell(b"USD", b"alice", 10));
        let xfer = TokenTx {
            standard: TokenStandard::Fungible,
            auths: vec![],
            code_hash: [20u8; 32],
            args: b"USD".to_vec(),
            inputs: vec![ft_cell(b"USD", b"alice", 10)],
            outputs: vec![ft_cell(b"USD", b"bob", 10)],
        };
        b.apply(&carrier.clone().with_token_txs(vec![xfer]));

        let (a_ids, a_root, a_pom, a_tok) = a.ledger.state_digest();
        let (b_ids, b_root, b_pom, b_tok) = b.ledger.state_digest();
        assert_eq!(
            (&a_ids, &a_root, &a_pom),
            (&b_ids, &b_root, &b_pom),
            "token movement perturbed the attribution fold"
        );
        assert_ne!(
            a_tok, b_tok,
            "token movement must show up in the token digest (it changed token state)"
        );
        assert!(a_tok.is_empty(), "the no-token run must hold no token cells");
    }

    #[test]
    fn pos_pom_both_dims_finalize() {
        // two validators each carrying capital (pos) AND contribution (pom); both vote.
        let all = vec![v(0, 0.0, 100.0, 100.0), v(1, 0.0, 100.0, 100.0)];
        assert!(finalizes_pos_pom(&all, &all, 1, 0, false, TWO_THIRDS_BPS));
    }

    #[test]
    fn live_finalizes_wrapper_routes_through_pos_pom_not_the_pow_mix() {
        // T3-WIRING ANTI-THEATER: the LIVE `runtime::finalizes` (the wrapper every node call site
        // uses) must route through `finalizes_pos_pom` — PoW out of finality + anti-concentration —
        // NOT the PoW-inclusive constitution mix. The existing tests above exercise
        // `finalizes_pos_pom` DIRECTLY; this one pins the wrapper itself, so reverting `finalizes`
        // back to `finalizes_hybrid(c.mix, ...)` goes RED.
        //
        // A,B vote carrying PoW+PoM but NO PoS; C (PoS-only) abstains.
        let a = v(0, 1.0, 0.0, 1.0);
        let b = v(1, 1.0, 0.0, 1.0);
        let c_pos = v(2, 0.0, 1.0, 0.0);
        let all = vec![a.clone(), b.clone(), c_pos];
        let voters = vec![a, b];
        let c = Constitution::default(); // NCI mix 10/30/60

        // PRECONDITION — the discriminator is real: under the OLD PoW-inclusive rule, A+B's
        // PoW+PoM weight (2×(0.10+0.60)=1.40 of 1.70 ≈ 82%) clears 2/3, so it WOULD finalize.
        assert!(
            crate::consensus::finalizes_hybrid(
                &voters, &all, c.mix, 0, c.horizon, c.decay_pos, c.threshold_bps, c.quorum_floor_bps,
            ),
            "precondition: the PoW-inclusive rule WOULD finalize this set (else the test proves nothing)"
        );
        // THE WIRING: the live wrapper must REJECT it — PoW is excluded and PoS is absent from the
        // voters, so the anti-concentration floor (dim_ok pos) fails.
        assert!(
            !super::finalizes(&c, &voters, &all, 0),
            "live finalization still counts PoW / skips anti-concentration — T3 wiring not live"
        );
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
        assert!(!finalizes_pos_pom(
            &[pow_giant],
            &all,
            1,
            0,
            false,
            TWO_THIRDS_BPS
        ));
        // the two fast-final dimensions together finalize, PoW irrelevant.
        assert!(finalizes_pos_pom(&[pos, pom], &all, 1, 0, false, TWO_THIRDS_BPS));
        // the mix really does exclude PoW.
        assert_eq!(FINALITY_MIX.pow, 0.0);
        assert_eq!(MIN_DIM_BPS, 5000);
    }
}
