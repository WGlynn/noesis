//! Testnet step 7, end to end across two real OS processes: a producer (`noesisd --serve-api`) and a
//! replica (`noesisd --follow <producer-url> <http> <store>`), both on the testnet spec, on temp ports and
//! temp stores. The replica catches up, then tracks a new block live, sends submits and faucet requests
//! to the producer with a 409, needs no faucet seed, and resumes from its own store after a restart.
//! Nothing here touches `~/.noesis` or a fixed port.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

fn tmp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("noesis-follow-live-{tag}-{}-{nanos}", std::process::id()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A child process whose stdout lines arrive on a channel (drained, so the pipe never fills).
struct Proc {
    child: Child,
    lines: Receiver<String>,
}

impl Proc {
    fn spawn(cmd: &mut Command) -> Proc {
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().expect("spawn noesisd");
        let out = child.stdout.take().expect("piped stdout");
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
        Proc { child, lines }
    }

    /// The rest of the first stdout line starting with `prefix`, within 60 s.
    fn wait_line(&self, prefix: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut seen = Vec::new();
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.lines.recv_timeout(left) {
                Ok(l) => {
                    if let Some(rest) = l.strip_prefix(prefix) {
                        return rest.trim().to_string();
                    }
                    seen.push(l);
                }
                Err(_) => break,
            }
        }
        panic!("no line starting {prefix:?}; saw:\n{}", seen.join("\n"));
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One HTTP request to `addr` (host:port), returning (status, JSON body).
fn http(addr: &str, method: &str, path: &str, body: &[u8]) -> (u16, Value) {
    let mut s = TcpStream::connect(addr).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(120))).unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes()).unwrap();
    s.write_all(body).unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("an HTTP header");
    let status = String::from_utf8_lossy(&raw[..split]).split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, serde_json::from_slice(&raw[split + 4..]).expect("a JSON body"))
}

/// A contribution signed the way the browser wallet signs it (wallet format v2), with hot seed `master`.
fn signed(master: &[u8; 32], index: u32, data: &str) -> Vec<u8> {
    use noesis_core::wallet;
    let hot_root = wallet::key_root(master);
    let cold_commit = wallet::cold_commit(&[0xC0; 32]);
    let identity = wallet::identity(&hot_root, &cold_commit);
    let sig = wallet::sign(master, index, &wallet::contribution_digest(&identity, index, None, data.as_bytes()));
    let part = |t: &noesis_core::xmss::Signature| {
        json!({ "ots_root": hex(&t.ots_root), "auth": t.auth.iter().map(|a| hex(a)).collect::<Vec<_>>(), "ots_sig": hex(&t.ots_sig) })
    };
    json!({
        "identity": hex(&identity), "hot_root": hex(&hot_root), "cold_commit": hex(&cold_commit),
        "index": index, "bottom": part(&sig.bottom), "top": part(&sig.top), "data": data,
    })
    .to_string()
    .into_bytes()
}

fn submit(addr: &str, n: u8, data: &str) {
    let (s, v) = http(addr, "POST", "/submit", &signed(&[0x60 + n; 32], 0, data));
    assert_eq!(s, 200, "{v}");
}

/// Poll `addr`'s `/state` until its height reaches `h` (60 s).
fn wait_height(addr: &str, h: u64) -> Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let (_, v) = http(addr, "GET", "/state", b"");
        if v["height"].as_u64() >= Some(h) {
            return v;
        }
        assert!(Instant::now() < deadline, "{addr} stayed at height {} (wanted {h}): {v}", v["height"]);
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[test]
fn a_replica_follows_the_producer_live_and_resumes_from_its_own_store() {
    let bin = env!("CARGO_BIN_EXE_noesisd");
    let (pstore, fstore, seed, no_seed) = (tmp("producer.log"), tmp("follower.log"), tmp("faucet.seed"), tmp("absent.seed"));
    std::fs::write(&seed, hex(&noesis::faucet::DEV_FAUCET_SEED)).unwrap();

    let producer = Proc::spawn(
        Command::new(bin)
            .env("NOESIS_NET", "testnet")
            .env("NOESIS_FAUCET_SEED", &seed)
            .args(["--serve-api", "127.0.0.1:0"])
            .arg(&pstore),
    );
    let paddr = producer.wait_line("LISTENING http://");
    submit(&paddr, 0, "a first note on harbor fog");
    submit(&paddr, 1, "an entirely separate subject, cold rivers");

    // The follower boots the testnet with NO faucet seed (its path points at a file that does not exist).
    let follow = |p: &PathBuf| {
        Proc::spawn(
            Command::new(bin)
                .env("NOESIS_NET", "testnet")
                .env("NOESIS_FAUCET_SEED", &no_seed)
                .args(["--follow", &format!("http://{paddr}/"), "127.0.0.1:0"])
                .arg(p),
        )
    };
    let follower = follow(&fstore);
    let faddr = follower.wait_line("LISTENING http://");
    assert_eq!(follower.wait_line("FOLLOWING "), format!("http://{paddr} from height 0"));
    wait_height(&faddr, 2);

    // live: a block made after the follower joined reaches it
    submit(&paddr, 2, "a sketch of a tide-driven water clock");
    let f = wait_height(&faddr, 3);
    let (_, p) = http(&paddr, "GET", "/state", b"");
    for k in ["height", "work", "jul_issued", "cells", "contributors", "chain_id"] {
        assert_eq!(f[k], p[k], "{k}");
    }
    assert_eq!((f["role"].clone(), f["replica"]["following"].clone()), (json!("replica"), json!(true)));
    assert_eq!(p["role"], "producer");

    // the replica sends what would need a block to the producer
    let (s, v) = http(&faddr, "POST", "/submit", &signed(&[0x6F; 32], 0, "notes on kiln glazes and wood ash"));
    assert_eq!((s, v["producer"].clone()), (409, json!(format!("http://{paddr}"))));
    let (s, v) = http(&faddr, "POST", "/faucet", json!({ "address": "ab".repeat(32) }).to_string().as_bytes());
    assert_eq!((s, v["producer"].clone()), (409, json!(format!("http://{paddr}"))));
    assert_eq!(http(&faddr, "GET", "/state", b"").1["height"], 3, "the replica took nothing in");

    // restart: the follower resumes from its own store and asks only for what it lacks
    drop(follower);
    submit(&paddr, 3, "the migration of eels through the sargasso sea");
    let follower = follow(&fstore);
    assert!(follower.wait_line("resumed durable chain").contains("replayed 3 blocks"));
    let faddr = follower.wait_line("LISTENING http://");
    assert_eq!(follower.wait_line("FOLLOWING "), format!("http://{paddr} from height 3"));
    let f = wait_height(&faddr, 4);
    let (_, p) = http(&paddr, "GET", "/state", b"");
    assert_eq!((f["contributors"].clone(), f["jul_issued"].clone()), (p["contributors"].clone(), p["jul_issued"].clone()));

    drop(follower);
    drop(producer);
    for path in [pstore, fstore, seed] {
        let _ = std::fs::remove_file(path);
    }
}
