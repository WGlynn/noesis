//! store — the node's durable chain: an append-only, length-framed block log so the chain SURVIVES a
//! restart instead of resetting to genesis (the difference between a demo and a node that stays on).
//!
//! Each finalized block is appended as `u32 big-endian length || encode_block(block)` (the same wire
//! shape a peer serves), in ONE write call, and flushed to disk (`sync_all`) before the append returns.
//! A node records a block as served (`GET /blocks`) or final (`POST /submit`) only after that, so a block
//! a follower has seen is already durable on the node that served it.
//!
//! On boot the log is replayed through the EXACT path a joiner uses — `decode_block` → `Node::validate`
//! → `Node::apply` — so persistence trusts the RULEBOOK, not the bytes on disk. Where the replay meets a
//! record that is not a good block, what happens depends on what a crash can leave (2026-11-18):
//!
//! * A crash mid-append tears only the record being written, which is the last one. So when the bytes
//!   after the last good record are only part of one record (the file ends inside it, and no whole
//!   block record starts among them), or are all zeros (space a power cut left unwritten), that is a
//!   TORN TAIL. A node that will append to the log ([`load_for_append`]: `--serve-api`, `--follow`)
//!   copies those bytes to a file next to the log, `<log name>.torn-at-<byte offset>-<unix seconds>`,
//!   never deleted, cuts the log back to the last good record, flushes both, prints one line saying so,
//!   and only then appends. Before this, the torn bytes stayed, new blocks were appended after them, and
//!   the next boot stopped at the torn record and dropped every later block. A read-only load
//!   ([`load_blocks`]: `--listen <store>`, the peers seed) stops before a torn tail and changes nothing,
//!   since the node that writes the log may be in the middle of an append.
//! * Anything else is DAMAGE, not a crash: a whole record that is not a block, a block the chain's
//!   rules refuse, a length no block record has, or a whole block record after the broken one. The load
//!   fails, says where, and leaves the file as it is. Cutting it back would throw away good blocks.
//!
//! Lean by design (the `net`/`sync` precedent): std file I/O only, no database, no serialization
//! framework beyond the chain's own `wire`. Single-node durability for the invite-a-few-friends tier;
//! log compaction / snapshots are a later concern once chains get long.

use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::chainspec::ChainSpec;
use crate::runtime::{Block, Node};
use crate::wire::{decode_block, encode_block};

/// Upper bound on a single framed record (a block is small). A corrupt/hostile length prefix cannot
/// make the loader pre-allocate an enormous buffer — the same "never let the input choose the
/// allocation size" discipline as the on-VM index codec.
const MAX_RECORD: usize = 16 * 1024 * 1024;

/// Append one finalized block to the log (create-if-absent, append, flush to disk). Length-framed so
/// the reader can re-delimit records from the byte stream.
pub fn append_block(path: &Path, block: &Block) -> io::Result<()> {
    append_block_at(path, block).map(|_| ())
}

/// [`append_block`], returning the byte offset where the new frame starts (the file's length before
/// the write), so a node that serves its log (`GET /blocks?since=N`) can find each block again.
///
/// The frame goes out in one write call and is flushed to disk before this returns. If the write or
/// the flush fails, the log is cut back to `offset` (its last whole record), so no later append lands
/// behind half a record; if even that fails, the next boot finds a torn tail and sets it aside.
pub fn append_block_at(path: &Path, block: &Block) -> io::Result<u64> {
    let body = encode_block(block);
    if body.is_empty() || body.len() > MAX_RECORD {
        // Never write a record the loader would refuse.
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("a {}-byte block record is out of bounds", body.len()),
        ));
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let offset = f.metadata()?.len();
    if let Err(e) = f.write_all(&frame).and_then(|()| f.sync_all()) {
        drop(f);
        // An append-only handle cannot truncate on every platform: open one that can.
        let _ = OpenOptions::new().write(true).open(path).and_then(|t| {
            t.set_len(offset)?;
            t.sync_all()
        });
        return Err(e);
    }
    if offset == 0 {
        sync_parent_dir(path)?; // a new log's directory entry must be durable too
    }
    Ok(offset)
}

