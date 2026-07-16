//! rpc — the node's live HTTP/JSON interface: the API a friend (or a browser frontend) calls to
//! SUBMIT a contribution and READ the chain. This is what turns `noesisd` from "runs a scripted
//! workload" into "a node your friends can poke."
//!
//! Three routes, plus CORS so a browser frontend on another origin can also call them:
//!   * `GET  /`       — the embedded frontend itself (`text/html`), so ONE public URL is the whole app.
//!   * `GET  /state`  — the chain view: height, JUL issued, per-contributor PoM standing, cell counts.
//!   * `POST /submit` — a JSON `{ "contributor": "...", "data": "..." }`; the node builds a cell, runs
//!                      the ONE proven per-block engine ([`crate::chainspec::ChainSpec::produce_block`]),
//!                      and returns the new state. A submitted contribution travels the EXACT path the
//!                      tested chain does — no second, drifting pipeline.
//!
//! DELIBERATELY LEAN (the `net` precedent): std `TcpListener` + threads, NO async runtime and NO web
//! framework. The request parser is bounded (a body cap + socket read timeout) — enough for a LOCAL
//! devnet a few friends share. HONEST SCOPE: strict request-size limits, rate limiting, and auth are
//! PRE-PUBLIC-SEED hardening, not built here (this is the invite-a-few-friends tier, not the open net).
//!
//! The dispatch core [`handle_request`] is a PURE function (state + method + path + body → status +
//! json) so the whole API is unit-tested without opening a socket; `serve_api` is the thin shell.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use crate::chainspec::ChainSpec;
use crate::commit_order::Committed;
use crate::runtime::Node;
use crate::{Cell, Script};

/// Max request body we will read (a submitted contribution is small). Bounds memory per connection.
const MAX_BODY: usize = 64 * 1024;
/// Per-socket read/write deadline — a stalled client's thread dies instead of hanging forever.
const IO_TIMEOUT: Duration = Duration::from_secs(30);
/// The frontend, EMBEDDED at compile time so the node is ONE self-contained binary that serves its own
/// UI at `/`. Same-origin ⇒ the page fetches `/state` and `/submit` relative to wherever it is served,
/// so a single tunnel (one public URL) exposes the whole app with no CORS and no hardcoded node address.
const INDEX_HTML: &str = include_str!("../../frontend/index.html");

/// The live node's mutable state behind the API: the chain spec (the genesis + per-block engine), the
/// node replica, and a monotone cell-id counter for interactively-submitted contributions.
pub struct ServerState {
    spec: ChainSpec,
    node: Node,
    next_id: u64,
    /// Where finalized blocks are persisted. `None` ⇒ in-memory only (tests); `Some(path)` ⇒ durable:
    /// every finalized block is appended, and a restart replays the log instead of resetting to genesis.
    store: Option<std::path::PathBuf>,
    /// The node-local ingress screen: rejects trivial / near-duplicate submissions before the mempool.
    /// Advisory (never consensus); rebuilt from the persisted chain on boot so originality survives a restart.
    screen: crate::screen::Screen,
}

impl ServerState {
    /// In-memory genesis (no persistence) — friends' contributions ARE the chain from block zero. Used
    /// by tests; the live server uses [`with_store`](Self::with_store) so the chain survives a restart.
    pub fn new() -> Self {
        let spec = ChainSpec::dev();
        let (node, _keys) = spec.genesis_node();
        ServerState { spec, node, next_id: 1, store: None, screen: crate::screen::Screen::new() }
    }

