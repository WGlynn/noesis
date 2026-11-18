//! identity_mint — the issuer's side of the soulbound identity standard ([`crate::tokens::soulbound`]).
//!
//! Off-consensus tooling: nothing here is a rule, and the block gate checks whatever this builds. The
//! issuer key lives only with whoever runs the `identity-mint` binary, never in the node or the bot.
//!
//! INPUT, a batch file written by the bot side, in exactly this shape:
//! `{"collection":"vibeswap-identity-v1","records":[{"serial":1,"holder":"<64-hex address>","tier":"identity"}]}`
//! with `tier` one of `identity`, `member`, `revoked`. Each record mirrors one registry row, so a batch
//! is idempotent: a live serial already at that tier is `unchanged`, a changed tier is a re-tier, a new
//! serial is a mint. A live serial's holder never changes; a record that names another holder is an
//! error (burn and mint a new serial instead).
//!
//! KEY CHAIN (why no mint reuses a one-time key). The authority cell at generation `g` is locked by the
//! Lamport key derived from (issuer seed, g). A mint tx spends it and re-creates it at `g + 1` under
//! key `g + 1`, so key `g` signs exactly one tx. Identity cells written by that tx are locked by
//! per-cell keys derived from (seed, g, serial); a later re-tier spends the cell once with its key and
//! re-locks it under the new generation. The binary keeps every signed tx, by generation, in its state
//! file (the [`Journal`]), so a retry, and a re-mint after a chain reset, re-sends the same signed bytes
//! instead of signing a different digest with the same key.
//!
//! LIMITS (wallet decision A8, 2026-11-18). The seed file is sealed at rest ([`seal_seed`]); revokes,
//! re-tiers and `--abandon-pending` sign only with a typed confirmation of the operation digest
//! ([`confirm_matches`]); a state file signs at most [`DEFAULT_DAILY_CAP`] operations per UTC day.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use noesis_core::lamport;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use zeroize::Zeroizing;

use crate::rpc::{hex_decode, hex_encode, token_cell_from_json};
use crate::runtime::{TokenStandard, TokenTx};
use crate::tokens::soulbound::{self, Identity, Record, Tier};
use crate::{Cell, Script};

/// The Vibeswap chat identity collection (`type_script.args`). A readable name works because soulbound
/// authority is the collection's linear authority cell, not a lock key equal to this id.
pub const COLLECTION: &[u8] = b"vibeswap-identity-v1";

/// The PUBLIC devnet issuer seed: anyone can mint on a devnet with it, which is what a devnet is for.
/// The testnet authority is locked by a key whose seed is kept outside the repository.
pub const DEV_ISSUER_SEED: [u8; 32] = *b"noesis-dev-identity-issuer-seed!";

/// Per-tx caps, so a signed tx fits the node's request body cap ([`crate::rpc::MAX_BODY`]): each spent
/// input carries a 16 KiB one-time signature (32 KiB as hex). Records over the caps are reported
/// `deferred` and go in the next run. `worst_case_envelope_fits_the_body_cap` pins the fit.
pub const MAX_MINTS_PER_TX: usize = 64;
pub const MAX_RETIERS_PER_TX: usize = 2;

fn derive(tag: &[u8], master: &[u8; 32], generation: u64, serial: Option<u64>) -> [u8; 32] {
    let mut buf = tag.to_vec();
    buf.extend_from_slice(master);
    buf.extend_from_slice(&generation.to_le_bytes());
    if let Some(s) = serial {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    noesis_core::pow::hash(&buf)
}

/// The one-time key that locks the authority cell at `generation`.
pub fn authority_key(master: &[u8; 32], generation: u64) -> [u8; 32] {
    derive(b"noesis-sbt-authority-key-v1", master, generation, None)
}

/// The one-time key that locks identity `serial` as written by the tx spending authority `generation`.
pub fn cell_key(master: &[u8; 32], generation: u64, serial: u64) -> [u8; 32] {
    derive(b"noesis-sbt-cell-key-v1", master, generation, Some(serial))
}

/// The authority's lock root (`lock.args`) at `generation`.
pub fn authority_root(master: &[u8; 32], generation: u64) -> [u8; 32] {
    lamport::keygen_root(&authority_key(master, generation))
}

/// Conventional cell id for a record written at authority `generation` (a tool convention, not a
/// rule): distinct per (collection, serial or authority, generation), outside the reserved coinbase
/// half of the id space.
pub fn cell_id(collection: &[u8], serial: Option<u64>, generation: u64) -> u64 {
    let mut buf = b"noesis-sbt-cell-id-v1".to_vec();
    noesis_core::pow::put(&mut buf, collection);
    match serial {
        Some(s) => {
            buf.push(1);
            buf.extend_from_slice(&s.to_le_bytes());
        }
        None => buf.push(0),
    }
    buf.extend_from_slice(&generation.to_le_bytes());
    let h = noesis_core::pow::hash(&buf);
    u64::from_le_bytes(h[..8].try_into().expect("8 bytes")) & !crate::jul::COINBASE_ID_BIT
}

fn sb_cell(id: u64, lock_root: [u8; 32], collection: &[u8], data: Vec<u8>) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: lock_root.to_vec() },
        type_script: Script { code_hash: soulbound::CODE_HASH, args: collection.to_vec() },
        parent: None,
        timestamp: 0,
        data,
    }
}

/// The authority cell at `generation`, locked by `lock_root`. Genesis seeds generation 0.
pub fn authority_cell(collection: &[u8], lock_root: [u8; 32], generation: u64) -> Cell {
    sb_cell(cell_id(collection, None, generation), lock_root, collection, soulbound::encode_authority(generation))
}

/// Read a plain-text seed file: 64 hex characters (surrounding whitespace ignored). The faucet's seed
/// file uses this form; the issuer's is sealed once with [`encrypt_seed_file`].
pub fn parse_seed(text: &str) -> Result<[u8; 32], String> {
    let b = Zeroizing::new(hex_decode(text.trim()).ok_or("seed file is not hex")?);
    <[u8; 32]>::try_from(b.as_slice()).map_err(|_| "seed must be 32 bytes (64 hex characters)".to_string())
}

