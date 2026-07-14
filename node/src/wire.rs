//! T1 slice-1 — the wire codec + block-log persistence (the foundation a joinable network needs).
//!
//! HONEST SCOPE: this is the *node-layer* serialization of a finalized [`Block`], plus an append-only
//! block log on disk. It is deliberately SELF-CONTAINED: the consensus/core types carry NO serde
//! derives (and `Committed` lives in the `no_std` RISC-V core, which must stay serde-free), so this
//! module owns its own mirror structs (`W*`) and converts to/from the real types. Serialization is a
//! node concern, not a consensus-type concern — keeping the boundary here means the on-VM port is
//! never dragged a serde dependency.
//!
//! The property this enables (proved in `node/tests/persistence_roundtrip.rs`): a node restarted from
//! its persisted block log replays to a **byte-identical `state_digest`**. That is persistence
//! (state survives a restart) AND the exact substrate sync will build on — "send me your block log,
//! I'll replay it." State is derived from the canonical blocks, Bitcoin-style, not trusted as a blob.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::commit_order::Committed;
use crate::runtime::{Block, PowSeal, TokenStandard, TokenTx};
use crate::{Cell, Script};

// ============ wire mirror structs (the only serde in the block path) ============

#[derive(Serialize, Deserialize)]
struct WScript {
    code_hash: [u8; 32],
    args: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct WCell {
    id: u64,
    lock: WScript,
    type_script: WScript,
    parent: Option<u64>,
    timestamp: u64,
    data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct WCommitted {
    height: u64,
    secret: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct WTokenTx {
    standard: u8, // TokenStandard tag: 0=Fungible, 1=Nft, 2=Multi
    code_hash: [u8; 32],
    args: Vec<u8>,
    inputs: Vec<WCell>,
    outputs: Vec<WCell>,
    auths: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct WPowSeal {
    bits: u32,
    nonce: u64,
}

#[derive(Serialize, Deserialize)]
struct WBlock {
    height: u64,
    cells: Vec<WCell>,
    coords: Vec<WCommitted>,
    token_txs: Vec<WTokenTx>,
    /// JUL coinbase recipient (increment 2). `#[serde(default)]` ⇒ pre-increment-2 block logs (no
    /// field) decode as `None`, preserving restart/replay compatibility.
    #[serde(default)]
    coinbase: Option<WScript>,
    /// PoW seal (M2). `#[serde(default)]` ⇒ pre-M2 block logs (no field) decode as `None`, preserving
    /// the byte-identical restart/replay contract — the same additive precedent as `coinbase`.
    #[serde(default)]
    pow: Option<WPowSeal>,
    /// Committee-attested wall-clock timestamp (inc-CLK-1). `#[serde(default)]` ⇒ pre-CLK block logs
    /// (no field) decode as `None`, preserving the byte-identical restart/replay contract — the same
    /// additive precedent as `pow`/`coinbase`.
    #[serde(default)]
    timestamp: Option<u64>,
}

// ============ conversions (real type -> wire) ============

fn w_script(s: &Script) -> WScript {
    WScript { code_hash: s.code_hash, args: s.args.clone() }
}
fn w_cell(c: &Cell) -> WCell {
    WCell {
        id: c.id,
        lock: w_script(&c.lock),
        type_script: w_script(&c.type_script),
        parent: c.parent,
        timestamp: c.timestamp,
        data: c.data.clone(),
    }
}
fn w_committed(c: &Committed) -> WCommitted {
    WCommitted { height: c.height, secret: c.secret }
}
fn w_standard(s: &TokenStandard) -> u8 {
    match s {
        TokenStandard::Fungible => 0,
        TokenStandard::Nft => 1,
        TokenStandard::Multi => 2,
    }
}
fn w_tokentx(t: &TokenTx) -> WTokenTx {
    WTokenTx {
        standard: w_standard(&t.standard),
        code_hash: t.code_hash,
        args: t.args.clone(),
        inputs: t.inputs.iter().map(w_cell).collect(),
        outputs: t.outputs.iter().map(w_cell).collect(),
        auths: t.auths.clone(),
    }
}
fn w_block(b: &Block) -> WBlock {
    WBlock {
        height: b.height,
        cells: b.cells.iter().map(w_cell).collect(),
        coords: b.coords.iter().map(w_committed).collect(),
        token_txs: b.token_txs.iter().map(w_tokentx).collect(),
        coinbase: b.coinbase.as_ref().map(w_script),
        pow: b.pow.map(|s| WPowSeal { bits: s.bits, nonce: s.nonce }),
        timestamp: b.timestamp,
    }
}

// ============ conversions (wire -> real type) ============

fn r_script(s: WScript) -> Script {
    Script { code_hash: s.code_hash, args: s.args }
}
fn r_cell(c: WCell) -> Cell {
    Cell {
        id: c.id,
        lock: r_script(c.lock),
        type_script: r_script(c.type_script),
        parent: c.parent,
        timestamp: c.timestamp,
        data: c.data,
    }
}
fn r_committed(c: WCommitted) -> Committed {
    Committed { height: c.height, secret: c.secret }
}
fn r_standard(tag: u8) -> Result<TokenStandard, WireError> {
    match tag {
        0 => Ok(TokenStandard::Fungible),
        1 => Ok(TokenStandard::Nft),
        2 => Ok(TokenStandard::Multi),
        other => Err(WireError(format!("unknown TokenStandard tag {other}"))),
    }
}
fn r_tokentx(t: WTokenTx) -> Result<TokenTx, WireError> {
    Ok(TokenTx {
        standard: r_standard(t.standard)?,
        code_hash: t.code_hash,
        args: t.args,
        inputs: t.inputs.into_iter().map(r_cell).collect(),
        outputs: t.outputs.into_iter().map(r_cell).collect(),
        auths: t.auths,
    })
}
fn r_block(b: WBlock) -> Result<Block, WireError> {
    Ok(Block {
        height: b.height,
        cells: b.cells.into_iter().map(r_cell).collect(),
        coords: b.coords.into_iter().map(r_committed).collect(),
        token_txs: b.token_txs.into_iter().map(r_tokentx).collect::<Result<_, _>>()?,
        coinbase: b.coinbase.map(r_script),
        pow: b.pow.map(|s| PowSeal { bits: s.bits, nonce: s.nonce }),
        timestamp: b.timestamp,
    })
}

// ============ codec ============

/// Deterministic error for a malformed block on the wire / on disk.
#[derive(Debug)]
pub struct WireError(pub String);
impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wire error: {}", self.0)
    }
}
impl std::error::Error for WireError {}

/// Encode a finalized block to bytes. The wire mirror has no maps and fixed field order, so the
/// output is byte-deterministic across replicas (a joining node can compare wires bit-for-bit).
pub fn encode_block(b: &Block) -> Vec<u8> {
    serde_json::to_vec(&w_block(b)).expect("WBlock has no non-string map keys ⇒ always serializable")
}

/// Decode a block from bytes produced by [`encode_block`].
pub fn decode_block(bytes: &[u8]) -> Result<Block, WireError> {
    let wb: WBlock = serde_json::from_slice(bytes).map_err(|e| WireError(e.to_string()))?;
    r_block(wb)
}

// ============ block-log persistence ============

/// An append-only log of finalized blocks on disk (one JSON-encoded block per line). Load replays
/// them; a fresh node that applies the loaded blocks reconstructs the same ledger. This is the
/// honest "state = f(canonical blocks)" persistence: nothing is trusted as a snapshot blob.
pub struct BlockLog {
    path: PathBuf,
}

impl BlockLog {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Append one finalized block. Create-if-absent, append-only (never rewrites history).
    pub fn append(&self, b: &Block) -> io::Result<()> {
        let mut line = encode_block(b);
        line.push(b'\n');
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path)?;
        f.write_all(&line)
    }

    /// Load every block in order. A blank line is skipped; a malformed line is a hard error (a
    /// corrupt log must fail loudly, never silently drop a block and diverge).
    pub fn load(&self) -> io::Result<Vec<Block>> {
        let data = std::fs::read_to_string(&self.path)?;
        let mut out = Vec::new();
        for line in data.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let block = decode_block(line.as_bytes())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            out.push(block);
        }
        Ok(out)
    }
}