    /// Boot a DURABLE node: replay the persisted block log at `path` (or start at genesis if absent),
    /// and append every future finalized block there. This is what makes a hosted node "stay on" — a
    /// process restart resumes the chain rather than wiping it. Returns the state + how many blocks were
    /// replayed so the caller can announce it.
    pub fn with_store(path: impl Into<std::path::PathBuf>, spec: ChainSpec) -> std::io::Result<(Self, usize)> {
        let path = path.into();
        let (node, replayed) = crate::store::load_chain(&path, &spec)?;
        // Resume id assignment past the highest persisted cell id (never collide with a replayed cell).
        let next_id = node.ledger.cells.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        // Rebuild the ingress screen's seen-set from the replayed chain so originality checks survive a restart.
        let mut screen = crate::screen::Screen::new();
        for c in &node.ledger.cells {
            screen.record(&c.data);
        }
        Ok((ServerState { spec, node, next_id, store: Some(path), screen }, replayed))
    }

    /// The chain view as JSON.
    fn state_json(&self) -> Value {
        let (ids, _root, pom, tokens, work) = self.node.ledger.state_digest();
        let contributors: serde_json::Map<String, Value> = pom
            .iter()
            .map(|(k, v)| (String::from_utf8_lossy(k).into_owned(), json!(v)))
            .collect();
        json!({
            "chain_id": self.spec.chain_id,
            "height": self.node.ledger.height,
            "work": work,
            "jul_issued": self.node.ledger.jul_supply.issued() / crate::jul::JUL_BASE_UNITS,
            "cells": ids.len(),
            "coinbase_cells": tokens.len(),
            "pow_enforced": self.spec.constitution.pow_enforced,
            "contributors": contributors,
        })
    }

