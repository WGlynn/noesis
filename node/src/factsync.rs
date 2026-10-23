//! Fact sync over the network: two independently run nodes swap SIGNED contribution facts as framed
//! TCP messages, each records what it applied as a trace, and the trace replays to the same hash in
//! any order (the day-30 devnet bar, `docs/DESIGN-layer-map.md` section 7).
//!
//! Before this, reconciliation only ran as a function call inside one test process
//! (`reconcile::merge`). Here the only thing two nodes share is the bytes that cross the wire.
//!
//! A record is accepted iff (1) it decodes, (2) its ContentId re-derives from its fields, and (3) it
//! carries an XMSS signature by its author (`noesis_core::xmss`, the wallet's scheme; the author is
//! the 32-byte wallet address, as `type_script.args` is on the live chain) over [`fact_digest`] of
//! that ContentId. The ContentId commits to the author, the data and the parent, so the signature
//! vouches for all three: a forger can neither claim someone else's id, nor put new words in their
//! mouth, nor give their real work a false parent. The verdict on a frame depends on that frame
//! alone, so a forged copy that arrives before the genuine record cannot block it (Boardy's denial
//! case, 2026-10-22): rejection records nothing.
//!
//! HONEST SCOPE: CONSENSUS-ISOLATED SHADOW, like `reconcile`. Nothing is written to `Ledger`,
//! validation, finality or `state_digest`. One exchange per connection; each side sends everything
//! it holds. Only the demo chains can sign today: the live chain drops the wallet's signature after
//! the submit check, and the wallet signs `xmss::contribution_digest` (address, index, data), which
//! does not cover the parent, not [`fact_digest`]. The demo keys derive from public names, so anyone
//! can sign as a demo author. XMSS leaves are one-time and nothing here detects one leaf signing two
//! different facts. A Lamport signature is 16 KiB, so a signed record is about 16.8 KB on the wire (measured).
//!
//! Wire, over `net` framing: each frame is `[tag][payload]`, a [`FACT`] frame per record, then [`DONE`].
//! FACT payload: record, then signature. Record: cid (32) | data hash (32) | parent tag (0 = root,
//! 1 = a 32-byte cid follows) | author length (u32 LE) | author. Signature: leaf index (u32 LE) | OTS
//! root (32) | auth path (`xmss::H` x 32) | OTS signature length (u32 LE) | OTS signature.
//! Trace file: `net` frames, each `[origin][frame]` with origin [`OWN`] or [`WIRE`] (received).

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read};
use std::net::TcpStream;
use std::path::Path;

use noesis_core::xmss;

use crate::net::{read_frame, write_frame, IO_TIMEOUT};
use crate::reconcile::{content_id, ContentId, FactRecord, FactSet};

pub const FACT: u8 = 1;
pub const DONE: u8 = 2;
pub const OWN: u8 = 0;
pub const WIRE: u8 = 1;

/// Upper bound on frames accepted from one peer in one exchange (the DoS bound on a lying peer).
pub const MAX_FRAMES: usize = 1 << 20;

/// The message an author signs to vouch for a fact: blake2b-256 over its ContentId, with its own
/// personalization so a fact signature can't be replayed as any other kind of signature.
pub fn fact_digest(cid: &ContentId) -> [u8; 32] {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-fsig-v1\0\0").build();
    h.update(cid);
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

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

fn encode_signature(s: &xmss::Signature, out: &mut Vec<u8>) {
    out.extend_from_slice(&s.index.to_le_bytes());
    out.extend_from_slice(&s.ots_root);
    for a in &s.auth {
        out.extend_from_slice(a);
    }
    out.extend_from_slice(&(s.ots_sig.len() as u32).to_le_bytes());
    out.extend_from_slice(&s.ots_sig);
}

/// Splits `n` bytes off the front of `b`, or `None` if it is too short.
fn take<'a>(b: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
    if b.len() < n {
        return None;
    }
    let (head, rest) = b.split_at(n);
    *b = rest;
    Some(head)
}

fn take32(b: &mut &[u8]) -> Option<[u8; 32]> {
    take(b, 32)?.try_into().ok()
}

fn take_u32(b: &mut &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(take(b, 4)?.try_into().ok()?))
}

fn take_record(b: &mut &[u8]) -> Option<FactRecord> {
    let cid = take32(b)?;
    let data_hash = take32(b)?;
    let parent = match take(b, 1)?[0] {
        0 => None,
        1 => Some(take32(b)?),
        _ => return None,
    };
    let len = take_u32(b)? as usize;
    let author = take(b, len)?.to_vec();
    Some(FactRecord { cid, author, data_hash, parent })
}

