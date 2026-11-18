//! ChainSpec — the genesis definition ("what block zero is") as ONE single-sourced object.
//!
//! Before this, genesis lived hardcoded inside the `noesisd` driver as a `Constitution::default()`
//! (money-INERT: `pow_enforced == false`, so no mining and no JUL issuance). A network's genesis is
//! not driver code — it is the spec every honest node must agree on to compare digests at all
//! (`noesisd`'s seed and joiner MUST boot identically). This module lifts that agreement into a
//! `ChainSpec` value so a devnet, a seed, a joiner, and eventually a stranger all read ONE block-zero
//! definition, and it turns the RATIFIED M3 economics ON at genesis so the chain actually mines and
//! issues JUL from block zero (`internal/DECISIONS-M3-money-2026-07-15.md`).
//!
//! HONEST GENESIS (unchanged principle): the ledger starts EMPTY. PoM standing is EARNED by finalized
//! contribution, never pre-minted (runtime §2.5). Genesis seeds NO PoM and NO tokens — only a small
//! BONDED PoS set to carry finality from block zero (PoM is empty until cells clear the vesting
//! window; `W = 0` at launch). The finality RULE is PoS+PoM with PoW EXCLUDED (`FINALITY_MIX`).
//!
//! HONEST SCOPE: `dev()` is a LOCAL devnet spec. `genesis_bits` is a LOW placeholder chosen for
//! instant CPU mining, NOT a measured mainnet difficulty (that is `genesis_bits` measured-at-build,
//! ⚑ per the loop plan). The Moore's-law decay coefficient (`a_estim`) is NOT yet a Constitution
//! field, so JUL here issues flat-proportional to work — the decay wiring is the deferred follow tick
//! (`jul::reward_with_decay` is built and tested but not yet wired into the mint site).

use crate::consensus::Validator;
use crate::runtime::{header_digest, Block, Constitution, Node, TokenTx};
use crate::{Cell, Script};

/// The genesis definition every honest node agrees on. Two nodes that boot the same `ChainSpec`
/// produce comparable state digests; disagreeing on any field forks block zero.
#[derive(Clone, Debug)]
pub struct ChainSpec {
    /// The block-zero constitution: consensus mix, thresholds, the ratified money economics, and (since
    /// 2026-11-18) the two values that used to sit beside it here: the network id
    /// ([`Constitution::chain_id`], read through [`Self::chain_id`]) and the compact PoW target every
    /// block must claim ([`Constitution::pow_bits`], read through [`Self::genesis_bits`]). They moved
    /// into the rulebook because `validate_block` sees only the constitution: as spec fields, nothing
    /// in block validation read either, so a block named no network and its seal named its own
    /// difficulty. One copy each, so the bits a node mines at and the bits it requires cannot differ.
    pub constitution: Constitution,
    /// The bonded genesis validators, each paired with its soulbound contributor key. `pos`/
    /// `staked_balance` give bonded finality weight at genesis; `pom` starts 0 and is sourced live
    /// from the cleared-score bridge as each contributor's finalized work ages in.
    pub validators: Vec<(Vec<u8>, Validator)>,
    /// Token cells present at block zero, in `Ledger.token_cells` (never `cells`, so they carry no PoM
    /// standing and no finality weight). Today: the soulbound identity issuer's authority cell for
    /// [`crate::identity_mint::COLLECTION`], the only way a soulbound collection gets an authority.
    pub genesis_token_cells: Vec<Cell>,
    /// Who the block reward pays. Consensus constructs the reward AMOUNT (`apply_transition`) and never
    /// checks the recipient (`Block::coinbase` is producer-named), so this is production policy written
    /// into the spec. `false`: the first bonded validator's contributor key ([`Self::placeholder_reward`],
    /// a placeholder nobody spends; dev). `true`: the testnet faucet, one one-time key per block derived
    /// from the faucet seed ([`crate::faucet`]), so a node producing this chain must hold that seed
    /// (`--serve-api` refuses to start without it). JUL only: standing and finality never read it.
    pub reward_to_faucet: bool,
}

/// The testnet identity issuer's authority lock root at generation 0 (a public key, hex
/// `5407ffa5...27d6c7e`). Its seed was generated 2026-11-17 on the issuer's machine and is kept outside
/// the repository (`identity-mint root --seed <file>` prints this value from it); the dev spec uses the
/// public [`crate::identity_mint::DEV_ISSUER_SEED`] instead.
pub const TESTNET_IDENTITY_AUTHORITY_ROOT: [u8; 32] = [
    0x54, 0x07, 0xff, 0xa5, 0x70, 0x96, 0x9b, 0x73, 0x9c, 0x0a, 0x63, 0x13, 0x11, 0x59, 0x14, 0x9f, //
    0x63, 0x3a, 0x07, 0xfc, 0xd8, 0xd7, 0xae, 0x0a, 0x1c, 0xd3, 0x15, 0x2a, 0xd2, 0x7d, 0x6c, 0x7e,
];

