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
use crate::runtime::{Block, Bond, PowSeal, TokenStandard, TokenTx};
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
    standard: u8, // TokenStandard tag: 0=Fungible, 1=Nft, 2=Multi, 3=Soulbound, 4=IdentityKey
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
    /// The network the block was made for (2026-11-18). REQUIRED, unlike the additive fields below: a
    /// record written before blocks named their chain has no id to check, and its seal was mined over the
    /// v1 header digest, so it does not decode (a log from before this change cannot be replayed).
    chain_id: u64,
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
    /// Bound-B commit-deposits (L5). `#[serde(default)]` ⇒ pre-L5 / inert-deposit block logs (no field)
    /// decode as an empty vec, preserving byte-identical restart/replay — the additive precedent.
    #[serde(default)]
    bonds: Vec<WBond>,
    /// Sub-block absorption root (slice-2c). `#[serde(default)]` ⇒ pre-sub-block block logs (no field)
    /// decode as `None`, preserving byte-identical restart/replay — the same additive precedent as
    /// `bonds`/`timestamp`/`pow`/`coinbase`.
    #[serde(default)]
    subblock_root: Option<[u8; 32]>,
    /// Parent-block commitment (inc-2a). `#[serde(default)]` ⇒ pre-inc-2a block logs (no field) decode
    /// as `None`, preserving byte-identical restart/replay — the same additive precedent as
    /// `subblock_root`/`bonds`/`timestamp`.
    #[serde(default)]
    parent_hash: Option<[u8; 32]>,
}