// ============ the seed file, sealed at rest (wallet decision A8) ============
//
// A sealed seed file is one JSON object:
//   {"sealed":"noesis-issuer-seed-v1","authority_root":"<64 hex>","kdf":"PBKDF2-SHA256","iter":600000,
//    "salt":"<32 hex>","iv":"<24 hex>","ct":"<96 hex>"}
// ct = AES-256-GCM(key = PBKDF2-SHA256(passphrase, salt, iter), nonce = iv, plaintext = the 32-byte seed,
// associated data = "noesis-issuer-seed-v1|" + authority_root), the web wallet's scheme
// (`frontend/wallet.js`), with no separate data key because there is one unlock path. authority_root is
// the seed's public generation-0 root, in the clear so a person can tell which key a file holds; it is
// bound as associated data and checked against the opened seed.

/// PBKDF2-SHA256 rounds for a sealed seed: the web wallet's figure (`frontend/wallet.js` `PBKDF2_ITER`,
/// OWASP's for PBKDF2-HMAC-SHA256). The count is stored in each file, so it can rise later.
pub const SEED_KDF_ITER: u32 = 600_000;
/// The shortest passphrase a new seal takes (the web wallet's `MIN_PASS`).
pub const MIN_PASSPHRASE_CHARS: usize = 12;
const SEALED_FORMAT: &str = "noesis-issuer-seed-v1";
/// A stored round count above this is refused, so a file cannot make the tool spin for hours.
const MAX_KDF_ITER: u32 = 10_000_000;

fn kek(pass: &str, salt: &[u8], iter: u32) -> Zeroizing<[u8; 32]> {
    let mut k = Zeroizing::new([0u8; 32]);
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(pass.as_bytes(), salt, iter, k.as_mut());
    k
}

fn random<const N: usize>() -> Result<[u8; N], String> {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b).map_err(|e| format!("no system randomness: {e}"))?;
    Ok(b)
}

fn sealed_aad(root: &[u8; 32]) -> Vec<u8> {
    format!("{SEALED_FORMAT}|{}", hex_encode(root)).into_bytes()
}

struct Sealed {
    root: [u8; 32],
    iter: u32,
    salt: Vec<u8>,
    iv: [u8; 12],
    ct: Vec<u8>,
}

/// Whether a seed file's text is the sealed form (a JSON object) rather than 64 hex characters.
pub fn is_sealed(text: &str) -> bool {
    text.trim_start().starts_with('{')
}

/// A new passphrase must be at least [`MIN_PASSPHRASE_CHARS`] characters.
pub fn check_new_passphrase(pass: &str) -> Result<(), String> {
    if pass.chars().count() < MIN_PASSPHRASE_CHARS {
        return Err(format!(
            "the passphrase must be at least {MIN_PASSPHRASE_CHARS} characters (four or more random words is a good shape)"
        ));
    }
    Ok(())
}

fn seal_with(master: &[u8; 32], key: &[u8; 32], salt: &[u8], iter: u32) -> Result<String, String> {
    let root = authority_root(master, 0);
    let iv: [u8; 12] = random()?;
    let ct = Aes256Gcm::new_from_slice(key)
        .map_err(|_| "bad key length")?
        .encrypt(Nonce::from_slice(&iv), Payload { msg: master, aad: &sealed_aad(&root) })
        .map_err(|_| "encryption failed")?;
    Ok(json!({
        "sealed": SEALED_FORMAT,
        "authority_root": hex_encode(&root),
        "kdf": "PBKDF2-SHA256",
        "iter": iter,
        "salt": hex_encode(salt),
        "iv": hex_encode(&iv),
        "ct": hex_encode(&ct),
    })
    .to_string())
}

fn parse_sealed(text: &str) -> Result<Sealed, String> {
    let v: Value = serde_json::from_str(text).map_err(|_| "the seed file is neither 64 hex characters nor a sealed seed")?;
    if v.get("sealed").and_then(Value::as_str) != Some(SEALED_FORMAT) || v.get("kdf").and_then(Value::as_str) != Some("PBKDF2-SHA256") {
        return Err(format!("the seed file is not a {SEALED_FORMAT} sealed seed (PBKDF2-SHA256)"));
    }
    let iter = v
        .get("iter")
        .and_then(Value::as_u64)
        .filter(|i| (1..=u64::from(MAX_KDF_ITER)).contains(i))
        .ok_or(format!("sealed seed: 'iter' must be 1..={MAX_KDF_ITER}"))? as u32;
    let field = |f: &str| v.get(f).and_then(Value::as_str).and_then(hex_decode).ok_or(format!("sealed seed: '{f}' missing or not hex"));
    let fixed = |f: &str, len: usize| field(f).and_then(|b| if b.len() == len { Ok(b) } else { Err(format!("sealed seed: '{f}' must be {len} bytes")) });
    let salt = field("salt")?;
    if salt.len() < 16 {
        return Err("sealed seed: 'salt' must be at least 16 bytes".into());
    }
    Ok(Sealed {
        root: fixed("authority_root", 32)?.try_into().expect("32"),
        iter,
        salt,
        iv: fixed("iv", 12)?.try_into().expect("12"),
        ct: fixed("ct", 48)?,
    })
}

fn open_with(s: &Sealed, key: &[u8; 32]) -> Result<[u8; 32], String> {
    let plain = Zeroizing::new(
        Aes256Gcm::new_from_slice(key)
            .map_err(|_| "bad key length")?
            .decrypt(Nonce::from_slice(&s.iv), Payload { msg: &s.ct, aad: &sealed_aad(&s.root) })
            .map_err(|_| "the passphrase does not open this seed file (or the file was changed)")?,
    );
    let master: [u8; 32] = plain.as_slice().try_into().map_err(|_| "the sealed seed is not 32 bytes")?;
    if authority_root(&master, 0) != s.root {
        return Err("the sealed seed does not hold the authority root written beside it (the file was changed)".into());
    }
    Ok(master)
}

/// Seal `master` under `pass` with a fresh random salt and nonce: the text of a sealed seed file.
pub fn seal_seed(master: &[u8; 32], pass: &str, iter: u32) -> Result<String, String> {
    check_new_passphrase(pass)?;
    let salt: [u8; 16] = random()?;
    seal_with(master, &kek(pass, &salt, iter), &salt, iter)
}

