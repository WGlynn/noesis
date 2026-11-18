//! follow — testnet step 7, live sync with one producer (`docs/DESIGN-live-gossip.md`, topology A).
//!
//! One node produces (`noesisd --serve-api`). Every other node is a replica (`noesisd --follow
//! <producer-url> <http> <store>`): it pulls finalized blocks over HTTP from the producer's
//! `GET /blocks?since=N` (the one public port, so it works through a tunnel), runs each through the
//! producer's own acceptance path (`ChainSpec::accept_block`: the rulebook with PoW and control binding,
//! then the finality gate), appends it to its own durable store, and serves its own page and API. It
//! produces nothing: submit, faucet and token-tx answer 409 with the producer's URL.
//!
//! What a follower checks on every page, before it applies anything:
//! * the chain id is its own (another network is refused);
//! * the producer's height is not below its own (a producer that lost or dropped blocks is refused);
//! * the producer's chain link at the follower's height equals the follower's own ([`chain_link`]):
//!   a cumulative hash over every header digest from genesis, so a rewrite of ANY block the follower
//!   holds shows up, even one that leaves the later blocks byte-identical (blocks do not name their
//!   parent's hash today, `Block::parent_hash` is `None`, so a single header digest would miss that);
//! * every block passes `accept_block`. A refused block stops following; the follower keeps its chain.
//!
//! On a rewrite, a refused block or another network the follower STOPS, says why on stderr and in
//! `GET /state` (`replica.stopped`), and keeps serving the chain it validated. It never adopts the
//! other history by itself; restarting it re-checks and stops again. Transport trouble (the producer
//! offline, a tunnel error, a garbled response) is retried with backoff.
//!
//! Standing on a replica is whatever its own `apply` derives from the blocks it accepted: nothing about
//! standing is taken from the producer's word.
//!
//! HONEST SCOPE. One producer, for now (Will, 2026-11-17): no failover, no leader election, no fork
//! choice. The chain link is node-local, not consensus. A block carries no producer signature and a
//! contribution cell does not carry its wallet signature (the producer checked it at `POST /submit`), so
//! a follower verifies every state transition the rulebook covers but trusts the producer URL (and
//! TLS, when it is https) for who produced the chain and who signed each contribution.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use crate::rpc::ServerState;
use crate::runtime::{header_digest, Block};
use crate::wire::decode_block;

/// Most blocks one `GET /blocks?since=N` page carries.
pub const PAGE_MAX_BLOCKS: usize = 32;
/// A page's byte budget over the encoded blocks. A block that carries token txs holds 16 KiB Lamport
/// signatures (about 64 KiB each as JSON numbers), so pages are cut by size as well as count. A page
/// always carries at least one block when one is due, however large.
pub const PAGE_MAX_BYTES: usize = 1 << 20;
/// How often an up-to-date follower asks the producer for new blocks. A page that says `more` is
/// followed at once.
pub const POLL: Duration = Duration::from_secs(2);
/// The longest wait between retries while the producer cannot be reached.
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// The largest response a follower reads (a page plus one oversized block).
const MAX_RESPONSE: usize = 32 << 20;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IO_TIMEOUT: Duration = Duration::from_secs(60);

/// The cumulative chain link through `block`: blake2b over the link before it and the block's header
/// digest. `links[0]` is all zeros (genesis); `links[h]` commits to every block up to height h, so two
/// nodes with equal links at h hold the same history through h. Node-local, never consensus.
pub fn chain_link(prev: &[u8; 32], block: &Block) -> [u8; 32] {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-link-v1\0\0").build();
    h.update(prev);
    h.update(&header_digest(block));
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

/// Why a page was not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowError {
    /// Worth asking again later: the producer was unreachable, or answered something unreadable.
    Retry(String),
    /// Following must stop: the producer's chain is not a continuation of this node's, or it served a
    /// block this node's rulebook refuses. This node keeps the chain it has.
    Stop(String),
}

/// The request a follower at `height` sends next: everything after the blocks it already holds.
pub fn next_request(height: u64) -> String {
    format!("/blocks?since={height}")
}

/// A producer URL as `--follow` takes it: `http://` or `https://`, no trailing slash.
pub fn normalize_producer_url(url: &str) -> Result<String, String> {
    let u = url.trim().trim_end_matches('/');
    match u.strip_prefix("https://").or_else(|| u.strip_prefix("http://")) {
        Some(host) if !host.is_empty() => Ok(u.to_string()),
        _ => Err(format!("the producer URL must start with http:// or https:// and name a host (got {url:?})")),
    }
}