/// Flush the directory holding `path`, so a file just created there survives a power cut (unix). Std
/// cannot open a directory on Windows; there this does nothing and the file's own flush is all there is.
fn sync_parent_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        File::open(dir)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Read the one framed record that starts at `offset` (as returned by [`append_block_at`] or
/// [`load_blocks_indexed`]): the exact `encode_block` bytes that were appended.
pub fn read_frame_at(f: &mut File, offset: u64) -> io::Result<Vec<u8>> {
    f.seek(SeekFrom::Start(offset))?;
    let mut len_buf = [0u8; 4];
    f.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > MAX_RECORD {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "corrupt frame length"));
    }
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

/// Replay a persisted log into a fresh genesis node (from `spec`), returning the node AND the ordered
/// list of blocks that were applied. A missing log ⇒ genesis (empty). READ-ONLY: a torn tail (a crash
/// mid-append) ends the replay with the whole blocks before it and leaves the file alone; damage (see
/// the module docs) is an `InvalidData` error that says where.
///
/// The returned `Vec<Block>` IS the canonical block log a T1 seed serves to joiners (`sync::serve`),
/// so a store-backed `--listen` seed and the durable `--serve-api` node replay ONE on-disk chain
/// instead of two drifting drivers (slice-5b unify). `load_chain` is a thin wrapper for callers that
/// only need the reconstructed node.
pub fn load_blocks(path: &Path, spec: &ChainSpec) -> io::Result<(Node, Vec<Block>)> {
    load_blocks_indexed(path, spec).map(|(node, blocks, _offsets)| (node, blocks))
}

/// [`load_blocks`], also returning the byte offset of each applied block's frame (`offsets[i]` is the
/// frame of `blocks[i]`), so a serving node can re-read any block without keeping it in memory.
pub fn load_blocks_indexed(path: &Path, spec: &ChainSpec) -> io::Result<(Node, Vec<Block>, Vec<u64>)> {
    let r = replay(path, spec)?;
    Ok((r.node, r.blocks, r.offsets))
}

/// The boot path of a node that will APPEND to the log (`--serve-api`, `--follow`):
/// [`load_blocks_indexed`], and if the log ends in a torn tail, set those bytes aside in a new file next
/// to the log (`<log name>.torn-at-<byte offset>-<unix seconds>`, never overwritten or deleted), cut the
/// log back to its last good record, flush both to disk, and print one line saying what was set aside
/// and where, all before returning, so the first new block lands on a record boundary. A clean log is
/// not opened for writing at all. Damage fails as in [`load_blocks`], with the file untouched.
pub fn load_for_append(path: &Path, spec: &ChainSpec) -> io::Result<(Node, Vec<Block>, Vec<u64>)> {
    let r = replay(path, spec)?;
    if let Some(torn) = &r.torn {
        let aside = set_aside(path, r.good_len, torn)?;
        let f = OpenOptions::new().write(true).open(path)?;
        f.set_len(r.good_len)?;
        f.sync_all()?;
        eprintln!(
            "noesisd: the block log {} ended in a torn record (a crash mid-write): set its {} bytes from byte {} \
             aside in {} and cut the log back to its {} whole blocks",
            path.display(),
            torn.len(),
            r.good_len,
            aside.display(),
            r.blocks.len()
        );
    }
    Ok((r.node, r.blocks, r.offsets))
}

/// Replay a persisted log into a fresh genesis node and return the node + the number of blocks
/// applied. Thin wrapper over [`load_blocks`] (read-only, the same rules).
pub fn load_chain(path: &Path, spec: &ChainSpec) -> io::Result<(Node, usize)> {
    let (node, blocks) = load_blocks(path, spec)?;
    Ok((node, blocks.len()))
}

/// A replay: the node and blocks, the length of the good records, and the torn tail after them, if any.
struct Replay {
    node: Node,
    blocks: Vec<Block>,
    offsets: Vec<u64>,
    /// Bytes of good records from the start of the file: the whole file when `torn` is `None`.
    good_len: u64,
    /// The bytes after `good_len` when they are a torn tail.
    torn: Option<Vec<u8>>,
}