/// Open a sealed seed file's text with `pass`.
pub fn open_seed(text: &str, pass: &str) -> Result<[u8; 32], String> {
    let s = parse_sealed(text)?;
    open_with(&s, &kek(pass, &s.salt, s.iter))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

/// Write `text` to `path` (created or truncated in place) and flush it to the disk.
pub fn write_synced(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()
}

/// The one-time migration (`identity-mint encrypt-seed`): seal a plain-text seed file in place, the
/// web wallet's order. Encrypt, decrypt and compare; write a sealed copy beside the file and read it
/// back; only then overwrite the file itself (the same file, so its permissions stay), read it back,
/// and remove the copy. Any failure before the overwrite leaves the plain file as it was; a failure
/// during it names the verified sealed copy. Returns the seed's generation-0 authority root.
/// Best effort only against the disk: old copies of the plain text elsewhere (backups, the filesystem's
/// free blocks, an SSD's remapped cells) are not touched.
pub fn encrypt_seed_file(path: &Path, pass: &str, iter: u32) -> Result<[u8; 32], String> {
    let shown = path.display();
    let before = Zeroizing::new(std::fs::read_to_string(path).map_err(|e| format!("cannot read {shown}: {e}"))?);
    if is_sealed(&before) {
        return Err(format!("{shown} is already sealed; nothing changed"));
    }
    let master = Zeroizing::new(parse_seed(&before).map_err(|e| format!("{shown}: {e}; nothing changed"))?);
    check_new_passphrase(pass)?;
    let salt: [u8; 16] = random()?;
    let key = kek(pass, &salt, iter);
    let text = seal_with(&master, &key, &salt, iter)?;
    // A sealed text opens with this passphrase's key (same salt, same rounds) to this very seed.
    let opens = |t: &str| parse_sealed(t).ok().filter(|s| s.salt == salt && s.iter == iter).and_then(|s| open_with(&s, &key).ok()) == Some(*master);
    if !opens(&text) {
        return Err("the sealed copy did not decrypt to the same seed; nothing changed".into());
    }
    let copy = sibling(path, ".sealing");
    let fail_copy = |msg: String| {
        let _ = std::fs::remove_file(&copy);
        Err(msg)
    };
    if let Err(e) = write_synced(&copy, &text) {
        return fail_copy(format!("cannot write {}: {e}; nothing changed", copy.display()));
    }
    if std::fs::read_to_string(&copy).ok().as_deref() != Some(text.as_str()) {
        return fail_copy(format!("{} did not read back as written; nothing changed", copy.display()));
    }
    if std::fs::read_to_string(path).ok().as_deref() != Some(before.as_str()) {
        return fail_copy(format!("{shown} changed during the conversion; nothing replaced"));
    }
    let kept = format!("the verified sealed copy is {}: copy it over {shown}", copy.display());
    write_synced(path, &text).map_err(|e| format!("cannot rewrite {shown}: {e}; {kept}"))?;
    let back = std::fs::read_to_string(path).map_err(|e| format!("cannot read {shown} back: {e}; {kept}"))?;
    if back != text || !opens(&back) {
        return Err(format!("{shown} did not read back as sealed; {kept}"));
    }
    std::fs::remove_file(&copy).map_err(|e| format!("sealed {shown}, but could not remove {}: {e}", copy.display()))?;
    Ok(authority_root(&master, 0))
}

// ============ batch ============

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRecord {
    pub serial: u64,
    pub holder: [u8; 32],
    pub tier: Tier,
}

/// Parse and check a batch. The collection must match; serials must be unique; holders are 64 hex.
pub fn parse_batch(text: &str, collection: &[u8]) -> Result<Vec<BatchRecord>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("batch is not JSON: {e}"))?;
    let col = v.get("collection").and_then(Value::as_str).ok_or("batch needs a 'collection' string")?;
    if col.as_bytes() != collection {
        return Err(format!("batch collection {col:?} is not {:?}", String::from_utf8_lossy(collection)));
    }
    let recs = v.get("records").and_then(Value::as_array).ok_or("batch needs a 'records' array")?;
    let mut out = Vec::with_capacity(recs.len());
    let mut seen = std::collections::HashSet::new();
    for (i, r) in recs.iter().enumerate() {
        let serial = r.get("serial").and_then(Value::as_u64).ok_or(format!("record {i}: 'serial' must be a non-negative integer"))?;
        let holder_hex = r.get("holder").and_then(Value::as_str).ok_or(format!("record {i}: 'holder' must be a string"))?;
        let holder = hex_decode(holder_hex)
            .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
            .ok_or(format!("record {i}: 'holder' must be a 64-hex Noesis address"))?;
        let tier_name = r.get("tier").and_then(Value::as_str).ok_or(format!("record {i}: 'tier' must be a string"))?;
        let tier = Tier::from_name(tier_name).ok_or(format!("record {i}: tier {tier_name:?} is not identity, member or revoked"))?;
        if !seen.insert(serial) {
            return Err(format!("serial {serial} appears twice in the batch"));
        }
        out.push(BatchRecord { serial, holder, tier });
    }
    Ok(out)
}

// ============ live state, read from the node's GET /tokens ============

pub struct Live {
    pub authority: Cell,
    pub generation: u64,
    pub identities: HashMap<u64, (Cell, Identity)>,
}

/// The collection's live authority and identities, from a `GET /tokens` response.
pub fn parse_live(tokens: &Value, collection: &[u8]) -> Result<Live, String> {
    let cells = tokens.get("cells").and_then(Value::as_array).ok_or("/tokens response has no 'cells'")?;
    let mut authority = None;
    let mut identities = HashMap::new();
    for v in cells {
        let c = token_cell_from_json(v)?;
        if c.type_script.code_hash != soulbound::CODE_HASH || c.type_script.args != collection {
            continue;
        }
        match soulbound::decode(&c.data) {
            Some(Record::Authority { generation }) => {
                if authority.is_some() {
                    return Err("two live authority cells for the collection".into());
                }
                authority = Some((c, generation));
            }
            Some(Record::Identity(i)) => {
                identities.insert(i.serial, (c, i));
            }
            None => return Err(format!("cell {} of the collection is not a record", c.id)),
        }
    }
    let (authority, generation) = authority.ok_or("the chain has no live authority for this collection")?;
    Ok(Live { authority, generation, identities })
}

// ============ plan, build, sign ============

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Mint,
    Retier,
    Unchanged,
    Deferred,
}

