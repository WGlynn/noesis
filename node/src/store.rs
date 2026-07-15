//! store — the node's durable chain: an append-only, length-framed block log so the chain SURVIVES a
//! restart instead of resetting to genesis (the difference between a demo and a node that stays on).
//!
//! Each finalized block is appended as `u32 big-endian length || encode_block(block)` (the same wire
//! shape a peer serves). On boot, [`load_chain`] replays the log through the EXACT path a joiner uses —
//! `decode_block` → `Node::validate` → `Node::apply` — so persistence trusts the RULEBOOK, not the
//! bytes on disk: a record that fails validation stops the replay (fail-closed, the valid prefix is
//! kept), and a truncated tail (a crash mid-append) is treated as end-of-log, not an error.
//!
//! Lean by design (the `net`/`sync` precedent): std file I/O only, no database, no serialization
//! framework beyond the chain's own `wire`. Single-node durability for the invite-a-few-friends tier;
//! log compaction / snapshots are a later concern once chains get long.

use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use crate::chainspec::ChainSpec;
use crate::runtime::{Block, Node};
use crate::wire::{decode_block, encode_block};

/// Upper bound on a single framed record (a block is small). A corrupt/hostile length prefix cannot
/// make the loader pre-allocate an enormous buffer — the same "never let the input choose the
/// allocation size" discipline as the on-VM index codec.
const MAX_RECORD: usize = 16 * 1024 * 1024;

/// Append one finalized block to the log (create-if-absent, append, flush). Length-framed so the
/// reader can re-delimit records from the byte stream.
pub fn append_block(path: &Path, block: &Block) -> io::Result<()> {
    let bytes = encode_block(block);
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(&(bytes.len() as u32).to_be_bytes())?;
    f.write_all(&bytes)?;
    f.flush()
}

/// Replay a persisted log into a fresh genesis node (from `spec`) and return the node + the number of
/// blocks applied. A missing log ⇒ genesis (0 blocks). Fail-closed: an invalid or unparseable record
/// stops the replay with the valid prefix intact; a truncated final record (crash mid-append) is
/// treated as clean end-of-log.
pub fn load_chain(path: &Path, spec: &ChainSpec) -> io::Result<(Node, usize)> {
    let (mut node, _keys) = spec.genesis_node();
    if !path.exists() {
        return Ok((node, 0));
    }
    let mut r = BufReader::new(File::open(path)?);
    let mut applied = 0usize;
    loop {
        // Length prefix. EOF here = clean end of log.
        let mut len_buf = [0u8; 4];
        match r.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        if len == 0 || len > MAX_RECORD {
            break; // corrupt frame ⇒ stop, keep the valid prefix
        }
        // Body. A short read here = a torn tail from a crash mid-append ⇒ treat as end-of-log.
        let mut buf = vec![0u8; len];
        match r.read_exact(&mut buf) {
            Ok(()) => {}
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
        let block = match decode_block(&buf) {
            Ok(b) => b,
            Err(_) => break, // unparseable ⇒ stop, keep the valid prefix
        };
        // Trust the rulebook, not the disk (the sync_from discipline): never apply a block our own
        // validate rejects. A tampered/incompatible log stops here rather than corrupting state.
        if !node.validate(&block) {
            break;
        }
        node.apply(&block);
        applied += 1;
    }
    Ok((node, applied))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cell, Script};
    use std::path::PathBuf;

    fn tmp(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        p.push(format!("noesis-store-{tag}-{nanos}.log"));
        p
    }

    // Produce a chain of `n` blocks (one contribution each) via the real engine, appending each to `path`.
    fn build_and_persist(path: &Path, spec: &ChainSpec, n: u64) -> Node {
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
                crate::commit_order::Committed { height: i, secret: [i as u8; 32] },
            );
            let block = spec.produce_block(&mut node).expect("block finalizes");
            append_block(path, &block).expect("append");
        }
        node
    }

    #[test]
    fn replay_reconstructs_identical_state() {
        let spec = ChainSpec::dev();
        let path = tmp("replay");
        let live = build_and_persist(&path, &spec, 3);
        let (loaded, applied) = load_chain(&path, &spec).expect("load");
        assert_eq!(applied, 3);
        assert_eq!(loaded.ledger.height, live.ledger.height);
        assert_eq!(loaded.ledger.jul_supply.issued(), live.ledger.jul_supply.issued());
        assert_eq!(loaded.ledger.state_digest(), live.ledger.state_digest(), "restart converges byte-identical");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_log_is_genesis() {
        let spec = ChainSpec::dev();
        let (node, applied) = load_chain(Path::new("definitely-does-not-exist.log"), &spec).expect("load");
        assert_eq!(applied, 0);
        assert_eq!(node.ledger.height, 0);
    }

    #[test]
    fn a_torn_tail_keeps_the_valid_prefix() {
        let spec = ChainSpec::dev();
        let path = tmp("torn");
        build_and_persist(&path, &spec, 2);
        // Simulate a crash mid-append: corrupt the file by appending a partial framed record.
        {
            let mut f = OpenOptions::new().append(true).open(&path).unwrap();
            f.write_all(&(999u32).to_be_bytes()).unwrap(); // claims 999 bytes...
            f.write_all(b"only a few").unwrap(); // ...but the body is truncated
        }
        let (node, applied) = load_chain(&path, &spec).expect("load tolerates a torn tail");
        assert_eq!(applied, 2, "the two whole blocks survive; the torn record is dropped");
        assert_eq!(node.ledger.height, 2);
        let _ = std::fs::remove_file(&path);
    }
}
