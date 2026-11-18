//! The identity-key lane: one identity, a series of hot keys, and recovery with the cold key behind a
//! time lock the current hot key can cancel (wallet decisions A1 and A7, Will 2026-11-17; built
//! 2026-11-18, reference node only).
//!
//! WHY. A v2 identity is `H(hot root, cold commitment)` (`noesis_core::wallet::identity`), and PoM
//! standing is keyed on it (`type_script.args`). Before this lane the identity's hot key was fixed
//! forever, so a wallet lost its standing to key exhaustion, a lost device or a theft. Now an
//! identity-key cell records which hot key currently speaks for the identity. The identity, and with it
//! the standing, never moves; only the key behind it changes.
//!
//! THE CELL. One per identity, in `Ledger.token_cells` (never `cells`, so PoM, the novelty index and
//! finality weight never read it). Made by the identity's first key operation; before that an identity
//! has no cell and works exactly as before (its key is the birth hot root inside the identity hash).
//!   id            [`cell_id`] of the identity (deterministic; nobody chooses it)
//!   lock          code hash zero, args = the identity (the type rule authorizes, not the lock)
//!   type_script   [`CODE_HASH`], args = the identity
//!   data          [`Record`], 130 bytes:
//!     [0]        0xB1 (record version 1)
//!     [1..9]     generation u64 LE: key changes in effect (0 = the birth key)
//!     [9..41]    hot root: the key that signs for the identity now
//!     [41..73]   cold commitment: fixed at birth (it is inside the identity hash)
//!     [73..81]   hot floor u64 LE: lowest index the current hot key may sign a key operation at
//!     [81..89]   cold floor u64 LE: lowest index the cold key may sign a recovery request at
//!     [89]       0 = no recovery pending, 1 = one pending
//!     [90..122]  pending: the new hot root it names (zeros if none)
//!     [122..130] pending: the block height from which it is in effect (0 if none)
//!
//! THE OPERATIONS. Each is one token tx of standard [`TokenStandard::IdentityKey`] (tag 4): no input
//! (the identity's first operation, which must carry the two birth roots that hash to the identity) or
//! the live key cell, exactly one output (the record the operation leaves, fully determined by the
//! rule), and `auths[0]` = the encoded [`KeyOp`]. Every operation signs
//! `noesis_core::wallet::key_op_digest(identity, op, chain_id, generation, new_root, index)`.
//!   ROTATE   signed by the current hot key. In effect from the block that carries it: the new hot
//!            root signs from then on, the old one never again for this identity. Voids a pending
//!            recovery (the current key holder could cancel it anyway).
//!   RECOVER  signed by the cold key whose commitment is in the identity. Names a new hot root that
//!            takes effect at `height + recovery_delay_blocks + 1`: the current hot key has exactly
//!            `recovery_delay_blocks` blocks to cancel it. A newer request replaces a pending one and
//!            restarts the delay.
//!   CANCEL   signed by the current hot key while a recovery is pending: voids it.
//!
//! ONE-TIME KEYS. Every key is XMSS^MT (65,536 one-time leaves), so no leaf may sign two messages.
//! Counters are per key: a contribution's `lock.args` names its key generation ([`pack_label`]), a
//! hot-signed operation must use an index above every contribution of the current key and at or above
//! the record's hot floor, and a contribution in a block must sit at or above the hot floor the block
//! leaves. The cold key's floor rises past each request, so a replayed request is refused.
//!
//! WHERE EACH RULE LIVES. The operations carry their signatures, so [`check_block`] (called from
//! `runtime::validate_block`) checks all of them, and the producer, a follower, a store replay, a
//! syncing joiner and the reorg path enforce them alike. A contribution's own signature never enters a
//! block (the door checks it, `rpc::ServerState::submit_signed`), so "the retired key cannot sign" is
//! the door's check against the chain's current root; the rulebook adds what a replica can check from
//! the block alone: a contribution by an identity with a key record must carry the label of that
//! identity's current key generation.
//!
//! HONEST FLOOR. Reference node only: no on-VM script knows this standard, and the on-VM lock script's
//! `standard_of` refuses its code hash. The time lock counts blocks, and testnet blocks are made on
//! demand (one per accepted contribution), so anyone who makes blocks shortens it in wall-clock time.
//! The current hot key can cancel every recovery, so the cold key recovers a LOST hot key, not a stolen
//! one: a thief holding the hot key can rotate at once or cancel the owner's recovery.

use crate::runtime::{Block, Constitution, Ledger, TokenStandard, TokenTx};
use crate::{Cell, Script};
use noesis_core::{wallet, xmss};
use std::collections::{HashMap, HashSet};

/// The identity-key type-script id. A reserved placeholder tag (the soulbound precedent) with no on-VM
/// script behind it: these rules are the reference node's.
pub const CODE_HASH: [u8; 32] = *b"NOESIS-IDENTITY-KEY-CELL-TYPE-v1";
/// First byte of a record (its layout version).
pub const RECORD_TAG: u8 = 0xB1;
/// A record's exact length.
pub const RECORD_LEN: usize = 130;
/// First byte of an encoded key operation.
pub const OP_TAG: u8 = 0x4B;
/// Length of one encoded XMSS^MT signature: two trees of (OTS root, auth path, Lamport signature).
pub const SIG_LEN: usize = 2 * (32 + xmss::H * 32 + 256 * 64);

/// A recovery request waiting out its time lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    /// The hot root the request names.
    pub new_root: [u8; 32],
    /// The first block height at which it is in effect.
    pub effective_at: u64,
}

/// An identity's key record (the key cell's data).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub generation: u64,
    pub hot_root: [u8; 32],
    pub cold_commit: [u8; 32],
    pub hot_floor: u64,
    pub cold_floor: u64,
    pub pending: Option<Pending>,
}

