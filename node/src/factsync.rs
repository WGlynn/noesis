//! Fact sync over the network: two independently run nodes swap contribution facts as framed TCP
//! messages, each records what it applied as a trace, and the trace replays to the same hash in any
//! order (the day-30 devnet bar, `docs/DESIGN-layer-map.md` section 7).
//!
//! Before this, reconciliation only ran as a function call inside one test process
//! (`reconcile::merge`). Here the only thing two nodes share is the bytes that cross the wire.
//!
//! HONEST SCOPE: CONSENSUS-ISOLATED SHADOW, like `reconcile`. Facts are read from a chain's cells and
//! nothing is written back to `Ledger`, validation, finality or `state_digest`. One exchange per
//! connection, and both sides send everything they hold (no incremental gossip, no catch-up, no set
//! reconciliation). No signatures: a record is accepted iff its ContentId re-derives from its fields,
//! so a record claiming a ContentId it does not hash to is rejected, but a record forged under someone
//! else's author key is NOT (that needs the wallet's signature to travel with the record; not built).
//!
//! Wire, over `net` framing: each frame is `[tag][payload]`, a [`FACT`] frame per record, then [`DONE`].
//! Record encoding: cid (32) | data hash (32) | parent tag (0 = root, 1 = a 32-byte cid follows) |
//! author length (u32 LE) | author. Trace file: `net` frames, each `[origin][frame]` with origin
//! [`OWN`] (this node's fact) or [`WIRE`] (received).

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read};
use std::net::TcpStream;
use std::path::Path;

use crate::net::{read_frame, write_frame, IO_TIMEOUT};
use crate::reconcile::{ContentId, FactRecord, FactSet};

pub const FACT: u8 = 1;
pub const DONE: u8 = 2;
pub const OWN: u8 = 0;
pub const WIRE: u8 = 1;

/// Upper bound on frames accepted from one peer in one exchange (the DoS bound on a lying peer).
pub const MAX_FRAMES: usize = 1 << 20;

pub fn encode_record(r: &FactRecord) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + 32 + 33 + 4 + r.author.len());
    out.extend_from_slice(&r.cid);
    out.extend_from_slice(&r.data_hash);
    match &r.parent {
        None => out.push(0),
        Some(p) => {
            out.push(1);
            out.extend_from_slice(p);
        }
    }
    out.extend_from_slice(&(r.author.len() as u32).to_le_bytes());
    out.extend_from_slice(&r.author);
    out
}

/// Strict decode: `None` on a short buffer, an unknown parent tag, or trailing bytes. Decoding does
/// not check the ContentId; [`FactSet::insert`] does.
pub fn decode_record(b: &[u8]) -> Option<FactRecord> {
    fn take<'a>(b: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
        if b.len() < n {
            return None;
        }
        let (head, rest) = b.split_at(n);
        *b = rest;
        Some(head)
    }
    let mut b = b;
    let cid: ContentId = take(&mut b, 32)?.try_into().ok()?;
    let data_hash: [u8; 32] = take(&mut b, 32)?.try_into().ok()?;
    let parent = match take(&mut b, 1)?[0] {
        0 => None,
        1 => Some(take(&mut b, 32)?.try_into().ok()?),
        _ => return None,
    };
    let len = u32::from_le_bytes(take(&mut b, 4)?.try_into().ok()?) as usize;
    let author = take(&mut b, len)?.to_vec();
    if !b.is_empty() {
        return None;
    }
    Some(FactRecord { cid, author, data_hash, parent })
}

pub fn fact_frame(r: &FactRecord) -> Vec<u8> {
    let mut f = vec![FACT];
    f.extend_from_slice(&encode_record(r));
    f
}

/// How many frames were accepted (duplicates included: re-inserting a fact is a no-op) and rejected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub accepted: usize,
    pub rejected: usize,
}

/// Apply frames to an empty fact set. A [`FACT`] frame is accepted iff its record decodes and its
/// ContentId re-derives; every other frame is rejected. Each verdict depends on that frame alone,
/// never on what arrived before it, and accepted records merge by union, so neither the set nor the
/// counts can depend on arrival order.
pub fn apply_frames<'a>(frames: impl IntoIterator<Item = &'a [u8]>) -> (FactSet, Applied) {
    let mut set = FactSet::default();
    let mut n = Applied::default();
    for f in frames {
        let ok = match f.split_first() {
            Some((&FACT, body)) => decode_record(body).is_some_and(|r| set.insert(r)),
            _ => false,
        };
        if ok {
            n.accepted += 1;
        } else {
            n.rejected += 1;
        }
    }
    (set, n)
}

