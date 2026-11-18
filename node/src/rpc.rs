//! rpc — the node's live HTTP/JSON interface: the API a friend (or a browser frontend) calls to
//! SUBMIT a contribution and READ the chain. This is what turns `noesisd` from "runs a scripted
//! workload" into "a node your friends can poke."
//!
//! Three routes, plus CORS so a browser frontend on another origin can also call them:
//!   * `GET  /`       — the embedded frontend itself (`text/html`), so ONE public URL is the whole app.
//!   * `GET  /state`  — the chain view: height, JUL issued, per-contributor PoM standing, cell counts.
//!   * `POST /submit` — a contribution signed in wallet format v2 (`parse_signed`: identity, hot_root,
//!                      cold_commit, index, the XMSS^MT `bottom`/`top` signature, optional `parent`,
//!                      `data`); the node verifies it, builds a cell, runs
//!                      the ONE proven per-block engine ([`crate::chainspec::ChainSpec::produce_block`]),
//!                      and returns the new state. A submitted contribution travels the EXACT path the
//!                      tested chain does — no second, drifting pipeline.
//!   * `GET  /tokens`   — live non-JUL token cells (soulbound identities decoded), pending and recently
//!                      carried token txs.
//!   * `POST /token-tx` — a signed soulbound identity tx (`identity_mint::envelope`), held until the next
//!                      contribution block carries it. Soulbound only, every input signed.
//!   * `GET  /faucet`   — the testnet faucet: whether this node runs one, its JUL, the drip and limits.
//!   * `POST /faucet`   — `{address}`: queue a small test-JUL payout to that 32-byte lock, carried by the
//!                      next contribution block ([`crate::faucet`]; 429 on a limit, 503 when it cannot pay).
//!   * `GET  /balance/<address>` — the JUL held under that 32-byte lock.
//!   * `GET  /blocks`   — the explorer: summaries of the newest finalized blocks.
//!   * `GET  /blocks?since=N[&limit=M]` — full finalized blocks after height N, paged, with the chain link
//!                      at N: what a follower pulls (testnet step 7, [`crate::follow`]).
//!   * `GET  /keys/<identity>` — the identity's key record (wallet decisions A1/A7): its current hot key
//!                      generation and root, the one-time index that key last used, a pending
//!                      recovery and how many blocks remain to cancel it, and a key operation waiting
//!                      here for the next block.
//!   * `POST /key`      — a signed key operation (`parse_key_op`: rotate, recover or cancel), checked by
//!                      the rulebook's own rule ([`crate::identity_key::build_tx`]) and held until the
//!                      next contribution block carries it.
//!
//! A REPLICA (`noesisd --follow`, [`serve_follower`]) serves the same reads over its own store and answers
//! `POST /submit`, `/faucet`, `/token-tx` and `/key` with 409 and the producer's URL: one producer, for now.
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
use crate::identity_key::{self, pack_label, unpack_label, KeyOp, Kind, Record};
use crate::runtime::{txs_conserve_and_single_use, Node, TokenStandard, TokenTx};
use crate::{Cell, Script};

/// Max request body we will read. A SIGNED contribution carries an XMSS^MT signature: two 16 KiB
/// Lamport OTS signatures plus two Merkle auth paths, about 66 KiB as hex, so the budget is larger
/// than a plain JSON post — still bounded to keep per-connection memory sane.
pub const MAX_BODY: usize = 160 * 1024;
/// Per-socket read/write deadline — a stalled client's thread dies instead of hanging forever.
const IO_TIMEOUT: Duration = Duration::from_secs(30);
/// The frontend, EMBEDDED at compile time so the node is ONE self-contained binary that serves its own
/// UI at `/`. Same-origin ⇒ the page fetches `/state` and `/submit` relative to wherever it is served,
/// so a single tunnel (one public URL) exposes the whole app with no CORS and no hardcoded node address.
const INDEX_HTML: &str = include_str!("../../frontend/index.html");
/// The wallet's crypto (real hash-based keys), embedded and served at `/crypto.js`. Kept as its own
/// file so it stays auditable and is parity-tested against the Rust `noesis_core::{lamport, xmss}`.
const CRYPTO_JS: &str = include_str!("../../frontend/crypto.js");
/// The wallet's storage door (`frontend/wallet.js`), embedded and served at `/wallet.js`: the only page
/// code that touches the stored wallet. It keeps the hot seed encrypted at rest (a random data key wrapped
/// by a passphrase and optionally a passkey) and the one-time-key counter. Like the page, it reaches a
/// running node only through a rebuild, because it is compiled in.
const WALLET_JS: &str = include_str!("../../frontend/wallet.js");
/// Questions templated from the current doc corpus (`scripts/gen-questions.mjs`), embedded and served
/// at `/questions.json`. The UI shuffles + flows them, seeded by chain height — a deterministic feed of
/// open questions drawn from the repo's own docs. Regenerate when the docs change.
const QUESTIONS_JSON: &str = include_str!("../../frontend/questions.json");

/// Lower-case hex, for the wallet address + signature fields on the wire (JSON is text, keys are bytes).
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
/// Decode lower/upper hex to bytes; `None` on any non-hex char or odd length.
pub(crate) fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// The value of `key` in a URL query string (`a=1&b=2`), undecoded (the node's queries are numbers).
fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|kv| kv.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
}

/// Most `POST /token-tx` txs a node holds while they wait for a contribution block to carry them.
const MAX_PENDING_TOKEN_TXS: usize = 16;
/// How many recent (digest, height) inclusions `GET /tokens` reports, so a minting tool can learn
/// which block carried its tx. Node-local memory: lost on restart (the tool also sees the cells land).
const MAX_INCLUDED_RECORD: usize = 256;

/// One token cell as `GET /tokens` shows it: every identity field in hex (enough to rebuild the exact
/// cell a tx must name as an input), plus the decoded record for a soulbound cell.
pub fn token_cell_json(c: &Cell) -> Value {
    use crate::tokens::soulbound::{self, Record};
    let mut v = json!({
        "id": c.id,
        "lock_code_hash": hex_encode(&c.lock.code_hash),
        "lock_args": hex_encode(&c.lock.args),
        "type_code_hash": hex_encode(&c.type_script.code_hash),
        "type_args": hex_encode(&c.type_script.args),
        "data": hex_encode(&c.data),
    });
    if c.type_script.code_hash == soulbound::CODE_HASH {
        v["standard"] = json!("soulbound");
        v["collection"] = json!(String::from_utf8_lossy(&c.type_script.args));
        v["record"] = match soulbound::decode(&c.data) {
            Some(Record::Authority { generation }) => json!({ "kind": "authority", "generation": generation }),
            Some(Record::Identity(i)) => json!({
                "kind": "identity",
                "serial": i.serial,
                "tier": i.tier.name(),
                "holder": hex_encode(&i.holder),
            }),
            None => Value::Null,
        };
    } else if c.type_script.code_hash == identity_key::CODE_HASH {
        v["standard"] = json!("identity-key");
        v["record"] = identity_key::Record::decode(&c.data).map_or(Value::Null, |r| record_json(&r, 0));
    }
    v
}

/// Rebuild a token cell from [`token_cell_json`]'s hex fields (`parent`/`timestamp` are not part of a
/// cell's identity and come back as `None`/0, the token-cell convention).
pub fn token_cell_from_json(v: &Value) -> Result<Cell, String> {
    let hex = |f: &str| -> Result<Vec<u8>, String> {
        v.get(f).and_then(Value::as_str).and_then(hex_decode).ok_or(format!("token cell: '{f}' missing or not hex"))
    };
    let h32 = |f: &str| -> Result<[u8; 32], String> {
        <[u8; 32]>::try_from(hex(f)?.as_slice()).map_err(|_| format!("token cell: '{f}' must be 32 bytes"))
    };
    Ok(Cell {
        id: v.get("id").and_then(Value::as_u64).ok_or("token cell: 'id' missing")?,
        lock: Script { code_hash: h32("lock_code_hash")?, args: hex("lock_args")? },
        type_script: Script { code_hash: h32("type_code_hash")?, args: hex("type_args")? },
        parent: None,
        timestamp: 0,
        data: hex("data")?,
    })
}
/// Decode a hex field expected to be exactly 32 bytes (an address / root / sibling).
fn hex32(v: &Value, field: &str) -> Result<[u8; 32], String> {
    let s = v.get(field).and_then(Value::as_str).ok_or_else(|| format!("missing '{field}'"))?;
    let b = hex_decode(s).ok_or_else(|| format!("'{field}' is not valid hex"))?;
    <[u8; 32]>::try_from(b.as_slice()).map_err(|_| format!("'{field}' must be 32 bytes"))
}

/// A signed contribution in wallet format v2 (`noesis_core::wallet`): the identity, the two roots it
/// hashes from, the one-time index, the hot key's XMSS^MT signature, the optional parent, and the data.
struct SignedContribution {
    identity: [u8; 32],
    hot_root: [u8; 32],
    cold_commit: [u8; 32],
    index: u32,
    sig: noesis_core::wallet::Signature,
    parent: Option<u64>,
    data: String,
}

/// One tree's part of an XMSS^MT signature (`bottom` or `top`): `{ots_root, auth, ots_sig}`.
fn parse_tree_sig(v: &Value, field: &str, index: u32) -> Result<noesis_core::xmss::Signature, String> {
    use noesis_core::xmss;
    let part = v.get(field).ok_or_else(|| format!("missing '{field}' signature"))?;
    let ots_root = hex32(part, "ots_root").map_err(|e| format!("{field}: {e}"))?;
    let ots_sig = hex_decode(part.get("ots_sig").and_then(Value::as_str).unwrap_or(""))
        .ok_or_else(|| format!("{field}: 'ots_sig' is not valid hex"))?;
    let auth_arr = part.get("auth").and_then(Value::as_array).ok_or_else(|| format!("{field}: missing 'auth' path"))?;
    if auth_arr.len() != xmss::H {
        return Err(format!("{field}: 'auth' path must have exactly {} entries", xmss::H));
    }
    let mut auth = [[0u8; 32]; xmss::H];
    for (i, e) in auth_arr.iter().enumerate() {
        let b = hex_decode(e.as_str().unwrap_or("")).ok_or_else(|| format!("{field}: 'auth' entry is not valid hex"))?;
        auth[i] = <[u8; 32]>::try_from(b.as_slice()).map_err(|_| format!("{field}: 'auth' entry must be 32 bytes"))?;
    }
    Ok(xmss::Signature { index, ots_root, auth, ots_sig })
}

/// Parse a `POST /submit` body (wallet format v2). The wallet signs `wallet::contribution_digest(
/// identity, index, parent, data)` with its hot key; [`ServerState::submit_signed`] re-derives the
/// identity and the digest and verifies, so the identity that earns PoM standing is a real key the
/// submitter controls, and the parent it builds on is the one it signed.
fn parse_signed(v: &Value) -> Result<SignedContribution, String> {
    use noesis_core::{wallet, xmss, xmss_mt};
    if v.get("hot_root").is_none() && v.get("address").is_some() {
        return Err("wallet format v1 is retired: this node takes format v2 (identity, hot_root, cold_commit, \
                    an XMSS^MT signature over a digest that covers the parent)"
            .into());
    }
    let identity = hex32(v, "identity")?;
    let hot_root = hex32(v, "hot_root")?;
    let cold_commit = hex32(v, "cold_commit")?;
    let index = v
        .get("index")
        .and_then(Value::as_u64)
        .filter(|&g| g < wallet::MAX_SIGS)
        .ok_or_else(|| format!("'index' must be a one-time key index below {}", wallet::MAX_SIGS))? as u32;
    let parent = match v.get("parent") {
        None | Some(Value::Null) => None,
        Some(p) => Some(p.as_u64().ok_or("'parent' must be a contribution id")?),
    };
    let data = v.get("data").and_then(Value::as_str).ok_or("missing 'data'")?.to_string();
    // The MT index splits into (subtree, leaf); each tree's part carries its own index.
    let bottom = parse_tree_sig(v, "bottom", index & (xmss::LEAVES as u32 - 1))?;
    let top = parse_tree_sig(v, "top", index >> xmss::H)?;
    let sig = xmss_mt::MtSignature { g: index, bottom, top };
    Ok(SignedContribution { identity, hot_root, cold_commit, index, sig, parent, data })
}

/// The node's wall clock in seconds, for node-local policy only (the faucet's limits). Never consensus.
fn node_clock() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

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
    /// Anti-replay for the one-time XMSS^MT leaves: the highest index accepted per (identity, key
    /// generation), so counters are per key (wallet decision A1): a rotated or recovered identity's new
    /// key starts at 0. A new signed submit must use a STRICTLY GREATER index than its key's (each
    /// Lamport leaf signs at most once), and above any index that key spent on a key operation
    /// ([`Self::highest_used`]). Durable by construction: rebuilt on boot from each cell's `lock.args`
    /// label ([`identity_key::unpack_label`]), so it needs no sidecar and survives a restart.
    used_index: std::collections::HashMap<([u8; 32], u64), u64>,
    /// Key operations (`POST /key`) waiting for the next contribution block. Node-local.
    pending_key_ops: Vec<QueuedKeyOp>,
    /// Signed soulbound txs from `POST /token-tx`, waiting for the next contribution block to carry
    /// them (a block needs a contribution, so a token tx never makes a block alone). Node-local.
    pending_token_txs: Vec<TokenTx>,
    /// Recent inclusions `(tx digest, block height)`, newest last, bounded. Node-local.
    included_token_txs: Vec<([u8; 32], u64)>,
    /// The explorer's view (`GET /blocks`, testnet step 4): one short summary per finalized block,
    /// oldest first, bounded to the newest [`MAX_BLOCK_SUMMARIES`]. Rebuilt from the chain log at boot.
    blocks: Vec<Value>,
    /// The testnet faucet (step 3), present iff this node holds the faucet seed ([`Self::with_faucet`]).
    /// On a `reward_to_faucet` chain it is also where every block reward goes.
    faucet: Option<crate::faucet::Faucet>,
    /// Seconds, for the faucet's limits. The wall clock; a test may pin it.
    pub clock: fn() -> u64,
    /// The cumulative chain link through each height ([`crate::follow::chain_link`]): `links[0]` is
    /// genesis (zeros), `links[h]` commits to every block up to h. Node-local, never consensus; served as
    /// the `anchor` of `GET /blocks?since=N` so a follower can tell its history is still the producer's.
    links: Vec<[u8; 32]>,
    /// Byte offset of each finalized block's frame in the store (`offsets[h - 1]` is block h). Served
    /// blocks are read back from disk, never held in memory. It grows only while appends succeed in
    /// order, so `GET /blocks?since=N` serves a contiguous durable prefix and nothing else.
    offsets: Vec<u64>,
    /// `Some(url)`: this node is a replica of the producer at `url` (testnet step 7, [`crate::follow`]).
    /// It produces nothing: `POST /submit`, `/faucet` and `/token-tx` answer 409 with this URL.
    producer: Option<String>,
    /// A replica's follow status for `GET /state`: the last transport error (cleared on success) and,
    /// once following has stopped, why.
    follow_error: Option<String>,
    follow_stopped: Option<String>,
    /// `Some(why)`: a finalized block could not be written to the block log. Nothing after it is
    /// written (the log would have a gap no boot can replay) and this node takes no more blocks until it
    /// is restarted, when it resumes from the blocks that did reach the log.
    store_failed: Option<String>,
}

/// How many block summaries the explorer keeps (the newest).
const MAX_BLOCK_SUMMARIES: usize = 1000;

/// A finalized block as the explorer shows it: height, header digest, the contribution cell ids it
/// finalized, how many token txs it carried, and who its JUL reward was paid to (hex of the lock args).
fn block_summary(b: &crate::runtime::Block) -> Value {
    json!({
        "height": b.height,
        "hash": hex_encode(&crate::runtime::header_digest(b)),
        "cells": b.cells.iter().map(|c| c.id).collect::<Vec<u64>>(),
        "token_txs": b.token_txs.len(),
        "coinbase": b.coinbase.as_ref().map(|s| hex_encode(&s.args)),
    })
}

/// A key operation this node holds for the next block (`POST /key`), with the tx the rulebook's rule built
/// for that block and the record it leaves. Rebuilt before every block ([`ServerState::refresh_key_ops`]);
/// dropped once a block carries it or once it no longer holds. Node-local: lost on restart (the wallet
/// keeps the signed body and sends it again).
struct QueuedKeyOp {
    identity: [u8; 32],
    op: KeyOp,
    encoded: Vec<u8>,
    height: u64,
    tx: TokenTx,
    after: Record,
}

/// Most key operations a node holds while they wait for a contribution block (one per identity).
const MAX_PENDING_KEY_OPS: usize = 16;

/// A key record as the API shows it. `next_block` is the height of the block a cancel would land in.
fn record_json(r: &Record, next_block: u64) -> Value {
    json!({
        "generation": r.generation,
        "hot_root": hex_encode(&r.hot_root),
        "cold_commit": hex_encode(&r.cold_commit),
        "hot_floor": r.hot_floor,
        "cold_floor": r.cold_floor,
        "pending": r.pending.map(|p| json!({
            "new_root": hex_encode(&p.new_root),
            "effective_at": p.effective_at,
            "cancel_blocks_left": p.effective_at.saturating_sub(next_block),
        })),
    })
}

/// Parse a `POST /key` body: `{op, identity, index, new_root?, birth?: {hot_root, cold_commit},
/// cold_root?, bottom, top}`. `new_root` for a rotate or recover, `cold_root` for a recover, `birth` for an
/// identity's first key operation. The signature is the signer's XMSS^MT signature over
/// `wallet::key_op_digest(identity, op, chain_id, generation, new_root, index)`.
fn parse_key_op(v: &Value) -> Result<([u8; 32], KeyOp), String> {
    use noesis_core::{wallet, xmss, xmss_mt};
    let kind = v
        .get("op")
        .and_then(Value::as_str)
        .and_then(Kind::from_name)
        .ok_or("'op' must be rotate, recover or cancel")?;
    let identity = hex32(v, "identity")?;
    let index = v
        .get("index")
        .and_then(Value::as_u64)
        .filter(|&g| g < wallet::MAX_SIGS)
        .ok_or_else(|| format!("'index' must be a one-time key index below {}", wallet::MAX_SIGS))? as u32;
    let new_root = if kind == Kind::Cancel { [0u8; 32] } else { hex32(v, "new_root")? };
    let birth = match v.get("birth") {
        None | Some(Value::Null) => None,
        Some(b) => Some((hex32(b, "hot_root")?, hex32(b, "cold_commit")?)),
    };
    let cold_root = if kind == Kind::Recover { Some(hex32(v, "cold_root")?) } else { None };
    let bottom = parse_tree_sig(v, "bottom", index & (xmss::LEAVES as u32 - 1))?;
    let top = parse_tree_sig(v, "top", index >> xmss::H)?;
    if bottom.ots_sig.len() != 256 * 64 || top.ots_sig.len() != 256 * 64 {
        return Err("'ots_sig' must be a 16 KiB Lamport signature".into());
    }
    let sig = xmss_mt::MtSignature { g: index, bottom, top };
    Ok((identity, KeyOp { kind, index, new_root, birth, cold_root, sig }))
}

