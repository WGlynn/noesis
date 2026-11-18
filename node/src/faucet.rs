//! faucet — testnet step 3 (`docs/TESTNET.md`): a funded wallet that pays a small amount of test JUL to
//! a newcomer's address. App layer, not consensus, as designed: the faucet is a key holder like any
//! other, its payouts are ordinary signed JUL token txs that face the block gate, and its limits are
//! node-local policy.
//!
//! FUNDING. On a spec with `reward_to_faucet` (the testnet) the producing node names the faucet as each
//! block's reward recipient. Consensus constructs the reward amount and never checks the recipient
//! (`Block::coinbase` is producer-named), so this is the producer's choice written into the spec. JUL
//! cells live in `Ledger.token_cells`, which standing and finality never read.
//!
//! A KEY PER CELL, NOT ONE ADDRESS. A cell's lock is a Lamport one-time key (`lock.args` is its root,
//! checked by `TokenTx::spend_is_authorized`), safe for one signature. A single faucet address would
//! sign every payout with one key, and a few signatures in, anyone could forge a spend. So every faucet
//! cell has its own key, derived from the faucet seed:
//!   * the block reward at height `h` is locked by [`reward_key`]`(seed, h)`;
//!   * the faucet's change, its purse at generation `g`, is locked by [`purse_key`]`(seed, g)` and has id
//!     [`purse_id`]`(g)`. A payout spends purse `g` (plus block rewards when the purse runs short) and
//!     writes purse `g + 1`, the key chain the soulbound authority uses (`identity_mint`).
//!
//! A key's one public signature sits in the block that spends its cell, so the key is never needed again.
//!
//! RE-SIGNING A PAYOUT THAT HAS NOT LANDED IS SAFE HERE. A payout waits in node memory until a
//! contribution block carries it, and its signatures leave the process only inside a finalized block: no
//! route returns them (`GET /faucet` and `GET /tokens` show digests only). So a new request rebuilds and
//! re-signs the one pending payout to cover every waiting payee, and a restart that forgets a pending
//! payout published nothing. Only the block that lands makes a signature public, one per key.
//!
//! KEY CUSTODY. The seed is a file outside the repository, read once at startup by `--serve-api`
//! ([`seed_path`]: the path in `NOESIS_FAUCET_SEED`, else `~/.noesis/noesis-faucet-testnet.seed`), never
//! logged and never in a response. [`DEV_FAUCET_SEED`] is public and for tests only. ONE SEED PER CHAIN:
//! the keys derive from (seed, height) and (seed, generation), so after a chain reset the same seed would
//! sign a second, different message with each key, and the old chain's log is published (the reset
//! policy). Make a new seed for every reset.
//!
//! LIMITS (node-local, in memory, reset by a restart; testnet JUL is worthless by design): [`DRIP`] per
//! payout; one payout per address per [`ADDRESS_WINDOW_SECS`]; at most [`WINDOW_MAX_PAYOUTS`] payouts per
//! [`WINDOW_SECS`] across all addresses; at most [`MAX_PAYEES`] payees waiting for one block.
//!
//! HONEST SCOPE. A faucet needs control binding (`Constitution::control_binding`): without it anyone can
//! spend its cells with an empty signature, so [`crate::rpc::ServerState::with_faucet`] refuses a chain
//! without it. Anyone can name any 32 bytes as the address. With binding on, only the holder of the
//! one-time key whose Lamport root is those bytes can spend the payout; the web wallet has no
//! JUL-spending key yet, so JUL paid to a wallet identity (a hash of two roots) can be held and shown
//! but not moved. A payout lands only when a contribution makes a block, like every token tx.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use noesis_core::lamport;

use crate::runtime::{Block, TokenStandard, TokenTx};
use crate::{Cell, Script};

/// The PUBLIC faucet seed for tests. A chain whose faucet used it would be drained by anyone.
pub const DEV_FAUCET_SEED: [u8; 32] = *b"noesis-dev-faucet-seed-PUBLIC-00";

