//! T1 slice-3 — gossip: fan-out a block announcement to peers, and dedup so a relayed block
//! terminates instead of looping the mesh forever.
//!
//! HONEST SCOPE: this is the gossip LOGIC — a fan-out broadcast (over slice-2 `Peer`s) plus a per-node
//! seen-set for loop-prevention. It is transport-abstracted so the load-bearing property (flooding a
//! peer graph TERMINATES, each node handling a block exactly once) is unit-testable without a live
//! mesh. The seen-set is per-node and ephemeral (rebuilt each run); wiring peers' reader threads into
//! a running node is slice-5 (the noesisd integration). Dedup uses the project's canonical blake2b so
//! every node agrees on what "the same message" is.

use std::collections::HashSet;
use std::io;

use crate::net::Peer;

/// Per-node gossip dedup state: the content-keys of frames this node has already processed.
pub struct Gossip {
    seen: HashSet<[u8; 32]>,
}

impl Default for Gossip {
    fn default() -> Self {
        Self::new()
    }
}

impl Gossip {
    pub fn new() -> Self {
        Self { seen: HashSet::new() }
    }

    /// The gossip identity of a frame: a domain-separated 32-byte blake2b of its bytes. Deterministic
    /// and replica-identical (the same canonical hasher the rest of the node uses), so every node
    /// agrees on what "the same message" is.
    pub fn key(payload: &[u8]) -> [u8; 32] {
        let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-gossip\0\0\0").build();
        h.update(payload);
        let mut out = [0u8; 32];
        h.finalize(&mut out);
        out
    }

    /// Observe a frame. Returns `true` the FIRST time this node sees it — the caller should then apply
    /// it and relay it to its OTHER peers. Returns `false` on any repeat, so a block circulating the
    /// mesh is handled exactly once per node and the flood terminates (no infinite relay loop).
    pub fn observe(&mut self, payload: &[u8]) -> bool {
        self.seen.insert(Self::key(payload))
    }

    /// Whether this node has already observed a frame (without recording it).
    pub fn already_seen(&self, payload: &[u8]) -> bool {
        self.seen.contains(&Self::key(payload))
    }

    /// Distinct frames observed (diagnostics / tests).
    pub fn seen_count(&self) -> usize {
        self.seen.len()
    }
}

/// Fan-out one frame to every peer. A RELAY passes the peers EXCLUDING the source, so the frame flows
/// outward and — with each node's [`Gossip::observe`] dedup — the flood terminates. Best-effort: a
/// send error on one peer is returned; the caller decides whether a dead peer aborts the round.
pub fn broadcast(peers: &mut [Peer], payload: &[u8]) -> io::Result<()> {
    for p in peers.iter_mut() {
        p.send(payload)?;
    }
    Ok(())
}