impl ServerState {
    /// In-memory genesis (no persistence) — friends' contributions ARE the chain from block zero. Used
    /// by tests; the live server uses [`with_store`](Self::with_store) so the chain survives a restart.
    pub fn new() -> Self {
        let spec = ChainSpec::dev();
        let (node, _keys) = spec.genesis_node();
        ServerState {
            spec,
            node,
            next_id: 1,
            store: None,
            screen: crate::screen::Screen::new(),
            used_index: std::collections::HashMap::new(),
            pending_key_ops: Vec::new(),
            pending_token_txs: Vec::new(),
            included_token_txs: Vec::new(),
            blocks: Vec::new(),
            faucet: None,
            clock: node_clock,
            links: vec![[0u8; 32]],
            offsets: Vec::new(),
            producer: None,
            follow_error: None,
            follow_stopped: None,
            store_failed: None,
        }
    }

    /// In-memory node on any spec (tests: the testnet genesis without a store).
    pub fn with_spec(spec: ChainSpec) -> Self {
        let (node, _keys) = spec.genesis_node();
        ServerState { spec, node, ..ServerState::new() }
    }

    /// Boot a DURABLE node: replay the persisted block log at `path` (or start at genesis if absent),
    /// and append every future finalized block there. This is what makes a hosted node "stay on" — a
    /// process restart resumes the chain rather than wiping it. Returns the state + how many blocks were
    /// replayed so the caller can announce it. A torn last record (a crash mid-write) is set aside before
    /// anything is appended; a damaged log is an error and is left as it is ([`crate::store::load_for_append`]).
    pub fn with_store(path: impl Into<std::path::PathBuf>, spec: ChainSpec) -> std::io::Result<(Self, usize)> {
        let path = path.into();
        let (node, loaded, offsets) = crate::store::load_for_append(&path, &spec)?;
        let replayed = loaded.len();
        let skip = replayed.saturating_sub(MAX_BLOCK_SUMMARIES);
        let blocks: Vec<Value> = loaded[skip..].iter().map(block_summary).collect();
        let mut links = vec![[0u8; 32]];
        for b in &loaded {
            links.push(crate::follow::chain_link(links.last().expect("starts with genesis"), b));
        }
        // Resume id assignment past the highest persisted cell id (never collide with a replayed cell).
        let next_id = node.ledger.cells.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        // Rebuild the ingress screen's seen-set AND the per-address anti-replay high-water mark from the
        // replayed chain, so both originality checks and one-time-leaf enforcement survive a restart.
        let mut screen = crate::screen::Screen::new();
        let mut used_index: std::collections::HashMap<([u8; 32], u64), u64> = std::collections::HashMap::new();
        for c in &node.ledger.cells {
            screen.record(&c.data);
            if let Some((addr, index, generation)) = unpack_label(&c.lock.args) {
                let e = used_index.entry((addr, generation)).or_insert(0);
                *e = (*e).max(index);
            }
        }
        Ok((
            ServerState {
                spec,
                node,
                next_id,
                store: Some(path),
                screen,
                used_index,
                pending_key_ops: Vec::new(),
                pending_token_txs: Vec::new(),
                included_token_txs: Vec::new(),
                blocks,
                faucet: None,
                clock: node_clock,
                links,
                offsets,
                producer: None,
                follow_error: None,
                follow_stopped: None,
                store_failed: None,
            },
            replayed,
        ))
    }

    /// Make this node a replica of the producer at `producer` (a normalized `http(s)://` URL): it
    /// produces nothing and runs no faucet, so it needs no faucet seed even on a `reward_to_faucet`
    /// chain (consensus fixes the reward amount and never checks the recipient, so a follower validates
    /// a reward paid to the faucet without knowing the faucet's key). Submit, faucet and token-tx
    /// answer 409 with the producer's URL.
    pub fn as_replica(mut self, producer: String) -> Self {
        self.producer = Some(producer);
        self.faucet = None;
        self
    }

    /// This node's chain height.
    pub fn height(&self) -> u64 {
        self.node.ledger.height
    }

    /// This node's chain id.
    pub fn chain_id(&self) -> u64 {
        self.spec.chain_id()
    }

    /// The cumulative chain link through height `h` (zeros for genesis or a height this node lacks).
    pub fn link(&self, h: u64) -> [u8; 32] {
        self.links.get(h as usize).copied().unwrap_or([0u8; 32])
    }

    /// The full chain state digest (cells, index root, PoM, token cells, work clock) rendered as text:
    /// two nodes print the same string iff they hold the same state.
    pub fn state_digest(&self) -> String {
        format!("{:?}", self.node.ledger.state_digest())
    }

    /// The replica's last transport error, if the last attempt failed.
    pub fn follow_error(&self) -> Option<&str> {
        self.follow_error.as_deref()
    }

    pub(crate) fn set_follow_error(&mut self, e: Option<String>) {
        self.follow_error = e;
    }

    /// Following stopped for good (a rewrite, a refused block, another network): record why.
    pub(crate) fn stop_following(&mut self, why: String) {
        self.follow_stopped = Some(why);
    }

    /// Why this node takes no more blocks, if a finalized block could not be written to its block log.
    pub fn store_failed(&self) -> Option<&str> {
        self.store_failed.as_deref()
    }

    /// FOLLOWER: accept a block pulled from the producer through the producer's own acceptance path
    /// ([`ChainSpec::accept_block`]: the rulebook, with PoW and control binding, then the finality
    /// gate), then record it exactly as a produced block is recorded (screen, one-time-key marks,
    /// explorer, chain link, durable store). Standing is what this node's `apply` derives. On a refusal
    /// nothing changes, in memory or on disk. An error after acceptance means the block could not be
    /// written to the block log ([`Self::store_failed`]).
    pub fn follow_block(&mut self, block: &crate::runtime::Block) -> Result<(), String> {
        self.spec.accept_block(&mut self.node, block)?;
        self.record_block(block)
    }

    /// Fold a block that has just been accepted into the node-local views, the same for a produced
    /// block and a followed one: the ingress screen, the per-identity one-time-key high-water mark, the
    /// id counter, token-tx inclusions, the explorer summaries, the chain link, and the durable store.
    /// `Err` iff the block could not be written to the block log (see [`Self::persist`]).
    fn record_block(&mut self, block: &crate::runtime::Block) -> Result<(), String> {
        for c in &block.cells {
            self.screen.record(&c.data);
            if let Some((addr, index, generation)) = unpack_label(&c.lock.args) {
                let e = self.used_index.entry((addr, generation)).or_insert(0);
                *e = (*e).max(index);
            }
            self.next_id = self.next_id.max(c.id + 1);
        }
        for t in &block.token_txs {
            if t.standard == TokenStandard::IdentityKey {
                self.pending_key_ops.retain(|q| q.identity.as_slice() != t.args.as_slice());
            }
            let d = t.digest();
            self.pending_token_txs.retain(|p| p.digest() != d);
            self.included_token_txs.push((d, block.height));
        }
        let excess = self.included_token_txs.len().saturating_sub(MAX_INCLUDED_RECORD);
        self.included_token_txs.drain(..excess);
        self.blocks.push(block_summary(block));
        let over = self.blocks.len().saturating_sub(MAX_BLOCK_SUMMARIES);
        self.blocks.drain(..over);
        let link = crate::follow::chain_link(self.links.last().expect("starts with genesis"), block);
        self.links.push(link);
        self.persist(block)
    }

    /// Durability: append the finalized block to the block log, written and flushed to disk
    /// (`store::append_block_at`), and only then add it to the served log (`offsets`), so `GET /blocks`
    /// hands a follower only blocks already on disk, in order. If the write fails the block stays applied
    /// in memory but is not served, and this node writes nothing more and takes no more blocks
    /// (`store_failed`): a later block written without it would leave a gap no boot can replay. A restart
    /// resumes from the blocks that reached the log.
    fn persist(&mut self, block: &crate::runtime::Block) -> Result<(), String> {
        let Some(path) = &self.store else { return Ok(()) };
        if let Some(why) = &self.store_failed {
            return Err(why.clone());
        }
        let written = if self.offsets.len() as u64 + 1 != block.height {
            Err(format!("the log holds {} blocks, so block {} would leave a gap", self.offsets.len(), block.height))
        } else {
            crate::store::append_block_at(path, block).map_err(|e| e.to_string())
        };
        match written {
            Ok(offset) => {
                self.offsets.push(offset);
                Ok(())
            }
            Err(e) => {
                let why = format!(
                    "block {} could not be written to the block log {}: {e}. It is not served and will not survive a \
                     restart; this node takes no more blocks until it is restarted",
                    block.height,
                    path.display()
                );
                eprintln!("noesisd: ERROR {why}");
                self.store_failed = Some(why.clone());
                Err(why)
            }
        }
    }