fn take_signature(b: &mut &[u8]) -> Option<xmss::Signature> {
    let index = take_u32(b)?;
    let ots_root = take32(b)?;
    let mut auth = [[0u8; 32]; xmss::H];
    for a in auth.iter_mut() {
        *a = take32(b)?;
    }
    let len = take_u32(b)? as usize;
    let ots_sig = take(b, len)?.to_vec();
    Some(xmss::Signature { index, ots_root, auth, ots_sig })
}

/// Strict decode of a FACT payload: `None` on a short buffer, an unknown parent tag, a missing
/// signature, or trailing bytes. Decoding checks nothing else; [`verify_fact`] does.
pub fn decode_signed(b: &[u8]) -> Option<(FactRecord, xmss::Signature)> {
    let mut b = b;
    let r = take_record(&mut b)?;
    let s = take_signature(&mut b)?;
    b.is_empty().then_some((r, s))
}

/// True iff the record's ContentId re-derives from its fields and `sig` is its author's signature
/// over that ContentId. The author must be a 32-byte wallet address.
pub fn verify_fact(r: &FactRecord, sig: &xmss::Signature) -> bool {
    let Ok(address) = <[u8; 32]>::try_from(r.author.as_slice()) else {
        return false;
    };
    r.cid == content_id(&r.author, &r.data_hash, r.parent.as_ref())
        && xmss::verify(&address, &fact_digest(&r.cid), sig)
}

/// A FACT frame for `r` carrying `sig` (which may or may not be valid: that is the receiver's call).
pub fn fact_frame(r: &FactRecord, sig: &xmss::Signature) -> Vec<u8> {
    let mut f = vec![FACT];
    f.extend_from_slice(&encode_record(r));
    encode_signature(sig, &mut f);
    f
}

/// Sign `r` with leaf `index` of the key `master` and frame it. One-time leaves: never sign two
/// different facts with one index.
pub fn signed_frame(master: &[u8; 32], index: u32, r: &FactRecord) -> Vec<u8> {
    fact_frame(r, &xmss::sign(master, index, &fact_digest(&r.cid)))
}

/// How many frames were accepted (duplicates included: re-inserting a fact is a no-op) and rejected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub accepted: usize,
    pub rejected: usize,
}

