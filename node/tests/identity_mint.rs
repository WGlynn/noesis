//! B4: soulbound identities minted on a devnet through the node's own API, the same path the
//! `identity-mint` binary takes: read `GET /tokens`, build and sign one tx from a batch file, `POST
//! /token-tx`, and let the next contribution block carry it. Three consecutive mint rounds (so the
//! authority key chain advances three times), a re-tier to revoked, and PoM standing compared with a
//! twin chain that received the same contributions and no mints.

use noesis::chainspec::{ChainSpec, TESTNET_IDENTITY_AUTHORITY_ROOT};
use noesis::identity_mint as im;
use noesis::rpc::{handle_request, ServerState};
use serde_json::{json, Value};

fn get(st: &mut ServerState, path: &str) -> Value {
    let (status, _, body) = handle_request(st, "GET", path, b"");
    assert_eq!(status, 200, "GET {path}");
    serde_json::from_slice(&body).unwrap()
}

/// A signed contribution (wallet format v2, no parent), built the way the browser wallet builds one.
fn contribution(master: &[u8; 32], index: u32, data: &str) -> Vec<u8> {
    use noesis_core::wallet;
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let tree = |t: &noesis_core::xmss::Signature| {
        json!({ "ots_root": hex(&t.ots_root), "auth": t.auth.iter().map(|s| hex(s)).collect::<Vec<_>>(), "ots_sig": hex(&t.ots_sig) })
    };
    let hot_root = wallet::key_root(master);
    let cold_commit = wallet::cold_commit(&[0xC0; 32]);
    let identity = wallet::identity(&hot_root, &cold_commit);
    let sig = wallet::sign(master, index, &wallet::contribution_digest(&identity, index, None, data.as_bytes()));
    json!({
        "identity": hex(&identity),
        "hot_root": hex(&hot_root),
        "cold_commit": hex(&cold_commit),
        "index": index,
        "bottom": tree(&sig.bottom),
        "top": tree(&sig.top),
        "data": data,
    })
    .to_string()
    .into_bytes()
}

/// A batch file in the bot's exact format.
fn batch(records: &[(u64, u8, &str)]) -> String {
    let recs: Vec<Value> = records
        .iter()
        .map(|&(serial, holder, tier)| json!({ "serial": serial, "holder": format!("{holder:02x}").repeat(32), "tier": tier }))
        .collect();
    json!({ "collection": "vibeswap-identity-v1", "records": recs }).to_string()
}

/// One mint round: batch -> plan against /tokens -> sign -> POST /token-tx. Returns the plan.
fn mint_round(st: &mut ServerState, batch_text: &str) -> Vec<im::Planned> {
    let records = im::parse_batch(batch_text, im::COLLECTION).expect("batch parses");
    let live = im::parse_live(&get(st, "/tokens"), im::COLLECTION).expect("live authority");
    let mut built = im::build(&records, &live, &im::DEV_ISSUER_SEED, im::COLLECTION).expect("builds");
    built.sign();
    let env = im::envelope(built.tx.as_ref().expect("something to send"));
    let (status, _, body) = handle_request(st, "POST", "/token-tx", env.to_string().as_bytes());
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));
    built.plan
}