/// The testnet's key-change time lock (`Constitution::key_change_delay_blocks`): a rotation or a
/// recovery request takes effect 101 blocks after the block that carries it, so the other key has 100
/// blocks in which to cancel it (the cold key a rotation; the hot key a recovery, unless the cold key has
/// frozen the identity). Why 100: testnet blocks are made on demand, one per accepted contribution, at a
/// few to a few dozen a day, so 100 blocks of ordinary traffic is days to weeks, time for an owner to see
/// the page's warning and object; and an owner is never stuck waiting on others, because anyone (the
/// owner too, from a second wallet) can make blocks. HONEST FLOOR: the same is true of a thief: about
/// 100 cheap submissions make 100 blocks in minutes, so on the testnet the delay bounds chain activity,
/// not wall-clock time. Against a stolen hot key that does not matter, because the cold key's freeze is
/// immediate; against a stolen cold key the hot key's window is the only defense, and it is this short in
/// wall-clock time on a quiet chain. A wall-clock floor needs block timestamps under clock enforcement
/// (the testnet's existing blocks carry none, so turning it on would refuse them on replay) or a
/// producer-side block rate; both are open.
pub const TESTNET_KEY_CHANGE_DELAY_BLOCKS: u64 = 100;

impl ChainSpec {
    /// The network id ([`Constitution::chain_id`]): every block carries it and the rulebook refuses one
    /// whose id differs, so a devnet, a testnet and mainnet never accept each other's blocks.
    pub fn chain_id(&self) -> u64 {
        self.constitution.chain_id
    }

    /// The compact PoW target every block on this chain must claim and meet ([`Constitution::pow_bits`]).
    /// Fixed for every height: no retarget is wired. LOW for a devnet ⇒ near-instant CPU mining; a
    /// measured value is the mainnet ⚑.
    pub fn genesis_bits(&self) -> u32 {
        self.constitution.pow_bits
    }