impl Record {
    /// The record an identity implicitly has before its first key operation: its birth key.
    pub fn birth(hot_root: [u8; 32], cold_commit: [u8; 32]) -> Record {
        Record {
            generation: 0,
            hot_root,
            cold_commit,
            hot_floor: 0,
            cold_floor: 0,
            pending: None,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(RECORD_LEN);
        out.push(RECORD_TAG);
        out.extend_from_slice(&self.generation.to_le_bytes());
        out.extend_from_slice(&self.hot_root);
        out.extend_from_slice(&self.cold_commit);
        out.extend_from_slice(&self.hot_floor.to_le_bytes());
        out.extend_from_slice(&self.cold_floor.to_le_bytes());
        match &self.pending {
            None => {
                out.push(0);
                out.extend_from_slice(&[0u8; 40]);
            }
            Some(p) => {
                out.push(1);
                out.extend_from_slice(&p.new_root);
                out.extend_from_slice(&p.effective_at.to_le_bytes());
            }
        }
        out
    }

    /// Parse a record. Exact length and canonical bytes only (an absent pending is all zeros).
    pub fn decode(data: &[u8]) -> Option<Record> {
        if data.len() != RECORD_LEN || data[0] != RECORD_TAG {
            return None;
        }
        let u64_at = |i: usize| u64::from_le_bytes(data[i..i + 8].try_into().expect("8 bytes"));
        let a32 = |i: usize| -> [u8; 32] { data[i..i + 32].try_into().expect("32 bytes") };
        let pending = match data[89] {
            0 if data[90..130].iter().all(|&b| b == 0) => None,
            1 => Some(Pending {
                new_root: a32(90),
                effective_at: u64_at(122),
            }),
            _ => return None,
        };
        Some(Record {
            generation: u64_at(1),
            hot_root: a32(9),
            cold_commit: a32(41),
            hot_floor: u64_at(73),
            cold_floor: u64_at(81),
            pending,
        })
    }

    /// The record as it stands at block `height`: a pending recovery whose delay has run is in effect
    /// (the new hot root signs, the generation is one higher, the new key's floor is 0). Nothing needs
    /// to be sent for that to happen; the next operation on the identity starts from this state.
    pub fn at(&self, height: u64) -> Record {
        match self.pending {
            Some(p) if height >= p.effective_at => Record {
                generation: self.generation.saturating_add(1),
                hot_root: p.new_root,
                cold_commit: self.cold_commit,
                hot_floor: 0,
                cold_floor: self.cold_floor,
                pending: None,
            },
            _ => *self,
        }
    }
}

/// Which key operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rotate,
    Recover,
    Cancel,
}

impl Kind {
    pub fn byte(self) -> u8 {
        match self {
            Kind::Rotate => wallet::OP_ROTATE,
            Kind::Recover => wallet::OP_RECOVER,
            Kind::Cancel => wallet::OP_CANCEL,
        }
    }
    pub fn from_byte(b: u8) -> Option<Kind> {
        match b {
            wallet::OP_ROTATE => Some(Kind::Rotate),
            wallet::OP_RECOVER => Some(Kind::Recover),
            wallet::OP_CANCEL => Some(Kind::Cancel),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Rotate => "rotate",
            Kind::Recover => "recover",
            Kind::Cancel => "cancel",
        }
    }
    pub fn from_name(s: &str) -> Option<Kind> {
        match s {
            "rotate" => Some(Kind::Rotate),
            "recover" => Some(Kind::Recover),
            "cancel" => Some(Kind::Cancel),
            _ => None,
        }
    }
}

/// A signed key operation, as carried in the tx's `auths[0]`.
///   [0] OP_TAG  [1] kind  [2..6] index u32 LE  [6..38] new root (zeros for a cancel)
///   [38] flags: bit 0 = birth roots follow (hot root, cold commitment), bit 1 = cold root follows
///   then the birth roots (64 bytes) and the cold root (32 bytes) if flagged, then the signature
///   (bottom tree, then top tree: OTS root, auth path, Lamport signature).
pub struct KeyOp {
    pub kind: Kind,
    /// The signer's one-time index (the hot key's for a rotate or cancel, the cold key's for a recover).
    pub index: u32,
    /// The new hot root (zeros for a cancel).
    pub new_root: [u8; 32],
    /// The identity's two birth roots `(hot root, cold commitment)`, required exactly when the identity
    /// has no key cell yet, so the chain can check they hash to the identity.
    pub birth: Option<([u8; 32], [u8; 32])>,
    /// The cold key's public root (a recover only); its commitment must be the identity's.
    pub cold_root: Option<[u8; 32]>,
    pub sig: wallet::Signature,
}

fn put_tree(out: &mut Vec<u8>, t: &xmss::Signature) {
    out.extend_from_slice(&t.ots_root);
    for a in &t.auth {
        out.extend_from_slice(a);
    }
    out.extend_from_slice(&t.ots_sig);
}

fn take_tree(b: &[u8], index: u32) -> xmss::Signature {
    let mut auth = [[0u8; 32]; xmss::H];
    for (i, a) in auth.iter_mut().enumerate() {
        *a = b[32 + i * 32..64 + i * 32].try_into().expect("32 bytes");
    }
    let off = 32 + xmss::H * 32;
    xmss::Signature {
        index,
        ots_root: b[..32].try_into().expect("32 bytes"),
        auth,
        ots_sig: b[off..off + 256 * 64].to_vec(),
    }
}

