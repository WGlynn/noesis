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
    build_durable_store_on(ChainSpec::dev(), path, n)
}

/// [`build_durable_store`] on a chosen chain spec.
fn build_durable_store_on(spec: ChainSpec, path: &Path, n: u64) {
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

/// A faucet seed file holding the PUBLIC test seed. A testnet `--serve-api` refuses to start without a
/// faucet seed, and a test must never fall through to the real `~/.noesis` path.
fn test_faucet_seed(tag: &str) -> PathBuf {
    let p = tmp_store(&format!("{tag}-faucet-seed"));
    let hex: String = noesis::faucet::DEV_FAUCET_SEED.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::write(&p, hex).expect("write the test faucet seed");
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

/// The testnet joiner (found 2026-11-17 while writing the run-a-node guide): `--connect` booted the
/// DEV genesis whatever `NOESIS_NET` said, while `--listen <store>` booted the spec `NOESIS_NET`
/// selects. So a stranger joining the testnet replayed testnet blocks onto a dev genesis and could
/// not converge. Both sides now boot the same spec.
#[test]
fn testnet_joiner_boots_the_testnet_genesis_and_converges() {
    let bin = env!("CARGO_BIN_EXE_noesisd");
    let store = tmp_store("testnet");
    build_durable_store_on(ChainSpec::testnet(), &store, 2);
    let mut seed = Command::new(bin)
        .env("NOESIS_NET", "testnet")
        .arg("--listen")
        .arg("127.0.0.1:0")
        .arg(&store)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn testnet seed");
    let (addr, seed_digest) = read_seed_ready(&mut seed);
    let joiner = Command::new(bin)
        .env("NOESIS_NET", "testnet")
        .arg("--connect")
        .arg(&addr)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .expect("run testnet joiner");
    let _ = seed.kill();
    let _ = seed.wait();
    let _ = std::fs::remove_file(&store);
    assert!(joiner.status.success(), "testnet joiner exited non-zero: {:?}", joiner.status);
    let joiner_digest = extract_digest(&String::from_utf8_lossy(&joiner.stdout));
    assert_eq!(seed_digest, joiner_digest, "testnet seed and joiner converged to different state");
    assert!(joiner_digest.contains("height=2"), "expected the 2-block testnet chain; digest was: {joiner_digest}");
}

/// Testnet step 6: one process serves the page AND joiners. `--serve-api <http> <store> <peers>` opens
/// a peers listener beside the HTTP API, and a `--connect` joiner converges to the durable chain.
#[test]
fn serve_api_also_serves_joiners_from_one_process() {
    let bin = env!("CARGO_BIN_EXE_noesisd");
    let store = tmp_store("one-process");
    build_durable_store_on(ChainSpec::testnet(), &store, 2);
    let seed = test_faucet_seed("one-process");
    let mut node = Command::new(bin)
        .env("NOESIS_NET", "testnet")
        .env("NOESIS_FAUCET_SEED", &seed)
        .arg("--serve-api")
        .arg("127.0.0.1:0")
        .arg(&store)
        .arg("127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn noesisd --serve-api with a peers address");
    let mut lines = BufReader::new(node.stdout.take().expect("piped stdout")).lines();
    let peers = loop {
        let line = lines.next().expect("node exited before announcing its peers address").expect("read stdout");
        if let Some(a) = line.strip_prefix("PEERS LISTENING ") {
            break a.trim().to_string();
        }
    };
    let joiner = Command::new(bin)
        .env("NOESIS_NET", "testnet")
        .arg("--connect")
        .arg(&peers)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .expect("run joiner");
    let _ = node.kill();
    let _ = node.wait();
    let _ = std::fs::remove_file(&store);
    let _ = std::fs::remove_file(&seed);
    assert!(joiner.status.success(), "joiner exited non-zero: {:?}", joiner.status);
    let digest = extract_digest(&String::from_utf8_lossy(&joiner.stdout));
    assert!(digest.contains("height=2") && digest.contains("cells=[1, 2]"), "expected the 2-block durable chain; digest was: {digest}");
}

/// The files a boot set aside next to `log` (`<log name>.torn-at-<offset>-<time>`).
fn aside_files(log: &Path) -> Vec<PathBuf> {
    let prefix = format!("{}.torn-at-", log.file_name().unwrap().to_string_lossy());
    std::fs::read_dir(log.parent().unwrap())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&prefix)))
        .collect()
}

/// The whole frames of a clean log: a 4-byte big-endian length, then that many bytes.
fn frames_of(path: &Path) -> Vec<Vec<u8>> {
    let bytes = std::fs::read(path).unwrap();
    let (mut out, mut at) = (Vec::new(), 0usize);
    while at < bytes.len() {
        let len = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        out.push(bytes[at..at + 4 + len].to_vec());
        at += 4 + len;
    }
    out
}

