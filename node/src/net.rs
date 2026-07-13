//! T1 slice-2 — framed TCP transport (the wire that carries blocks between nodes).
//!
//! HONEST SCOPE: this is the transport layer only — length-prefixed byte frames over a TCP stream,
//! plus a listener and a peer connection. It is deliberately CODEC-AGNOSTIC: it moves opaque
//! `Vec<u8>` frames and knows nothing about blocks. In slice-4 (sync) those frames will carry
//! `wire::encode_block` output; here we just prove bytes cross the network intact and framed. Built on
//! `std::net` + `std::thread` — NO async runtime, NO new dependency (minimal footprint by design).
//!
//! Why framing: TCP is a byte stream with no message boundaries — one `read` may return half a message
//! or two messages. A u32 length prefix makes each message self-delimiting, so a peer always reads
//! exactly one logical frame regardless of how the OS chunked the bytes.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Resource-DoS bound: reject a length header claiming more than this before allocating. A peer
/// cannot make us allocate gigabytes by lying in the 4-byte header. 16 MiB is far above any real
/// block; slice-4 can lower it to a block-size ceiling once blocks have a known bound.
pub const MAX_FRAME: u32 = 16 * 1024 * 1024;

/// Per-socket I/O deadline. A `read`/`write` that makes no progress for this long fails with a
/// timeout error instead of blocking the thread forever — closing the "peer sends a length header
/// then stalls" hang (a joiner never converges; a seed's serve thread never returns). 30s is far
/// above any honest frame's transit time on a local/testnet link; a real WAN deploy can tune it.
pub const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Apply the read+write deadlines to a stream (best-effort: a platform that rejects the call leaves
/// the socket blocking rather than failing the connection outright).
fn arm_timeouts(stream: &TcpStream) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
}

/// Write one length-prefixed frame: u32 big-endian length, then the payload. Flushes so the peer
/// sees the whole frame promptly.
pub fn write_frame(w: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    let len = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame length exceeds u32"))?;
    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "frame exceeds MAX_FRAME"));
    }
    w.write_all(&len.to_be_bytes())?;
    w.write_all(payload)?;
    w.flush()
}

/// Read exactly one length-prefixed frame. Rejects an oversized length header BEFORE allocating the
/// buffer (the DoS bound). `read_exact` reassembles the frame across however many TCP reads it takes,
/// so this is robust to the stream splitting or coalescing messages.
pub fn read_frame(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame header exceeds MAX_FRAME"));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// A framed peer connection over TCP. `send`/`recv` move one logical frame each, regardless of TCP
/// chunking. Codec-agnostic: the payload is whatever the caller puts in (an encoded block, later).
pub struct Peer {
    stream: TcpStream,
}

impl Peer {
    /// Dial a remote peer. The connection is armed with [`IO_TIMEOUT`] so a stalled peer can never
    /// hang this side forever.
    pub fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        arm_timeouts(&stream);
        Ok(Self { stream })
    }

    /// Wrap an already-accepted stream (from [`Listener::accept`]), armed with [`IO_TIMEOUT`].
    pub fn from_stream(stream: TcpStream) -> Self {
        arm_timeouts(&stream);
        Self { stream }
    }

    /// Send one frame.
    pub fn send(&mut self, payload: &[u8]) -> io::Result<()> {
        write_frame(&mut self.stream, payload)
    }

    /// Receive one frame (blocking until a full frame arrives).
    pub fn recv(&mut self) -> io::Result<Vec<u8>> {
        read_frame(&mut self.stream)
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }
}

/// A TCP listener that accepts framed [`Peer`] connections.
pub struct Listener {
    inner: TcpListener,
}

impl Listener {
    /// Bind and start listening. Use `"127.0.0.1:0"` to get an OS-assigned ephemeral port
    /// (read it back with [`Listener::local_addr`]).
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        Ok(Self { inner: TcpListener::bind(addr)? })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    /// Accept the next incoming connection as a framed [`Peer`] (blocking).
    pub fn accept(&self) -> io::Result<Peer> {
        let (stream, _addr) = self.inner.accept()?;
        Ok(Peer::from_stream(stream))
    }
}
