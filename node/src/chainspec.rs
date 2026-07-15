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
use crate::runtime::{header_digest, Block, Constitution, Node};
use crate::Script;

/// The genesis definition every honest node agrees on. Two nodes that boot the same `ChainSpec`
/// produce comparable state digests; disagreeing on any field forks block zero.
#[derive(Clone, Debug)]
pub struct ChainSpec {
    /// Network identifier — distinguishes a devnet from a testnet from mainnet so their blocks can
    /// never be replayed across networks.
    pub chain_id: u64,
    /// The block-zero constitution: consensus mix, thresholds, and the ratified money economics.
    pub constitution: Constitution,
    /// The compact PoW target every block is mined against (Phase-1 fixed difficulty, no retarget).
    /// LOW for a devnet ⇒ near-instant CPU mining; a measured value is the mainnet ⚑.
    pub genesis_bits: u32,
    /// The bonded genesis validators, each paired with its soulbound contributor key. `pos`/
    /// `staked_balance` give bonded finality weight at genesis; `pom` starts 0 and is sourced live
    /// from the cleared-score bridge as each contributor's finalized work ages in.
    pub validators: Vec<(Vec<u8>, Validator)>,
}

impl ChainSpec {
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
        let constitution = Constitution { pow_enforced: true, work_clock_ceiling: 1 << 40, ..Default::default() };

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
            chain_id: 0xde0, // "dev"
            constitution,
            // 0x2000ffff ⇒ target ≈ 2^248 (near the maximum) ⇒ almost every hash meets it ⇒ instant
            // local mining. A valid compact the decoder accepts (see pow_arithmetic tests).
            genesis_bits: 0x2000_ffff,
            validators,
        }
    }

    /// Boot the genesis node from this spec: an EMPTY ledger with the spec's constitution and bonded
    /// validator set. Returns the node plus the ordered contributor keys (so a driver can source each
    /// validator's live PoM from the cleared-score bridge). Every mode (devnet / seed / joiner) calls
    /// THIS — genesis agreement is what makes their digests comparable.
    pub fn genesis_node(&self) -> (Node, Vec<Vec<u8>>) {
        let validators: Vec<Validator> = self.validators.iter().map(|(_, v)| v.clone()).collect();
        let keys: Vec<Vec<u8>> = self.validators.iter().map(|(k, _)| k.clone()).collect();
        (Node::new(0, validators, self.constitution.clone()), keys)
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
        if node.mempool.is_empty() {
            return None; // no contributions ⇒ nothing to finalize (an empty block is invalid)
        }
        // The producer (block reward recipient) is the first bonded validator's contributor key.
        let producer = Script {
            code_hash: [0u8; 32],
            args: self.validators.first().map(|(k, _)| k.clone()).unwrap_or_default(),
        };
        let block = mine(node.propose().with_coinbase(producer), self.genesis_bits);
        if !node.validate(&block) {
            node.clear_mempool();
            return None;
        }
        // THE FINALITY GATE: rebuild the bonded validator set, sourcing each one's live PoM weight from
        // the cleared-score bridge (empty at genesis ⇒ pom 0 ⇒ bonded PoS carries finality). Single
        // proposer ⇒ every honest validator votes for the one valid proposal.
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
        if !node.checkpoint_finalizes(&validators, &validators) {
            node.clear_mempool();
            return None;
        }
        node.apply(&block);
        node.clear_mempool();
        Some(block)
    }
}

/// Mine a proposed block against `bits`: grind the seal's nonce until the block's `header_digest`
/// meets the compact target — the exact predicate `validate_block`'s `pow_check` enforces under
/// `pow_enforced`. Deterministic: the same block content yields the same lowest valid nonce on every
/// node, so a seed and a joiner agree on the sealed block byte-for-byte. Because the digest folds in
/// the nonce, each grind step changes it (real proof-of-work, not a stamp). At the devnet's easy
/// difficulty this returns within a handful of iterations.
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
        let mined = mine(raw, spec.genesis_bits);
        let target = noesis_core::pow::compact_to_target(spec.genesis_bits).unwrap();
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