/// One exchange over an open connection: send every own fact plus `extra` frames, then `DONE`, while
/// reading the peer's frames until its `DONE`. Sending runs on its own thread so two peers that both
/// send first cannot deadlock on full socket buffers. Returns the received frames in arrival order.
pub fn exchange(stream: TcpStream, own: &FactSet, extra: &[Vec<u8>]) -> io::Result<Vec<Vec<u8>>> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let mut outgoing: Vec<Vec<u8>> = own.records().map(fact_frame).collect();
    outgoing.extend(extra.iter().cloned());
    let mut w = stream.try_clone()?;
    let sender = std::thread::spawn(move || -> io::Result<()> {
        for f in &outgoing {
            write_frame(&mut w, f)?;
        }
        write_frame(&mut w, &[DONE])
    });
    let mut r = stream;
    let mut received = Vec::new();
    let read_result = loop {
        match read_frame(&mut r) {
            Ok(f) if f == [DONE] => break Ok(()),
            Ok(_) if received.len() >= MAX_FRAMES => {
                break Err(io::Error::new(io::ErrorKind::InvalidData, "peer exceeded MAX_FRAMES"))
            }
            Ok(f) => received.push(f),
            Err(e) => break Err(e),
        }
    };
    let send_result = sender
        .join()
        .unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::Other, "sender thread panicked")));
    read_result?;
    send_result?;
    Ok(received)
}

/// The trace of one exchange: this node's own facts, then the received frames in arrival order.
pub fn trace_entries(own: &FactSet, received: &[Vec<u8>]) -> Vec<(u8, Vec<u8>)> {
    let mut t: Vec<(u8, Vec<u8>)> = own.records().map(|r| (OWN, fact_frame(r))).collect();
    t.extend(received.iter().map(|f| (WIRE, f.clone())));
    t
}

pub fn write_trace(path: &Path, entries: &[(u8, Vec<u8>)]) -> io::Result<()> {
    let mut w = BufWriter::new(File::create(path)?);
    for (origin, frame) in entries {
        let mut e = Vec::with_capacity(1 + frame.len());
        e.push(*origin);
        e.extend_from_slice(frame);
        write_frame(&mut w, &e)?;
    }
    Ok(())
}

/// Read a trace written by [`write_trace`]. A clean EOF ends it; an entry with no origin byte is an error.
pub fn read_trace(path: &Path) -> io::Result<Vec<(u8, Vec<u8>)>> {
    let mut r = BufReader::new(File::open(path)?);
    let mut out = Vec::new();
    loop {
        let mut probe = [0u8; 1];
        match r.read(&mut probe)? {
            0 => return Ok(out),
            _ => {
                let mut chained = io::Cursor::new(probe).chain(&mut r);
                let e = read_frame(&mut chained)?;
                let (origin, frame) = e
                    .split_first()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty trace entry"))?;
                out.push((*origin, frame.to_vec()));
            }
        }
    }
}

