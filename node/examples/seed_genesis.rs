//! seed_genesis — emit a real XMSS-signed /submit body for the FOUNDATIONAL contribution:
//! the Noesis codebase itself (the DAG root everything else builds on). Prints the JSON body to
//! stdout; a caller curls it into `POST /submit`. Deterministic genesis identity (a fixed master
//! seed) so the founder address is stable and reproducible, not a throwaway key.
//!
//!   cargo run --release --example seed_genesis -p noesis > /tmp/genesis.json
//!   curl -s -X POST -H 'content-type: application/json' --data @/tmp/genesis.json <node>/submit
//!
//! HONEST SCOPE: on the novelty-scoring testnet this earns novelty standing like any contribution.
//! The mainnet design is a genesis-BAKED root at standing 0 (anchor, not premine) — see CONTINUE.md.

use noesis_core::xmss;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    // 32-byte deterministic genesis master seed → stable founder identity for the codebase root.
    let master: [u8; 32] = *b"noesis:genesis:codebase-root:001";
    let address = xmss::keygen_address(&master);
    let index: u32 = 0;

    let data = "Noesis genesis: the codebase that measures contribution. \
        The foundational contribution every other one builds on - the mechanism itself \
        (commit eee60e5, github.com/WGlynn/noesis). Something from nothing: the one \
        contribution that provably exists before anyone arrives.";

    let msg = xmss::contribution_digest(&address, index, data.as_bytes());
    let sig = xmss::sign(&master, index, &msg);
    assert!(xmss::verify(&address, &msg, &sig), "genesis signature self-check failed");

    let auth = sig
        .auth
        .iter()
        .map(|a| format!("\"{}\"", hex(a)))
        .collect::<Vec<_>>()
        .join(",");

    // data has no quotes/backslashes/newlines ⇒ safe to embed directly in JSON.
    println!(
        "{{\"address\":\"{}\",\"index\":{},\"ots_root\":\"{}\",\"ots_sig\":\"{}\",\"auth\":[{}],\"data\":\"{}\"}}",
        hex(&address),
        index,
        hex(&sig.ots_root),
        hex(&sig.ots_sig),
        auth,
        data
    );
}
