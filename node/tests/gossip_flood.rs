//! T1 slice-3 — gossip.
//!
//! Two load-bearing properties: a fan-out broadcast reaches every connected peer, and flooding a peer
//! graph TERMINATES with each node handling the block exactly once (dedup = loop prevention). The
//! mesh-termination test is a pure simulation (no sockets), so it exercises the relay/dedup logic
//! directly; the broadcast test uses real localhost TCP from slice-2.

use noesis::gossip::{broadcast, Gossip};
use noesis::net::{Listener, Peer};
use std::collections::VecDeque;
use std::thread;

#[test]
fn a_repeated_frame_is_observed_once() {
    let mut g = Gossip::new();
    assert!(g.observe(b"block X"), "first sight is new -> process + relay");
    assert!(!g.observe(b"block X"), "second sight is a repeat -> drop (loop prevention)");
    assert!(g.observe(b"block Y"), "a different frame is new");
    assert_eq!(g.seen_count(), 2);
}

#[test]
fn broadcast_reaches_every_connected_peer() {
    let l1 = Listener::bind("127.0.0.1:0").unwrap();
    let a1 = l1.local_addr().unwrap();
    let l2 = Listener::bind("127.0.0.1:0").unwrap();
    let a2 = l2.local_addr().unwrap();

    let s1 = thread::spawn(move || l1.accept().unwrap().recv().unwrap());
    let s2 = thread::spawn(move || l2.accept().unwrap().recv().unwrap());

    let mut peers = vec![Peer::connect(a1).unwrap(), Peer::connect(a2).unwrap()];
    broadcast(&mut peers, b"one block to all peers").unwrap();

    assert_eq!(s1.join().unwrap(), b"one block to all peers");
    assert_eq!(s2.join().unwrap(), b"one block to all peers");
}

#[test]
fn flooding_a_mesh_terminates_and_each_node_sees_it_once() {
    // Fully-connected 3-node mesh. Node 0 originates a block; every node that sees it FIRST relays to
    // its other peers. Dedup must make this terminate (the relayed copies hit already-seen nodes and
    // stop), with each node observing the block exactly once.
    let n = 3usize;
    let mut nodes: Vec<Gossip> = (0..n).map(|_| Gossip::new()).collect();
    let msg = b"a gossiped block announcement".to_vec();

    // (recipient, from) delivery events. Origin: node 0 "receives from itself".
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    queue.push_back((0, 0));
    let mut deliveries = 0usize;

    while let Some((node, from)) = queue.pop_front() {
        deliveries += 1;
        assert!(deliveries < 1000, "flood must terminate via dedup, not run away");
        if nodes[node].observe(&msg) {
            // first sight here -> relay to every OTHER node except the source.
            for peer in 0..n {
                if peer != node && peer != from {
                    queue.push_back((peer, node));
                }
            }
        }
    }

    for (i, g) in nodes.iter().enumerate() {
        assert_eq!(g.seen_count(), 1, "node {i} must observe the block exactly once");
    }
}
