//! T1 slice-4 — sync: a joining node pulls a peer's block log and replays it to converge.
//!
//! THE join. A fresh (genesis) node connects to a seed, requests its block log, receives the blocks
//! as frames, decodes each via slice-1 `wire::decode_block`, applies them via `Node::apply`, and
//! converges to a byte-identical `state_digest`. State is derived by replaying the canonical blocks
//! (Bitcoin-style), so the joiner trusts the *rules*, not the peer — a lying seed can only send blocks
//! that `apply` accepts, and the digest either matches or it doesn't.
//!
//! Wire protocol (over slice-2 framed transport): each frame is `[tag byte][payload]`.
//!   * joiner -> seed: `[GET_BLOCKS]`
//!   * seed -> joiner: `[BLOCK][encode_block(b)]` per block, then `[DONE]`
//!
//! HONEST SCOPE: a linear "give me your whole log from genesis" full-sync — no headers-first, no
//! range requests, no fork choice (there is one canonical log here). Those are later optimizations;
//! this is the mechanism that makes a node *join*.

use std::io;

use crate::net::Peer;
use crate::runtime::{Block, Node};
use crate::wire::{decode_block, encode_block, WireError};

const TAG_GET_BLOCKS: u8 = 1;
const TAG_BLOCK: u8 = 2;
const TAG_DONE: u8 = 3;

/// Error while syncing: a transport failure, a malformed block, or a protocol violation.
#[derive(Debug)]
pub enum SyncError {
    Io(io::Error),
    Wire(WireError),
    Protocol(String),
}
impl From<io::Error> for SyncError {
    fn from(e: io::Error) -> Self {
        SyncError::Io(e)
    }
}
impl From<WireError> for SyncError {
    fn from(e: WireError) -> Self {
        SyncError::Wire(e)
    }
}
impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Io(e) => write!(f, "sync io error: {e}"),
            SyncError::Wire(e) => write!(f, "sync decode error: {e}"),
            SyncError::Protocol(m) => write!(f, "sync protocol error: {m}"),
        }
    }
}
impl std::error::Error for SyncError {}

/// SEED side: answer one joiner. Reads its `GET_BLOCKS` request, streams every block in order, then a
/// `DONE` marker. `blocks` is the seed's canonical block log (e.g. from `wire::BlockLog::load`).
pub fn serve(peer: &mut Peer, blocks: &[Block]) -> Result<(), SyncError> {
    let req = peer.recv()?;
    if req.first().copied() != Some(TAG_GET_BLOCKS) {
        return Err(SyncError::Protocol("expected a GET_BLOCKS request".into()));
    }
    for b in blocks {
        let encoded = encode_block(b);
        let mut frame = Vec::with_capacity(1 + encoded.len());
        frame.push(TAG_BLOCK);
        frame.extend_from_slice(&encoded);
        peer.send(&frame)?;
    }
    peer.send(&[TAG_DONE])?;
    Ok(())
}

/// JOINER side: request the peer's block log and apply each block to `node`, converging its state.
/// `node` must be a fresh genesis identical to the seed's. Returns the number of blocks applied.
pub fn sync_from(peer: &mut Peer, node: &mut Node) -> Result<usize, SyncError> {
    peer.send(&[TAG_GET_BLOCKS])?;
    let mut applied = 0usize;
    loop {
        let frame = peer.recv()?;
        match frame.first().copied() {
            Some(TAG_BLOCK) => {
                let block = decode_block(&frame[1..])?;
                node.apply(&block);
                applied += 1;
            }
            Some(TAG_DONE) => break,
            _ => return Err(SyncError::Protocol("unexpected sync frame tag".into())),
        }
    }
    Ok(applied)
}