/// SplitMix64 (Steele, Lea and Flood, 2014): a few lines in any language, so anyone can rerun a seed.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A permutation of `0..n` from `seed` (Fisher-Yates, drawing `j` in `0..=i` as `next % (i + 1)`).
/// The modulo bias doesn't matter: this picks test orders and secures nothing.
pub fn shuffled_order(n: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    let mut state = seed;
    for i in (1..n).rev() {
        let j = (splitmix64(&mut state) % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

/// Replay a trace into an empty set, own and received frames alike, in trace order (`None`) or in
/// the order [`shuffled_order`] gives for `seed`.
pub fn replay(entries: &[(u8, Vec<u8>)], seed: Option<u64>) -> (FactSet, Applied) {
    let order: Vec<usize> = match seed {
        None => (0..entries.len()).collect(),
        Some(s) => shuffled_order(entries.len(), s),
    };
    apply_frames(order.iter().map(|&i| entries[i].1.as_slice()))
}

/// One decimal `u64` per line; blank lines and `#` comments are skipped.
pub fn parse_seeds(text: &str) -> Result<Vec<u64>, String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.parse::<u64>().map_err(|e| format!("bad seed {l:?}: {e}")))
        .collect()
}

/// Frames a misbehaving peer might send, built from `r` (one of the sender's own facts): `r` again
/// (accepted: a repeat is a no-op), `r`'s ContentId claimed under another author key (rejected: the
/// id doesn't re-derive), a truncated `FACT` frame, an unknown tag, and an empty frame (all rejected).
pub fn junk_frames(r: &FactRecord) -> Vec<Vec<u8>> {
    let mut stolen = r.clone();
    stolen.author = b"mallory".to_vec();
    let full = fact_frame(r);
    vec![full.clone(), fact_frame(&stolen), full[..20].to_vec(), vec![0xFF, 1, 2, 3], Vec::new()]
}

/// Lowercase hex, for printing digests.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Small deterministic chains for the devnet run, each built through the full node pipeline (submit,
/// mine, finality gate, apply) on its own fresh genesis, so two machines need no shared files.
/// `a`: alice's two works, the second built on the first. `b`: bob's work plus an overlap record
/// naming alice's first work by ContentId. `c`: carol's work, alice's first work recorded again (the
/// same non-rival fact on a second chain), and carol claiming alice's second text under her own key
/// (a rival claim, which merge flags and does not resolve).
pub fn demo_facts(name: &str) -> Option<FactSet> {
    use crate::chainspec::ChainSpec;
    use crate::commit_order::Committed;
    use crate::reconcile::{content_id, data_hash};
    use crate::{Cell, Script};

    const A1: &[u8] = b"alice: the quick brown fox";
    const A2: &[u8] = b"alice: the fox jumps over the lazy dog";
    const B1: &[u8] = b"bob: notes on winter light";
    const OV: &[u8] = b"bob: a proof that extends alice's fox";
    const C1: &[u8] = b"carol: tide tables for the north shore";

    fn cell(id: u64, author: &[u8], parent: Option<u64>, ts: u64, data: &[u8]) -> Cell {
        Cell {
            id,
            lock: Script { code_hash: [0u8; 32], args: author[..2].to_vec() },
            type_script: Script { code_hash: [1u8; 32], args: author.to_vec() },
            parent,
            timestamp: ts,
            data: data.to_vec(),
        }
    }
    fn chain(blocks: Vec<Vec<Cell>>) -> FactSet {
        let spec = ChainSpec::dev();
        let (mut node, _keys) = spec.genesis_node();
        for (i, cells) in blocks.into_iter().enumerate() {
            let h = node.ledger.height + 1;
            for (j, c) in cells.into_iter().enumerate() {
                let secret = [(i * 16 + j + 1) as u8; 32];
                assert!(node.submit(c, Committed { height: h, secret }), "demo cell rejected");
            }
            assert!(spec.produce_block(&mut node).is_some(), "demo block did not finalize");
        }
        FactSet::from_cells(&node.ledger.cells).expect("demo parents are committed first")
    }

    match name {
        "a" => Some(chain(vec![
            vec![cell(1, b"alice", None, 1, A1)],
            vec![cell(2, b"alice", Some(1), 2, A2)],
        ])),
        "b" => {
            let mut set = chain(vec![vec![cell(1, b"bob", None, 1, B1)]]);
            let a1 = content_id(b"alice", &data_hash(A1), None);
            assert!(set.insert(FactRecord::new(b"bob", OV, Some(a1))));
            Some(set)
        }
        "c" => Some(chain(vec![
            vec![cell(10, b"carol", None, 5, C1)],
            vec![cell(11, b"alice", None, 9, A1), cell(12, b"carol", None, 9, A2)],
        ])),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::merge;

    fn sample() -> Vec<FactRecord> {
        let a = FactRecord::new(b"alice", b"one", None);
        let b = FactRecord::new(b"bob", b"two", Some(a.cid));
        let c = FactRecord::new(b"", b"", Some([7u8; 32]));
        vec![a, b, c]
    }

    #[test]
    fn record_codec_round_trips_and_rejects_malformed_bytes() {
        for r in sample() {
            let e = encode_record(&r);
            assert_eq!(decode_record(&e), Some(r.clone()));
            assert_eq!(decode_record(&e[..e.len() - 1]), None, "truncated");
            let mut long = e.clone();
            long.push(0);
            assert_eq!(decode_record(&long), None, "trailing byte");
        }
        let mut bad_tag = encode_record(&sample()[0]);
        bad_tag[64] = 2;
        assert_eq!(decode_record(&bad_tag), None, "unknown parent tag");
    }

    #[test]
    fn shuffled_order_is_a_deterministic_permutation() {
        for n in [0usize, 1, 2, 7, 50] {
            for seed in 0..20u64 {
                let o = shuffled_order(n, seed);
                assert_eq!(o, shuffled_order(n, seed), "same seed, same order");
                let mut sorted = o.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..n).collect::<Vec<_>>(), "a permutation");
            }
        }
        assert_ne!(shuffled_order(50, 1), shuffled_order(50, 2), "different seeds move things");
    }

    #[test]
    fn every_shuffle_of_a_trace_with_junk_lands_on_the_union() {
        let a = demo_facts("a").unwrap();
        let b = demo_facts("b").unwrap();
        let mut received: Vec<Vec<u8>> = b.records().map(fact_frame).collect();
        received.extend(junk_frames(b.records().next().unwrap()));
        let trace = trace_entries(&a, &received);
        let (live, live_n) = replay(&trace, None);
        assert_eq!(live.digest(), merge(&a, &b).digest(), "trace order = the in-process merge");
        assert_eq!(live_n.rejected, 4, "stolen id, truncated, unknown tag, empty");
        for seed in 0..64u64 {
            let (s, n) = replay(&trace, Some(seed));
            assert_eq!(s.digest(), live.digest(), "seed {seed}");
            assert_eq!(n, live_n, "seed {seed}: verdicts don't depend on order");
        }
    }

    #[test]
    fn demo_c_flags_its_rival_claim_after_merge() {
        let merged = merge(&demo_facts("a").unwrap(), &demo_facts("c").unwrap());
        let rivals = merged.rival_claims();
        assert_eq!(rivals.len(), 1);
        assert_eq!(rivals[0].1, vec![b"alice".to_vec(), b"carol".to_vec()]);
    }

    #[test]
    fn seeds_parse_with_comments_and_reject_garbage() {
        assert_eq!(parse_seeds("# list\n1\n\n 42 # the answer\n"), Ok(vec![1, 42]));
        assert!(parse_seeds("1\nx\n").is_err());
    }
}