/// JUL paid per payout, in base units (10 JUL).
pub const DRIP: u128 = 10 * crate::jul::JUL_BASE_UNITS;
/// One payout per address per this many seconds (24 hours).
pub const ADDRESS_WINDOW_SECS: u64 = 24 * 60 * 60;
/// The global window: at most [`WINDOW_MAX_PAYOUTS`] payouts per this many seconds (1 hour).
pub const WINDOW_SECS: u64 = 60 * 60;
/// Payouts per [`WINDOW_SECS`] across all addresses.
pub const WINDOW_MAX_PAYOUTS: usize = 30;
/// Payees one pending payout covers; more wait for the next block.
pub const MAX_PAYEES: usize = 8;
/// Inputs one payout spends (the purse plus block rewards). Each carries a 16 KiB signature that every
/// mining attempt re-hashes with the header, so fewer inputs mean faster blocks.
pub const MAX_INPUTS: usize = 4;

/// The id space of the faucet's purse: "FAUC" in the high half. The top bit is clear, so it is outside
/// the reserved coinbase space a token tx may not write.
pub const PURSE_ID_TAG: u64 = 0x4641_5543 << 32;

fn derive(tag: &[u8], seed: &[u8; 32], n: u64) -> [u8; 32] {
    let mut buf = tag.to_vec();
    buf.extend_from_slice(seed);
    buf.extend_from_slice(&n.to_le_bytes());
    noesis_core::pow::hash(&buf)
}

/// The one-time key that locks the block reward at `height`.
pub fn reward_key(seed: &[u8; 32], height: u64) -> [u8; 32] {
    derive(b"noesis-faucet-reward-key-v1", seed, height)
}

/// The one-time key that locks the faucet's purse at `generation`.
pub fn purse_key(seed: &[u8; 32], generation: u64) -> [u8; 32] {
    derive(b"noesis-faucet-purse-key-v1", seed, generation)
}

/// The purse's cell id at `generation`.
pub fn purse_id(generation: u64) -> u64 {
    PURSE_ID_TAG | (generation & 0xffff_ffff)
}

/// A payout cell's id: distinct per (purse generation, payee slot), outside the coinbase space.
fn payout_id(generation: u64, slot: usize) -> u64 {
    let mut buf = b"noesis-faucet-payout-id-v1".to_vec();
    buf.extend_from_slice(&generation.to_le_bytes());
    buf.extend_from_slice(&(slot as u64).to_le_bytes());
    let h = noesis_core::pow::hash(&buf);
    u64::from_le_bytes(h[..8].try_into().expect("8 bytes")) & !crate::jul::COINBASE_ID_BIT
}

fn jul_type() -> Script {
    Script { code_hash: crate::jul::JUL_CODE_HASH, args: crate::jul::JUL_ISSUER.to_vec() }
}

fn lock(root: &[u8]) -> Script {
    Script { code_hash: [0u8; 32], args: root.to_vec() }
}

/// The seed file `--serve-api` reads: `NOESIS_FAUCET_SEED` if set, else
/// `<home>/.noesis/noesis-faucet-testnet.seed` (home is `HOME`, or `USERPROFILE` on Windows).
pub fn seed_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("NOESIS_FAUCET_SEED") {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".noesis").join("noesis-faucet-testnet.seed"))
}

/// Read a faucet seed file: 64 hex characters in plain text (`identity_mint::parse_seed`; the identity
/// issuer's file can be sealed since wallet decision A8, this one cannot yet). Errors name the path,
/// never the contents.
pub fn load_seed(path: &Path) -> Result<[u8; 32], String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read the faucet seed {}: {e}", path.display()))?;
    crate::identity_mint::parse_seed(&text).map_err(|e| format!("faucet seed {}: {e}", path.display()))
}

/// Why a request was not queued. `Limited` is an abuse limit (HTTP 429); `Unavailable` means the
/// faucet cannot pay right now (HTTP 503).
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    Limited(String),
    Unavailable(String),
}

