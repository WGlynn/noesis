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
use crate::{coverage, pom_scores_with_similarity_floor_q16, tokens, Cell, Script};
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
    /// near-duplicate similarity floor for the PoM / attribution gate, Q16.16. A cell whose
    /// coverage overlap with earlier-committed coverage exceeds this fraction earns 0 novelty —
    /// the paraphrase-padding-ring defense (loop 3). Conservative default 0.95 cuts only
    /// near-identical cells; honest novel work (low overlap) is untouched. Lives here because it
    /// governs HOW value is measured (the measurement-amendment frame).
    pub theta_sim_q16: u64,
    /// Vesting window `W` (DESIGN-vesting-W §2.2, D1/D2) in cumulative-work units ([`Ledger::now`]).
    /// A finalized cell contributes ZERO finality-safety PoM weight until it has aged past `W`
    /// (cliff: cleared iff `finalized_at ≤ now − W`); a younger cell still earns reward/influence
    /// via the full attribution map, it just cannot vote its OWN finalization. `W` buys a dispute
    /// window: gamed value must survive `W` of challenge exposure before it clears into usable
    /// finality weight (the moat's stand-in at launch). Governed constant — NOT a controller output
    /// (a window that reacts to participation is reflexively gameable). Anchored to (≥) the dispute
    /// window (`dispute::Params.window`, one dispute clock). Default `0` ⇒ every finalized cell is
    /// already cleared ⇒ the finality bridge is byte-identical to the full attribution map ⇒ the
    /// feature is INERT until governance sets `W` (mirrors `quorum_floor_bps`/`horizon` defaulting
    /// to 0). Read by [`Node::finality_pom_weight`] (the cleared-score bridge), nowhere else.
    pub vesting_w: u64,
    /// Resource-DoS bound A: the maximum number of pre-consensus proposals a replica
    /// will admit into its local mempool. `Node::submit` rejects admission once the
    /// pool is at this cap, giving a deterministic, economics-independent ceiling on
    /// mempool memory and the downstream per-proposal compute (a flood of cheap,
    /// well-formed-but-worthless cells can no longer exhaust the node). This is a
    /// per-replica LIVENESS/RESOURCE guard, not a SAFETY one — it never enters
    /// `validate`, so it does not change which blocks are valid or final. See
    /// `docs/RESOURCE-DOS-BOUNDING.md` (Bound A); the commit-deposit (Bound B) is
    /// the designed-not-built economic teeth.
    pub max_mempool: usize,
    /// JUL money-layer issuance rate (increment 2 wiring). `apply_transition` reads this to size the
    /// coinbase = `jul::reward_for_work(block_work(b), jul)`. v0 default `JulParams::default()` (1 JUL
    /// per unit of work); governance-BOUNDING it (an amendment GovField) is increment 4.
    pub jul: crate::jul::JulParams,
}

impl Default for Constitution {
    fn default() -> Self {
        Constitution {
            mix: NCI,
            threshold_bps: TWO_THIRDS_BPS,
            quorum_floor_bps: 0,
            horizon: 0,
            decay_pos: false,
            theta_sim_q16: 62259, // floor(0.95 · 2^16) — only near-identical cells are cut
            vesting_w: 0,         // inert: every finalized cell already cleared ⇒ full attribution
            max_mempool: 10_000,  // resource-DoS bound A: per-replica mempool admission cap
            jul: crate::jul::JulParams::default(), // 1 JUL per unit of work (v0 unit-definition)
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
    /// cumulative-work clock — the canonical `now` for every time-denominated mechanism
    /// (franchise decay / `horizon`, and forward: vesting `W`, dispute windows, the interval
    /// controller). Monotone non-decreasing, advanced by [`block_work`] as each block finalizes.
    /// PRE-POW per-block work is the constant [`WORK_PER_BLOCK`], so the clock degrades EXACTLY to
    /// a height-clock (`work == height`); the interface already carries per-block work, so real
    /// mined difficulty folds in with no change to any call site. Replica-deterministic: two nodes
    /// finalizing the same blocks read the same clock (folded into [`Ledger::state_digest`]).
    pub work: u64,
    /// finalized TOKEN cells — the value UTXO set, kept SEPARATE from `cells` so value movement
    /// never touches the novelty index or PoM attribution (both fold over `cells` only, so they
    /// stay token-blind). Token-tx input existence resolves HERE; [`Node::apply`] retires consumed
    /// inputs from and appends produced outputs to this set, enabling multi-hop token flow while
    /// preserving cross-block single-use. Issuance authority cells are seeded into it.
    pub token_cells: Vec<Cell>,
    /// Phase-1 vesting index: the cumulative-work time ([`Ledger::now`]) at which each cell was
    /// finalized, keyed by `cell.id`. The clearing rule (a cell is *cleared* at `now` iff
    /// `finalized_at ≤ now − W`) reads this; a younger cell is *pending* — it still earns
    /// reward/influence but contributes ZERO finality-safety weight (DESIGN-vesting-W §2.1–2.2).
    /// Populated in [`Node::apply`] as each cell enters (first finalization wins). Replica-
    /// deterministic (same blocks + order ⇒ same map) and fully derivable from history, so it is
    /// DELIBERATELY excluded from [`Ledger::state_digest`] — additive + non-consensus-affecting
    /// until the cleared-score bridge (Phase 2) reads it.
    pub finalized_at: HashMap<u64, u64>,
    /// Phase-3 dispute-during-`W` index (DESIGN-vesting-W §2.4): cell ids removed from FUTURE
    /// finality-PoM weight by a LANDED refutation — a dispute whose settlement actually canceled
    /// unvested value. Keyed by `cell.id` (the same id space as `finalized_at` and the dispute
    /// layer's `Challenge.target`). [`Node::finality_pom_weight`] filters these out as a SECOND gate
    /// beside the vesting cliff, so a cell gamed-in and refuted while still PENDING never ages into
    /// finality weight. FORWARD-ONLY: past finalized checkpoints are never recomputed — a refutation
    /// only strips the bad cell from finality decisions going forward. Written by
    /// [`Node::record_refutation`].
    ///
    /// Also excluded from [`Ledger::state_digest`], but for a DIFFERENT reason than `finalized_at`:
    /// `finalized_at` is derivable from block replay, whereas refutations are NOT yet block-embedded,
    /// so `refuted` is populated out-of-band and is not history-derivable until dispute resolutions
    /// enter the replayable block/Op stream (the deploy-coupled step, designed-not-built). Until that
    /// lands, two replicas converge on `refuted` only if fed the same refutations — this build is the
    /// reference mechanism + its forward-only exclusion property, NOT cross-replica determinism.
    pub refuted: HashSet<u64>,
    /// Cumulative JUL issued via block coinbases (increment 2). Monotone; the ONLY writer is
    /// `apply_transition`'s coinbase mint. DELIBERATELY excluded from [`Ledger::state_digest`] (the
    /// `finalized_at` precedent): a pure fold over the replayed blocks' coinbase amounts —
    /// history-derivable and replica-deterministic — so re-hashing it adds only tuple churn with no
    /// divergence-detection power (a supply divergence implies a `token_ids`/`work` divergence the
    /// digest already catches). Conservation (`jul_supply == Σ live JUL + burns`) is proven by test.
    pub jul_supply: crate::jul::JulSupply,
}

/// Comparable digest of a replica's state (see [`Ledger::state_digest`]): finalized cell-id
/// sequence, novelty-index root, sorted PoM attribution, token-cell id sequence, and the
/// cumulative-work clock. Two honest replicas that finalized the same blocks hold equal digests.
type StateDigest = (Vec<u64>, Hash, Vec<(Vec<u8>, u64)>, Vec<u64>, u64);

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            cells: Vec::new(),
            index: NoveltyIndex::new(),
            pom: HashMap::new(),
            height: 0,
            work: 0,
            token_cells: Vec::new(),
            finalized_at: HashMap::new(),
            refuted: HashSet::new(),
            jul_supply: crate::jul::JulSupply::new(),
        }
    }

    /// Compact, comparable digest of replica state for convergence checks: the finalized
    /// cell-id sequence, the novelty-index root, the sorted PoM attribution map, the finalized
    /// token-cell id sequence (deterministic apply order ⇒ replicas converge on token state too,
    /// not just attribution), and the cumulative-work clock (so a replica that diverged on the
    /// `now` base is caught by the SAME convergence check the 2-node / gaming harnesses run).
    pub fn state_digest(&self) -> StateDigest {
        let ids: Vec<u64> = self.cells.iter().map(|c| c.id).collect();
        let mut pom: Vec<(Vec<u8>, u64)> = self.pom.iter().map(|(k, v)| (k.clone(), *v)).collect();
        pom.sort();
        let token_ids: Vec<u64> = self.token_cells.iter().map(|c| c.id).collect();
        (ids, self.index.root(), pom, token_ids, self.work)
    }

    /// Read the cumulative-work clock — the canonical `now`. Every time-denominated mechanism
    /// (franchise decay, and forward vesting `W` / dispute windows / the interval controller)
    /// measures elapsed time against THIS, not block-height or wall-clock. Monotone;
    /// replica-deterministic. Pre-PoW `now() == height`.
    pub fn now(&self) -> u64 {
        self.work
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
    /// JUL coinbase recipient (increment 2): the lock paid this block's issuance. `None` ⇒ no mint
    /// (all pre-increment-2 blocks ⇒ replay-parity preserved). The AMOUNT is NOT carried — it is
    /// CONSTRUCTED in `apply_transition` as `jul::reward_for_work(block_work(b), params.jul)`, so a
    /// forged amount is unrepresentable. RECIPIENT POLICY (Will-gated): producer-named (Bitcoin-style);
    /// the amount is protocol-fixed, so recipient choice can only direct the subsidy, never enlarge it.
    pub coinbase: Option<Script>,
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
            coinbase: None,
        }
    }

    /// Attach token movements to a proposed block (builder; empty by default). Each tx is
    /// gated by [`Node::validate`] — one non-conserving movement makes the whole block invalid.
    pub fn with_token_txs(mut self, txs: Vec<TokenTx>) -> Block {
        self.token_txs = txs;
        self
    }

    /// Attach a JUL coinbase recipient (increment 2; builder, `None` by default). The issuance amount
    /// is constructed at apply time, not carried here — this only names who is paid.
    pub fn with_coinbase(mut self, recipient: Script) -> Block {
        self.coinbase = Some(recipient);
        self
    }
}