fn replay(path: &Path, spec: &ChainSpec) -> io::Result<Replay> {
    let (mut node, _keys) = spec.genesis_node();
    let mut blocks = Vec::new();
    let mut offsets = Vec::new();
    if !path.exists() {
        return Ok(Replay { node, blocks, offsets, good_len: 0, torn: None });
    }
    // Replay a fixed length: the file as it stood when the replay began. A read-only load can run while
    // the writer appends; judging only these bytes keeps a record finished mid-replay from looking like
    // a whole record after a broken one.
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut r = BufReader::new(file.take(file_len));
    let mut at: u64 = 0;
    // `None`: the file ended on a record boundary. `Some(why)`: the record at `at` is not a good block.
    let broken = loop {
        let mut len_buf = [0u8; 4];
        match read_up_to(&mut r, &mut len_buf)? {
            0 => break None,
            4 => {}
            n => break Some(format!("the file ends {n} bytes into a record's length")),
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        if len == 0 || len > MAX_RECORD {
            break Some(format!("a record gives its length as {len} bytes, which no block record has"));
        }
        let mut buf = vec![0u8; len];
        let got = read_up_to(&mut r, &mut buf)?;
        if got < len {
            break Some(format!("the file ends {got} bytes into a {len}-byte record"));
        }
        let block = match decode_block(&buf) {
            Ok(b) => b,
            Err(e) => break Some(format!("a whole {len}-byte record does not decode as a block ({e})")),
        };
        // Trust the rulebook, not the disk (the sync_from discipline): never apply a block our own
        // validate rejects.
        if !node.validate(&block) {
            break Some(format!(
                "this chain's rules (chain 0x{:x}) refuse the block it holds, at height {} (is this the right \
                 network's log? check NOESIS_NET)",
                spec.chain_id, block.height
            ));
        }
        node.apply(&block);
        blocks.push(block);
        offsets.push(at);
        at += 4 + len as u64;
    };
    let torn = match broken {
        None => None,
        Some(why) => Some(torn_tail(path, at, file_len, blocks.len(), &why)?),
    };
    Ok(Replay { node, blocks, offsets, good_len: at, torn })
}

/// Read until `buf` is full or the file ends, returning how many bytes were read.
fn read_up_to(r: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// The bytes from `at` (the end of the last good record) to `file_len` (the end of the file as the
/// replay saw it), when they are what a crash mid-append can leave: part of ONE record (the file ends inside it, and no whole block record
/// starts among them), or all zeros (space a power cut left unwritten). Otherwise an `InvalidData`
/// error saying what is wrong and that the file was not changed.
fn torn_tail(path: &Path, at: u64, file_len: u64, good: usize, why: &str) -> io::Result<Vec<u8>> {
    let refuse = |detail: String| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "the block log {} is damaged at byte {at}, after {good} good blocks: {why}; {detail}. A crash \
                 mid-write leaves only part of the last record, so this is not a crash, and the node will not cut \
                 blocks away to start. The file is unchanged. Restore it from a copy, or start the node on a new \
                 chain log and keep this one.",
                path.display()
            ),
        )
    };
    let rest_len = file_len.saturating_sub(at);
    if rest_len > (4 + MAX_RECORD) as u64 {
        return Err(refuse(format!("{rest_len} bytes follow it, more than one record can hold")));
    }
    let mut rest = Vec::with_capacity(rest_len as usize);
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(at))?;
    f.take(rest_len).read_to_end(&mut rest)?;
    if rest.len() < 4 || rest.iter().all(|&b| b == 0) {
        return Ok(rest);
    }
    let len = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
    if len == 0 || len > MAX_RECORD {
        return Err(refuse(format!("{} bytes follow it", rest.len())));
    }
    if 4 + len <= rest.len() {
        return Err(refuse("the record is whole, so it was written in full".into()));
    }
    if let Some(i) = (1..rest.len()).find(|&i| whole_block_record(&rest[i..])) {
        return Err(refuse(format!("a whole block record starts after it, at byte {}", at + i as u64)));
    }
    Ok(rest)
}

/// Does `b` start with a whole record that decodes as a block?
fn whole_block_record(b: &[u8]) -> bool {
    if b.len() < 4 {
        return false;
    }
    let len = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
    len != 0 && len <= MAX_RECORD && 4 + len <= b.len() && decode_block(&b[4..4 + len]).is_ok()
}

