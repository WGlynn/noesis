//! The identity-key lane: one identity, a series of hot keys, and a cold key that is the identity's root
//! of trust (wallet decisions A1 and A7, Will 2026-11-17; built 2026-11-18 and conformed the same day to
//! the approved design, decision 7: a rotation waits out a delay the cold key can cancel, a recovery waits
//! out the same delay, and the cold key can freeze the identity at once; reference node only).
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
//!   data          [`Record`], 131 bytes:
//!     [0]        0xB1 (record version 1)
//!     [1..9]     generation u64 LE: key changes in effect (0 = the birth key)
//!     [9..41]    hot root: the key that signs for the identity now
//!     [41..73]   cold commitment: fixed at birth (it is inside the identity hash)
//!     [73..81]   hot floor u64 LE: lowest index the current hot key may sign a key operation at
//!     [81..89]   cold floor u64 LE: lowest index the cold key may sign at
//!     [89]       1 = frozen by the cold key, 0 = not
//!     [90]       the pending key change: 0 none, 1 a rotation, 2 a recovery
//!     [91..123]  pending: the new hot root it names (zeros if none)
//!     [123..131] pending: the block height from which it is in effect (0 if none)
//!
//! THE OPERATIONS. Each is one token tx of standard [`TokenStandard::IdentityKey`] (tag 4): no input
//! (the identity's first operation, which must carry the two birth roots that hash to the identity) or
//! the live key cell, exactly one output (the record the operation leaves, fully determined by the
//! rule), and `auths[0]` = the encoded [`KeyOp`]. Every operation signs
//! `noesis_core::wallet::key_op_digest(identity, op, chain_id, generation, new_root, index)`. T is
//! `Constitution::key_change_delay_blocks`; a key change carried by block `h` takes effect at
//! `h + T + 1`, so the other key has exactly T blocks to cancel it.
//!   Signed by the current hot key, and refused while the identity is frozen:
//!   ROTATE            names a new hot root; pending for T blocks, cancelable by the cold key, then in
//!                     effect with no further transaction. The current key keeps signing until then.
//!                     Refused while any key change is pending.
//!   CANCEL-RECOVERY   voids a pending recovery request.
//!   Signed by the cold key whose commitment is in the identity:
//!   RECOVER           names a new hot root; pending for T blocks, then in effect, and the identity is
//!                     unfrozen. Replaces any pending change (a rotation, or an older request) and
//!                     restarts the delay. Cancelable by the hot key only while the identity is not frozen.
//!   CANCEL-ROTATION   voids a pending rotation.
//!   FREEZE            at once: from the block that carries it the identity is credited nothing and its
//!                     hot key signs no key operation; a pending rotation cannot take effect while frozen.
//!   UNFREEZE          at once. Refused while a rotation is pending: the cold key cancels it first, so a
//!                     rotation pending at a freeze can never take effect.
//!
//! WHO WINS. The cold key, always. Against a thief holding the hot key: freeze (immediate), cancel the
//! rotation if one is pending (or let the recovery replace it), recover; the hot key can neither cancel
//! the recovery nor do anything else while frozen, so the owner's new key is in effect T + 3 blocks after
//! the freeze lands (T + 2 if the recovery replaces the rotation directly). A thief holding the cold key
//! wins the same way: it can freeze first, and the hot key's cancel window protects only an unfrozen
//! identity. The cold key is ultimately authoritative; that is why it lives offline, on paper.
//!
//! ONE-TIME KEYS. Every key is XMSS^MT (65,536 one-time leaves), so no leaf may sign two messages.
//! Counters are per key: a contribution's `lock.args` names its key generation ([`pack_label`]), a
//! hot-signed operation must use an index above every contribution of the current key and at or above
//! the record's hot floor, and a contribution in a block must sit at or above the hot floor the block
//! leaves. Every key operation raises its signer's floor past its own index, so a replayed operation is
//! refused. Each hot key keeps its last leaf ([`ROTATION_LEAF`]) for a rotation: no contribution and no
//! cancel may use it, so a key that has signed with every other leaf can still rotate.
//!
//! WHERE EACH RULE LIVES. The operations carry their signatures, so [`check_block`] (called from
//! `runtime::validate_block`) checks all of them, and the producer, a follower, a store replay, a
//! syncing joiner and the reorg path enforce them alike. A contribution's own signature never enters a
//! block (the door checks it, `rpc::ServerState::submit_signed`), so "the retired key cannot sign" is
//! the door's check against the chain's current root; the rulebook adds what a replica can check from
//! the block alone: a contribution by an identity with a key record must carry the label of that
//! identity's current key generation, must not use the rotation leaf, and is refused while the identity
//! is frozen.
//!
//! HONEST FLOOR. Reference node only: no on-VM script knows this standard, and the on-VM lock script's
//! `standard_of` refuses its code hash. The time lock counts blocks, and testnet blocks are made on
//! demand (one per accepted contribution), so anyone who makes blocks shortens it in wall-clock time.
//! That does not weaken the cold key against a hot-key thief (its freeze is immediate), but it does
//! shorten the hot key's window against a cold-key thief, who in any case can freeze first. A cancel
//! signs zeros in the new-root field (the `noesis_core` digest, unchanged here), so it names no target:
//! a cancel that was signed but never carried stays valid for a later change of the same key generation
//! until its signer's floor passes its index.

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
pub const RECORD_LEN: usize = 131;
/// First byte of an encoded key operation.
pub const OP_TAG: u8 = 0x4B;
/// Length of one encoded XMSS^MT signature: two trees of (OTS root, auth path, Lamport signature).
pub const SIG_LEN: usize = 2 * (32 + xmss::H * 32 + 256 * 64);
/// `op` byte of a freeze (signed by the cold key). The bytes of a rotation (1), a recovery (2) and a
/// cancel of a recovery (3) are `noesis_core::wallet`'s; these three live here because noesis-core's
/// sources feed the on-VM fixtures, and no on-VM script reads key operations.
pub const OP_FREEZE: u8 = 4;
/// `op` byte of an unfreeze (signed by the cold key).
pub const OP_UNFREEZE: u8 = 5;
/// `op` byte of a cancel of a pending rotation (signed by the cold key).
pub const OP_CANCEL_ROTATION: u8 = 6;
/// The one-time index every hot key keeps for a rotation: its last. No contribution and no cancel may
/// sign at it, so a key that has used every other leaf can still move the identity to a new key.
pub const ROTATION_LEAF: u64 = wallet::MAX_SIGS - 1;