    /// The local devnet spec: the ratified M3 economics turned ON (PoW enforced ⇒ real mined
    /// difficulty ⇒ JUL issues from block zero), an easy fixed difficulty for instant local mining,
    /// and a small bonded PoS set (alice / bob / carol). Everything else is the honest default:
    /// `vesting_w = 0`, empty `coinbase_split` (ratified `infra_bps = NONE`), `submission_deposit = 0`,
    /// `clock_enforced = false`.
    pub fn dev() -> Self {
        // Turn the money layer ON: `pow_enforced` ⇒ real mined difficulty flows through `block_work`,
        // so the coinbase mints `reward_for_work(block_work, jul)` from block zero (Lever A live). A
        // FINITE `work_clock_ceiling` is a genesis-admission precondition of `pow_enforced` (`Node::new`
        // asserts it — an infinite ceiling under real difficulty ships the vesting-collapse attack);
        // generous here so it never clamps a LOW-difficulty devnet block. Everything else stays default.
        let constitution = Constitution {
            pow_enforced: true,
            work_clock_ceiling: 1 << 40,
            chain_id: 0xde0, // "dev"
            // 0x2000ffff ⇒ target 0x00ffff·2^232 ≈ 2^248 ⇒ about 1 hash in 256 meets it ⇒ instant
            // local mining. A valid compact the decoder accepts (see pow_arithmetic tests).
            pow_bits: 0x2000_ffff,
            // Identity keys (A1/A7) on, with a short key-change delay so a local devnet can walk a rotation
            // or a recovery through in a few blocks. The testnet's delay is TESTNET_KEY_CHANGE_DELAY_BLOCKS.
            key_rotation: true,
            key_change_delay_blocks: 3,
            ..Default::default()
        };

        let keys: Vec<Vec<u8>> = vec![b"alice".to_vec(), b"bob".to_vec(), b"carol".to_vec()];
        let validators = keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                (
                    k.clone(),
                    Validator { id: i as u64, pow: 0.0, pos: 1000.0, pom: 0.0, last_heartbeat: 0, staked_balance: 1000.0 },
                )
            })
            .collect();

        ChainSpec {
            constitution,
            validators,
            genesis_token_cells: vec![crate::identity_mint::authority_cell(
                crate::identity_mint::COLLECTION,
                crate::identity_mint::authority_root(&crate::identity_mint::DEV_ISSUER_SEED, 0),
                0,
            )],
            reward_to_faucet: false, // the dev reward goes to the placeholder, unchanged
        }
    }

    /// The PUBLIC TESTNET spec: the first genesis a stranger boots. Identical honest structure to
    /// [`dev`](Self::dev) — money layer ON, empty ledger, PoM earned not pre-minted, a small bonded PoS
    /// set carrying finality from block zero — but with a DISTINCT `chain_id` (every block carries it and
    /// the rulebook refuses another network's, so testnet blocks never replay onto a devnet or mainnet) and
    /// a HARDER-than-dev `genesis_bits` (every seal must claim it) so a public node is not a free-for-all
    /// where a few hundred hashes seal a block.
    ///
    /// HONEST SCOPE (loud, do NOT round up): `genesis_bits` here is a TESTNET PLACEHOLDER — 2^8 harder
    /// than dev's so a modest always-on VM must actually grind for a block (about 65,536 hashes on
    /// average), but still CPU-mineable. It is NOT the measured mainnet difficulty; that
    /// remains the standing ⚑ (`genesis_bits` measured-at-build on real hardware, loop-plan M2/⚑).
    /// Everything else is the honest launch default: `vesting_w = 0`, empty `coinbase_split`
    /// (ratified `infra_bps = NONE`), `submission_deposit = 0`, `clock_enforced = false` — the testnet
    /// ships at the FLOOR, never claiming the moat.
    ///
    /// CONTROL BINDING ON, REWARD TO THE FAUCET (Will 2026-11-17, package C). The node's spend check
    /// requires every spent input to carry its owner's Lamport signature (`control_binding: true`, see
    /// `Constitution::control_binding`), and each block reward pays the faucet (`reward_to_faucet`), so a
    /// newcomer can get test JUL that only its key holder can move. Only the node's switch is on: the
    /// four on-VM binding switches in `onchain/*` stay `false` until the go-live milestone (Will-gated).
    ///
    /// JUL ON TESTNET (the resolved design — dial the PARAMETER, never the MECHANISM): PoW stays fully
    /// in consensus (`pow_enforced = true`) so the testnet tests the real PoW / issuance security model, and JUL mints
    /// from work through the exact same path as mainnet (`block_work → reward_for_work`). The money layer
    /// is genuinely exercised. But because difficulty is LOW, the energy behind a block is about 65,536
    /// CPU hashes on average (no GPU, no meaningful electricity) — so testnet JUL is a
    /// functional-but-WORTHLESS test token by construction (distinct `chain_id` + trivial energy + it is
    /// a testnet). The energy-peg is an EMERGENT property of HIGH (mainnet) difficulty, not a coded flag;
    /// lowering difficulty makes JUL a test token with ZERO consensus change. NEVER set
    /// `pow_enforced = false` to "save power" — that removes PoW from consensus, breaks the
    /// genesis-admission invariants, and stops the testnet testing the real thing. Honest scope (do NOT
    /// round up): low difficulty ⇒ low Sybil-cost / cheap blocks, expected FOR a testnet. AND, SEPARATELY,
    /// the PoM CONTRIBUTION FRANCHISE ships at the v0 novelty floor (`pom_scores_with_similarity_floor_q16`):
    /// it rewards first-appearance coverage and zeroes only near-DUPLICATES, so novel-but-worthless content
    /// still earns standing, and it is farmable at ~zero cost while `submission_deposit = 0` and the on-VM
    /// bindings are `false`. So testnet PoM standing is NOT yet a Sybil-resistant signal — the deployed path
    /// tests the FLOOR (identity + near-dup + PoW), never the value moat (learned v(S), 🔬 open). A public
    /// PERMISSIONLESS testnet therefore wants the rest of the go-live knot (`submission_deposit > 0`, which
    /// control binding now permits but `pow_enforced` still blocks until the header binds bonds, + a
    /// per-identity standing cap or an allowlist) first. Audit:
    /// `docs/SYBIL-SURFACE-deployed-franchise-2026-07-19.md`.
    pub fn testnet() -> Self {
        // genesis_unscored: the chain's first contribution earns no standing (Will 2026-11-17, "I just
        // didn't want to take any money from genesis"); see `Constitution::genesis_unscored`.
        let constitution = Constitution {
            pow_enforced: true,
            work_clock_ceiling: 1 << 40,
            genesis_unscored: true,
            control_binding: true, // every spend carries its owner's signature (package C, 2026-11-17)
            chain_id: 0x_7e57,     // "teST" — distinct from dev (0xde0); blocks carry it, so they never cross
            // Exponent 0x1f (one below dev's 0x20) ⇒ target 0x00ffff·2^224 ≈ 2^240, 2^8 harder than dev's
            // ≈ 2^248: about 1 hash in 65,536 meets it, against 1 in 256 on dev. Same mantissa shape as dev
            // (0x00ffff) ⇒ a compact the decoder accepts. TESTNET PLACEHOLDER — the measured mainnet value
            // is the standing ⚑.
            pow_bits: 0x1f00_ffff,
            // Identity keys (wallet decisions A1 and A7, 2026-11-18): a rotation behind a time lock the cold
            // key can cancel, a cold-key recovery behind the same time lock the hot key can cancel unless the
            // identity is frozen, and the cold key's immediate freeze.
            key_rotation: true,
            key_change_delay_blocks: TESTNET_KEY_CHANGE_DELAY_BLOCKS,
            ..Default::default()
        };

        let keys: Vec<Vec<u8>> = vec![b"alice".to_vec(), b"bob".to_vec(), b"carol".to_vec()];
        let validators = keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                (
                    k.clone(),
                    Validator { id: i as u64, pow: 0.0, pos: 1000.0, pom: 0.0, last_heartbeat: 0, staked_balance: 1000.0 },
                )
            })
            .collect();

        ChainSpec {
            constitution,
            validators,
            genesis_token_cells: vec![crate::identity_mint::authority_cell(
                crate::identity_mint::COLLECTION,
                TESTNET_IDENTITY_AUTHORITY_ROOT,
                0,
            )],
            reward_to_faucet: true, // the block reward funds the faucet (testnet step 3)
        }
    }

    /// Boot the genesis node from this spec: an EMPTY ledger with the spec's constitution and bonded
    /// validator set. Returns the node plus the ordered contributor keys (so a driver can source each
    /// validator's live PoM from the cleared-score bridge). Every mode (devnet / seed / joiner) calls
    /// THIS — genesis agreement is what makes their digests comparable.
    pub fn genesis_node(&self) -> (Node, Vec<Vec<u8>>) {
        let validators: Vec<Validator> = self.validators.iter().map(|(_, v)| v.clone()).collect();
        let keys: Vec<Vec<u8>> = self.validators.iter().map(|(k, _)| k.clone()).collect();
        let mut node = Node::new(0, validators, self.constitution.clone());
        node.ledger.token_cells.extend(self.genesis_token_cells.iter().cloned());
        (node, keys)
    }

    /// Produce ONE block from whatever is currently in `node`'s mempool, through the FULL proven
    /// pipeline — propose → name a coinbase → mine → validate → finality-GATE → apply — and return the
    /// finalized block (or `None` if the round produced nothing valid/final). This is the SINGLE
    /// per-block engine shared by every driver: the scripted devnet, the seed, and the live API's
    /// submit path all call THIS, so an interactively-submitted contribution travels the exact same
    /// path the tested chain does (no second, drifting pipeline).
    ///
    /// An empty mempool yields `None` (a block with no cells is invalid by rule — `EmptyBlock`). On a
    /// validation or finality miss the mempool is cleared and `None` returned (the round is dropped,
    /// never half-applied). The caller must `submit` contributions before calling.
    pub fn produce_block(&self, node: &mut Node) -> Option<Block> {
        self.produce_block_with(node, Vec::new())
    }

    /// [`produce_block`](Self::produce_block) carrying `token_txs` in the block (empty ⇒ exactly
    /// `produce_block`). A token movement never makes a block by itself: the block still needs a
    /// contribution (`EmptyBlock`), and the txs face the same block gate as any other. The reward pays
    /// [`placeholder_reward`](Self::placeholder_reward); a node producing a `reward_to_faucet` chain for
    /// real names the faucet through [`produce_block_paying`](Self::produce_block_paying) (`rpc.rs` does).
    pub fn produce_block_with(&self, node: &mut Node, token_txs: Vec<TokenTx>) -> Option<Block> {
        self.produce_block_paying(node, token_txs, self.placeholder_reward())
    }

    /// The placeholder reward recipient: the first bonded validator's contributor key (`alice` on dev
    /// and testnet). A label, not a key anyone holds, so with control binding on nobody can spend it.
    pub fn placeholder_reward(&self) -> Script {
        Script { code_hash: [0u8; 32], args: self.validators.first().map(|(k, _)| k.clone()).unwrap_or_default() }
    }

    /// [`produce_block_with`](Self::produce_block_with), paying the block reward to `reward_to`. Consensus
    /// fixes the amount, never the recipient, so this changes who holds the new JUL and nothing else.
    pub fn produce_block_paying(&self, node: &mut Node, token_txs: Vec<TokenTx>, reward_to: Script) -> Option<Block> {
        if node.mempool.is_empty() {
            return None; // no contributions ⇒ nothing to finalize (an empty block is invalid)
        }
        let block = mine(node.propose().with_token_txs(token_txs).with_coinbase(reward_to), self.genesis_bits());
        let accepted = self.accept_block(node, &block);
        node.clear_mempool();
        accepted.ok().map(|()| block)
    }

    /// THE FINALITY GATE: rebuild the bonded validator set, sourcing each one's live PoM weight from
    /// the cleared-score bridge (empty at genesis ⇒ pom 0 ⇒ bonded PoS carries finality). Single
    /// proposer ⇒ every honest validator votes for the one valid proposal. Read on the state BEFORE the
    /// block applies, by the producer and by a follower alike.
    fn finality_gate(&self, node: &Node) -> bool {
        let fpw = node.finality_pom_weight();
        let validators: Vec<Validator> = self
            .validators
            .iter()
            .map(|(k, v)| {
                let mut v = v.clone();
                v.pom = fpw.get(k).map(|w| *w as f64).unwrap_or(0.0);
                v
            })
            .collect();
        node.checkpoint_finalizes(&validators, &validators)
    }

    /// Accept one block onto `node`: the rulebook ([`validate_block`](crate::runtime::validate_block):
    /// height, order, token conservation and spend signatures under control binding, the PoW seal,
    /// clock, bonds), then the finality gate, then apply. The producer runs this on the block it just
    /// mined ([`produce_block_paying`](Self::produce_block_paying)); a follower runs it on every block it
    /// pulls (`rpc::ServerState::follow_block`), so a follower applies nothing on the producer's word.
    /// On any refusal `node` is unchanged and the reason is returned.
    pub fn accept_block(&self, node: &mut Node, block: &Block) -> Result<(), String> {
        use crate::runtime::Violation;
        crate::runtime::validate_block(&node.ledger, block, &node.constitution).map_err(|v| {
            let h = block.height;
            match v {
                Violation::WrongChain { expected, got } => {
                    format!("block {h} fails the rulebook: WrongChain (it was made for chain 0x{got:x}; this chain is 0x{expected:x})")
                }
                Violation::PowBitsWrong { expected, got } => format!(
                    "block {h} fails the rulebook: PowBitsWrong (its seal claims bits {got:#010x}; this chain requires {expected:#010x})"
                ),
                v => format!("block {h} fails the rulebook: {v:?}"),
            }
        })?;
        if !self.finality_gate(node) {
            return Err(format!("block {} does not finalize", block.height));
        }
        node.apply(block);
        Ok(())
    }
}

