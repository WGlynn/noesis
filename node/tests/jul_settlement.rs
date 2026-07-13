//! JUL money layer, increment 2 — coinbase settlement (consensus-affecting).
//!
//! Proves the coinbase mints the exact protocol-fixed reward to the producer-named recipient, that JUL
//! is conserve-or-burn-ONLY through the token path (so the coinbase is the structurally unique inflation
//! channel — the security finding both Fable-5 planners caught), that supply is conserved, that two nodes
//! converge with coinbases present, and that JUL never perturbs attribution. Each test names the
//! anti-theater break that turns it RED.

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::jul::{self, coinbase_id, JulParams, JUL_BASE_UNITS, JUL_CODE_HASH, JUL_ISSUER};
use noesis::runtime::{Constitution, Node, TokenStandard, TokenTx};
use noesis::tokens::fungible;
use noesis::{Cell, Script};

// ============ fixtures ============

fn genesis() -> Node {
    let validators = vec![Validator {
        id: 0,
        pow: 0.0,
        pos: 1000.0,
        pom: 0.0,
        last_heartbeat: 0,
        staked_balance: 1000.0,
    }];
    Node::new(0, validators, Constitution::default())
}

fn cell(id: u64, contributor: &[u8], ts: u64, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: b"own".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent: None,
        timestamp: ts,
        data: data.to_vec(),
    }
}

fn committed(height: u64, s: u8) -> Committed {
    Committed { height, secret: [s; 32] }
}

/// A JUL fungible cell: JUL type-script identity, `owner` as the lock args, `amount` base units.
fn jul_cell(id: u64, owner: &[u8], amount: u128) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: JUL_CODE_HASH, args: JUL_ISSUER.to_vec() },
        parent: None,
        timestamp: 0,
        data: fungible::encode(amount),
    }
}

fn recipient(owner: &[u8]) -> Script {
    Script { code_hash: [0u8; 32], args: owner.to_vec() }
}

/// Produce + apply one carrier block carrying an optional coinbase recipient. Returns the block height.
fn produce(node: &mut Node, height: u64, data: &[u8], coinbase: Option<Script>) {
    node.submit(cell(height, b"alice", height, data), committed(height, height as u8));
    let mut block = node.propose();
    if let Some(r) = coinbase {
        block = block.with_coinbase(r);
    }
    assert!(node.validate(&block), "carrier block must validate");
    node.apply(&block);
    node.clear_mempool();
}

// ============ tests ============

/// The coinbase mints EXACTLY `reward_for_work(block_work, params.jul)` to the producer-named recipient,
/// under the JUL identity, at the reserved coinbase id. Pre-PoW that is `JUL_BASE_UNITS` (1 JUL).
/// RED break: have `apply_transition` read the amount from a producer field instead of constructing it.
#[test]
fn coinbase_mints_exact_reward_to_recipient() {
    let expected = jul::reward_for_work(1, JulParams::default());
    assert_eq!(expected, JUL_BASE_UNITS, "pre-PoW default rate = 1 JUL per block");

    let mut node = genesis();
    produce(&mut node, 1, b"a genuinely novel first contribution", Some(recipient(b"miner")));

    let jul_cells: Vec<&Cell> = node.ledger.token_cells.iter().filter(|c| jul::is_jul(c)).collect();
    assert_eq!(jul_cells.len(), 1, "exactly one coinbase cell minted");
    let cb = jul_cells[0];
    assert_eq!(cb.id, coinbase_id(1), "coinbase uses the reserved id");
    assert_eq!(cb.lock.args, b"miner".to_vec(), "paid the producer-named recipient");
    assert_eq!(fungible::amount(cb), expected, "exact protocol-fixed reward");
    assert_eq!(node.ledger.jul_supply.issued(), expected, "supply credited by exactly the reward");
}

/// A `coinbase: None` block mints nothing and leaves supply untouched (existing blocks are unaffected).
/// RED break: mint unconditionally, ignoring the `Option`.
#[test]
fn coinbase_none_mints_nothing() {
    let mut node = genesis();
    produce(&mut node, 1, b"a first novel contribution with no coinbase", None);
    assert!(!node.ledger.token_cells.iter().any(jul::is_jul), "no JUL cell without a coinbase");
    assert_eq!(node.ledger.jul_supply.issued(), 0, "supply unchanged");
}

/// Conservation: after N coinbase blocks, cumulative supply equals the sum of live JUL cell amounts
/// equals N × per-block reward (no burns in this run).
/// RED break: credit the supply twice per coinbase, or mint two cells.
#[test]
fn jul_supply_equals_sum_of_live_jul_cells() {
    let mut node = genesis();
    let per = jul::reward_for_work(1, JulParams::default());
    let n = 4u128;
    let datas: [&[u8]; 4] = [b"winter mornings one", b"summer evenings two", b"autumn light three", b"spring rain four"];
    for (i, d) in datas.iter().enumerate() {
        produce(&mut node, i as u64 + 1, d, Some(recipient(b"miner")));
    }
    let live = fungible::total(&node.ledger.token_cells, &JUL_CODE_HASH, JUL_ISSUER);
    assert_eq!(node.ledger.jul_supply.issued(), n * per, "supply == N · per-block reward");
    assert_eq!(live, n * per, "live JUL total == supply (no burns)");
    assert_eq!(node.ledger.jul_supply.issued(), live, "the conservation theorem holds");
}