#[derive(Serialize, Deserialize)]
struct WBond {
    for_cell: u64,
    deposit: WCell,
    auth: Vec<u8>,
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
        TokenStandard::Soulbound => 3,
        TokenStandard::IdentityKey => 4,
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
        chain_id: b.chain_id,
        height: b.height,
        cells: b.cells.iter().map(w_cell).collect(),
        coords: b.coords.iter().map(w_committed).collect(),
        token_txs: b.token_txs.iter().map(w_tokentx).collect(),
        coinbase: b.coinbase.as_ref().map(w_script),
        pow: b.pow.map(|s| WPowSeal { bits: s.bits, nonce: s.nonce }),
        timestamp: b.timestamp,
        bonds: b
            .bonds
            .iter()
            .map(|bd| WBond { for_cell: bd.for_cell, deposit: w_cell(&bd.deposit), auth: bd.auth.clone() })
            .collect(),
        subblock_root: b.subblock_root,
        parent_hash: b.parent_hash,
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
        3 => Ok(TokenStandard::Soulbound),
        4 => Ok(TokenStandard::IdentityKey),
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
        chain_id: b.chain_id,
        height: b.height,
        cells: b.cells.into_iter().map(r_cell).collect(),
        coords: b.coords.into_iter().map(r_committed).collect(),
        token_txs: b.token_txs.into_iter().map(r_tokentx).collect::<Result<_, _>>()?,
        coinbase: b.coinbase.map(r_script),
        pow: b.pow.map(|s| PowSeal { bits: s.bits, nonce: s.nonce }),
        timestamp: b.timestamp,
        bonds: b
            .bonds
            .into_iter()
            .map(|bd| Bond { for_cell: bd.for_cell, deposit: r_cell(bd.deposit), auth: bd.auth })
            .collect(),
        subblock_root: b.subblock_root,
        parent_hash: b.parent_hash,
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

/// One token movement as JSON, in the same mirror shape the block codec uses (so a tx handed to a
/// node and the block that later carries it share one format).
pub fn token_tx_to_json(t: &TokenTx) -> serde_json::Value {
    serde_json::to_value(w_tokentx(t)).expect("WTokenTx has no non-string map keys ⇒ always serializable")
}

/// Decode a token movement from [`token_tx_to_json`]'s shape. An unknown standard tag is an error.
pub fn token_tx_from_json(v: serde_json::Value) -> Result<TokenTx, WireError> {
    let wt: WTokenTx = serde_json::from_value(v).map_err(|e| WireError(e.to_string()))?;
    r_tokentx(wt)
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
        // Guard against unbounded reads: a multi-gigabyte (corrupt or hostile) log must fail loudly
        // rather than OOM the node. "Send me your block log, I'll replay it" is an attack surface.
        const MAX_BLOCKLOG_SIZE: u64 = 1 << 30; // 1 GiB
        let file_size = std::fs::metadata(&self.path)?.len();
        if file_size > MAX_BLOCKLOG_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("block log exceeds maximum size of {MAX_BLOCKLOG_SIZE} bytes"),
            ));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::soulbound::{self, Identity, Tier};

    fn sb_cell(id: u64, lock: &[u8], data: Vec<u8>) -> Cell {
        Cell {
            id,
            lock: Script { code_hash: [0u8; 32], args: lock.to_vec() },
            type_script: Script { code_hash: soulbound::CODE_HASH, args: b"test-identity-v1".to_vec() },
            parent: None,
            timestamp: 0,
            data,
        }
    }

    fn soulbound_tx() -> TokenTx {
        let ident = Identity { serial: 9, tier: Tier::Member, holder: [5u8; 32] };
        TokenTx {
            standard: TokenStandard::Soulbound,
            code_hash: soulbound::CODE_HASH,
            args: b"test-identity-v1".to_vec(),
            inputs: vec![sb_cell(1, &[1u8; 32], soulbound::encode_authority(0))],
            outputs: vec![
                sb_cell(2, &[2u8; 32], soulbound::encode_authority(1)),
                sb_cell(3, &[3u8; 32], soulbound::encode_identity(&ident)),
            ],
            auths: vec![vec![7u8; 64]],
        }
    }

    fn same_tx(a: &TokenTx, b: &TokenTx) -> bool {
        let cells_eq = |x: &[Cell], y: &[Cell]| {
            x.len() == y.len()
                && x.iter().zip(y).all(|(p, q)| {
                    p.id == q.id && p.lock == q.lock && p.type_script == q.type_script && p.data == q.data
                })
        };
        a.standard == b.standard
            && a.code_hash == b.code_hash
            && a.args == b.args
            && a.auths == b.auths
            && cells_eq(&a.inputs, &b.inputs)
            && cells_eq(&a.outputs, &b.outputs)
    }

    #[test]
    fn soulbound_tx_round_trips_through_the_block_codec_with_tag_3() {
        let tx = soulbound_tx();
        assert_eq!(TokenStandard::Soulbound as u8, 3, "the digest tag (standard as u8) is 3");
        assert_eq!(w_standard(&TokenStandard::Soulbound), 3, "the wire tag is 3");
        let block = Block::assemble(1, &[]).with_token_txs(vec![tx.clone()]);
        let bytes = encode_block(&block);
        let back = decode_block(&bytes).expect("a soulbound block decodes");
        assert_eq!(back.token_txs.len(), 1);
        assert!(same_tx(&back.token_txs[0], &tx), "every field survives the round trip");
        assert_eq!(encode_block(&back), bytes, "byte-identical re-encode");
        // and the single-tx JSON codec
        let back_tx = token_tx_from_json(token_tx_to_json(&tx)).expect("tx JSON decodes");
        assert!(same_tx(&back_tx, &tx));
        assert_eq!(back_tx.digest(), tx.digest(), "the signed digest survives the codec");
    }

    #[test]
    fn an_unknown_standard_tag_is_still_rejected() {
        let mut v = token_tx_to_json(&soulbound_tx());
        v["standard"] = serde_json::json!(5);
        assert!(token_tx_from_json(v).is_err(), "tag 5 is unknown");
    }

    /// An identity-key tx (tag 4) survives the block codec byte for byte, so a block carrying a key
    /// operation replays from the chain log and pages to a follower unchanged.
    #[test]
    fn an_identity_key_tx_round_trips_through_the_block_codec_with_tag_4() {
        use crate::identity_key::{key_cell, Record, CODE_HASH};
        let id = [7u8; 32];
        let tx = TokenTx {
            standard: TokenStandard::IdentityKey,
            code_hash: CODE_HASH,
            args: id.to_vec(),
            inputs: vec![],
            outputs: vec![key_cell(&id, &Record::birth([1u8; 32], [2u8; 32]))],
            auths: vec![vec![0x4B; 100]],
        };
        assert_eq!(TokenStandard::IdentityKey as u8, 4);
        let block = Block::assemble(1, &[]).with_token_txs(vec![tx.clone()]);
        let bytes = encode_block(&block);
        let back = decode_block(&bytes).expect("decodes");
        assert!(same_tx(&back.token_txs[0], &tx));
        assert_eq!(encode_block(&back), bytes, "byte-identical re-encode");
        assert_eq!(back.token_txs[0].digest(), tx.digest());
    }
}
