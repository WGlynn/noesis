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
//! re-locks it under the new generation. The binary persists the one signed tx per generation next to
//! the seed, so a retry re-sends the same signed bytes instead of signing a different digest with the
//! same key.

use std::collections::HashMap;

use noesis_core::lamport;
use serde_json::{json, Value};

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

/// Read a seed file: 64 hex characters (surrounding whitespace ignored).
pub fn parse_seed(text: &str) -> Result<[u8; 32], String> {
    let b = hex_decode(text.trim()).ok_or("seed file is not hex")?;
    <[u8; 32]>::try_from(b.as_slice()).map_err(|_| "seed must be 32 bytes (64 hex characters)".to_string())
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
    keys: Vec<[u8; 32]>,
}

impl Built {
    /// The digest the issuer keys sign (`None` when there is nothing to do).
    pub fn digest(&self) -> Option<[u8; 32]> {
        self.tx.as_ref().map(TokenTx::digest)
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
    let (mut mints, mut retiers) = (0usize, 0usize);
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
    Ok(Built { generation: g, tx, plan, keys })
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
}