/// Apply frames to an empty fact set. A [`FACT`] frame is accepted iff it decodes and
/// [`verify_fact`] holds; every other frame is rejected. Each verdict depends on that frame alone,
/// never on what arrived before it, a rejected frame leaves no trace, and accepted records merge by
/// union, so neither the set nor the counts can depend on arrival order.
pub fn apply_frames<'a>(frames: impl IntoIterator<Item = &'a [u8]>) -> (FactSet, Applied) {
    let mut set = FactSet::default();
    let mut n = Applied::default();
    for f in frames {
        let ok = match f.split_first() {
            Some((&FACT, body)) => match decode_signed(body) {
                Some((r, sig)) if verify_fact(&r, &sig) => set.insert(r),
                _ => false,
            },
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

/// One exchange over an open connection: send `outgoing` then `DONE`, while reading the peer's frames
/// until its `DONE`. Sending runs on its own thread so two peers that both send first cannot deadlock
/// on full socket buffers. Returns the received frames in arrival order.
pub fn exchange(stream: TcpStream, outgoing: Vec<Vec<u8>>) -> io::Result<Vec<Vec<u8>>> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
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

/// The trace of one exchange: this node's own frames, then the received frames in arrival order.
pub fn trace_entries(own: &[Vec<u8>], received: &[Vec<u8>]) -> Vec<(u8, Vec<u8>)> {
    let mut t: Vec<(u8, Vec<u8>)> = own.iter().map(|f| (OWN, f.clone())).collect();
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
        if r.read(&mut probe)? == 0 {
            return Ok(out);
        }
        let e = read_frame(&mut io::Cursor::new(probe).chain(&mut r))?;
        let (origin, frame) =
            e.split_first().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty trace entry"))?;
        out.push((*origin, frame.to_vec()));
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

/// A demo key: the XMSS master seed derived from a public name. Anyone can rebuild it, which is the
/// point for a devnet demo and why it must never hold anything real.
pub fn demo_master(name: &str) -> [u8; 32] {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-demo-key\0").build();
    h.update(name.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

/// Frames a misbehaving peer might send, built from `genuine` (one of the sender's own signed frames,
/// for record r by author A). Sent BEFORE the genuine frames, so the first one is Boardy's case: a
/// forged copy of r arriving ahead of the real one. In order:
/// 1. r signed by mallory instead of A (rejected; the genuine r that follows must still get in);
/// 2. a new record "by A" that A never wrote, signed by mallory (rejected);
/// 3. r's ContentId claimed under mallory's key, with A's signature (rejected: the id doesn't re-derive);
/// 4. r with no signature (rejected); 5. a truncated frame; 6. an unknown tag; 7. an empty frame (rejected);
/// 8. the genuine frame again (accepted: a repeat is a no-op).
pub fn junk_frames(genuine: &[u8]) -> Vec<Vec<u8>> {
    let (r, sig) = decode_signed(&genuine[1..]).expect("junk is built from a well-formed frame");
    let mallory = demo_master("mallory");
    let mallory_addr = xmss::keygen_address(&mallory);
    let words = FactRecord::new(&r.author, b"words this author never wrote", None);
    let mut stolen = r.clone();
    stolen.author = mallory_addr.to_vec();
    let mut unsigned = vec![FACT];
    unsigned.extend_from_slice(&encode_record(&r));
    vec![
        signed_frame(&mallory, 0, &r),
        signed_frame(&mallory, 1, &words),
        fact_frame(&stolen, &sig),
        unsigned,
        genuine[..20].to_vec(),
        vec![0xFF, 1, 2, 3],
        Vec::new(),
        genuine.to_vec(),
    ]
}

/// Lowercase hex, for printing digests.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Small deterministic chains for the devnet run, as signed frames. Each chain is built through the
/// full node pipeline (submit, mine, finality gate, apply) on its own fresh genesis, with demo wallet
/// addresses as authors, so two machines need no shared files. Each author signs their works with
/// fixed leaf indexes (the same fact always gets the same signature, so re-signing is safe).
/// `a`: alice's two works, the second built on the first. `b`: bob's work plus an overlap record
/// naming alice's first work by ContentId. `c`: carol's work, alice's first work recorded again (the
/// same non-rival fact on a second chain), and carol claiming alice's second text under her own key
/// (a rival claim, which merge flags and does not resolve).
pub fn demo_frames(name: &str) -> Option<Vec<Vec<u8>>> {
    use crate::chainspec::ChainSpec;
    use crate::commit_order::Committed;
    use crate::reconcile::data_hash;
    use crate::{Cell, Script};

    const A1: &[u8] = b"alice: the quick brown fox";
    const A2: &[u8] = b"alice: the fox jumps over the lazy dog";
    const B1: &[u8] = b"bob: notes on winter light";
    const OV: &[u8] = b"bob: a proof that extends alice's fox";
    const C1: &[u8] = b"carol: tide tables for the north shore";
    // Each author's works in leaf-index order.
    const WORKS: [(&str, &[&[u8]]); 3] = [("alice", &[A1, A2]), ("bob", &[B1, OV]), ("carol", &[C1, A2])];

    let authors: &[&str] = match name {
        "a" => &["alice"],
        "b" => &["bob", "alice"],
        "c" => &["carol", "alice"],
        _ => return None,
    };
    let keys: Vec<(&str, [u8; 32], [u8; 32])> = authors
        .iter()
        .map(|&n| {
            let m = demo_master(n);
            (n, m, xmss::keygen_address(&m))
        })
        .collect();
    let addr = |n: &str| keys.iter().find(|k| k.0 == n).expect("demo author has a key").2;

    let cell = |id: u64, author: &str, parent: Option<u64>, ts: u64, data: &[u8]| Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: addr(author).to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: addr(author).to_vec() },
        parent,
        timestamp: ts,
        data: data.to_vec(),
    };
    let chain = |blocks: Vec<Vec<Cell>>| -> FactSet {
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
    };

    let set = match name {
        "a" => chain(vec![vec![cell(1, "alice", None, 1, A1)], vec![cell(2, "alice", Some(1), 2, A2)]]),
        "b" => {
            let mut set = chain(vec![vec![cell(1, "bob", None, 1, B1)]]);
            let a1 = content_id(&addr("alice"), &data_hash(A1), None);
            assert!(set.insert(FactRecord::new(&addr("bob"), OV, Some(a1))));
            set
        }
        _ => chain(vec![
            vec![cell(10, "carol", None, 5, C1)],
            vec![cell(11, "alice", None, 9, A1), cell(12, "carol", None, 9, A2)],
        ]),
    };
    let frames = set
        .records()
        .map(|r| {
            let (n, master, _) = keys.iter().find(|k| k.2[..] == r.author[..]).expect("record by a demo author");
            let works = WORKS.iter().find(|w| w.0 == *n).expect("demo author has works").1;
            let index = works.iter().position(|d| data_hash(d) == r.data_hash).expect("a listed work");
            signed_frame(master, index as u32, r)
        })
        .collect();
    Some(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::merge;
    use std::sync::OnceLock;

    /// Demo frames are slow to build in a debug test (every XMSS signature rebuilds a 256-leaf tree),
    /// so the tests share one copy of each.
    fn demo(name: &str) -> &'static [Vec<u8>] {
        static A: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
        static B: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
        static C: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
        let cell = match name {
            "a" => &A,
            "b" => &B,
            _ => &C,
        };
        cell.get_or_init(|| demo_frames(name).unwrap())
    }

    fn set_of(frames: &[Vec<u8>]) -> FactSet {
        let (s, n) = apply_frames(frames.iter().map(Vec::as_slice));
        assert_eq!(n.rejected, 0, "demo frames are all genuine");
        s
    }

    #[test]
    fn signed_frame_codec_round_trips_and_rejects_malformed_bytes() {
        let f = &demo("a")[0];
        let (r, sig) = decode_signed(&f[1..]).expect("decodes");
        assert!(verify_fact(&r, &sig));
        assert_eq!(fact_frame(&r, &sig), *f, "re-encodes byte for byte");
        assert!(decode_signed(&f[1..f.len() - 1]).is_none(), "truncated");
        let mut long = f[1..].to_vec();
        long.push(0);
        assert!(decode_signed(&long).is_none(), "trailing byte");
        let mut bad_tag = f[1..].to_vec();
        bad_tag[64] = 2;
        assert!(decode_signed(&bad_tag).is_none(), "unknown parent tag");
    }

    #[test]
    fn a_forged_copy_arriving_first_does_not_block_the_genuine_record() {
        let genuine = &demo("a")[0];
        let forged = &junk_frames(genuine)[0];
        for frames in [[forged, genuine], [genuine, forged]] {
            let (set, n) = apply_frames(frames.iter().map(|f| f.as_slice()));
            assert_eq!(n, Applied { accepted: 1, rejected: 1 });
            assert_eq!(set.digest(), set_of(&demo("a")[..1]).digest(), "the genuine record got in");
        }
    }

    #[test]
    fn every_kind_of_junk_is_rejected_except_the_repeat() {
        let junk = junk_frames(&demo("b")[0]);
        let verdicts: Vec<bool> =
            junk.iter().map(|f| apply_frames(std::iter::once(f.as_slice())).1.accepted == 1).collect();
        assert_eq!(verdicts, [false, false, false, false, false, false, false, true]);
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
        let (a, b) = (demo("a"), demo("b"));
        let mut received = junk_frames(&b[0]);
        received.extend(b.iter().cloned());
        let trace = trace_entries(a, &received);
        let (live, live_n) = replay(&trace, None);
        assert_eq!(live.digest(), merge(&set_of(a), &set_of(b)).digest(), "trace order = the in-process merge");
        assert_eq!(live_n.rejected, 7);
        for seed in 0..64u64 {
            let (s, n) = replay(&trace, Some(seed));
            assert_eq!(s.digest(), live.digest(), "seed {seed}");
            assert_eq!(n, live_n, "seed {seed}: verdicts don't depend on order");
        }
    }

    #[test]
    fn demo_c_flags_its_rival_claim_after_merge() {
        let merged = merge(&set_of(demo("a")), &set_of(demo("c")));
        let rivals = merged.rival_claims();
        assert_eq!(rivals.len(), 1);
        let mut want = vec![xmss::keygen_address(&demo_master("alice")).to_vec(), xmss::keygen_address(&demo_master("carol")).to_vec()];
        want.sort();
        assert_eq!(rivals[0].1, want);
    }

    #[test]
    fn seeds_parse_with_comments_and_reject_garbage() {
        assert_eq!(parse_seeds("# list\n1\n\n 42 # the answer\n"), Ok(vec![1, 42]));
        assert!(parse_seeds("1\nx\n").is_err());
    }
}
