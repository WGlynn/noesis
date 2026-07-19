//! T1 slice-5b — `--listen [addr] [store]` serves the DURABLE chain (the unify).
//!
//! Before this, `--listen` built its own scripted in-memory chain, so the join demo and the durable
//! `--serve-api` node were two separate drivers serving DIFFERENT chains. This proves the unify end to
//! end across real OS processes: we build a durable block log on disk (the exact length-framed shape
//! `store::append_block` / `--serve-api` writes), point a real `noesisd --listen <addr> <store>`
//! process at it, and a `--connect` joiner converges to the DURABLE chain's digest — a 3-cell chain,
//! distinguishable from the 6-cell scripted demo the bare `--listen` still serves.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use noesis::chainspec::ChainSpec;
use noesis::commit_order::Committed;
use noesis::{Cell, Script};

/// Build a durable, length-framed block log at `path` with `n` finalized contribution blocks, via the
/// SAME `store::append_block` path the live node uses. Returns nothing — the seed process reads it.
fn build_durable_store(path: &Path, n: u64) {
    let spec = ChainSpec::dev();
    let (mut node, _keys) = spec.genesis_node();
    for i in 1..=n {
        node.submit(
            Cell {
                id: i,
                lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
                type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
                parent: None,
                timestamp: i,
                data: format!("a distinct contribution number {i} about winter light").into_bytes(),
            },
            Committed { height: i, secret: [i as u8; 32] },
        );
        let block = spec.produce_block(&mut node).expect("block finalizes");
        noesis::store::append_block(path, &block).expect("append to durable store");
    }
}

/// Read the seed's stdout until we have BOTH its `LISTENING <addr>` and `DIGEST <...>` lines. Panics
/// (loudly, with the lines seen) if the process ends before emitting both — never a silent hang.
fn read_seed_ready(child: &mut Child) -> (String, String) {
    let stdout = child.stdout.take().expect("seed stdout is piped");
    let reader = BufReader::new(stdout);
    let (mut addr, mut digest): (Option<String>, Option<String>) = (None, None);
    let mut seen: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line.expect("read seed stdout line");
        if let Some(rest) = line.strip_prefix("LISTENING ") {
            addr = Some(rest.trim().to_string());
        }
        if let Some(rest) = line.strip_prefix("DIGEST ") {
            digest = Some(rest.trim().to_string());
        }
        seen.push(line);
        if let (Some(a), Some(d)) = (&addr, &digest) {
            return (a.clone(), d.clone());
        }
    }
    panic!("seed ended before emitting LISTENING + DIGEST; saw:\n{}", seen.join("\n"));
}

fn extract_digest(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("DIGEST ").map(|r| r.trim().to_string()))
        .unwrap_or_else(|| panic!("no DIGEST line in output:\n{stdout}"))
}

fn tmp_store(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    p.push(format!("noesis-listen-store-{tag}-{nanos}.log"));
    p
}

#[test]
fn listen_serves_the_durable_store_and_joiner_converges() {
    let bin = env!("CARGO_BIN_EXE_noesisd");
    let store = tmp_store("unify");
    build_durable_store(&store, 3);

    // 1) Seed serves the DURABLE log (not the scripted demo) on an ephemeral port.
    let mut seed = Command::new(bin)
        .arg("--listen")
        .arg("127.0.0.1:0")
        .arg(&store)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn seed noesisd --listen <store>");
    let (addr, seed_digest) = read_seed_ready(&mut seed);

    // 2) Joiner boots a fresh genesis, dials the seed, replays, and exits.
    let joiner = Command::new(bin)
        .arg("--connect")
        .arg(&addr)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .expect("run joiner noesisd --connect");

    // 3) Reap the seed (its accept loop runs forever by design).
    let _ = seed.kill();
    let _ = seed.wait();
    let _ = std::fs::remove_file(&store);

    assert!(joiner.status.success(), "joiner process exited non-zero: {:?}", joiner.status);
    let joiner_out = String::from_utf8_lossy(&joiner.stdout);
    let joiner_digest = extract_digest(&joiner_out);

    // Convergence: the two independent processes agree on the full canonical digest.
    assert_eq!(
        seed_digest, joiner_digest,
        "seed and joiner converged to DIFFERENT state:\n seed:   {seed_digest}\n joiner: {joiner_digest}"
    );
    // And it is the DURABLE 3-cell chain — proving the seed served the store, NOT the 6-cell scripted demo.
    assert!(
        joiner_digest.contains("cells=[1, 2, 3]") && joiner_digest.contains("height=3"),
        "expected the durable 3-cell chain (height=3); digest was: {joiner_digest}"
    );
}
