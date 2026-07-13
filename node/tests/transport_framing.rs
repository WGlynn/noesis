//! T1 slice-2 — framed TCP transport.
//!
//! Proves the transport moves self-delimiting messages intact: two peers on localhost exchange
//! frames byte-for-byte; the length-prefix framing survives the stream not respecting message
//! boundaries; and an oversized length header is rejected before allocating (the DoS bound).
//! This is codec-agnostic (opaque bytes) — in slice-4 these frames carry `wire::encode_block` output.

use noesis::net::{read_frame, write_frame, Listener, Peer};
use std::thread;

#[test]
fn two_peers_exchange_framed_messages_over_localhost() {
    let listener = Listener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");

    // Server: accept one peer, receive two frames, echo the first back.
    let server = thread::spawn(move || {
        let mut peer = listener.accept().expect("accept");
        let m1 = peer.recv().expect("recv frame 1");
        let m2 = peer.recv().expect("recv frame 2");
        peer.send(&m1).expect("echo frame 1");
        (m1, m2)
    });

    let mut client = Peer::connect(addr).expect("connect");
    client.send(b"the first framed message").expect("send 1");
    client.send(b"a second frame of a different length").expect("send 2");
    let echo = client.recv().expect("recv echo");

    let (m1, m2) = server.join().expect("server thread");
    assert_eq!(m1, b"the first framed message");
    assert_eq!(m2, b"a second frame of a different length");
    assert_eq!(echo, b"the first framed message", "frames round-trip byte-identical over TCP");
}

#[test]
fn length_prefix_framing_does_not_rely_on_stream_boundaries() {
    // Two frames concatenated into one buffer must read back as two distinct frames — proving the
    // framing, not any reliance on how TCP happened to chunk the bytes.
    let mut buf: Vec<u8> = Vec::new();
    write_frame(&mut buf, b"alpha").unwrap();
    write_frame(&mut buf, b"bravo-is-longer").unwrap();

    let mut cur = std::io::Cursor::new(buf);
    assert_eq!(read_frame(&mut cur).unwrap(), b"alpha");
    assert_eq!(read_frame(&mut cur).unwrap(), b"bravo-is-longer");
}

#[test]
fn an_oversized_frame_header_is_rejected_before_allocating() {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_be_bytes()); // a header claiming ~4 GiB
    let mut cur = std::io::Cursor::new(buf);
    assert!(
        read_frame(&mut cur).is_err(),
        "a length header above MAX_FRAME must be rejected, never allocated"
    );
}
