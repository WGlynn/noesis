//! noesisd — the T0 local-devnet driver: a single Noesis node that boots from an honest genesis,
//! produces + finality-gates + applies blocks, and exposes its state.
//!
//! This is the first RUNNABLE Noesis node. Until now the state machine ran only under `cargo test`
//! (node/tests/two_node.rs proves the exact submit->propose->validate->finalize->apply sequence
//! composes and converges byte-for-byte). This binary wraps that proven sequence in a real
//! `fn main`, with ZERO new consensus mechanism.
//!
//! It closes the one honest-critical gap the reference had: finality is now a REAL GATE on the
//! run path. `finalizes()` had no non-test caller, so a running chain would have stamped blocks
//! final on validation alone. Here the driver calls `Node::checkpoint_finalizes` (the finality
//! gadget) as the gate before `apply`, sourcing each validator's PoM weight from
//! `Node::finality_pom_weight` (the cleared-score bridge) — never a hand-built fixture.
//!
//! HONEST GENESIS (why this is the honest chain):
//!   * The ledger starts EMPTY. PoM standing is EARNED by finalized contribution, never pre-minted
//!     (runtime.rs §2.5). So genesis seeds NO PoM and NO tokens.
//!   * A small BONDED PoS set carries finality from block zero, precisely because PoM is empty at
//!     genesis (finality_pom_weight() is empty until cells clear the vesting window; W=0 default).
//!   * The finality RULE is PoS+PoM with PoW EXCLUDED and an anti-concentration floor
//!     (FINALITY_MIX + MIN_DIM_BPS, runtime.rs) — the exact rule the reference suite tests and the
//!     CKB-VM type-script mirrors.
//!
//! HONEST SCOPE (what this is NOT): a single process, no P2P, no persistence, no on-disk chain-spec,
//! no signed token transactions. Those are the T1 testnet build (wire codec + transport + gossip +
//! sync). This is a devnet you can boot and watch produce an honest chain, not a network you can join.

use std::collections::HashMap;

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::runtime::{Constitution, Node};
use noesis::{Cell, Script};

/// A genesis bonded-PoS validator, keyed to a contributor handle. `pos`/`staked_balance` give it
/// bonded capital weight (carries finality at genesis); `pom` starts 0 and is sourced live from the
/// cleared-score bridge each round as this contributor's finalized work ages in.
fn genesis_validator(id: u64) -> Validator {
    Validator {
        id,
        pow: 0.0,
        pos: 1000.0,
        pom: 0.0,
        last_heartbeat: 0,
        staked_balance: 1000.0,
    }
}

/// The honest genesis: an empty ledger (via `Node::new`), the default Constitution (NCI mix,
/// 2/3 bar, W=0, theta=0.95), and a small bonded PoS set each paired with its contributor key.
/// PoM is NOT seeded — standing is earned, not pre-minted.
fn genesis() -> (Node, Vec<Vec<u8>>) {
    let keys: Vec<Vec<u8>> = vec![b"alice".to_vec(), b"bob".to_vec(), b"carol".to_vec()];
    let validators: Vec<Validator> = (0..keys.len() as u64).map(genesis_validator).collect();
    let node = Node::new(0, validators, Constitution::default());
    (node, keys)
}

fn cell(id: u64, owner: &[u8], contributor: &[u8], parent: Option<u64>, ts: u64, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent,
        timestamp: ts,
        data: data.to_vec(),
    }
}

fn secret(b: u8) -> [u8; 32] {
    [b; 32]
}