/// Which key change is pending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Started by the hot key; the cold key can cancel it.
    Rotation,
    /// Started by the cold key; the hot key can cancel it while the identity is not frozen.
    Recovery,
}

impl Change {
    fn byte(self) -> u8 {
        match self {
            Change::Rotation => 1,
            Change::Recovery => 2,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Change::Rotation => "rotation",
            Change::Recovery => "recovery",
        }
    }
}

/// A key change waiting out its time lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    pub change: Change,
    /// The hot root it names.
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
    /// Frozen by the cold key: the hot key can do nothing, and the identity is credited nothing.
    pub frozen: bool,
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
            frozen: false,
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
        out.push(u8::from(self.frozen));
        match &self.pending {
            None => {
                out.push(0);
                out.extend_from_slice(&[0u8; 40]);
            }
            Some(p) => {
                out.push(p.change.byte());
                out.extend_from_slice(&p.new_root);
                out.extend_from_slice(&p.effective_at.to_le_bytes());
            }
        }
        out
    }

    /// Parse a record. Exact length and canonical bytes only (a flag is 0 or 1; an absent pending change
    /// is all zeros).
    pub fn decode(data: &[u8]) -> Option<Record> {
        if data.len() != RECORD_LEN || data[0] != RECORD_TAG {
            return None;
        }
        let u64_at = |i: usize| u64::from_le_bytes(data[i..i + 8].try_into().expect("8 bytes"));
        let a32 = |i: usize| -> [u8; 32] { data[i..i + 32].try_into().expect("32 bytes") };
        let frozen = match data[89] {
            0 => false,
            1 => true,
            _ => return None,
        };
        let change = match data[90] {
            0 if data[91..131].iter().all(|&b| b == 0) => None,
            1 => Some(Change::Rotation),
            2 => Some(Change::Recovery),
            _ => return None,
        };
        Some(Record {
            generation: u64_at(1),
            hot_root: a32(9),
            cold_commit: a32(41),
            hot_floor: u64_at(73),
            cold_floor: u64_at(81),
            frozen,
            pending: change.map(|change| Pending {
                change,
                new_root: a32(91),
                effective_at: u64_at(123),
            }),
        })
    }

    /// The record as it stands at block `height`: a pending change whose delay has run is in effect (the
    /// new hot root signs, the generation is one higher, the new key's floor is 0, the identity is not
    /// frozen), except a rotation on a frozen identity, which waits until the cold key cancels it or a
    /// recovery replaces it. Nothing needs to be sent for a change to take effect; the next operation on
    /// the identity starts from this state.
    pub fn at(&self, height: u64) -> Record {
        match self.pending {
            Some(p) if height >= p.effective_at && !(self.frozen && p.change == Change::Rotation) => Record {
                generation: self.generation.saturating_add(1),
                hot_root: p.new_root,
                hot_floor: 0,
                frozen: false,
                pending: None,
                ..*self
            },
            _ => *self,
        }
    }
}

/// Which key operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Hot key: start a rotation to a new hot root.
    Rotate,
    /// Hot key: cancel a pending recovery.
    CancelRecovery,
    /// Cold key: request a new hot root.
    Recover,
    /// Cold key: cancel a pending rotation.
    CancelRotation,
    /// Cold key: freeze the identity at once.
    Freeze,
    /// Cold key: unfreeze it.
    Unfreeze,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Rotate,
        Kind::CancelRecovery,
        Kind::Recover,
        Kind::CancelRotation,
        Kind::Freeze,
        Kind::Unfreeze,
    ];
    pub fn byte(self) -> u8 {
        match self {
            Kind::Rotate => wallet::OP_ROTATE,
            Kind::Recover => wallet::OP_RECOVER,
            Kind::CancelRecovery => wallet::OP_CANCEL,
            Kind::Freeze => OP_FREEZE,
            Kind::Unfreeze => OP_UNFREEZE,
            Kind::CancelRotation => OP_CANCEL_ROTATION,
        }
    }
    pub fn from_byte(b: u8) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.byte() == b)
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Rotate => "rotate",
            Kind::CancelRecovery => "cancel-recovery",
            Kind::Recover => "recover",
            Kind::CancelRotation => "cancel-rotation",
            Kind::Freeze => "freeze",
            Kind::Unfreeze => "unfreeze",
        }
    }
    pub fn from_name(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.name() == s)
    }
    /// Signed by the cold key (the others by the current hot key).
    pub fn by_cold(self) -> bool {
        matches!(self, Kind::Recover | Kind::CancelRotation | Kind::Freeze | Kind::Unfreeze)
    }
    /// Names a new hot root (the others carry zeros there).
    pub fn names_root(self) -> bool {
        matches!(self, Kind::Rotate | Kind::Recover)
    }
}

/// A signed key operation, as carried in the tx's `auths[0]`.
///   [0] OP_TAG  [1] kind  [2..6] index u32 LE  [6..38] new root (zeros unless a rotate or recover)
///   [38] flags: bit 0 = birth roots follow (hot root, cold commitment), bit 1 = cold root follows
///   then the birth roots (64 bytes) and the cold root (32 bytes) if flagged, then the signature
///   (bottom tree, then top tree: OTS root, auth path, Lamport signature).
pub struct KeyOp {
    pub kind: Kind,
    /// The signer's one-time index (the hot key's or the cold key's, by [`Kind::by_cold`]).
    pub index: u32,
    /// The new hot root (zeros unless a rotate or recover).
    pub new_root: [u8; 32],
    /// The identity's two birth roots `(hot root, cold commitment)`, required exactly when the identity
    /// has no key cell yet, so the chain can check they hash to the identity.
    pub birth: Option<([u8; 32], [u8; 32])>,
    /// The cold key's public root, on every cold-key operation and only there; its commitment must be
    /// the identity's.
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