// ============ Cumulative-work clock — the canonical `now` ============
//
// Every time-denominated mechanism in Noesis measures elapsed time as CUMULATIVE WORK, not
// block-height or wall-clock: franchise decay (`horizon`), and forward — vesting `W`, dispute
// windows, the interval controller. Work is MONOTONE and REPLICA-DETERMINISTIC, so two honest
// nodes finalizing the same blocks read the same clock (folded into `Ledger::state_digest`).
//
// PRE-POW, per-block work is the constant `WORK_PER_BLOCK`, so the clock degrades EXACTLY to a
// height-clock (`work == height`). But `block_work` already takes the block, so when real PoW
// lands it returns the block's mined difficulty and every call site keeps reading ONE monotone
// clock with no change — the "right interface" the height-clock degenerate case hides behind.

/// Work contributed by one finalized block. Pre-PoW: a constant ⇒ the cumulative clock == height.
pub const WORK_PER_BLOCK: u64 = 1;

/// The work a block adds to the cumulative-work clock. Pre-PoW this is `WORK_PER_BLOCK` for every
/// block (the height-clock degenerate case); post-PoW, replace the body with the block's mined
/// difficulty (a future `Block::difficulty`) and every reader of the clock is unchanged — the
/// signature is already difficulty-ready. Single source of per-block work.
pub fn block_work(_b: &Block) -> u64 {
    WORK_PER_BLOCK
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

    /// Gossip a proposal into the local mempool (pre-consensus). Returns `true` if the
    /// proposal was admitted, `false` if the mempool is at its `max_mempool` cap and the
    /// proposal was rejected. The cap is resource-DoS bound A: it bounds mempool memory
    /// and downstream per-proposal compute against a flood of cheap, well-formed-but-
    /// worthless cells, deterministically and independently of any economic assumption.
    /// Reject-when-full (not evict-by-quality) is the lean v1 rule — quality-prioritised
    /// eviction needs an at-admission quality signal, which is Bound B's commit-deposit.
    /// See `docs/RESOURCE-DOS-BOUNDING.md`.
    pub fn submit(&mut self, cell: Cell, coord: Committed) -> bool {
        if self.mempool.len() >= self.constitution.max_mempool {
            return false;
        }
        self.mempool.push((cell, coord));
        true
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
        // Thin caller of the rulebook's acceptance gate ([`validate_block`]). Block validity is
        // now defined in exactly ONE place, shared with [`apply_block`] and its Phase 2-4 consumers.
        validate_block(&self.ledger, b).is_ok()
    }

    /// A node's vote on a proposal is its honest local validation.
    pub fn vote(&self, b: &Block) -> bool {
        self.validate(b)
    }

    /// Does a proposed checkpoint finalize *at the current clock*? Sources `now` from the
    /// cumulative-work clock ([`Ledger::now`]) rather than an externally-supplied counter — the
    /// wiring that makes work the canonical time base for the finality/liveness decision (and,
    /// forward, for every mechanism that reads `now`: vesting `W`, dispute windows, the interval
    /// controller). `voters_for` are the validators supporting the checkpoint; `all` is the set.
    pub fn checkpoint_finalizes(&self, voters_for: &[Validator], all: &[Validator]) -> bool {
        finalizes(&self.constitution, voters_for, all, self.ledger.now())
    }

    /// The A4-accountable [`Node::checkpoint_finalizes`] — sources `now` from the work clock exactly
    /// like [`Node::checkpoint_finalizes`], but runs the equivocation guard (slash-before-count) over
    /// the epoch `ballots` = `(validator_id, proposal_id)` pairs. Returns `(finalizes?,
    /// equivocator_ids)`: a double-signer's weight is stripped from BOTH the supporting set and the
    /// basis before any weight is summed, so a tainted vote can never contribute to finalization — the
    /// property bare [`Node::checkpoint_finalizes`] (a stateless weight predicate with no vote history)
    /// structurally cannot provide. This wires the built A4 guard onto the LIVE finality path, closing
    /// the finality-SAFETY half of the code's own `[GAP]` "Lifecycle omitted: equivocation slashing".
    ///
    /// HONEST SCOPE: the tainted vote is void on the DECISION here. Making the slash PERSIST — durably
    /// reducing a validator's stake across epochs via [`crate::consensus::slash`] — needs a persistent
    /// validator registry (the T1 network build); the single-process devnet rebuilds its set each
    /// round, so this returns the equivocator ids for a persistent caller to slash.
    pub fn checkpoint_finalizes_guarded(
        &self,
        voters_for: &[Validator],
        all: &[Validator],
        ballots: &[(u64, u64)],
    ) -> (bool, Vec<u64>) {
        finalizes_guarded(&self.constitution, voters_for, all, ballots, self.ledger.now())
    }

    /// The cleared-score bridge (DESIGN-vesting-W §2.3, build-stage §3.2) — the production
    /// `Standing → Validator.pom` source on the finality path. Returns PoM finality weight per
    /// contributor (`type_script.args`) computed over ONLY the cells that have CLEARED the vesting
    /// window: `finalized_at ≤ now − W` (cliff, D2). A cell younger than `W` is *pending* — it
    /// still earns reward/influence via [`Ledger::pom`] (the full, uncleared attribution), but
    /// contributes ZERO here, so gamed value gets a full `W`-window of dispute exposure before it
    /// can vote its own finalization (§2.4). Callers assembling the validator set source each
    /// `Validator.pom` from this map (absent contributor ⇒ 0), replacing the test-constructor value.
    ///
    /// CONSENSUS-AFFECTING: unlike [`Ledger::pom`] (all finalized cells), this is what feeds the
    /// finality decision. Properties that fall out of the definition:
    ///   * **Genesis** (§2.5): nothing has aged ⇒ the map is empty ⇒ every sourced `Validator.pom`
    ///     is 0 ⇒ bonded PoS carries finality from block zero. No special-case code.
    ///   * **`W == 0`** (the inert default): `now − 0 = now`, and every finalized cell has
    ///     `finalized_at ≤ now`, so ALL clear ⇒ this equals the full attribution map exactly.
    ///
    /// Sources `W` and the similarity floor from the [`Constitution`]. `now` is the cumulative-work
    /// clock ([`Ledger::now`]). Filtering preserves canonical commit order, so the similarity floor
    /// evaluates the cleared cells among themselves deterministically ⇒ replica-identical.
    ///
    /// Phase 3 (dispute-during-`W`, BUILT): a refutation that LANDS on a still-pending cell records
    /// it in [`Ledger::refuted`] (via [`Node::record_refutation`]); this map filters those cells out
    /// as a SECOND gate beside the cliff, so a gamed cell refuted before it ages past `W` never
    /// contributes finality weight (forward-only — past checkpoints untouched).
    pub fn finality_pom_weight(&self) -> HashMap<Vec<u8>, u64> {
        // clearing frontier: a cell clears iff its finalization work-time is at least `W` old.
        // `saturating_sub` floors at 0 (pre-genesis / W > now ⇒ frontier 0 ⇒ nothing clears, since
        // every stamp is ≥ 1: `block_work ≥ 1` so the first block advances the clock past 0).
        let frontier = self.ledger.now().saturating_sub(self.constitution.vesting_w);
        let cleared: Vec<Cell> = self
            .ledger
            .cells
            .iter()
            .filter(|c| {
                // (1) vesting cliff — fail closed on the safety input: a cell with no stamp (cannot
                // occur — `cells` and `finalized_at` are written in the same `apply` pass — but do not
                // assume) is treated as pending, never cleared.
                let past_cliff = self.ledger.finalized_at.get(&c.id).is_some_and(|&f| f <= frontier);
                // (2) Phase-3 dispute gate — a cell refuted while pending is stripped from finality
                // weight forever (forward-only). AND-ed with the cliff so a refuted cell never clears
                // here even once it ages past `W`.
                past_cliff && !self.ledger.refuted.contains(&c.id)
            })
            .cloned()
            .collect();
        pom_scores_with_similarity_floor_q16(&cleared, self.constitution.theta_sim_q16)
    }

    /// Phase 3 (dispute-during-`W`): record a LANDED refutation so its target cell is excluded from
    /// all FUTURE [`Node::finality_pom_weight`] (forward-only). A refutation "lands" iff the dispute
    /// [`crate::dispute::Settlement`] actually CANCELED unvested value (`canceled > 0.0`) — an UPHELD
    /// challenge or a guarded acquittal returns a zero/empty settlement and records NOTHING, so an
    /// unrefuted cell is never excluded. Idempotent (a `HashSet`). Touches no finalized state
    /// (`cells` / `finalized_at` / the digest) ⇒ no block un-finalizes; only future finality weight
    /// loses the bad cell. `target` is the challenged cell id (the dispute layer's `Challenge.target`,
    /// the same id space `finalized_at` / `finality_pom_weight` key on).
    pub fn record_refutation(&mut self, target: u64, settlement: &crate::dispute::Settlement) {
        if settlement.canceled > 0.0 {
            self.ledger.refuted.insert(target);
        }
    }

    /// Apply a finalized block: the in-place state transition. Delegates to the pure rulebook
    /// [`apply_transition`] (the extracted body) so `Node::apply` and the value-returning
    /// [`apply_block`] share ONE definition of the transition. Precondition (unchanged): `b` has
    /// already passed [`Node::validate`] in the consensus flow — [`apply_transition`] performs no
    /// validation, exactly as this method did before the extraction (byte-identical behaviour).
    pub fn apply(&mut self, b: &Block) {
        apply_transition(&mut self.ledger, b, &self.constitution);
    }

    /// Drop mempool entries (called after a block finalizes; v1 clears the whole pool since
    /// the round's proposals are all included).
    pub fn clear_mempool(&mut self) {
        self.mempool.clear();
    }
}