/// Apply one `GET /blocks?since=N` page (`body`) to the replica `st`. Returns how many blocks were
/// applied and whether the producer has more. Checks the network, the producer's height and the chain
/// link before applying anything, then accepts each block through `ServerState::follow_block`.
pub fn apply_page(st: &mut ServerState, body: &[u8]) -> Result<(usize, bool), FollowError> {
    use FollowError::{Retry, Stop};
    let v: Value = serde_json::from_slice(body).map_err(|e| Retry(format!("unreadable page from the producer: {e}")))?;
    let field = |k: &str| v.get(k).and_then(Value::as_u64).ok_or_else(|| Retry(format!("the producer's page has no '{k}'")));
    let (chain_id, since, tip) = (field("chain_id")?, field("since")?, field("tip")?);
    let ours = st.height();
    if chain_id != st.chain_id() {
        return Err(Stop(format!(
            "the producer runs chain 0x{chain_id:x} and this node chain 0x{:x}: another network (check NOESIS_NET)",
            st.chain_id()
        )));
    }
    if since != ours {
        return Err(Retry(format!("the producer answered for height {since}, not {ours} (a stale or cached response)")));
    }
    if tip < ours {
        return Err(Stop(format!(
            "the producer's chain is at height {tip}, below this node's {ours}: it lost or rewrote blocks this node \
             holds. This node keeps its chain. If the chain was reset on purpose, run the follower on a new store."
        )));
    }
    let anchor = v
        .get("anchor")
        .and_then(Value::as_str)
        .and_then(crate::rpc::hex_decode)
        .ok_or_else(|| Retry("the producer's page has no chain link".into()))?;
    if anchor.as_slice() != st.link(ours).as_slice() {
        return Err(Stop(format!(
            "the producer's history differs from this node's at or before height {ours} (the chain links do not \
             match): it was rewritten, or this is another chain. This node keeps its chain. If the chain was reset \
             on purpose, run the follower on a new store."
        )));
    }
    let blocks = v.get("blocks").and_then(Value::as_array).ok_or_else(|| Retry("the producer's page has no blocks".into()))?;
    let mut applied = 0usize;
    for bv in blocks {
        let bytes = serde_json::to_vec(bv).map_err(|e| Retry(e.to_string()))?;
        let block = decode_block(&bytes)
            .map_err(|e| Stop(format!("block {} from the producer does not decode: {e}", ours + applied as u64 + 1)))?;
        st.follow_block(&block).map_err(|e| match st.store_failed() {
            Some(_) => Stop(e), // this node's own block log failed the write; it says so itself
            None => Stop(format!("refused a block from the producer: {e}. This node keeps its chain at height {}.", st.height())),
        })?;
        applied += 1;
    }
    let more = v.get("more").and_then(Value::as_bool).unwrap_or(false);
    Ok((applied, more))
}

/// `GET url`, returning (status, body). `http://` goes over a plain std socket; `https://` (a tunnel)
/// runs the system's `curl`, which checks the certificate, because the node links no TLS library.
pub fn http_get(url: &str) -> Result<(u16, Vec<u8>), String> {
    if let Some(rest) = url.strip_prefix("http://") {
        plain_get(rest)
    } else if url.starts_with("https://") {
        curl_get(url)
    } else {
        Err(format!("unsupported URL {url:?}"))
    }
}

fn plain_get(rest: &str) -> Result<(u16, Vec<u8>), String> {
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let with_port = if host.rsplit(']').next().is_some_and(|tail| tail.contains(':')) { host.to_string() } else { format!("{host}:80") };
    let addr = with_port
        .to_socket_addrs()
        .map_err(|e| format!("cannot resolve {host}: {e}"))?
        .next()
        .ok_or_else(|| format!("cannot resolve {host}"))?;
    let mut s = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|e| format!("cannot reach {host}: {e}"))?;
    let _ = s.set_read_timeout(Some(IO_TIMEOUT));
    let _ = s.set_write_timeout(Some(IO_TIMEOUT));
    // HTTP/1.0 so no proxy answers chunked; the node closes after each response.
    let req = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nAccept: application/json\r\nConnection: close\r\n\r\n");
    s.write_all(req.as_bytes()).map_err(|e| format!("send to {host}: {e}"))?;
    let mut raw = Vec::new();
    s.take(MAX_RESPONSE as u64 + 1).read_to_end(&mut raw).map_err(|e| format!("read from {host}: {e}"))?;
    if raw.len() > MAX_RESPONSE {
        return Err(format!("the response from {host} is over {MAX_RESPONSE} bytes"));
    }
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| format!("no HTTP header from {host}"))?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let body = raw[split + 4..].to_vec();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or_else(|| format!("no HTTP status from {host}"))?;
    let declared = head.lines().find_map(|l| {
        let l = l.to_ascii_lowercase();
        l.strip_prefix("content-length:").and_then(|n| n.trim().parse::<usize>().ok())
    });
    if declared.is_some_and(|n| body.len() < n) {
        return Err(format!("the response from {host} was cut short"));
    }
    Ok((status, body))
}

