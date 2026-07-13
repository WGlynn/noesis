//! noesisd — the Noesis node driver.
//!
//! Three modes (dispatched on argv):
//!   * `noesisd` — T0 local devnet: one process boots an honest genesis, produces + finality-gates +
//!     applies a scripted chain, and prints its state.
//!   * `noesisd --listen [addr]` — T1 SEED: build that chain, then bind `addr` (default an OS ephemeral
//!     port) and serve the canonical block log to any joiner over framed TCP. "A network you can join."
//!   * `noesisd --connect <addr>` — T1 JOINER: boot a fresh genesis, dial the seed, pull + re-validate +
//!     replay its block log, and converge to a byte-identical digest.
//!
//! The devnet (no-arg) path is UNCHANGED from the T0 driver: it wraps the proven
//! submit->propose->validate->finalize->apply sequence (node/tests/two_node.rs) in a real `fn main`
//! with ZERO new consensus mechanism, and gates finality on `Node::checkpoint_finalizes` every block.
//!
//! T1 slice-5 (this file's new modes) adds NO consensus mechanism either — it reuses slices 1-4
//! wholesale: `wire` (encode/decode + the on-disk shape), `net` (framed TCP), `sync` (the join). The
//! seed's `serve` and the joiner's `sync_from` are the slice-4 primitives; here they run in two real OS
//! processes instead of two threads, which is the actual "join a running node" demo.
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
//! HONEST SCOPE (what slice-5 is NOT): the joiner does a full replay-from-genesis sync (no
//! headers-first, no range requests, no fork choice — there is one canonical log). And it is
//! HISTORICAL sync only: a block produced AFTER a peer has joined is not yet gossiped to it live —
//! wiring `gossip`'s reader loop into a running node (so new blocks propagate to already-joined peers)
//! is slice-5b. What IS proven here: two OS processes converge to identical state via the join.

use std::collections::HashMap;
use std::io::{self, Write};

use noesis::commit_order::Committed;
use noesis::consensus::Validator;
use noesis::net::{Listener, Peer};
use noesis::runtime::{Block, Constitution, Node};
use noesis::sync::{serve, sync_from};
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
/// PoM is NOT seeded — standing is earned, not pre-minted. A seed and a joiner MUST call this
/// identically (same validators, same Constitution) — genesis agreement is what makes the digests
/// comparable at all.
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

/// A canonical, cross-process-comparable rendering of the full `state_digest` tuple (finalized cell
/// ids, novelty-index root, sorted PoM map, token-cell ids, cumulative-work clock). Two nodes print
/// the SAME string iff they converged on byte-identical state — this is what the two-process join test
/// compares, and it captures every component the in-process convergence harnesses assert on.
fn digest_string(node: &Node) -> String {
    let (ids, root, pom, tokens, work) = node.ledger.state_digest();
    let root_hex: String = root.iter().map(|b| format!("{b:02x}")).collect();
    let pom_str: Vec<String> =
        pom.iter().map(|(k, v)| format!("{}={}", String::from_utf8_lossy(k), v)).collect();
    format!(
        "root={root_hex} height={} work={work} cells={ids:?} tokens={tokens:?} pom=[{}]",
        node.ledger.height,
        pom_str.join(",")
    )
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

/// Build the honest chain: run the scripted workload through the FULL proven pipeline
/// (submit -> propose -> validate -> finality-gate -> apply), collecting the finalized blocks in
/// canonical order. Shared by all three modes — the devnet narrates it, the seed serves the blocks.
/// `verbose` prints per-block progress (devnet) vs staying quiet (seed banner handles its own lines).
fn build_chain(verbose: bool) -> (Node, Vec<Block>) {
    let (mut node, keys) = genesis();
    // id -> contributor key, so each validator's PoM weight is sourced from the cleared-score bridge.
    let key_of: HashMap<u64, Vec<u8>> =
        keys.iter().enumerate().map(|(i, k)| (i as u64, k.clone())).collect();

    let mut blocks = Vec::new();
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
            if verbose {
                println!("block {height}: REJECTED by validation — not applied");
            }
            node.clear_mempool();
            continue;
        }

        // THE FINALITY GATE: source each validator's PoM from the cleared-score bridge, then require
        // the finality gadget to say YES before applying. At genesis this map is empty => pom=0 =>
        // bonded PoS carries finality (§2.5).
        let fpw = node.finality_pom_weight();
        let mut validators: Vec<Validator> = (0..keys.len() as u64).map(genesis_validator).collect();
        for v in &mut validators {
            v.pom = key_of
                .get(&v.id)
                .and_then(|k| fpw.get(k))
                .map(|w| *w as f64)
                .unwrap_or(0.0);
        }
        // single proposer: every honest validator votes for its own valid proposal.
        let voters_for = validators.clone();

        if !node.checkpoint_finalizes(&voters_for, &validators) {
            if verbose {
                println!("block {height}: validated but did NOT finalize — not applied");
            }
            node.clear_mempool();
            continue;
        }

        // finalized: apply, clear the round's mempool, record the canonical block.
        node.apply(&block);
        node.clear_mempool();
        blocks.push(block);
        if verbose {
            println!("block {height}: FINALIZED ({} cells)", proposals.len());
            print_state(&node);
        }
    }
    (node, blocks)
}