impl Action {
    /// The status word printed once the tx has landed.
    pub fn done(self) -> &'static str {
        match self {
            Action::Mint => "minted",
            Action::Retier => "retiered",
            Action::Unchanged => "unchanged",
            Action::Deferred => "deferred",
        }
    }

    /// The status word a dry run prints: what a real run would do.
    pub fn planned(self) -> &'static str {
        match self {
            Action::Mint => "mint",
            Action::Retier => "retier",
            Action::Unchanged => "unchanged",
            Action::Deferred => "deferred",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Planned {
    pub serial: u64,
    pub tier: Tier,
    pub action: Action,
    /// The new cell's id (mint, re-tier), the live cell's id (unchanged), none (deferred).
    pub cell_id: Option<u64>,
}

/// A planned tx: unsigned until [`Built::sign`]. `keys[i]` signs `tx.inputs[i]`.
pub struct Built {
    pub generation: u64,
    pub tx: Option<TokenTx>,
    pub plan: Vec<Planned>,
    /// Records deferred because the day's budget ran out (not the per-tx caps).
    pub over_budget: usize,
    keys: Vec<[u8; 32]>,
}

impl Built {
    /// The digest the issuer keys sign (`None` when there is nothing to do).
    pub fn digest(&self) -> Option<[u8; 32]> {
        self.tx.as_ref().map(TokenTx::digest)
    }

    /// Issuer operations in the tx: each mint and each re-tier (a revoke is a re-tier) counts one.
    pub fn ops(&self) -> usize {
        self.plan.iter().filter(|p| matches!(p.action, Action::Mint | Action::Retier)).count()
    }

    /// Sign every input with its one-time key. Call once per generation (see the module note).
    pub fn sign(&mut self) {
        if let Some(tx) = self.tx.as_mut() {
            let d = tx.digest();
            tx.auths = self.keys.iter().map(|k| lamport::sign(k, &d)).collect();
        }
    }
}

/// Turn a batch into one soulbound tx against the live state. Fails (nothing signed) if the seed does
/// not hold the live authority, a record names a new holder for a live serial, a holder would get a
/// second identity, or no key from this seed locks a cell that needs re-tiering.
pub fn build(records: &[BatchRecord], live: &Live, master: &[u8; 32], collection: &[u8]) -> Result<Built, String> {
    build_within(records, live, master, collection, usize::MAX)
}

/// [`build`] with at most `budget` operations (mints plus re-tiers): what is over it is `deferred`
/// and counted in [`Built::over_budget`]. The tool passes what is left of the day's cap.
pub fn build_within(records: &[BatchRecord], live: &Live, master: &[u8; 32], collection: &[u8], budget: usize) -> Result<Built, String> {
    let g = live.generation;
    let ak = authority_key(master, g);
    if lamport::keygen_root(&ak).as_slice() != live.authority.lock.args.as_slice() {
        return Err(format!("this seed does not hold the issuer authority (live at generation {g})"));
    }
    let next = g.checked_add(1).ok_or("authority generation is exhausted")?;
    let mut inputs = vec![live.authority.clone()];
    let mut keys = vec![ak];
    let mut outputs = vec![authority_cell(collection, authority_root(master, next), next)];
    let mut holder_of: HashMap<[u8; 32], u64> = live.identities.values().map(|(_, i)| (i.holder, i.serial)).collect();
    let mut plan = Vec::with_capacity(records.len());
    let (mut mints, mut retiers, mut over_budget) = (0usize, 0usize, 0usize);
    let new_cell = |serial: u64, tier: Tier, holder: [u8; 32]| {
        let data = soulbound::encode_identity(&Identity { serial, tier, holder });
        let root = lamport::keygen_root(&cell_key(master, g, serial));
        sb_cell(cell_id(collection, Some(serial), g), root, collection, data)
    };
    for r in records {
        let planned = |action, cell_id| Planned { serial: r.serial, tier: r.tier, action, cell_id };
        match live.identities.get(&r.serial) {
            Some((cell, cur)) => {
                if cur.holder != r.holder {
                    return Err(format!(
                        "serial {}: the chain holds it for a different address, and a holder never changes \
                         (burn it and mint a new serial)",
                        r.serial
                    ));
                }
                if cur.tier == r.tier {
                    plan.push(planned(Action::Unchanged, Some(cell.id)));
                    continue;
                }
                if retiers == MAX_RETIERS_PER_TX {
                    plan.push(planned(Action::Deferred, None));
                    continue;
                }
                if mints + retiers == budget {
                    over_budget += 1;
                    plan.push(planned(Action::Deferred, None));
                    continue;
                }
                let key = (0..g)
                    .rev()
                    .map(|wg| cell_key(master, wg, r.serial))
                    .find(|k| lamport::keygen_root(k).as_slice() == cell.lock.args.as_slice())
                    .ok_or(format!("serial {}: no key from this seed locks the live cell", r.serial))?;
                inputs.push(cell.clone());
                keys.push(key);
                let out = new_cell(r.serial, r.tier, r.holder);
                plan.push(planned(Action::Retier, Some(out.id)));
                outputs.push(out);
                retiers += 1;
            }
            None => {
                if let Some(&other) = holder_of.get(&r.holder) {
                    return Err(format!("serial {}: this address already holds serial {other}", r.serial));
                }
                if mints == MAX_MINTS_PER_TX {
                    plan.push(planned(Action::Deferred, None));
                    continue;
                }
                if mints + retiers == budget {
                    over_budget += 1;
                    plan.push(planned(Action::Deferred, None));
                    continue;
                }
                holder_of.insert(r.holder, r.serial);
                let out = new_cell(r.serial, r.tier, r.holder);
                plan.push(planned(Action::Mint, Some(out.id)));
                outputs.push(out);
                mints += 1;
            }
        }
    }
    let tx = (mints + retiers > 0).then(|| TokenTx {
        standard: TokenStandard::Soulbound,
        code_hash: soulbound::CODE_HASH,
        args: collection.to_vec(),
        inputs,
        outputs,
        auths: Vec::new(),
    });
    Ok(Built { generation: g, tx, plan, over_budget, keys })
}

// ============ confirmation (wallet decision A8) ============

/// The shortest `--confirm` value: 8 hex characters (32 bits) of the operation digest.
pub const MIN_CONFIRM_HEX: usize = 8;

/// Why a plan needs the issuer's typed confirmation, if it does: it re-tiers or revokes a live identity,
/// or mints a serial as revoked (which bars that address from an identity, so it is a revoke too).
/// Mints at the identity and member tiers need none.
pub fn needs_confirm(plan: &[Planned]) -> Option<&'static str> {
    plan.iter()
        .any(|p| p.action == Action::Retier || (p.action == Action::Mint && p.tier == Tier::Revoked))
        .then_some("it re-tiers or revokes an identity")
}