/// A crash mid-write tears the last record of the chain log. A real `--serve-api` process sets those bytes
/// aside in a file next to the log, cuts the log back to its last whole record, says so in one line, and
/// starts on the whole blocks.
#[test]
fn serve_api_sets_a_torn_tail_aside_and_starts_on_the_whole_blocks() {
    let bin = env!("CARGO_BIN_EXE_noesisd");
    let store = tmp_store("torn");
    build_durable_store(&store, 2);
    let whole = std::fs::read(&store).unwrap();
    let torn = [&999u32.to_be_bytes()[..], b"{\"height\":3,\"cel"].concat();
    let mut bytes = whole.clone();
    bytes.extend_from_slice(&torn);
    std::fs::write(&store, &bytes).unwrap();
    let mut node = Command::new(bin)
        .env("NOESIS_NET", "dev")
        .arg("--serve-api")
        .arg("127.0.0.1:0")
        .arg(&store)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn noesisd --serve-api");
    let mut seen = Vec::new();
    for line in BufReader::new(node.stdout.take().expect("piped stdout")).lines() {
        let line = line.expect("read stdout");
        let up = line.starts_with("LISTENING http://");
        seen.push(line);
        if up {
            break;
        }
    }
    let _ = node.kill();
    let _ = node.wait();
    let mut err = String::new();
    std::io::Read::read_to_string(&mut node.stderr.take().unwrap(), &mut err).unwrap();
    let aside = aside_files(&store);
    let on_disk = std::fs::read(&store).unwrap();
    let held = aside.first().map(|a| std::fs::read(a).unwrap());
    for a in &aside {
        let _ = std::fs::remove_file(a);
    }
    let _ = std::fs::remove_file(&store);
    let seen = seen.join("\n");
    assert!(seen.contains("replayed 2 blocks, height 2") && seen.contains("LISTENING http://"), "stdout:\n{seen}\nstderr:\n{err}");
    assert!(on_disk == whole, "the log is cut back to its last whole record: {} bytes, {} whole", on_disk.len(), whole.len());
    assert_eq!(aside.len(), 1, "one set-aside file; stderr:\n{err}");
    assert!(held.unwrap() == torn, "it holds the torn bytes");
    let name = aside[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(err.contains("torn record") && err.contains(&name) && err.lines().filter(|l| l.contains("torn")).count() == 1, "one plain line names the file:\n{err}");
}

/// Whole blocks after a broken record are damage, not a crash (a crash tears only the last record). A real
/// `--serve-api` process refuses to start, says why, and leaves the file alone.
#[test]
fn serve_api_refuses_to_start_on_whole_blocks_after_a_broken_record() {
    let bin = env!("CARGO_BIN_EXE_noesisd");
    let built = tmp_store("damaged-src");
    build_durable_store(&built, 4);
    let f = frames_of(&built);
    let store = tmp_store("damaged");
    let torn = [&999u32.to_be_bytes()[..], b"{\"height\":3"].concat();
    std::fs::write(&store, [f[0].clone(), f[1].clone(), torn, f[2].clone(), f[3].clone()].concat()).unwrap();
    let before = std::fs::read(&store).unwrap();
    let mut node = Command::new(bin)
        .env("NOESIS_NET", "dev")
        .arg("--serve-api")
        .arg("127.0.0.1:0")
        .arg(&store)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn noesisd --serve-api");
    // It must exit on its own; a node that started on the damaged log would serve forever.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let status = loop {
        if let Some(s) = node.try_wait().expect("poll the node") {
            break Some(s);
        }
        if std::time::Instant::now() > deadline {
            let _ = node.kill();
            let _ = node.wait();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let (mut out, mut err) = (String::new(), String::new());
    std::io::Read::read_to_string(&mut node.stdout.take().unwrap(), &mut out).unwrap();
    std::io::Read::read_to_string(&mut node.stderr.take().unwrap(), &mut err).unwrap();
    let after = std::fs::read(&store).unwrap();
    let aside = aside_files(&store);
    let _ = (std::fs::remove_file(&store), std::fs::remove_file(&built));
    let status = status.unwrap_or_else(|| panic!("the node started on a damaged log and kept running; stdout:\n{out}"));
    assert!(!status.success(), "a damaged log must not start a node; stderr:\n{err}");
    let at = f[0].len() + f[1].len();
    assert!(err.contains(&format!("damaged at byte {at}")) && err.contains("file is unchanged"), "says why:\n{err}");
    assert!(!out.contains("LISTENING http"), "it never opened the API");
    assert!(after == before, "the file is untouched");
    assert!(aside.is_empty(), "nothing is set aside");
}

/// The testnet pays its block reward to the faucet, so `--serve-api` on it refuses to start without the
/// faucet seed (fail loud, never a silent fallback to the placeholder key), and says why.
#[test]
fn testnet_serve_api_refuses_to_start_without_the_faucet_seed() {
    let bin = env!("CARGO_BIN_EXE_noesisd");
    let store = tmp_store("no-seed");
    let missing = tmp_store("no-seed-faucet-seed"); // never written
    let out = Command::new(bin)
        .env("NOESIS_NET", "testnet")
        .env("NOESIS_FAUCET_SEED", &missing)
        .arg("--serve-api")
        .arg("127.0.0.1:0")
        .arg(&store)
        .output()
        .expect("run noesisd --serve-api");
    let _ = std::fs::remove_file(&store);
    assert!(!out.status.success(), "a testnet node without the faucet seed must not start");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("faucet seed"), "the error names the missing seed: {err}");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("LISTENING"), "it never opened the API");
}