/// No-arg mode: the T0 devnet — boot an honest genesis, produce the scripted chain, print state.
fn run_devnet() {
    println!("noesisd — Noesis T0 local devnet");
    println!("genesis: empty ledger (PoM earned, not pre-minted) + Constitution::default() + bonded PoS set\n");
    println!("booted. producing the honest chain (finality GATED every block):\n");

    let (node, blocks) = build_chain(true);

    println!("\ndevnet run complete: {} blocks finalized, chain height {}.", blocks.len(), node.ledger.height);
    println!("finality was GATED on the finality gadget every block — not stamped on validation alone.");
    let total_pom: u64 = node.ledger.pom.values().sum();
    println!("PoM standing earned by contribution (not pre-minted): {total_pom} total across {} contributors.", node.ledger.pom.len());
}

/// `--listen [addr]` — the SEED. Build the canonical chain, then serve its block log to any joiner.
/// Prints `DIGEST <...>` (its converged state) and `LISTENING <addr>` (the bound address, resolved
/// from an ephemeral `:0` if given) so a joiner — or the two-node test — can find it. Then loops
/// accepting joiners and serving each the whole log (slice-4 `serve`). Runs until killed.
fn run_listen(addr: &str) {
    println!("noesisd --listen — Noesis T1 seed node");
    let (seed, blocks) = build_chain(false);
    println!("seeded honest chain: {} blocks, height {}.", blocks.len(), seed.ledger.height);
    // Machine-readable convergence anchor (a joiner must reproduce this exactly).
    println!("DIGEST {}", digest_string(&seed));

    let listener = Listener::bind(addr).unwrap_or_else(|e| {
        eprintln!("noesisd: failed to bind {addr}: {e}");
        std::process::exit(1);
    });
    let bound = listener.local_addr().expect("listener has a local address");
    // Emit the resolved address AFTER binding so `:0` (ephemeral) is usable by a joiner/test.
    println!("LISTENING {bound}");

    // Serve joiners sequentially: each gets the full canonical block log (read-only; no state change,
    // so any number of joiners converge to the same digest). A per-joiner failure is logged, not fatal.
    // Diagnostics use `let _ = writeln!` (never `println!`): a daemon must not crash because its stdout
    // was closed (e.g. a supervisor that only wanted the LISTENING line and dropped the pipe).
    loop {
        match listener.accept() {
            Ok(mut peer) => {
                let who = peer.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into());
                match serve(&mut peer, &blocks) {
                    Ok(()) => {
                        let _ = writeln!(io::stdout(), "served block log to joiner {who} ({} blocks)", blocks.len());
                    }
                    Err(e) => {
                        let _ = writeln!(io::stderr(), "noesisd: serve to {who} failed: {e}");
                    }
                }
            }
            Err(e) => {
                let _ = writeln!(io::stderr(), "noesisd: accept failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// `--connect <addr>` — the JOINER. Boot a fresh genesis identical to the seed's, dial it, pull +
/// re-validate + replay its block log (slice-4 `sync_from`), and converge. Prints `DIGEST <...>` — a
/// joiner that reached the seed's state prints the SAME line the seed did. Exits 0 on success.
fn run_connect(addr: &str) {
    println!("noesisd --connect — Noesis T1 joiner (fresh genesis, syncing from {addr})");
    let (mut joiner, _keys) = genesis();

    let mut peer = Peer::connect(addr).unwrap_or_else(|e| {
        eprintln!("noesisd: failed to connect to {addr}: {e}");
        std::process::exit(1);
    });
    let applied = sync_from(&mut peer, &mut joiner).unwrap_or_else(|e| {
        eprintln!("noesisd: sync failed: {e}");
        std::process::exit(1);
    });

    println!("synced: applied {applied} blocks, converged to height {}.", joiner.ledger.height);
    println!("DIGEST {}", digest_string(&joiner));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        None => run_devnet(),
        Some("--listen") => run_listen(args.get(2).map(String::as_str).unwrap_or("127.0.0.1:0")),
        Some("--connect") => {
            let addr = args.get(2).unwrap_or_else(|| {
                eprintln!("noesisd --connect needs a <addr> (e.g. 127.0.0.1:9944)");
                std::process::exit(2);
            });
            run_connect(addr);
        }
        Some(other) => {
            eprintln!("noesisd: unknown mode {other:?}");
            eprintln!("usage: noesisd                  # T0 devnet (produce a chain locally)");
            eprintln!("       noesisd --listen [addr]  # T1 seed (default 127.0.0.1:0)");
            eprintln!("       noesisd --connect <addr> # T1 joiner (sync from a seed)");
            std::process::exit(2);
        }
    }
}