    /// Ingest a contribution and try to finalize a block from it. Returns the finalized height on
    /// success, or an error string (rejected/not-finalized). One block per submit in this slice.
    fn submit(&mut self, contributor: &str, data: &str) -> Result<u64, String> {
        if contributor.is_empty() {
            return Err("contributor must be non-empty".into());
        }
        if data.is_empty() {
            return Err("data must be non-empty".into());
        }
        // Bootstrap ingress screen (advisory, node-local): reject trivial / near-duplicate content
        // before it reaches the mempool. Not a consensus rule — this node's own spam filter.
        if let Err(reject) = self.screen.check(data.as_bytes()) {
            return Err(reject.message());
        }
        let id = self.next_id;
        self.next_id += 1;
        // The contributor key is soulbound (`type_script.args`); the lock owner is the same handle for
        // this simple devnet. A distinct per-id secret keeps ordering coordinates unique.
        let mut secret = [0u8; 32];
        secret[..8].copy_from_slice(&id.to_le_bytes());
        let cell = Cell {
            id,
            lock: Script { code_hash: [0u8; 32], args: contributor.as_bytes().to_vec() },
            type_script: Script { code_hash: [1u8; 32], args: contributor.as_bytes().to_vec() },
            parent: None,
            timestamp: id,
            data: data.as_bytes().to_vec(),
        };
        self.node.submit(cell, Committed { height: self.node.ledger.height + 1, secret });
        match self.spec.produce_block(&mut self.node) {
            Some(block) => {
                // Accepted + finalized ⇒ fold its shingles into the screen so later copies are caught.
                self.screen.record(data.as_bytes());
                // Durability: append the finalized block before returning. Best-effort — the block is
                // already applied in memory, so a write failure degrades durability (a restart would
                // lose this one block), not correctness; surface it loudly rather than crash the node.
                if let Some(path) = &self.store {
                    if let Err(e) = crate::store::append_block(path, &block) {
                        eprintln!("noesisd: WARN failed to persist block {}: {e}", block.height);
                    }
                }
                Ok(block.height)
            }
            None => Err("contribution did not finalize a block".into()),
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// The PURE request dispatcher: `(state, method, path, body) -> (http_status, content_type, body)`. No
/// socket, so it is fully unit-testable. The content-type rides along because the node serves both the
/// HTML frontend (`GET /`) and JSON (`/state`, `/submit`). `serve_api` is the thin transport shell.
pub fn handle_request(
    state: &mut ServerState,
    method: &str,
    path: &str,
    body: &[u8],
) -> (u16, &'static str, Vec<u8>) {
    let out = |status: u16, v: Value| (status, "application/json", v.to_string().into_bytes());
    match (method, path) {
        ("OPTIONS", _) => (204, "application/json", Vec::new()), // CORS preflight
        // The embedded frontend: one URL serves the whole app (same-origin ⇒ its /state + /submit
        // fetches are relative, so a single tunnel needs no CORS and no hardcoded node address).
        ("GET", "/") | ("GET", "/index.html") => (200, "text/html; charset=utf-8", INDEX_HTML.as_bytes().to_vec()),
        ("GET", "/state") => out(200, state.state_json()),
        ("POST", "/submit") => {
            let parsed: Result<Value, _> = serde_json::from_slice(body);
            let v = match parsed {
                Ok(v) => v,
                Err(e) => return out(400, json!({ "error": format!("invalid JSON: {e}") })),
            };
            let contributor = v.get("contributor").and_then(Value::as_str).unwrap_or("");
            let data = v.get("data").and_then(Value::as_str).unwrap_or("");
            match state.submit(contributor, data) {
                Ok(height) => {
                    let mut resp = state.state_json();
                    resp["finalized"] = json!(true);
                    resp["block_height"] = json!(height);
                    out(200, resp)
                }
                Err(e) => out(400, json!({ "error": e })),
            }
        }
        ("GET", _) | ("POST", _) => out(404, json!({ "error": "not found" })),
        _ => out(405, json!({ "error": "method not allowed" })),
    }
}

/// Read a bounded HTTP request (request line + headers + `Content-Length` body) from a stream clone.
/// Returns `(method, path, body)`. The body is capped at [`MAX_BODY`]; the socket carries a read
/// timeout so a slow-loris client cannot hang the serving thread indefinitely.
fn read_http_request(read_half: TcpStream) -> std::io::Result<(String, String, Vec<u8>)> {
    let mut reader = BufReader::new(read_half);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 || line.trim_end().is_empty() {
            break; // end of headers (blank line) or EOF
        }
        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let content_length = content_length.min(MAX_BODY);
    let mut bodybuf = vec![0u8; content_length];
    reader.read_exact(&mut bodybuf)?;
    Ok((method, path, bodybuf))
}

/// Write an HTTP/1.1 response with the given `content_type` (HTML for the frontend, JSON for the API)
/// and permissive CORS (so a browser frontend on any origin can also call the node during development).
fn write_http_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// Bind `addr` and serve the API forever (one thread per connection, the shared node behind a mutex).
/// The node is DURABLE: it replays the block log at `store_path` on boot and appends every finalized
/// block, so a restart resumes the chain instead of resetting to genesis. Every `POST /submit` builds
/// the chain one finalized block at a time.
pub fn serve_api(addr: &str, store_path: &str, spec: ChainSpec) {
    let listener = TcpListener::bind(addr).unwrap_or_else(|e| {
        eprintln!("noesisd: failed to bind {addr}: {e}");
        std::process::exit(1);
    });
    let bound = listener.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| addr.to_string());
    let chain_id = spec.chain_id;
    let (server_state, replayed) = ServerState::with_store(store_path, spec).unwrap_or_else(|e| {
        eprintln!("noesisd: failed to open chain store {store_path}: {e}");
        std::process::exit(1);
    });
    println!("noesisd --serve-api — Noesis live node API (chain_id=0x{chain_id:x})");
    if replayed > 0 {
        println!("resumed durable chain from {store_path}: replayed {replayed} blocks, height {}.", server_state.node.ledger.height);
    } else {
        println!("genesis: empty ledger, ChainSpec::dev() [PoW enforced, JUL issuing]. persisting to {store_path}.");
    }
    println!("LISTENING http://{bound}");
    println!("  open http://{bound}/ in a browser — the node serves its own frontend (one URL = whole app)");
    println!("  GET  /                   -> the embedded web UI (submit + watch PoM/JUL)");
    println!("  GET  /state              -> chain view (height, jul, contributors)");
    println!("  POST /submit {{contributor,data}} -> add a contribution, mine+finalize a block");

    let state = Arc::new(Mutex::new(server_state));
    for conn in listener.incoming() {
        let mut stream = match conn {
            Ok(s) => s,
            Err(e) => {
                eprintln!("noesisd: accept failed: {e}");
                continue;
            }
        };
        let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
        let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            let read_half = match stream.try_clone() {
                Ok(s) => s,
                Err(_) => return,
            };
            let (method, path, body) = match read_http_request(read_half) {
                Ok(t) => t,
                Err(_) => return, // malformed/timed-out request: drop the connection
            };
            let (status, content_type, resp) = {
                let mut st = state.lock().unwrap_or_else(|p| p.into_inner());
                handle_request(&mut st, &method, &path, &body)
            };
            let _ = write_http_response(&mut stream, status, content_type, &resp);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_of(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).expect("response is JSON")
    }

    #[test]
    fn get_state_reports_an_empty_genesis() {
        let mut st = ServerState::new();
        let (status, ctype, body) = handle_request(&mut st, "GET", "/state", b"");
        assert_eq!(status, 200);
        assert_eq!(ctype, "application/json");
        let v = state_of(&body);
        assert_eq!(v["height"], 0);
        assert_eq!(v["jul_issued"], 0);
        assert_eq!(v["pow_enforced"], true);
    }

    #[test]
    fn get_root_serves_the_embedded_html_frontend() {
        let mut st = ServerState::new();
        let (status, ctype, body) = handle_request(&mut st, "GET", "/", b"");
        assert_eq!(status, 200);
        assert_eq!(ctype, "text/html; charset=utf-8");
        let html = String::from_utf8(body).expect("frontend is UTF-8");
        assert!(html.contains("<!"), "serves an HTML document");
        assert!(html.contains("/submit"), "the served page targets the submit endpoint");
        // /index.html is the same document (so a browser's default path also works)
        let (s2, c2, b2) = handle_request(&mut st, "GET", "/index.html", b"");
        assert_eq!((s2, c2), (200, "text/html; charset=utf-8"));
        assert_eq!(b2.len(), html.len());
    }

    #[test]
    fn submit_finalizes_a_block_and_updates_state() {
        let mut st = ServerState::new();
        let body = br#"{"contributor":"dave","data":"a genuinely new idea about winter light"}"#;
        let (status, _ctype, resp) = handle_request(&mut st, "POST", "/submit", body);
        assert_eq!(status, 200);
        let v = state_of(&resp);
        assert_eq!(v["finalized"], true);
        assert_eq!(v["block_height"], 1);
        assert_eq!(v["height"], 1);
        assert!(v["jul_issued"].as_u64().unwrap() > 0, "mined work issued JUL");
        assert!(v["contributors"].get("dave").is_some(), "dave earned standing");
        // a second contribution advances the chain to height 2
        let body2 = br#"{"contributor":"erin","data":"an entirely separate subject, cold rivers"}"#;
        let (_s, _c, resp2) = handle_request(&mut st, "POST", "/submit", body2);
        assert_eq!(state_of(&resp2)["height"], 2);
    }

    #[test]
    fn malformed_and_unknown_requests_are_rejected_cleanly() {
        let mut st = ServerState::new();
        // bad JSON body
        let (s1, _, _) = handle_request(&mut st, "POST", "/submit", b"not json");
        assert_eq!(s1, 400);
        // empty contributor
        let (s2, _, _) = handle_request(&mut st, "POST", "/submit", br#"{"contributor":"","data":"x"}"#);
        assert_eq!(s2, 400);
        // unknown path
        let (s3, _, _) = handle_request(&mut st, "GET", "/nope", b"");
        assert_eq!(s3, 404);
        // CORS preflight
        let (s4, _, _) = handle_request(&mut st, "OPTIONS", "/submit", b"");
        assert_eq!(s4, 204);
        // nothing finalized ⇒ height still 0
        assert_eq!(state_of(&handle_request(&mut st, "GET", "/state", b"").2)["height"], 0);
    }
}