fn curl_get(url: &str) -> Result<(u16, Vec<u8>), String> {
    let out = std::process::Command::new("curl")
        .args(["-sS", "--max-time", "60", "--max-filesize", &MAX_RESPONSE.to_string()])
        .args(["-H", "Accept: application/json", "-w", "\n%{http_code}", url])
        .output()
        .map_err(|e| format!("an https producer needs the curl program on PATH: {e}"))?;
    if !out.status.success() {
        return Err(format!("curl: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let raw = out.stdout;
    let nl = raw.iter().rposition(|&b| b == b'\n').ok_or("curl printed no status")?;
    let status = std::str::from_utf8(&raw[nl + 1..]).ok().and_then(|c| c.trim().parse::<u16>().ok()).ok_or("curl printed no status")?;
    Ok((status, raw[..nl].to_vec()))
}

/// The follower loop: ask the producer for the blocks after this node's height, apply them, repeat.
/// Runs until following has to stop (see [`FollowError::Stop`]); transport errors are retried with
/// backoff. The page is fetched without holding the node's lock; only applying it takes the lock.
pub fn run(state: Arc<Mutex<ServerState>>, producer: String) {
    let lock = || state.lock().unwrap_or_else(|p| p.into_inner());
    let mut backoff = POLL;
    loop {
        let since = lock().height();
        let fetched = http_get(&format!("{producer}{}", next_request(since))).and_then(|(status, body)| match status {
            200 => Ok(body),
            s => Err(format!("the producer answered HTTP {s}: {}", String::from_utf8_lossy(&body).chars().take(200).collect::<String>())),
        });
        let outcome = fetched.map_err(FollowError::Retry).and_then(|body| apply_page(&mut lock(), &body));
        match outcome {
            Ok((applied, more)) => {
                let mut st = lock();
                if applied > 0 {
                    println!("followed {applied} block(s) from {producer}: height {}", st.height());
                }
                if st.follow_error().is_some() {
                    println!("reached the producer again at {producer}");
                }
                st.set_follow_error(None);
                drop(st);
                backoff = POLL;
                if !more {
                    std::thread::sleep(POLL);
                }
            }
            Err(FollowError::Retry(e)) => {
                let mut st = lock();
                if st.follow_error() != Some(e.as_str()) {
                    eprintln!("noesisd: follow: {e} (retrying)");
                }
                st.set_follow_error(Some(e));
                drop(st);
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(FollowError::Stop(e)) => {
                eprintln!("noesisd: FOLLOWING STOPPED: {e}");
                lock().stop_following(e);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server on a free local port that answers each of `n` connections with `response`.
    fn canned_server(response: &'static [u8], n: usize) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for _ in 0..n {
                if let Ok((mut s, _)) = l.accept() {
                    let mut buf = [0u8; 4096];
                    let _ = s.read(&mut buf);
                    let _ = s.write_all(response);
                }
            }
        });
        addr
    }

    #[test]
    fn both_transports_return_the_status_and_the_body() {
        let resp: &[u8] = b"HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nContent-Length: 16\r\nConnection: close\r\n\r\n{\"producer\":\"x\"}";
        let addr = canned_server(resp, 2);
        let want = Ok((409, br#"{"producer":"x"}"#.to_vec()));
        assert_eq!(http_get(&format!("http://{addr}/blocks?since=0")), want);
        // the https path runs curl; curl speaks plain http too, so its output parsing is checked here
        if std::process::Command::new("curl").arg("--version").output().is_ok() {
            assert_eq!(curl_get(&format!("http://{addr}/blocks?since=0")), want);
        } else {
            eprintln!("curl not on PATH: the curl transport was not exercised");
        }
        let short = canned_server(b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n{}", 1);
        assert!(http_get(&format!("http://{short}/")).unwrap_err().contains("cut short"));
        assert!(http_get("ftp://example").is_err());
    }

    #[test]
    fn producer_urls_must_be_http_or_https_and_lose_a_trailing_slash() {
        assert_eq!(normalize_producer_url("https://abc.trycloudflare.com/"), Ok("https://abc.trycloudflare.com".into()));
        assert_eq!(normalize_producer_url(" http://127.0.0.1:9955 "), Ok("http://127.0.0.1:9955".into()));
        assert_eq!(normalize_producer_url("http://a"), Ok("http://a".into()));
        for bad in ["127.0.0.1:9955", "ws://x", "https://", ""] {
            assert!(normalize_producer_url(bad).is_err(), "{bad}");
        }
        assert_eq!(next_request(7), "/blocks?since=7");
    }
}
