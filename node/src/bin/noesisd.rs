//! noesisd — the Noesis node driver.
//!
//! Three modes (dispatched on argv):
//!   * `noesisd` — T0 local devnet: one process boots an honest genesis, produces + finality-gates +
//!     applies a scripted chain, and prints its state.
//!   * `noesisd --listen [addr] [store]` — T1 SEED: serve a canonical block log to any joiner over
//!     framed TCP (default `addr` = an OS ephemeral port). With `store` it serves the DURABLE log the
//!     `--serve-api` node persisted (slice-5b unify — ONE on-disk chain); with no store it builds the
//!     scripted honest chain in-memory (the zero-config demo). "A network you can join."
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

use std::io::{self, Write};
use std::sync::Arc;
use std::thread;

use noesis::chainspec::ChainSpec;
use noesis::commit_order::Committed;
use noesis::net::{Listener, Peer};
use noesis::runtime::{Block, Node};
use noesis::sync::{serve, sync_from};
use noesis::{Cell, Script};

/// The honest genesis, now single-sourced from [`ChainSpec::dev`]: an empty ledger, the ratified M3
/// economics turned ON (PoW enforced ⇒ mined difficulty ⇒ JUL issues from block zero), and a small
/// bonded PoS set each paired with its contributor key. PoM is NOT seeded — standing is earned, not
/// pre-minted. A seed and a joiner MUST boot the same spec — genesis agreement is what makes the
/// digests comparable at all.
fn genesis() -> (Node, Vec<Vec<u8>>) {
    ChainSpec::dev().genesis_node()
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
    let (ids, root, pom, tokens, work) = node.ledger.state_digest();
    let root_hex: String = root.iter().take(6).map(|b| format!("{b:02x}")).collect();
    let mut pom_sorted = pom.clone();
    pom_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let pom_str: Vec<String> = pom_sorted
        .iter()
        .map(|(k, v)| format!("{}={}", String::from_utf8_lossy(k), v))
        .collect();
    // JUL issued so far, in whole JUL (base units / 10^8) — minted from the block's mined work.
    let jul = node.ledger.jul_supply.issued() / noesis::jul::JUL_BASE_UNITS;
    println!(
        "  height={} work={} cells={} jul={} coinbase_cells={} index_root={}.. pom[{}]",
        node.ledger.height,
        work,
        ids.len(),
        jul,
        tokens.len(),
        root_hex,
        pom_str.join(" ")
    );
}

/// Build the honest chain: run the scripted workload through the FULL proven pipeline
/// (submit -> propose -> validate -> finality-gate -> apply), collecting the finalized blocks in
/// canonical order. Shared by all three modes — the devnet narrates it, the seed serves the blocks.
/// `verbose` prints per-block progress (devnet) vs staying quiet (seed banner handles its own lines).
fn build_chain(verbose: bool) -> (Node, Vec<Block>) {
    let spec = ChainSpec::dev();
    let (mut node, _keys) = spec.genesis_node();

    let mut blocks = Vec::new();
    for (round_ix, proposals) in workload().into_iter().enumerate() {
        let height = round_ix as u64 + 1;
        let n = proposals.len();

        // intake -> mempool, then run the ONE proven per-block engine (mine -> validate ->
        // finality-gate -> apply), single-sourced in ChainSpec::produce_block.
        for (c, co) in proposals {
            node.submit(c, co);
        }
        match spec.produce_block(&mut node) {
            Some(block) => {
                blocks.push(block);
                if verbose {
                    println!("block {height}: FINALIZED ({n} cells)");
                    print_state(&node);
                }
            }
            None if verbose => println!("block {height}: not finalized — not applied"),
            None => {}
        }
    }
    (node, blocks)
}

/// No-arg mode: the T0 devnet — boot an honest genesis, produce the scripted chain, print state.
fn run_devnet() {
    println!("noesisd — Noesis T0 local devnet");
    println!("genesis: empty ledger (PoM earned, not pre-minted) + ChainSpec::dev() [PoW enforced, JUL issuing] + bonded PoS set\n");
    println!("booted. producing the honest chain (finality GATED every block):\n");

    let (node, blocks) = build_chain(true);

    println!("\ndevnet run complete: {} blocks finalized, chain height {}.", blocks.len(), node.ledger.height);
    println!("finality was GATED on the finality gadget every block — not stamped on validation alone.");
    let total_pom: u64 = node.ledger.pom.values().sum();
    println!("PoM standing earned by contribution (not pre-minted): {total_pom} total across {} contributors.", node.ledger.pom.len());
    let jul = node.ledger.jul_supply.issued() / noesis::jul::JUL_BASE_UNITS;
    println!("JUL issued from mined work (energy-anchored, no pre-mine): {jul} JUL over {} blocks.", blocks.len());
}

/// Which genesis a store-backed / durable mode boots, selected by `NOESIS_NET` (default dev = safe
/// local; `testnet` for the public chain). Single-sourced so a store-backed `--listen` and
/// `--serve-api` can never disagree on which chain-zero the SAME on-disk log replays against.
fn spec_from_env() -> ChainSpec {
    match std::env::var("NOESIS_NET").as_deref() {
        Ok("testnet") => ChainSpec::testnet(),
        Ok("dev") | Err(_) => ChainSpec::dev(),
        Ok(other) => {
            eprintln!("noesisd: unknown NOESIS_NET={other:?} (use 'dev' or 'testnet')");
            std::process::exit(2);
        }
    }
}