impl KeyOp {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(39 + 96 + SIG_LEN);
        out.push(OP_TAG);
        out.push(self.kind.byte());
        out.extend_from_slice(&self.index.to_le_bytes());
        out.extend_from_slice(&self.new_root);
        out.push(u8::from(self.birth.is_some()) | (u8::from(self.cold_root.is_some()) << 1));
        if let Some((hot, cold)) = &self.birth {
            out.extend_from_slice(hot);
            out.extend_from_slice(cold);
        }
        if let Some(c) = &self.cold_root {
            out.extend_from_slice(c);
        }
        put_tree(&mut out, &self.sig.bottom);
        put_tree(&mut out, &self.sig.top);
        out
    }

    /// Parse an encoded operation. Exact length only; a cold root exactly on a recover; a cancel names
    /// no new root; the index is a valid one-time index.
    pub fn decode(b: &[u8]) -> Option<KeyOp> {
        if b.len() < 39 || b[0] != OP_TAG {
            return None;
        }
        let kind = Kind::from_byte(b[1])?;
        let index = u32::from_le_bytes(b[2..6].try_into().ok()?);
        if index as u64 >= wallet::MAX_SIGS {
            return None;
        }
        let new_root: [u8; 32] = b[6..38].try_into().ok()?;
        let flags = b[38];
        if flags & !0b11 != 0 {
            return None;
        }
        let (has_birth, has_cold) = (flags & 1 == 1, flags & 2 == 2);
        if has_cold != (kind == Kind::Recover) || (kind == Kind::Cancel && new_root != [0u8; 32]) {
            return None;
        }
        let mut at = 39;
        let want = at + if has_birth { 64 } else { 0 } + if has_cold { 32 } else { 0 } + SIG_LEN;
        if b.len() != want {
            return None;
        }
        let birth = if has_birth {
            let hot: [u8; 32] = b[at..at + 32].try_into().ok()?;
            let cold: [u8; 32] = b[at + 32..at + 64].try_into().ok()?;
            at += 64;
            Some((hot, cold))
        } else {
            None
        };
        let cold_root = if has_cold {
            let c: [u8; 32] = b[at..at + 32].try_into().ok()?;
            at += 32;
            Some(c)
        } else {
            None
        };
        let half = SIG_LEN / 2;
        let bottom = take_tree(&b[at..at + half], index & (xmss::LEAVES as u32 - 1));
        let top = take_tree(&b[at + half..at + SIG_LEN], index >> xmss::H);
        Some(KeyOp {
            kind,
            index,
            new_root,
            birth,
            cold_root,
            sig: noesis_core::xmss_mt::MtSignature {
                g: index,
                bottom,
                top,
            },
        })
    }
}

/// The key cell's id for `identity`: a hash of it, outside the reserved coinbase id space.
pub fn cell_id(identity: &[u8; 32]) -> u64 {
    let mut pre = b"noesis-identity-key-cell-id".to_vec();
    pre.extend_from_slice(identity);
    let h = noesis_core::pow::hash(&pre);
    u64::from_le_bytes(h[..8].try_into().expect("8 bytes")) & !crate::jul::COINBASE_ID_BIT
}

/// The key cell an identity holds with `rec`.
pub fn key_cell(identity: &[u8; 32], rec: &Record) -> Cell {
    Cell {
        id: cell_id(identity),
        lock: Script {
            code_hash: [0u8; 32],
            args: identity.to_vec(),
        },
        type_script: Script {
            code_hash: CODE_HASH,
            args: identity.to_vec(),
        },
        parent: None,
        timestamp: 0,
        data: rec.encode(),
    }
}

/// Is this a key cell (of any identity)?
pub fn is_key_cell(c: &Cell) -> bool {
    c.type_script.code_hash == CODE_HASH
}

/// The identity's live key cell, if it has one.
pub fn live_cell<'a>(token_cells: &'a [Cell], identity: &[u8; 32]) -> Option<&'a Cell> {
    token_cells
        .iter()
        .find(|c| is_key_cell(c) && c.type_script.args == identity.as_slice())
}

/// The identity's key record as stored (not yet advanced to a height; see [`Record::at`]).
pub fn live_record(ledger: &Ledger, identity: &[u8; 32]) -> Option<Record> {
    live_cell(&ledger.token_cells, identity).and_then(|c| Record::decode(&c.data))
}

// ============ contribution labels ============

/// A contribution cell's `lock.args`: `identity || index_le64` (40 bytes) for key generation 0, the
/// form every contribution had before this lane, so an identity that never changes its key makes
/// byte-identical cells; `identity || index_le64 || generation_le64` (48 bytes) for generation 1 and up.
pub fn pack_label(identity: &[u8; 32], index: u64, generation: u64) -> Vec<u8> {
    let mut a = identity.to_vec();
    a.extend_from_slice(&index.to_le_bytes());
    if generation > 0 {
        a.extend_from_slice(&generation.to_le_bytes());
    }
    a
}

/// `(identity, index, generation)` from a contribution's `lock.args`, if it is a canonical label.
pub fn unpack_label(args: &[u8]) -> Option<([u8; 32], u64, u64)> {
    let identity: [u8; 32] = args.get(..32)?.try_into().ok()?;
    let index = u64::from_le_bytes(args.get(32..40)?.try_into().ok()?);
    match args.len() {
        40 => Some((identity, index, 0)),
        48 => {
            let g = u64::from_le_bytes(args[40..48].try_into().ok()?);
            (g > 0).then_some((identity, index, g))
        }
        _ => None,
    }
}

/// The highest one-time index key `generation` of `identity` has used for a contribution in `cells`.
pub fn highest_label_index(cells: &[Cell], identity: &[u8; 32], generation: u64) -> Option<u64> {
    cells
        .iter()
        .filter_map(|c| unpack_label(&c.lock.args))
        .filter(|(id, _, g)| id == identity && *g == generation)
        .map(|(_, k, _)| k)
        .max()
}

// ============ the rule ============