/// The faucet's node-local state. The cells it owns are found in the live ledger by key derivation, so
/// nothing here needs to survive a restart.
pub struct Faucet {
    seed: [u8; 32],
    /// Lamport roots already derived, by (kind, n): a keygen is about a thousand hashes.
    roots: HashMap<(u8, u64), [u8; 32]>,
    /// Addresses the pending payout covers, oldest first.
    payees: Vec<[u8; 32]>,
    /// The signed payout covering `payees`, rebuilt whenever a payee joins.
    pending: Option<TokenTx>,
    /// When each address last had a payout queued (node clock, seconds).
    last_payout: HashMap<[u8; 32], u64>,
    /// When recent payouts were queued, oldest first (the global window).
    recent: VecDeque<u64>,
}

const REWARD: u8 = 0;
const PURSE: u8 = 1;

impl Faucet {
    pub fn new(seed: [u8; 32]) -> Self {
        Faucet {
            seed,
            roots: HashMap::new(),
            payees: Vec::new(),
            pending: None,
            last_payout: HashMap::new(),
            recent: VecDeque::new(),
        }
    }

    fn root(&mut self, kind: u8, n: u64) -> [u8; 32] {
        let seed = self.seed;
        *self.roots.entry((kind, n)).or_insert_with(|| {
            let key = if kind == REWARD { reward_key(&seed, n) } else { purse_key(&seed, n) };
            lamport::keygen_root(&key)
        })
    }

    fn key(&self, kind: u8, n: u64) -> [u8; 32] {
        if kind == REWARD {
            reward_key(&self.seed, n)
        } else {
            purse_key(&self.seed, n)
        }
    }

    /// The lock the block reward at `height` pays (what the producing node names as the coinbase).
    pub fn reward_lock(&mut self, height: u64) -> Script {
        lock(&self.root(REWARD, height))
    }

    /// Every live cell the faucet can spend, as (kind, n, cell): its block rewards (lowest height first)
    /// after its purse. The purse is the highest live generation, the first such cell in ledger order (a
    /// copy made later under the same public lock cannot come first). Older generations are ignored:
    /// their key has signed.
    fn owned(&mut self, live: &[Cell]) -> Vec<(u8, u64, Cell)> {
        let mut purse: Option<(u64, &Cell)> = None;
        let mut rewards: Vec<(u64, &Cell)> = Vec::new();
        for c in live.iter().filter(|c| crate::jul::is_jul(c) && c.lock.code_hash == [0u8; 32]) {
            if c.id & crate::jul::COINBASE_ID_BIT != 0 && c.id & crate::jul::SPLIT_SLICE_BIT == 0 {
                let h = c.id & !crate::jul::COINBASE_ID_BIT;
                if crate::jul::coinbase_id(h) == c.id && c.lock.args == self.root(REWARD, h) {
                    rewards.push((h, c));
                }
            } else if c.id & !0xffff_ffff == PURSE_ID_TAG {
                let g = c.id & 0xffff_ffff;
                if purse.is_none_or(|(best, _)| g > best) && c.lock.args == self.root(PURSE, g) {
                    purse = Some((g, c));
                }
            }
        }
        rewards.sort_by_key(|(h, _)| *h);
        purse
            .map(|(g, c)| (PURSE, g, c.clone()))
            .into_iter()
            .chain(rewards.into_iter().map(|(h, c)| (REWARD, h, c.clone())))
            .collect()
    }

    /// JUL the faucet holds in `live`, in base units.
    pub fn balance(&mut self, live: &[Cell]) -> u128 {
        self.owned(live).iter().map(|(_, _, c)| crate::tokens::fungible::amount(c)).sum()
    }

    /// The addresses the pending payout covers.
    pub fn waiting(&self) -> &[[u8; 32]] {
        &self.payees
    }

    /// The signed payout waiting for the next contribution block, if any.
    pub fn pending(&self) -> Option<&TokenTx> {
        self.pending.as_ref()
    }

