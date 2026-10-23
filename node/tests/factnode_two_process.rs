//! The day-30 devnet bar, first cut (`docs/DESIGN-layer-map.md` section 7): two separate `factnode`
//! processes that share nothing but a TCP connection swap their contribution facts, and both end on
//! the same merged hash. Each then replays its own wire trace once per seed in the public list
//! (`docs/devnet/shuffle-seeds.txt`), and every replay must land on that same hash.
//!
//! What this does NOT show: two machines (both processes run here, over loopback; the code path is the
//! same with a real address), signatures (a record forged under someone else's key would be accepted),
//! catch-up, or more than one exchange.

use std::io::{BufRead, BufReader, Lines};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, Output, Stdio};

use noesis::factsync::{demo_facts, hex};
use noesis::reconcile::merge;

const BIN: &str = env!("CARGO_BIN_EXE_factnode");

fn tmp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("noesis-factnode-{tag}-{nanos}.trace"))
}

fn seeds() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/devnet/shuffle-seeds.txt")
}

/// The value after `KEY ` on the first line that starts with it. Panics with the whole output if absent.
fn field(out: &str, key: &str) -> String {
    out.lines()
        .find_map(|l| l.strip_prefix(&format!("{key} ")).map(str::to_string))
        .unwrap_or_else(|| panic!("no {key} line in:\n{out}"))
}

/// Start `serve` on an ephemeral port and return it with the address it printed.
fn start_server(demo: &str, trace: &PathBuf) -> (Child, Lines<BufReader<ChildStdout>>, String, Vec<String>) {
    let mut child = Command::new(BIN)
        .args(["serve", "127.0.0.1:0", "--demo", demo, "--trace"])
        .arg(trace)
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn factnode serve");
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut seen = Vec::new();
    for line in lines.by_ref() {
        let line = line.expect("read serve stdout");
        seen.push(line.clone());
        if let Some(addr) = line.strip_prefix("LISTENING ") {
            return (child, lines, addr.to_string(), seen);
        }
    }
    panic!("serve exited before LISTENING; saw:\n{}", seen.join("\n"));
}

/// Run `serve <server_demo>` and `connect <client_demo>` as two processes; return both outputs.
fn run_pair(server_demo: &str, client_demo: &str, client_junk: bool) -> (String, String, PathBuf, PathBuf) {
    let (ts, tc) = (tmp(&format!("serve-{server_demo}")), tmp(&format!("connect-{client_demo}")));
    let (mut server, rest, addr, mut seen) = start_server(server_demo, &ts);
    let mut cmd = Command::new(BIN);
    cmd.args(["connect", &addr, "--demo", client_demo, "--trace"]).arg(&tc);
    if client_junk {
        cmd.arg("--junk");
    }
    let client: Output = cmd.output().expect("run factnode connect");
    if !client.status.success() {
        let _ = server.kill();
        panic!("connect failed: {}", String::from_utf8_lossy(&client.stderr));
    }
    seen.extend(rest.map(|l| l.expect("read serve stdout")));
    assert!(server.wait().expect("serve exits").success(), "serve failed:\n{}", seen.join("\n"));
    (seen.join("\n"), String::from_utf8(client.stdout).unwrap(), ts, tc)
}

fn replay(trace: &PathBuf) -> String {
    let out = Command::new(BIN).arg("replay").arg(trace).arg("--seeds").arg(seeds()).output().expect("run replay");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "replay failed:\n{text}\n{}", String::from_utf8_lossy(&out.stderr));
    text
}

#[test]
fn two_processes_swap_facts_over_tcp_and_every_seeded_replay_lands_on_one_hash() {
    let (srv, cli, ts, tc) = run_pair("a", "b", true);

    // Both processes end on the same merged set, and it is the union the in-process merge computes.
    let expected = merge(&demo_facts("a").unwrap(), &demo_facts("b").unwrap());
    let want = format!("{} {}", expected.len(), hex(&expected.digest()));
    assert_eq!(field(&srv, "MERGED"), want, "serve:\n{srv}");
    assert_eq!(field(&cli, "MERGED"), want, "connect:\n{cli}");

    // The client's junk reached the server over the wire: one repeat accepted, four frames rejected
    // (a ContentId claimed under another key, a truncated frame, an unknown tag, an empty frame).
    assert!(field(&srv, "ACCEPTED").ends_with("REJECTED 4"), "serve:\n{srv}");
    assert!(field(&cli, "ACCEPTED").ends_with("REJECTED 0"), "connect:\n{cli}");
    assert_eq!(field(&srv, "DANGLING"), "0", "bob's overlap names alice's work, which arrived");

    // Each trace, replayed once per public seed, lands on the same hash every time.
    for trace in [&ts, &tc] {
        let r = replay(trace);
        let all = field(&r, "ALL");
        assert!(all.starts_with(&format!("64 SEEDS MATCH {want} ")), "{all}");
        assert!(!r.contains("MISMATCH"), "{r}");
    }
    let _ = std::fs::remove_file(ts);
    let _ = std::fs::remove_file(tc);
}

#[test]
fn rival_authorship_crosses_the_wire_flagged_not_resolved() {
    let (srv, cli, ts, tc) = run_pair("c", "a", false);
    assert_eq!(field(&srv, "RIVALS"), "1", "carol's copy of alice's second text is flagged:\n{srv}");
    assert_eq!(field(&cli, "RIVALS"), "1", "on both ends:\n{cli}");
    assert_eq!(field(&srv, "MERGED"), field(&cli, "MERGED"));
    for trace in [&ts, &tc] {
        replay(trace);
    }
    let _ = std::fs::remove_file(ts);
    let _ = std::fs::remove_file(tc);
}
