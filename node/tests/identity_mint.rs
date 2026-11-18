//! B4: soulbound identities minted on a devnet through the node's own API, the same path the
//! `identity-mint` binary takes: read `GET /tokens`, build and sign one tx from a batch file, `POST
//! /token-tx`, and let the next contribution block carry it. Three consecutive mint rounds (so the
//! authority key chain advances three times), a re-tier to revoked, and PoM standing compared with a
//! twin chain that received the same contributions and no mints.
//!
//! Wallet decision A8 (the issuer key's limits), through the real binaries: `noesisd` on a temp port and
//! a temp store, `identity-mint` on temp seed, batch and state files. Nothing here touches `~/.noesis`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

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

/// Wallet decision A8 changes only the mint tool and its files, never a genesis: the dev and testnet
/// genesis token cells (the issuer authority) stay byte-identical to HEAD f4a942f2. The pinned digests
/// were computed at that commit, before the change (every field of every genesis token cell).
#[test]
fn genesis_token_cells_are_unchanged_by_the_issuer_key_limits() {
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    for (spec, pinned) in [
        (ChainSpec::dev(), "a2541867040bbdb156d6c3e130f34a7a5fb11d2041b24ec0a899b396cb340bf3"),
        (ChainSpec::testnet(), "c3dec2a7b33b401c15daa577dd9c9b49461563887aadd4e89cd02f8ab1cc3659"),
    ] {
        let (node, _) = spec.genesis_node();
        let text: Vec<String> = node
            .ledger
            .token_cells
            .iter()
            .map(|c| format!("{}|{:?}|{}", noesis::rpc::token_cell_json(c), c.parent, c.timestamp))
            .collect();
        let digest = hex(&noesis_core::pow::hash(text.join("\n").as_bytes()));
        assert_eq!(digest, pinned, "chain 0x{:x}: the genesis token cells changed", spec.chain_id());
    }
}

// ============ wallet decision A8, through the real binaries ============

const PASS_ENV: &str = "NOESIS_ISSUER_PASSPHRASE";

fn hexs(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn tmp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("noesis-identity-mint-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A dev-chain `noesisd --serve-api` on a free port; killed on drop.
struct Node {
    child: Child,
    addr: String,
}

impl Node {
    fn start(store: &Path) -> Node {
        let mut child = Command::new(env!("CARGO_BIN_EXE_noesisd"))
            .env("NOESIS_NET", "dev")
            .args(["--serve-api", "127.0.0.1:0"])
            .arg(store)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn noesisd");
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let addr = loop {
            let l = lines.next().expect("noesisd printed LISTENING").unwrap();
            if let Some(a) = l.strip_prefix("LISTENING http://") {
                break a.trim().to_string();
            }
        };
        std::thread::spawn(move || lines.for_each(drop)); // keep draining so the pipe never fills
        Node { child, addr }
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn http(&self, method: &str, path: &str, body: &[u8]) -> (u16, Value) {
        let mut s = TcpStream::connect(&self.addr).expect("connect");
        s.set_read_timeout(Some(Duration::from_secs(300))).unwrap();
        let head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.addr,
            body.len()
        );
        s.write_all(head.as_bytes()).unwrap();
        s.write_all(body).unwrap();
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).unwrap();
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let status = std::str::from_utf8(&raw[..split]).unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, serde_json::from_slice(&raw[split + 4..]).unwrap_or(Value::Null))
    }

    fn tokens(&self) -> Value {
        self.http("GET", "/tokens", b"").1
    }

    /// One contribution block (it carries whatever token tx is pending).
    fn block(&self, body: &[u8]) {
        let (s, v) = self.http("POST", "/submit", body);
        assert_eq!(s, 200, "{v}");
    }

    /// Every live identity cell, by serial: (serial, tier, holder, id, lock args).
    fn identities(&self) -> Vec<(u64, String, String, u64, String)> {
        let mut v: Vec<_> = self.tokens()["cells"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["record"]["kind"] == "identity")
            .map(|c| {
                let r = &c["record"];
                let s = |x: &Value| x.as_str().unwrap().to_string();
                (r["serial"].as_u64().unwrap(), s(&r["tier"]), s(&r["holder"]), c["id"].as_u64().unwrap(), s(&c["lock_args"]))
            })
            .collect();
        v.sort();
        v
    }

    fn generation(&self) -> u64 {
        im::parse_live(&self.tokens(), im::COLLECTION).unwrap().generation
    }

    fn pending(&self) -> Vec<String> {
        self.tokens()["pending"].as_array().unwrap().iter().map(|d| d.as_str().unwrap().to_string()).collect()
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Three carrier contributions (one test wallet, indices 0..3), signed once and shared by every test
/// here: XMSS^MT signing is slow in a debug build, and a reset chain takes the same bodies again.
fn carriers() -> &'static [Vec<u8>] {
    static C: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
    C.get_or_init(|| {
        [
            "a carrier note on how barnacles settle on a new hull",
            "a second carrier note, on the drying of salt cod in the wind",
            "a third carrier note about counting rings in a cut oak",
        ]
        .iter()
        .enumerate()
        .map(|(i, t)| contribution(&[0x77; 32], i as u32, t))
        .collect()
    })
}

fn write_batch(dir: &Path, name: &str, records: &[(u64, u8, &str)]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, batch(records)).unwrap();
    p
}