/// A `--confirm` value must be at least [`MIN_CONFIRM_HEX`] hex characters and a prefix of the digest
/// of the exact tx about to be signed, the one a `--dry-run` of the same batch printed.
pub fn confirm_matches(digest: &[u8; 32], confirm: Option<&str>) -> Result<(), String> {
    let c = confirm.ok_or("no --confirm given")?.trim().to_ascii_lowercase();
    if c.len() < MIN_CONFIRM_HEX || !c.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("--confirm takes at least {MIN_CONFIRM_HEX} hex characters of the operation digest"));
    }
    if !hex_encode(digest).starts_with(&c) {
        return Err(format!("--confirm {c} is not this operation's digest (it is {})", hex_encode(digest)));
    }
    Ok(())
}

// ============ the state file: what each one-time key signed, and the day's count (wallet decision A8) ============

/// Issuer operations (each mint, re-tier or revoke counts one) a state file signs per UTC day, unless
/// the issuer passes a confirmed higher `--daily-cap`. 64 = one full mint tx ([`MAX_MINTS_PER_TX`]): a
/// normal day's batch from the chat never waits, and a run gone wrong writes at most 64 records a day
/// (re-tiers and revokes also need a typed confirmation). A testnet figure, not a measured one.
pub const DEFAULT_DAILY_CAP: usize = MAX_MINTS_PER_TX;

/// One signature event: the digest, the UTC day (days since 1970-01-01) and its operation count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signing {
    pub digest: String,
    pub utc_day: u64,
    pub ops: usize,
}

/// What the keys of one authority generation signed. `envelope` is the `POST /token-tx` body, re-sent
/// byte for byte (after a reset too) instead of a new signature.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signed {
    pub generation: u64,
    pub digest: String,
    pub envelope: Value,
    pub utc_day: u64,
    pub ops: usize,
    /// Digests these keys signed before, each given up with `--abandon-pending`. Every one weakens the
    /// keys (two Lamport signatures from one key let a third party forge others).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub earlier: Vec<Signing>,
}

/// The state file (`<seed-file>.pending.json` by default; the name is kept from before A8): every tx
/// this seed's authority keys signed, by generation, on any chain. The tool never deletes it. It is not
/// secret and not authenticated: it bounds the tool, not someone who can rewrite or delete the file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Journal {
    pub v: u32,
    /// The generation-0 authority root of the seed this file belongs to.
    pub authority_root: String,
    /// The highest live authority generation the tool has seen, on any chain: every key below it signed.
    pub seen_generation: u64,
    pub signed: Vec<Signed>,
}

/// What the journal allows at the live authority generation.
#[derive(Debug)]
pub enum Step<'a> {
    /// No key of this generation has signed: a new tx may be signed.
    Fresh,
    /// This generation's keys signed this tx (here, or on a chain since reset): re-send it.
    Recorded(&'a Signed),
    /// This generation's keys signed a tx the journal holds no copy of: signing again would reuse them.
    Unrecorded,
}

impl Journal {
    pub fn new(master: &[u8; 32]) -> Journal {
        Journal { v: 2, authority_root: hex_encode(&authority_root(master, 0)), seen_generation: 0, signed: Vec::new() }
    }

    /// Read a state file of the seed `master`: this format, or the one-record pending file the tool kept
    /// before A8 (`{generation, digest, envelope}`).
    pub fn parse(text: &str, master: &[u8; 32]) -> Result<Journal, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
        let mine = hex_encode(&authority_root(master, 0));
        if v.get("v").is_none() {
            let g = v.get("generation").and_then(Value::as_u64).ok_or("neither a journal nor a pending record")?;
            let digest = v.get("digest").and_then(Value::as_str).ok_or("pending record without 'digest'")?.to_string();
            let envelope = v.get("envelope").cloned().ok_or("pending record without 'envelope'")?;
            let tx = from_envelope(&envelope)?;
            let ours = tx.inputs.first().is_some_and(|c| c.lock.args == authority_root(master, g));
            if !ours || hex_encode(&tx.digest()) != digest {
                return Err("the pending record belongs to another issuer seed".into());
            }
            let signed = vec![Signed { generation: g, digest, envelope, utc_day: 0, ops: 0, earlier: Vec::new() }];
            return Ok(Journal { v: 2, authority_root: mine, seen_generation: g, signed });
        }
        let j: Journal = serde_json::from_value(v).map_err(|e| format!("malformed journal: {e}"))?;
        if j.v != 2 {
            return Err(format!("journal version {} is not 2", j.v));
        }
        if j.authority_root != mine {
            return Err(format!("it belongs to another issuer seed (authority root {})", j.authority_root));
        }
        Ok(j)
    }

    pub fn to_text(&self) -> String {
        serde_json::to_string(self).expect("a journal serializes")
    }

    pub fn step(&self, g: u64) -> Step<'_> {
        if let Some(s) = self.signed.iter().find(|s| s.generation == g) {
            return Step::Recorded(s);
        }
        if g < self.seen_generation || self.signed.iter().any(|s| s.generation > g) {
            return Step::Unrecorded;
        }
        Step::Fresh
    }

    /// Note that a chain showed the authority at generation `g`. True when that is news.
    pub fn see(&mut self, g: u64) -> bool {
        let news = g > self.seen_generation;
        self.seen_generation = self.seen_generation.max(g);
        news
    }

    /// Operations signed on `day`, leaving out the latest tx of generation `except` (the one a run at
    /// that generation is about to re-send or compare with, so it does not count against itself).
    pub fn ops_on(&self, day: u64, except: Option<u64>) -> usize {
        let mut n = 0;
        for s in &self.signed {
            if s.utc_day == day && Some(s.generation) != except {
                n += s.ops;
            }
            n += s.earlier.iter().filter(|e| e.utc_day == day).map(|e| e.ops).sum::<usize>();
        }
        n
    }

    /// Record a newly signed tx for generation `g`. A tx already recorded for `g` (abandoned) moves to
    /// `earlier`; the new one is what a later run re-sends.
    pub fn record(&mut self, g: u64, digest: String, envelope: Value, utc_day: u64, ops: usize) {
        self.see(g);
        match self.signed.iter_mut().find(|s| s.generation == g) {
            Some(s) => {
                s.earlier.push(Signing { digest: std::mem::take(&mut s.digest), utc_day: s.utc_day, ops: s.ops });
                (s.digest, s.envelope, s.utc_day, s.ops) = (digest, envelope, utc_day, ops);
            }
            None => {
                self.signed.push(Signed { generation: g, digest, envelope, utc_day, ops, earlier: Vec::new() });
                self.signed.sort_by_key(|s| s.generation);
            }
        }
    }
}