// ============ apply_block — the single rulebook (pure, deterministic) ============
//
// The complete state transition, extracted as a pure function of (previous state, block):
// same inputs ⇒ same output, byte-for-byte, forever. No I/O, no clock (only the derived
// cumulative-work counter, which is itself part of the state), no randomness, no floats — the
// purity verdict in `docs/rulebook-map.md` §4, verified at source. `Node::validate` and
// `Node::apply` are now thin callers, so consensus behaviour is defined in exactly ONE place:
// the function the commitment layer (Phase 2), the zkVM guest (Phase 3), and the formal spec
// (Phase 4) all consume. Extraction preserves behaviour bit-identically — see the replay-parity
// test `node/tests/apply_block_parity.rs`.

/// Why a block was rejected by the rulebook — one variant per acceptance check in
/// [`validate_block`]. Carrying the reason (vs a bare `bool`) is what the `Result` shape of
/// [`apply_block`] gives the zkVM guest and the Phase-4 invariant proofs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Violation {
    /// (1) block does not extend the chain by exactly one.
    HeightMismatch { expected: u64, got: u64 },
    /// (2) cells and coords are not 1:1.
    CellCoordCountMismatch { cells: usize, coords: usize },
    /// (3) empty block.
    EmptyBlock,
    /// (4) coords are not in canonical commit order (a producer reorder).
    NonCanonicalOrder,
    /// (5) a coordinate's height does not equal the block height.
    CoordHeightMismatch,
    /// (6) a token movement is not ledger-valid, or an input identity is spent twice in-block.
    TokenTxInvalidOrDoubleSpend,
}

/// The acceptance gate: the six conjoined checks of the old `Node::validate`, returning the typed
/// [`Violation`] instead of a `bool`. Pure over `(&state, &block)` — no `Node`, no I/O, no clock.
pub fn validate_block(ledger: &Ledger, b: &Block) -> Result<(), Violation> {
    if b.height != ledger.height + 1 {
        return Err(Violation::HeightMismatch { expected: ledger.height + 1, got: b.height });
    }
    if b.cells.len() != b.coords.len() {
        return Err(Violation::CellCoordCountMismatch { cells: b.cells.len(), coords: b.coords.len() });
    }
    if b.cells.is_empty() {
        return Err(Violation::EmptyBlock);
    }
    if !is_canonical_order(&b.coords) {
        return Err(Violation::NonCanonicalOrder);
    }
    if !b.coords.iter().all(|c| c.height == b.height) {
        return Err(Violation::CoordHeightMismatch);
    }
    if !token_txs_conserve_and_single_use(ledger, b) {
        return Err(Violation::TokenTxInvalidOrDoubleSpend);
    }
    Ok(())
}

