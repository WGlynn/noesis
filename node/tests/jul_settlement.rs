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

/// A `Node` whose Constitution carries a JUL coinbase split (inc-M3-3); all other params default.
fn genesis_with_split(split: Vec<(Script, u16)>) -> Node {
    let validators = vec![Validator { id: 0, pow: 0.0, pos: 1000.0, pom: 0.0, last_heartbeat: 0, staked_balance: 1000.0 }];
    Node::new(0, validators, Constitution { coinbase_split: split, ..Constitution::default() })
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

/// SMUGGLE DEFENSE (Pragma-confluence finding): the conserve-only check keys on the CELLS' actual JUL
/// identity (`is_jul`), not the tx's DECLARED one. A tx declaring a benign token but OUTPUTTING a JUL
/// cell (no JUL input) would mint JUL from nothing under the declaration — must be REJECTED.
/// RED break: key the clause on `tx.code_hash`/`tx.args` instead of `is_jul` (the original bug).
#[test]
fn jul_cannot_be_smuggled_under_a_foreign_token_identity() {
    let mut node = genesis();
    node.submit(cell(1, b"alice", 1, b"carrier for the smuggle-defense block"), committed(1, 1));
    let carrier = node.propose();

    // declared identity = a benign non-JUL token ("USD"); output = a real JUL cell; no JUL input.
    let smuggle = TokenTx {
        standard: TokenStandard::Fungible,
        auths: vec![],
        code_hash: [20u8; 32],
        args: b"USD".to_vec(),
        inputs: vec![],
        outputs: vec![jul_cell(11, b"attacker", 1000)],
    };
    assert!(
        !node.validate(&carrier.with_token_txs(vec![smuggle])),
        "JUL SMUGGLE: a JUL output under a foreign token declaration must be rejected (mint-from-nothing)"
    );
}

/// Conservation under BURN (Council monetary finding: the no-burn case was tested, the burn case was
/// not). A JUL tx that outputs LESS than it inputs (a burn) validates, and afterward
/// `jul_supply.issued == live JUL + burned` — issued is cumulative (unchanged), live drops by the burn.
/// RED break: decrement jul_supply on a burn in `apply_transition` (issued must stay cumulative).
#[test]
fn conservation_holds_across_a_jul_burn() {
    let mut node = genesis();
    produce(&mut node, 1, b"a novel contribution earning a coinbase", Some(recipient(b"miner")));
    let issued = node.ledger.jul_supply.issued();
    assert!(issued > 0, "the coinbase issued real JUL");

    let coinbase_cell = node.ledger.token_cells.iter().find(|c| jul::is_jul(c)).cloned().unwrap();
    let in_amt = fungible::amount(&coinbase_cell);
    let keep = in_amt / 4; // burn three-quarters
    node.submit(cell(2, b"alice", 2, b"a second carrier so the burn block is valid"), committed(2, 2));
    let carrier = node.propose();
    let burn = TokenTx {
        standard: TokenStandard::Fungible,
        auths: vec![],
        code_hash: JUL_CODE_HASH,
        args: JUL_ISSUER.to_vec(),
        inputs: vec![coinbase_cell],
        outputs: vec![jul_cell(500, b"miner", keep)], // plain id (not reserved), non-colliding
    };
    let block = carrier.with_token_txs(vec![burn]);
    assert!(node.validate(&block), "a JUL burn (out<in) must validate");
    node.apply(&block);

    let live = fungible::total(&node.ledger.token_cells, &JUL_CODE_HASH, JUL_ISSUER);
    let burned = in_amt - keep;
    assert_eq!(node.ledger.jul_supply.issued(), issued, "issued is cumulative — a burn does NOT reduce it");
    assert_eq!(live, keep, "live JUL == the kept amount");
    assert_eq!(node.ledger.jul_supply.issued(), live + burned, "conservation: issued == live + burned");
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

/// The N-way split (inc-M3-3) routes each `(recipient, bps)` slice from the ONE constructed reward and
/// hands the producer the REMAINDER — conserving supply exactly, with slices under disjoint reserved ids.
/// RED break: drop the `if remaining > 0` producer mint ⇒ Σ minted < reward ⇒ conservation fails.
#[test]
fn coinbase_split_routes_slices_and_conserves() {
    let reward = jul::reward_for_work(1, JulParams::default());
    let mut node = genesis_with_split(vec![(recipient(b"infra"), 2000), (recipient(b"treasury"), 500)]);
    produce(&mut node, 1, b"a novel contribution funding an infra split", Some(recipient(b"miner")));

    let jul_cells: Vec<&Cell> = node.ledger.token_cells.iter().filter(|c| jul::is_jul(c)).collect();
    assert_eq!(jul_cells.len(), 3, "producer + 2 slices");
    let amount_for = |owner: &[u8]| -> u128 {
        jul_cells.iter().filter(|c| c.lock.args.as_slice() == owner).map(|c| fungible::amount(c)).sum()
    };
    assert_eq!(amount_for(b"infra"), reward * 2000 / 10_000, "infra slice = 20%");
    assert_eq!(amount_for(b"treasury"), reward * 500 / 10_000, "treasury slice = 5%");
    assert_eq!(amount_for(b"miner"), reward - reward * 2000 / 10_000 - reward * 500 / 10_000, "producer = remainder (75%)");
    let total: u128 = jul_cells.iter().map(|c| fungible::amount(c)).sum();
    assert_eq!(total, reward, "Σ slices + producer == reward (conservation by construction)");
    assert_eq!(node.ledger.jul_supply.issued(), reward, "supply credited exactly once, by the reward");

    let producer = jul_cells.iter().find(|c| c.lock.args.as_slice() == b"miner").unwrap();
    assert_eq!(producer.id, coinbase_id(1), "the producer keeps the base coinbase id");
    for c in jul_cells.iter().filter(|c| c.lock.args.as_slice() != b"miner") {
        assert_ne!(c.id, coinbase_id(1), "a slice id never collides with the producer id");
        assert!(c.id & jul::SPLIT_SLICE_BIT != 0, "slices sit in the reserved slice space");
        assert!(c.id & jul::COINBASE_ID_BIT != 0, "slices stay barred from token-tx outputs");
    }
    // ANTI-THEATER: drop the `if remaining > 0` producer mint ⇒ Σ < reward ⇒ the conservation assert RED.
}

/// A misconfigured split summing PAST 100% can starve the producer but can NEVER over-mint: each slice is
/// capped at the remaining balance, so Σ minted == reward exactly.
/// RED break: remove the `.min(remaining)` cap ⇒ each slice mints its full share ⇒ Σ > reward (inflation).
#[test]
fn coinbase_split_over_100pct_cannot_over_mint() {
    let reward = jul::reward_for_work(1, JulParams::default());
    let mut node = genesis_with_split(vec![(recipient(b"a"), 8000), (recipient(b"b"), 8000)]);
    produce(&mut node, 1, b"a novel contribution with an over-allocated split", Some(recipient(b"miner")));

    let jul_cells: Vec<&Cell> = node.ledger.token_cells.iter().filter(|c| jul::is_jul(c)).collect();
    let total: u128 = jul_cells.iter().map(|c| fungible::amount(c)).sum();
    assert_eq!(total, reward, "even a >100% split mints EXACTLY the reward, never more");
    assert_eq!(node.ledger.jul_supply.issued(), reward, "supply == reward (no over-issuance)");
    let amount_for = |owner: &[u8]| -> u128 {
        jul_cells.iter().filter(|c| c.lock.args.as_slice() == owner).map(|c| fungible::amount(c)).sum()
    };
    assert_eq!(amount_for(b"a"), reward * 8000 / 10_000, "first slice = 80%");
    assert_eq!(amount_for(b"b"), reward - reward * 8000 / 10_000, "second slice capped at the remaining 20%");
    assert_eq!(amount_for(b"miner"), 0, "producer starved to 0 — but no over-mint");
    // ANTI-THEATER: remove `.min(remaining)` ⇒ slice b mints a full 80% ⇒ Σ = 160% of reward ⇒ RED.
}

/// A split at the MAX boundary (256 slices) mints 256 distinct-id slice cells — no id wraps/collides.
/// This pins the exact edge the 8-bit slice-id index field allows (index 0..=255).
#[test]
fn coinbase_split_at_max_boundary_has_distinct_ids() {
    let split: Vec<(Script, u16)> = (0..jul::MAX_COINBASE_SPLIT)
        .map(|i| (recipient(format!("r{i}").as_bytes()), 1)) // 1 bps each ⇒ tiny slices, producer keeps rest
        .collect();
    let mut node = genesis_with_split(split);
    produce(&mut node, 1, b"a novel contribution funding a max-width split", Some(recipient(b"miner")));

    let ids: Vec<u64> = node.ledger.token_cells.iter().filter(|c| jul::is_jul(c)).map(|c| c.id).collect();
    let mut uniq = ids.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), ids.len(), "every coinbase cell id is distinct at the 256-slice boundary");
}

/// A coinbase split LONGER than [`jul::MAX_COINBASE_SPLIT`] is rejected fail-loud at genesis admission
/// (the 257th slice would `& 0xff`-wrap to the id of the 1st and silently mint a colliding coinbase cell).
/// ANTI-THEATER: remove the `assert!` in `Node::new` ⇒ construction succeeds, no panic ⇒ this test RED.
#[test]
#[should_panic(expected = "MAX_COINBASE_SPLIT")]
fn coinbase_split_over_max_is_rejected_at_genesis() {
    let split: Vec<(Script, u16)> = (0..=jul::MAX_COINBASE_SPLIT) // 257 entries: one past the cap
        .map(|i| (recipient(format!("r{i}").as_bytes()), 1))
        .collect();
    let _ = genesis_with_split(split); // must panic before returning a Node
}
