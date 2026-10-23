//! factnode: the devnet fact-sync node (the day-30 bar, `docs/DESIGN-layer-map.md` section 7).
//!
//!   factnode serve <addr> <source> [--trace FILE] [--junk]     wait for one peer, swap facts, print the merged hash
//!   factnode connect <addr> <source> [--trace FILE] [--junk]   dial a peer, swap facts, print the merged hash
//!   factnode replay <trace> --seeds FILE                       replay a trace in its own order and once per seed
//!
//! `<source>` is `--demo a|b|c`: built-in chains whose records are signed by demo keys, so two
//! machines need no shared files. There is no `--store` source yet: the live chain doesn't keep the
//! wallet's signatures, so its records can't travel signed. `--junk` first sends the frames a
//! misbehaving peer might send (`factsync::junk_frames`), including a forged copy of a genuine record
//! that arrives before it.
//!
//! Two machines: on A, `factnode serve 0.0.0.0:7777 --demo a --trace a.trace`; on B,
//! `factnode connect <A's address>:7777 --demo b --trace b.trace`. Then each runs
//! `factnode replay <its trace> --seeds docs/devnet/shuffle-seeds.txt`. Every line must show the same hash.
//!
//! Output is `KEY value` lines so scripts and tests can read it. Counts are over the whole trace (own
//! frames plus received frames). HONEST SCOPE: as in `noesis::factsync` (demo keys, one exchange).

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::Duration;

use noesis::factsync::{self, hex};

fn fail(msg: &str) -> ! {
    eprintln!("factnode: {msg}");
    exit(2)
}

#[derive(Default)]
struct Opts {
    source: Option<Vec<Vec<u8>>>,
    trace: Option<PathBuf>,
    seeds: Option<PathBuf>,
    junk: bool,
}

fn parse(args: &[String]) -> Opts {
    let mut o = Opts::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| fail(&format!("{a} needs a value")));
        match a.as_str() {
            "--demo" => {
                let n = value();
                o.source = Some(factsync::demo_frames(n).unwrap_or_else(|| fail(&format!("no demo {n:?} (a, b, c)"))));
            }
            "--trace" => o.trace = Some(PathBuf::from(value())),
            "--seeds" => o.seeds = Some(PathBuf::from(value())),
            "--junk" => o.junk = true,
            other => fail(&format!("unknown argument {other:?}")),
        }
    }
    o
}

fn swap(stream: TcpStream, o: Opts) {
    let own = o.source.unwrap_or_else(|| fail("need --demo"));
    println!("PEER {}", stream.peer_addr().map(|a| a.to_string()).unwrap_or_default());
    let mut outgoing = match (o.junk, own.first()) {
        (true, Some(f)) => factsync::junk_frames(f),
        _ => Vec::new(),
    };
    outgoing.extend(own.iter().cloned());
    let received = factsync::exchange(stream, outgoing).unwrap_or_else(|e| fail(&format!("exchange: {e}")));
    let trace = factsync::trace_entries(&own, &received);
    let (merged, n) = factsync::replay(&trace, None);
    println!("RECEIVED {}", received.len());
    println!("ACCEPTED {} REJECTED {}", n.accepted, n.rejected);
    println!("RIVALS {}", merged.rival_claims().len());
    println!("DANGLING {}", merged.dangling().len());
    println!("MERGED {} {}", merged.len(), hex(&merged.digest()));
    if let Some(p) = o.trace {
        factsync::write_trace(&p, &trace).unwrap_or_else(|e| fail(&format!("trace {}: {e}", p.display())));
        println!("TRACE {}", p.display());
    }
}

fn own_line(o: &Opts) {
    let own = o.source.as_ref().unwrap_or_else(|| fail("need --demo"));
    let (set, n) = factsync::apply_frames(own.iter().map(Vec::as_slice));
    println!("OWN {} {} rejected {}", set.len(), hex(&set.digest()), n.rejected);
}

fn serve(addr: &str, o: Opts) {
    own_line(&o);
    let listener = TcpListener::bind(addr).unwrap_or_else(|e| fail(&format!("bind {addr}: {e}")));
    let local = listener.local_addr().unwrap_or_else(|e| fail(&format!("local addr: {e}")));
    println!("LISTENING {local}");
    let (stream, _) = listener.accept().unwrap_or_else(|e| fail(&format!("accept: {e}")));
    swap(stream, o);
}

/// Dial, retrying for about five seconds so the two ends can be started by hand in either order.
fn connect(addr: &str, o: Opts) {
    own_line(&o);
    let mut tries = 0;
    let stream = loop {
        match TcpStream::connect(addr) {
            Ok(s) => break s,
            Err(_) if tries < 20 => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(e) => fail(&format!("connect {addr}: {e}")),
        }
    };
    swap(stream, o);
}

fn replay(trace_path: &str, o: Opts) {
    let trace = factsync::read_trace(Path::new(trace_path)).unwrap_or_else(|e| fail(&format!("trace {trace_path}: {e}")));
    let seeds_path = o.seeds.unwrap_or_else(|| fail("replay needs --seeds FILE"));
    let text = std::fs::read_to_string(&seeds_path).unwrap_or_else(|e| fail(&format!("seeds {}: {e}", seeds_path.display())));
    let seeds = factsync::parse_seeds(&text).unwrap_or_else(|e| fail(&e));
    if seeds.is_empty() {
        fail("the seed list is empty");
    }
    let wire = trace.iter().filter(|(origin, _)| *origin == factsync::WIRE).count();
    println!("ENTRIES {} (own {}, wire {wire})", trace.len(), trace.len() - wire);
    let (base, base_n) = factsync::replay(&trace, None);
    println!("TRACE_ORDER {} {}", base.len(), hex(&base.digest()));
    let mut bad = 0;
    for &s in &seeds {
        let (set, n) = factsync::replay(&trace, Some(s));
        let same = set.digest() == base.digest() && n == base_n;
        println!("SEED {s} {}{}", hex(&set.digest()), if same { "" } else { " MISMATCH" });
        bad += usize::from(!same);
    }
    if bad > 0 {
        println!("FAIL {bad} of {} seeds differ", seeds.len());
        exit(1);
    }
    println!(
        "ALL {} SEEDS MATCH {} {} accepted {} rejected {}",
        seeds.len(),
        base.len(),
        hex(&base.digest()),
        base_n.accepted,
        base_n.rejected
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, target, rest @ ..] if cmd == "serve" => serve(target, parse(rest)),
        [cmd, target, rest @ ..] if cmd == "connect" => connect(target, parse(rest)),
        [cmd, target, rest @ ..] if cmd == "replay" => replay(target, parse(rest)),
        _ => fail(
            "usage: factnode serve|connect <addr> --demo a|b|c [--trace FILE] [--junk]\n       factnode replay <trace> --seeds FILE",
        ),
    }
}