/// The live identities by serial: (tier, holder hex, cell id).
fn identities(st: &mut ServerState) -> Vec<(u64, String, String, u64)> {
    let t = get(st, "/tokens");
    let mut v: Vec<(u64, String, String, u64)> = t["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["record"]["kind"] == "identity")
        .map(|c| {
            (
                c["record"]["serial"].as_u64().unwrap(),
                c["record"]["tier"].as_str().unwrap().to_string(),
                c["record"]["holder"].as_str().unwrap().to_string(),
                c["id"].as_u64().unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}

#[test]
fn devnet_batches_mint_and_retier_and_leave_pom_unchanged() {
    let mut chain = ServerState::new(); // ChainSpec::dev(), in memory
    let mut twin = ServerState::new(); // the same contributions, no mints
    let (ann, bo) = ([0x11u8; 32], [0x22u8; 32]);
    let notes = [
        (&ann, 0, "a field note on estuary sediment and the spring tide"),
        (&bo, 0, "an unrelated sketch of a wind pump made from bicycle parts"),
        (&ann, 1, "a later note on how salt marsh grasses trap silt"),
        (&bo, 1, "measurements of the wind pump output over one week"),
    ];
    let contribute = |st: &mut ServerState, i: usize| {
        let (m, idx, data) = notes[i];
        let (status, _, body) = handle_request(st, "POST", "/submit", &contribution(m, idx, data));
        assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));
    };

    // Round 1: three identities (no real person; holders are test bytes).
    let plan = mint_round(&mut chain, &batch(&[(1, 0xA1, "identity"), (2, 0xA2, "member"), (3, 0xA3, "identity")]));
    assert!(plan.iter().all(|p| p.action == im::Action::Mint));
    contribute(&mut chain, 0);
    contribute(&mut twin, 0);
    let ids = identities(&mut chain);
    assert_eq!(ids.len(), 3);
    for (p, (serial, _, _, id)) in plan.iter().zip(&ids) {
        assert_eq!(p.cell_id, Some(*id), "the tool's predicted cell id is the live cell's id (serial {serial})");
    }
    let included = get(&mut chain, "/tokens")["included"].clone();
    assert_eq!(included[0]["height"], 1, "the tool learns the carrying block's height");

    // Round 2 (second consecutive mint): re-tier serial 3 to revoked, mint serial 4, leave 1 and 2.
    let plan = mint_round(
        &mut chain,
        &batch(&[(1, 0xA1, "identity"), (2, 0xA2, "member"), (3, 0xA3, "revoked"), (4, 0xA4, "identity")]),
    );
    let actions: Vec<im::Action> = plan.iter().map(|p| p.action).collect();
    assert_eq!(actions, vec![im::Action::Unchanged, im::Action::Unchanged, im::Action::Retier, im::Action::Mint]);
    contribute(&mut chain, 1);
    contribute(&mut twin, 1);

    // Round 3 (third consecutive mint): serial 5.
    mint_round(&mut chain, &batch(&[(5, 0xA5, "member")]));
    contribute(&mut chain, 2);
    contribute(&mut twin, 2);
    // a block with no pending token tx still works and carries nothing
    contribute(&mut chain, 3);
    contribute(&mut twin, 3);

    let ids = identities(&mut chain);
    let tiers: Vec<(u64, &str)> = ids.iter().map(|(s, t, _, _)| (*s, t.as_str())).collect();
    assert_eq!(tiers, vec![(1, "identity"), (2, "member"), (3, "revoked"), (4, "identity"), (5, "member")]);
    assert_eq!(ids[2].2, "a3".repeat(32), "the re-tier kept the holder");
    let auth = get(&mut chain, "/tokens")["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["record"]["kind"] == "authority")
        .cloned()
        .unwrap();
    assert_eq!(auth["record"]["generation"], 3, "three mint rounds, three authority spends");

    // PoM: identical standing, cells and height to the twin that never minted.
    let (s, t) = (get(&mut chain, "/state"), get(&mut twin, "/state"));
    assert_eq!(s["contributors"], t["contributors"], "identity mints moved PoM standing");
    assert_eq!((s["height"].clone(), s["cells"].clone()), (t["height"].clone(), t["cells"].clone()));
    assert_eq!(s["contributors"].as_object().unwrap().len(), 2, "only the two contributors hold standing");

    // Replaying round 3's batch is idempotent: nothing to send.
    let live = im::parse_live(&get(&mut chain, "/tokens"), im::COLLECTION).unwrap();
    let again = im::build(&im::parse_batch(&batch(&[(5, 0xA5, "member")]), im::COLLECTION).unwrap(), &live, &im::DEV_ISSUER_SEED, im::COLLECTION).unwrap();
    assert!(again.tx.is_none());
}

#[test]
fn testnet_genesis_holds_the_issuer_authority_and_refuses_the_dev_seed() {
    let mut st = ServerState::with_spec(ChainSpec::testnet());
    let t = get(&mut st, "/tokens");
    assert_eq!(t["chain_id"], 0x7e57);
    let live = im::parse_live(&t, im::COLLECTION).expect("the testnet genesis has the authority");
    assert_eq!(live.generation, 0);
    assert_eq!(live.authority.lock.args, TESTNET_IDENTITY_AUTHORITY_ROOT.to_vec());
    let recs = im::parse_batch(&batch(&[(1, 0xA1, "identity")]), im::COLLECTION).unwrap();
    let err = im::build(&recs, &live, &im::DEV_ISSUER_SEED, im::COLLECTION).err().expect("the public dev seed is not the testnet issuer");
    assert!(err.contains("does not hold"));
}

/// Opt-in, local only: the real testnet issuer seed (kept outside the repo) mints against an
/// in-memory testnet genesis. Run with
/// `NOESIS_TESTNET_ISSUER_SEED=<seed-file> cargo test -p noesis --test identity_mint -- --ignored`.
#[test]
#[ignore]
fn testnet_seed_mints_against_the_testnet_genesis() {
    let path = std::env::var("NOESIS_TESTNET_ISSUER_SEED").expect("set NOESIS_TESTNET_ISSUER_SEED to the seed file");
    let master = im::parse_seed(&std::fs::read_to_string(path).unwrap()).unwrap();
    // the testnet pays its block reward to the faucet, so producing a block needs a faucet seed (public here)
    let mut st = ServerState::with_spec(ChainSpec::testnet()).with_faucet(noesis::faucet::DEV_FAUCET_SEED).unwrap();
    let live = im::parse_live(&get(&mut st, "/tokens"), im::COLLECTION).unwrap();
    let recs = im::parse_batch(&batch(&[(1, 0xA1, "identity")]), im::COLLECTION).unwrap();
    let mut built = im::build(&recs, &live, &master, im::COLLECTION).expect("the testnet seed holds the authority");
    built.sign();
    let env = im::envelope(built.tx.as_ref().unwrap());
    assert_eq!(handle_request(&mut st, "POST", "/token-tx", env.to_string().as_bytes()).0, 200);
    let (status, _, body) = handle_request(&mut st, "POST", "/submit", &contribution(&[0x33; 32], 0, "a testnet carrier note on tidal channels"));
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));
    assert_eq!(identities(&mut st).len(), 1, "minted on the testnet genesis");
}