/// The record `op` leaves on an identity whose record at this block is `eff`. `contrib_high` is the
/// highest index the current key has used for a contribution before this block. Pure.
pub fn next_record(
    identity: &[u8; 32],
    eff: &Record,
    op: &KeyOp,
    height: u64,
    c: &Constitution,
    contrib_high: Option<u64>,
) -> Result<Record, &'static str> {
    let j = op.index as u64;
    let hot_index_free = j >= eff.hot_floor && contrib_high.is_none_or(|h| j > h);
    match op.kind {
        Kind::Rotate => {
            if !hot_index_free {
                return Err("this one-time index of the current hot key is already used");
            }
            if op.new_root == eff.hot_root {
                return Err("the rotation names the key that is already current");
            }
            if wallet::cold_commit(&op.new_root) == eff.cold_commit {
                return Err("the new hot key is the identity's cold key");
            }
            let msg = wallet::key_op_digest(
                identity,
                wallet::OP_ROTATE,
                c.chain_id,
                eff.generation,
                &op.new_root,
                op.index,
            );
            if !wallet::verify(&eff.hot_root, op.index, &msg, &op.sig) {
                return Err("the rotation is not signed by this identity's current hot key");
            }
            let generation = eff.generation.checked_add(1).ok_or("generation overflow")?;
            Ok(Record {
                generation,
                hot_root: op.new_root,
                hot_floor: 0,
                pending: None,
                ..*eff
            })
        }
        Kind::Recover => {
            let cold_root = op
                .cold_root
                .ok_or("a recovery request carries the cold key's root")?;
            if wallet::cold_commit(&cold_root) != eff.cold_commit {
                return Err("the cold key does not match this identity's cold commitment");
            }
            if j < eff.cold_floor {
                return Err("this one-time index of the cold key is already used (a replayed or stale recovery request)");
            }
            if op.new_root == eff.hot_root {
                return Err("the recovery names the key that is already current");
            }
            if op.new_root == cold_root {
                return Err("the new hot key is the identity's cold key");
            }
            let msg = wallet::key_op_digest(
                identity,
                wallet::OP_RECOVER,
                c.chain_id,
                eff.generation,
                &op.new_root,
                op.index,
            );
            if !wallet::verify(&cold_root, op.index, &msg, &op.sig) {
                return Err("the recovery request is not signed by the cold key");
            }
            let effective_at = height
                .checked_add(c.recovery_delay_blocks)
                .and_then(|h| h.checked_add(1))
                .ok_or("height overflow")?;
            Ok(Record {
                cold_floor: j + 1,
                pending: Some(Pending {
                    new_root: op.new_root,
                    effective_at,
                }),
                ..*eff
            })
        }
        Kind::Cancel => {
            if eff.pending.is_none() {
                return Err("no recovery is pending for this identity");
            }
            if !hot_index_free {
                return Err("this one-time index of the current hot key is already used");
            }
            let msg = wallet::key_op_digest(
                identity,
                wallet::OP_CANCEL,
                c.chain_id,
                eff.generation,
                &[0u8; 32],
                op.index,
            );
            if !wallet::verify(&eff.hot_root, op.index, &msg, &op.sig) {
                return Err("the cancel is not signed by this identity's current hot key");
            }
            Ok(Record {
                hot_floor: j + 1,
                pending: None,
                ..*eff
            })
        }
    }
}

/// The record `op` leaves on `identity` in a block at `height`, given its live key cell (if any).
fn transition(
    ledger: &Ledger,
    height: u64,
    identity: &[u8; 32],
    live: Option<&Cell>,
    op: &KeyOp,
    c: &Constitution,
) -> Result<Record, &'static str> {
    if !c.key_rotation {
        return Err("identity-key operations are off on this chain");
    }
    let prior = match (live, &op.birth) {
        (Some(cell), None) => {
            Record::decode(&cell.data).ok_or("the identity's key cell holds no valid record")?
        }
        (None, Some((hot, cold))) => {
            if wallet::identity(hot, cold) != *identity {
                return Err("the identity is not the hash of these birth roots");
            }
            Record::birth(*hot, *cold)
        }
        (Some(_), Some(_)) => {
            return Err("this identity already has a key record, so the operation names no birth roots")
        }
        (None, None) => return Err(
            "this identity has no key record yet, so its first key operation must carry its two birth roots",
        ),
    };
    let eff = prior.at(height);
    let high = highest_label_index(&ledger.cells, identity, eff.generation);
    next_record(identity, &eff, op, height, c, high)
}

fn same_cell(a: &Cell, b: &Cell) -> bool {
    a.id == b.id && a.lock == b.lock && a.type_script == b.type_script && a.data == b.data
}

/// THE RULE for one identity-key tx in a block at `height` (shared by [`check_block`] and the route,
/// through [`build_tx`]). Returns the record the tx leaves.
pub fn check_tx(
    ledger: &Ledger,
    height: u64,
    tx: &TokenTx,
    c: &Constitution,
) -> Result<Record, &'static str> {
    if tx.standard != TokenStandard::IdentityKey || tx.code_hash != CODE_HASH {
        return Err("not an identity-key tx");
    }
    let identity: [u8; 32] = tx
        .args
        .as_slice()
        .try_into()
        .map_err(|_| "an identity is 32 bytes")?;
    if tx.auths.len() != 1 || tx.outputs.len() != 1 {
        return Err("an identity-key tx carries one operation and leaves one cell");
    }
    let op = KeyOp::decode(&tx.auths[0]).ok_or("the key operation is malformed")?;
    let live = live_cell(&ledger.token_cells, &identity);
    let inputs_ok = match live {
        Some(cell) => tx.inputs.len() == 1 && same_cell(&tx.inputs[0], cell),
        None => tx.inputs.is_empty(),
    };
    if !inputs_ok {
        return Err(
            "an identity-key tx spends exactly the identity's live key cell, or none before the first",
        );
    }
    let next = transition(ledger, height, &identity, live, &op, c)?;
    let out = &tx.outputs[0];
    if !same_cell(out, &key_cell(&identity, &next)) || out.parent.is_some() || out.timestamp != 0 {
        return Err("the output is not the record this operation leaves");
    }
    Ok(next)
}

