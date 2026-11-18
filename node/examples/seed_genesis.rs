//! seed_genesis — emit a real signed /submit body (wallet format v2) for the FOUNDATIONAL
//! contribution: the Noesis codebase itself (the DAG root everything else builds on). Prints the JSON
//! body to stdout; a caller curls it into `POST /submit`. Deterministic genesis identity (fixed hot and
//! cold seeds) so the founder identity is stable and reproducible, not a throwaway key.
//!
//!   cargo run --release --example seed_genesis -p noesis > /tmp/genesis.json
//!   curl -s -X POST -H 'content-type: application/json' --data @/tmp/genesis.json <node>/submit
//!
//! HONEST SCOPE: both seeds are public strings in this file, so anyone can sign as this identity and,
//! once recovery exists, recover it. It is a reproducible demo identity, not a key anyone should rely
//! on. On the novelty-scoring testnet this earns novelty standing like any contribution (the testnet
//! genesis rule leaves the chain's first contribution unscored). The mainnet design is a
//! genesis-BAKED root at standing 0 (anchor, not premine) — see CONTINUE.md.

use noesis_core::wallet;
use serde_json::json;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn tree(t: &noesis_core::xmss::Signature) -> serde_json::Value {
    json!({ "ots_root": hex(&t.ots_root), "auth": t.auth.iter().map(|a| hex(a)).collect::<Vec<_>>(), "ots_sig": hex(&t.ots_sig) })
}

fn main() {
    // 32-byte deterministic genesis seeds → a stable founder identity for the codebase root.
    let hot: [u8; 32] = *b"noesis:genesis:codebase-root:001";
    let cold: [u8; 32] = *b"noesis:genesis:codebase-cold:001";
    let hot_root = wallet::key_root(&hot);
    let cold_commit = wallet::cold_commit(&wallet::key_root(&cold));
    let identity = wallet::identity(&hot_root, &cold_commit);
    let index: u32 = 0;

    let data = "Noesis genesis: the codebase that measures contribution. \
        The foundational contribution every other one builds on - the mechanism itself \
        (commit eee60e5, github.com/WGlynn/noesis). Something from nothing: the one \
        contribution that provably exists before anyone arrives.";

    let msg = wallet::contribution_digest(&identity, index, None, data.as_bytes());
    let sig = wallet::sign(&hot, index, &msg);
    assert!(wallet::verify(&hot_root, index, &msg, &sig), "genesis signature self-check failed");

    let body = json!({
        "identity": hex(&identity),
        "hot_root": hex(&hot_root),
        "cold_commit": hex(&cold_commit),
        "index": index,
        "bottom": tree(&sig.bottom),
        "top": tree(&sig.top),
        "data": data,
    });
    println!("{body}");
}