    /// Build and sign one payout of [`DRIP`] to every waiting payee from the cells in `live`: spend the
    /// purse, then block rewards (at most [`MAX_INPUTS`] inputs) until the inputs exceed the payout, and
    /// write the rest to the next purse. The change is never zero, so a purse is always live once the
    /// faucet has paid, and its generation counter survives in the chain.
    fn build(&mut self, live: &[Cell]) -> Result<TokenTx, String> {
        let need = DRIP * self.payees.len() as u128;
        let owned = self.owned(live);
        let next_gen = match owned.first() {
            Some((PURSE, g, _)) => g + 1,
            _ => 0,
        };
        let mut have: u128 = 0;
        let mut inputs: Vec<(u8, u64, Cell)> = Vec::new();
        for o in owned {
            if have > need || inputs.len() == MAX_INPUTS {
                break;
            }
            have += crate::tokens::fungible::amount(&o.2);
            inputs.push(o);
        }
        if have <= need {
            return Err(format!(
                "the faucet holds too little JUL for this payout ({} base units); it refills with each block",
                have
            ));
        }
        let mut outputs: Vec<Cell> = self
            .payees
            .iter()
            .enumerate()
            .map(|(slot, a)| Cell {
                id: payout_id(next_gen, slot),
                lock: lock(a),
                type_script: jul_type(),
                parent: None,
                timestamp: 0,
                data: crate::tokens::fungible::encode(DRIP),
            })
            .collect();
        outputs.push(Cell {
            id: purse_id(next_gen),
            lock: lock(&self.root(PURSE, next_gen)),
            type_script: jul_type(),
            parent: None,
            timestamp: 0,
            data: crate::tokens::fungible::encode(have - need),
        });
        let keys: Vec<[u8; 32]> = inputs.iter().map(|(kind, n, _)| self.key(*kind, *n)).collect();
        let mut tx = TokenTx {
            standard: TokenStandard::Fungible,
            code_hash: crate::jul::JUL_CODE_HASH,
            args: crate::jul::JUL_ISSUER.to_vec(),
            inputs: inputs.into_iter().map(|(_, _, c)| c).collect(),
            outputs,
            auths: Vec::new(),
        };
        let digest = tx.digest();
        tx.auths = keys.iter().map(|k| lamport::sign(k, &digest)).collect();
        Ok(tx)
    }

    /// Queue a payout of [`DRIP`] to `address` at node time `now` (seconds), against the live token
    /// cells. Returns the digest of the pending payout that now covers it. Asking again while the payout
    /// waits returns the same digest.
    pub fn request(&mut self, address: [u8; 32], now: u64, live: &[Cell]) -> Result<[u8; 32], Refusal> {
        if self.payees.contains(&address) {
            if let Some(p) = &self.pending {
                return Ok(p.digest());
            }
        }
        if let Some(&t) = self.last_payout.get(&address) {
            if now < t.saturating_add(ADDRESS_WINDOW_SECS) {
                return Err(Refusal::Limited(format!(
                    "this address had a payout in the last 24 hours; ask again in {} s",
                    t.saturating_add(ADDRESS_WINDOW_SECS) - now
                )));
            }
        }
        while self.recent.front().is_some_and(|&t| t.saturating_add(WINDOW_SECS) <= now) {
            self.recent.pop_front();
        }
        if self.recent.len() >= WINDOW_MAX_PAYOUTS {
            return Err(Refusal::Limited(format!("the faucet paid {WINDOW_MAX_PAYOUTS} times this hour; try later")));
        }
        if self.payees.len() >= MAX_PAYEES {
            return Err(Refusal::Limited("payouts are waiting for the next block; ask again after it".into()));
        }
        self.payees.push(address);
        match self.build(live) {
            Ok(tx) => {
                let d = tx.digest();
                self.pending = Some(tx);
                self.last_payout.insert(address, now);
                self.recent.push_back(now);
                Ok(d)
            }
            Err(e) => {
                self.payees.pop();
                Err(Refusal::Unavailable(e))
            }
        }
    }

    /// Re-sign the pending payout against `live` if it no longer holds there (a missed block can leave it
    /// stale). If it cannot be rebuilt, drop it and its payees' limit records, so they can ask again.
    /// Safe because the replaced signatures were never published (see the module).
    pub fn refresh(&mut self, live: &[Cell], control_binding: bool) {
        let ok = self
            .pending
            .as_ref()
            .is_none_or(|p| crate::runtime::txs_conserve_and_single_use(live, std::slice::from_ref(p), control_binding));
        if ok {
            return;
        }
        match self.build(live) {
            Ok(tx) => self.pending = Some(tx),
            Err(_) => {
                for a in self.payees.drain(..) {
                    self.last_payout.remove(&a);
                }
                self.pending = None;
            }
        }
    }

