//! identity-mint — the issuer's mint tool for the soulbound identity collection
//! (`noesis::identity_mint`, standard `tokens::soulbound`). Used by the issuer's session only; the
//! issuer key never goes to the node or the bot.
//!
//! USAGE
//!   identity-mint mint --node http://127.0.0.1:9955 --seed <seed-file> <batch.json>
//!                      [--wait <seconds>] [--state <file>] [--abandon-pending]
//!   identity-mint root --seed <seed-file>     # print the authority's generation-0 lock root (hex)
//!
//! The seed file holds 64 hex characters and must never enter a repository. `mint` reads the node's
//! `GET /tokens`, builds ONE soulbound tx for the batch (mints, re-tiers; records already on chain are
//! `unchanged`; records over the per-tx caps are `deferred`), signs it with the key chain and posts it
//! to `POST /token-tx`. The node holds it until the next contribution block carries it; `--wait`
//! polls until then. Output: one JSON line per record on stdout with `status` minted / retiered /
//! unchanged / queued / deferred, `cell_id` and `block_height`.
//!
//! One-time keys: the signed tx for each authority generation is written to the state file (default
//! `<seed-file>.pending.json`) before it is posted. While it has not landed, a different batch is
//! refused (the pending tx is re-sent instead), because signing a second digest with the same key
//! weakens it. `--abandon-pending` overrides that, knowingly.
//!
//! Exit codes: 0 done (or queued) · 2 usage / input / node error · 3 refused by the node ·
//! 4 a different signed tx for this generation is still pending (re-sent) · 5 state file ahead of chain.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use noesis::identity_mint::{self as im, Action};
use serde_json::{json, Value};

fn die(code: i32, msg: impl std::fmt::Display) -> ! {
    eprintln!("identity-mint: {msg}");
    std::process::exit(code);
}

/// Minimal HTTP/1.1 client over std (the node's server closes each connection after one response).
fn http(method: &str, node: &str, path: &str, body: Option<&[u8]>) -> Result<(u16, Value), String> {
    let hostport = node.strip_prefix("http://").ok_or("--node must look like http://host:port")?.trim_end_matches('/');
    let mut s = TcpStream::connect(hostport).map_err(|e| format!("cannot reach {hostport}: {e}"))?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(60)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(60)));
    let body = body.unwrap_or(b"");
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {hostport}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(body)).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("malformed HTTP response")?;
    let status: u16 = std::str::from_utf8(&raw[..split])
        .ok()
        .and_then(|h| h.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or("malformed HTTP status line")?;
    let v = serde_json::from_slice(&raw[split + 4..]).map_err(|e| format!("response is not JSON: {e}"))?;
    Ok((status, v))
}

fn get_tokens(node: &str) -> Value {
    match http("GET", node, "/tokens", None) {
        Ok((200, v)) => v,
        Ok((s, v)) => die(2, format!("GET /tokens returned {s}: {v}")),
        Err(e) => die(2, e),
    }
}

fn post_envelope(node: &str, envelope: &Value) -> Value {
    match http("POST", node, "/token-tx", Some(envelope.to_string().as_bytes())) {
        Ok((200, v)) => v,
        Ok((_, v)) => die(3, format!("the node refused the tx: {}", v["error"])),
        Err(e) => die(2, e),
    }
}

struct Args {
    cmd: String,
    node: String,
    seed: String,
    batch: Option<String>,
    wait: u64,
    state: Option<String>,
    abandon: bool,
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_default();
    let mut a = Args { cmd, node: "http://127.0.0.1:9955".into(), seed: String::new(), batch: None, wait: 0, state: None, abandon: false };
    while let Some(x) = it.next() {
        match x.as_str() {
            "--node" => a.node = it.next().unwrap_or_else(|| die(2, "--node needs a value")),
            "--seed" => a.seed = it.next().unwrap_or_else(|| die(2, "--seed needs a file")),
            "--wait" => a.wait = it.next().and_then(|w| w.parse().ok()).unwrap_or_else(|| die(2, "--wait needs seconds")),
            "--state" => a.state = Some(it.next().unwrap_or_else(|| die(2, "--state needs a file"))),
            "--abandon-pending" => a.abandon = true,
            other if !other.starts_with("--") && a.batch.is_none() => a.batch = Some(other.to_string()),
            other => die(2, format!("unknown argument {other:?}")),
        }
    }
    if a.seed.is_empty() {
        die(2, "usage: identity-mint mint --node http://host:port --seed <seed-file> <batch.json> [--wait <s>] | identity-mint root --seed <seed-file>");
    }
    a
}