/// THE security test: JUL is conserve-or-burn-ONLY through the token path, so the coinbase is the
/// structurally unique inflation channel. Even WITH a (hypothetical) authority input whose
/// `lock.args == JUL_ISSUER` — the exact vector both planners found — an out>in JUL tx is REJECTED.
/// RED break: delete the JUL conserve-only clause in `token_txs_conserve_and_single_use`.
#[test]
fn jul_cannot_be_minted_through_the_token_path() {
    let mut node = genesis();
    // seed a live JUL "authority" cell (the attack precondition the clause defeats).
    node.ledger.token_cells.push(jul_cell(10, JUL_ISSUER, 5));

    node.submit(cell(1, b"alice", 1, b"a carrier so the block is non-empty and valid"), committed(1, 1));
    let carrier = node.propose();

    // the attack: consume the authority cell (in=5), mint 1000 out. is_valid's derived minter would
    // AUTHORIZE this (input.lock.args == args == JUL_ISSUER) — the clause is the only thing stopping it.
    let mint = TokenTx {
        standard: TokenStandard::Fungible,
        auths: vec![],
        code_hash: JUL_CODE_HASH,
        args: JUL_ISSUER.to_vec(),
        inputs: vec![jul_cell(10, JUL_ISSUER, 5)],
        outputs: vec![jul_cell(11, b"attacker", 1000)],
    };
    assert!(
        !node.validate(&carrier.clone().with_token_txs(vec![mint])),
        "JUL MINT HOLE: out>in JUL tx must be rejected even with an authority input"
    );

    // control: a CONSERVING JUL transfer (out == in) still validates — the clause rejects only inflation.
    let transfer = TokenTx {
        standard: TokenStandard::Fungible,
        auths: vec![],
        code_hash: JUL_CODE_HASH,
        args: JUL_ISSUER.to_vec(),
        inputs: vec![jul_cell(10, JUL_ISSUER, 5)],
        outputs: vec![jul_cell(12, b"bob", 5)],
    };
    assert!(
        node.validate(&carrier.with_token_txs(vec![transfer])),
        "conserve-only over-rejected: an honest out==in JUL transfer must still validate"
    );
}

/// A token-tx output may not squat a reserved coinbase id (retirement matches `(id,lock,type)`, so a
/// collision could grief a real reward).
/// RED break: drop the `out.id & COINBASE_ID_BIT` check.
#[test]
fn token_tx_output_cannot_squat_a_coinbase_id() {
    let mut node = genesis();
    node.ledger.token_cells.push(jul_cell(10, JUL_ISSUER, 5));
    node.submit(cell(1, b"alice", 1, b"carrier for the squat test block"), committed(1, 1));
    let carrier = node.propose();

    // conserving transfer, but the OUTPUT claims a coinbase id.
    let squat = TokenTx {
        standard: TokenStandard::Fungible,
        auths: vec![],
        code_hash: JUL_CODE_HASH,
        args: JUL_ISSUER.to_vec(),
        inputs: vec![jul_cell(10, JUL_ISSUER, 5)],
        outputs: vec![jul_cell(coinbase_id(999), b"attacker", 5)],
    };
    assert!(
        !node.validate(&carrier.with_token_txs(vec![squat])),
        "reserved coinbase id space is protocol-only; a token-tx output must not squat it"
    );
}

/// Two nodes applying the same coinbase chain converge to a byte-identical `state_digest` AND equal JUL
/// totals (the digest carries token ids; the amount equality is the extra proof the digest can't see).
/// RED break: derive the coinbase id/amount from anything replica-local (clock, node id).
#[test]
fn two_nodes_converge_with_coinbases_present() {
    let build = || {
        let mut node = genesis();
        let datas: [&[u8]; 3] = [b"alpha novel content", b"beta novel content", b"gamma novel content"];
        for (i, d) in datas.iter().enumerate() {
            produce(&mut node, i as u64 + 1, d, Some(recipient(b"miner")));
        }
        node
    };
    let a = build();
    let b = build();
    assert_eq!(a.ledger.state_digest(), b.ledger.state_digest(), "digests converge with coinbases");
    assert_eq!(a.ledger.jul_supply.issued(), b.ledger.jul_supply.issued(), "JUL supply converges");
    assert!(a.ledger.jul_supply.issued() > 0, "anti-triviality: the chain actually minted JUL");
}

/// JUL never perturbs attribution/finality: a chain WITH coinbases produces the identical `pom` map and
/// identical finalized-cell digest component as the same carrier chain WITHOUT coinbases. JUL lives in
/// `token_cells`; attribution folds over `cells` only.
/// RED break: fold coinbase cells into `cells` (or into the pom recompute) instead of `token_cells`.
#[test]
fn coinbase_does_not_touch_attribution() {
    let run = |with_coinbase: bool| {
        let mut node = genesis();
        let datas: [&[u8]; 3] = [b"one novel thing here", b"two novel things here", b"three novel here"];
        for (i, d) in datas.iter().enumerate() {
            let cb = if with_coinbase { Some(recipient(b"miner")) } else { None };
            produce(&mut node, i as u64 + 1, d, cb);
        }
        node
    };
    let with = run(true);
    let without = run(false);
    assert_eq!(with.ledger.pom, without.ledger.pom, "JUL must not change PoM attribution");
    // the cell-id + index-root + pom components of the digest are attribution; only token_ids differ.
    let (ids_w, root_w, pom_w, _tok_w, work_w) = with.ledger.state_digest();
    let (ids_wo, root_wo, pom_wo, _tok_wo, work_wo) = without.ledger.state_digest();
    assert_eq!((ids_w, root_w, pom_w, work_w), (ids_wo, root_wo, pom_wo, work_wo), "attribution digest unaffected by JUL");
}