/// `--listen [addr] [store]` — the SEED. Serve a canonical block log to any joiner. Prints `DIGEST
/// <...>` (its converged state) and `LISTENING <addr>` (the bound address, resolved from an ephemeral
/// `:0` if given) so a joiner — or the two-node test — can find it, then loops serving each joiner the
/// whole log (slice-4 `serve`). Runs until killed.
///
/// slice-5b UNIFY: with a `store` path, the seed serves the DURABLE chain the `--serve-api` node
/// persisted (`store::load_blocks`, the length-framed log) — ONE on-disk chain, not a second scripted
/// driver. With no store (the zero-config demo), it builds the scripted honest chain in-memory.
fn run_listen(addr: &str, store: Option<&str>) {
    println!("noesisd --listen — Noesis T1 seed node");
    let (digest, blocks) = match store {
        Some(path) => {
            let spec = spec_from_env();
            let (node, blocks) = noesis::store::load_blocks(std::path::Path::new(path), &spec)
                .unwrap_or_else(|e| {
                    eprintln!("noesisd: failed to open chain store {path}: {e}");
                    std::process::exit(1);
                });
            if blocks.is_empty() {
                println!("durable store {path} empty/absent — serving an empty chain (height 0).");
            } else {
                println!("serving durable chain from {path}: {} blocks, height {}.", blocks.len(), node.ledger.height);
            }
            (digest_string(&node), blocks)
        }
        None => {
            let (seed, blocks) = build_chain(false);
            println!("seeded honest chain (scripted demo): {} blocks, height {}.", blocks.len(), seed.ledger.height);
            (digest_string(&seed), blocks)
        }
    };
    // Machine-readable convergence anchor (a joiner must reproduce this exactly).
    println!("DIGEST {digest}");

    let listener = Listener::bind(addr).unwrap_or_else(|e| {
        eprintln!("noesisd: failed to bind {addr}: {e}");
        std::process::exit(1);
    });
    let bound = listener.local_addr().expect("listener has a local address");
    // Emit the resolved address AFTER binding so `:0` (ephemeral) is usable by a joiner/test.
    println!("LISTENING {bound}");

    // Serve each joiner on its OWN thread (Council C2): a slow or stalled peer can no longer starve
    // the others, because it no longer blocks the accept loop. Combined with per-socket I/O deadlines
    // (`net::IO_TIMEOUT`, Council C1), a hung peer's serve thread simply times out and dies. `blocks`
    // is read-only ⇒ shared via `Arc`, no per-joiner copy. Diagnostics use `let _ = writeln!` (never
    // `println!`): a daemon must not crash because its stdout was closed.
    // Residual (honest): thread-per-connection is unbounded — a connection flood spawns unbounded
    // threads. A bounded pool / connection cap is the next hardening if a public seed needs it.
    let blocks = Arc::new(blocks);
    loop {
        match listener.accept() {
            Ok(mut peer) => {
                let who = peer.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into());
                let blocks = Arc::clone(&blocks);
                thread::spawn(move || match serve(&mut peer, &blocks) {
                    Ok(()) => {
                        let _ = writeln!(io::stdout(), "served block log to joiner {who} ({} blocks)", blocks.len());
                    }
                    Err(e) => {
                        let _ = writeln!(io::stderr(), "noesisd: serve to {who} failed: {e}");
                    }
                });
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
        Some("--listen") => run_listen(
            args.get(2).map(String::as_str).unwrap_or("127.0.0.1:0"),
            args.get(3).map(String::as_str),
        ),
        Some("--connect") => {
            let addr = args.get(2).unwrap_or_else(|| {
                eprintln!("noesisd --connect needs a <addr> (e.g. 127.0.0.1:9944)");
                std::process::exit(2);
            });
            run_connect(addr);
        }
        Some("--serve-api") => {
            // Which genesis this durable node boots is selected by NOESIS_NET (default dev = safe
            // local). A public testnet node sets NOESIS_NET=testnet so a stranger who joins boots the
            // ONE testnet block-zero (distinct chain_id ⇒ testnet blocks never cross to dev/mainnet).
            let spec = spec_from_env();
            noesis::rpc::serve_api(
                args.get(2).map(String::as_str).unwrap_or("127.0.0.1:9955"),
                args.get(3).map(String::as_str).unwrap_or("noesis-chain.log"),
                spec,
            );
        }
        Some(other) => {
            eprintln!("noesisd: unknown mode {other:?}");
            eprintln!("usage: noesisd                    # T0 devnet (produce a chain locally)");
            eprintln!("       noesisd --listen [addr] [store]  # T1 seed (default 127.0.0.1:0; store ⇒ serve the durable log)");
            eprintln!("       noesisd --connect <addr>   # T1 joiner (sync from a seed)");
            eprintln!("       noesisd --serve-api [addr] [store]  # live HTTP API + embedded UI (NOESIS_NET=dev|testnet, default dev)");
            std::process::exit(2);
        }
    }
}
