//! T1 slice-5 — two-node join, proven across two real OS PROCESSES.
//!
//! The slice-4 test (`sync_join.rs`) proved a joiner converges to a seed over localhost TCP using two
//! THREADS in one process. This raises the bar to the actual demo: two independent `noesisd` processes
//! — `--listen` (seed) and `--connect` (joiner) — where the joiner boots a fresh genesis, dials the
//! seed over the network, replays its block log, and prints the SAME canonical state digest the seed
//! printed. Real process isolation: no shared memory, no shared allocator; the only channel is the
//! framed TCP socket. That is "a network you can join," end to end.
//!
//! The seed binds an OS-ephemeral port (`127.0.0.1:0`) and announces it on stdout as `LISTENING
//! <addr>`; the test reads that, then launches the joiner against it. Both print `DIGEST <...>`; the
//! test asserts the two strings are equal (byte-identical convergence) and non-trivial (the chain
//! actually had blocks), then reaps the seed.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

/// Read the seed's stdout until we have BOTH the `LISTENING <addr>` line (the bound ephemeral port)
/// and its `DIGEST <...>` line (its converged state), returning `(addr, digest)`. Panics with the
/// lines seen so far if the process ends before emitting both — a loud failure, never a silent hang.
fn read_seed_ready(child: &mut Child) -> (String, String) {
    let stdout = child.stdout.take().expect("seed stdout is piped");
    let reader = BufReader::new(stdout);
    let mut addr: Option<String> = None;
    let mut digest: Option<String> = None;
    let mut seen: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line.expect("read seed stdout line");
        if let Some(rest) = line.strip_prefix("LISTENING ") {
            addr = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("DIGEST ") {
            digest = Some(rest.trim().to_string());
        }
        seen.push(line);
        if addr.is_some() && digest.is_some() {
            return (addr.unwrap(), digest.unwrap());
        }
    }
    panic!("seed exited before announcing LISTENING + DIGEST; stdout was:\n{}", seen.join("\n"));
}

/// Pull the single `DIGEST <...>` line out of a finished process's captured stdout.
fn extract_digest(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("DIGEST ").map(|r| r.trim().to_string()))
        .unwrap_or_else(|| panic!("no DIGEST line in output:\n{stdout}"))
}

#[test]
fn two_processes_join_and_converge_to_identical_state() {
    let bin = env!("CARGO_BIN_EXE_noesisd");

    // 1) Start the seed on an ephemeral port; capture stdout so we can read its address + digest.
    let mut seed = Command::new(bin)
        .arg("--listen")
        .arg("127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn seed noesisd --listen");

    let (addr, seed_digest) = read_seed_ready(&mut seed);

    // 2) Run the joiner against the seed's announced address; it syncs then exits.
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

    assert!(
        joiner.status.success(),
        "joiner process exited non-zero: {:?}",
        joiner.status
    );

    let joiner_out = String::from_utf8_lossy(&joiner.stdout);
    let joiner_digest = extract_digest(&joiner_out);

    // The two independent processes must agree on the FULL canonical state digest.
    assert_eq!(
        seed_digest, joiner_digest,
        "seed and joiner converged to DIFFERENT state:\n seed:   {seed_digest}\n joiner: {joiner_digest}"
    );

    // And it must be a real chain, not two empty genesis ledgers trivially matching.
    assert!(
        joiner_digest.contains("cells=[1, 2, 3, 4, 5, 6]"),
        "expected the full 6-cell scripted chain to have synced; digest was: {joiner_digest}"
    );
    assert!(
        !joiner_digest.contains("height=0"),
        "joiner is still at genesis height — nothing synced: {joiner_digest}"
    );
}