fn mint_cmd(node: &Node, seed: &Path, batch: &Path, extra: &[&str]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_identity-mint"));
    c.args(["mint", "--node", &node.url(), "--seed"]).arg(seed).arg(batch).args(extra);
    c.stdin(Stdio::null()).env_remove(PASS_ENV);
    c
}

/// (exit code, stdout, stderr)
fn run(mut c: Command) -> (i32, String, String) {
    let o = c.stdout(Stdio::piped()).stderr(Stdio::piped()).output().expect("run identity-mint");
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

/// The operation digest a dry run prints.
fn dry_digest(stderr: &str) -> String {
    let at = stderr.find("operation digest ").unwrap_or_else(|| panic!("no digest in:\n{stderr}")) + "operation digest ".len();
    stderr[at..at + 64].to_string()
}

#[test]
fn after_a_reset_the_tool_resends_what_each_key_signed_instead_of_signing_again() {
    let dir = tmp("reset");
    let seed = dir.join("dev.seed");
    std::fs::write(&seed, hexs(&im::DEV_ISSUER_SEED)).unwrap();
    let b1 = write_batch(&dir, "b1.json", &[(1, 0xA1, "identity"), (2, 0xA2, "member")]);
    let b2 = write_batch(&dir, "b2.json", &[(1, 0xA1, "identity"), (2, 0xA2, "member"), (3, 0xA3, "identity")]);
    let b3 = write_batch(&dir, "b3.json", &[(1, 0xA1, "identity"), (2, 0xA2, "member"), (3, 0xA3, "identity"), (4, 0xA4, "member")]);
    let c = carriers();

    // Chain A: two rounds, each waited for until its block lands (the 2026-11-17 devnet run's flow).
    let a = Node::start(&dir.join("a.log"));
    let mut after_round = Vec::new();
    for (batch, body) in [(&b1, &c[0]), (&b2, &c[1])] {
        let child = mint_cmd(&a, &seed, batch, &["--wait", "300"]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let t = Instant::now();
        while a.pending().is_empty() {
            assert!(t.elapsed() < Duration::from_secs(120), "the tool never posted");
            std::thread::sleep(Duration::from_millis(200));
        }
        a.block(body);
        let o = child.wait_with_output().unwrap();
        assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
        after_round.push(a.identities());
    }
    let carried_a: Vec<Value> = a.tokens()["included"].as_array().unwrap().clone();
    drop(a);

    // A reset: a new chain from the same genesis, so the issuer's authority is back at generation 0, whose
    // one-time key already signed chain A's first tx.
    let b = Node::start(&dir.join("b.log"));
    assert_eq!(b.generation(), 0);
    let (code, _, err) = run(mint_cmd(&b, &seed, &b3, &[]));
    assert_eq!(code, 4, "generation 0's key signed on chain A: re-send that tx, never sign another\n{err}");
    assert_eq!(b.pending(), vec![carried_a[0]["digest"].as_str().unwrap().to_string()], "the identical tx");
    b.block(&c[0]);
    assert_eq!(b.identities(), after_round[0], "the same cells as chain A after its first round");
    let (code, _, err) = run(mint_cmd(&b, &seed, &b3, &[]));
    assert_eq!(code, 4, "generation 1 likewise\n{err}");
    b.block(&c[1]);
    assert_eq!(b.identities(), after_round[1]);
    // past everything recorded, a fresh key signs the new record
    let (code, out, err) = run(mint_cmd(&b, &seed, &b3, &[]));
    assert_eq!(code, 0, "{err}");
    assert!(out.lines().any(|l| l.contains("\"serial\":4") && l.contains("queued")), "{out}");
    b.block(&c[2]);
    assert_eq!(b.identities().len(), 4);
    assert_eq!(b.generation(), 3);

    // The journal: exactly one digest per generation, and the tool never deleted it.
    let j: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("dev.seed.pending.json")).unwrap()).unwrap();
    let gens: Vec<u64> = j["signed"].as_array().unwrap().iter().map(|s| s["generation"].as_u64().unwrap()).collect();
    assert_eq!(gens, vec![0, 1, 2]);
    assert!(j["signed"].as_array().unwrap().iter().all(|s| s.get("earlier").is_none()), "no key signed twice");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn revokes_wait_for_a_confirmed_dry_run_and_the_day_is_capped() {
    let dir = tmp("confirm");
    let seed = dir.join("dev.seed");
    std::fs::write(&seed, hexs(&im::DEV_ISSUER_SEED)).unwrap();
    let state = dir.join("dev.seed.pending.json");
    let three = write_batch(&dir, "three.json", &[(1, 0xB1, "identity"), (2, 0xB2, "member"), (3, 0xB3, "identity")]);
    let c = carriers();
    let store = dir.join("c.log");
    let mut node = Node::start(&store);
    assert_eq!(run(mint_cmd(&node, &seed, &three, &["--abandon-pending"])).0, 7, "--abandon-pending always needs --confirm");

    // The cap: lowering it needs no confirmation; over it a record waits as deferred.
    let (code, out, err) = run(mint_cmd(&node, &seed, &three, &["--daily-cap", "2"]));
    assert_eq!(code, 0, "{err}");
    assert_eq!((out.matches("\"queued\"").count(), out.matches("\"deferred\"").count()), (2, 1), "{out}");
    assert!(err.contains("daily cap"), "{err}");
    let (code, _, err) = run(mint_cmd(&node, &seed, &three, &["--daily-cap", "2"]));
    assert_eq!(code, 0, "the same operation again re-sends the same bytes\n{err}");
    node.block(&c[0]);
    // A seed that does not hold this chain's authority (now at generation 1) is refused before anything is
    // recorded for it, so pointing a seed at the wrong node never marks its keys as used.
    let foreign = dir.join("foreign.seed");
    std::fs::write(&foreign, "42".repeat(32)).unwrap();
    assert_eq!(run(mint_cmd(&node, &foreign, &three, &[])).0, 2);
    assert!(!dir.join("foreign.seed.pending.json").exists());
    let (code, _, err) = run(mint_cmd(&node, &seed, &three, &["--daily-cap", "2"]));
    assert_eq!(code, 6, "two of two used today\n{err}");
    let (code, _, err) = run(mint_cmd(&node, &seed, &three, &["--daily-cap", "100"]));
    assert_eq!(code, 7, "raising the cap needs the issuer's confirmation\n{err}");
    let (code, _, err) = run(mint_cmd(&node, &seed, &three, &[]));
    assert_eq!(code, 0, "the default cap has room for serial 3\n{err}");
    node.block(&c[1]);
    assert_eq!(node.identities().len(), 3);

    // A revoke: refused without --confirm, and nothing is signed, written or sent.
    let revoke = write_batch(&dir, "revoke.json", &[(3, 0xB3, "revoked")]);
    let (code, _, err) = run(mint_cmd(&node, &seed, &revoke, &[]));
    assert_eq!(code, 7, "{err}");
    let signed_gens = |s: &[u8]| -> Vec<u64> {
        let j: Value = serde_json::from_slice(s).unwrap();
        j["signed"].as_array().unwrap().iter().map(|s| s["generation"].as_u64().unwrap()).collect()
    };
    let before = std::fs::read(&state).unwrap();
    assert_eq!(signed_gens(&before), vec![0, 1], "the refused revoke signed nothing");
    assert!(node.pending().is_empty());
    let (code, out, err) = run(mint_cmd(&node, &seed, &revoke, &["--dry-run"]));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"retier\""), "{out}");
    assert_eq!((std::fs::read(&state).unwrap(), node.pending()), (before, vec![]), "a dry run signs, writes and sends nothing");
    let d = dry_digest(&err);
    let wrong = if d.starts_with('0') { "1" } else { "0" }.repeat(8);
    assert_eq!(run(mint_cmd(&node, &seed, &revoke, &["--confirm", &wrong])).0, 7, "another operation's digest");
    assert_eq!(run(mint_cmd(&node, &seed, &revoke, &["--confirm", &d[..7]])).0, 7, "too short");
    let (code, _, err) = run(mint_cmd(&node, &seed, &revoke, &["--confirm", &d[..8]]));
    assert_eq!(code, 0, "{err}");
    assert_eq!(node.pending(), vec![d.clone()], "the confirmed digest is what was signed");
    node.block(&c[2]);
    assert!(node.identities().iter().any(|i| i.0 == 3 && i.1 == "revoked"));

    // --abandon-pending signs a second digest with a key that already signed: confirmed too.
    let five = write_batch(&dir, "five.json", &[(5, 0xB5, "member")]);
    let six = write_batch(&dir, "six.json", &[(6, 0xB6, "member")]);
    assert_eq!(run(mint_cmd(&node, &seed, &five, &[])).0, 0);
    assert_eq!(run(mint_cmd(&node, &seed, &six, &[])).0, 4, "re-sends the pending tx");
    assert_eq!(run(mint_cmd(&node, &seed, &six, &["--abandon-pending"])).0, 7);
    let (code, _, err) = run(mint_cmd(&node, &seed, &six, &["--abandon-pending", "--dry-run"]));
    assert_eq!(code, 0, "{err}");
    let d6 = dry_digest(&err);
    drop(node); // the node forgets its pending txs on restart, which is when abandoning one makes sense
    node = Node::start(&store);
    let (code, _, err) = run(mint_cmd(&node, &seed, &six, &["--abandon-pending", "--confirm", &d6[..8]]));
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("WARNING"), "{err}");
    let j: Value = serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    let g3 = j["signed"].as_array().unwrap().iter().find(|s| s["generation"] == 3).unwrap().clone();
    assert_eq!((g3["digest"].as_str().unwrap(), g3["earlier"].as_array().unwrap().len()), (d6.as_str(), 1));
    drop(node);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn encrypt_seed_seals_the_file_and_the_passphrase_never_comes_from_a_pipe() {
    let dir = tmp("seal");
    let seed = dir.join("issuer.seed");
    let plain = hexs(&im::DEV_ISSUER_SEED);
    std::fs::write(&seed, &plain).unwrap();
    let root = hexs(&im::authority_root(&im::DEV_ISSUER_SEED, 0));
    let tool = |args: &[&str], pass: Option<&str>| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_identity-mint"));
        c.args(args).arg("--seed").arg(&seed).stdin(Stdio::null()).env_remove(PASS_ENV);
        if let Some(p) = pass {
            c.env(PASS_ENV, p);
        }
        run(c)
    };
    // No console (stdin is not a terminal, as in an agent's shell) and no opt-in variable: nothing is read.
    let (code, _, err) = tool(&["encrypt-seed"], None);
    assert_eq!(code, 2);
    assert!(err.contains("console"), "{err}");
    assert_eq!(std::fs::read_to_string(&seed).unwrap(), plain, "untouched");
    // The issuer opted in through the variable: the file is sealed in place, at the full round count.
    let (code, out, err) = tool(&["encrypt-seed"], Some("a long test passphrase"));
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.trim(), root);
    let text = std::fs::read_to_string(&seed).unwrap();
    assert!(!text.contains(&plain), "the plain seed is gone");
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["iter"], 600_000);
    assert!(!err.contains("a long test passphrase") && !out.contains("a long test passphrase"), "never printed");
    // It opens with the passphrase, and only with it; and it is not sealed twice.
    let (code, out, _) = tool(&["root"], Some("a long test passphrase"));
    assert_eq!((code, out.trim()), (0, root.as_str()));
    assert_eq!(tool(&["root"], Some("the wrong passphrase")).0, 2);
    assert!(tool(&["root"], None).2.contains("console"));
    let (code, _, err) = tool(&["encrypt-seed"], Some("a long test passphrase"));
    assert_eq!(code, 2);
    assert!(err.contains("already sealed"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Opt-in, local only: the real testnet issuer seed (kept outside the repo) mints against an
/// in-memory testnet genesis. Run with
/// `NOESIS_TESTNET_ISSUER_SEED=<seed-file> cargo test -p noesis --test identity_mint -- --ignored`
/// (a sealed seed file also needs NOESIS_ISSUER_PASSPHRASE).
#[test]
#[ignore]
fn testnet_seed_mints_against_the_testnet_genesis() {
    let path = std::env::var("NOESIS_TESTNET_ISSUER_SEED").expect("set NOESIS_TESTNET_ISSUER_SEED to the seed file");
    let text = std::fs::read_to_string(path).unwrap();
    let master = if im::is_sealed(&text) {
        im::open_seed(&text, &std::env::var(PASS_ENV).expect("a sealed seed needs NOESIS_ISSUER_PASSPHRASE")).unwrap()
    } else {
        im::parse_seed(&text).unwrap()
    };
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