/// Checks (5)+(6) over the ledger's token-cell set: each tx is ledger-valid (conserves AND spends
/// only live inputs — [`TokenTx::is_valid_in_ledger`]), AND no input identity is consumed more than
/// once across the whole block. Extracted from the old `Node` method so it is callable without a
/// `Node`. Identity is the consensus-derived tuple `(id, lock, type_script)`, not a producer-asserted
/// nullifier — the crypto nullifier / on-VM UTXO-set retirement is the deploy-coupled layer; this
/// reference set models both block scopes.
fn token_txs_conserve_and_single_use(ledger: &Ledger, b: &Block) -> bool {
    let mut consumed: HashSet<(u64, Script, Script)> = HashSet::new();
    for tx in &b.token_txs {
        if !tx.is_valid_in_ledger(&ledger.token_cells) {
            return false;
        }
        // JUL SECURITY (increment 2): JUL is conserve-or-burn-ONLY through the token path ⇒ the block
        // coinbase is the structurally unique JUL inflation channel. Without this, a holder could pay
        // JUL to a cell with `lock.args == JUL_ISSUER`, then consume it as an "authority input" and mint
        // unbounded JUL (both Fable-5 planners found this hole independently). Also keeps the FV
        // spec-oracle (`Σout > Σin ⇒ reject`) honest for JUL.
        if tx.code_hash == crate::jul::JUL_CODE_HASH && tx.args.as_slice() == crate::jul::JUL_ISSUER {
            let out = crate::tokens::fungible::total(&tx.outputs, &tx.code_hash, &tx.args);
            let inp = crate::tokens::fungible::total(&tx.inputs, &tx.code_hash, &tx.args);
            if out > inp {
                return false;
            }
        }
        // The reserved coinbase id space is protocol-only: a token-tx output may not squat a coinbase
        // id (retirement matches `(id, lock, type_script)`, so a collision could grief a real reward).
        for out in &tx.outputs {
            if out.id & crate::jul::COINBASE_ID_BIT != 0 {
                return false;
            }
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

/// The pure state transition (no validation) — the exact body of the old `Node::apply`, reshaped to
/// thread the `Ledger` + measurement params explicitly instead of `&mut self`. DETERMINISTIC:
/// identical `(state, block, params)` ⇒ identical next state. Uses `&mut` purely as a zero-copy impl
/// detail of the owned-in/owned-out [`apply_block`]; it performs no I/O, reads no clock/rand/float.
fn apply_transition(state: &mut Ledger, b: &Block, params: &Constitution) {
    // TOKEN STATE TRANSITION (over the SEPARATE `token_cells` set, never `cells`):
    //   (a) CROSS-BLOCK single-use — retire each consumed token input (UTXO retirement), so a later
    //       block's existence check fails for an already-spent cell; then
    //   (b) PERSIST OUTPUTS — append each tx's produced outputs (multi-hop A→B→C flow).
    // Retire BEFORE appending; `cells`/index/`pom` are untouched here ⇒ value movement does not
    // perturb attribution. (Crypto nullifier + on-VM UTXO-set retirement are the deploy-coupled layer.)
    for tx in &b.token_txs {
        for inp in &tx.inputs {
            state
                .token_cells
                .retain(|c| !(c.id == inp.id && c.lock == inp.lock && c.type_script == inp.type_script));
        }
    }
    for tx in &b.token_txs {
        for out in &tx.outputs {
            state.token_cells.push(out.clone());
        }
    }
    // JUL COINBASE (increment 2): settle this block's issuance as a Fungible cell to the producer-named
    // recipient. The amount is CONSTRUCTED here (never carried on the block), so a forged amount is
    // unrepresentable, and it is the ONLY authorized JUL inflation (token_txs are JUL-conserve-only).
    // Touches `token_cells` only — never `cells`/index/pom — so PoW/JUL stays finality-excluded by construction.
    if let Some(recipient) = &b.coinbase {
        let reward = crate::jul::reward_for_work(block_work(b), params.jul);
        if reward > 0 {
            state.token_cells.push(Cell {
                id: crate::jul::coinbase_id(b.height),
                lock: recipient.clone(),
                type_script: Script {
                    code_hash: crate::jul::JUL_CODE_HASH,
                    args: crate::jul::JUL_ISSUER.to_vec(),
                },
                parent: None,
                timestamp: 0,
                data: crate::tokens::fungible::encode(reward),
            });
            state.jul_supply.credit(reward);
        }
    }
    for cell in &b.cells {
        for key in coverage(&cell.data) {
            state.index.insert(key);
        }
        state.cells.push(cell.clone());
    }
    state.height = b.height;
    // advance the cumulative-work clock (the canonical `now`). Monotone by construction
    // (block_work ≥ 1 and saturating_add never wraps); pre-PoW this tracks height exactly.
    state.work = state.work.saturating_add(block_work(b));
    // vesting stamp: `or_insert` ⇒ a cell's FIRST finalization wins (idempotent under re-apply).
    // Not folded into `state_digest` ⇒ additive; same blocks + order ⇒ same map (replica-deterministic).
    let finalized_now = state.work;
    for cell in &b.cells {
        state.finalized_at.entry(cell.id).or_insert(finalized_now);
    }
    // recompute PoM attribution over the full chain (integer Q16.16 similarity floor).
    state.pom = pom_scores_with_similarity_floor_q16(&state.cells, params.theta_sim_q16);
}

/// THE RULEBOOK. Pure, deterministic `(prev_state, block) -> Result<next_state, violation>`:
/// validate the block, then apply the transition. No I/O, clock, randomness, or floats — the single
/// definition of Noesis's state transition that the commitment layer, the zkVM guest, and the formal
/// spec all consume. `params` are genesis-fixed consensus constants (the measurement frame): a
/// deterministic input, not mutable state. On rejection the (moved-in) state is dropped and the
/// typed [`Violation`] returned — an invalid block produces no next state.
pub fn apply_block(state: Ledger, b: &Block, params: &Constitution) -> Result<Ledger, Violation> {
    validate_block(&state, b)?;
    let mut next = state;
    apply_transition(&mut next, b, params);
    Ok(next)
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
/// `FINALITY_MIX`), so the 235-test core rule is intact; the `c.mix` field governs the
/// production/ordering path, not this fast-final gate. `c.quorum_floor_bps` DOES govern this gate
/// (wired 2026-07-11): it is the finality participation floor / safe-halt backstop, default 0.
/// Forward parity: when the on-VM
/// finalization mirror (🟡 `ON-VM-FINALIZATION.md`) is built, it must mirror THIS rule (PoW-out +
/// anti-concentration), not bare `finalizes_hybrid`.
pub fn finalizes(c: &Constitution, voters_for: &[Validator], all: &[Validator], now: u64) -> bool {
    finality::finalizes_pos_pom(
        voters_for,
        all,
        now,
        c.horizon,
        c.decay_pos,
        c.threshold_bps,
        c.quorum_floor_bps,
    )
}

/// The A4-accountable variant of [`finalizes`] — the same PoW-out + anti-concentration rule, run
/// through the equivocation guard (slash-before-count). `ballots` = the `(validator_id, proposal_id)`
/// pairs seen this epoch. Returns `(finalizes?, equivocator_ids)`; a double-signer's weight is stripped
/// from BOTH the supporting set and the basis before any weight is counted, so a tainted vote can never
/// contribute to finalization. With no double-vote in `ballots` this is identical to [`finalizes`] and
/// the id list is empty. The caller slashes the returned ids on its persistent validator set.
pub fn finalizes_guarded(
    c: &Constitution,
    voters_for: &[Validator],
    all: &[Validator],
    ballots: &[(u64, u64)],
    now: u64,
) -> (bool, Vec<u64>) {
    finality::finalizes_with_equivocation_guard(
        voters_for,
        all,
        ballots,
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
        quorum_floor_bps: u64,
    ) -> bool {
        // `quorum_floor_bps` (from `Constitution.quorum_floor_bps`) floors the finalization
        // denominator at that fraction of the FULL registered set's base weight, so a live set
        // that thins below the floor cannot finalize a minority among itself — it SAFE-HALTS
        // (no finality) instead of finalizing a checkpoint the absent majority never saw. This is
        // the load-bearing backstop for the deactivation composition (block-logistics §123):
        // withhold → stall ≤ grace → deactivated → finality resumes on the live set IFF it still
        // clears the floor, else safe halt. Default 0 ⇒ no floor ⇒ liveness-favoring historical
        // behavior (finalize on whoever is live). A SAFETY floor, kept a governed constant — never
        // a controller output — because a floor that reacts to participation is reflexively
        // gameable (an attacker induces the low-participation condition that lowers it).
        if !consensus::finalizes_hybrid(
            voters_for,
            all,
            FINALITY_MIX,
            now,
            horizon,
            decay_pos,
            threshold_bps,
            quorum_floor_bps,
        ) {
            return false;
        }
        let pos_for: f64 = voters_for.iter().map(|v| v.pos).sum();
        let pos_all: f64 = all.iter().map(|v| v.pos).sum();
        let pom_for: f64 = voters_for.iter().map(|v| v.pom).sum();
        let pom_all: f64 = all.iter().map(|v| v.pom).sum();
        dim_ok(pos_for, pos_all) && dim_ok(pom_for, pom_all)
    }

    /// A4 accountable-safety — closes the code's own `[GAP]` "Lifecycle omitted: equivocation
    /// slashing" (`lib.rs`). An equivocator is a validator that supports two DISTINCT proposals in
    /// one epoch (a double-vote). `ballots` = the `(validator_id, proposal_id)` pairs seen this
    /// epoch. Deterministic (BTree-ordered ⇒ replicas agree on the same equivocator set). Reuses the
    /// A4 primitive [`consensus::is_equivocation`] as the single source of the double-vote rule.
    pub fn epoch_equivocators(ballots: &[(u64, u64)]) -> std::collections::BTreeSet<u64> {
        use std::collections::{BTreeMap, BTreeSet};
        let mut first_vote: BTreeMap<u64, u64> = BTreeMap::new();
        let mut equivocators: BTreeSet<u64> = BTreeSet::new();
        for &(validator, proposal) in ballots {
            match first_vote.get(&validator) {
                // a later ballot for a DIFFERENT proposal than this validator's first = equivocation.
                Some(&prev) => {
                    if consensus::is_equivocation(Some(prev), proposal) {
                        equivocators.insert(validator);
                    }
                }
                None => {
                    first_vote.insert(validator, proposal);
                }
            }
        }
        equivocators
    }

    /// Finalize WITH the A4 equivocation guard — **slash-before-count**. Every equivocator's weight is
    /// stripped from BOTH the supporting set AND the basis BEFORE any weight is summed, so a
    /// double-signer's tainted weight can never contribute to ANY proposal's finalization (the property
    /// bare [`finalizes_pos_pom`] — a stateless weight predicate with no vote history — structurally
    /// cannot provide). Returns `(finalizes?, equivocator_ids)`; the caller applies the stake slash
    /// ([`consensus::slash`]) to the equivocators on the persistent validator set (detection ⇒ the
    /// offending vote is void AND the offender is accountable). Additive: the existing gate is unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn finalizes_with_equivocation_guard(
        voters_for: &[Validator],
        all: &[Validator],
        ballots: &[(u64, u64)],
        now: u64,
        horizon: u64,
        decay_pos: bool,
        threshold_bps: u64,
        quorum_floor_bps: u64,
    ) -> (bool, Vec<u64>) {
        let equivocators = epoch_equivocators(ballots);
        // strip tainted weight before counting — from voters_for (their support is void) and from
        // `all` (their weight leaves the basis too, so the honest remainder decides on its own total).
        let strip = |set: &[Validator]| -> Vec<Validator> {
            set.iter().filter(|v| !equivocators.contains(&v.id)).cloned().collect()
        };
        let ok = finalizes_pos_pom(
            &strip(voters_for),
            &strip(all),
            now,
            horizon,
            decay_pos,
            threshold_bps,
            quorum_floor_bps,
        );
        (ok, equivocators.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::finality::{
        epoch_equivocators, finalizes_pos_pom, finalizes_with_equivocation_guard, FINALITY_MIX,
        MIN_DIM_BPS,
    };
    use super::{block_work, Block, Constitution, Node, TokenStandard, TokenTx, WORK_PER_BLOCK};
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

    // ---- A4 accountable safety: equivocation guard (slash-before-count) — closes the [GAP] ----

    #[test]
    fn equivocation_guard_excludes_tainted_weight_and_reports_the_double_voter() {
        // A validator supporting TWO different proposals in one epoch is an equivocator; its weight
        // must be void for finalization (slash-before-count) and it must be reported so its stake is
        // slashed. Balanced validators (pos==pom) so every check reduces to a clean weight fraction.
        // Cohort: v0 is the PIVOTAL big validator (weight 200); v1,v2 weigh 100 each ⇒ total 400.
        let bal = |id, w: f64| Validator {
            id,
            pow: 0.0,
            pos: w,
            pom: w,
            last_heartbeat: 0,
            staked_balance: w,
        };
        let all = vec![bal(0, 200.0), bal(1, 100.0), bal(2, 100.0)];
        let for_7 = vec![all[0].clone(), all[1].clone()]; // v0 (200) + v1 (100) support proposal 7 = 300/400

        // HONEST baseline: v0 and v1 each vote ONLY for 7 ⇒ 300 of 400 = 75% ≥ 2/3 ⇒ finalizes.
        let honest = vec![(0u64, 7u64), (1u64, 7u64)];
        let (ok, eq) =
            finalizes_with_equivocation_guard(&for_7, &all, &honest, 1, 0, false, TWO_THIRDS_BPS, 0);
        assert!(ok, "honest support (300 of 400 = 75%) finalizes");
        assert!(eq.is_empty(), "no equivocators in the honest epoch");

        // ATTACK: the PIVOTAL v0 ALSO casts a conflicting vote for proposal 8 ⇒ it equivocates and is
        // stripped BEFORE counting ⇒ only v1 (100) genuinely supports 7, out of the honest remainder
        // {v1,v2}=200 ⇒ 50% < 2/3 ⇒ NOT final. Its tainted 200 could not carry the proposal.
        let attack = vec![(0u64, 7u64), (1u64, 7u64), (0u64, 8u64)];
        let (ok2, eq2) =
            finalizes_with_equivocation_guard(&for_7, &all, &attack, 1, 0, false, TWO_THIRDS_BPS, 0);
        assert!(!ok2, "an equivocator's tainted weight cannot finalize (slash-before-count)");
        assert_eq!(eq2, vec![0], "the double-voter is reported for slashing");

        // ANTI-THEATER: WITHOUT the guard the SAME support (300 of 400) finalizes — so the strip is
        // what changed the outcome, not a coincidence of the cohort.
        assert!(
            finalizes_pos_pom(&for_7, &all, 1, 0, false, TWO_THIRDS_BPS, 0),
            "control: the same support finalizes when the equivocator is NOT stripped"
        );

        // ACCOUNTABILITY: a reported equivocator is slashable to zero (composes with A5), so the
        // lifecycle is detect -> void the vote -> slash the offender, not detection alone.
        let mut offender = all[0].clone();
        crate::consensus::slash(&mut offender, 200.0);
        assert_eq!(offender.staked_balance, 0.0, "reported equivocator slashed to zero stake");
        assert_eq!(offender.pom, 0.0, "and its franchise is revoked");

        // determinism: the equivocator set is BTree-ordered ⇒ identical across replicas/runs.
        assert_eq!(epoch_equivocators(&attack).into_iter().collect::<Vec<_>>(), vec![0]);
    }

    // ---- resource-DoS bound A: bounded mempool admission cap ----

    #[test]
    fn resource_dos_flood_is_bounded_by_mempool_cap() {
        // A flood of cheap, well-formed-but-worthless proposals cannot grow the mempool
        // past the constitutional cap. This is the deterministic, economics-independent
        // ceiling on mempool memory + downstream per-proposal compute (resource-DoS bound
        // A; see docs/RESOURCE-DOS-BOUNDING.md). The economic gate already makes the flood
        // unprofitable (it scores 0); this bounds the RESOURCE to evaluate it.
        let mk = |i: u64| {
            (
                Cell {
                    id: i,
                    lock: Script {
                        code_hash: [0u8; 32],
                        args: vec![i as u8],
                    },
                    type_script: Script {
                        code_hash: [1u8; 32],
                        args: b"flooder".to_vec(),
                    },
                    parent: None,
                    timestamp: i,
                    data: vec![i as u8; 8], // distinct, well-formed payloads
                },
                Committed {
                    height: 1,
                    secret: [0u8; 32],
                },
            )
        };

        // Capped pool: the first `max_mempool` proposals admit, every one after is rejected.
        let con = Constitution {
            max_mempool: 4,
            ..Constitution::default()
        };
        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], con);
        let admitted = (0..100u64)
            .filter(|&i| {
                let (c, co) = mk(i);
                node.submit(c, co)
            })
            .count();
        assert_eq!(admitted, 4, "exactly `max_mempool` proposals admitted out of 100 offered");
        assert_eq!(
            node.mempool.len(),
            4,
            "mempool memory bounded by the cap, not by attacker volume"
        );

        // ANTI-THEATER (break-on-purpose): lift the cap and the SAME flood all admits, so
        // the cap is genuinely what bounds it — not a coincidental limit in the harness.
        let uncapped = Constitution {
            max_mempool: usize::MAX,
            ..Constitution::default()
        };
        let mut open = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], uncapped);
        let open_admitted = (0..100u64)
            .filter(|&i| {
                let (c, co) = mk(i);
                open.submit(c, co)
            })
            .count();
        assert_eq!(
            open_admitted, 100,
            "uncapped pool admits the whole flood — the cap is what bounds it"
        );
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

    // ---- Phase-1 vesting: cell finalization stamp (DESIGN-vesting-W §2.1, build-stage §3.1) ----

    #[test]
    fn finalized_at_stamps_each_cell_with_its_blocks_work_time() {
        // Each cell is stamped with the cumulative-work clock AS OF the block that finalized it.
        // Pre-PoW the clock == height, and carrier_at(h, h, ..) makes cell.id == h, so the cell
        // finalized by block h must carry finalized_at == h (== now() at that finalization).
        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        assert!(
            node.ledger.finalized_at.is_empty(),
            "genesis: nothing finalized ⇒ empty stamp map ⇒ nothing cleared (bootstrap §2.5)"
        );
        for h in 1..=3u64 {
            node.apply(&carrier_at(h, h, h as u8, format!("phase1 stamp {}", h).as_bytes()));
            assert_eq!(
                node.ledger.finalized_at.get(&h).copied(),
                Some(node.ledger.now()),
                "cell {} stamped with the work-time of the block that finalized it",
                h
            );
        }
        // A cell's finalization time is fixed once set — earlier stamps do not move as the clock runs.
        assert_eq!(node.ledger.finalized_at.get(&1).copied(), Some(1));
        assert_eq!(node.ledger.finalized_at.get(&2).copied(), Some(2));
        assert_eq!(node.ledger.finalized_at.get(&3).copied(), Some(3));
    }

    #[test]
    fn finalized_at_is_replica_deterministic() {
        // Two honest nodes finalizing the SAME blocks in the SAME order hold identical stamp maps —
        // the property Phase 2's cleared-score bridge relies on (same cleared set on every replica).
        let blocks: Vec<Block> = (1..=4u64)
            .map(|h| carrier_at(h, h, h as u8, format!("replica det {}", h).as_bytes()))
            .collect();
        let mut a = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        let mut b = Node::new(1, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        for blk in &blocks {
            a.apply(blk);
            b.apply(blk);
        }
        let mut am: Vec<(u64, u64)> = a.ledger.finalized_at.iter().map(|(k, v)| (*k, *v)).collect();
        let mut bm: Vec<(u64, u64)> = b.ledger.finalized_at.iter().map(|(k, v)| (*k, *v)).collect();
        am.sort();
        bm.sort();
        assert_eq!(am, bm, "same blocks ⇒ same finalized_at on every replica");
    }

    #[test]
    fn finalized_at_stamps_every_cell_once_and_stays_off_the_digest() {
        // Completeness + additivity: every finalized cell is stamped exactly once (map size ==
        // finalized-cell count, every id present), and the stamp is NOT in the consensus digest —
        // so it cannot perturb existing scoring/convergence (nothing reads it for finality yet).
        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        for h in 1..=5u64 {
            node.apply(&carrier_at(h, h, h as u8, format!("completeness {}", h).as_bytes()));
        }
        assert_eq!(
            node.ledger.finalized_at.len(),
            node.ledger.cells.len(),
            "exactly one stamp per finalized cell (no misses, no doubles)"
        );
        for c in &node.ledger.cells {
            assert!(node.ledger.finalized_at.contains_key(&c.id), "cell {} is stamped", c.id);
        }
        // state_digest is the 5-tuple (ids, novelty-root, pom, token-ids, work); finalized_at is
        // absent by construction, so a twin replica on the same blocks agrees on the digest
        // regardless of the (non-hashed) stamp map — additive, non-consensus-affecting.
        let mut twin = Node::new(9, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        for h in 1..=5u64 {
            twin.apply(&carrier_at(h, h, h as u8, format!("completeness {}", h).as_bytes()));
        }
        assert_eq!(
            node.ledger.state_digest(),
            twin.ledger.state_digest(),
            "digest unchanged by the stamp — it is derived state, not consensus state"
        );
    }

    // ---- Phase-2 vesting: the cleared-score bridge (DESIGN-vesting-W §2.3, build-stage §3.2) ----

    /// A carrier block at `height` (id == height ⇒ pre-PoW `finalized_at == height`) whose single
    /// attribution cell is owned by `contributor` (its `type_script.args` — the PoM key), with a
    /// distinct payload so the similarity floor never zeroes it. Lets a test give each block a
    /// different contributor and observe per-contributor clearing.
    fn carrier_for(height: u64, contributor: &[u8]) -> Block {
        let c = Cell {
            id: height,
            lock: Script {
                code_hash: [0u8; 32],
                args: b"lk".to_vec(),
            },
            type_script: Script {
                code_hash: [1u8; 32],
                args: contributor.to_vec(),
            },
            parent: None,
            timestamp: height,
            // payload = a run of a per-height-unique byte ⇒ each block's coverage is a DISJOINT
            // 4-gram set (no cross-block overlap) ⇒ every contributor earns novelty independently.
            // This isolates the VESTING cliff under test from the near-duplicate similarity floor
            // (θ=0.95), which would otherwise zero later near-identical payloads and MASK clearing
            // (a one-char-apart payload overlaps >95% and earns 0 novelty regardless of vesting).
            // 64 bytes ≫ the 4-byte shingle floor in `coverage`.
            data: vec![b'A' + (height % 26) as u8; 64],
        };
        Block::assemble(
            height,
            &[(
                c,
                Committed {
                    height,
                    secret: [height as u8; 32],
                },
            )],
        )
    }

    #[test]
    fn finality_pom_weight_clears_on_the_cliff_pending_contributes_zero() {
        // §2.2/D2 cliff: a cell contributes finality PoM weight iff finalized_at ≤ now − W. Distinct
        // contributor per block ⇒ per-contributor clearing is directly observable, and the exact
        // cliff boundary (block 3 cleared, block 4 pending) is pinned.
        let w = 2u64;
        let con = Constitution { vesting_w: w, ..Constitution::default() };
        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], con);
        let who = |h: u64| format!("contrib{}", h).into_bytes();
        for h in 1..=5u64 {
            node.apply(&carrier_for(h, &who(h)));
        }
        assert_eq!(node.ledger.now(), 5, "pre-PoW clock == height after 5 blocks");
        // now == 5, frontier = now − W = 3 ⇒ blocks 1..=3 CLEARED, 4..=5 PENDING.
        let cleared = node.finality_pom_weight();
        for h in 1..=3u64 {
            assert!(
                cleared.get(&who(h)).copied().unwrap_or(0) > 0,
                "block {} finalized_at {} ≤ frontier 3 ⇒ CLEARED ⇒ contributes finality weight",
                h,
                h
            );
        }
        for h in 4..=5u64 {
            assert_eq!(
                cleared.get(&who(h)).copied().unwrap_or(0),
                0,
                "block {} finalized_at {} > frontier 3 ⇒ PENDING ⇒ ZERO finality weight",
                h,
                h
            );
        }
        // the PENDING work still earns full standing/reward via the (uncleared) attribution map —
        // vesting gates FINALITY weight only, not reward (§2.2). This is the usable-vs-gameable split.
        for h in 4..=5u64 {
            assert!(
                node.ledger.pom.get(&who(h)).copied().unwrap_or(0) > 0,
                "pending block {} still earns standing — only its FINALITY weight is withheld",
                h
            );
        }
        // ANTI-THEATER: if the bridge did NOT filter (returned the full map), blocks 4/5 would be
        // non-zero and the PENDING asserts above would fail ⇒ the filter is load-bearing, not decor.
    }

    // ---- Phase-3 dispute-during-`W`: refuted-while-pending cells never age into finality ----

    #[test]
    fn refuted_pending_cell_never_ages_into_finality_weight() {
        // Phase 3 (dispute-during-`W`): a cell gamed in and REFUTED while still pending must never
        // contribute finality PoM weight, even after it ages past `W`. Its un-refuted sibling (an
        // UPHELD challenge, which records nothing) still clears — no over-exclusion.
        let w = 2u64;
        let con = Constitution { vesting_w: w, ..Constitution::default() };
        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], con);
        let who = |h: u64| format!("contrib{}", h).into_bytes();
        // blocks 1 & 2 finalize (cell id == height). At now == 2, frontier = 2 − 2 = 0 ⇒ both PENDING.
        node.apply(&carrier_for(1, &who(1)));
        node.apply(&carrier_for(2, &who(2)));
        assert_eq!(node.ledger.now(), 2, "pre-PoW clock == height");
        // A dispute LANDS on cell 1 while it is still pending: the refuting settlement cancels its
        // unvested value (canceled > 0) ⇒ record_refutation stamps cell 1 into `refuted`.
        let refuting = crate::dispute::Settlement { canceled: 5.0, ..Default::default() };
        node.record_refutation(1, &refuting);
        // A challenge on cell 2 is UPHELD / acquitted: a zero/empty settlement records NOTHING.
        node.record_refutation(2, &crate::dispute::Settlement::default());
        assert!(node.ledger.refuted.contains(&1), "refuting settlement recorded cell 1");
        assert!(!node.ledger.refuted.contains(&2), "upheld settlement recorded nothing for cell 2");
        // advance the clock so BOTH cells age past `W` (finalized_at 1,2 would otherwise clear).
        node.apply(&carrier_for(3, &who(3)));
        node.apply(&carrier_for(4, &who(4)));
        assert_eq!(node.ledger.now(), 4);
        // now == 4, frontier = 4 − 2 = 2 ⇒ cells 1 and 2 both CLEARED the vesting cliff.
        let weight = node.finality_pom_weight();
        // (i) the REFUTED cell 1 contributes ZERO despite clearing the cliff — the dispute gate holds.
        assert_eq!(
            weight.get(&who(1)).copied().unwrap_or(0),
            0,
            "cell 1 cleared the cliff but was refuted while pending ⇒ filtered out of finality weight"
        );
        // (ii) the un-refuted sibling cell 2 clears normally — the gate does not over-exclude.
        assert!(
            weight.get(&who(2)).copied().unwrap_or(0) > 0,
            "un-refuted cell 2 aged past W ⇒ contributes finality weight"
        );
        // reward is untouched: refutation gates FINALITY weight only; the (uncleared) attribution map
        // still credits the contributor. The value-layer standing slash is the dispute mod's separate
        // rail (`apply_slashes` / soulbound::Op::Slash) — the two rails, now BOTH responsive to a
        // refutation (this build closed the finality rail; before, a slash did nothing here).
        assert!(node.ledger.pom.get(&who(1)).copied().unwrap_or(0) > 0);
        // ANTI-THEATER: drop `&& !self.ledger.refuted.contains(&c.id)` from finality_pom_weight and
        // (i) flips to non-zero ⇒ the dispute gate is load-bearing, not decoration.
    }

    #[test]
    fn refutation_is_forward_only_and_off_the_state_digest() {
        // (iii) forward-only: recording a refutation changes only FUTURE finality weight. It never
        // mutates a finalized cell, the height, or the digest of already-finalized blocks — no past
        // block un-finalizes. This also pins the design decision that `refuted` is OFF the digest.
        let w = 1u64;
        let con = Constitution { vesting_w: w, ..Constitution::default() };
        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], con);
        let who = |h: u64| format!("contrib{}", h).into_bytes();
        node.apply(&carrier_for(1, &who(1)));
        node.apply(&carrier_for(2, &who(2)));
        let height_before = node.ledger.height;
        let cells_before = node.ledger.cells.len();
        let digest_before = node.ledger.state_digest();
        node.record_refutation(1, &crate::dispute::Settlement { canceled: 3.0, ..Default::default() });
        assert_eq!(node.ledger.height, height_before, "refutation does not roll back height");
        assert_eq!(node.ledger.cells.len(), cells_before, "refutation removes no finalized cell");
        assert_eq!(
            node.ledger.state_digest(),
            digest_before,
            "`refuted` is off the digest ⇒ past finalized state is byte-identical (forward-only)"
        );
    }

    #[test]
    fn genesis_clears_nothing_so_bonded_pos_carries_finality() {
        // §2.5 bootstrap: at genesis nothing has aged past W ⇒ the finality PoM map is empty ⇒ every
        // Validator.pom sourced from it is 0 ⇒ bonded PoS carries finality from block zero, with NO
        // special-case code — it falls out of the definition.
        let con = Constitution { vesting_w: 4, ..Constitution::default() };
        let node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], con);
        let cleared = node.finality_pom_weight();
        assert!(cleared.is_empty(), "genesis: no finalized cells ⇒ nothing cleared ⇒ empty PoM map");
        // Assemble the validator set the production way: each Validator.pom sourced from the cleared
        // map (0 here). A full PoS quorum still finalizes; the PoM dimension is simply absent.
        let pom_of = |who: &[u8]| cleared.get(who).copied().unwrap_or(0) as f64;
        let all = vec![
            Validator { pom: pom_of(b"a"), ..v(0, 0.0, 100.0, 0.0) },
            Validator { pom: pom_of(b"b"), ..v(1, 0.0, 100.0, 0.0) },
        ];
        assert!(
            node.checkpoint_finalizes(&all, &all),
            "genesis ⇒ PoS-only finality: bonded stake finalizes with zero cleared PoM weight"
        );
    }

    #[test]
    fn vesting_w_zero_is_inert_bridge_equals_full_attribution() {
        // Additive default (§4 D-note): W == 0 ⇒ every finalized cell is already cleared ⇒ the
        // finality bridge is byte-identical to the full attribution map. The feature is inert until
        // governance sets W, mirroring quorum_floor_bps/horizon defaulting to 0.
        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        assert_eq!(node.constitution.vesting_w, 0, "default W is inert (0)");
        for h in 1..=5u64 {
            node.apply(&carrier_for(h, format!("c{}", h).as_bytes()));
        }
        let bridge: std::collections::BTreeMap<Vec<u8>, u64> =
            node.finality_pom_weight().into_iter().collect();
        let full: std::collections::BTreeMap<Vec<u8>, u64> =
            node.ledger.pom.iter().map(|(k, v)| (k.clone(), *v)).collect();
        assert!(!bridge.is_empty(), "sanity: five distinct contributors earned standing");
        assert_eq!(bridge, full, "W=0 ⇒ cleared score == full attribution (inert bridge)");
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

        let (a_ids, a_root, a_pom, a_tok, a_work) = a.ledger.state_digest();
        let (b_ids, b_root, b_pom, b_tok, b_work) = b.ledger.state_digest();
        assert_eq!(
            (&a_ids, &a_root, &a_pom, a_work),
            (&b_ids, &b_root, &b_pom, b_work),
            "token movement perturbed the attribution fold or the work clock"
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
        assert!(finalizes_pos_pom(&all, &all, 1, 0, false, TWO_THIRDS_BPS, 0));
    }

    // ---- cumulative-work clock (the canonical `now`) ----

    #[test]
    fn cumulative_work_clock_is_monotone_and_degrades_to_height_pre_pow() {
        // PRE-POW, WORK_PER_BLOCK == 1, so the cumulative-work clock is EXACTLY the height-clock:
        // strictly monotone and replica-deterministic. When real PoW lands, `block_work` returns
        // mined difficulty and this same clock keeps being the single `now` with no call-site
        // change — the "right interface" the height-clock degenerate case hides behind.
        assert_eq!(
            block_work(&carrier_at(1, 1, 1, b"pre-pow degenerate")),
            WORK_PER_BLOCK,
            "pre-PoW every block contributes exactly WORK_PER_BLOCK"
        );

        let mut node = Node::new(0, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        assert_eq!(node.ledger.now(), 0, "genesis clock is 0");
        let mut prev = 0u64;
        for h in 1..=5u64 {
            node.apply(&carrier_at(h, h, h as u8, format!("clock filler {}", h).as_bytes()));
            assert_eq!(node.ledger.now(), node.ledger.height, "clock must track height pre-PoW");
            assert_eq!(node.ledger.now(), h, "clock must equal blocks finalized pre-PoW");
            assert!(node.ledger.now() > prev, "clock must be strictly monotone");
            prev = node.ledger.now();
        }

        // replica-determinism: a second node finalizing the SAME blocks reads the SAME clock (also
        // folded into state_digest, so the 2-node / gaming convergence harnesses assert it too).
        let mut other = Node::new(1, vec![v(0, 0.0, 100.0, 100.0)], Constitution::default());
        for h in 1..=5u64 {
            other.apply(&carrier_at(h, h, h as u8, format!("clock filler {}", h).as_bytes()));
        }
        assert_eq!(
            node.ledger.now(),
            other.ledger.now(),
            "replicas diverged on the work clock"
        );
    }

    #[test]
    fn checkpoint_finalizes_sources_now_from_the_cumulative_work_clock() {
        // ANTI-THEATER for the wiring: `Node::checkpoint_finalizes` must feed the LIVE clock into
        // the finality decision, not a hardcoded `now`. We build a set whose outcome FLIPS with
        // `now` via differential franchise decay — an abstaining validator that is fresh at now=0
        // but fully decayed at now=horizon — then advance the clock by finalizing `horizon` blocks
        // and assert the checkpoint tracks the now=clock branch. Hardcoding now=0 flips it RED.
        let horizon = 4u64;
        let mut c = Constitution::default();
        c.horizon = horizon;
        c.decay_pos = true; // symmetric decay ⇒ the abstainer's weight fully fades at the horizon

        // A votes and stays fresh at now=horizon (heartbeat=horizon); the abstainer is fresh at
        // now=0 but fully decayed at now=horizon. Both carry pos+pom so anti-concentration holds.
        let mut a = v(0, 0.0, 1.0, 2.0);
        a.last_heartbeat = horizon;
        let abstainer = v(1, 0.0, 1.0, 2.0); // last_heartbeat = 0
        let all = vec![a.clone(), abstainer];
        let voters = vec![a];

        // The discriminator is real: the SAME set is rejected at now=0, finalizes at now=horizon.
        assert!(
            !super::finalizes(&c, &voters, &all, 0),
            "precondition: must NOT finalize at now=0 (else the test proves nothing)"
        );
        assert!(
            super::finalizes(&c, &voters, &all, horizon),
            "precondition: must finalize at now=horizon"
        );

        // A node whose clock has not advanced matches the now=0 branch (reject); after finalizing
        // `horizon` blocks the clock reads `horizon` and the SAME checkpoint finalizes — proving
        // `now` is sourced from the clock, not a constant.
        let mut node = Node::new(0, all.clone(), c);
        assert_eq!(node.ledger.now(), 0);
        assert!(
            !node.checkpoint_finalizes(&voters, &all),
            "clock at 0 ⇒ must match the now=0 branch (reject)"
        );
        for h in 1..=horizon {
            node.apply(&carrier_at(h, h, h as u8, format!("advance {}", h).as_bytes()));
        }
        assert_eq!(node.ledger.now(), horizon);
        assert_eq!(node.ledger.now(), node.ledger.height, "pre-PoW the clock == height");
        assert!(
            node.checkpoint_finalizes(&voters, &all),
            "clock at horizon ⇒ must match the now=horizon branch (finalize)"
        );
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
            !finalizes_pos_pom(&pom_only, &all, 1, 0, false, TWO_THIRDS_BPS, 0),
            "PoM unilaterally finalized — capital-orthogonality not enforced"
        );
        // but both axes participating DOES finalize.
        assert!(finalizes_pos_pom(&all, &all, 1, 0, false, TWO_THIRDS_BPS, 0));
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
            TWO_THIRDS_BPS,
            0
        ));
        // the two fast-final dimensions together finalize, PoW irrelevant.
        assert!(finalizes_pos_pom(&[pos, pom], &all, 1, 0, false, TWO_THIRDS_BPS, 0));
        // the mix really does exclude PoW.
        assert_eq!(FINALITY_MIX.pow, 0.0);
        assert_eq!(MIN_DIM_BPS, 5000);
    }

    #[test]
    fn quorum_floor_prevents_minority_finalization_safe_halt() {
        // SAFETY (quorum floor wired into the live finality gate, 2026-07-11). Two identically
        // registered validators A,B, each holding exactly 50% of BOTH dimensions (so either passes
        // anti-concentration). A stays live; B decays fully out (stale heartbeat at now=horizon,
        // symmetric decay). The vector: with Q=0 the denominator collapses to the live weight, so
        // A finalizes ALONE — a minority of the registered base finalizes a checkpoint the absent
        // half never saw. A nonzero floor anchors the denominator to the FULL registered base, so
        // A's 50% cannot clear the bar ⇒ SAFE HALT until participation returns. This is the
        // block-logistics §123 backstop; it is INERT whenever the gate hardcodes 0.
        let horizon = 100u64;
        let now = horizon;
        let mut a = v(0, 0.0, 100.0, 100.0);
        a.last_heartbeat = horizon; // fresh ⇒ retention 1.0
        let b = v(1, 0.0, 100.0, 100.0); // last_heartbeat 0 ⇒ fully decayed at now=horizon
        let all = vec![a.clone(), b];
        let voters = vec![a.clone()];

        // Q = 0 (today's default): the sole live validator finalizes a minority checkpoint — the
        // exact behavior we are making it possible to close.
        assert!(
            finalizes_pos_pom(&voters, &all, now, horizon, true, TWO_THIRDS_BPS, 0),
            "precondition: with no floor, the live minority finalizes (else the test proves nothing)"
        );

        // A high floor keeps the denominator anchored to the full registered base ⇒ 50% < bar ⇒
        // the checkpoint does NOT finalize (safe halt). This goes RED if the gate ignores the floor.
        assert!(
            !finalizes_pos_pom(&voters, &all, now, horizon, true, TWO_THIRDS_BPS, 9000),
            "quorum floor not enforced on the finality gate — minority finalization still possible"
        );

        // The floor is not a blanket halt: with FULL participation (both fresh and voting) the same
        // floor finalizes — it gates minority finalization, not honest liveness.
        let mut b_live = v(1, 0.0, 100.0, 100.0);
        b_live.last_heartbeat = horizon;
        let all_live = vec![a, b_live];
        assert!(
            finalizes_pos_pom(&all_live, &all_live, now, horizon, true, TWO_THIRDS_BPS, 9000),
            "quorum floor over-halts: full participation must still finalize under the floor"
        );
    }
}