    /// A block finalized: if it carried the pending payout, the payees are paid.
    pub fn landed(&mut self, block: &Block) {
        let Some(p) = &self.pending else { return };
        let d = p.digest();
        if block.token_txs.iter().any(|t| t.digest() == d) {
            self.pending = None;
            self.payees.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chainspec::ChainSpec;
    use crate::commit_order::Committed;
    use crate::runtime::Node;

    /// The testnet spec at dev's easy bits: the testnet's constitution (binding on, PoW on, its chain id)
    /// except the difficulty, so each block seals in a few hundred hashes. Since 2026-11-18 every seal
    /// must claim its chain's bits, so these blocks are valid on this test chain and refused by the real
    /// testnet (`PowBitsWrong`); nothing here depends on difficulty.
    fn fast_testnet() -> ChainSpec {
        let mut spec = ChainSpec::testnet();
        spec.constitution.pow_bits = ChainSpec::dev().genesis_bits();
        spec
    }

    fn contribution(id: u64, contributor: &[u8; 32]) -> (Cell, Committed) {
        let mut secret = [0u8; 32];
        secret[..8].copy_from_slice(&id.to_le_bytes());
        (
            Cell {
                id,
                lock: Script { code_hash: [0u8; 32], args: b"lk".to_vec() },
                type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
                parent: None,
                timestamp: id,
                data: format!("note {id}: a distinct field observation about tides, kilns and rivers").into_bytes(),
            },
            Committed { height: 0, secret },
        )
    }

    /// Submit one contribution and produce a block paying `reward_to`, carrying `txs`.
    fn block(spec: &ChainSpec, node: &mut Node, id: u64, contributor: &[u8; 32], txs: Vec<TokenTx>, reward_to: Script) -> Block {
        let (cell, mut coord) = contribution(id, contributor);
        coord.height = node.ledger.height + 1;
        node.submit(cell, coord);
        spec.produce_block_paying(node, txs, reward_to).expect("the block finalizes")
    }

    fn jul_of(live: &[Cell], address: &[u8; 32]) -> u128 {
        live.iter()
            .filter(|c| crate::jul::is_jul(c) && c.lock.args == address.to_vec())
            .map(crate::tokens::fungible::amount)
            .sum()
    }

    #[test]
    fn rewards_fund_the_faucet_and_payouts_land_signed_one_key_per_cell() {
        let spec = fast_testnet();
        let (mut node, _) = spec.genesis_node();
        let mut f = Faucet::new(DEV_FAUCET_SEED);
        let contributor = [0xC1u8; 32];
        // empty at genesis: nothing to pay with
        assert!(matches!(f.request([0xA1; 32], 100, &node.ledger.token_cells), Err(Refusal::Unavailable(_))));
        assert!(f.pending().is_none() && f.waiting().is_empty(), "a refused request leaves nothing behind");
        // block 1 pays the faucet's height-1 key
        let reward_to = f.reward_lock(1);
        let b1 = block(&spec, &mut node, 1, &contributor, vec![], reward_to);
        let reward = node.ledger.jul_supply.issued();
        assert!(reward > DRIP, "one testnet block reward covers a drip");
        assert_eq!(f.balance(&node.ledger.token_cells), reward, "the faucet owns the whole reward");
        assert_eq!(b1.coinbase.as_ref().unwrap().args, lamport::keygen_root(&reward_key(&DEV_FAUCET_SEED, 1)).to_vec());

        // two newcomers ask; one pending payout covers both, re-signed for the second
        let (a, b) = ([0xA1u8; 32], [0xB2u8; 32]);
        let d1 = f.request(a, 100, &node.ledger.token_cells).expect("queued");
        assert_eq!(f.request(a, 101, &node.ledger.token_cells), Ok(d1), "asking again while waiting is a no-op");
        let d2 = f.request(b, 102, &node.ledger.token_cells).expect("queued");
        assert_ne!(d1, d2, "the pending payout was rebuilt to cover both payees");
        let tx = f.pending().unwrap().clone();
        assert_eq!(tx.inputs.len(), 1, "one block reward pays both");
        assert_eq!(tx.auths.len(), 1);
        // the block gate accepts it with control binding on, and refuses it unsigned
        assert!(node.constitution.control_binding);
        assert!(crate::runtime::txs_conserve_and_single_use(&node.ledger.token_cells, std::slice::from_ref(&tx), true));
        let unsigned = TokenTx { auths: vec![], ..tx.clone() };
        assert!(!crate::runtime::txs_conserve_and_single_use(&node.ledger.token_cells, &[unsigned], true));

        let reward_to = f.reward_lock(2);
        let b2 = block(&spec, &mut node, 2, &contributor, vec![tx], reward_to);
        f.landed(&b2);
        assert!(f.pending().is_none() && f.waiting().is_empty(), "paid");
        let live = &node.ledger.token_cells;
        assert_eq!(jul_of(live, &a), DRIP);
        assert_eq!(jul_of(live, &b), DRIP);
        // the faucet keeps the rest: block 1's reward minus two drips, in purse 0, plus block 2's reward
        assert_eq!(f.balance(live), 2 * reward - 2 * DRIP, "nothing minted or lost");
        let purse0 = live.iter().find(|c| c.id == purse_id(0)).expect("purse 0");
        assert_eq!(purse0.lock.args, lamport::keygen_root(&purse_key(&DEV_FAUCET_SEED, 0)).to_vec());

        // a third payout spends purse 0 (its key's first signature) and writes purse 1
        let c = [0xC3u8; 32];
        f.request(c, 200, &node.ledger.token_cells).expect("queued");
        let tx = f.pending().unwrap().clone();
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.inputs[0].id, purse_id(0), "the purse pays first");
        assert!(tx.outputs.iter().any(|o| o.id == purse_id(1)));
        let reward_to = f.reward_lock(3);
        let b3 = block(&spec, &mut node, 3, &contributor, vec![tx], reward_to);
        f.landed(&b3);
        let live = &node.ledger.token_cells;
        assert_eq!(jul_of(live, &c), DRIP);
        assert!(live.iter().all(|cell| cell.id != purse_id(0)), "purse 0 is spent");
        assert_eq!(f.balance(live), 3 * reward - 3 * DRIP);

        // every faucet key that signed on chain signed exactly once: no lock root repeats across inputs
        let spent: Vec<Vec<u8>> = [&b2, &b3].iter().flat_map(|b| b.token_txs.iter().flat_map(|t| t.inputs.iter().map(|i| i.lock.args.clone()))).collect();
        let distinct: std::collections::HashSet<&Vec<u8>> = spent.iter().collect();
        assert_eq!(distinct.len(), spent.len(), "no faucet key signed twice");
    }

    /// JUL is the money layer, orthogonal to PoM standing: who the block reward pays, and what the faucet
    /// pays out, must not move standing or finality weight. Two replicas apply the same contribution
    /// blocks on the testnet constitution: A pays every reward to the placeholder and carries nothing; B
    /// pays the faucet and carries a faucet payout in most blocks, one of them to the contributor's own
    /// 32 bytes, so any fold of JUL into attribution would show as a changed score. Checked with the
    /// testnet's `vesting_w = 0` and with a vesting window, where finality weight (cleared cells only)
    /// is a different map from `pom`. After every block: identical cells, index root, PoM, work clock,
    /// finality weight and finality decision; only token state differs.
    #[test]
    fn the_reward_recipient_and_faucet_payouts_move_no_standing_and_no_finality_weight() {
        use crate::consensus::Validator;
        for vesting_w in [0u64, 600] {
            let mut spec = fast_testnet();
            spec.constitution.vesting_w = vesting_w;
            let (mut a, _) = spec.genesis_node();
            let (mut b, _) = spec.genesis_node();
            let mut f = Faucet::new(DEV_FAUCET_SEED);
            let contributors = [[0xC1u8; 32], [0xC2u8; 32]];
            for h in 1..=6u64 {
                let who = &contributors[(h % 2) as usize];
                let payout = if h >= 2 {
                    // the payee is the contributor's own key bytes in block 3, a stranger otherwise
                    let payee = if h == 3 { *who } else { [0xD0 + h as u8; 32] };
                    f.request(payee, h * 10_000, &b.ledger.token_cells).expect("funded by block 1");
                    vec![f.pending().unwrap().clone()]
                } else {
                    vec![]
                };
                let placeholder = spec.placeholder_reward();
                let ba = block(&spec, &mut a, h, who, vec![], placeholder);
                let reward_to = f.reward_lock(h);
                let bb = block(&spec, &mut b, h, who, payout.clone(), reward_to);
                f.landed(&bb);
                assert_eq!(bb.token_txs.len(), payout.len(), "block {h}: the payout was carried");
                assert_ne!(ba.coinbase, bb.coinbase);

                let (a_ids, a_root, a_pom, _, a_work) = a.ledger.state_digest();
                let (b_ids, b_root, b_pom, _, b_work) = b.ledger.state_digest();
                assert_eq!(
                    (&a_ids, &a_root, &a_pom, a_work),
                    (&b_ids, &b_root, &b_pom, b_work),
                    "W={vesting_w} block {h}: the reward recipient or a payout moved cells, the index, PoM or the work clock"
                );
                // (state_digest's token part is the id list; the reward cell has the same id on both, so
                // compare the locks: the reward and the payouts are real, different token state)
                let locks = |n: &Node| n.ledger.token_cells.iter().map(|c| (c.id, c.lock.args.clone())).collect::<Vec<_>>();
                assert_ne!(locks(&a), locks(&b), "block {h}: the token state differs");
                let (fa, fb) = (a.finality_pom_weight(), b.finality_pom_weight());
                assert_eq!(fa, fb, "W={vesting_w} block {h}: finality weight moved");
                // the finality decision itself, from validators built the way produce_block builds them
                let set = |fpw: &HashMap<Vec<u8>, u64>| -> Vec<Validator> {
                    spec.validators
                        .iter()
                        .map(|(k, v)| Validator { pom: fpw.get(k).map(|w| *w as f64).unwrap_or(0.0), ..v.clone() })
                        .collect()
                };
                let (va, vb) = (set(&fa), set(&fb));
                assert_eq!(a.checkpoint_finalizes(&va, &va), b.checkpoint_finalizes(&vb, &vb));
                // no faucet lock and no payee ever becomes a PoM key
                for c in &b.ledger.token_cells {
                    if crate::jul::is_jul(c) && c.lock.args.len() == 32 && !contributors.iter().any(|k| k.to_vec() == c.lock.args) {
                        assert!(!b.ledger.pom.contains_key(&c.lock.args), "a JUL lock became a PoM key");
                    }
                }
            }
            assert!(jul_of(&b.ledger.token_cells, &contributors[1]) == DRIP, "the contributor holds the JUL it was paid");
            assert_eq!(a.ledger.jul_supply.issued(), b.ledger.jul_supply.issued(), "same issuance on both");
        }
    }

    #[test]
    fn a_fresh_faucet_finds_its_cells_from_the_chain_alone() {
        // A restart keeps no faucet state: a new Faucet on the same seed finds the same purse and rewards.
        let spec = fast_testnet();
        let (mut node, _) = spec.genesis_node();
        let mut f = Faucet::new(DEV_FAUCET_SEED);
        let contributor = [0xC1u8; 32];
        let r1 = f.reward_lock(1);
        block(&spec, &mut node, 1, &contributor, vec![], r1);
        f.request([0xA1; 32], 10, &node.ledger.token_cells).unwrap();
        let tx = f.pending().unwrap().clone();
        let r2 = f.reward_lock(2);
        let b = block(&spec, &mut node, 2, &contributor, vec![tx], r2);
        f.landed(&b);
        let mut again = Faucet::new(DEV_FAUCET_SEED);
        assert_eq!(again.balance(&node.ledger.token_cells), f.balance(&node.ledger.token_cells));
        again.request([0xB2; 32], 20, &node.ledger.token_cells).unwrap();
        assert_eq!(again.pending().unwrap().inputs[0].id, purse_id(0), "it resumes at the live purse");
        // another seed owns nothing here
        assert_eq!(Faucet::new([7u8; 32]).balance(&node.ledger.token_cells), 0);
    }

    #[test]
    fn limits_per_address_per_window_and_per_block() {
        // Funded by hand: a reward cell at height 1 under the faucet's key, enough for every payout.
        let mut f = Faucet::new(DEV_FAUCET_SEED);
        let live = vec![Cell {
            id: crate::jul::coinbase_id(1),
            lock: f.reward_lock(1),
            type_script: jul_type(),
            parent: None,
            timestamp: 0,
            data: crate::tokens::fungible::encode(1_000_000 * DRIP),
        }];
        let addr = |i: u32| {
            let mut a = [0u8; 32];
            a[..4].copy_from_slice(&i.to_le_bytes());
            a
        };
        // per block: MAX_PAYEES wait at once, the next is told to wait
        for i in 0..MAX_PAYEES as u32 {
            f.request(addr(i), 1000, &live).expect("queued");
        }
        assert!(matches!(f.request(addr(99), 1000, &live), Err(Refusal::Limited(_))));
        assert_eq!(f.pending().unwrap().outputs.len(), MAX_PAYEES + 1, "every payee plus the purse");
        // pretend the block landed
        f.pending = None;
        f.payees.clear();
        // per address: the same address waits 24 hours
        let again = f.request(addr(0), 1000 + ADDRESS_WINDOW_SECS - 1, &live);
        assert!(matches!(again, Err(Refusal::Limited(ref m)) if m.contains("24 hours")));
        // global window: WINDOW_MAX_PAYOUTS per hour, counting the 8 above
        let mut paid = MAX_PAYEES;
        let mut i = 100u32;
        while paid < WINDOW_MAX_PAYOUTS {
            if f.payees.len() == MAX_PAYEES {
                f.pending = None;
                f.payees.clear();
            }
            f.request(addr(i), 1500, &live).expect("inside the window");
            paid += 1;
            i += 1;
        }
        f.pending = None;
        f.payees.clear();
        let full = f.request(addr(i), 1500, &live);
        assert!(matches!(full, Err(Refusal::Limited(ref m)) if m.contains("this hour")), "{full:?}");
        // an hour after the first payouts the window has room again, and a day later address 0 does too
        assert!(f.request(addr(i), 1000 + WINDOW_SECS, &live).is_ok());
        f.pending = None;
        f.payees.clear();
        assert!(f.request(addr(0), 1000 + ADDRESS_WINDOW_SECS, &live).is_ok());
    }

    #[test]
    fn the_faucet_ignores_copies_under_its_public_locks_and_an_old_purse() {
        // Anyone can write a JUL cell under a lock root once that root is public. Copies must never make
        // the faucet sign twice with one key: an old purse generation is ignored, and of two cells at
        // the live generation the faucet spends the first in ledger order (its own; a copy comes later).
        let mut f = Faucet::new(DEV_FAUCET_SEED);
        let cell = |id: u64, root: [u8; 32], amount: u128| Cell {
            id,
            lock: lock(&root),
            type_script: jul_type(),
            parent: None,
            timestamp: 0,
            data: crate::tokens::fungible::encode(amount),
        };
        let p3 = f.root(PURSE, 3);
        let p2 = f.root(PURSE, 2);
        let live = vec![
            cell(purse_id(3), p3, 50 * DRIP),      // the faucet's purse at generation 3
            cell(purse_id(2), p2, 900 * DRIP),     // a copy of an older purse: its key has signed
            cell(purse_id(3), p3, 5000 * DRIP),    // a later copy of the live purse
            cell(purse_id(9), [0xEE; 32], DRIP),   // a purse-shaped id under a lock the faucet never made
        ];
        assert_eq!(f.balance(&live), 50 * DRIP, "only the faucet's own purse counts");
        f.request([0xA1; 32], 1, &live).unwrap();
        let tx = f.pending().unwrap();
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(crate::tokens::fungible::amount(&tx.inputs[0]), 50 * DRIP, "it spends its own purse");
        assert!(tx.outputs.iter().any(|o| o.id == purse_id(4)), "and writes generation 4");
    }
}