    /// `GET /blocks?since=N`: the finalized blocks after height `since`, oldest first, in full (the
    /// exact bytes in the store), at most `limit` blocks and about `max_bytes` (always at least one
    /// when one is due), with the chain link at `since` as the `anchor`. Only blocks in the durable log
    /// are served, and a block reaches it only after it finalized, so a pending faucet payout's
    /// signatures never leave the node here before the block that spends its cells is final.
    fn blocks_page(&self, since: u64, limit: usize, max_bytes: usize) -> (u16, Vec<u8>) {
        let Some(path) = &self.store else {
            return (503, json!({ "error": "this node keeps no block log to serve" }).to_string().into_bytes());
        };
        let tip = self.offsets.len() as u64;
        let mut frames: Vec<Vec<u8>> = Vec::new();
        if since < tip {
            let mut f = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(e) => return (503, json!({ "error": format!("cannot read the block log: {e}") }).to_string().into_bytes()),
            };
            let mut bytes = 0usize;
            for h in since + 1..=tip.min(since + limit.max(1) as u64) {
                if !frames.is_empty() && bytes >= max_bytes {
                    break;
                }
                match crate::store::read_frame_at(&mut f, self.offsets[(h - 1) as usize]) {
                    Ok(frame) => {
                        bytes += frame.len();
                        frames.push(frame);
                    }
                    Err(e) => {
                        return (503, json!({ "error": format!("cannot read block {h}: {e}") }).to_string().into_bytes())
                    }
                }
                if bytes >= max_bytes {
                    break;
                }
            }
        }
        let next = since + frames.len() as u64;
        let head = json!({
            "chain_id": self.spec.chain_id(),
            "since": since,
            "tip": tip,
            "anchor": if since <= tip { Value::String(hex_encode(&self.link(since))) } else { Value::Null },
            "next": next,
            "more": next < tip,
        })
        .to_string();
        // Splice the stored block bytes in as they are (each is one JSON object from `encode_block`).
        let mut body = head.into_bytes();
        body.pop(); // the closing brace
        body.extend_from_slice(b",\"blocks\":[");
        for (i, frame) in frames.iter().enumerate() {
            if i > 0 {
                body.push(b',');
            }
            body.extend_from_slice(frame);
        }
        body.extend_from_slice(b"]}");
        (200, body)
    }

    /// Run the faucet with `seed`. Refused on a chain without control binding: there an empty
    /// signature spends any cell, so the faucet's JUL would be anyone's. The faucet keeps no state of its
    /// own across restarts; it finds its cells in the live ledger by key derivation.
    pub fn with_faucet(mut self, seed: [u8; 32]) -> Result<Self, String> {
        if !self.node.constitution.control_binding {
            return Err(format!(
                "chain 0x{:x} has control binding off, so anyone could spend the faucet's cells; no faucet here",
                self.spec.chain_id()
            ));
        }
        self.faucet = Some(crate::faucet::Faucet::new(seed));
        Ok(self)
    }

    /// The chain view as JSON.
    fn state_json(&self) -> Value {
        let (ids, _root, pom, tokens, work) = self.node.ledger.state_digest();
        // PoM standing is keyed by the contributor's soulbound identity = their wallet address (32
        // bytes, the hash of the hot root and cold commitment), presented as hex (a real key, not a
        // human-typed handle).
        let contributors: serde_json::Map<String, Value> =
            pom.iter().map(|(k, v)| (hex_encode(k), json!(v))).collect();
        json!({
            "chain_id": self.spec.chain_id(),
            "height": self.node.ledger.height,
            "work": work,
            "jul_issued": self.node.ledger.jul_supply.issued() / crate::jul::JUL_BASE_UNITS,
            "cells": ids.len(),
            // JUL cells only (every token cell was one before other tokens existed); the rest: /tokens.
            "coinbase_cells": self.node.ledger.token_cells.iter().filter(|c| crate::jul::is_jul(c)).count(),
            "token_cells": tokens.len(),
            "pow_enforced": self.spec.constitution.pow_enforced,
            "control_binding": self.node.constitution.control_binding,
            "contributors": contributors,
            "role": if self.producer.is_some() { "replica" } else { "producer" },
            "replica": self.producer.as_ref().map(|p| json!({
                "producer": p,
                "following": self.follow_stopped.is_none(),
                "stopped": self.follow_stopped,
                "last_error": self.follow_error,
            })),
        })
    }

    /// The 409 a replica answers to anything that would need it to produce a block.
    fn replica_refusal(&self) -> Option<Value> {
        self.producer.as_ref().map(|p| {
            json!({
                "error": format!("this node is a replica of {p} and produces no blocks; send this request to the producer"),
                "producer": p,
            })
        })
    }

    /// `GET /tokens`: every live non-JUL token cell (JUL coinbase cells are summed in `/state`), plus
    /// the token txs this node holds and recently carried.
    fn tokens_json(&self) -> Value {
        let cells: Vec<Value> = self
            .node
            .ledger
            .token_cells
            .iter()
            .filter(|c| !crate::jul::is_jul(c))
            .map(token_cell_json)
            .collect();
        let jul_cells = self.node.ledger.token_cells.len() - cells.len();
        let pending: Vec<String> = self.pending_token_txs.iter().map(|t| hex_encode(&t.digest())).collect();
        let included: Vec<Value> = self
            .included_token_txs
            .iter()
            .map(|(d, h)| json!({ "digest": hex_encode(d), "height": h }))
            .collect();
        json!({
            "chain_id": self.spec.chain_id(),
            "height": self.node.ledger.height,
            "jul_cells": jul_cells,
            "cells": cells,
            "pending": pending,
            "included": included,
        })
    }

    /// `POST /token-tx`: hold a signed soulbound tx for the next contribution block. Node-local policy,
    /// stricter than consensus today: soulbound only (the route opens no other value movement), and
    /// every input must present its owner's signature, verified now (control binding on this route
    /// ahead of the consensus flip, so only the issuer's key can use it). The tx must be valid against
    /// the live chain together with every tx already pending. Re-sending a pending tx is a no-op.
    fn queue_token_tx(&mut self, tx: TokenTx) -> Result<[u8; 32], String> {
        if tx.standard != TokenStandard::Soulbound {
            return Err("this route takes soulbound identity txs only".into());
        }
        if tx.inputs.is_empty() || tx.auths.len() != tx.inputs.len() || tx.auths.iter().any(Vec::is_empty) {
            return Err("every input must carry its owner's signature".into());
        }
        let digest = tx.digest();
        if self.pending_token_txs.iter().any(|p| p.digest() == digest) {
            return Ok(digest);
        }
        if self.pending_token_txs.len() >= MAX_PENDING_TOKEN_TXS {
            return Err("token tx queue is full; wait for a block".into());
        }
        let mut batch = self.pending_token_txs.clone();
        batch.push(tx.clone());
        if !txs_conserve_and_single_use(&self.node.ledger.token_cells, &batch, self.node.constitution.control_binding) {
            return Err("token tx is not valid against the live chain (or conflicts with a pending one)".into());
        }
        self.pending_token_txs.push(tx);
        Ok(digest)
    }

    /// The pending token txs a block built now can carry: drop any that can no longer land (an input
    /// spent since), then keep the largest valid prefix-greedy set, the faucet's pending payout last.
    fn carry_pending(&mut self) -> Vec<TokenTx> {
        let live = &self.node.ledger.token_cells;
        let binding = self.node.constitution.control_binding;
        self.pending_token_txs.retain(|t| txs_conserve_and_single_use(live, std::slice::from_ref(t), binding));
        let mut carried: Vec<TokenTx> = Vec::new();
        let faucet_tx = self.faucet.as_mut().and_then(|f| {
            f.refresh(live, binding);
            f.pending().cloned()
        });
        for t in self.pending_token_txs.iter().cloned().chain(faucet_tx) {
            carried.push(t);
            if !txs_conserve_and_single_use(live, &carried, binding) {
                carried.pop();
            }
        }
        // Key operations: each was rebuilt for this block by the rulebook's own rule.
        self.refresh_key_ops();
        carried.extend(self.pending_key_ops.iter().map(|q| q.tx.clone()));
        carried
    }

    /// Rebuild every held key operation for the next block, dropping any that no longer holds.
    fn refresh_key_ops(&mut self) {
        let height = self.node.ledger.height + 1;
        let (ledger, c) = (&self.node.ledger, &self.node.constitution);
        self.pending_key_ops.retain_mut(|q| match identity_key::build_tx(ledger, height, q.identity, &q.op, c) {
            Ok((tx, after)) => {
                (q.tx, q.after, q.height) = (tx, after, height);
                true
            }
            Err(_) => false,
        });
    }

    /// The key record that governs `identity` in the block at `height`: the record a key operation this
    /// node holds for that block leaves, else the chain's record as it stands at that height, else
    /// `None` (no key record: the identity's birth key signs).
    fn key_state(&self, identity: &[u8; 32], height: u64) -> Option<Record> {
        if let Some(q) = self.pending_key_ops.iter().find(|q| &q.identity == identity && q.height == height) {
            return Some(q.after);
        }
        identity_key::live_record(&self.node.ledger, identity).map(|r| r.at(height))
    }

    /// The highest one-time index key `generation` of `identity` has used, for a contribution or for a
    /// key operation (a cancel raises the key's hot floor past its own index), as of the block at `height`.
    fn highest_used(&self, identity: &[u8; 32], generation: u64, height: u64) -> Option<u64> {
        let contributions = self.used_index.get(&(*identity, generation)).copied();
        let key_ops = self
            .key_state(identity, height)
            .filter(|r| r.generation == generation && r.hot_floor > 0)
            .map(|r| r.hot_floor - 1);
        contributions.max(key_ops)
    }

    /// `POST /key`: hold a signed key operation for the next block, if the rulebook's rule takes it there
    /// ([`identity_key::build_tx`], the function the block check runs). One per identity; re-sending the
    /// same operation is a no-op.
    fn queue_key_op(&mut self, identity: [u8; 32], op: KeyOp) -> Result<Value, String> {
        self.refresh_key_ops();
        let height = self.node.ledger.height + 1;
        let encoded = op.encode();
        if let Some(q) = self.pending_key_ops.iter().find(|q| q.identity == identity) {
            if q.encoded == encoded {
                return Ok(self.key_op_json(q));
            }
            return Err("another key operation for this identity is waiting for the next block; send this one after it lands".into());
        }
        if self.pending_key_ops.len() >= MAX_PENDING_KEY_OPS {
            return Err("the key operation queue is full; wait for a block".into());
        }
        let (tx, after) = identity_key::build_tx(&self.node.ledger, height, identity, &op, &self.node.constitution)?;
        let q = QueuedKeyOp { identity, op, encoded, height, tx, after };
        let v = self.key_op_json(&q);
        self.pending_key_ops.push(q);
        Ok(v)
    }

    fn key_op_json(&self, q: &QueuedKeyOp) -> Value {
        json!({
            "queued": true,
            "op": q.op.kind.name(),
            "identity": hex_encode(&q.identity),
            "lands_in_block": q.height,
            "after": record_json(&q.after, q.height),
            "note": "carried by the next contribution block; from that block on the identity's key is the one in 'after'",
        })
    }

    /// `GET /keys/<identity>`: the identity's key record on chain, a key operation held here for the next
    /// block, and the record that governs the next block (`current`, `null` when the identity has no key
    /// record and its birth key signs), with the one-time index its key last used.
    fn keys_json(&self, identity: &[u8; 32]) -> Value {
        let next = self.node.ledger.height + 1;
        let c = &self.node.constitution;
        let on_chain = identity_key::live_record(&self.node.ledger, identity).map(|r| r.at(next));
        let queued = self.pending_key_ops.iter().find(|q| &q.identity == identity && q.height == next);
        let current = self.key_state(identity, next);
        let generation = current.map_or(0, |r| r.generation);
        json!({
            "identity": hex_encode(identity),
            "chain_id": c.chain_id,
            "height": self.node.ledger.height,
            "next_block": next,
            "key_rotation": c.key_rotation,
            "recovery_delay_blocks": c.recovery_delay_blocks,
            "on_chain": on_chain.as_ref().map(|r| record_json(r, next)),
            "queued": queued.map(|q| json!({ "op": q.op.kind.name(), "index": q.op.index, "new_root": hex_encode(&q.op.new_root) })),
            "current": current.as_ref().map(|r| record_json(r, next)),
            "generation": generation,
            "highest_used": self.highest_used(identity, generation, next),
        })
    }

    /// `GET /faucet`: whether this node runs the faucet, what it holds, the drip and the limits.
    fn faucet_json(&mut self) -> Value {
        use crate::faucet;
        let Some(f) = self.faucet.as_mut() else {
            return json!({ "enabled": false, "chain_id": self.spec.chain_id(), "producer": self.producer });
        };
        let balance = f.balance(&self.node.ledger.token_cells);
        json!({
            "enabled": true,
            "chain_id": self.spec.chain_id(),
            "balance_jul": (balance / crate::jul::JUL_BASE_UNITS) as u64,
            "drip_jul": (faucet::DRIP / crate::jul::JUL_BASE_UNITS) as u64,
            "waiting": f.waiting().len(),
            "pending": f.pending().map(|t| hex_encode(&t.digest())),
            "limits": {
                "per_address_secs": faucet::ADDRESS_WINDOW_SECS,
                "window_secs": faucet::WINDOW_SECS,
                "window_max_payouts": faucet::WINDOW_MAX_PAYOUTS,
                "max_waiting": faucet::MAX_PAYEES,
            },
            "note": "a payout lands with the next contribution block; only the holder of the one-time key \
                     whose Lamport root is the address can spend it",
        })
    }

    /// `GET /balance/<address>`: the JUL held under that 32-byte lock (`lock.args`).
    fn balance_json(&self, address: &[u8; 32]) -> Value {
        let cells: Vec<&Cell> = self
            .node
            .ledger
            .token_cells
            .iter()
            .filter(|c| crate::jul::is_jul(c) && c.lock.args == address.to_vec())
            .collect();
        let total: u128 = cells.iter().map(|c| crate::tokens::fungible::amount(c)).sum();
        json!({
            "address": hex_encode(address),
            "jul": (total / crate::jul::JUL_BASE_UNITS) as u64,
            "base_units": total.to_string(),
            "cells": cells.len(),
        })
    }

    /// The per-contribution view — the CONNECTION made visible. Each finalized contribution with its
    /// provenance (`parent`: what it built on) and its DOWNSTREAM usage (`downstream`: how many later
    /// contributions built on it) — the on-chain events that connect a contribution to whether it
    /// mattered. `novelty` is the temporal-novelty score (aligned to canonical commit order). Read-only
    /// projection of the finalized ledger; touches no consensus. This is what a corroboration/curation
    /// frontend consumes: you cannot corroborate what you cannot see.
    fn contributions_json(&self) -> Value {
        let cells = &self.node.ledger.cells;
        // Temporal-novelty per cell, index-aligned (ledger.cells is canonical commit order).
        let novelty = crate::temporal_novelty(cells);
        // Downstream fan-in: how many finalized cells name each cell as their provenance parent.
        let mut downstream: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        for c in cells {
            if let Some(p) = c.parent {
                *downstream.entry(p).or_insert(0) += 1;
            }
        }
        let items: Vec<Value> = cells
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let preview: String = String::from_utf8_lossy(&c.data).chars().take(80).collect();
                json!({
                    "id": c.id,
                    "contributor": hex_encode(&c.type_script.args),
                    "preview": preview,
                    "parent": c.parent,
                    "downstream": downstream.get(&c.id).copied().unwrap_or(0),
                    "novelty": novelty.get(i).copied().unwrap_or(0),
                    "timestamp": c.timestamp,
                })
            })
            .collect();
        json!({ "height": self.node.ledger.height, "count": items.len(), "contributions": items })
    }

    /// Ingest a SIGNED contribution (wallet format v2) and try to finalize a block from it. The
    /// submitter proves control of their identity (a real hash-based key, fixed with its recovery
    /// commitment at creation) by signing `wallet::contribution_digest(identity, index, parent, data)`
    /// with the hot key: no plaintext-handle theater, standing accrues only to a key someone holds.
    /// Returns the finalized height on success, or an error string (rejected/not-finalized).
    fn submit_signed(&mut self, c: SignedContribution) -> Result<u64, String> {
        use noesis_core::wallet;
        // Who this block's reward pays: the faucet's key for this height when the node runs one, else the
        // placeholder. A chain that pays its reward to the faucet never falls back to the placeholder.
        let height = self.node.ledger.height + 1;
        let reward_to = match self.faucet.as_mut() {
            Some(f) => f.reward_lock(height),
            None if self.spec.reward_to_faucet => {
                return Err("this chain pays its block reward to the faucet, and this node holds no faucet seed".into())
            }
            None => self.spec.placeholder_reward(),
        };
        let SignedContribution { identity, hot_root, cold_commit, index, sig, parent, data } = c;
        let data = data.as_str();
        if data.is_empty() {
            return Err("data must be non-empty".into());
        }
        // Provenance link (optional): a contribution may declare the finalized contribution it BUILDS
        // ON, so downstream usage — the on-chain event that a prior contribution mattered — can form.
        // Reject a dangling parent (it must name an existing finalized contribution). A real link only
        // ever CREDITS the ancestor's downstream flow (`value_v5`/`flow_gate`), never inflates the
        // child; fake-lineage value-spoofs are closed separately at the value layer (semantic floor).
        if let Some(p) = parent {
            if !self.node.ledger.cells.iter().any(|c| c.id == p) {
                return Err(format!("parent {p} names no finalized contribution"));
            }
        }
        // 1. Who may sign for this identity in this block (wallet decisions A1/A7). With no key record:
        //    its birth key, exactly as before: the identity must hash from this hot root and cold
        //    commitment. With a key record (on chain, or a key operation this node holds for this very
        //    block): its current hot root and nothing else, so a retired key is refused from the block of
        //    the operation that retired it on. The rulebook cannot re-check this (a contribution's
        //    signature never enters the block); it checks the key label written below.
        let generation = match self.key_state(&identity, height) {
            None => {
                if wallet::identity(&hot_root, &cold_commit) != identity {
                    return Err("identity is not the hash of this hot_root and cold_commit".into());
                }
                0
            }
            Some(rec) => {
                if hot_root != rec.hot_root || cold_commit != rec.cold_commit {
                    return Err(format!(
                        "the key that signed is not this identity's current hot key: the identity is at key \
                         generation {} (hot root {}); a retired key signs nothing for it",
                        rec.generation,
                        hex_encode(&rec.hot_root)
                    ));
                }
                rec.generation
            }
        };
        // 2. Cryptographic authorization: the hot key's signature must verify over the exact
        //    (identity, index, parent, data) message. This is what makes the identity REAL (existence ≠
        //    control), and the signed parent means a relay cannot re-point what this builds on.
        let digest = wallet::contribution_digest(&identity, index, parent, data.as_bytes());
        if !wallet::verify(&hot_root, index, &digest, &sig) {
            return Err("signature does not verify under this identity's hot key".into());
        }
        // 3. One-time-leaf anti-replay, per key: each XMSS^MT leaf signs at most once, so a wallet must
        //    advance its index past every contribution and key operation this key has signed. Reuse or
        //    regression is a replay attempt (or a stale wallet).
        if let Some(last) = self.highest_used(&identity, generation, height) {
            if index as u64 <= last {
                return Err(format!(
                    "leaf index {index} already used for this identity (last accepted {last}); the wallet must advance"
                ));
            }
        }
        // 4. Bootstrap ingress screen (advisory, node-local): reject trivial / near-duplicate content.
        if let Err(reject) = self.screen.check(data.as_bytes()) {
            return Err(reject.message());
        }
        let id = self.next_id;
        self.next_id += 1;
        // Soulbound identity (`type_script.args`) = the wallet identity. The lock carries the key label
        // (identity, index, and the key generation from 1 on), so the per-key high-water mark rebuilds
        // from the durable chain. A distinct per-id secret keeps ordering coordinates unique.
        let mut secret = [0u8; 32];
        secret[..8].copy_from_slice(&id.to_le_bytes());
        let cell = Cell {
            id,
            lock: Script { code_hash: [0u8; 32], args: pack_label(&identity, index as u64, generation) },
            type_script: Script { code_hash: [1u8; 32], args: identity.to_vec() },
            parent,
            timestamp: id,
            data: data.as_bytes().to_vec(),
        };
        self.node.submit(cell, Committed { height: self.node.ledger.height + 1, secret });
        // Pending token txs (POST /token-tx) ride this contribution's block; it gives them no standing.
        let carried = self.carry_pending();
        match self.spec.produce_block_paying(&mut self.node, carried, reward_to) {
            Some(block) => {
                if let Some(f) = self.faucet.as_mut() {
                    f.landed(&block);
                }
                // Accepted + finalized ⇒ fold its shingles into the screen so later copies are caught,
                // advance this identity's one-time-leaf high-water mark, and append it to the durable
                // log (before returning), the same bookkeeping a follower does for a pulled block. A block
                // the log could not take is not reported final.
                self.record_block(&block)?;
                debug_assert_eq!(self.used_index.get(&(identity, generation)), Some(&(index as u64)));
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
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    // A replica produces nothing: whatever would need a block goes to the producer (409 + its URL).
    if method == "POST" && matches!(path, "/submit" | "/faucet" | "/token-tx" | "/key") {
        if let Some(refusal) = state.replica_refusal() {
            return out(409, refusal);
        }
    }
    // A node whose block log failed a write makes no more blocks until it is restarted.
    if method == "POST" && path == "/submit" {
        if let Some(why) = state.store_failed() {
            return out(503, json!({ "error": why }));
        }
    }
    match (method, path) {
        ("OPTIONS", _) => (204, "application/json", Vec::new()), // CORS preflight
        // The embedded frontend: one URL serves the whole app (same-origin ⇒ its /state + /submit
        // fetches are relative, so a single tunnel needs no CORS and no hardcoded node address).
        ("GET", "/") | ("GET", "/index.html") => (200, "text/html; charset=utf-8", INDEX_HTML.as_bytes().to_vec()),
        ("GET", "/crypto.js") => (200, "application/javascript; charset=utf-8", CRYPTO_JS.as_bytes().to_vec()),
        ("GET", "/wallet.js") => (200, "application/javascript; charset=utf-8", WALLET_JS.as_bytes().to_vec()),
        ("GET", "/questions.json") => (200, "application/json; charset=utf-8", QUESTIONS_JSON.as_bytes().to_vec()),
        ("GET", "/state") => out(200, state.state_json()),
        ("GET", "/contributions") => out(200, state.contributions_json()),
        ("GET", "/tokens") => out(200, state.tokens_json()),
        // `?since=N[&limit=M]`: full finalized blocks after height N, oldest first, paged (live sync,
        // testnet step 7). Without `since`: the explorer, the newest 100 summaries, newest first.
        ("GET", "/blocks") => match query_param(query, "since") {
            None => out(200, json!({ "blocks": state.blocks.iter().rev().take(100).collect::<Vec<_>>() })),
            Some(since) => {
                let limit = match query_param(query, "limit") {
                    None => Some(crate::follow::PAGE_MAX_BLOCKS),
                    Some(l) => l.parse::<usize>().ok().map(|l| l.clamp(1, crate::follow::PAGE_MAX_BLOCKS)),
                };
                match (since.parse::<u64>(), limit) {
                    (Ok(since), Some(limit)) => {
                        let (status, body) = state.blocks_page(since, limit, crate::follow::PAGE_MAX_BYTES);
                        (status, "application/json", body)
                    }
                    _ => out(400, json!({ "error": "since and limit must be whole numbers" })),
                }
            }
        },
        ("POST", "/token-tx") => {
            let tx = match serde_json::from_slice::<Value>(body)
                .map_err(|e| format!("invalid JSON: {e}"))
                .and_then(|v| crate::identity_mint::from_envelope(&v))
            {
                Ok(tx) => tx,
                Err(e) => return out(400, json!({ "error": e })),
            };
            match state.queue_token_tx(tx) {
                Ok(d) => out(
                    200,
                    json!({
                        "queued": true,
                        "digest": hex_encode(&d),
                        "pending": state.pending_token_txs.len(),
                        "note": "carried by the next contribution block",
                    }),
                ),
                Err(e) => out(400, json!({ "error": e })),
            }
        }
        ("POST", "/submit") => {
            let parsed: Result<Value, _> = serde_json::from_slice(body);
            let v = match parsed {
                Ok(v) => v,
                Err(e) => return out(400, json!({ "error": format!("invalid JSON: {e}") })),
            };
            // The optional provenance link (`parent`) is parsed here and covered by the signature.
            let signed = match parse_signed(&v) {
                Ok(t) => t,
                Err(e) => return out(400, json!({ "error": e })),
            };
            match state.submit_signed(signed) {
                Ok(height) => {
                    let mut resp = state.state_json();
                    resp["finalized"] = json!(true);
                    resp["block_height"] = json!(height);
                    out(200, resp)
                }
                // the block log failed this block's write: a server fault, not the submitter's
                Err(e) if state.store_failed().is_some() => out(503, json!({ "error": e })),
                Err(e) => out(400, json!({ "error": e })),
            }
        }
        ("GET", "/faucet") => out(200, state.faucet_json()),
        ("POST", "/faucet") => {
            let address = serde_json::from_slice::<Value>(body)
                .map_err(|e| format!("invalid JSON: {e}"))
                .and_then(|v| hex32(&v, "address"));
            let address = match address {
                Ok(a) => a,
                Err(e) => return out(400, json!({ "error": e })),
            };
            let now = (state.clock)();
            let Some(f) = state.faucet.as_mut() else {
                return out(503, json!({ "error": "this node runs no faucet" }));
            };
            match f.request(address, now, &state.node.ledger.token_cells) {
                Ok(d) => out(
                    200,
                    json!({
                        "queued": true,
                        "digest": hex_encode(&d),
                        "amount_jul": (crate::faucet::DRIP / crate::jul::JUL_BASE_UNITS) as u64,
                        "waiting": f.waiting().len(),
                        "note": "carried by the next contribution block",
                    }),
                ),
                Err(crate::faucet::Refusal::Limited(e)) => out(429, json!({ "error": e })),
                Err(crate::faucet::Refusal::Unavailable(e)) => out(503, json!({ "error": e })),
            }
        }
        ("GET", p) if p.starts_with("/balance/") => match hex_decode(&p["/balance/".len()..]) {
            Some(a) if a.len() == 32 => {
                let addr = <[u8; 32]>::try_from(a.as_slice()).expect("checked length");
                out(200, state.balance_json(&addr))
            }
            _ => out(400, json!({ "error": "the address must be 64 hex characters" })),
        },
        // A wallet's one-time-key high-water mark (wallet design decision 6): the highest index this
        // node has accepted for the identity (the address), or null. A wallet resumes at max(local, this + 1),
        // so a lost or rolled-back local counter can never reuse a one-time key the chain already saw.
        // Per key (A1): the identity's CURRENT key, so a rotated or recovered wallet resumes its new key.
        ("GET", p) if p.starts_with("/index/") => match hex_decode(&p["/index/".len()..]) {
            Some(a) if a.len() == 32 => {
                let addr = <[u8; 32]>::try_from(a.as_slice()).expect("checked length");
                let next = state.node.ledger.height + 1;
                let generation = state.key_state(&addr, next).map_or(0, |r| r.generation);
                out(
                    200,
                    json!({
                        "address": hex_encode(&addr),
                        "highest_used": state.highest_used(&addr, generation, next),
                        "generation": generation,
                    }),
                )
            }
            _ => out(400, json!({ "error": "the address must be 64 hex characters" })),
        },
        ("GET", p) if p.starts_with("/keys/") => match hex_decode(&p["/keys/".len()..]) {
            Some(a) if a.len() == 32 => out(200, state.keys_json(&<[u8; 32]>::try_from(a.as_slice()).expect("checked length"))),
            _ => out(400, json!({ "error": "the identity must be 64 hex characters" })),
        },
        ("POST", "/key") => {
            let parsed = serde_json::from_slice::<Value>(body)
                .map_err(|e| format!("invalid JSON: {e}"))
                .and_then(|v| parse_key_op(&v));
            let (identity, op) = match parsed {
                Ok(t) => t,
                Err(e) => return out(400, json!({ "error": e })),
            };
            match state.queue_key_op(identity, op) {
                Ok(v) => out(200, v),
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
    // Bound the number of header lines so a client cannot exhaust memory by sending
    // millions of header lines before the terminating blank line (DoS defense).
    const MAX_HEADERS: usize = 100;
    let mut header_count = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 || line.trim_end().is_empty() {
            break; // end of headers (blank line) or EOF
        }
        header_count += 1;
        if header_count > MAX_HEADERS {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "too many headers"));
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
        409 => "Conflict",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "OK",
    };
    // no-store: the chain moves, so no tunnel or proxy may answer a follower or the page from a cache.
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
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
    serve(addr, store_path, spec, None)
}

/// [`serve_api`] as a REPLICA of the producer at `producer` (testnet step 7, [`crate::follow`]): the
/// same page and read API over its own durable store, plus a background follower that pulls, validates
/// and appends the producer's finalized blocks. It produces nothing and reads no faucet seed.
pub fn serve_follower(addr: &str, store_path: &str, spec: ChainSpec, producer: &str) {
    serve(addr, store_path, spec, Some(producer))
}

fn serve(addr: &str, store_path: &str, spec: ChainSpec, producer: Option<&str>) {
    let chain_id = spec.chain_id();
    let producer = producer.map(|p| {
        crate::follow::normalize_producer_url(p).unwrap_or_else(|e| {
            eprintln!("noesisd: {e}");
            std::process::exit(2);
        })
    });
    // A chain that pays its block reward to the faucet needs the faucet seed: read it before anything
    // else, and refuse to start without it (never fall back to the placeholder). The seed is never printed.
    // A replica produces nothing, so it needs no seed.
    let faucet_seed = if spec.reward_to_faucet && producer.is_none() {
        let path = crate::faucet::seed_path().unwrap_or_else(|| {
            eprintln!("noesisd: no home directory; set NOESIS_FAUCET_SEED to the faucet seed file");
            std::process::exit(1);
        });
        match crate::faucet::load_seed(&path) {
            Ok(seed) => {
                println!("faucet: seed read from {}", path.display());
                Some(seed)
            }
            Err(e) => {
                eprintln!("noesisd: {e}");
                eprintln!(
                    "  chain 0x{chain_id:x} pays its block reward to the faucet, so this node needs the faucet seed: \
                     a file of 64 hex characters, made from real randomness and kept out of the repository \
                     (docs/TESTNET.md, step 3). NOESIS_FAUCET_SEED overrides the path."
                );
                std::process::exit(1);
            }
        }
    } else {
        None
    };
    let listener = TcpListener::bind(addr).unwrap_or_else(|e| {
        eprintln!("noesisd: failed to bind {addr}: {e}");
        std::process::exit(1);
    });
    let bound = listener.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| addr.to_string());
    let (server_state, replayed) = ServerState::with_store(store_path, spec).unwrap_or_else(|e| {
        eprintln!("noesisd: failed to open chain store {store_path}: {e}");
        std::process::exit(1);
    });
    let server_state = match faucet_seed {
        Some(seed) => server_state.with_faucet(seed).unwrap_or_else(|e| {
            eprintln!("noesisd: {e}");
            std::process::exit(1);
        }),
        None => server_state,
    };
    let server_state = match &producer {
        Some(p) => server_state.as_replica(p.clone()),
        None => server_state,
    };
    if producer.is_some() {
        println!("noesisd --follow — Noesis replica (chain_id=0x{chain_id:x}): produces nothing, validates every block it pulls");
    } else {
        println!("noesisd --serve-api — Noesis live node API (chain_id=0x{chain_id:x})");
    }
    if replayed > 0 {
        println!("resumed durable chain from {store_path}: replayed {replayed} blocks, height {}.", server_state.node.ledger.height);
    } else {
        println!("genesis: empty ledger (chain_id=0x{chain_id:x}) [PoW enforced, JUL issuing]. persisting to {store_path}.");
    }
    println!("LISTENING http://{bound}");
    println!("  open http://{bound}/ in a browser — the node serves its own frontend (one URL = whole app)");
    println!("  GET  /                   -> the embedded web UI (submit + watch PoM/JUL)");
    println!("  GET  /state              -> chain view (height, jul, contributors)");
    println!("  GET  /tokens             -> live non-JUL token cells (soulbound identities decoded)");
    println!("  POST /submit {{identity,hot_root,cold_commit,index,bottom,top,parent?,data}} -> signed contribution (wallet v2), mine+finalize");
    println!("  POST /token-tx {{tx,auths}} -> signed soulbound tx, carried by the next contribution block");
    println!("  GET  /faucet  POST /faucet {{address}}  GET /balance/<address> -> test JUL (faucet chains only)");
    println!("  GET  /blocks?since=N     -> finalized blocks after height N, paged (what a follower pulls)");
    println!("  GET  /keys/<identity>  POST /key {{op,identity,index,...}} -> identity keys: rotate, recover, cancel");

    let height = server_state.height();
    let state = Arc::new(Mutex::new(server_state));
    if let Some(p) = producer {
        println!("  POST /submit, /faucet, /token-tx, /key -> 409 {{producer}} (this node is a replica)");
        println!("FOLLOWING {p} from height {height}");
        let follower = Arc::clone(&state);
        std::thread::spawn(move || crate::follow::run(follower, p));
    }
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

    /// Test wallets commit to a fixed cold root (the node never sees a cold key, only its commitment).
    const TEST_COLD_ROOT: [u8; 32] = [0xC0; 32];

    /// The wallet identity for hot seed `master` (and the test cold commitment).
    fn identity_of(master: &[u8; 32]) -> ([u8; 32], [u8; 32], [u8; 32]) {
        use noesis_core::wallet;
        let hot_root = wallet::key_root(master);
        let cold_commit = wallet::cold_commit(&TEST_COLD_ROOT);
        (wallet::identity(&hot_root, &cold_commit), hot_root, cold_commit)
    }

    /// One tree's part of an MT signature, as the browser wallet sends it.
    fn tree_json(t: &noesis_core::xmss::Signature) -> Value {
        json!({
            "ots_root": hex_encode(&t.ots_root),
            "auth": t.auth.iter().map(|s| hex_encode(s)).collect::<Vec<String>>(),
            "ots_sig": hex_encode(&t.ots_sig),
        })
    }

    /// Build a real signed-contribution request body the way the browser wallet does (format v2):
    /// derive the identity, sign `wallet::contribution_digest(identity, index, parent, data)` with
    /// the hot key, hex-encode.
    fn signed_body_on(master: &[u8; 32], index: u32, parent: Option<u64>, data: &str) -> Vec<u8> {
        use noesis_core::wallet;
        let (identity, hot_root, cold_commit) = identity_of(master);
        let sig = wallet::sign(master, index, &wallet::contribution_digest(&identity, index, parent, data.as_bytes()));
        let mut body = json!({
            "identity": hex_encode(&identity),
            "hot_root": hex_encode(&hot_root),
            "cold_commit": hex_encode(&cold_commit),
            "index": index,
            "bottom": tree_json(&sig.bottom),
            "top": tree_json(&sig.top),
            "data": data,
        });
        if let Some(p) = parent {
            body["parent"] = json!(p);
        }
        body.to_string().into_bytes()
    }

    /// A signed contribution that builds on nothing.
    fn signed_body(master: &[u8; 32], index: u32, data: &str) -> Vec<u8> {
        signed_body_on(master, index, None, data)
    }

    fn id_hex(master: &[u8; 32]) -> String {
        hex_encode(&identity_of(master).0)
    }

    /// Cross-implementation parity anchor. These exact bytes are also asserted by the browser wallet's
    /// `frontend/parity-test.mjs`, so the Rust verifier and the JS signer can NEVER silently diverge
    /// (a hashing change on either side breaks this test loudly). No theater: the key math is one math.
    /// The signature is deterministic, so its pinned hash proves the JS signer makes these exact bytes,
    /// and this side verifies them.
    #[test]
    fn wallet_v2_parity_vectors_are_pinned() {
        use noesis_core::{wallet, xmss};
        let (hot, cold) = ([7u8; 32], [8u8; 32]);
        assert_eq!(
            hex_encode(&xmss::keygen_address(&hot)),
            "b8034627416d512d88c00d9cda4dfe0d1edb513102674e018c4c02732a34612e",
            "single-tree XMSS address for seed=[7;32]"
        );
        let hot_root = wallet::key_root(&hot);
        assert_eq!(hex_encode(&hot_root), "12d796f0ff1476ddf8f735cf76d07eba31c2b282d26f926baf4834f13bc3cc2f", "XMSS^MT root");
        let cold_commit = wallet::cold_commit(&wallet::key_root(&cold));
        assert_eq!(hex_encode(&cold_commit), "67c96506105d8c24e3c4656e70625808b8117eeafad410c8623087cb8e1e811c", "cold_commit");
        let id = wallet::identity(&hot_root, &cold_commit);
        assert_eq!(hex_encode(&id), "af3b14f66bde666ba77b3f7486f678f88b39ece95dded520587d8c9ebcf4ebe7", "identity");
        assert_eq!(
            hex_encode(&wallet::contribution_digest(&id, 300, None, b"hello noesis")),
            "172e999b35d8e167e3c8a9d7cfaebf61b6cf124950320349fb013f60e6f07d93",
            "digest, no parent"
        );
        let on_one = wallet::contribution_digest(&id, 300, Some(1), b"hello noesis");
        assert_eq!(hex_encode(&on_one), "92bdcd71855d9533256b66e95fd0cc4765371487ca204aae6b09a6ed1b8bffe9", "digest, parent 1");
        let sig = wallet::sign(&hot, 300, &on_one);
        assert!(wallet::verify(&hot_root, 300, &on_one, &sig), "the pinned signature verifies");
        // BLAKE2b-512 over the signature bytes in wire order (bottom then top: ots_root, auth, ots_sig)
        let mut h = blake2b_ref::Blake2bBuilder::new(64).build();
        for t in [&sig.bottom, &sig.top] {
            h.update(&t.ots_root);
            for a in &t.auth {
                h.update(a);
            }
            h.update(&t.ots_sig);
        }
        let mut out = [0u8; 64];
        h.finalize(&mut out);
        assert_eq!(
            hex_encode(&out),
            "406733314cd931ba7fe698409ffd3174fcb51fc0a29cdfe83679614028a4c59eff95e26e634a6ab0676f46731970b240dc7d74e39b0ccc23ec0bc256bd31a543",
            "blake2b-512 of the MT signature at 300"
        );
    }

    /// The key series and the key-operation digest (wallet decisions A1 and A7), pinned with the same bytes
    /// as `frontend/parity-test.mjs`, so the browser's rotate, recover and cancel sign exactly what the
    /// node's rule verifies.
    #[test]
    fn key_series_parity_vectors_are_pinned() {
        use noesis_core::wallet;
        let (hot, cold) = ([7u8; 32], [8u8; 32]);
        let id = wallet::identity(&wallet::key_root(&hot), &wallet::cold_commit(&wallet::key_root(&cold)));
        let k1 = wallet::derive_key(&hot, 1);
        assert_eq!(hex_encode(&k1), "50daba6867e74718e73d9f282f3980df8d7f2f04e5849a655aed67dcdf2dd63b", "key 1 of seed [7;32]");
        let root1 = wallet::key_root(&k1);
        assert_eq!(hex_encode(&root1), "5e3e12e768494c491e23c7838a17315520f1e53b327ef9b43cbe5b8e7fa42dca", "key 1's root");
        let rot = wallet::key_op_digest(&id, wallet::OP_ROTATE, 0x7e57, 0, &root1, 300);
        assert_eq!(hex_encode(&rot), "4fdf5ffb1dc0f2b9becf130282a16e9e0b43af03ba3f4c225121f8f6d46ae597", "rotate digest");
        assert_eq!(
            hex_encode(&wallet::key_op_digest(&id, wallet::OP_CANCEL, 0x7e57, 5, &[0u8; 32], 65535)),
            "71c7833770367d9344de35ee4211b8c355f25246147d7477887e909abdb232ad",
            "cancel digest, generation 5, the last index"
        );
        let sig = wallet::sign(&hot, 300, &rot);
        assert!(wallet::verify(&wallet::key_root(&hot), 300, &rot, &sig));
        let mut h = blake2b_ref::Blake2bBuilder::new(64).build();
        for t in [&sig.bottom, &sig.top] {
            h.update(&t.ots_root);
            for a in &t.auth {
                h.update(a);
            }
            h.update(&t.ots_sig);
        }
        let mut out = [0u8; 64];
        h.finalize(&mut out);
        assert_eq!(
            hex_encode(&out),
            "6142885ad721bec2d17c0a43066851591a28d0c1fdb62b5162c1aadf11ec8144d08ec126e8372b32c0ff680df007e6bb5567c074f4f1071d15b6176db64960f9",
            "blake2b-512 of the rotation signature at 300"
        );
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
    fn the_page_and_both_wallet_scripts_are_served_and_linked() {
        let mut st = ServerState::new();
        let (_, _, page) = handle_request(&mut st, "GET", "/", b"");
        let page = String::from_utf8(page).expect("frontend is UTF-8");
        for (path, marker) in [("/crypto.js", "NoesisCrypto"), ("/wallet.js", "NoesisWallet")] {
            let (status, ctype, body) = handle_request(&mut st, "GET", path, b"");
            assert_eq!((status, ctype), (200, "application/javascript; charset=utf-8"), "{path} is served");
            assert!(String::from_utf8(body).expect("UTF-8").contains(marker), "{path} is the right file");
            assert!(page.contains(&format!("<script src=\"{path}\"></script>")), "the page loads {path}");
        }
        // the wallet keeps the seed sealed, never in the old plain-text record
        assert!(WALLET_JS.contains("noesis_wallet_v2_sealed") && WALLET_JS.contains("PBKDF2"));
    }

    #[test]
    fn get_questions_serves_the_corpus_question_feed() {
        let mut st = ServerState::new();
        let (status, ctype, body) = handle_request(&mut st, "GET", "/questions.json", b"");
        assert_eq!(status, 200);
        assert_eq!(ctype, "application/json; charset=utf-8");
        let v: serde_json::Value = serde_json::from_slice(&body).expect("questions.json is valid JSON");
        assert!(v["questions"].as_array().map(|a| !a.is_empty()).unwrap_or(false), "serves a non-empty question set");
        assert!(v["questions"][0]["q"].is_string(), "each question carries a `q` string");
    }

    #[test]
    fn index_route_reports_the_highest_used_one_time_key() {
        let mut st = ServerState::new();
        let key = [0x33u8; 32];
        let addr = id_hex(&key);
        let (s, _, b) = handle_request(&mut st, "GET", &format!("/index/{addr}"), b"");
        assert_eq!(s, 200);
        assert!(state_of(&b)["highest_used"].is_null(), "a fresh address has used no key");
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&key, 0, "the first new idea, about harbor fog")).0, 200);
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&key, 1, "a second, unrelated idea about kiln glazes")).0, 200);
        let (_, _, b) = handle_request(&mut st, "GET", &format!("/index/{addr}"), b"");
        assert_eq!(state_of(&b)["highest_used"], 1, "resume at 2");
        assert_eq!(handle_request(&mut st, "GET", "/index/not-hex", b"").0, 400);
    }

    #[test]
    fn blocks_route_lists_finalized_blocks_newest_first_and_survives_a_restart() {
        let path = std::env::temp_dir().join(format!("noesis-blocks-route-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let (mut st, _) = ServerState::with_store(&path, ChainSpec::dev()).unwrap();
        let first = signed_body(&[0x11u8; 32], 0, "a genuinely new idea about winter light");
        let second = signed_body(&[0x22u8; 32], 0, "an entirely separate subject, cold rivers");
        assert_eq!(handle_request(&mut st, "POST", "/submit", &first).0, 200);
        assert_eq!(handle_request(&mut st, "POST", "/submit", &second).0, 200);
        let (status, _ctype, body) = handle_request(&mut st, "GET", "/blocks", b"");
        assert_eq!(status, 200);
        let v = state_of(&body);
        let blocks = v["blocks"].as_array().expect("a blocks array");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["height"], 2, "newest first");
        assert_eq!(blocks[1]["height"], 1);
        assert_eq!(blocks[0]["hash"].as_str().unwrap().len(), 64, "a 32-byte header digest in hex");
        assert_eq!(blocks[0]["cells"].as_array().unwrap().len(), 1, "one contribution per block");
        // a restart rebuilds the same view from the chain log
        let (mut again, replayed) = ServerState::with_store(&path, ChainSpec::dev()).unwrap();
        assert_eq!(replayed, 2);
        let (_, _, body2) = handle_request(&mut again, "GET", "/blocks", b"");
        assert_eq!(state_of(&body2)["blocks"], v["blocks"]);
        let _ = std::fs::remove_file(&path);
    }

    /// Package C, the question that had to be answered before turning binding on: a wallet-v2
    /// contribution still lands through `POST /submit` on the real testnet spec (its difficulty, its
    /// constitution) with control binding ON, because a contribution is not a spend (it is checked by
    /// the wallet signature, and the coinbase is constructed, not spent). And the block reward pays the
    /// faucet's key for that height, never the placeholder validator key.
    #[test]
    fn a_v2_submit_lands_on_the_testnet_with_control_binding_on_and_pays_the_faucet() {
        use crate::faucet::{reward_key, DEV_FAUCET_SEED};
        let mut st = ServerState::with_spec(ChainSpec::testnet()).with_faucet(DEV_FAUCET_SEED).expect("binding is on");
        let before = state_of(&handle_request(&mut st, "GET", "/state", b"").2);
        assert_eq!(before["chain_id"], 0x7e57);
        assert_eq!(before["control_binding"], true, "the testnet spec turns control binding on");
        let dave = [0x11u8; 32];
        let (s, _, b) = handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 0, "a first note on harbor fog"));
        assert_eq!(s, 200, "{}", String::from_utf8_lossy(&b));
        let v = state_of(&b);
        assert_eq!(v["finalized"], true);
        assert_eq!(v["block_height"], 1);
        assert!(v["jul_issued"].as_u64().unwrap() > 0, "mined work issued JUL");
        // genesis_unscored: the chain's first contribution lands and earns no standing
        assert!(v["contributors"].get(id_hex(&dave)).is_none_or(|x| *x == json!(0)), "{}", v["contributors"]);
        let blocks = state_of(&handle_request(&mut st, "GET", "/blocks", b"").2);
        let paid = blocks["blocks"][0]["coinbase"].clone();
        assert_ne!(paid, hex_encode(b"alice"), "the reward left the placeholder");
        assert_eq!(paid, hex_encode(&noesis_core::lamport::keygen_root(&reward_key(&DEV_FAUCET_SEED, 1))), "the faucet's height-1 key");
        // the second contributor's v2 submit lands too, and earns standing as usual
        let erin = [0x22u8; 32];
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&erin, 0, "an entirely separate subject, cold rivers")).0, 200);
        let after = state_of(&handle_request(&mut st, "GET", "/state", b"").2);
        assert_eq!(after["height"], 2);
        assert!(after["contributors"][id_hex(&erin)].as_u64().unwrap() > 0, "erin's identity earned standing");
    }

    /// A chain that pays its reward to the faucet never falls back to the placeholder: without the
    /// faucet seed the node produces nothing. And no faucet runs where binding is off.
    #[test]
    fn a_faucet_chain_without_the_seed_produces_nothing_and_dev_refuses_a_faucet() {
        let mut st = ServerState::with_spec(ChainSpec::testnet());
        let (s, _, b) = handle_request(&mut st, "POST", "/submit", &signed_body(&[0x11u8; 32], 0, "a first note on harbor fog"));
        assert_eq!(s, 400);
        assert!(state_of(&b)["error"].as_str().unwrap().contains("no faucet seed"));
        assert_eq!(state_of(&handle_request(&mut st, "GET", "/state", b"").2)["height"], 0);
        assert!(st.node.mempool.is_empty() && st.next_id == 1, "nothing was half-submitted");
        let err = ServerState::new().with_faucet(crate::faucet::DEV_FAUCET_SEED).err().expect("dev has binding off");
        assert!(err.contains("control binding off"));
        let (s, _, _) = handle_request(&mut ServerState::new(), "POST", "/faucet", json!({ "address": "ab".repeat(32) }).to_string().as_bytes());
        assert_eq!(s, 503, "the dev node runs no faucet");
        assert_eq!(state_of(&handle_request(&mut ServerState::new(), "GET", "/faucet", b"").2)["enabled"], false);
    }

    /// The faucet route end to end on the testnet constitution (at dev's easy bits so the test is quick):
    /// empty until a block pays it, a payout queued by
    /// address and carried by the next contribution block, the per-address limit, and no change to
    /// anyone's standing.
    #[test]
    fn the_faucet_route_pays_a_newcomer_with_the_next_block_and_moves_no_standing() {
        let fast = fast_testnet();
        let mut st = ServerState::with_spec(fast.clone()).with_faucet(crate::faucet::DEV_FAUCET_SEED).unwrap();
        st.clock = || 1_000;
        let newcomer = [0x7Au8; 32];
        let ask = |st: &mut ServerState, a: &[u8; 32]| {
            let (s, _, b) = handle_request(st, "POST", "/faucet", json!({ "address": hex_encode(a) }).to_string().as_bytes());
            (s, state_of(&b))
        };
        let jul = |st: &mut ServerState, a: &[u8; 32]| state_of(&handle_request(st, "GET", &format!("/balance/{}", hex_encode(a)), b"").2)["jul"].clone();
        let f = state_of(&handle_request(&mut st, "GET", "/faucet", b"").2);
        assert_eq!((f["enabled"].clone(), f["balance_jul"].clone(), f["drip_jul"].clone()), (json!(true), json!(0), json!(10)));
        assert_eq!(ask(&mut st, &newcomer).0, 503, "nothing to pay with before the first block");
        // malformed requests
        assert_eq!(handle_request(&mut st, "POST", "/faucet", b"{").0, 400);
        assert_eq!(handle_request(&mut st, "POST", "/faucet", br#"{"address":"abc"}"#).0, 400);
        assert_eq!(handle_request(&mut st, "GET", "/balance/xyz", b"").0, 400);

        let dave = [0x11u8; 32];
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 0, "a first note on harbor fog")).0, 200);
        assert!(state_of(&handle_request(&mut st, "GET", "/faucet", b"").2)["balance_jul"].as_u64().unwrap() > 10);
        let (s, q) = ask(&mut st, &newcomer);
        assert_eq!(s, 200, "{q}");
        assert_eq!(q["amount_jul"], 10);
        let digest = q["digest"].as_str().unwrap().to_string();
        assert_eq!(ask(&mut st, &newcomer).1["digest"], digest.as_str(), "asking again while it waits is a no-op");
        assert_eq!(jul(&mut st, &newcomer), 0, "nothing lands until a contribution makes a block");

        let erin = [0x22u8; 32];
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&erin, 0, "an entirely separate subject, cold rivers")).0, 200);
        assert_eq!(jul(&mut st, &newcomer), 10, "the payout landed with block 2");
        let t = state_of(&handle_request(&mut st, "GET", "/tokens", b"").2);
        assert!(t["included"].as_array().unwrap().iter().any(|i| i["digest"] == digest.as_str() && i["height"] == 2));
        assert!(state_of(&handle_request(&mut st, "GET", "/faucet", b"").2)["pending"].is_null());
        let (s, r) = ask(&mut st, &newcomer);
        assert_eq!(s, 429, "one payout per address per day: {r}");

        // standing: exactly as on a twin chain where nobody asked the faucet
        let mut twin = ServerState::with_spec(fast).with_faucet(crate::faucet::DEV_FAUCET_SEED).unwrap();
        handle_request(&mut twin, "POST", "/submit", &signed_body(&dave, 0, "a first note on harbor fog"));
        handle_request(&mut twin, "POST", "/submit", &signed_body(&erin, 0, "an entirely separate subject, cold rivers"));
        let (a, b) = (state_of(&handle_request(&mut st, "GET", "/state", b"").2), state_of(&handle_request(&mut twin, "GET", "/state", b"").2));
        assert_eq!(a["contributors"], b["contributors"], "a faucet payout moved standing");
        assert_eq!(a["jul_issued"], b["jul_issued"], "a payout mints nothing");
    }

    #[test]
    fn signed_submit_finalizes_a_block_and_credits_the_address() {
        let mut st = ServerState::new();
        let dave = [0x11u8; 32];
        let body = signed_body(&dave, 0, "a genuinely new idea about winter light");
        let (status, _ctype, resp) = handle_request(&mut st, "POST", "/submit", &body);
        assert_eq!(status, 200);
        let v = state_of(&resp);
        assert_eq!(v["finalized"], true);
        assert_eq!(v["block_height"], 1);
        assert_eq!(v["height"], 1);
        assert!(v["jul_issued"].as_u64().unwrap() > 0, "mined work issued JUL");
        assert!(v["contributors"].get(id_hex(&dave)).is_some(), "dave's identity earned standing");
        // a second contributor (different key) advances the chain to height 2
        let erin = [0x22u8; 32];
        let body2 = signed_body(&erin, 0, "an entirely separate subject, cold rivers");
        let (_s, _c, resp2) = handle_request(&mut st, "POST", "/submit", &body2);
        assert_eq!(state_of(&resp2)["height"], 2);
    }

    #[test]
    fn rejects_reused_leaf_index_and_forged_identity() {
        let mut st = ServerState::new();
        let dave = [0x11u8; 32];
        // valid submit at leaf 0
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 0, "first novel thought")).0, 200);
        // reusing leaf 0 (even with different data) is a one-time-signature replay ⇒ rejected
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 0, "a different second thought"));
        assert_eq!(s, 400, "reused leaf index rejected");
        assert!(state_of(&r)["error"].as_str().unwrap().contains("already used"));
        // advancing to leaf 1 works
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 1, "a properly novel thought about tides")).0, 200);
        // a body whose identity is swapped (claim someone else's identity) is rejected
        let mut forged: Value = serde_json::from_slice(&signed_body(&dave, 2, "identity theft attempt")).unwrap();
        forged["identity"] = json!(hex_encode(&[0xEEu8; 32]));
        let (sf, _, rf) = handle_request(&mut st, "POST", "/submit", forged.to_string().as_bytes());
        assert_eq!(sf, 400, "a signature not matching the claimed identity is rejected");
        assert!(state_of(&rf)["error"].as_str().unwrap().contains("identity is not the hash"));
    }

    #[test]
    fn the_identity_must_hash_from_its_hot_root_and_cold_commitment() {
        let mut st = ServerState::new();
        let erin = [0x22u8; 32];
        let genuine: Value = serde_json::from_slice(&signed_body(&erin, 0, "a sketch of a tide-driven water clock")).unwrap();
        // a relay swaps the cold commitment (to plant its own recovery key): the identity no longer hashes
        let mut cold = genuine.clone();
        cold["cold_commit"] = json!(hex_encode(&noesis_core::wallet::cold_commit(&[0xBAu8; 32])));
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", cold.to_string().as_bytes());
        assert_eq!(s, 400);
        assert!(state_of(&r)["error"].as_str().unwrap().contains("identity is not the hash"));
        // ... and re-derives the identity to match: the signature covers the identity, so it fails
        let mut both = cold.clone();
        let (hot_root, cold_commit) = (hex32(&both, "hot_root").unwrap(), hex32(&both, "cold_commit").unwrap());
        both["identity"] = json!(hex_encode(&noesis_core::wallet::identity(&hot_root, &cold_commit)));
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", both.to_string().as_bytes());
        assert_eq!(s, 400);
        assert!(state_of(&r)["error"].as_str().unwrap().contains("signature does not verify"));
        // another hot root (with the identity re-derived): the signature is not under that key
        let mut hot = genuine.clone();
        let other_root = noesis_core::wallet::key_root(&[0x23u8; 32]);
        let cc = hex32(&hot, "cold_commit").unwrap();
        hot["hot_root"] = json!(hex_encode(&other_root));
        hot["identity"] = json!(hex_encode(&noesis_core::wallet::identity(&other_root, &cc)));
        assert_eq!(handle_request(&mut st, "POST", "/submit", hot.to_string().as_bytes()).0, 400);
        // nothing was spent: the genuine body still lands, at the same index
        assert_eq!(handle_request(&mut st, "POST", "/submit", genuine.to_string().as_bytes()).0, 200);
        assert!(state_of(&handle_request(&mut st, "GET", "/state", b"").2)["contributors"].get(id_hex(&erin)).is_some());
    }

    #[test]
    fn the_parent_is_signed_so_a_relay_cannot_rewire_it() {
        let mut st = ServerState::new();
        let (alice, bob, carol) = ([0x33u8; 32], [0x44u8; 32], [0x55u8; 32]);
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&alice, 0, "a first idea about salt marsh carbon")).0, 200);
        // bob signed a root; a relay adds a parent (to hand cell 1 downstream credit): rejected
        let bob_root: Value = serde_json::from_slice(&signed_body(&bob, 0, "an unrelated note on bell founding")).unwrap();
        let mut added = bob_root.clone();
        added["parent"] = json!(1);
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", added.to_string().as_bytes());
        assert_eq!(s, 400, "an added parent is rejected");
        assert!(state_of(&r)["error"].as_str().unwrap().contains("signature does not verify"));
        assert_eq!(handle_request(&mut st, "POST", "/submit", bob_root.to_string().as_bytes()).0, 200);
        // carol signed "builds on 1"; a relay re-points it to 2, or strips it: both rejected
        let carol_on_1: Value = serde_json::from_slice(&signed_body_on(&carol, 0, Some(1), "marsh carbon, measured by season")).unwrap();
        let mut repointed = carol_on_1.clone();
        repointed["parent"] = json!(2);
        assert_eq!(handle_request(&mut st, "POST", "/submit", repointed.to_string().as_bytes()).0, 400, "re-pointed");
        let mut stripped = carol_on_1.clone();
        stripped.as_object_mut().unwrap().remove("parent");
        assert_eq!(handle_request(&mut st, "POST", "/submit", stripped.to_string().as_bytes()).0, 400, "stripped");
        // the genuine link lands, and the chain shows the parent carol signed
        assert_eq!(handle_request(&mut st, "POST", "/submit", carol_on_1.to_string().as_bytes()).0, 200);
        let v = state_of(&handle_request(&mut st, "GET", "/contributions", b"").2);
        let items = v["contributions"].as_array().unwrap();
        assert_eq!(items.iter().find(|c| c["id"] == 3).unwrap()["parent"], 1);
        assert_eq!(items.iter().find(|c| c["id"] == 2).unwrap()["parent"], Value::Null);
    }

    #[test]
    fn an_mt_key_spans_subtrees_and_its_index_only_goes_up() {
        let mut st = ServerState::new();
        let dave = [0x11u8; 32];
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 0, "a note from the first subtree")).0, 200);
        // index 300 = subtree 1, leaf 44: past the 256 leaves a single tree had; skipping is allowed
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 300, "a later note, about kelp forests")).0, 200);
        // a validly signed lower index (256, the first leaf of subtree 1) is refused: indexes only go up
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 256, "an out-of-order note on dune grass"));
        assert_eq!(s, 400);
        assert!(state_of(&r)["error"].as_str().unwrap().contains("already used"));
        let (_, _, b) = handle_request(&mut st, "GET", &format!("/index/{}", id_hex(&dave)), b"");
        assert_eq!(state_of(&b)["highest_used"], 300);
    }

    #[test]
    fn v1_bodies_out_of_range_indexes_and_bad_parents_are_refused_before_any_signature_check() {
        let mut st = ServerState::new();
        let h = "ab".repeat(32);
        let v1 = json!({ "address": h, "index": 0, "ots_root": h, "ots_sig": "", "auth": [], "data": "x" });
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", v1.to_string().as_bytes());
        assert_eq!(s, 400);
        assert!(state_of(&r)["error"].as_str().unwrap().contains("format v1 is retired"));
        let base = json!({ "identity": h, "hot_root": h, "cold_commit": h, "index": 0, "data": "x" });
        let mut far = base.clone();
        far["index"] = json!(noesis_core::wallet::MAX_SIGS);
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", far.to_string().as_bytes());
        assert_eq!(s, 400);
        assert!(state_of(&r)["error"].as_str().unwrap().contains("'index' must be"));
        let mut bad_parent = base.clone();
        bad_parent["parent"] = json!("one");
        let (s, _, r) = handle_request(&mut st, "POST", "/submit", bad_parent.to_string().as_bytes());
        assert_eq!(s, 400);
        assert!(state_of(&r)["error"].as_str().unwrap().contains("'parent' must be"));
        assert_eq!(state_of(&handle_request(&mut st, "GET", "/state", b"").2)["height"], 0);
    }

    #[test]
    fn malformed_and_unknown_requests_are_rejected_cleanly() {
        let mut st = ServerState::new();
        // bad JSON body
        let (s1, _, _) = handle_request(&mut st, "POST", "/submit", b"not json");
        assert_eq!(s1, 400);
        // missing signature fields (no address) ⇒ rejected before any state change
        let (s2, _, _) = handle_request(&mut st, "POST", "/submit", br#"{"data":"x"}"#);
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

    #[test]
    fn contributions_expose_provenance_and_downstream_events() {
        let mut st = ServerState::new();
        // A: a foundational contribution (no parent). First submit ⇒ cell id 1.
        let alice = [0x33u8; 32];
        assert_eq!(
            handle_request(&mut st, "POST", "/submit", &signed_body(&alice, 0, "a foundational idea about tidal energy capture at estuaries")).0,
            200
        );
        // B builds ON A via the live API (parent = 1) ⇒ cell id 2.
        let bob = [0x44u8; 32];
        let b = signed_body_on(&bob, 0, Some(1), "wave-lensing lattices and lunar-phase timing, a separate downstream method");
        assert_eq!(handle_request(&mut st, "POST", "/submit", &b).0, 200);

        let (status, _ct, body) = handle_request(&mut st, "GET", "/contributions", b"");
        assert_eq!(status, 200);
        let v = state_of(&body);
        let items = v["contributions"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        // B names A as its provenance — the link formed through the live API (was impossible before).
        let b_item = items.iter().find(|c| c["id"] == 2).unwrap();
        assert_eq!(b_item["parent"], 1);
        // A now has one downstream event: B built on it — a contribution connected to an "actual event".
        let a_item = items.iter().find(|c| c["id"] == 1).unwrap();
        assert_eq!(a_item["downstream"], 1);
    }

    #[test]
    fn a_dangling_parent_is_rejected() {
        let mut st = ServerState::new();
        let carol = [0x55u8; 32];
        let body = signed_body_on(&carol, 0, Some(999), "an idea claiming to build on a contribution that does not exist");
        let (status, _ct, resp) = handle_request(&mut st, "POST", "/submit", &body);
        assert_eq!(status, 400);
        assert!(state_of(&resp)["error"].as_str().unwrap().contains("names no finalized contribution"));
    }

    // ---- B5: GET /tokens ----

    #[test]
    fn get_tokens_lists_soulbound_cells_and_keeps_jul_out() {
        use crate::tokens::soulbound::{encode_identity, Identity, Tier, CODE_HASH};
        let mut st = ServerState::new();
        let ident = Cell {
            id: 77,
            lock: Script { code_hash: [0u8; 32], args: vec![0x5A; 32] },
            type_script: Script { code_hash: CODE_HASH, args: b"test-identity-v1".to_vec() },
            parent: None,
            timestamp: 0,
            data: encode_identity(&Identity { serial: 4, tier: Tier::Member, holder: [0xAB; 32] }),
        };
        st.node.ledger.token_cells.push(ident.clone());
        // a block adds a JUL coinbase cell: counted, not listed
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&[0x11; 32], 0, "a first note on river deltas")).0, 200);
        let (status, _, body) = handle_request(&mut st, "GET", "/tokens", b"");
        assert_eq!(status, 200);
        let t = state_of(&body);
        assert_eq!(t["height"], 1);
        assert_eq!(t["jul_cells"], 1);
        let cells: Vec<&Value> =
            t["cells"].as_array().unwrap().iter().filter(|c| c["standard"] == "soulbound").collect();
        let c = cells.iter().find(|c| c["id"] == 77).expect("the identity is listed");
        assert_eq!(c["collection"], "test-identity-v1");
        assert_eq!(c["record"]["kind"], "identity");
        assert_eq!(c["record"]["serial"], 4);
        assert_eq!(c["record"]["tier"], "member");
        assert_eq!(c["record"]["holder"], "ab".repeat(32));
        // the hex fields rebuild the exact cell (what a tx must name as its input)
        let back = token_cell_from_json(c).unwrap();
        assert!(back.id == ident.id && back.lock == ident.lock && back.type_script == ident.type_script && back.data == ident.data);
        let s = state_of(&handle_request(&mut st, "GET", "/state", b"").2);
        assert_eq!(s["coinbase_cells"], 1, "coinbase_cells counts JUL cells only");
    }

    // ---- B4: the genesis authority, POST /token-tx ----

    fn tokens(st: &mut ServerState) -> Value {
        let (status, _, body) = handle_request(st, "GET", "/tokens", b"");
        assert_eq!(status, 200);
        state_of(&body)
    }

    /// A signed mint of `records` from the live chain, with the public dev issuer seed.
    fn signed_mint(st: &mut ServerState, records: &[(u64, u8, &str)]) -> Value {
        use crate::identity_mint as im;
        let recs: Vec<im::BatchRecord> = records
            .iter()
            .map(|&(serial, holder, tier)| im::BatchRecord {
                serial,
                holder: [holder; 32],
                tier: crate::tokens::soulbound::Tier::from_name(tier).unwrap(),
            })
            .collect();
        let live = im::parse_live(&tokens(st), im::COLLECTION).expect("live authority");
        let mut built = im::build(&recs, &live, &im::DEV_ISSUER_SEED, im::COLLECTION).expect("builds");
        built.sign();
        im::envelope(built.tx.as_ref().expect("a tx"))
    }

    #[test]
    fn get_tokens_lists_the_genesis_authority_and_keeps_jul_out() {
        let mut st = ServerState::new();
        let t = tokens(&mut st);
        assert_eq!(t["height"], 0);
        assert_eq!(t["jul_cells"], 0);
        let cells = t["cells"].as_array().unwrap();
        assert_eq!(cells.len(), 1, "genesis holds the identity issuer's authority, nothing else");
        assert_eq!(cells[0]["standard"], "soulbound");
        assert_eq!(cells[0]["collection"], "vibeswap-identity-v1");
        assert_eq!(cells[0]["record"]["kind"], "authority");
        assert_eq!(cells[0]["record"]["generation"], 0);
        // the hex fields rebuild the exact cell (what a tx must name as its input)
        let back = token_cell_from_json(&cells[0]).unwrap();
        let live = &st.node.ledger.token_cells[0];
        assert!(back.id == live.id && back.lock == live.lock && back.type_script == live.type_script && back.data == live.data);
        // a block adds a JUL coinbase cell: counted in jul_cells, not listed
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&[0x11; 32], 0, "a first note on river deltas")).0, 200);
        let t = tokens(&mut st);
        assert_eq!(t["jul_cells"], 1);
        assert_eq!(t["cells"].as_array().unwrap().len(), 1);
        let s = state_of(&handle_request(&mut st, "GET", "/state", b"").2);
        assert_eq!(s["coinbase_cells"], 1, "coinbase_cells counts JUL cells only");
        assert_eq!(s["token_cells"], 2);
    }

    #[test]
    fn token_tx_rides_the_next_contribution_block_and_earns_no_standing() {
        let mut st = ServerState::new();
        let env = signed_mint(&mut st, &[(1, 0xA1, "identity"), (2, 0xA2, "member")]);
        let (status, _, body) = handle_request(&mut st, "POST", "/token-tx", env.to_string().as_bytes());
        assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));
        let digest = state_of(&body)["digest"].as_str().unwrap().to_string();
        assert_eq!(tokens(&mut st)["pending"][0], digest.as_str());
        // re-sending the same tx is a no-op
        assert_eq!(handle_request(&mut st, "POST", "/token-tx", env.to_string().as_bytes()).0, 200);
        assert_eq!(tokens(&mut st)["pending"].as_array().unwrap().len(), 1);
        // nothing lands until a contribution makes a block
        assert_eq!(state_of(&handle_request(&mut st, "GET", "/state", b"").2)["height"], 0);
        let dave = [0x11u8; 32];
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 0, "a note on estuary sediment")).0, 200);
        let t = tokens(&mut st);
        assert_eq!(t["pending"].as_array().unwrap().len(), 0);
        assert_eq!(t["included"][0]["digest"], digest.as_str());
        assert_eq!(t["included"][0]["height"], 1);
        let kinds: Vec<&str> = t["cells"].as_array().unwrap().iter().map(|c| c["record"]["kind"].as_str().unwrap()).collect();
        assert_eq!(kinds.iter().filter(|k| **k == "identity").count(), 2);
        assert_eq!(kinds.iter().filter(|k| **k == "authority").count(), 1);
        // standing: only the contributor, exactly as without the mint
        let s = state_of(&handle_request(&mut st, "GET", "/state", b"").2);
        let contributors = s["contributors"].as_object().unwrap();
        assert_eq!(contributors.len(), 1);
        assert!(contributors.contains_key(&id_hex(&dave)));
        let mut twin = ServerState::new();
        handle_request(&mut twin, "POST", "/submit", &signed_body(&dave, 0, "a note on estuary sediment"));
        assert_eq!(s["contributors"], state_of(&handle_request(&mut twin, "GET", "/state", b"").2)["contributors"]);
    }

    #[test]
    fn token_tx_route_refuses_unsigned_foreign_and_malformed_txs() {
        let mut st = ServerState::new();
        let env = signed_mint(&mut st, &[(1, 0xA1, "identity")]);
        // unsigned: the route enforces control binding now, ahead of consensus
        let mut unsigned = env.clone();
        unsigned["auths"] = json!([]);
        let (s1, _, b1) = handle_request(&mut st, "POST", "/token-tx", unsigned.to_string().as_bytes());
        assert_eq!(s1, 400);
        assert!(state_of(&b1)["error"].as_str().unwrap().contains("signature"));
        // a wrong signature
        let mut forged = env.clone();
        forged["auths"] = json!([hex_encode(&noesis_core::lamport::sign(&[9u8; 32], &[0u8; 32]))]);
        assert_eq!(handle_request(&mut st, "POST", "/token-tx", forged.to_string().as_bytes()).0, 400);
        // another standard: the route opens no other value movement
        let mut fungible = env.clone();
        fungible["tx"]["standard"] = json!(0);
        assert_eq!(handle_request(&mut st, "POST", "/token-tx", fungible.to_string().as_bytes()).0, 400);
        // garbage
        assert_eq!(handle_request(&mut st, "POST", "/token-tx", b"{").0, 400);
        assert_eq!(tokens(&mut st)["pending"].as_array().unwrap().len(), 0, "nothing was queued");
        // the genuine one still queues
        assert_eq!(handle_request(&mut st, "POST", "/token-tx", env.to_string().as_bytes()).0, 200);
    }

    // ---- testnet step 7: live sync, one producer and its replicas ----

    use crate::follow::{apply_page, next_request, FollowError};

    /// Distinct texts, so the ingress screen takes each as a new contribution.
    const TEXTS: [&str; 8] = [
        "a first note on harbor fog",
        "an entirely separate subject, cold rivers",
        "a sketch of a tide-driven water clock",
        "notes on kiln glazes and wood ash",
        "the migration of eels through the sargasso sea",
        "bell founding and the ratio of copper to tin",
        "salt marsh carbon, measured by season",
        "dune grass anchoring sand through winter storms",
    ];

    /// The testnet constitution at dev's easy bits, so tests mine in a few hundred hashes. Every seal must
    /// claim its chain's bits, so the bits are the spec's own, not only what `mine` is handed.
    fn fast_testnet() -> ChainSpec {
        let mut spec = ChainSpec::testnet();
        spec.constitution.pow_bits = ChainSpec::dev().genesis_bits();
        spec
    }

    fn tmp_log(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("noesis-follow-{tag}-{}-{nanos}.log", std::process::id()))
    }

    /// A durable producer on `spec` (with the public test faucet seed when the chain pays a faucet).
    fn producer_on(spec: ChainSpec, tag: &str) -> (ServerState, std::path::PathBuf) {
        let path = tmp_log(tag);
        let pays_faucet = spec.reward_to_faucet;
        let (st, _) = ServerState::with_store(&path, spec).unwrap();
        let st = if pays_faucet { st.with_faucet(crate::faucet::DEV_FAUCET_SEED).unwrap() } else { st };
        (st, path)
    }

    /// A durable replica on `spec`. It never holds a faucet seed.
    fn follower_on(spec: ChainSpec, tag: &str) -> (ServerState, std::path::PathBuf) {
        let path = tmp_log(tag);
        let (st, _) = ServerState::with_store(&path, spec).unwrap();
        (st.as_replica("http://producer.test".into()), path)
    }

    fn submit(st: &mut ServerState, n: usize) {
        let (s, _, b) = handle_request(st, "POST", "/submit", &signed_body(&[0x40 + n as u8; 32], 0, TEXTS[n]));
        assert_eq!(s, 200, "{}", String::from_utf8_lossy(&b));
    }

    /// Pull pages from `producer` until the follower is level, `limit` blocks a page. Returns pages read.
    fn sync(f: &mut ServerState, producer: &mut ServerState, limit: usize) -> Result<usize, FollowError> {
        let mut pages = 0;
        loop {
            let (s, _, body) = handle_request(producer, "GET", &format!("{}&limit={limit}", next_request(f.height())), b"");
            assert_eq!(s, 200);
            let (_, more) = apply_page(f, &body)?;
            pages += 1;
            if !more {
                return Ok(pages);
            }
        }
    }

    fn stopped(r: Result<(usize, bool), FollowError>) -> String {
        match r {
            Err(FollowError::Stop(e)) => e,
            other => panic!("expected following to stop, got {other:?}"),
        }
    }

    fn page(st: &mut ServerState, since: u64) -> Value {
        state_of(&handle_request(st, "GET", &next_request(since), b"").2)
    }

    #[test]
    fn blocks_since_pages_full_blocks_and_a_follower_catches_up_across_pages() {
        let (mut p, pp) = producer_on(ChainSpec::dev(), "page-p");
        for n in 0..5 {
            submit(&mut p, n);
        }
        let v = state_of(&handle_request(&mut p, "GET", "/blocks?since=0&limit=2", b"").2);
        assert_eq!((v["chain_id"].clone(), v["tip"].clone(), v["since"].clone()), (json!(0xde0), json!(5), json!(0)));
        assert_eq!((v["next"].clone(), v["more"].clone()), (json!(2), json!(true)));
        assert_eq!(v["anchor"], hex_encode(&[0u8; 32]), "the genesis link");
        let blocks = v["blocks"].as_array().unwrap();
        assert_eq!(blocks.iter().map(|b| b["height"].clone()).collect::<Vec<_>>(), vec![json!(1), json!(2)], "full blocks, oldest first");
        assert!(blocks[0]["pow"]["nonce"].is_u64() && blocks[0]["cells"][0]["data"].is_array(), "the whole block, not a summary");
        // the byte budget cuts a page too, but never below one block
        let (s, body) = p.blocks_page(0, 32, 1);
        assert_eq!(s, 200);
        let one = state_of(&body);
        assert_eq!((one["blocks"].as_array().unwrap().len(), one["more"].clone()), (1, json!(true)));
        // a follower that is 5 behind catches up 2 blocks a page
        let (mut f, fp) = follower_on(ChainSpec::dev(), "page-f");
        assert_eq!(sync(&mut f, &mut p, 2), Ok(3));
        assert_eq!(f.height(), 5);
        assert_eq!(f.state_digest(), p.state_digest(), "the replica derives the producer's state from the blocks");
        assert_eq!(f.link(5), p.link(5));
        let (a, b) = (state_of(&handle_request(&mut p, "GET", "/state", b"").2), state_of(&handle_request(&mut f, "GET", "/state", b"").2));
        for k in ["height", "work", "jul_issued", "cells", "contributors"] {
            assert_eq!(a[k], b[k], "{k}");
        }
        assert_eq!(state_of(&handle_request(&mut f, "GET", "/blocks", b"").2), state_of(&handle_request(&mut p, "GET", "/blocks", b"").2));
        let addr = id_hex(&[0x40u8; 32]);
        assert_eq!(state_of(&handle_request(&mut f, "GET", &format!("/index/{addr}"), b"").2)["highest_used"], 0);
        // level: an empty page, the same anchor, nothing to do
        assert_eq!(apply_page(&mut f, &handle_request(&mut p, "GET", &next_request(5), b"").2), Ok((0, false)));
        // a replica serves its own log the same way, so a follower can follow it
        assert_eq!(page(&mut f, 3)["blocks"], page(&mut p, 3)["blocks"]);
        assert_eq!(handle_request(&mut p, "GET", "/blocks?since=x", b"").0, 400);
        assert_eq!(handle_request(&mut ServerState::new(), "GET", "/blocks?since=0", b"").0, 503, "no store, no log to serve");
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    /// The follower re-validates every block with the producer's own path: a tampered block is refused,
    /// following stops, and the follower keeps its chain (memory and disk).
    #[test]
    fn a_follower_refuses_a_tampered_block_and_keeps_its_chain() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "tamper-p");
        for n in 0..3 {
            submit(&mut p, n);
        }
        let (mut f, fp) = follower_on(spec.clone(), "tamper-f");
        apply_page(&mut f, &handle_request(&mut p, "GET", "/blocks?since=0&limit=1", b"").2).unwrap();
        assert_eq!(f.height(), 1);
        let before = f.state_digest();
        let genuine = page(&mut p, 1);
        let mut other_bits = genuine.clone(); // a seal claiming bits other than the chain's
        other_bits["blocks"][0]["pow"]["bits"] = json!(0x0300_ffffu32);
        // the chain's bits, and a nonce whose header digest misses the target
        let mut unmet_block = crate::wire::decode_block(&serde_json::to_vec(&genuine["blocks"][0]).unwrap()).unwrap();
        let target = noesis_core::pow::compact_to_target(spec.genesis_bits()).unwrap();
        while crate::runtime::header_digest(&unmet_block) <= target {
            unmet_block.pow.as_mut().unwrap().nonce += 1;
        }
        let mut unmet = genuine.clone();
        unmet["blocks"][0] = serde_json::from_slice(&crate::wire::encode_block(&unmet_block)).unwrap();
        let mut bare = genuine.clone(); // no seal at all
        bare["blocks"][0]["pow"] = Value::Null;
        let mut skip = genuine.clone(); // block 3 offered as the next block
        skip["blocks"] = json!([genuine["blocks"][1].clone()]);
        for (tampered, why) in
            [(other_bits, "PowBitsWrong"), (unmet, "PowUnmet"), (bare, "PowMissing"), (skip, "HeightMismatch")]
        {
            let e = stopped(apply_page(&mut f, tampered.to_string().as_bytes()));
            assert!(e.contains(why) && e.contains("keeps its chain at height 1"), "{e}");
            assert_eq!((f.height(), f.state_digest()), (1, before.clone()), "nothing applied");
        }
        let (again, replayed) = ServerState::with_store(&fp, spec).unwrap();
        assert_eq!((replayed, again.height()), (1, 1), "nothing reached the follower's store either");
        assert_eq!(apply_page(&mut f, genuine.to_string().as_bytes()), Ok((2, false)), "the genuine page applies");
        assert_eq!(f.state_digest(), p.state_digest());
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    /// On the testnet constitution an unsigned or forged spend is invalid, for the follower as for the
    /// producer: tampering with the faucet payout's signature is refused (control binding).
    #[test]
    fn a_follower_refuses_a_payout_whose_signature_was_tampered() {
        let spec = fast_testnet();
        let (mut p, pp) = producer_on(spec.clone(), "bind-p");
        p.clock = || 1_000;
        submit(&mut p, 0);
        let ask = json!({ "address": hex_encode(&[0x7Au8; 32]) }).to_string();
        assert_eq!(handle_request(&mut p, "POST", "/faucet", ask.as_bytes()).0, 200);
        submit(&mut p, 1);
        let (mut f, fp) = follower_on(spec, "bind-f");
        apply_page(&mut f, &handle_request(&mut p, "GET", "/blocks?since=0&limit=1", b"").2).unwrap();
        let genuine = page(&mut p, 1);
        assert_eq!(genuine["blocks"][0]["token_txs"].as_array().unwrap().len(), 1, "block 2 carries the payout");
        let mut forged = genuine.clone();
        let byte = genuine["blocks"][0]["token_txs"][0]["auths"][0][0].as_u64().unwrap();
        forged["blocks"][0]["token_txs"][0]["auths"][0][0] = json!(byte ^ 1);
        let mut unsigned = genuine.clone();
        unsigned["blocks"][0]["token_txs"][0]["auths"][0] = json!([]);
        for tampered in [forged, unsigned] {
            let e = stopped(apply_page(&mut f, tampered.to_string().as_bytes()));
            assert!(e.contains("TokenTxInvalidOrDoubleSpend"), "{e}");
            assert_eq!(f.height(), 1);
        }
        assert_eq!(apply_page(&mut f, genuine.to_string().as_bytes()), Ok((1, false)));
        assert_eq!(f.state_digest(), p.state_digest());
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    #[test]
    fn a_follower_resumes_from_its_own_store_after_a_restart() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "resume-p");
        submit(&mut p, 0);
        submit(&mut p, 1);
        let (mut f, fp) = follower_on(spec.clone(), "resume-f");
        sync(&mut f, &mut p, 32).unwrap();
        assert_eq!(f.height(), 2);
        drop(f); // the follower process ends
        submit(&mut p, 2);
        submit(&mut p, 3);
        let (f, replayed) = ServerState::with_store(&fp, spec).unwrap();
        let mut f = f.as_replica("http://producer.test".into());
        assert_eq!((replayed, f.height()), (2, 2), "it replays its own store, not the producer's");
        assert_eq!(f.link(2), p.link(2), "the chain link is rebuilt from its own store");
        assert_eq!(next_request(f.height()), "/blocks?since=2", "it asks only for what it lacks");
        assert_eq!(apply_page(&mut f, &handle_request(&mut p, "GET", &next_request(2), b"").2), Ok((2, false)));
        assert_eq!(f.state_digest(), p.state_digest());
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    /// A producer that rewrites history is not followed, even when the rewrite leaves the later blocks
    /// byte-identical (blocks do not name their parent's hash, so the tip's header digest alone would
    /// match): the chain link commits to every block from genesis.
    #[test]
    fn a_follower_stops_on_a_rewritten_history_even_one_that_keeps_the_later_blocks() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "rw-p");
        for n in 0..3 {
            submit(&mut p, n);
        }
        let (mut f, fp) = follower_on(spec.clone(), "rw-f");
        sync(&mut f, &mut p, 32).unwrap();
        let before = f.state_digest();
        // the rewrite: block 1's contribution text changed and re-mined; blocks 2 and 3 kept as they are
        let (_, blocks) = crate::store::load_blocks(&pp, &spec).unwrap();
        let mut b1 = blocks[0].clone();
        b1.cells[0].data = b"a different first contribution, about pine resin".to_vec();
        let b1 = crate::chainspec::mine(b1, spec.genesis_bits());
        let rw = tmp_log("rw-q");
        for b in [&b1, &blocks[1], &blocks[2]] {
            crate::store::append_block(&rw, b).unwrap();
        }
        let (mut q, replayed) = ServerState::with_store(&rw, spec.clone()).unwrap();
        assert_eq!(replayed, 3, "the rewritten chain is itself valid");
        assert_eq!(q.blocks.last().unwrap()["hash"], p.blocks.last().unwrap()["hash"], "same tip header digest");
        assert_ne!(q.link(3), p.link(3));
        let e = stopped(apply_page(&mut f, &handle_request(&mut q, "GET", &next_request(3), b"").2));
        assert!(e.contains("history differs") && e.contains("keeps its chain"), "{e}");
        assert_eq!(f.state_digest(), before);
        // the true producer still serves a continuation the follower takes
        submit(&mut p, 3);
        assert_eq!(apply_page(&mut f, &handle_request(&mut p, "GET", &next_request(3), b"").2), Ok((1, false)));
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp), std::fs::remove_file(&rw));
    }

    #[test]
    fn a_follower_stops_on_a_producer_behind_it_and_on_another_network() {
        let (mut p, pp) = producer_on(ChainSpec::dev(), "behind-p");
        submit(&mut p, 0);
        submit(&mut p, 1);
        let (mut f, fp) = follower_on(ChainSpec::dev(), "behind-f");
        sync(&mut f, &mut p, 32).unwrap();
        // a producer that lost its log and started over (same chain id, height 1)
        let (mut reset, rp) = producer_on(ChainSpec::dev(), "behind-reset");
        submit(&mut reset, 5);
        let v = page(&mut reset, 2);
        assert_eq!((v["tip"].clone(), v["anchor"].clone(), v["blocks"].clone()), (json!(1), Value::Null, json!([])));
        let e = stopped(apply_page(&mut f, v.to_string().as_bytes()));
        assert!(e.contains("below this node's 2"), "{e}");
        // the same chain id but an empty reset log: still below
        let (mut empty, ep) = producer_on(ChainSpec::dev(), "behind-empty");
        assert!(stopped(apply_page(&mut f, page(&mut empty, 2).to_string().as_bytes())).contains("below"));
        // another network
        let (mut t, tp) = producer_on(fast_testnet(), "behind-net");
        let e = stopped(apply_page(&mut f, page(&mut t, 2).to_string().as_bytes()));
        assert!(e.contains("another network"), "{e}");
        assert_eq!(f.height(), 2);
        // transport trouble is retried, never a stop
        assert!(matches!(apply_page(&mut f, b"<html>502 bad gateway</html>"), Err(FollowError::Retry(_))));
        for path in [pp, fp, rp, ep, tp] {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Gap 1 (2026-11-18), follower path: a producer serves a block whose seal claims easier bits than
    /// the chain spec and is valid for them. The follower refuses it, stops, and keeps its chain.
    #[test]
    fn a_follower_refuses_a_block_mined_at_easier_bits_than_the_spec() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "bits-p");
        submit(&mut p, 0);
        let (mut f, fp) = follower_on(spec.clone(), "bits-f");
        let genuine = page(&mut p, 0);
        let block = crate::wire::decode_block(&serde_json::to_vec(&genuine["blocks"][0]).unwrap()).unwrap();
        let easier = 0x2100_ffffu32; // one exponent easier than dev's 0x2000ffff
        let cheap = crate::chainspec::mine(block, easier);
        assert!(crate::runtime::header_digest(&cheap) <= noesis_core::pow::compact_to_target(easier).unwrap());
        let mut tampered = genuine.clone();
        tampered["blocks"][0] = serde_json::from_slice(&crate::wire::encode_block(&cheap)).unwrap();
        let e = stopped(apply_page(&mut f, tampered.to_string().as_bytes()));
        assert!(e.contains("PowBitsWrong") && e.contains("0x2100ffff") && e.contains("keeps its chain at height 0"), "{e}");
        assert_eq!(f.height(), 0, "nothing applied");
        let (again, replayed) = ServerState::with_store(&fp, spec).unwrap();
        assert_eq!((replayed, again.height()), (0, 0), "nothing reached the follower's store either");
        assert_eq!(apply_page(&mut f, genuine.to_string().as_bytes()), Ok((1, false)), "the genuine page applies");
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    /// Gap 2 (2026-11-18), follower path: a page from another network whose `chain_id` field was
    /// rewritten to the follower's (the page-level check reads the producer's word). The blocks still
    /// name their own chain, so the follower refuses the first one and keeps its chain.
    #[test]
    fn a_follower_refuses_blocks_of_another_network_even_when_the_page_claims_its_chain_id() {
        let mut other = ChainSpec::dev(); // the same rules, genesis and bits: another network
        other.constitution.chain_id = 0xb0b;
        let (mut q, qp) = producer_on(other, "net-q");
        submit(&mut q, 0);
        let (mut f, fp) = follower_on(ChainSpec::dev(), "net-f");
        let mut v = page(&mut q, 0);
        v["chain_id"] = json!(0xde0);
        let e = stopped(apply_page(&mut f, v.to_string().as_bytes()));
        assert!(e.contains("WrongChain") && e.contains("0xb0b") && e.contains("keeps its chain at height 0"), "{e}");
        assert_eq!(f.height(), 0, "nothing applied");
        let _ = (std::fs::remove_file(&qp), std::fs::remove_file(&fp));
    }

    /// A faucet payout is signed while it waits, and the faucet re-signs it freely until the block that
    /// spends its cells is final, so those signatures must not leave the producer before that. The
    /// block log a follower pulls holds finalized blocks only.
    #[test]
    fn blocks_since_serves_only_finalized_blocks_and_never_a_pending_payout() {
        let (mut p, pp) = producer_on(fast_testnet(), "final-p");
        p.clock = || 1_000;
        submit(&mut p, 0);
        let ask = json!({ "address": hex_encode(&[0x7Au8; 32]) }).to_string();
        let q = state_of(&handle_request(&mut p, "POST", "/faucet", ask.as_bytes()).2);
        let pending = p.faucet.as_ref().unwrap().pending().cloned().expect("a signed payout waits");
        assert_eq!(q["digest"], hex_encode(&pending.digest()));
        assert!(pending.auths.iter().all(|a| !a.is_empty()), "the waiting payout is already signed");
        let sig = serde_json::to_string(&pending.auths[0]).unwrap();
        let sig = &sig[..sig.len().min(400)]; // the opening of its first signature, as a page would carry it
        for path in ["/blocks?since=0", "/blocks", "/faucet", "/tokens", "/state"] {
            let body = String::from_utf8(handle_request(&mut p, "GET", path, b"").2).unwrap();
            assert!(!body.contains(sig), "{path} leaked the pending payout's signature");
        }
        let v = page(&mut p, 0);
        assert_eq!((v["tip"].clone(), v["blocks"].as_array().unwrap().len()), (json!(1), 1), "only block 1, which is final");
        // a submit that is refused makes no block and serves nothing new
        assert_eq!(handle_request(&mut p, "POST", "/submit", &signed_body(&[0x55u8; 32], 0, TEXTS[0])).0, 400);
        assert_eq!(page(&mut p, 0)["tip"], 1);
        // the block that carries the payout is final, and only now does the signature leave
        submit(&mut p, 1);
        let body = String::from_utf8(handle_request(&mut p, "GET", &next_request(1), b"").2).unwrap();
        assert!(body.contains(sig), "the finalized block carries the payout");
        let block = crate::wire::decode_block(&serde_json::to_vec(&state_of(body.as_bytes())["blocks"][0]).unwrap()).unwrap();
        assert_eq!(block.token_txs.iter().map(|t| t.digest()).collect::<Vec<_>>(), vec![pending.digest()]);
        let _ = std::fs::remove_file(&pp);
    }

    #[test]
    fn a_replica_sends_submit_faucet_and_token_tx_to_the_producer() {
        let mut r = ServerState::with_spec(fast_testnet()).as_replica("https://producer.example".into());
        let ask = json!({ "address": hex_encode(&[0x7Au8; 32]) }).to_string().into_bytes();
        let paths = [("/submit", signed_body(&[0x11u8; 32], 0, TEXTS[0])), ("/faucet", ask), ("/token-tx", b"{}".to_vec()), ("/key", b"{}".to_vec())];
        for (path, body) in paths {
            let (s, _, b) = handle_request(&mut r, "POST", path, &body);
            assert_eq!(s, 409, "{path}");
            let v = state_of(&b);
            assert_eq!(v["producer"], "https://producer.example", "{path}");
            assert!(v["error"].as_str().unwrap().contains("replica"));
        }
        assert!(r.height() == 0 && r.node.mempool.is_empty() && r.next_id == 1, "nothing was taken in");
        let f = state_of(&handle_request(&mut r, "GET", "/faucet", b"").2);
        assert_eq!((f["enabled"].clone(), f["producer"].clone()), (json!(false), json!("https://producer.example")));
        let s = state_of(&handle_request(&mut r, "GET", "/state", b"").2);
        assert_eq!((s["role"].clone(), s["replica"]["following"].clone()), (json!("replica"), json!(true)));
        r.stop_following("the producer's history differs".into());
        let s = state_of(&handle_request(&mut r, "GET", "/state", b"").2);
        assert_eq!((s["replica"]["following"].clone(), s["replica"]["stopped"].clone()), (json!(false), json!("the producer's history differs")));
        let s = state_of(&handle_request(&mut ServerState::new(), "GET", "/state", b"").2);
        assert_eq!((s["role"].clone(), s["replica"].clone()), (json!("producer"), Value::Null));
    }

    /// The testnet pays every block reward to the faucet, and a producer needs the faucet seed. A
    /// follower does not: consensus fixes the reward amount and never checks the recipient, so it
    /// validates a reward to the faucet without the faucet's key. On the real testnet spec (its
    /// difficulty), where a tampered contribution also misses the PoW target.
    #[test]
    fn a_follower_runs_a_reward_to_faucet_chain_without_the_faucet_seed() {
        let spec = ChainSpec::testnet();
        assert!(spec.reward_to_faucet);
        let (mut p, pp) = producer_on(spec.clone(), "seedless-p");
        p.clock = || 1_000;
        submit(&mut p, 0);
        let newcomer = [0x7Au8; 32];
        let ask = json!({ "address": hex_encode(&newcomer) }).to_string();
        assert_eq!(handle_request(&mut p, "POST", "/faucet", ask.as_bytes()).0, 200);
        submit(&mut p, 1);
        let (mut f, fp) = follower_on(spec, "seedless-f");
        assert!(f.faucet.is_none(), "the follower holds no faucet seed");
        let genuine = page(&mut p, 0);
        let mut tampered = genuine.clone();
        tampered["blocks"][1]["cells"][0]["data"] = json!(b"a contribution the producer never took".to_vec());
        let e = stopped(apply_page(&mut f, tampered.to_string().as_bytes()));
        assert!(e.contains("block 2") && e.contains("PowUnmet"), "{e}");
        assert_eq!(f.height(), 1, "block 1 was valid and stays; the tampered block 2 is refused");
        assert_eq!(apply_page(&mut f, &handle_request(&mut p, "GET", &next_request(1), b"").2), Ok((1, false)));
        assert_eq!(f.state_digest(), p.state_digest());
        let bal = format!("/balance/{}", hex_encode(&newcomer));
        assert_eq!(state_of(&handle_request(&mut f, "GET", &bal, b"").2)["jul"], 10, "the replica shows the payout");
        assert_eq!(state_of(&handle_request(&mut f, "GET", "/faucet", b"").2)["enabled"], false);
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    // ============ Crash safety of the chain log (2026-11-18) ============

    /// Append raw bytes to a log, the way a crash mid-write or old damage leaves them.
    fn append_raw(path: &std::path::Path, bytes: &[u8]) {
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(path).unwrap().write_all(bytes).unwrap();
    }

    /// The files a boot set aside next to `log` (`<log name>.torn-at-<offset>-<time>`).
    fn aside_files(log: &std::path::Path) -> Vec<std::path::PathBuf> {
        let prefix = format!("{}.torn-at-", log.file_name().unwrap().to_string_lossy());
        let mut v: Vec<_> = std::fs::read_dir(log.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&prefix)))
            .collect();
        v.sort();
        v
    }

    fn remove_with_aside(log: &std::path::Path) {
        for a in aside_files(log) {
            let _ = std::fs::remove_file(a);
        }
        let _ = std::fs::remove_file(log);
    }

    /// A crash mid-write leaves a torn last record. The next boot sets those bytes aside in a file next to
    /// the log and cuts the log back to its last whole record, so the blocks made after the restart land
    /// on a record boundary and survive the restart after that. Before this fix the second restart stopped
    /// at the torn bytes and silently dropped every later block.
    #[test]
    fn a_torn_tail_is_set_aside_at_boot_and_every_later_block_survives() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "torn-p");
        submit(&mut p, 0);
        submit(&mut p, 1);
        drop(p); // the process ends
        let whole = std::fs::read(&pp).unwrap();
        let torn = [&999u32.to_be_bytes()[..], b"{\"height\":3,\"cel"].concat(); // a length, then part of a body
        append_raw(&pp, &torn);
        let (mut p, replayed) = ServerState::with_store(&pp, spec.clone()).unwrap();
        assert_eq!((replayed, p.height()), (2, 2));
        let (cut_back, aside) = (std::fs::read(&pp).unwrap(), aside_files(&pp));
        submit(&mut p, 2);
        submit(&mut p, 3);
        let digest = p.state_digest();
        drop(p);
        let (again, replayed) = ServerState::with_store(&pp, spec).unwrap();
        assert_eq!((replayed, again.height()), (4, 4), "every block survives the second restart");
        assert_eq!(again.state_digest(), digest);
        assert!(cut_back == whole, "the first boot cut the log back to its last whole record: {} bytes, {} whole", cut_back.len(), whole.len());
        assert_eq!(aside.len(), 1, "one set-aside file");
        assert!(std::fs::read(&aside[0]).unwrap() == torn, "it holds the torn bytes as they were");
        let name = aside[0].file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.contains(&format!(".torn-at-{}-", whole.len())), "named with the byte offset: {name}");
        assert_eq!(aside_files(&pp).len(), 1, "a clean boot sets nothing more aside");
        remove_with_aside(&pp);
    }

    /// The same for a follower's store: a follower that crashed mid-write sets its torn tail aside at
    /// boot, follows the producer on, and keeps every block across the next restart.
    #[test]
    fn a_follower_sets_its_torn_tail_aside_and_keeps_every_block_it_follows_after() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "torn-fp");
        for n in 0..4 {
            submit(&mut p, n);
        }
        let (mut f, fp) = follower_on(spec.clone(), "torn-f");
        apply_page(&mut f, &handle_request(&mut p, "GET", "/blocks?since=0&limit=2", b"").2).unwrap();
        assert_eq!(f.height(), 2);
        drop(f);
        let whole = std::fs::read(&fp).unwrap();
        let torn = [0x00u8, 0x00, 0x0B]; // a crash three bytes into the next record's length
        append_raw(&fp, &torn);
        let (f, replayed) = ServerState::with_store(&fp, spec.clone()).unwrap();
        let mut f = f.as_replica("http://producer.test".into());
        assert_eq!(replayed, 2);
        let (cut_back, aside) = (std::fs::read(&fp).unwrap(), aside_files(&fp));
        sync(&mut f, &mut p, 32).unwrap();
        assert_eq!(f.height(), 4);
        drop(f);
        let (f, replayed) = ServerState::with_store(&fp, spec).unwrap();
        assert_eq!((replayed, f.height()), (4, 4), "every followed block survives the restart");
        assert_eq!(f.state_digest(), p.state_digest());
        assert!(cut_back == whole, "cut back to the last whole record: {} bytes, {} whole", cut_back.len(), whole.len());
        assert_eq!(aside.len(), 1);
        assert!(std::fs::read(&aside[0]).unwrap() == torn, "it holds the torn bytes");
        remove_with_aside(&fp);
        let _ = std::fs::remove_file(&pp);
    }

    /// Good records after a broken one cannot come from a crash: a crash tears only the record being
    /// written, which is the last. So that is damage, and the boot refuses, says where, and leaves the
    /// file as it is rather than cut good blocks away. Covered: a torn record with whole blocks written
    /// after it (what an older node left by appending after a torn tail), on a producer's log and on a
    /// follower's; a whole record that is not a block, in the middle and at the end; and a block the
    /// chain's rules refuse (a repeated block).
    #[test]
    fn a_broken_record_that_is_not_a_torn_tail_stops_the_boot_and_the_file_is_left_alone() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "bad-p");
        for n in 0..4 {
            submit(&mut p, n);
        }
        let (mut f, fp) = follower_on(spec.clone(), "bad-f");
        sync(&mut f, &mut p, 32).unwrap();
        drop((p, f));
        // the whole frames of a log: its 4-byte big-endian length, then that many bytes
        let frames = |path: &std::path::Path| {
            let bytes = std::fs::read(path).unwrap();
            let (mut out, mut at) = (Vec::new(), 0usize);
            while at < bytes.len() {
                let len = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
                out.push(bytes[at..at + 4 + len].to_vec());
                at += 4 + len;
            }
            out
        };
        let (pf, ff) = (frames(&pp), frames(&fp));
        assert_eq!((pf.len(), ff.len()), (4, 4));
        let torn = [&999u32.to_be_bytes()[..], b"{\"height\":3"].concat();
        let not_a_block = [&11u32.to_be_bytes()[..], b"not a block"].concat();
        let cases: Vec<(&str, Vec<Vec<u8>>, ChainSpec)> = vec![
            ("producer, torn then whole", vec![pf[0].clone(), pf[1].clone(), torn.clone(), pf[2].clone(), pf[3].clone()], spec.clone()),
            ("follower, torn then whole", vec![ff[0].clone(), ff[1].clone(), torn.clone(), ff[2].clone(), ff[3].clone()], spec.clone()),
            ("not a block, mid-log", vec![pf[0].clone(), pf[1].clone(), not_a_block.clone(), pf[2].clone()], spec.clone()),
            ("not a block, at the end", vec![pf[0].clone(), pf[1].clone(), not_a_block.clone()], spec.clone()),
            ("a repeated block", vec![pf[0].clone(), pf[1].clone(), pf[1].clone()], spec.clone()),
        ];
        let at = pf[0].len() + pf[1].len();
        for (what, parts, boot_spec) in cases {
            let path = tmp_log("bad-case");
            std::fs::write(&path, parts.concat()).unwrap();
            let before = std::fs::read(&path).unwrap();
            let err = match ServerState::with_store(&path, boot_spec.clone()) {
                Err(e) => e.to_string(),
                Ok((s, n)) => panic!("{what}: booted at height {} ({n} replayed)", s.height()),
            };
            assert!(err.contains(&format!("damaged at byte {at}")) && err.contains("file is unchanged"), "{what}: {err}");
            assert!(std::fs::read(&path).unwrap() == before, "{what}: the file is untouched");
            assert!(aside_files(&path).is_empty(), "{what}: nothing is set aside");
            // a read-only reader (`--listen <store>`, the peers seed) refuses it too, rather than serve a prefix
            assert!(crate::store::load_blocks(&path, &boot_spec).is_err(), "{what}: read-only load");
            remove_with_aside(&path);
        }
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    /// A clean log boots exactly as before: the same chain, the file byte for byte, nothing set aside.
    #[test]
    fn a_clean_log_boots_as_before_and_nothing_is_set_aside() {
        let spec = ChainSpec::dev();
        let (mut p, pp) = producer_on(spec.clone(), "clean-p");
        for n in 0..3 {
            submit(&mut p, n);
        }
        let digest = p.state_digest();
        drop(p);
        let before = std::fs::read(&pp).unwrap();
        let (again, replayed) = ServerState::with_store(&pp, spec.clone()).unwrap();
        assert_eq!((replayed, again.state_digest()), (3, digest));
        assert!(std::fs::read(&pp).unwrap() == before, "the file is byte-identical");
        assert!(aside_files(&pp).is_empty());
        let absent = tmp_log("clean-absent");
        let (g, replayed) = ServerState::with_store(&absent, spec).unwrap();
        assert_eq!((replayed, g.height()), (0, 0));
        assert!(!absent.exists() && aside_files(&absent).is_empty(), "an absent log is genesis and creates nothing");
        remove_with_aside(&pp);
    }

    /// The write path: a block reaches `GET /blocks` (a follower) only once it is written and flushed to
    /// disk. A block the log cannot take is not reported final and not served, and the node takes no more
    /// blocks until it is restarted: a later block written without it would leave a gap no boot can
    /// replay. The log keeps its last whole record.
    #[test]
    fn a_block_the_log_cannot_take_is_not_reported_final_or_served_and_production_stops() {
        let (mut p, pp) = producer_on(ChainSpec::dev(), "nowrite-p");
        submit(&mut p, 0);
        // the log can no longer be written: a directory now stands where the file was
        let saved = std::fs::read(&pp).unwrap();
        std::fs::remove_file(&pp).unwrap();
        std::fs::create_dir(&pp).unwrap();
        let (s, _, b) = handle_request(&mut p, "POST", "/submit", &signed_body(&[0x41u8; 32], 0, TEXTS[1]));
        assert_eq!(s, 503, "{}", String::from_utf8_lossy(&b));
        let e = state_of(&b)["error"].as_str().unwrap().to_string();
        assert!(e.contains("could not be written to the block log") && !String::from_utf8_lossy(&b).contains("finalized"), "{e}");
        assert_eq!(page(&mut p, 1)["tip"], 1, "the unwritten block is not served");
        let (s, _, b) = handle_request(&mut p, "POST", "/submit", &signed_body(&[0x42u8; 32], 0, TEXTS[2]));
        assert_eq!(s, 503, "no more blocks until a restart: {}", String::from_utf8_lossy(&b));
        assert_eq!(p.height(), 2, "block 2 is in memory only; block 3 was never made");
        // put the log back and restart: the durable chain is block 1, and the node produces again
        std::fs::remove_dir(&pp).unwrap();
        std::fs::write(&pp, &saved).unwrap();
        let (mut q, replayed) = ServerState::with_store(&pp, ChainSpec::dev()).unwrap();
        assert_eq!(replayed, 1);
        submit(&mut q, 3);
        assert_eq!(page(&mut q, 0)["tip"], 2);
        remove_with_aside(&pp);
    }

    // ---- identity keys (wallet decisions A1 and A7) through the API ----

    /// A wallet with a real cold key (the route tests above commit to a bare cold root).
    struct KeyWallet {
        hot: [u8; 32],
        cold: [u8; 32],
        hot_root: [u8; 32],
        cold_commit: [u8; 32],
        id: [u8; 32],
    }

    fn key_wallet(hot: u8, cold: u8) -> KeyWallet {
        use noesis_core::wallet;
        let (hot, cold) = ([hot; 32], [cold; 32]);
        let (hot_root, cold_root) = (wallet::key_root(&hot), wallet::key_root(&cold));
        let cold_commit = wallet::cold_commit(&cold_root);
        KeyWallet { hot, cold, hot_root, cold_commit, id: wallet::identity(&hot_root, &cold_commit) }
    }

    /// A contribution by `w`'s identity signed by `signer` (the key the wallet holds now).
    fn contribution(w: &KeyWallet, signer: &[u8; 32], index: u32, data: &str) -> Vec<u8> {
        use noesis_core::wallet;
        let sig = wallet::sign(signer, index, &wallet::contribution_digest(&w.id, index, None, data.as_bytes()));
        json!({
            "identity": hex_encode(&w.id),
            "hot_root": hex_encode(&wallet::key_root(signer)),
            "cold_commit": hex_encode(&w.cold_commit),
            "index": index,
            "bottom": tree_json(&sig.bottom),
            "top": tree_json(&sig.top),
            "data": data,
        })
        .to_string()
        .into_bytes()
    }

    /// A `POST /key` body, signed the way the page signs it.
    #[allow(clippy::too_many_arguments)]
    fn key_op_body(w: &KeyWallet, op: &str, signer: &[u8; 32], chain: u64, generation: u64, new_root: [u8; 32], index: u32, first: bool) -> Vec<u8> {
        use noesis_core::wallet;
        let kind = Kind::from_name(op).unwrap();
        let sig = wallet::sign(signer, index, &wallet::key_op_digest(&w.id, kind.byte(), chain, generation, &new_root, index));
        let mut v = json!({ "op": op, "identity": hex_encode(&w.id), "index": index, "bottom": tree_json(&sig.bottom), "top": tree_json(&sig.top) });
        if kind != Kind::Cancel {
            v["new_root"] = json!(hex_encode(&new_root));
        }
        if kind == Kind::Recover {
            v["cold_root"] = json!(hex_encode(&wallet::key_root(signer)));
        }
        if first {
            v["birth"] = json!({ "hot_root": hex_encode(&w.hot_root), "cold_commit": hex_encode(&w.cold_commit) });
        }
        v.to_string().into_bytes()
    }

    fn keys(st: &mut ServerState, w: &KeyWallet) -> Value {
        let (s, _, b) = handle_request(st, "GET", &format!("/keys/{}", hex_encode(&w.id)), b"");
        assert_eq!(s, 200);
        state_of(&b)
    }

    fn post(st: &mut ServerState, path: &str, body: &[u8]) -> (u16, Value) {
        let (s, _, b) = handle_request(st, "POST", path, body);
        (s, state_of(&b))
    }

    /// Before any key operation an identity works exactly as before: the same check, a 40-byte label (the
    /// pre-A1 cell bytes), no key record, generation 0.
    #[test]
    fn an_identity_with_no_key_record_submits_exactly_as_before() {
        let mut st = ServerState::new();
        let dave = [0x11u8; 32];
        assert_eq!(handle_request(&mut st, "POST", "/submit", &signed_body(&dave, 0, "a note on tidal mills")).0, 200);
        let cell = &st.node.ledger.cells[0];
        let mut label = identity_of(&dave).0.to_vec();
        label.extend_from_slice(&0u64.to_le_bytes());
        assert_eq!(cell.lock.args, label, "identity || index, 40 bytes, as before A1");
        let id = id_hex(&dave);
        let k = state_of(&handle_request(&mut st, "GET", &format!("/keys/{id}"), b"").2);
        assert_eq!((k["current"].clone(), k["on_chain"].clone(), k["generation"].clone(), k["highest_used"].clone()), (Value::Null, Value::Null, json!(0), json!(0)));
        assert_eq!((k["key_rotation"].clone(), k["recovery_delay_blocks"].clone()), (json!(true), json!(3)), "the dev spec");
        let i = state_of(&handle_request(&mut st, "GET", &format!("/index/{id}"), b"").2);
        assert_eq!((i["highest_used"].clone(), i["generation"].clone()), (json!(0), json!(0)));
        assert_eq!(ChainSpec::testnet().constitution.recovery_delay_blocks, crate::chainspec::TESTNET_RECOVERY_DELAY_BLOCKS);
        assert!(ChainSpec::testnet().constitution.key_rotation);
    }

    /// A1 end to end: the current key signs a rotation to the next key in its series; from the block that
    /// carries it the old key is refused and the new key signs from index 0; the counters are per key;
    /// standing stays on the identity.
    #[test]
    fn a_rotation_through_the_route_moves_the_identity_to_its_next_key() {
        use noesis_core::wallet;
        let mut st = ServerState::new();
        let chain = st.chain_id();
        let w = key_wallet(0x13, 0xC3);
        let k1 = wallet::derive_key(&w.hot, 1);
        let root1 = wallet::key_root(&k1);
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &w.hot, 0, TEXTS[0])).0, 200);
        // a rotation at the leaf the contribution used is refused (one leaf, one message)
        let (s, v) = post(&mut st, "/key", &key_op_body(&w, "rotate", &w.hot, chain, 0, root1, 0, true));
        assert_eq!(s, 400);
        assert!(v["error"].as_str().unwrap().contains("already used"), "{v}");
        let body = key_op_body(&w, "rotate", &w.hot, chain, 0, root1, 1, true);
        let (s, v) = post(&mut st, "/key", &body);
        assert_eq!(s, 200, "{v}");
        assert_eq!((v["lands_in_block"].clone(), v["after"]["generation"].clone()), (json!(2), json!(1)));
        assert_eq!(post(&mut st, "/key", &body).0, 200, "re-sending the same operation is a no-op");
        let k = keys(&mut st, &w);
        assert_eq!((k["queued"]["op"].clone(), k["current"]["generation"].clone(), k["highest_used"].clone()), (json!("rotate"), json!(1), Value::Null));
        assert_eq!(k["on_chain"], Value::Null, "nothing on chain until a block carries it");
        // from the rotation's block on, the old key is refused...
        let (s, v) = post(&mut st, "/submit", &contribution(&w, &w.hot, 2, TEXTS[1]));
        assert_eq!(s, 400);
        assert!(v["error"].as_str().unwrap().contains("not this identity's current hot key"), "{v}");
        assert_eq!(st.height(), 1, "nothing was made");
        // ... and the new key signs from index 0, in the block that carries the rotation
        let (s, v) = post(&mut st, "/submit", &contribution(&w, &k1, 0, TEXTS[1]));
        assert_eq!(s, 200, "{v}");
        assert_eq!(st.node.ledger.cells[1].lock.args, identity_key::pack_label(&w.id, 0, 1), "the generation-1 label");
        let b = state_of(&handle_request(&mut st, "GET", "/blocks", b"").2);
        assert_eq!((b["blocks"][0]["height"].clone(), b["blocks"][0]["token_txs"].clone()), (json!(2), json!(1)));
        let k = keys(&mut st, &w);
        assert_eq!((k["on_chain"]["generation"].clone(), k["on_chain"]["hot_root"].clone()), (json!(1), json!(hex_encode(&root1))));
        assert_eq!((k["queued"].clone(), k["highest_used"].clone()), (Value::Null, json!(0)));
        let i = state_of(&handle_request(&mut st, "GET", &format!("/index/{}", hex_encode(&w.id)), b"").2);
        assert_eq!((i["generation"].clone(), i["highest_used"].clone()), (json!(1), json!(0)), "the wallet resumes its new key");
        // per-key counters: the new key cannot reuse 0, the old key cannot sign at all
        assert!(post(&mut st, "/submit", &contribution(&w, &k1, 0, TEXTS[2])).1["error"].as_str().unwrap().contains("already used"));
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &w.hot, 9, TEXTS[2])).0, 400);
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &k1, 1, TEXTS[2])).0, 200);
        // standing: one identity, credited for all three contributions across both keys
        let state = state_of(&handle_request(&mut st, "GET", "/state", b"").2);
        assert_eq!(state["contributors"].as_object().unwrap().len(), 1);
        assert!(state["contributors"][hex_encode(&w.id)].as_u64().unwrap() > 0);
        let t = tokens(&mut st);
        let kc = t["cells"].as_array().unwrap().iter().find(|c| c["standard"] == "identity-key").expect("the key cell is listed");
        assert_eq!(kc["record"]["generation"], 1);
    }

    /// A7 end to end on dev (delay 3): a wrong cold key is refused; a request waits out its delay; the
    /// current hot key cancels inside it and that leaf is then spent for contributions too; a replayed
    /// request is refused; an uncancelled request takes effect, after which the old key is retired and the
    /// recovered key signs from index 0.
    #[test]
    fn recovery_through_the_route_waits_out_the_delay_and_the_hot_key_can_cancel_it() {
        use noesis_core::wallet;
        let mut st = ServerState::new();
        let chain = st.chain_id();
        let w = key_wallet(0x14, 0xC4);
        let fresh = [0x15u8; 32]; // the recovering device's new hot seed
        let fresh_root = wallet::key_root(&fresh);
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &w.hot, 0, TEXTS[0])).0, 200);
        // a thief's cold key (the body names its root): its commitment is not the identity's
        let (s, v) = post(&mut st, "/key", &key_op_body(&w, "recover", &[0x66; 32], chain, 0, fresh_root, 0, true));
        assert_eq!(s, 400);
        assert!(v["error"].as_str().unwrap().contains("does not match this identity's cold commitment"), "{v}");
        // the real cold key: queued, lands with the next block
        let request = key_op_body(&w, "recover", &w.cold, chain, 0, fresh_root, 0, true);
        assert_eq!(post(&mut st, "/key", &request).0, 200);
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &w.hot, 1, TEXTS[1])).0, 200, "the old key still signs");
        let k = keys(&mut st, &w);
        assert_eq!(k["current"]["pending"]["effective_at"], 2 + 3 + 1);
        assert_eq!(k["current"]["pending"]["cancel_blocks_left"], 3);
        // the hot key cancels; its leaf (2) is then spent for a contribution too
        assert_eq!(post(&mut st, "/key", &key_op_body(&w, "cancel", &w.hot, chain, 0, [0u8; 32], 2, false)).0, 200);
        assert!(post(&mut st, "/submit", &contribution(&w, &w.hot, 2, TEXTS[2])).1["error"].as_str().unwrap().contains("already used"));
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &w.hot, 3, TEXTS[2])).0, 200);
        let k = keys(&mut st, &w);
        assert_eq!((k["current"]["pending"].clone(), k["current"]["hot_floor"].clone()), (Value::Null, json!(3)));
        // the same request bytes again: refused
        assert_eq!(post(&mut st, "/key", &request).0, 400);
        // a second request (cold index 1) that nobody cancels
        let second = key_op_body(&w, "recover", &w.cold, chain, 0, fresh_root, 1, false);
        assert_eq!(post(&mut st, "/key", &second).0, 200);
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &w.hot, 4, TEXTS[3])).0, 200); // block 4 carries it
        assert_eq!(keys(&mut st, &w)["current"]["pending"]["effective_at"], 4 + 3 + 1);
        for n in 4..7 {
            submit(&mut st, n); // blocks 5, 6, 7 by other identities
        }
        // block 8: in effect. The old key is retired, the replay is refused, the new key signs from 0.
        let k = keys(&mut st, &w);
        assert_eq!((k["current"]["generation"].clone(), k["current"]["hot_root"].clone()), (json!(1), json!(hex_encode(&fresh_root))));
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &w.hot, 5, TEXTS[7])).0, 400);
        assert!(post(&mut st, "/key", &second).1["error"].as_str().unwrap().contains("already used"));
        assert_eq!(post(&mut st, "/submit", &contribution(&w, &fresh, 0, TEXTS[7])).0, 200);
        assert_eq!(keys(&mut st, &w)["highest_used"], 0);
        let state = state_of(&handle_request(&mut st, "GET", "/state", b"").2);
        assert!(state["contributors"][hex_encode(&w.id)].as_u64().unwrap() > 0, "standing stayed on the identity");
    }

    /// The key lane in the rulebook, where every replica runs it: a follower derives the same key record
    /// from the blocks, refuses a block whose key operation was tampered with (a forged signature or a
    /// forged output), and a restart rebuilds the per-key counters from the chain log. On the testnet
    /// constitution (control binding, the faucet, its 100-block delay).
    #[test]
    fn the_key_lane_replays_from_the_store_and_a_follower_enforces_it() {
        use noesis_core::wallet;
        let spec = fast_testnet();
        let (mut p, pp) = producer_on(spec.clone(), "keys-p");
        let chain = p.chain_id();
        let w = key_wallet(0x16, 0xC6);
        let k1 = wallet::derive_key(&w.hot, 1);
        assert_eq!(post(&mut p, "/submit", &contribution(&w, &w.hot, 0, TEXTS[0])).0, 200);
        assert_eq!(post(&mut p, "/key", &key_op_body(&w, "rotate", &w.hot, chain, 0, wallet::key_root(&k1), 1, true)).0, 200);
        assert_eq!(post(&mut p, "/submit", &contribution(&w, &k1, 0, TEXTS[1])).0, 200); // block 2: the rotation
        assert_eq!(post(&mut p, "/key", &key_op_body(&w, "recover", &w.cold, chain, 1, wallet::key_root(&[0x17; 32]), 0, false)).0, 200);
        assert_eq!(post(&mut p, "/submit", &contribution(&w, &k1, 1, TEXTS[2])).0, 200); // block 3: the request
        assert_eq!(keys(&mut p, &w)["current"]["pending"]["effective_at"], 3 + crate::chainspec::TESTNET_RECOVERY_DELAY_BLOCKS + 1);
        // a follower: the same record, derived from the blocks
        let (mut f, fp) = follower_on(spec.clone(), "keys-f");
        apply_page(&mut f, &handle_request(&mut p, "GET", "/blocks?since=0&limit=1", b"").2).unwrap();
        let genuine = page(&mut p, 1);
        let rot = &genuine["blocks"][0]["token_txs"][0];
        assert_eq!(rot["standard"], 4, "block 2 carries the rotation");
        let mut forged_sig = genuine.clone();
        let byte = rot["auths"][0][500].as_u64().unwrap();
        forged_sig["blocks"][0]["token_txs"][0]["auths"][0][500] = json!(byte ^ 1);
        let mut forged_out = genuine.clone(); // the producer names another hot root in the output
        let mut data: Vec<u8> = serde_json::from_value(rot["outputs"][0]["data"].clone()).unwrap();
        data[9] ^= 1;
        forged_out["blocks"][0]["token_txs"][0]["outputs"][0]["data"] = json!(data);
        for tampered in [forged_sig, forged_out] {
            let e = stopped(apply_page(&mut f, tampered.to_string().as_bytes()));
            assert!(e.contains("IdentityKey"), "{e}");
            assert_eq!(f.height(), 1);
        }
        assert_eq!(apply_page(&mut f, genuine.to_string().as_bytes()), Ok((2, false)));
        assert_eq!(f.state_digest(), p.state_digest());
        for (who, st) in [("follower", &mut f), ("producer", &mut p)] {
            let k = keys(st, &w);
            assert_eq!(k["on_chain"]["generation"], 1, "{who}");
            assert_eq!(k["highest_used"], 1, "{who}");
        }
        // a restart replays the chain log through the rulebook and rebuilds the per-key counters
        drop(p);
        let (mut again, replayed) = ServerState::with_store(&pp, spec).unwrap();
        assert_eq!(replayed, 3);
        let again = &mut again;
        let k = keys(again, &w);
        assert_eq!((k["on_chain"]["generation"].clone(), k["highest_used"].clone()), (json!(1), json!(1)));
        assert!(k["on_chain"]["pending"]["effective_at"].is_u64());
        let i = state_of(&handle_request(again, "GET", &format!("/index/{}", hex_encode(&w.id)), b"").2);
        assert_eq!((i["generation"].clone(), i["highest_used"].clone()), (json!(1), json!(1)));
        let _ = (std::fs::remove_file(&pp), std::fs::remove_file(&fp));
    }

    /// Malformed key operations are refused before anything is held: bad JSON, an unknown op, a short
    /// signature, an out-of-range index.
    #[test]
    fn malformed_key_operations_are_refused_and_nothing_is_held() {
        let mut st = ServerState::new();
        let w = key_wallet(0x18, 0xC8);
        let good: Value = serde_json::from_slice(&key_op_body(&w, "rotate", &w.hot, st.chain_id(), 0, [9u8; 32], 0, true)).unwrap();
        let mut unknown = good.clone();
        unknown["op"] = json!("transfer");
        let mut short = good.clone();
        short["bottom"]["ots_sig"] = json!("00");
        let mut far = good.clone();
        far["index"] = json!(noesis_core::wallet::MAX_SIGS);
        for bad in [b"{".to_vec(), unknown.to_string().into_bytes(), short.to_string().into_bytes(), far.to_string().into_bytes()] {
            assert_eq!(handle_request(&mut st, "POST", "/key", &bad).0, 400);
        }
        assert!(st.pending_key_ops.is_empty());
        assert_eq!(handle_request(&mut st, "GET", "/keys/not-hex", b"").0, 400);
    }
}