/// Copy `bytes` (a torn tail found at byte `at`) to a new file next to the log and flush it to disk.
/// Never overwrites: a name already taken gets a counter.
fn set_aside(path: &Path, at: u64, bytes: &[u8]) -> io::Result<PathBuf> {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "chain.log".into());
    let mut n = 0u32;
    loop {
        let suffix = if n == 0 { String::new() } else { format!("-{n}") };
        let aside = path.with_file_name(format!("{name}.torn-at-{at}-{secs}{suffix}"));
        match OpenOptions::new().write(true).create_new(true).open(&aside) {
            Ok(mut f) => {
                f.write_all(bytes)?;
                f.sync_all()?;
                sync_parent_dir(&aside)?;
                return Ok(aside);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e),
        }
    }
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
    fn load_blocks_returns_the_canonical_log_a_seed_serves() {
        // The unify invariant (slice-5b): `load_blocks` yields the SAME ordered blocks that were
        // appended, and a fresh node applying them converges byte-identical — so a store-backed seed
        // serves exactly the durable chain, not a separate scripted one.
        let spec = ChainSpec::dev();
        let path = tmp("load-blocks");
        let live = build_and_persist(&path, &spec, 3);
        let (loaded, blocks) = load_blocks(&path, &spec).expect("load");
        assert_eq!(blocks.len(), 3, "all three durable blocks are returned in order");
        assert_eq!(blocks.iter().map(|b| b.height).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(loaded.ledger.state_digest(), live.ledger.state_digest(), "node matches the live chain");
        // Re-applying the returned log into a fresh genesis reconstructs the same state (what a joiner does).
        let (mut joiner, _keys) = spec.genesis_node();
        for b in &blocks {
            assert!(joiner.validate(b), "a served block passes the joiner's own rulebook");
            joiner.apply(b);
        }
        assert_eq!(joiner.ledger.state_digest(), live.ledger.state_digest(), "joiner converges via the served log");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn frame_offsets_from_append_and_from_replay_agree_and_read_back_the_block() {
        let spec = ChainSpec::dev();
        let path = tmp("offsets");
        let (mut node, _keys) = spec.genesis_node();
        let mut appended = Vec::new();
        for i in 1..=3u64 {
            node.submit(
                Cell {
                    id: i,
                    lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
                    type_script: Script { code_hash: [1u8; 32], args: b"alice".to_vec() },
                    parent: None,
                    timestamp: i,
                    data: format!("offset test contribution {i}, about tide pools").into_bytes(),
                },
                crate::commit_order::Committed { height: i, secret: [i as u8; 32] },
            );
            let block = spec.produce_block(&mut node).expect("block finalizes");
            appended.push((append_block_at(&path, &block).expect("append"), block));
        }
        let (_node, blocks, offsets) = load_blocks_indexed(&path, &spec).expect("load");
        assert_eq!(offsets, appended.iter().map(|(o, _)| *o).collect::<Vec<_>>(), "one offset per frame, in order");
        assert_eq!(offsets[0], 0);
        let mut f = File::open(&path).unwrap();
        for ((off, block), loaded) in appended.iter().zip(&blocks) {
            let bytes = read_frame_at(&mut f, *off).expect("read the frame");
            assert_eq!(bytes, encode_block(block), "the frame holds the exact appended bytes");
            assert_eq!(decode_block(&bytes).unwrap().height, loaded.height);
        }
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
        let before = std::fs::read(&path).unwrap();
        let (node, applied) = load_chain(&path, &spec).expect("load tolerates a torn tail");
        assert_eq!(applied, 2, "the two whole blocks survive; the torn record is dropped");
        assert_eq!(node.ledger.height, 2);
        assert!(std::fs::read(&path).unwrap() == before, "a read-only load never changes the file");
        let _ = std::fs::remove_file(&path);
    }

    /// The whole frames of a clean log.
    fn frames_of(path: &Path) -> Vec<Vec<u8>> {
        let bytes = std::fs::read(path).unwrap();
        let (mut out, mut at) = (Vec::new(), 0usize);
        while at < bytes.len() {
            let len = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            out.push(bytes[at..at + 4 + len].to_vec());
            at += 4 + len;
        }
        out
    }

    #[test]
    fn good_records_after_a_broken_one_are_refused_not_cut_away() {
        // What an older node left after a crash: a torn record, then whole blocks appended after it.
        let spec = ChainSpec::dev();
        let built = tmp("good-after-bad-src");
        build_and_persist(&built, &spec, 4);
        let f = frames_of(&built);
        let path = tmp("good-after-bad");
        let torn = [&999u32.to_be_bytes()[..], b"{\"height\":3"].concat();
        std::fs::write(&path, [f[0].clone(), f[1].clone(), torn, f[2].clone(), f[3].clone()].concat()).unwrap();
        let before = std::fs::read(&path).unwrap();
        let err = load_blocks(&path, &spec).err().expect("damage is refused, not read as a 2-block chain");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let at = f[0].len() + f[1].len();
        assert!(err.to_string().contains(&format!("damaged at byte {at}, after 2 good blocks")), "{err}");
        assert!(std::fs::read(&path).unwrap() == before, "the file is untouched");
        let _ = (std::fs::remove_file(&path), std::fs::remove_file(&built));
    }

    /// The files [`load_for_append`] set aside next to `log`.
    fn aside_of(log: &Path) -> Vec<PathBuf> {
        let prefix = format!("{}.torn-at-", log.file_name().unwrap().to_string_lossy());
        std::fs::read_dir(log.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&prefix)))
            .collect()
    }

    #[test]
    fn every_shape_a_crash_leaves_is_set_aside_and_the_next_block_lands_on_a_boundary() {
        let spec = ChainSpec::dev();
        let built = tmp("shapes-src");
        build_and_persist(&built, &spec, 3);
        let f = frames_of(&built);
        let whole = [f[0].clone(), f[1].clone()].concat();
        let third = &f[2];
        let block3 = decode_block(&third[4..]).unwrap();
        let shapes: Vec<(&str, Vec<u8>)> = vec![
            ("one byte of a length", third[..1].to_vec()),
            ("three bytes of a length", third[..3].to_vec()),
            ("a length and no body", third[..4].to_vec()),
            ("a length and half a body", third[..third.len() / 2].to_vec()),
            ("all but the last byte", third[..third.len() - 1].to_vec()),
            ("zeros a power cut left", vec![0u8; 4096]),
        ];
        for (what, tail) in shapes {
            let path = tmp("shape");
            std::fs::write(&path, [whole.clone(), tail.clone()].concat()).unwrap();
            // read-only: the whole blocks, and the file left alone
            assert_eq!(load_blocks(&path, &spec).expect(what).1.len(), 2, "{what}");
            assert!(aside_of(&path).is_empty(), "{what}: a read-only load sets nothing aside");
            // appending boot: set aside, cut back, flushed
            let (node, blocks, offsets) = load_for_append(&path, &spec).expect(what);
            assert_eq!((blocks.len(), node.ledger.height, offsets.len()), (2, 2, 2), "{what}");
            assert!(std::fs::read(&path).unwrap() == whole, "{what}: cut back to the last whole record");
            let aside = aside_of(&path);
            assert_eq!(aside.len(), 1, "{what}");
            assert!(std::fs::read(&aside[0]).unwrap() == tail, "{what}: the set-aside file holds the bytes");
            let name = aside[0].file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.contains(&format!(".torn-at-{}-", whole.len())), "{what}: {name}");
            assert_eq!(append_block_at(&path, &block3).unwrap(), whole.len() as u64, "{what}: on the boundary");
            let (_, blocks, _) = load_for_append(&path, &spec).unwrap();
            assert_eq!(blocks.len(), 3, "{what}: the new block survives the next boot");
            assert_eq!(aside_of(&path).len(), 1, "{what}: a clean boot sets nothing more aside");
            for a in aside {
                let _ = std::fs::remove_file(a);
            }
            let _ = std::fs::remove_file(&path);
        }
        let _ = std::fs::remove_file(&built);
    }

    /// A read-only load can run while the writer appends (the peers seed reads the log on its own
    /// thread). The tail is judged on the bytes the replay saw, so a record the writer finished after the
    /// replay began is a torn tail to this reader, never "a whole record after a broken one".
    #[test]
    fn a_record_finished_during_a_read_only_replay_is_a_torn_tail_not_damage() {
        let spec = ChainSpec::dev();
        let path = tmp("race");
        build_and_persist(&path, &spec, 3);
        let f = frames_of(&path);
        let at = (f[0].len() + f[1].len()) as u64;
        let seen = at + (f[2].len() / 2) as u64; // the replay saw half of record 3; the file now holds all of it
        let tail = torn_tail(&path, at, seen, 2, "the file ends inside a record").expect("a torn tail to this reader");
        assert!(tail == f[2][..f[2].len() / 2], "only the bytes the replay saw");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_clean_log_is_not_opened_for_writing_by_the_appending_boot() {
        let spec = ChainSpec::dev();
        let path = tmp("clean-append");
        build_and_persist(&path, &spec, 2);
        let before = (std::fs::read(&path).unwrap(), std::fs::metadata(&path).unwrap().modified().unwrap());
        let (_, blocks, _) = load_for_append(&path, &spec).unwrap();
        assert_eq!(blocks.len(), 2);
        let after = (std::fs::read(&path).unwrap(), std::fs::metadata(&path).unwrap().modified().unwrap());
        assert!(after == before, "same bytes, same modification time");
        assert!(aside_of(&path).is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