    /// Parse an encoded operation. Exact length only; a cold root exactly on a cold-key operation; a new
    /// root only on a rotate or recover; the index is a valid one-time index.
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
        if has_cold != kind.by_cold() || (!kind.names_root() && new_root != [0u8; 32]) {
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

/// Refusal: a hot-key operation on a frozen identity.
pub const FROZEN_OP: &str = "this identity is frozen by its cold key: its hot key can do nothing until the cold key unfreezes it or a recovery takes effect";
/// Refusal: a contribution credited to a frozen identity (the block rule).
pub const FROZEN_CONTRIBUTION: &str = "a contribution is credited to an identity its cold key has frozen";
/// Refusal: a contribution at the rotation leaf (the block rule).
pub const LEAF_CONTRIBUTION: &str =
    "a contribution uses the last one-time index of its key, which signs only a rotation";
/// Refusal: a contribution whose label is not the identity's current key, or is below its hot floor.
pub const RETIRED_LABEL: &str = "a contribution carries a retired or wrong key label for its identity";

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
    let msg = wallet::key_op_digest(
        identity,
        op.kind.byte(),
        c.chain_id,
        eff.generation,
        &op.new_root,
        op.index,
    );
    let effective_at = || {
        height
            .checked_add(c.key_change_delay_blocks)
            .and_then(|h| h.checked_add(1))
            .ok_or("height overflow")
    };
    if op.kind.by_cold() {
        let cold_root = op
            .cold_root
            .ok_or("a cold-key operation carries the cold key's root")?;
        if wallet::cold_commit(&cold_root) != eff.cold_commit {
            return Err("the cold key does not match this identity's cold commitment");
        }
        if j < eff.cold_floor {
            return Err("this one-time index of the cold key is already used (a replayed or stale request)");
        }
        let after = match op.kind {
            Kind::Recover => {
                if op.new_root == eff.hot_root {
                    return Err("the recovery names the key that is already current");
                }
                if op.new_root == cold_root {
                    return Err("the new hot key is the identity's cold key");
                }
                // replaces any pending change: a rotation the hot key started, or an older request
                Record {
                    pending: Some(Pending {
                        change: Change::Recovery,
                        new_root: op.new_root,
                        effective_at: effective_at()?,
                    }),
                    ..*eff
                }
            }
            Kind::CancelRotation => {
                if !matches!(eff.pending, Some(p) if p.change == Change::Rotation) {
                    return Err("no rotation is pending for this identity");
                }
                Record { pending: None, ..*eff }
            }
            Kind::Freeze => {
                if eff.frozen {
                    return Err("this identity is already frozen");
                }
                Record { frozen: true, ..*eff }
            }
            Kind::Unfreeze => {
                if !eff.frozen {
                    return Err("this identity is not frozen");
                }
                if matches!(eff.pending, Some(p) if p.change == Change::Rotation) {
                    return Err("a rotation is pending: the cold key cancels it before unfreezing, or unfreezing would let it take effect");
                }
                Record { frozen: false, ..*eff }
            }
            Kind::Rotate | Kind::CancelRecovery => unreachable!("hot-key operations"),
        };
        if !wallet::verify(&cold_root, op.index, &msg, &op.sig) {
            return Err("the operation is not signed by the cold key");
        }
        return Ok(Record { cold_floor: j + 1, ..after });
    }
    if eff.frozen {
        return Err(FROZEN_OP);
    }
    if j < eff.hot_floor || contrib_high.is_some_and(|h| j <= h) {
        return Err("this one-time index of the current hot key is already used");
    }
    let after = match op.kind {
        Kind::Rotate => {
            match eff.pending {
                None => {}
                Some(p) if p.change == Change::Rotation => {
                    return Err("a rotation is already pending for this identity")
                }
                Some(_) => {
                    return Err("a recovery request is pending for this identity; its hot key can cancel it first")
                }
            }
            if op.new_root == eff.hot_root {
                return Err("the rotation names the key that is already current");
            }
            if wallet::cold_commit(&op.new_root) == eff.cold_commit {
                return Err("the new hot key is the identity's cold key");
            }
            Record {
                pending: Some(Pending {
                    change: Change::Rotation,
                    new_root: op.new_root,
                    effective_at: effective_at()?,
                }),
                ..*eff
            }
        }
        Kind::CancelRecovery => {
            if j == ROTATION_LEAF {
                return Err("the last one-time index of a hot key signs only a rotation");
            }
            if !matches!(eff.pending, Some(p) if p.change == Change::Recovery) {
                return Err("no recovery is pending for this identity");
            }
            Record { pending: None, ..*eff }
        }
        _ => unreachable!("cold-key operations"),
    };
    if !wallet::verify(&eff.hot_root, op.index, &msg, &op.sig) {
        return Err(match op.kind {
            Kind::Rotate => "the rotation is not signed by this identity's current hot key",
            _ => "the cancel is not signed by this identity's current hot key",
        });
    }
    Ok(Record { hot_floor: j + 1, ..after })
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
/// an identity that has a key record (live, or made by this block) is refused if the record the block
/// leaves is frozen, and otherwise must carry the label of the key generation that record has in
/// effect, at or above that key's hot floor and below its rotation leaf. Off (and any key tx refused)
/// unless [`Constitution::key_rotation`].
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
        if rec.frozen {
            return Err(FROZEN_CONTRIBUTION);
        }
        match unpack_label(&cell.lock.args) {
            Some((id, k, g)) if id == identity && g == rec.generation && k >= rec.hot_floor => {
                if k >= ROTATION_LEAF {
                    return Err(LEAF_CONTRIBUTION);
                }
            }
            _ => return Err(RETIRED_LABEL),
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
    const ZERO: [u8; 32] = [0u8; 32];
    const RETIRED: &str = RETIRED_LABEL;
    const FROZEN: &str = FROZEN_CONTRIBUTION;
    const HOT_SPENT: &str = "this one-time index of the current hot key is already used";
    const COLD_SPENT: &str = "this one-time index of the cold key is already used (a replayed or stale request)";
    const NOT_COLD: &str = "the cold key does not match this identity's cold commitment";
    /// Someone else, whose contributions fill the blocks the identity under test makes no contribution in.
    const OTHER: [u8; 32] = [0xF0; 32];

    fn con() -> Constitution {
        Constitution {
            key_rotation: true,
            key_change_delay_blocks: DELAY,
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
    /// attaches the birth roots (the identity's first operation). A cold-key operation names `w`'s cold
    /// root whoever signs it.
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
            cold_root: kind.by_cold().then_some(w.cold_root),
            sig: wallet::sign(signer, index, &msg),
        }
    }

    /// A one-cell block at `height` crediting `identity`, labelled (index, generation).
    fn block(height: u64, identity: &[u8; 32], index: u64, generation: u64, txs: Vec<TokenTx>) -> Block {
        let mut secret = [0u8; 32];
        secret[..8].copy_from_slice(&(height * 1000 + index % 1000).to_le_bytes());
        let cell = Cell {
            id: height * 1000 + index % 1000,
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

    /// A block at `height` crediting only `OTHER`, carrying `txs`.
    fn filler(height: u64, txs: Vec<TokenTx>) -> Block {
        block(height, &OTHER, height, 0, txs)
    }

    fn apply(ledger: Ledger, b: &Block) -> Ledger {
        apply_block(ledger, b, &con()).expect("valid block")
    }

    /// Build `o` on `w`'s identity at `height` and land it in a filler block; the record it leaves.
    fn land(ledger: &mut Ledger, height: u64, w: &Wallet, o: &KeyOp) -> Record {
        let (tx, rec) = build_tx(ledger, height, w.id, o, &con()).expect("a valid key operation");
        *ledger = apply(ledger.clone(), &filler(height, vec![tx]));
        rec
    }

    /// `w`'s key record as it stands at `height`.
    fn now(ledger: &Ledger, w: &Wallet, height: u64) -> Record {
        live_record(ledger, &w.id).expect("a key record").at(height)
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

    /// A well-formed (not valid) signature at `index`, for codec tests that never verify.
    fn dummy_sig(index: u32) -> wallet::Signature {
        let tree = |i: u32| xmss::Signature {
            index: i,
            ots_root: [3u8; 32],
            auth: [[4u8; 32]; xmss::H],
            ots_sig: vec![5u8; 256 * 64],
        };
        noesis_core::xmss_mt::MtSignature {
            g: index,
            bottom: tree(index & (xmss::LEAVES as u32 - 1)),
            top: tree(index >> xmss::H),
        }
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
            frozen: true,
            pending: Some(Pending {
                change: Change::Rotation,
                new_root: [6u8; 32],
                effective_at: 77,
            }),
        };
        assert_eq!(rec.encode().len(), RECORD_LEN);
        assert_eq!(Record::decode(&rec.encode()), Some(rec));
        let recovery = Record {
            frozen: false,
            pending: rec.pending.map(|p| Pending {
                change: Change::Recovery,
                ..p
            }),
            ..rec
        };
        assert_eq!(Record::decode(&recovery.encode()), Some(recovery));
        let none = Record { pending: None, ..rec };
        assert_eq!(Record::decode(&none.encode()), Some(none));
        // not canonical: a pending root under "none", a flag of 2, a third kind of change
        for (at, byte) in [(100usize, 1u8), (89, 2), (90, 3)] {
            let mut junk = none.encode();
            junk[at] = byte;
            assert_eq!(Record::decode(&junk), None, "byte {at} = {byte}");
        }
        for kind in Kind::ALL {
            assert_eq!(Kind::from_name(kind.name()), Some(kind));
            assert_eq!(Kind::from_byte(kind.byte()), Some(kind));
            let o = KeyOp {
                kind,
                index: 300,
                new_root: if kind.names_root() { [7u8; 32] } else { ZERO },
                birth: Some((w.hot_root, w.cold_commit)),
                cold_root: kind.by_cold().then_some(w.cold_root),
                sig: dummy_sig(300),
            };
            let bytes = o.encode();
            let back = KeyOp::decode(&bytes).expect("decodes");
            assert_eq!(back.encode(), bytes, "{}: byte-identical re-encode", kind.name());
            assert!(KeyOp::decode(&bytes[..bytes.len() - 1]).is_none(), "a short op is refused");
            // a root where none belongs, or a cold root on the wrong kind of operation (or missing)
            if !kind.names_root() {
                let mut with_root = bytes.clone();
                with_root[6] = 1;
                assert!(KeyOp::decode(&with_root).is_none(), "{} names no new root", kind.name());
            }
            let flipped = KeyOp {
                cold_root: if kind.by_cold() { None } else { Some(w.cold_root) },
                sig: dummy_sig(300),
                ..o
            };
            assert!(KeyOp::decode(&flipped.encode()).is_none(), "{}: the cold root is wrong", kind.name());
        }
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

    /// A1 as approved: a rotation signed by the current hot key is PENDING for DELAY blocks, during which
    /// the current key keeps signing (above the rotation's own leaf) and no second key change can start;
    /// from the block it takes effect in, the old key is retired and the new key's counter starts at 0.
    #[test]
    fn a_rotation_waits_out_its_delay_then_moves_the_identity_to_the_next_key() {
        let w = wallet_of(0x21, 0x22);
        let (k1, k2) = (wallet::derive_key(&w.hot, 1), wallet::derive_key(&w.hot, 2));
        let (root1, root2) = (wallet::key_root(&k1), wallet::key_root(&k2));
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        ledger = apply(ledger, &block(2, &w.id, 1, 0, vec![]));
        // the rotation's leaf must sit above every index the key used for a contribution
        assert_eq!(
            build_err(&ledger, 3, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 1, true)),
            HOT_SPENT
        );
        let (tx, rec) = build_tx(
            &ledger,
            3,
            w.id,
            &op(Kind::Rotate, &w.hot, &w, 0, root1, 2, true),
            &con(),
        )
        .expect("a valid rotation");
        let effective = 3 + DELAY + 1;
        assert_eq!(
            (rec.generation, rec.hot_root, rec.hot_floor),
            (0, w.hot_root, 3),
            "nothing changes yet; the rotation's leaf is spent"
        );
        assert_eq!(
            rec.pending,
            Some(Pending {
                change: Change::Rotation,
                new_root: root1,
                effective_at: effective
            })
        );
        // its own block: the new key's label is refused (not in effect), and so is the old key's at the
        // rotation's own leaf
        assert_eq!(
            refused(&ledger, &block(3, &w.id, 0, 1, vec![tx.clone()])),
            RETIRED
        );
        assert_eq!(
            refused(&ledger, &block(3, &w.id, 2, 0, vec![tx.clone()])),
            RETIRED
        );
        ledger = apply(ledger, &block(3, &w.id, 3, 0, vec![tx]));
        // the window: the old key keeps signing; no second change can start; the hot key has no cancel
        // for a rotation
        assert_eq!(
            build_err(&ledger, 4, &w, &op(Kind::Rotate, &w.hot, &w, 0, root2, 9, false)),
            "a rotation is already pending for this identity"
        );
        assert_eq!(
            build_err(
                &ledger,
                4,
                &w,
                &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 9, false)
            ),
            "no recovery is pending for this identity"
        );
        for h in 4..effective {
            assert_eq!(now(&ledger, &w, h).hot_root, w.hot_root, "block {h}: the old key still signs");
            ledger = apply(ledger, &block(h, &w.id, h, 0, vec![]));
        }
        // in effect: the old key is retired, the new key signs from 0
        let r = now(&ledger, &w, effective);
        assert_eq!(
            (r.generation, r.hot_root, r.hot_floor, r.pending, r.frozen),
            (1, root1, 0, None, false)
        );
        assert_eq!(refused(&ledger, &block(effective, &w.id, 50, 0, vec![])), RETIRED);
        assert_eq!(
            build_err(
                &ledger,
                effective,
                &w,
                &op(Kind::Rotate, &w.hot, &w, 1, root2, 60, false)
            ),
            "the rotation is not signed by this identity's current hot key"
        );
        ledger = apply(ledger, &block(effective, &w.id, 0, 1, vec![]));
        // key 1 starts a rotation on to key 2, above its own contribution at index 0
        let (tx2, next2) = build_tx(
            &ledger,
            effective + 1,
            w.id,
            &op(Kind::Rotate, &k1, &w, 1, root2, 1, false),
            &con(),
        )
        .expect("key 1 rotates");
        assert_eq!(
            next2.pending.map(|p| (p.change, p.new_root)),
            Some((Change::Rotation, root2))
        );
        ledger = apply(ledger, &block(effective + 1, &w.id, 2, 1, vec![tx2]));
        assert_eq!(now(&ledger, &w, effective + 2 + DELAY).generation, 2);
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
            validate_block(&ledger, &block(1, &w.id, 1, 0, vec![tx.clone()]), &off),
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
            refused(&ledger, &block(1, &w.id, 1, 0, vec![forged])),
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
        assert_eq!(build_err(&ledger, 2, &w, &thief), NOT_COLD);
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
                change: Change::Recovery,
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
                now(&ledger, &w, h).hot_root,
                w.hot_root,
                "block {h}: the old key still signs"
            );
            ledger = apply(ledger, &block(h, &w.id, h, 0, vec![]));
        }
        let h = 3 + DELAY; // the block it takes effect in
        assert_eq!(now(&ledger, &w, h).hot_root, fresh);
        assert_eq!(refused(&ledger, &block(h, &w.id, 50, 0, vec![])), RETIRED);
        assert_eq!(
            build_err(
                &ledger,
                h,
                &w,
                &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 60, false)
            ),
            "no recovery is pending for this identity"
        );
        ledger = apply(ledger, &block(h, &w.id, 0, 1, vec![]));
        assert_eq!(now(&ledger, &w, h + 1).generation, 1);
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
                change: Change::Recovery,
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
        let cancel = op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 10, false);
        let (ctx, rec) = build_tx(&ledger, h, w.id, &cancel, &con()).expect("inside the window");
        assert_eq!((rec.pending, rec.hot_floor, rec.cold_floor), (None, 11, 8));
        // the cancel's leaf cannot also sign a contribution in its block
        assert_eq!(
            refused(&ledger, &block(h, &w.id, 10, 0, vec![ctx.clone()])),
            RETIRED
        );
        ledger = apply(ledger, &block(h, &w.id, 11, 0, vec![ctx]));
        assert_eq!(
            now(&ledger, &w, 1000).hot_root,
            w.hot_root,
            "nothing takes effect"
        );
        // replays: the same request bytes, the first request's bytes and the same cancel bytes are refused
        assert_eq!(build_err(&ledger, h + 1, &w, &req), COLD_SPENT);
        assert_eq!(
            build_err(&ledger, h + 1, &w, &first),
            "this identity already has a key record, so the operation names no birth roots"
        );
        let again = op(Kind::Recover, &w.cold, &w, 0, fresh, 8, false);
        let (tx, _) =
            build_tx(&ledger, h + 1, w.id, &again, &con()).expect("a new request at a fresh cold index");
        ledger = apply(ledger, &block(h + 1, &w.id, 12, 0, vec![tx]));
        assert_eq!(build_err(&ledger, h + 2, &w, &cancel), HOT_SPENT);
    }

    /// Decision 7: the cold key cancels a hot-key rotation inside its window. Only the cold key can: the
    /// hot key has no cancel for a rotation, and a hot key posing as the cold key fails the commitment.
    /// After the window the rotation is in effect and there is nothing left to cancel.
    #[test]
    fn the_cold_key_cancels_a_rotation_inside_its_window() {
        let w = wallet_of(0x91, 0x92);
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        assert_eq!(
            build_err(
                &ledger,
                2,
                &w,
                &op(Kind::CancelRotation, &w.cold, &w, 0, ZERO, 0, true)
            ),
            "no rotation is pending for this identity"
        );
        land(&mut ledger, 2, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 1, true));
        // the hot key signing a cancel of its rotation, presenting its own root as the cold root
        let mut posing = op(Kind::CancelRotation, &w.hot, &w, 0, ZERO, 2, false);
        posing.cold_root = Some(w.hot_root);
        assert_eq!(build_err(&ledger, 3, &w, &posing), NOT_COLD);
        // the real cold key, in the last block of the window
        for h in 3..2 + DELAY {
            ledger = apply(ledger, &filler(h, vec![]));
        }
        let h = 2 + DELAY;
        let cancel = op(Kind::CancelRotation, &w.cold, &w, 0, ZERO, 4, false);
        let rec = land(&mut ledger, h, &w, &cancel);
        assert_eq!(
            (rec.pending, rec.hot_floor, rec.cold_floor, rec.generation),
            (None, 2, 5, 0),
            "voided; the rotation's leaf stays spent"
        );
        assert_eq!(
            now(&ledger, &w, 1000).hot_root,
            w.hot_root,
            "the rotation never takes effect"
        );
        // the same cancel again: its cold leaf is spent
        assert_eq!(build_err(&ledger, h + 1, &w, &cancel), COLD_SPENT);
        // a rotation nobody cancels is in effect at its block, and a cancel there is too late
        let r = land(&mut ledger, h + 1, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 2, false));
        let effective = r.pending.expect("pending").effective_at;
        for hh in h + 2..effective {
            ledger = apply(ledger, &filler(hh, vec![]));
        }
        assert_eq!(
            build_err(
                &ledger,
                effective,
                &w,
                &op(Kind::CancelRotation, &w.cold, &w, 0, ZERO, 5, false)
            ),
            "no rotation is pending for this identity"
        );
        assert_eq!(now(&ledger, &w, effective).hot_root, root1);
    }

    /// Decision 7: the cold key freezes at once. From the block that carries the freeze the identity is
    /// credited nothing and its hot key signs no key operation; nothing thaws on its own; only the cold key
    /// unfreezes, after which the hot key signs and rotates again.
    #[test]
    fn a_freeze_stops_the_hot_key_at_once_and_only_the_cold_key_unfreezes() {
        let w = wallet_of(0xA1, 0xA2);
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        let (tx, rec) = build_tx(
            &ledger,
            2,
            w.id,
            &op(Kind::Freeze, &w.cold, &w, 0, ZERO, 0, true),
            &con(),
        )
        .expect("a valid freeze");
        assert!(rec.frozen && rec.pending.is_none() && rec.cold_floor == 1 && rec.hot_root == w.hot_root);
        assert_eq!(
            refused(&ledger, &block(2, &w.id, 1, 0, vec![tx.clone()])),
            FROZEN,
            "at once: in the freeze's own block"
        );
        ledger = apply(ledger, &filler(2, vec![tx]));
        assert_eq!(refused(&ledger, &block(3, &w.id, 1, 0, vec![])), FROZEN);
        assert_eq!(
            build_err(&ledger, 3, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 1, false)),
            FROZEN_OP
        );
        assert_eq!(
            build_err(
                &ledger,
                3,
                &w,
                &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 1, false)
            ),
            FROZEN_OP
        );
        assert_eq!(
            build_err(&ledger, 3, &w, &op(Kind::Freeze, &w.cold, &w, 0, ZERO, 1, false)),
            "this identity is already frozen"
        );
        let mut posing = op(Kind::Unfreeze, &w.hot, &w, 0, ZERO, 1, false);
        posing.cold_root = Some(w.hot_root);
        assert_eq!(build_err(&ledger, 3, &w, &posing), NOT_COLD);
        for h in 3..20 {
            ledger = apply(ledger, &filler(h, vec![]));
        }
        assert!(now(&ledger, &w, 20).frozen, "nothing thaws on its own");
        let rec = land(&mut ledger, 20, &w, &op(Kind::Unfreeze, &w.cold, &w, 0, ZERO, 1, false));
        assert!(!rec.frozen && rec.cold_floor == 2);
        assert_eq!(
            build_err(&ledger, 21, &w, &op(Kind::Unfreeze, &w.cold, &w, 0, ZERO, 2, false)),
            "this identity is not frozen"
        );
        ledger = apply(ledger, &block(21, &w.id, 1, 0, vec![])); // the hot key signs again
        land(&mut ledger, 22, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 2, false));
        assert_eq!(now(&ledger, &w, 23).pending.map(|p| p.change), Some(Change::Rotation));
    }

    /// One pending key change at a time, and the cold key's operations take precedence:
    ///   a rotation pending + a cold-key recovery: the recovery replaces it and restarts the delay;
    ///   a recovery pending + a hot-key rotation: refused (the hot key cancels the request first);
    ///   frozen + a rotation pending: it never takes effect, even past its block, and unfreezing waits
    ///     until the cold key cancels it;
    ///   frozen + a recovery pending: the hot key cannot cancel it; unfreezing makes it cancelable again;
    ///   a recovery that takes effect unfreezes the identity.
    #[test]
    fn one_pending_key_change_at_a_time_and_the_cold_key_takes_precedence() {
        let w = wallet_of(0xB1, 0xB2);
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let fresh = wallet::key_root(&[0xB3; 32]);
        let other = wallet::key_root(&[0xB4; 32]);
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        // one key operation per identity per block, in either order: the hot key's and the cold key's
        // operations never race inside a block
        let rotate = op(Kind::Rotate, &w.hot, &w, 0, root1, 1, true);
        let (rot_tx, _) = build_tx(&ledger, 2, w.id, &rotate, &con()).unwrap();
        let (fz_tx, _) = build_tx(
            &ledger,
            2,
            w.id,
            &op(Kind::Freeze, &w.cold, &w, 0, ZERO, 0, true),
            &con(),
        )
        .unwrap();
        for txs in [vec![rot_tx.clone(), fz_tx.clone()], vec![fz_tx, rot_tx]] {
            assert_eq!(
                refused(&ledger, &filler(2, txs)),
                "a block carries at most one key operation per identity"
            );
        }
        land(&mut ledger, 2, &w, &rotate);
        let rec = land(&mut ledger, 3, &w, &op(Kind::Recover, &w.cold, &w, 0, fresh, 0, false));
        assert_eq!(
            rec.pending,
            Some(Pending {
                change: Change::Recovery,
                new_root: fresh,
                effective_at: 3 + DELAY + 1
            }),
            "the recovery replaces the rotation"
        );
        assert_eq!(
            build_err(&ledger, 4, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 2, false)),
            "a recovery request is pending for this identity; its hot key can cancel it first"
        );
        land(&mut ledger, 4, &w, &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 2, false));
        assert_eq!(now(&ledger, &w, 1000).hot_root, w.hot_root, "neither change took effect");
        // frozen with a rotation pending: suspended past its block; unfreezing waits for its cancel
        let r = land(&mut ledger, 5, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, 3, false));
        let rotation_at = r.pending.expect("pending").effective_at;
        land(&mut ledger, 6, &w, &op(Kind::Freeze, &w.cold, &w, 0, ZERO, 1, false));
        let hh = rotation_at + 3;
        for h in 7..hh {
            ledger = apply(ledger, &filler(h, vec![]));
        }
        let held = now(&ledger, &w, hh);
        assert_eq!(
            (held.generation, held.hot_root, held.frozen),
            (0, w.hot_root, true),
            "the rotation does not take effect while frozen"
        );
        assert_eq!(held.pending.map(|p| p.change), Some(Change::Rotation));
        assert_eq!(
            build_err(&ledger, hh, &w, &op(Kind::Unfreeze, &w.cold, &w, 0, ZERO, 2, false)),
            "a rotation is pending: the cold key cancels it before unfreezing, or unfreezing would let it take effect"
        );
        land(&mut ledger, hh, &w, &op(Kind::CancelRotation, &w.cold, &w, 0, ZERO, 2, false));
        // frozen with a recovery pending: the hot key cannot cancel it ...
        land(&mut ledger, hh + 1, &w, &op(Kind::Recover, &w.cold, &w, 0, fresh, 3, false));
        assert_eq!(
            build_err(
                &ledger,
                hh + 2,
                &w,
                &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 4, false)
            ),
            FROZEN_OP
        );
        // ... until the cold key unfreezes, which makes it cancelable again
        land(&mut ledger, hh + 2, &w, &op(Kind::Unfreeze, &w.cold, &w, 0, ZERO, 4, false));
        let (_, c) = build_tx(
            &ledger,
            hh + 3,
            w.id,
            &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 4, false),
            &con(),
        )
        .expect("cancelable once unfrozen");
        assert!(c.pending.is_none());
        // a frozen identity whose recovery takes effect is unfrozen, under the new key
        land(&mut ledger, hh + 3, &w, &op(Kind::Freeze, &w.cold, &w, 0, ZERO, 5, false));
        let r = land(&mut ledger, hh + 4, &w, &op(Kind::Recover, &w.cold, &w, 0, other, 6, false));
        let done = now(&ledger, &w, r.pending.expect("pending").effective_at);
        assert_eq!(
            (done.generation, done.hot_root, done.frozen, done.pending, done.hot_floor, done.cold_floor),
            (1, other, false, None, 0, 7)
        );
    }

    /// THE THEFT SCENARIO (decision 7; test plan item 11). A thief holds the hot key and starts a rotation
    /// to a key of his own. The owner freezes with the cold key, cancels the rotation and recovers. After
    /// the freeze every move the thief makes is refused, the thief's key never takes effect (not even
    /// past its own block), and the owner's new key signs T + 3 blocks after the freeze landed, which is
    /// at most 2T for any T >= 3: the cold key wins, and nothing the hot key does can stall it.
    #[test]
    fn the_cold_key_beats_a_thief_holding_the_hot_key() {
        let w = wallet_of(0x81, 0x82);
        let thief_key = [0x83u8; 32];
        let thief_root = wallet::key_root(&thief_key);
        let owner_key = [0x84u8; 32];
        let owner_root = wallet::key_root(&owner_key);
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        // block 2: the thief, holding the hot seed, starts a rotation to his own key
        let r = land(&mut ledger, 2, &w, &op(Kind::Rotate, &w.hot, &w, 0, thief_root, 1, true));
        let thief_at = r.pending.expect("pending").effective_at;
        assert_eq!(thief_at, 2 + DELAY + 1);
        // block 3: before any freeze the hot key still signs, for the thief too
        ledger = apply(ledger, &block(3, &w.id, 2, 0, vec![]));
        // block 4: the owner freezes with the cold key; the identity is credited nothing from this block
        let freeze_block = 4;
        let (ftx, frec) = build_tx(
            &ledger,
            freeze_block,
            w.id,
            &op(Kind::Freeze, &w.cold, &w, 0, ZERO, 10, false),
            &con(),
        )
        .unwrap();
        assert!(frec.frozen);
        assert_eq!(
            refused(&ledger, &block(freeze_block, &w.id, 3, 0, vec![ftx.clone()])),
            FROZEN
        );
        ledger = apply(ledger, &filler(freeze_block, vec![ftx]));
        // every move the thief can make with the hot key is refused: its own two operations, and each of
        // the cold key's four signed with the hot key (signed once here, tried at every block below)
        let hot_moves = [
            op(Kind::Rotate, &w.hot, &w, 0, wallet::key_root(&[0x85; 32]), 40, false),
            op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, 41, false),
        ];
        let posing: Vec<KeyOp> = [Kind::Unfreeze, Kind::CancelRotation, Kind::Recover, Kind::Freeze]
            .into_iter()
            .map(|kind| {
                let root = if kind == Kind::Recover { thief_root } else { ZERO };
                let mut o = op(kind, &w.hot, &w, 0, root, 42, false);
                o.cold_root = Some(w.hot_root);
                o
            })
            .collect();
        let thief_moves = |ledger: &Ledger, h: u64| {
            assert_eq!(refused(ledger, &block(h, &w.id, 40 + h, 0, vec![])), FROZEN, "block {h}: contribute");
            for o in &hot_moves {
                assert_eq!(build_err(ledger, h, &w, o), FROZEN_OP, "block {h}: {}", o.kind.name());
            }
            for o in &posing {
                assert_eq!(build_err(ledger, h, &w, o), NOT_COLD, "block {h}: {}", o.kind.name());
            }
        };
        thief_moves(&ledger, 5);
        // block 5: the owner cancels the thief's rotation; block 6: the owner recovers
        let c = land(&mut ledger, 5, &w, &op(Kind::CancelRotation, &w.cold, &w, 0, ZERO, 11, false));
        assert!(c.frozen && c.pending.is_none());
        let r = land(&mut ledger, 6, &w, &op(Kind::Recover, &w.cold, &w, 0, owner_root, 12, false));
        let owner_at = r.pending.expect("pending").effective_at;
        assert_eq!(owner_at, 6 + DELAY + 1);
        // the window, which runs past the block the thief's rotation named: the thief can do nothing,
        // and the hot key is still the stolen one, not the thief's
        assert!(thief_at < owner_at);
        for h in 7..owner_at {
            thief_moves(&ledger, h);
            assert_eq!(now(&ledger, &w, h).hot_root, w.hot_root);
            ledger = apply(ledger, &filler(h, vec![]));
        }
        // the owner's key is in effect, the identity unfrozen, within the bound
        assert_eq!(owner_at - freeze_block, DELAY + 3);
        assert!(owner_at - freeze_block <= 2 * DELAY);
        let done = now(&ledger, &w, owner_at);
        assert_eq!(
            (done.generation, done.hot_root, done.frozen, done.pending),
            (1, owner_root, false, None)
        );
        // the thief: the stolen key's label is refused, and neither the stolen key nor the thief's own
        // key signs anything for the identity
        assert_eq!(refused(&ledger, &block(owner_at, &w.id, 60, 0, vec![])), RETIRED);
        for key in [&w.hot, &thief_key] {
            assert_eq!(
                build_err(
                    &ledger,
                    owner_at,
                    &w,
                    &op(Kind::Rotate, key, &w, 1, wallet::key_root(&[0x86; 32]), 61, false)
                ),
                "the rotation is not signed by this identity's current hot key"
            );
        }
        // the owner's new key signs from index 0, and the standing stayed on the identity
        ledger = apply(ledger, &block(owner_at, &w.id, 0, 1, vec![]));
        assert!(ledger.pom.get(w.id.as_slice()).is_some_and(|s| *s > 0));
    }

    /// Exhaustion (design D4): every hot key keeps its last one-time index for a rotation. Neither a
    /// contribution nor a cancel may use it, so a key that has spent every other leaf still rotates.
    #[test]
    fn the_last_leaf_of_a_hot_key_signs_only_a_rotation() {
        let w = wallet_of(0xC1, 0xC2);
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let fresh = wallet::key_root(&[0xC3; 32]);
        let last = ROTATION_LEAF as u32;
        assert_eq!(ROTATION_LEAF, wallet::MAX_SIGS - 1);
        let mut ledger = apply(Ledger::new(), &block(1, &w.id, 0, 0, vec![]));
        // a recovery request makes the key record, so the rulebook reads this key's labels from here on
        land(&mut ledger, 2, &w, &op(Kind::Recover, &w.cold, &w, 0, fresh, 0, true));
        assert_eq!(
            refused(&ledger, &block(3, &w.id, ROTATION_LEAF, 0, vec![])),
            LEAF_CONTRIBUTION
        );
        ledger = apply(ledger, &block(3, &w.id, ROTATION_LEAF - 2, 0, vec![]));
        // a cancel cannot take the last leaf; it takes the one before it
        assert_eq!(
            build_err(
                &ledger,
                4,
                &w,
                &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, last, false)
            ),
            "the last one-time index of a hot key signs only a rotation"
        );
        land(&mut ledger, 4, &w, &op(Kind::CancelRecovery, &w.hot, &w, 0, ZERO, last - 1, false));
        // every leaf but the last is spent; the last still signs a rotation
        let r = land(&mut ledger, 5, &w, &op(Kind::Rotate, &w.hot, &w, 0, root1, last, false));
        assert_eq!(r.hot_floor, wallet::MAX_SIGS);
        let effective = r.pending.expect("pending").effective_at;
        assert_eq!(refused(&ledger, &block(6, &w.id, ROTATION_LEAF, 0, vec![])), RETIRED);
        for h in 6..effective {
            ledger = apply(ledger, &filler(h, vec![]));
        }
        ledger = apply(ledger, &block(effective, &w.id, 0, 1, vec![]));
        assert_eq!(now(&ledger, &w, effective + 1).hot_root, root1);
    }

    /// Standing stays with the identity across every key operation, and no key operation moves standing
    /// or finality weight: a twin chain with the same contribution cells and no key operation holds the
    /// same cells, index root, PoM, work clock and finality weight. Here: a rotation started, a freeze
    /// (during which the identity is credited nothing), the rotation cancelled by the cold key, a
    /// recovery on the frozen identity that takes effect, then a rotation of the recovered key that takes
    /// effect. ANTI-THEATER: fold the key cell into `cells` in `apply_transition` and the cells and PoM
    /// differ.
    #[test]
    fn key_operations_move_no_standing_and_no_finality_weight() {
        let w = wallet_of(0x71, 0x72);
        let root1 = wallet::key_root(&wallet::derive_key(&w.hot, 1));
        let fresh = [0x73u8; 32];
        let fresh_root = wallet::key_root(&fresh);
        let fresh1 = wallet::derive_key(&fresh, 1);
        let mut with = Node::new(0, vec![], con());
        let mut twin = Node::new(0, vec![], con());
        let step = |with: &mut Node, twin: &mut Node, b: Block| {
            with.ledger = apply_block(with.ledger.clone(), &b, &con()).expect("valid on the key chain");
            twin.apply(&Block {
                token_txs: vec![],
                ..b
            });
        };
        let tx = |with: &Node, h: u64, o: KeyOp| build_tx(&with.ledger, h, w.id, &o, &con()).unwrap().0;
        step(&mut with, &mut twin, block(1, &w.id, 0, 0, vec![]));
        let rot = tx(&with, 2, op(Kind::Rotate, &w.hot, &w, 0, root1, 1, true));
        step(&mut with, &mut twin, block(2, &w.id, 2, 0, vec![rot]));
        let freeze = tx(&with, 3, op(Kind::Freeze, &w.cold, &w, 0, ZERO, 0, false));
        step(&mut with, &mut twin, filler(3, vec![freeze]));
        let veto = tx(&with, 4, op(Kind::CancelRotation, &w.cold, &w, 0, ZERO, 1, false));
        step(&mut with, &mut twin, filler(4, vec![veto]));
        let req = tx(&with, 5, op(Kind::Recover, &w.cold, &w, 0, fresh_root, 2, false));
        step(&mut with, &mut twin, filler(5, vec![req]));
        for h in 6..=5 + DELAY {
            step(&mut with, &mut twin, filler(h, vec![]));
        }
        let h = 6 + DELAY; // the recovered key signs, unfrozen
        step(&mut with, &mut twin, block(h, &w.id, 0, 1, vec![]));
        let rot2 = tx(&with, h + 1, op(Kind::Rotate, &fresh, &w, 1, wallet::key_root(&fresh1), 1, false));
        step(&mut with, &mut twin, block(h + 1, &w.id, 2, 1, vec![rot2]));
        for hh in h + 2..h + 2 + DELAY {
            step(&mut with, &mut twin, block(hh, &w.id, hh, 1, vec![]));
        }
        step(&mut with, &mut twin, block(h + 2 + DELAY, &w.id, 0, 2, vec![])); // the third key signs
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
        assert!(
            with.ledger.pom.keys().all(|k| k.as_slice() == w.id.as_slice() || k.as_slice() == OTHER.as_slice()),
            "standing sits on the identity under test (across three keys) and the filler identity only"
        );
        assert!(with.ledger.pom.get(w.id.as_slice()).is_some_and(|s| *s > 0));
        assert_ne!(a.3, b.3, "only the token cells differ: the key cell");
        assert_eq!(
            now(&with.ledger, &w, h + 3 + DELAY).hot_root,
            wallet::key_root(&fresh1)
        );
    }
}