fn main() {
    let a = parse_args();
    let seed_text = std::fs::read_to_string(&a.seed).unwrap_or_else(|e| die(2, format!("cannot read seed {}: {e}", a.seed)));
    let master = im::parse_seed(&seed_text).unwrap_or_else(|e| die(2, e));
    match a.cmd.as_str() {
        "root" => {
            let r = im::authority_root(&master, 0);
            println!("{}", r.iter().map(|b| format!("{b:02x}")).collect::<String>());
        }
        "mint" => mint(&a, &master),
        other => die(2, format!("unknown command {other:?} (mint | root)")),
    }
}

fn mint(a: &Args, master: &[u8; 32]) {
    let col = im::COLLECTION;
    let batch_path = a.batch.as_deref().unwrap_or_else(|| die(2, "mint needs a batch file"));
    let batch_text = std::fs::read_to_string(batch_path).unwrap_or_else(|e| die(2, format!("cannot read {batch_path}: {e}")));
    let records = im::parse_batch(&batch_text, col).unwrap_or_else(|e| die(2, e));
    let live = im::parse_live(&get_tokens(&a.node), col).unwrap_or_else(|e| die(2, e));
    let mut built = im::build(&records, &live, master, col).unwrap_or_else(|e| die(2, e));
    let g = built.generation;

    let Some(digest) = built.digest() else {
        for p in &built.plan {
            println!("{}", im::report(p, p.action.done(), None, col));
        }
        eprintln!("identity-mint: nothing to send (authority generation {g})");
        return;
    };
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();

    // One signed tx per generation: never sign a second, different digest with the same key.
    let state_path = a.state.clone().unwrap_or_else(|| format!("{}.pending.json", a.seed));
    if let Some(p) = std::fs::read_to_string(&state_path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        let pg = p["generation"].as_u64().unwrap_or(0);
        if pg > g {
            die(5, format!("state file {state_path} is at generation {pg}, ahead of the chain's {g}: wrong node or wrong state file"));
        }
        if pg == g && p["digest"].as_str() != Some(hex(&digest).as_str()) {
            if a.abandon {
                eprintln!("identity-mint: WARNING --abandon-pending: signing a second digest with generation {g}'s one-time key");
            } else {
                post_envelope(&a.node, &p["envelope"]);
                die(4, format!(
                    "a different signed tx for generation {g} is pending (digest {}); re-sent it. Re-run this batch after it lands",
                    p["digest"].as_str().unwrap_or("?")
                ));
            }
        }
    }

    // Size check before any signature exists (signatures have a fixed length).
    let tx_len = built.tx.as_ref().map(|t| t.inputs.len()).unwrap_or(0);
    let mut probe = built.tx.clone().expect("a digest implies a tx");
    probe.auths = vec![vec![0u8; 16 * 1024]; tx_len];
    if im::envelope(&probe).to_string().len() >= noesis::rpc::MAX_BODY {
        die(2, "the signed tx would exceed the node's body cap; split the batch");
    }

    built.sign();
    let tx = built.tx.as_ref().expect("signed tx");
    let envelope = im::envelope(tx);
    let record = json!({ "generation": g, "digest": hex(&digest), "envelope": envelope });
    std::fs::write(&state_path, record.to_string()).unwrap_or_else(|e| die(2, format!("cannot write state {state_path}: {e}")));
    let resp = post_envelope(&a.node, &envelope);
    eprintln!("identity-mint: queued tx {} (authority generation {g} -> {}), {}", hex(&digest), g + 1, resp["note"]);

    // Wait for the carrying block, if asked.
    let deadline = Instant::now() + Duration::from_secs(a.wait);
    let mut landed: Option<Option<u64>> = None;
    loop {
        let t = get_tokens(&a.node);
        let height = t["included"]
            .as_array()
            .and_then(|v| v.iter().find(|i| i["digest"].as_str() == Some(hex(&digest).as_str())))
            .and_then(|i| i["height"].as_u64());
        let advanced = im::parse_live(&t, col).map(|l| l.generation > g).unwrap_or(false);
        if height.is_some() || advanced {
            landed = Some(height);
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    for p in &built.plan {
        let (status, h) = match (p.action, landed) {
            (Action::Mint | Action::Retier, Some(h)) => (p.action.done(), h),
            (Action::Mint | Action::Retier, None) => ("queued", None),
            (other, _) => (other.done(), None),
        };
        println!("{}", im::report(p, status, h, col));
    }
    if landed.is_some() {
        let _ = std::fs::remove_file(&state_path);
        eprintln!("identity-mint: landed");
    } else {
        eprintln!("identity-mint: queued; the next contribution block carries it (re-run the same batch to check)");
    }
}