// ============ the POST /token-tx envelope ============

/// The request body for `POST /token-tx`: the tx in the block codec's shape with its signatures moved
/// out as hex (half the size of the codec's number arrays).
pub fn envelope(tx: &TokenTx) -> Value {
    let auths: Vec<String> = tx.auths.iter().map(|a| hex_encode(a)).collect();
    let mut bare = tx.clone();
    bare.auths = Vec::new();
    json!({ "tx": crate::wire::token_tx_to_json(&bare), "auths": auths })
}

/// Parse a `POST /token-tx` body back into a tx.
pub fn from_envelope(v: &Value) -> Result<TokenTx, String> {
    let mut tx = crate::wire::token_tx_from_json(v.get("tx").cloned().ok_or("missing 'tx'")?).map_err(|e| e.to_string())?;
    if !tx.auths.is_empty() {
        return Err("signatures go in the envelope's 'auths' (hex), not inside 'tx'".into());
    }
    let auths = v.get("auths").and_then(Value::as_array).ok_or("missing 'auths'")?;
    tx.auths = auths
        .iter()
        .map(|a| a.as_str().and_then(hex_decode).ok_or("an 'auths' entry is not hex".to_string()))
        .collect::<Result<_, _>>()?;
    Ok(tx)
}

/// One output line per record: what happened to it, and where the cell is.
pub fn report(p: &Planned, status: &str, block_height: Option<u64>, collection: &[u8]) -> Value {
    json!({
        "collection": String::from_utf8_lossy(collection),
        "serial": p.serial,
        "tier": p.tier.name(),
        "status": status,
        "cell_id": p.cell_id,
        "block_height": block_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOLDER: &str = "abababababababababababababababababababababababababababababababab";

    #[test]
    fn batch_parses_the_bot_format_and_rejects_bad_records() {
        let ok = format!(
            r#"{{"collection":"vibeswap-identity-v1","records":[{{"serial":1,"holder":"{HOLDER}","tier":"identity"}},{{"serial":2,"holder":"{}","tier":"revoked"}}]}}"#,
            "cd".repeat(32)
        );
        let recs = parse_batch(&ok, COLLECTION).expect("the bot's format parses");
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[1].tier, Tier::Revoked);
        let bad = |records: &str| parse_batch(&format!(r#"{{"collection":"vibeswap-identity-v1","records":{records}}}"#), COLLECTION);
        assert!(bad(&format!(r#"[{{"serial":1,"holder":"{HOLDER}","tier":"admin"}}]"#)).is_err(), "unknown tier");
        assert!(bad(r#"[{"serial":1,"holder":"abcd","tier":"identity"}]"#).is_err(), "short holder");
        assert!(bad(&format!(r#"[{{"serial":-1,"holder":"{HOLDER}","tier":"identity"}}]"#)).is_err(), "negative serial");
        assert!(
            bad(&format!(r#"[{{"serial":1,"holder":"{HOLDER}","tier":"identity"}},{{"serial":1,"holder":"{HOLDER}","tier":"member"}}]"#)).is_err(),
            "duplicate serial"
        );
        assert!(parse_batch(&ok.replace("vibeswap-identity-v1", "other-v1"), COLLECTION).is_err(), "wrong collection");
    }

    #[test]
    fn key_chain_keys_are_distinct_per_generation_and_serial() {
        let m = DEV_ISSUER_SEED;
        assert_ne!(authority_key(&m, 0), authority_key(&m, 1));
        assert_ne!(cell_key(&m, 0, 1), cell_key(&m, 1, 1));
        assert_ne!(cell_key(&m, 0, 1), cell_key(&m, 0, 2));
        assert_ne!(authority_key(&m, 0), cell_key(&m, 0, 0), "the two derivations are domain-separated");
        assert_eq!(cell_id(COLLECTION, Some(1), 0) & crate::jul::COINBASE_ID_BIT, 0, "ids stay out of the coinbase space");
        assert_eq!(parse_seed(&format!("  {}\r\n", "0f".repeat(32))).unwrap(), [0x0f; 32]);
    }

    /// A live state at authority generation 2 holding serials 1..=3, written by the generation-1 tx.
    fn live_at_gen2() -> Live {
        let m = DEV_ISSUER_SEED;
        let mut identities = HashMap::new();
        for s in 1..=3u64 {
            let i = Identity { serial: s, tier: Tier::Identity, holder: [s as u8; 32] };
            let root = lamport::keygen_root(&cell_key(&m, 1, s));
            identities.insert(s, (sb_cell(cell_id(COLLECTION, Some(s), 1), root, COLLECTION, soulbound::encode_identity(&i)), i));
        }
        Live { authority: authority_cell(COLLECTION, authority_root(&m, 2), 2), generation: 2, identities }
    }

    #[test]
    fn worst_case_envelope_fits_the_body_cap_and_overflow_is_deferred() {
        let live = live_at_gen2();
        let mut recs: Vec<BatchRecord> =
            (1..=3u64).map(|s| BatchRecord { serial: s, holder: [s as u8; 32], tier: Tier::Member }).collect();
        recs.extend((100..100 + MAX_MINTS_PER_TX as u64 + 1).map(|s| BatchRecord {
            serial: s,
            holder: noesis_core::pow::hash(&s.to_le_bytes()),
            tier: Tier::Identity,
        }));
        let mut built = build(&recs, &live, &DEV_ISSUER_SEED, COLLECTION).expect("builds");
        let count = |a: Action| built.plan.iter().filter(|p| p.action == a).count();
        assert_eq!(count(Action::Retier), MAX_RETIERS_PER_TX);
        assert_eq!(count(Action::Mint), MAX_MINTS_PER_TX);
        assert_eq!(count(Action::Deferred), 2, "the third re-tier and the 65th mint wait for the next run");
        built.sign();
        let tx = built.tx.as_ref().unwrap();
        assert_eq!(tx.auths.len(), 1 + MAX_RETIERS_PER_TX, "the authority plus each re-tiered cell signs");
        // signed for real: valid with control binding on (every input carries its signature)
        assert!(tx.is_valid_in_ledger(&[live.authority.clone(), live.identities[&1].0.clone(), live.identities[&2].0.clone()], true));
        let size = envelope(tx).to_string().len();
        assert!(size < crate::rpc::MAX_BODY, "worst-case envelope {size} bytes must fit MAX_BODY");
        // the envelope round-trips to the same signed tx
        assert_eq!(from_envelope(&envelope(tx)).unwrap().digest(), tx.digest());
    }

    #[test]
    fn build_refuses_a_foreign_seed_a_new_holder_and_a_second_identity_per_address() {
        let live = live_at_gen2();
        let r = |serial: u64, holder: u8, tier: Tier| BatchRecord { serial, holder: [holder; 32], tier };
        assert!(build(&[r(9, 9, Tier::Identity)], &live, &[7u8; 32], COLLECTION).is_err(), "wrong seed");
        assert!(build(&[r(1, 8, Tier::Member)], &live, &DEV_ISSUER_SEED, COLLECTION).is_err(), "holder change");
        assert!(build(&[r(9, 1, Tier::Identity)], &live, &DEV_ISSUER_SEED, COLLECTION).is_err(), "address 1 holds serial 1");
        let same = build(&[r(1, 1, Tier::Identity)], &live, &DEV_ISSUER_SEED, COLLECTION).unwrap();
        assert!(same.tx.is_none(), "nothing to do");
        assert_eq!(same.plan[0].action, Action::Unchanged);
    }

    // ============ wallet decision A8: the sealed seed file ============

    const PASS: &str = "correct horse battery staple";
    const FAST: u32 = 1_000; // unit tests only; the tool always seals with SEED_KDF_ITER

    #[test]
    fn the_kdf_is_pbkdf2_sha256_at_the_web_wallets_round_count() {
        // RFC 7914 section 11, PBKDF2-HMAC-SHA256 ("passwd", "salt", 1 round), first 32 bytes: the same
        // function WebCrypto's PBKDF2 computes for the web wallet.
        let k = kek("passwd", b"salt", 1);
        assert_eq!(hex_encode(k.as_ref()), "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc");
        assert_eq!(SEED_KDF_ITER, 600_000, "frontend/wallet.js PBKDF2_ITER");
        assert_eq!(MIN_PASSPHRASE_CHARS, 12, "frontend/wallet.js MIN_PASS");
    }

    #[test]
    fn a_sealed_seed_opens_only_with_its_passphrase_and_only_unchanged() {
        let m = DEV_ISSUER_SEED;
        let text = seal_seed(&m, PASS, FAST).expect("seals");
        assert!(is_sealed(&text) && !is_sealed(&hex_encode(&m)));
        assert!(!text.contains(&hex_encode(&m)), "no plain seed in the file");
        assert_eq!(open_seed(&text, PASS).unwrap(), m);
        assert!(open_seed(&text, "correct horse battery stapler").is_err(), "wrong passphrase");
        assert_ne!(seal_seed(&m, PASS, FAST).unwrap(), text, "a fresh salt and nonce every time");
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["authority_root"], hex_encode(&authority_root(&m, 0)), "the public root, in the clear");
        assert_eq!((v["kdf"].as_str(), v["iter"].as_u64()), (Some("PBKDF2-SHA256"), Some(FAST as u64)));
        let edit = |field: &str, val: Value| {
            let mut w = v.clone();
            w[field] = val;
            open_seed(&w.to_string(), PASS)
        };
        let mut ct = hex_decode(v["ct"].as_str().unwrap()).unwrap();
        ct[0] ^= 1;
        assert!(edit("ct", json!(hex_encode(&ct))).is_err(), "a changed ciphertext");
        assert!(edit("authority_root", json!(hex_encode(&authority_root(&[7; 32], 0)))).is_err(), "a swapped root (bound as associated data)");
        assert!(edit("iter", json!(FAST + 1)).is_err(), "a changed round count");
        assert!(edit("iter", json!(u64::from(u32::MAX))).is_err(), "a round count no file may ask for");
        assert!(seal_seed(&m, "short pass", FAST).is_err(), "under 12 characters");
    }

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("noesis-a8-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn encrypt_seed_file_converts_in_place_and_verifies_before_the_plain_text_goes() {
        let dir = tmp_dir("encrypt");
        let path = dir.join("issuer.seed");
        let m = [0x5a; 32];
        std::fs::write(&path, format!("{}\r\n", hex_encode(&m))).unwrap();
        let root = encrypt_seed_file(&path, PASS, FAST).expect("converts");
        assert_eq!(root, authority_root(&m, 0));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(is_sealed(&text) && !text.contains(&hex_encode(&m)), "the plain text is gone from the file");
        assert_eq!(open_seed(&text, PASS).unwrap(), m);
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(left, vec![std::ffi::OsString::from("issuer.seed")], "no temporary copy left behind");
        // a second run, a short passphrase and a file that is not a seed all change nothing
        assert!(encrypt_seed_file(&path, PASS, FAST).unwrap_err().contains("already sealed"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        let plain = dir.join("plain.seed");
        std::fs::write(&plain, hex_encode(&m)).unwrap();
        assert!(encrypt_seed_file(&plain, "too short", FAST).is_err());
        assert_eq!(std::fs::read_to_string(&plain).unwrap(), hex_encode(&m));
        let junk = dir.join("junk.seed");
        std::fs::write(&junk, "not a seed").unwrap();
        assert!(encrypt_seed_file(&junk, PASS, FAST).is_err());
        assert_eq!(std::fs::read_to_string(&junk).unwrap(), "not a seed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ============ wallet decision A8: confirmation and the daily cap ============

    #[test]
    fn re_tiers_revokes_and_revoked_mints_need_confirmation_and_mints_do_not() {
        let live = live_at_gen2();
        let r = |serial: u64, holder: u8, tier: Tier| BatchRecord { serial, holder: [holder; 32], tier };
        let plan = |recs: &[BatchRecord]| build(recs, &live, &DEV_ISSUER_SEED, COLLECTION).unwrap().plan;
        assert_eq!(needs_confirm(&plan(&[r(9, 9, Tier::Identity), r(10, 10, Tier::Member)])), None, "mints stay as before");
        assert!(needs_confirm(&plan(&[r(1, 1, Tier::Member)])).is_some(), "a re-tier");
        assert!(needs_confirm(&plan(&[r(1, 1, Tier::Revoked)])).is_some(), "a revoke");
        assert!(needs_confirm(&plan(&[r(9, 9, Tier::Revoked)])).is_some(), "a mint as revoked bars that address");
        let d = noesis_core::pow::hash(b"an operation");
        let h = hex_encode(&d);
        assert!(confirm_matches(&d, Some(&h[..MIN_CONFIRM_HEX])).is_ok());
        assert!(confirm_matches(&d, Some(&h)).is_ok(), "the whole digest");
        assert!(confirm_matches(&d, Some(&h[..MIN_CONFIRM_HEX].to_uppercase())).is_ok(), "case does not matter");
        assert!(confirm_matches(&d, Some(&h[..MIN_CONFIRM_HEX - 1])).is_err(), "too short to mean anything");
        let other = hex_encode(&noesis_core::pow::hash(b"another operation"));
        assert_ne!(other[..MIN_CONFIRM_HEX], h[..MIN_CONFIRM_HEX]);
        assert!(confirm_matches(&d, Some(&other[..MIN_CONFIRM_HEX])).is_err(), "another operation's digest");
        assert!(confirm_matches(&d, None).is_err());
    }

    #[test]
    fn the_daily_budget_defers_what_is_over_it() {
        let live = live_at_gen2();
        let recs: Vec<BatchRecord> =
            (9..12u64).map(|s| BatchRecord { serial: s, holder: [s as u8; 32], tier: Tier::Identity }).collect();
        let two = build_within(&recs, &live, &DEV_ISSUER_SEED, COLLECTION, 2).unwrap();
        assert_eq!((two.ops(), two.over_budget), (2, 1));
        assert_eq!(two.plan[2].action, Action::Deferred);
        let none = build_within(&recs, &live, &DEV_ISSUER_SEED, COLLECTION, 0).unwrap();
        assert!(none.tx.is_none());
        assert_eq!(none.over_budget, 3);
        let all = build(&recs, &live, &DEV_ISSUER_SEED, COLLECTION).unwrap();
        assert_eq!((all.ops(), all.over_budget), (3, 0));
        assert_eq!(DEFAULT_DAILY_CAP, MAX_MINTS_PER_TX, "one full mint tx a day");
    }

    // ============ wallet decision A8: the journal (no one-time key signs twice, across resets) ============

    fn env_for(g: u64, tag: &[u8]) -> (String, Value) {
        (hex_encode(&noesis_core::pow::hash(tag)), json!({ "generation": g, "tag": String::from_utf8_lossy(tag) }))
    }

    #[test]
    fn the_journal_keeps_what_each_key_signed_and_counts_the_day() {
        let m = DEV_ISSUER_SEED;
        let mut j = Journal::new(&m);
        assert!(matches!(j.step(0), Step::Fresh));
        let (d0, e0) = env_for(0, b"gen0");
        j.record(0, d0.clone(), e0.clone(), 100, 3);
        let (d1, e1) = env_for(1, b"gen1");
        j.record(1, d1.clone(), e1, 101, 2);
        assert!(j.see(2));
        // a reset brings the chain back to generation 0: its key already signed d0, so the step is to re-send it
        match j.step(0) {
            Step::Recorded(s) => assert_eq!((s.digest.as_str(), &s.envelope), (d0.as_str(), &e0)),
            _ => panic!("generation 0 is recorded"),
        }
        assert!(matches!(j.step(2), Step::Fresh));
        // the text round-trips and stays bound to its seed
        let back = Journal::parse(&j.to_text(), &m).unwrap();
        assert!(matches!(back.step(1), Step::Recorded(s) if s.digest == d1));
        assert!(Journal::parse(&j.to_text(), &[7u8; 32]).unwrap_err().contains("another issuer seed"));
        // the day's count: every signing that day, minus the one being re-sent at that generation
        assert_eq!((j.ops_on(100, None), j.ops_on(101, None), j.ops_on(102, None)), (3, 2, 0));
        assert_eq!(j.ops_on(101, Some(1)), 0);
        // --abandon-pending: the new digest becomes the record, the old one stays on file and on the day's count
        let (d1b, e1b) = env_for(1, b"gen1-again");
        j.record(1, d1b.clone(), e1b, 101, 1);
        let s = match j.step(1) {
            Step::Recorded(s) => s.clone(),
            _ => panic!(),
        };
        assert_eq!((s.digest, s.earlier.len(), s.earlier[0].digest.clone()), (d1b, 1, d1));
        assert_eq!((j.ops_on(101, None), j.ops_on(101, Some(1))), (3, 2));
    }

    #[test]
    fn a_key_that_signed_with_no_copy_kept_is_refused() {
        let m = DEV_ISSUER_SEED;
        let mut j = Journal::new(&m);
        j.see(3); // the tool saw the chain at generation 3 (keys 0..=2 signed), before it kept copies
        assert!(matches!(j.step(0), Step::Unrecorded));
        assert!(matches!(j.step(2), Step::Unrecorded));
        assert!(matches!(j.step(3), Step::Fresh));
        let mut k = Journal::new(&m);
        let (d, e) = env_for(5, b"gen5");
        k.record(5, d, e, 1, 1);
        assert!(matches!(k.step(4), Step::Unrecorded), "a later key signed, so this one did too");
    }

    #[test]
    fn the_old_one_record_state_file_reads_as_a_journal_of_its_own_seed_only() {
        let live = live_at_gen2();
        let mut built =
            build(&[BatchRecord { serial: 9, holder: [9; 32], tier: Tier::Identity }], &live, &DEV_ISSUER_SEED, COLLECTION).unwrap();
        built.sign();
        let d = hex_encode(&built.digest().unwrap());
        let old = json!({ "generation": 2, "digest": d, "envelope": envelope(built.tx.as_ref().unwrap()) }).to_string();
        let j = Journal::parse(&old, &DEV_ISSUER_SEED).expect("the pre-A8 pending record");
        assert!(matches!(j.step(2), Step::Recorded(s) if s.digest == d));
        assert!(matches!(j.step(0), Step::Unrecorded), "generations 0 and 1 signed before it");
        assert!(Journal::parse(&old, &[7u8; 32]).is_err(), "another seed's record");
    }
}