/// A deterministic scripted workload: distinct content per cell (real temporal novelty), some cells
/// building on earlier ones (provenance edges), attributed to the genesis contributors so their PoM
/// standing grows and flows onto the finality path over the run.
fn workload() -> Vec<Vec<(Cell, Committed)>> {
    vec![
        vec![
            (cell(1, b"al", b"alice", None, 1, b"the quick brown fox jumps high"), Committed { height: 1, secret: secret(11) }),
            (cell(2, b"bo", b"bob", None, 1, b"lorem ipsum dolor sit amet now"), Committed { height: 1, secret: secret(22) }),
        ],
        vec![(cell(3, b"al", b"alice", Some(1), 2, b"fox jumps over the lazy sleeping dog"), Committed { height: 2, secret: secret(33) })],
        vec![
            (cell(4, b"ca", b"carol", None, 3, b"entirely separate subject matter here today"), Committed { height: 3, secret: secret(44) }),
            (cell(5, b"bo", b"bob", Some(2), 3, b"ipsum dolor greatly expanded with extra material"), Committed { height: 3, secret: secret(55) }),
        ],
        vec![(cell(6, b"al", b"alice", None, 4, b"a fresh account of winter mornings and cold light"), Committed { height: 4, secret: secret(66) })],
    ]
}

fn print_state(node: &Node) {
    let (ids, root, pom, _tokens, work) = node.ledger.state_digest();
    let root_hex: String = root.iter().take(6).map(|b| format!("{b:02x}")).collect();
    let mut pom_sorted = pom.clone();
    pom_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let pom_str: Vec<String> = pom_sorted
        .iter()
        .map(|(k, v)| format!("{}={}", String::from_utf8_lossy(k), v))
        .collect();
    println!(
        "  height={} work={} cells={} index_root={}.. pom[{}]",
        node.ledger.height,
        work,
        ids.len(),
        root_hex,
        pom_str.join(" ")
    );
}

fn main() {
    println!("noesisd — Noesis T0 local devnet");
    println!("genesis: empty ledger (PoM earned, not pre-minted) + Constitution::default() + bonded PoS set\n");

    let (mut node, keys) = genesis();
    // id -> contributor key, so each validator's PoM weight is sourced from the cleared-score bridge.
    let key_of: HashMap<u64, Vec<u8>> =
        keys.iter().enumerate().map(|(i, k)| (i as u64, k.clone())).collect();

    println!("booted. block 0 (genesis) state:");
    print_state(&node);
    println!(
        "  validators (bonded PoS, pom sourced live): {}\n",
        (0..keys.len()).map(|i| String::from_utf8_lossy(&keys[i]).to_string()).collect::<Vec<_>>().join(", ")
    );

    let mut finalized = 0u64;
    for (round_ix, proposals) in workload().into_iter().enumerate() {
        let height = round_ix as u64 + 1;

        // intake -> mempool
        for (c, co) in &proposals {
            node.submit(c.clone(), co.clone());
        }

        // leader move: assemble the next block
        let block = node.propose();

        // honest local validation (the vote)
        if !node.validate(&block) {
            println!("block {height}: REJECTED by validation — not applied");
            node.clear_mempool();
            continue;
        }

        // THE FINALITY GATE (the honest-critical wire): source each validator's PoM from the
        // cleared-score bridge, then require the finality gadget to say YES before applying.
        // At genesis this map is empty => pom=0 => bonded PoS carries finality (§2.5).
        let fpw = node.finality_pom_weight();
        let mut validators: Vec<Validator> = (0..keys.len() as u64).map(genesis_validator).collect();
        for v in &mut validators {
            v.pom = key_of
                .get(&v.id)
                .and_then(|k| fpw.get(k))
                .map(|w| *w as f64)
                .unwrap_or(0.0);
        }
        // single node: every honest validator votes for its own valid proposal.
        let voters_for = validators.clone();

        if !node.checkpoint_finalizes(&voters_for, &validators) {
            println!("block {height}: validated but did NOT finalize (finality gadget said no) — not applied");
            node.clear_mempool();
            continue;
        }

        // finalized: apply, clear the round's mempool, expose state.
        node.apply(&block);
        node.clear_mempool();
        finalized += 1;
        println!("block {height}: FINALIZED ({} cells)", proposals.len());
        print_state(&node);
    }

    println!("\ndevnet run complete: {finalized} blocks finalized, chain height {}.", node.ledger.height);
    println!("finality was GATED on the finality gadget every block — not stamped on validation alone.");
    let total_pom: u64 = node.ledger.pom.values().sum();
    println!("PoM standing earned by contribution (not pre-minted): {total_pom} total across {} contributors.", node.ledger.pom.len());
}