/// Build the tx for `op` on `identity` in a block at `height`, and the record it leaves. Refused
/// exactly when [`check_tx`] would refuse the tx it builds.
pub fn build_tx(
    ledger: &Ledger,
    height: u64,
    identity: [u8; 32],
    op: &KeyOp,
    c: &Constitution,
) -> Result<(TokenTx, Record), &'static str> {
    // The tx carries the operation's encoding; one that does not decode to itself is refused here, the
    // way the block rule would refuse it.
    if KeyOp::decode(&op.encode()).is_none_or(|back| back.encode() != op.encode()) {
        return Err("the key operation is malformed");
    }
    let live = live_cell(&ledger.token_cells, &identity).cloned();
    let next = transition(ledger, height, &identity, live.as_ref(), op, c)?;
    let tx = TokenTx {
        standard: TokenStandard::IdentityKey,
        code_hash: CODE_HASH,
        args: identity.to_vec(),
        inputs: live.into_iter().collect(),
        outputs: vec![key_cell(&identity, &next)],
        auths: vec![op.encode()],
    };
    debug_assert_eq!(check_tx(ledger, height, &tx, c), Ok(next));
    Ok((tx, next))
}

/// The block rule (called from `runtime::validate_block`). Every identity-key tx is checked against
/// the state before the block; at most one per identity per block; and every contribution credited to
/// an identity that has a key record (live, or made by this block) carries the label of the key
/// generation the block leaves in effect, at or above that key's hot floor. Off (and any key tx
/// refused) unless [`Constitution::key_rotation`].
pub fn check_block(ledger: &Ledger, b: &Block, c: &Constitution) -> Result<(), &'static str> {
    let mut after: HashMap<[u8; 32], Record> = HashMap::new();
    let mut seen: HashSet<[u8; 32]> = HashSet::new();
    for tx in b
        .token_txs
        .iter()
        .filter(|t| t.standard == TokenStandard::IdentityKey)
    {
        let identity: [u8; 32] = tx
            .args
            .as_slice()
            .try_into()
            .map_err(|_| "an identity is 32 bytes")?;
        if !seen.insert(identity) {
            return Err("a block carries at most one key operation per identity");
        }
        let rec = check_tx(ledger, b.height, tx, c)?;
        after.insert(identity, rec);
    }
    if !c.key_rotation {
        return Ok(()); // no key record can exist on such a chain, and key txs were refused above
    }
    for cell in &b.cells {
        let Ok(identity) = <[u8; 32]>::try_from(cell.type_script.args.as_slice()) else {
            continue;
        };
        let rec = match after.get(&identity) {
            Some(r) => *r,
            None => match live_record(ledger, &identity) {
                Some(r) => r.at(b.height),
                None => continue, // no key record: the identity's birth key, checked at the door as before
            },
        };
        match unpack_label(&cell.lock.args) {
            Some((id, k, g)) if id == identity && g == rec.generation && k >= rec.hot_floor => {}
            _ => return Err("a contribution carries a retired or wrong key label for its identity"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit_order::Committed;
    use crate::runtime::{apply_block, validate_block, Node, Violation};

    const CHAIN: u64 = 0x7e57;
    const DELAY: u64 = 3;
    const RETIRED: &str = "a contribution carries a retired or wrong key label for its identity";

    fn con() -> Constitution {
        Constitution {
            key_rotation: true,
            recovery_delay_blocks: DELAY,
            chain_id: CHAIN,
            ..Constitution::default()
        }
    }

    /// A test wallet: hot seed, cold seed, and the identity they make.
    struct Wallet {
        hot: [u8; 32],
        cold: [u8; 32],
        hot_root: [u8; 32],
        cold_root: [u8; 32],
        cold_commit: [u8; 32],
        id: [u8; 32],
    }

    fn wallet_of(hot: u8, cold: u8) -> Wallet {
        let (hot, cold) = ([hot; 32], [cold; 32]);
        let hot_root = wallet::key_root(&hot);
        let cold_root = wallet::key_root(&cold);
        let cold_commit = wallet::cold_commit(&cold_root);
        Wallet {
            hot,
            cold,
            hot_root,
            cold_root,
            cold_commit,
            id: wallet::identity(&hot_root, &cold_commit),
        }
    }

    /// An operation on `w`'s identity signed by `signer` for `generation` at one-time `index`. `first`
    /// attaches the birth roots (the identity's first operation).
    fn op(
        kind: Kind,
        signer: &[u8; 32],
        w: &Wallet,
        generation: u64,
        new_root: [u8; 32],
        index: u32,
        first: bool,
    ) -> KeyOp {
        let msg = wallet::key_op_digest(&w.id, kind.byte(), CHAIN, generation, &new_root, index);
        KeyOp {
            kind,
            index,
            new_root,
            birth: first.then_some((w.hot_root, w.cold_commit)),
            cold_root: (kind == Kind::Recover).then_some(w.cold_root),
            sig: wallet::sign(signer, index, &msg),
        }
    }

    /// A one-cell block at `height` crediting `identity`, labelled (index, generation).
    fn block(height: u64, identity: &[u8; 32], index: u64, generation: u64, txs: Vec<TokenTx>) -> Block {
        let mut secret = [0u8; 32];
        secret[..8].copy_from_slice(&(height * 1000 + index).to_le_bytes());
        let cell = Cell {
            id: height * 1000 + index,
            lock: Script {
                code_hash: [0u8; 32],
                args: pack_label(identity, index, generation),
            },
            type_script: Script {
                code_hash: [1u8; 32],
                args: identity.to_vec(),
            },
            parent: None,
            timestamp: height,
            data: format!(
                "contribution {height} {index} {generation} about {:?}",
                &identity[..4]
            )
            .into_bytes(),
        };
        Block::assemble(height, &[(cell, Committed { height, secret })])
            .with_chain_id(CHAIN)
            .with_token_txs(txs)
    }

    fn apply(ledger: Ledger, b: &Block) -> Ledger {
        apply_block(ledger, b, &con()).expect("valid block")
    }

    fn refused(ledger: &Ledger, b: &Block) -> &'static str {
        match validate_block(ledger, b, &con()) {
            Err(Violation::IdentityKey { reason }) => reason,
            Err(v) => panic!("expected an identity-key refusal, got {v:?}"),
            Ok(()) => panic!("expected an identity-key refusal, the block is valid"),
        }
    }

    fn build_err(ledger: &Ledger, height: u64, w: &Wallet, o: &KeyOp) -> &'static str {
        build_tx(ledger, height, w.id, o, &con())
            .err()
            .expect("expected the operation to be refused")
    }

    #[test]
    fn record_and_operation_round_trip_byte_for_byte() {
        let w = wallet_of(0x11, 0x12);
        let rec = Record {
            generation: 3,
            hot_root: [5u8; 32],
            cold_commit: w.cold_commit,
            hot_floor: 9,
            cold_floor: 300,
            pending: Some(Pending {
                new_root: [6u8; 32],
                effective_at: 77,
            }),
        };
        assert_eq!(rec.encode().len(), RECORD_LEN);
        assert_eq!(Record::decode(&rec.encode()), Some(rec));
        let none = Record { pending: None, ..rec };
        assert_eq!(Record::decode(&none.encode()), Some(none));
        let mut junk = none.encode();
        junk[100] = 1; // a pending root under a "none" flag is not canonical
        assert_eq!(Record::decode(&junk), None);
        let o = op(Kind::Recover, &w.cold, &w, 0, [7u8; 32], 5, true);
        let bytes = o.encode();
        let back = KeyOp::decode(&bytes).expect("decodes");
        assert_eq!(back.encode(), bytes, "byte-identical re-encode");
        assert!(
            KeyOp::decode(&bytes[..bytes.len() - 1]).is_none(),
            "a short op is refused"
        );
        let mut cancel_with_root = op(Kind::Cancel, &w.hot, &w, 0, [0u8; 32], 5, true).encode();
        cancel_with_root[6] = 1;
        assert!(
            KeyOp::decode(&cancel_with_root).is_none(),
            "a cancel names no new root"
        );
    }

    #[test]
    fn labels_keep_generation_zero_byte_identical_and_name_later_generations() {
        let id = [9u8; 32];
        assert_eq!(pack_label(&id, 5, 0).len(), 40, "the pre-A1 form, unchanged");
        assert_eq!(unpack_label(&pack_label(&id, 5, 0)), Some((id, 5, 0)));
        assert_eq!(unpack_label(&pack_label(&id, 5, 2)), Some((id, 5, 2)));
        let mut odd = pack_label(&id, 5, 0);
        odd.extend_from_slice(&0u64.to_le_bytes());
        assert_eq!(unpack_label(&odd), None, "generation 0 has one form only");
    }

    /// A1: a rotation signed by the current key moves the identity to the next key in its series from
    /// the rotation's own block on; the old key is refused after; the new key's counter starts fresh.
    #[test]
    fn a_rotation_moves_the_identity_to_the_next_key_from_its_own_block() {
        let w = wallet_of(0x21, 0x22);
        let (k1, k2) = (wallet::derive_key(&w.hot, 1), wallet::derive_key(&w.hot, 2));
        let (root1, root2) = (wallet::key_root(&k1), wallet::key_root(&k2));
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        ledger = apply(ledger, &block(2, &w.id, 1, 0, vec![]));
        // the rotation's leaf must sit above every index the key used for a contribution
        assert_eq!(
            build_err(&ledger, 3, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 1, true)),
            "this one-time index of the current hot key is already used"
        );
        let (tx, next) = build_tx(
            &ledger,
            3,
            w.id,
            &op(Kind::Rotate, &w.hot, &w, 0, root1, 2, true),
            &con(),
        )
        .expect("a valid rotation");
        assert_eq!((next.generation, next.hot_root, next.hot_floor), (1, root1, 0));
        // the rotation's own block: a contribution with the old key's label is refused, the new key's
        // label (its counter at 0) is accepted
        assert_eq!(
            refused(&ledger, &block(3, &w.id, 3, 0, vec![tx.clone()])),
            RETIRED
        );
        ledger = apply(ledger, &block(3, &w.id, 0, 1, vec![tx]));
        assert_eq!(
            live_record(&ledger, &w.id).map(|r| (r.generation, r.hot_root)),
            Some((1, root1))
        );
        // after it: the old key can neither rotate nor contribute
        assert_eq!(
            build_err(&ledger, 4, &w, &op(Kind::Rotate, &w.hot, &w, 1, root2, 9, false)),
            "the rotation is not signed by this identity's current hot key"
        );
        assert_eq!(refused(&ledger, &block(4, &w.id, 9, 0, vec![])), RETIRED);
        // key 1 rotates on to key 2, above its own contribution at index 0
        let (tx2, next2) = build_tx(
            &ledger,
            4,
            w.id,
            &op(Kind::Rotate, &k1, &w, 1, root2, 1, false),
            &con(),
        )
        .expect("key 1 rotates");
        assert_eq!(next2.generation, 2);
        ledger = apply(ledger, &block(4, &w.id, 0, 2, vec![tx2]));
        assert_eq!(live_record(&ledger, &w.id).unwrap().generation, 2);
    }

    #[test]
    fn the_first_key_operation_must_prove_the_birth_roots_and_the_lane_can_be_off() {
        let w = wallet_of(0x31, 0x32);
        let ledger = Ledger::new();
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let mut no_birth = op(Kind::Rotate, &w.hot, &w, 0, root1, 0, false);
        assert!(build_err(&ledger, 1, &w, &no_birth).contains("must carry its two birth roots"));
        no_birth.birth = Some((w.hot_root, wallet::cold_commit(&[0x99; 32])));
        assert_eq!(
            build_err(&ledger, 1, &w, &no_birth),
            "the identity is not the hash of these birth roots"
        );
        // a foreign key signing, with the right birth roots
        assert_eq!(
            build_err(
                &ledger,
                1,
                &w,
                &op(Kind::Rotate, &[0x55; 32], &w, 0, root1, 0, true)
            ),
            "the rotation is not signed by this identity's current hot key"
        );
        // rotating to the cold key would let one key sign as both
        assert_eq!(
            build_err(
                &ledger,
                1,
                &w,
                &op(Kind::Rotate, &w.hot, &w, 0, w.cold_root, 0, true)
            ),
            "the new hot key is the identity's cold key"
        );
        // a chain with the lane off refuses even a valid operation, at the route and in a block
        let good = op(Kind::Rotate, &w.hot, &w, 0, root1, 0, true);
        let off = Constitution {
            key_rotation: false,
            ..con()
        };
        assert_eq!(
            build_tx(&ledger, 1, w.id, &good, &off).err(),
            Some("identity-key operations are off on this chain")
        );
        let (tx, _) = build_tx(&ledger, 1, w.id, &good, &con()).unwrap();
        assert!(matches!(
            validate_block(&ledger, &block(1, &w.id, 0, 1, vec![tx.clone()]), &off),
            Err(Violation::IdentityKey { .. })
        ));
        // a tampered output (the producer names another hot root) is refused
        let mut forged = tx;
        forged.outputs[0] = key_cell(
            &w.id,
            &Record {
                hot_root: [0xEE; 32],
                ..Record::birth(w.hot_root, w.cold_commit)
            },
        );
        assert_eq!(
            refused(&ledger, &block(1, &w.id, 0, 1, vec![forged])),
            "the output is not the record this operation leaves"
        );
    }

    /// A7: only the cold key that matches the commitment; a delay of exactly DELAY blocks during which
    /// the old key still signs; the old hot key retired from the block the request takes effect in.
    #[test]
    fn recovery_waits_out_its_delay_and_then_retires_the_old_hot_key() {
        let w = wallet_of(0x41, 0x42);
        let fresh = wallet::key_root(&[0x43; 32]); // the recovering device's new seed
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        let mut thief = op(Kind::Recover, &[0x66; 32], &w, 0, fresh, 0, true);
        thief.cold_root = Some(wallet::key_root(&[0x66; 32]));
        assert_eq!(
            build_err(&ledger, 2, &w, &thief),
            "the cold key does not match this identity's cold commitment"
        );
        let (tx, rec) = build_tx(
            &ledger,
            2,
            w.id,
            &op(Kind::Recover, &w.cold, &w, 0, fresh, 0, true),
            &con(),
        )
        .expect("a valid request");
        assert_eq!(
            rec.pending,
            Some(Pending {
                new_root: fresh,
                effective_at: 2 + DELAY + 1
            })
        );
        assert_eq!(
            (rec.generation, rec.hot_root),
            (0, w.hot_root),
            "nothing changes yet"
        );
        ledger = apply(ledger, &block(2, &w.id, 1, 0, vec![tx]));
        for h in 3..=2 + DELAY {
            assert_eq!(
                live_record(&ledger, &w.id).unwrap().at(h).hot_root,
                w.hot_root,
                "block {h}: the old key still signs"
            );
            ledger = apply(ledger, &block(h, &w.id, h, 0, vec![]));
        }
        let h = 3 + DELAY; // the block it takes effect in
        assert_eq!(live_record(&ledger, &w.id).unwrap().at(h).hot_root, fresh);
        assert_eq!(refused(&ledger, &block(h, &w.id, 50, 0, vec![])), RETIRED);
        assert_eq!(
            build_err(
                &ledger,
                h,
                &w,
                &op(Kind::Cancel, &w.hot, &w, 0, [0u8; 32], 60, false)
            ),
            "no recovery is pending for this identity"
        );
        ledger = apply(ledger, &block(h, &w.id, 0, 1, vec![]));
        assert_eq!(live_record(&ledger, &w.id).unwrap().at(h + 1).generation, 1);
    }

    /// The current hot key cancels inside the window; a replayed request (or cancel) is refused because
    /// its one-time index is spent; a newer request replaces a pending one and restarts the delay.
    #[test]
    fn a_cancel_inside_the_delay_voids_the_request_and_replays_are_refused() {
        let w = wallet_of(0x51, 0x52);
        let fresh = wallet::key_root(&[0x53; 32]);
        let other = wallet::key_root(&[0x54; 32]);
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        let first = op(Kind::Recover, &w.cold, &w, 0, fresh, 3, true);
        let (tx, _) = build_tx(&ledger, 2, w.id, &first, &con()).unwrap();
        ledger = apply(ledger, &block(2, &w.id, 1, 0, vec![tx]));
        // a newer request replaces it and restarts the delay
        let req = op(Kind::Recover, &w.cold, &w, 0, other, 7, false);
        let (tx, rec) = build_tx(&ledger, 3, w.id, &req, &con()).unwrap();
        assert_eq!(
            rec.pending,
            Some(Pending {
                new_root: other,
                effective_at: 3 + DELAY + 1
            })
        );
        ledger = apply(ledger, &block(3, &w.id, 2, 0, vec![tx]));
        for h in 4..3 + DELAY {
            ledger = apply(ledger, &block(h, &w.id, h, 0, vec![]));
        }
        // the hot key cancels in the last block of the window (3 + DELAY)
        let h = 3 + DELAY;
        let cancel = op(Kind::Cancel, &w.hot, &w, 0, [0u8; 32], 10, false);
        let (ctx, rec) = build_tx(&ledger, h, w.id, &cancel, &con()).expect("inside the window");
        assert_eq!((rec.pending, rec.hot_floor, rec.cold_floor), (None, 11, 8));
        // the cancel's leaf cannot also sign a contribution in its block
        assert_eq!(
            refused(&ledger, &block(h, &w.id, 10, 0, vec![ctx.clone()])),
            RETIRED
        );
        ledger = apply(ledger, &block(h, &w.id, 11, 0, vec![ctx]));
        assert_eq!(
            live_record(&ledger, &w.id).unwrap().at(1000).hot_root,
            w.hot_root,
            "nothing takes effect"
        );
        // replays: the same request bytes, the first request's bytes and the same cancel bytes are refused
        assert_eq!(
            build_err(&ledger, h + 1, &w, &req),
            "this one-time index of the cold key is already used (a replayed or stale recovery request)"
        );
        assert_eq!(
            build_err(&ledger, h + 1, &w, &first),
            "this identity already has a key record, so the operation names no birth roots"
        );
        let again = op(Kind::Recover, &w.cold, &w, 0, fresh, 8, false);
        let (tx, _) =
            build_tx(&ledger, h + 1, w.id, &again, &con()).expect("a new request at a fresh cold index");
        ledger = apply(ledger, &block(h + 1, &w.id, 12, 0, vec![tx]));
        assert_eq!(
            build_err(&ledger, h + 2, &w, &cancel),
            "this one-time index of the current hot key is already used"
        );
    }

    /// The race: a rotation and a recovery in flight together. A block carries at most one key operation
    /// per identity; whichever lands first decides the next state, and the other is judged against it.
    #[test]
    fn rotate_and_recover_in_flight_together() {
        let w = wallet_of(0x61, 0x62);
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let fresh = wallet::key_root(&[0x63; 32]);
        let base = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        let (rot_tx, _) = build_tx(
            &base,
            2,
            w.id,
            &op(Kind::Rotate, &w.hot, &w, 0, root1, 1, true),
            &con(),
        )
        .unwrap();
        let (req_tx, _) = build_tx(
            &base,
            2,
            w.id,
            &op(Kind::Recover, &w.cold, &w, 0, fresh, 0, true),
            &con(),
        )
        .unwrap();
        let one_per_block = "a block carries at most one key operation per identity";
        assert_eq!(
            refused(
                &base,
                &block(2, &w.id, 0, 1, vec![rot_tx.clone(), req_tx.clone()])
            ),
            one_per_block
        );
        assert_eq!(
            refused(
                &base,
                &block(2, &w.id, 0, 1, vec![req_tx.clone(), rot_tx.clone()])
            ),
            one_per_block
        );
        // rotation first: the request (signed for generation 0, no key cell) no longer applies
        let after_rot = apply(base.clone(), &block(2, &w.id, 0, 1, vec![rot_tx]));
        assert!(validate_block(&after_rot, &block(3, &w.id, 1, 1, vec![req_tx.clone()]), &con()).is_err());
        let rebound = op(Kind::Recover, &w.cold, &w, 0, fresh, 1, false); // re-signed, but still for generation 0
        assert_eq!(
            build_err(&after_rot, 3, &w, &rebound),
            "the recovery request is not signed by the cold key"
        );
        // recovery first: the rotation then voids the pending request
        let after_req = apply(base, &block(2, &w.id, 1, 0, vec![req_tx]));
        let (rot_tx2, rec) = build_tx(
            &after_req,
            3,
            w.id,
            &op(Kind::Rotate, &w.hot, &w, 0, root1, 2, false),
            &con(),
        )
        .unwrap();
        assert_eq!((rec.generation, rec.pending), (1, None));
        let done = apply(after_req, &block(3, &w.id, 0, 1, vec![rot_tx2]));
        assert_eq!(
            live_record(&done, &w.id).unwrap().at(1000).hot_root,
            root1,
            "the voided request never takes effect"
        );
    }

    /// Standing stays with the identity across a rotation and a recovery, and no key operation moves
    /// standing or finality weight: a twin chain with the same contribution cells and no key operation
    /// holds the same cells, index root, PoM, work clock and finality weight. ANTI-THEATER: fold the key
    /// cell into `cells` in `apply_transition` and the cells and PoM differ.
    #[test]
    fn key_operations_move_no_standing_and_no_finality_weight() {
        let w = wallet_of(0x71, 0x72);
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let fresh = wallet::key_root(&[0x73; 32]);
        let mut with = Node::new(0, vec![], con());
        let mut twin = Node::new(0, vec![], con());
        let step = |with: &mut Node, twin: &mut Node, b: Block| {
            with.ledger = apply_block(with.ledger.clone(), &b, &con()).expect("valid on the key chain");
            twin.apply(&Block {
                token_txs: vec![],
                ..b
            });
        };
        step(&mut with, &mut twin, block(1, &w.id, 0, 0, vec![]));
        let (rot, _) = build_tx(
            &with.ledger,
            2,
            w.id,
            &op(Kind::Rotate, &w.hot, &w, 0, root1, 1, true),
            &con(),
        )
        .unwrap();
        step(&mut with, &mut twin, block(2, &w.id, 0, 1, vec![rot]));
        let (req, _) = build_tx(
            &with.ledger,
            3,
            w.id,
            &op(Kind::Recover, &w.cold, &w, 1, fresh, 0, false),
            &con(),
        )
        .unwrap();
        step(&mut with, &mut twin, block(3, &w.id, 1, 1, vec![req]));
        for h in 4..=3 + DELAY {
            step(&mut with, &mut twin, block(h, &w.id, h, 1, vec![]));
        }
        step(&mut with, &mut twin, block(4 + DELAY, &w.id, 0, 2, vec![])); // the recovered key signs
        let (a, b) = (with.ledger.state_digest(), twin.ledger.state_digest());
        assert_eq!(a.0, b.0, "same contribution cells");
        assert_eq!(a.1, b.1, "same novelty index root");
        assert_eq!(a.2, b.2, "same PoM standing");
        assert_eq!(a.4, b.4, "same work clock");
        assert_eq!(
            with.finality_pom_weight(),
            twin.finality_pom_weight(),
            "same finality weight"
        );
        assert_eq!(
            with.ledger.pom.len(),
            1,
            "one identity holds all of it, across three keys"
        );
        assert!(with.ledger.pom.get(w.id.as_slice()).is_some_and(|s| *s > 0));
        assert_ne!(a.3, b.3, "only the token cells differ: the key cell");
        assert_eq!(
            live_record(&with.ledger, &w.id).unwrap().at(5 + DELAY).hot_root,
            fresh
        );
    }
}