/// Mine a proposed block against `bits`: grind the seal's nonce until the block's `header_digest`
/// meets the compact target — the exact predicate `validate_block`'s `pow_check` enforces under
/// `pow_enforced`. Deterministic: the same block content yields the same lowest valid nonce on every
/// node, so a seed and a joiner agree on the sealed block byte-for-byte. Because the digest folds in
/// the nonce, each grind step changes it (real proof-of-work, not a stamp). At the devnet's easy
/// difficulty this returns within a few hundred iterations on average.
///
/// Panics only on a malformed `bits` (a caller bug — a `ChainSpec` always carries a valid compact).
pub fn mine(block: Block, bits: u32) -> Block {
    let target = noesis_core::pow::compact_to_target(bits).expect("ChainSpec carries valid genesis bits");
    let mut b = block.with_pow(bits, 0);
    let mut nonce: u64 = 0;
    loop {
        if let Some(seal) = b.pow.as_mut() {
            seal.nonce = nonce;
        }
        if header_digest(&b) <= target {
            return b;
        }
        nonce = nonce.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_spec_boots_pow_on_with_a_finite_ceiling() {
        let spec = ChainSpec::dev();
        assert!(spec.constitution.pow_enforced, "dev spec turns the money layer on");
        assert!(spec.constitution.work_clock_ceiling < u64::MAX, "pow_enforced requires a finite ceiling");
        assert!(spec.constitution.coinbase_split.is_empty(), "ratified infra_bps = NONE ⇒ empty split");
        // Node::new asserts the genesis-admission preconditions — this must NOT panic.
        let (node, keys) = spec.genesis_node();
        assert_eq!(keys, vec![b"alice".to_vec(), b"bob".to_vec(), b"carol".to_vec()]);
        assert_eq!(node.ledger.height, 0, "genesis ledger is empty");
        assert!(node.ledger.pom.is_empty(), "no pre-minted PoM standing");
    }

    #[test]
    fn testnet_spec_boots_pow_on_distinct_from_dev_and_mines() {
        use crate::commit_order::Committed;
        use crate::{Cell, Script};

        let dev = ChainSpec::dev();
        let net = ChainSpec::testnet();

        // Distinct network identity ⇒ blocks can never replay across networks.
        assert_ne!(net.chain_id(), dev.chain_id(), "testnet must not share the devnet chain_id");
        // Harder than dev's near-maximum target (smaller target = more work), still a valid compact.
        let dev_target = noesis_core::pow::compact_to_target(dev.genesis_bits()).unwrap();
        let net_target = noesis_core::pow::compact_to_target(net.genesis_bits()).unwrap();
        assert!(net_target < dev_target, "testnet difficulty must be harder than dev's free-for-all");

        // Same honest-genesis invariants as dev: money on, finite ceiling, empty ledger, no pre-mint.
        assert!(net.constitution.pow_enforced, "testnet turns the money layer on");
        assert!(net.constitution.work_clock_ceiling < u64::MAX, "pow_enforced requires a finite ceiling");
        assert!(net.constitution.coinbase_split.is_empty(), "ratified infra_bps = NONE ⇒ empty split");
        let (mut node, keys) = net.genesis_node(); // Node::new asserts genesis admission — must not panic
        assert_eq!(node.ledger.height, 0, "genesis ledger is empty");
        assert!(node.ledger.pom.is_empty(), "no pre-minted PoM standing");
        assert_eq!(keys, vec![b"alice".to_vec(), b"bob".to_vec(), b"carol".to_vec()]);

        // A real contribution mines + validates + finalizes + issues JUL against the testnet difficulty.
        node.submit(
            Cell {
                id: 1,
                lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
                type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
                parent: None,
                timestamp: 1,
                data: b"the quick brown fox jumps high".to_vec(),
            },
            Committed { height: 1, secret: [11u8; 32] },
        );
        let block = net.produce_block(&mut node).expect("a contribution finalizes a testnet block");
        assert_eq!(block.height, 1);
        assert!(node.ledger.jul_supply.issued() > 0, "mined work issues JUL from block zero on testnet too");
    }

    #[test]
    fn dev_and_testnet_genesis_seed_the_identity_issuer_authority() {
        use crate::identity_mint::{authority_root, COLLECTION, DEV_ISSUER_SEED};
        use crate::tokens::soulbound::{self, Record};
        for (spec, root) in [
            (ChainSpec::dev(), authority_root(&DEV_ISSUER_SEED, 0)),
            (ChainSpec::testnet(), TESTNET_IDENTITY_AUTHORITY_ROOT),
        ] {
            let (node, _) = spec.genesis_node();
            let sb: Vec<&Cell> = node
                .ledger
                .token_cells
                .iter()
                .filter(|c| c.type_script.code_hash == soulbound::CODE_HASH)
                .collect();
            assert_eq!(sb.len(), 1, "chain 0x{:x}: exactly one soulbound cell at genesis", spec.chain_id());
            assert_eq!(sb[0].type_script.args, COLLECTION.to_vec());
            assert_eq!(soulbound::decode(&sb[0].data), Some(Record::Authority { generation: 0 }));
            assert_eq!(sb[0].lock.args, root.to_vec(), "chain 0x{:x}: the issuer's generation-0 key", spec.chain_id());
            // the seeded cell is token state only: no contribution, no standing, no finality weight
            assert!(node.ledger.cells.is_empty() && node.ledger.pom.is_empty());
            assert!(node.finality_pom_weight().is_empty());
        }
        assert_ne!(
            TESTNET_IDENTITY_AUTHORITY_ROOT,
            authority_root(&DEV_ISSUER_SEED, 0),
            "the testnet authority is not the public dev key"
        );
    }

    /// What control binding ON changes, through the real block gate: a JUL cell locked by a one-time
    /// key, moved by a block with NO signature. The dev spec (binding off) still accepts it, as before;
    /// the testnet spec refuses it; the same move signed by the cell's key passes on both.
    #[test]
    fn testnet_refuses_an_unsigned_spend_that_dev_accepts() {
        use crate::commit_order::Committed;
        use crate::runtime::TokenStandard;
        use noesis_core::lamport;

        let owner_key = [0x5Au8; 32];
        let jul = Script { code_hash: crate::jul::JUL_CODE_HASH, args: crate::jul::JUL_ISSUER.to_vec() };
        let jul_cell = |id: u64, lock: Vec<u8>| Cell {
            id,
            lock: Script { code_hash: [0u8; 32], args: lock },
            type_script: jul.clone(),
            parent: None,
            timestamp: 0,
            data: crate::tokens::fungible::encode(7 * crate::jul::JUL_BASE_UNITS),
        };
        let owned = jul_cell(42, lamport::keygen_root(&owner_key).to_vec());
        let unsigned = TokenTx {
            standard: TokenStandard::Fungible,
            code_hash: jul.code_hash,
            args: jul.args.clone(),
            inputs: vec![owned.clone()],
            outputs: vec![jul_cell(43, vec![0xB0; 32])],
            auths: vec![],
        };
        let signed = TokenTx { auths: vec![lamport::sign(&owner_key, &unsigned.digest())], ..unsigned.clone() };
        // The testnet's rules at dev's easy bits keep this test instant (a block's seal must claim its
        // chain's bits, so the bits are set on the spec, not only passed to `mine`). Control binding is
        // the subject here, and it does not depend on difficulty.
        let mut fast_testnet = ChainSpec::testnet();
        fast_testnet.constitution.pow_bits = ChainSpec::dev().genesis_bits();
        for (spec, binding_on) in [(ChainSpec::dev(), false), (fast_testnet, true)] {
            let fast_bits = spec.genesis_bits();
            let (mut node, _) = spec.genesis_node();
            node.ledger.token_cells.push(owned.clone());
            node.submit(
                Cell {
                    id: 1,
                    lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
                    type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
                    parent: None,
                    timestamp: 1,
                    data: b"a carrier note about salt marsh tides".to_vec(),
                },
                Committed { height: 1, secret: [11u8; 32] },
            );
            let carrying = |tx: &TokenTx| mine(node.propose().with_token_txs(vec![tx.clone()]), fast_bits);
            assert_eq!(
                node.validate(&carrying(&unsigned)),
                !binding_on,
                "chain 0x{:x}: an unsigned spend is accepted iff control binding is off",
                spec.chain_id()
            );
            assert!(node.validate(&carrying(&signed)), "chain 0x{:x}: the owner's signature authorizes the spend", spec.chain_id());
        }
    }

    /// One contribution waiting in `node`'s mempool at height 1 (the carrier every gap test mines).
    fn one_contribution(node: &mut Node) {
        node.submit(
            Cell {
                id: 1,
                lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
                type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
                parent: None,
                timestamp: 1,
                data: b"a note on how fog banks form over cold water".to_vec(),
            },
            crate::commit_order::Committed { height: 1, secret: [11u8; 32] },
        );
    }

    /// Gap 1 (2026-11-18): a seal that claims EASIER bits than the chain spec, valid for those bits, is
    /// refused on the shared acceptance path (`accept_block`, which the producer runs on the block it mined
    /// and a follower on every block it pulls), and the node is unchanged. Dev: one exponent easier than
    /// its own bits. Testnet: dev's bits, the shortcut the test helpers used to take. The same dev block
    /// mined at the spec's own bits is accepted.
    #[test]
    fn a_seal_claiming_easier_bits_than_the_spec_is_refused_even_when_valid_for_them() {
        for (spec, easier) in [(ChainSpec::dev(), 0x2100_ffffu32), (ChainSpec::testnet(), ChainSpec::dev().genesis_bits())] {
            let spec_target = noesis_core::pow::compact_to_target(spec.genesis_bits()).unwrap();
            let easy_target = noesis_core::pow::compact_to_target(easier).unwrap();
            assert!(easy_target > spec_target, "test premise: the claimed bits are easier than the spec's");
            let (mut node, _) = spec.genesis_node();
            one_contribution(&mut node);
            let cheap = mine(node.propose().with_coinbase(spec.placeholder_reward()), easier);
            assert!(header_digest(&cheap) <= easy_target, "the seal is valid for the bits it claims");
            let before = node.ledger.state_digest();
            let refused = spec.accept_block(&mut node, &cheap).expect_err("a seal at easier bits than the spec is refused");
            assert!(
                refused.contains("PowBitsWrong") && refused.contains(&format!("{easier:#010x}")),
                "chain 0x{:x}: {refused}",
                spec.chain_id()
            );
            assert_eq!(
                crate::runtime::validate_block(&node.ledger, &cheap, &node.constitution),
                Err(crate::runtime::Violation::PowBitsWrong { expected: spec.genesis_bits(), got: easier }),
                "the rulebook itself refuses it, so every path that validates (replay, sync, follow) does"
            );
            assert_eq!((node.ledger.height, node.ledger.state_digest()), (0, before), "nothing applied");
        }
        let spec = ChainSpec::dev();
        let (mut node, _) = spec.genesis_node();
        one_contribution(&mut node);
        let honest = mine(node.propose().with_coinbase(spec.placeholder_reward()), spec.genesis_bits());
        assert_eq!(spec.accept_block(&mut node, &honest), Ok(()), "the spec's own bits are accepted");
    }

    /// Gap 2 (2026-11-18): a block mined for one chain is refused on another chain with the SAME rules,
    /// genesis and difficulty. Only the chain id tells them apart, so the block itself must carry it.
    #[test]
    fn a_block_mined_for_one_chain_is_refused_on_another_with_the_same_rules() {
        use crate::runtime::{validate_block, Violation};
        let a = ChainSpec::dev();
        let mut b = ChainSpec::dev();
        b.constitution.chain_id = 0xb0b;
        let (mut on_a, _) = a.genesis_node();
        one_contribution(&mut on_a);
        let block = mine(on_a.propose().with_coinbase(a.placeholder_reward()), a.genesis_bits());
        assert_eq!(block.chain_id, 0xde0, "the producer stamps its own chain id");
        let (mut on_b, _) = b.genesis_node();
        let before = on_b.ledger.state_digest();
        let refused = b.accept_block(&mut on_b, &block).expect_err("a block mined for chain A is refused on chain B");
        assert!(refused.contains("WrongChain") && refused.contains("0xde0") && refused.contains("0xb0b"), "{refused}");
        assert_eq!((on_b.ledger.height, on_b.ledger.state_digest()), (0, before), "nothing applied");
        // Relabel it for chain B without re-mining: the id is in the header digest, so the seal no longer
        // proves this block. At dev's bits 1 hash in 256 meets the target, so whether the relabeled digest
        // misses it is a property of these exact bytes; it is asserted here, not left to chance.
        let relabeled = block.clone().with_chain_id(b.chain_id());
        assert_ne!(header_digest(&relabeled), header_digest(&block), "the chain id is bound into the header digest");
        assert_eq!(validate_block(&on_b.ledger, &relabeled, &on_b.constitution), Err(Violation::PowUnmet));
        assert_eq!(a.accept_block(&mut on_a, &block), Ok(()), "its own chain accepts it");
    }

    #[test]
    fn mine_yields_a_block_that_meets_target_and_validates() {
        use crate::commit_order::Committed;
        use crate::{Cell, Script};

        let spec = ChainSpec::dev();
        let (mut node, _keys) = spec.genesis_node();
        // A realistic non-empty block (validate rejects EmptyBlock by rule): one contribution at height 1.
        let cell = Cell {
            id: 1,
            lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
            type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
            parent: None,
            timestamp: 1,
            data: b"the quick brown fox jumps high".to_vec(),
        };
        node.submit(cell, Committed { height: 1, secret: [11u8; 32] });
        let raw = node.propose();
        // Under pow_enforced, an UN-mined block carries no seal ⇒ validation rejects it (PowMissing).
        assert!(!node.validate(&raw), "an un-mined block must fail pow_check under pow_enforced");
        let mined = mine(raw, spec.genesis_bits());
        let target = noesis_core::pow::compact_to_target(spec.genesis_bits()).unwrap();
        assert!(header_digest(&mined) <= target, "the mined header must meet the target");
        assert!(node.validate(&mined), "the mined block must validate under the dev constitution");
    }

    #[test]
    fn produce_block_finalizes_a_submitted_contribution_and_issues_jul() {
        use crate::commit_order::Committed;
        use crate::{Cell, Script};

        let spec = ChainSpec::dev();
        let (mut node, _keys) = spec.genesis_node();

        // Empty mempool ⇒ nothing to finalize.
        assert!(spec.produce_block(&mut node).is_none(), "an empty mempool produces no block");

        // Submit one real contribution, then run the single per-block engine.
        node.submit(
            Cell {
                id: 1,
                lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
                type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
                parent: None,
                timestamp: 1,
                data: b"the quick brown fox jumps high".to_vec(),
            },
            Committed { height: 1, secret: [11u8; 32] },
        );
        let block = spec.produce_block(&mut node).expect("a submitted contribution finalizes a block");
        assert_eq!(block.height, 1);
        assert_eq!(node.ledger.height, 1, "the block was applied");
        assert!(node.ledger.jul_supply.issued() > 0, "mined work issued JUL from block zero");
        assert!(node.mempool.is_empty(), "the mempool is cleared after production");
    }
}
