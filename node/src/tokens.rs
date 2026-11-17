//! tokens.rs — starter Rust analogs of the ERC token standards in the Noesis cell model.
//!
//! CKB-shape (COMMITTED): a token lives in a [`Cell`]. `lock.args` = the transferable owner;
//! `type_script.code_hash` = the standard's program; `type_script.args` = the token / collection
//! identity (the issuer); `data` = the token's on-chain state. A transfer is consume-inputs /
//! produce-outputs, and the type-script enforces conservation. These are the REFERENCE analogs
//! (the on-VM RISC-V type-script port mirrors them later, single-sourced like the other cores);
//! deliberately minimal — the starter set a 2-node testnet needs to actually move value.
//!
//! NO PRICE / ATTESTATION ORACLE LAYER (Will 2026-06-16): because the chain's own value and
//! standing are native and the airgap is closed structurally, a token's accounting lives
//! ENTIRELY on-chain. Every invariant below is a pure function of the transaction's inputs and
//! outputs — there is no off-chain feed to attest, nothing to bridge, nothing to trust.

use crate::Cell;

/// Two cells belong to the SAME token iff their type-script (program + issuer args) matches.
fn same_token(c: &Cell, code_hash: &[u8; 32], args: &[u8]) -> bool {
    &c.type_script.code_hash == code_hash && c.type_script.args == args
}

// ============ Fungible — ERC-20 analog (CKB sUDT-style) ============

pub mod fungible {
    use super::same_token;
    use crate::Cell;

    /// Decode a fungible balance from a cell's data (u128 little-endian). Short/empty data reads
    /// as 0 — a cell carrying no valid amount holds no tokens.
    pub fn amount(cell: &Cell) -> u128 {
        if cell.data.len() < 16 {
            return 0;
        }
        let mut b = [0u8; 16];
        b.copy_from_slice(&cell.data[..16]);
        u128::from_le_bytes(b)
    }

    /// Encode an amount into a fresh data field.
    pub fn encode(amount: u128) -> Vec<u8> {
        amount.to_le_bytes().to_vec()
    }

    /// Total supply of one token across a cell set (matched by type-script).
    pub fn total(cells: &[Cell], code_hash: &[u8; 32], args: &[u8]) -> u128 {
        cells
            .iter()
            .filter(|c| same_token(c, code_hash, args))
            .map(amount)
            .fold(0u128, |acc, val| acc.saturating_add(val))
    }

    /// ERC-20 transfer invariant: output supply == input supply. Pure function of the tx.
    pub fn conserves(inputs: &[Cell], outputs: &[Cell], code_hash: &[u8; 32], args: &[u8]) -> bool {
        total(inputs, code_hash, args) == total(outputs, code_hash, args)
    }

    /// Supply may INCREASE only when the issuer authorizes the mint. The issuer identity is the
    /// token's `type_script.args`; `minter` is the authorizing party for this tx. Conservation
    /// and burns (supply decrease — owner destroys their own balance) are always permitted.
    pub fn mint_or_conserve(
        inputs: &[Cell],
        outputs: &[Cell],
        code_hash: &[u8; 32],
        args: &[u8],
        minter: &[u8],
    ) -> bool {
        let inp = total(inputs, code_hash, args);
        let out = total(outputs, code_hash, args);
        if out > inp {
            minter == args // only the issuer may mint new supply
        } else {
            true // conserve or burn
        }
    }
}

// ============ Non-fungible — ERC-721 analog ============

pub mod nft {
    use super::same_token;
    use crate::Cell;
    use std::collections::HashSet;

    /// A non-fungible token's unique id IS the cell's data payload (one cell, one token).
    pub fn token_id(cell: &Cell) -> &[u8] {
        &cell.data
    }

    fn ids<'a>(cells: &'a [Cell], code_hash: &[u8; 32], collection: &[u8]) -> Vec<&'a [u8]> {
        cells
            .iter()
            .filter(|c| same_token(c, code_hash, collection))
            .map(|c| c.data.as_slice())
            .collect()
    }

    /// Transfer invariant for a collection: the SET of token ids is preserved exactly — none
    /// created, none duplicated. (Pure move; no mint, no burn.)
    pub fn conserves(
        inputs: &[Cell],
        outputs: &[Cell],
        code_hash: &[u8; 32],
        collection: &[u8],
    ) -> bool {
        let outs = ids(outputs, code_hash, collection);
        let out_set: HashSet<&[u8]> = outs.iter().copied().collect();
        if out_set.len() != outs.len() {
            return false; // a duplicated id = forgery
        }
        let in_set: HashSet<&[u8]> = ids(inputs, code_hash, collection).into_iter().collect();
        in_set == out_set
    }

    /// The issuer (collection args) may introduce NEW ids; anyone else may only move or burn
    /// existing ids. Duplicate ids are never allowed.
    pub fn mint_or_conserve(
        inputs: &[Cell],
        outputs: &[Cell],
        code_hash: &[u8; 32],
        collection: &[u8],
        minter: &[u8],
    ) -> bool {
        let ins: HashSet<&[u8]> = ids(inputs, code_hash, collection).into_iter().collect();
        let outs = ids(outputs, code_hash, collection);
        let out_set: HashSet<&[u8]> = outs.iter().copied().collect();
        if out_set.len() != outs.len() {
            return false; // no duplicates, ever
        }
        let minted = out_set.difference(&ins).count() > 0;
        if minted && minter != collection {
            return false; // only the issuer mints new ids
        }
        true // moves and burns are open
    }
}

// ============ Multi-token — ERC-1155 analog ============

pub mod multi {
    use super::same_token;
    use crate::Cell;
    use std::collections::HashMap;

    /// (sub-token id, amount) decoded from a cell: data = id (u32 LE) ++ amount (u128 LE).
    pub fn entry(cell: &Cell) -> Option<(u32, u128)> {
        if cell.data.len() < 20 {
            return None;
        }
        let id = u32::from_le_bytes(cell.data[0..4].try_into().unwrap());
        let amt = u128::from_le_bytes(cell.data[4..20].try_into().unwrap());
        Some((id, amt))
    }

    /// Encode a (sub-token id, amount) into a cell data field.
    pub fn encode(id: u32, amount: u128) -> Vec<u8> {
        let mut out = Vec::with_capacity(20);
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&amount.to_le_bytes());
        out
    }

    fn totals(cells: &[Cell], code_hash: &[u8; 32], args: &[u8]) -> HashMap<u32, u128> {
        let mut m: HashMap<u32, u128> = HashMap::new();
        for c in cells.iter().filter(|c| same_token(c, code_hash, args)) {
            if let Some((id, amt)) = entry(c) {
                // saturating: a crafted set of cells whose amounts sum past u128::MAX must not
                // wrap a total down and let conserves() falsely balance (forged supply). Mirrors
                // fungible::total; byte-identical for any real (non-overflowing) token set.
                let slot = m.entry(id).or_insert(0);
                *slot = slot.saturating_add(amt);
            }
        }
        m
    }

    /// ERC-1155 conservation: every sub-token id conserves INDEPENDENTLY (a batch transfer can
    /// move many ids at once; each must balance). Pure function of the tx.
    pub fn conserves(inputs: &[Cell], outputs: &[Cell], code_hash: &[u8; 32], args: &[u8]) -> bool {
        totals(inputs, code_hash, args) == totals(outputs, code_hash, args)
    }
}

// ============ Soulbound identity — ERC-5484 "IssuerOnly" analog ============

/// A soulbound identity collection: one cell per serial, the holder's address fixed inside the cell,
/// the cell held by the issuer. Only the issuer mints, re-tiers or burns; nobody moves a cell to a
/// new holder. It lives in `Ledger.token_cells` like every token, so PoM standing, the novelty index
/// and finality weight never read it (they fold over `Ledger.cells` only).
///
/// ISSUER AUTHORITY IS A LINEAR CELL, NOT A KEY EQUALITY. Fungible/Nft derive mint authority from an
/// input whose `lock.args` equals the token args (`runtime::TokenTx::is_valid`), which pins the
/// authority's lock key forever; with one-time Lamport locks a second mint must then reuse that key
/// (`one_mint_limit_of_the_lock_args_rule_is_real`, runtime tests). Here authority is the one cell of
/// the collection that carries an AUTHORITY record. No tx can create one (genesis seeds it), each
/// spend must re-create it with `generation + 1` or retire it, and its lock is free, so the issuer
/// moves it to a fresh one-time key on every spend (a key chain).
///
/// Data layouts. No off-chain account id, name, count or text ever goes in a cell.
///   identity  = `[RECORD_V1][serial u64 LE][tier u8][holder 32 bytes]` (42 bytes)
///   authority = `[AUTHORITY_TAG][generation u64 LE]` (9 bytes)
pub mod soulbound {
    use super::same_token;
    use crate::Cell;
    use std::collections::{HashMap, HashSet};

    /// The standard's type-script program id. A reserved placeholder tag (the `JUL_CODE_HASH`
    /// precedent) until the on-VM type-script lands. Mirrored in `onchain/locksig-typescript`
    /// `standard_of` (tag 3); `node/tests/ckb_vm_locksig.rs` fails if the two drift.
    pub const CODE_HASH: [u8; 32] = *b"NOESIS-SOULBOUND-IDENTITY-TYPE-1";
    /// First byte of an identity record (its layout version).
    pub const RECORD_V1: u8 = 1;
    /// First byte of the authority record.
    pub const AUTHORITY_TAG: u8 = 0xA1;
    const IDENTITY_LEN: usize = 42;
    const AUTHORITY_LEN: usize = 9;

    /// The one mutable field of an identity.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Tier {
        Identity,
        Member,
        Revoked,
    }

    impl Tier {
        pub fn byte(self) -> u8 {
            match self {
                Tier::Identity => 0,
                Tier::Member => 1,
                Tier::Revoked => 2,
            }
        }
        pub fn from_byte(b: u8) -> Option<Tier> {
            match b {
                0 => Some(Tier::Identity),
                1 => Some(Tier::Member),
                2 => Some(Tier::Revoked),
                _ => None,
            }
        }
        pub fn name(self) -> &'static str {
            match self {
                Tier::Identity => "identity",
                Tier::Member => "member",
                Tier::Revoked => "revoked",
            }
        }
        pub fn from_name(s: &str) -> Option<Tier> {
            match s {
                "identity" => Some(Tier::Identity),
                "member" => Some(Tier::Member),
                "revoked" => Some(Tier::Revoked),
                _ => None,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Identity {
        pub serial: u64,
        pub tier: Tier,
        /// The holder's 32-byte Noesis address. Fixed for the cell's whole life.
        pub holder: [u8; 32],
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Record {
        Authority { generation: u64 },
        Identity(Identity),
    }

    pub fn encode_identity(i: &Identity) -> Vec<u8> {
        let mut out = Vec::with_capacity(IDENTITY_LEN);
        out.push(RECORD_V1);
        out.extend_from_slice(&i.serial.to_le_bytes());
        out.push(i.tier.byte());
        out.extend_from_slice(&i.holder);
        out
    }

    pub fn encode_authority(generation: u64) -> Vec<u8> {
        let mut out = Vec::with_capacity(AUTHORITY_LEN);
        out.push(AUTHORITY_TAG);
        out.extend_from_slice(&generation.to_le_bytes());
        out
    }

    /// Parse a cell's data. Exact lengths only; anything else is not a record.
    pub fn decode(data: &[u8]) -> Option<Record> {
        match (data.first().copied(), data.len()) {
            (Some(AUTHORITY_TAG), AUTHORITY_LEN) => {
                Some(Record::Authority { generation: u64::from_le_bytes(data[1..9].try_into().ok()?) })
            }
            (Some(RECORD_V1), IDENTITY_LEN) => Some(Record::Identity(Identity {
                serial: u64::from_le_bytes(data[1..9].try_into().ok()?),
                tier: Tier::from_byte(data[9])?,
                holder: data[10..42].try_into().ok()?,
            })),
            _ => None,
        }
    }

    /// One side of a tx, for one collection: authority generations, identities by serial. `None`
    /// if a cell of the collection is not a record or a serial repeats on this side.
    type Side = (Vec<u64>, HashMap<u64, Identity>);
    fn side(cells: &[Cell], code_hash: &[u8; 32], collection: &[u8]) -> Option<Side> {
        let mut auth = Vec::new();
        let mut ids = HashMap::new();
        for c in cells.iter().filter(|c| same_token(c, code_hash, collection)) {
            match decode(&c.data)? {
                Record::Authority { generation } => auth.push(generation),
                Record::Identity(i) => {
                    if ids.insert(i.serial, i).is_some() {
                        return None; // a duplicate serial
                    }
                }
            }
        }
        Some((auth, ids))
    }

    /// The type rule, a pure function of the tx:
    /// 1. every cell of the collection is a well-formed record, and no serial repeats on a side;
    /// 2. authority is linear: at most one authority input, never more authority out than in, and
    ///    a re-created authority carries exactly `generation + 1` (so a tx can never create it);
    /// 3. any identity in the inputs (re-tier, burn) or outputs (mint, re-tier) needs the issuer,
    ///    i.e. the authority input;
    /// 4. a serial in both inputs and outputs keeps its holder. Only the tier may change.
    pub fn valid(inputs: &[Cell], outputs: &[Cell], code_hash: &[u8; 32], collection: &[u8]) -> bool {
        let (Some((auth_in, ids_in)), Some((auth_out, ids_out))) =
            (side(inputs, code_hash, collection), side(outputs, code_hash, collection))
        else {
            return false;
        };
        if auth_in.len() > 1 || auth_out.len() > auth_in.len() {
            return false;
        }
        if let (Some(&g_in), Some(&g_out)) = (auth_in.first(), auth_out.first()) {
            if g_in.checked_add(1) != Some(g_out) {
                return false;
            }
        }
        let issuer = auth_in.len() == 1;
        if (!ids_in.is_empty() || !ids_out.is_empty()) && !issuer {
            return false;
        }
        ids_out.iter().all(|(serial, out)| ids_in.get(serial).map_or(true, |inp| inp.holder == out.holder))
    }

    fn serials<'a>(cells: &'a [Cell], code_hash: &'a [u8; 32], collection: &'a [u8]) -> impl Iterator<Item = u64> + 'a {
        cells.iter().filter(move |c| same_token(c, code_hash, collection)).filter_map(|c| match decode(&c.data) {
            Some(Record::Identity(i)) => Some(i.serial),
            _ => None,
        })
    }

    /// The ledger half of "no duplicate serial": a serial the tx mints (in its outputs, not its
    /// inputs) must not already be live in the collection.
    pub fn serials_fresh(live: &[Cell], inputs: &[Cell], outputs: &[Cell], code_hash: &[u8; 32], collection: &[u8]) -> bool {
        let spent: HashSet<u64> = serials(inputs, code_hash, collection).collect();
        let held: HashSet<u64> = serials(live, code_hash, collection).collect();
        serials(outputs, code_hash, collection).all(|s| spent.contains(&s) || !held.contains(&s))
    }
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Script;

    const FT: [u8; 32] = [20u8; 32]; // fungible standard program
    const NFT: [u8; 32] = [21u8; 32]; // nft standard program
    const MT: [u8; 32] = [22u8; 32]; // multi standard program

    fn tok(code: [u8; 32], issuer: &[u8], owner: &[u8], data: Vec<u8>) -> Cell {
        Cell {
            id: 0,
            lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
            type_script: Script { code_hash: code, args: issuer.to_vec() },
            parent: None,
            timestamp: 0,
            data,
        }
    }

    fn ft(issuer: &[u8], owner: &[u8], amt: u128) -> Cell {
        tok(FT, issuer, owner, fungible::encode(amt))
    }

    #[test]
    fn fungible_split_conserves() {
        let inp = vec![ft(b"USD", b"alice", 10)];
        let out = vec![ft(b"USD", b"alice", 7), ft(b"USD", b"bob", 3)];
        assert!(fungible::conserves(&inp, &out, &FT, b"USD"));
    }

    #[test]
    fn fungible_inflation_is_rejected() {
        let inp = vec![ft(b"USD", b"alice", 10)];
        let out = vec![ft(b"USD", b"alice", 11)];
        assert!(!fungible::conserves(&inp, &out, &FT, b"USD"));
        // and the mint path rejects it too, because the minter is not the issuer
        assert!(!fungible::mint_or_conserve(&inp, &out, &FT, b"USD", b"alice"));
    }

    #[test]
    fn fungible_total_saturates_on_overflow() {
        // Adversarial: many cells whose sum exceeds u128::MAX. A wrapping .sum() could
        // wrap DOWN and let conserves()/mint_or_conserve() pass a forged supply. Saturating
        // pins the total at u128::MAX so the overflow can never masquerade as a small number.
        let big = (u128::MAX / 9) + 1;
        let cells: Vec<Cell> = (0..10).map(|_| ft(b"USD", b"alice", big)).collect();
        assert_eq!(fungible::total(&cells, &FT, b"USD"), u128::MAX);
    }

    #[test]
    fn fungible_issuer_may_mint_others_may_not() {
        let inp = vec![ft(b"USD", b"USD", 0)];
        let out = vec![ft(b"USD", b"USD", 1000)];
        assert!(fungible::mint_or_conserve(&inp, &out, &FT, b"USD", b"USD"));
        assert!(!fungible::mint_or_conserve(&inp, &out, &FT, b"USD", b"mallory"));
    }

    #[test]
    fn fungible_burn_is_allowed() {
        let inp = vec![ft(b"USD", b"alice", 10)];
        let out = vec![ft(b"USD", b"alice", 4)];
        assert!(fungible::mint_or_conserve(&inp, &out, &FT, b"USD", b"alice"));
    }

    #[test]
    fn nft_move_preserves_id_set() {
        let inp = vec![tok(NFT, b"art", b"alice", b"mona-lisa".to_vec())];
        let out = vec![tok(NFT, b"art", b"bob", b"mona-lisa".to_vec())];
        assert!(nft::conserves(&inp, &out, &NFT, b"art"));
        assert_eq!(nft::token_id(&out[0]), b"mona-lisa");
    }

    #[test]
    fn nft_duplicate_is_forgery() {
        let inp = vec![tok(NFT, b"art", b"alice", b"mona-lisa".to_vec())];
        let out = vec![
            tok(NFT, b"art", b"alice", b"mona-lisa".to_vec()),
            tok(NFT, b"art", b"bob", b"mona-lisa".to_vec()),
        ];
        assert!(!nft::conserves(&inp, &out, &NFT, b"art"));
        assert!(!nft::mint_or_conserve(&inp, &out, &NFT, b"art", b"art"));
    }

    #[test]
    fn nft_only_issuer_mints_new_ids() {
        let inp: Vec<Cell> = vec![];
        let out = vec![tok(NFT, b"art", b"alice", b"new-piece".to_vec())];
        assert!(nft::mint_or_conserve(&inp, &out, &NFT, b"art", b"art"));
        assert!(!nft::mint_or_conserve(&inp, &out, &NFT, b"art", b"alice"));
    }

    #[test]
    fn multi_conserves_per_id_independently() {
        let inp = vec![
            tok(MT, b"game", b"alice", multi::encode(1, 100)),
            tok(MT, b"game", b"alice", multi::encode(2, 5)),
        ];
        // move 40 of id-1 to bob, keep id-2 whole — both ids must still balance
        let out = vec![
            tok(MT, b"game", b"alice", multi::encode(1, 60)),
            tok(MT, b"game", b"bob", multi::encode(1, 40)),
            tok(MT, b"game", b"alice", multi::encode(2, 5)),
        ];
        assert!(multi::conserves(&inp, &out, &MT, b"game"));
    }

    #[test]
    fn multi_rejects_cross_id_leak() {
        // taking from id-2 to inflate id-1 must fail (per-id, not aggregate, conservation)
        let inp = vec![
            tok(MT, b"game", b"alice", multi::encode(1, 100)),
            tok(MT, b"game", b"alice", multi::encode(2, 5)),
        ];
        let out = vec![
            tok(MT, b"game", b"alice", multi::encode(1, 105)),
            tok(MT, b"game", b"alice", multi::encode(2, 0)),
        ];
        assert!(!multi::conserves(&inp, &out, &MT, b"game"));
    }

    // ---- soulbound identity ----

    use soulbound::{Identity, Tier};
    const SB: [u8; 32] = soulbound::CODE_HASH;
    const COL: &[u8] = b"test-identity-v1";

    fn authority(lock: &[u8], generation: u64) -> Cell {
        tok(SB, COL, lock, soulbound::encode_authority(generation))
    }
    fn ident(lock: &[u8], serial: u64, tier: Tier, holder: u8) -> Cell {
        tok(SB, COL, lock, soulbound::encode_identity(&Identity { serial, tier, holder: [holder; 32] }))
    }
    fn ok(inputs: &[Cell], outputs: &[Cell]) -> bool {
        soulbound::valid(inputs, outputs, &SB, COL)
    }

    #[test]
    fn soulbound_records_round_trip_and_reject_malformed_data() {
        let i = Identity { serial: 7, tier: Tier::Member, holder: [3; 32] };
        assert_eq!(soulbound::decode(&soulbound::encode_identity(&i)), Some(soulbound::Record::Identity(i)));
        assert_eq!(
            soulbound::decode(&soulbound::encode_authority(4)),
            Some(soulbound::Record::Authority { generation: 4 })
        );
        let mut bad_tier = soulbound::encode_identity(&i);
        bad_tier[9] = 3;
        assert_eq!(soulbound::decode(&bad_tier), None, "tier byte outside 0..=2");
        let mut long = soulbound::encode_identity(&i);
        long.push(0);
        assert_eq!(soulbound::decode(&long), None, "trailing byte");
        assert_eq!(soulbound::decode(&[]), None);
        // a malformed cell of the collection makes the whole tx invalid
        assert!(!ok(&[authority(b"k0", 0)], &[authority(b"k1", 1), tok(SB, COL, b"k1", vec![1, 2, 3])]));
    }

    #[test]
    fn soulbound_issuer_mint_passes() {
        let ins = vec![authority(b"k0", 0)];
        let outs = vec![authority(b"k1", 1), ident(b"c1", 1, Tier::Identity, 9), ident(b"c2", 2, Tier::Member, 8)];
        assert!(ok(&ins, &outs), "the issuer spends its authority, re-creates it at generation + 1, mints two serials");
    }

    #[test]
    fn soulbound_mint_without_the_authority_fails() {
        // anyone else: no authority input, whatever lock they name
        assert!(!ok(&[], &[ident(COL, 1, Tier::Identity, 9)]), "a mint from nothing");
        assert!(!ok(&[], &[ident(b"k0", 1, Tier::Identity, 9)]));
        // and nobody can create an authority to mint with
        assert!(!ok(&[], &[authority(b"mallory", 0), ident(b"m", 1, Tier::Identity, 9)]), "authority out of nothing");
        assert!(!ok(&[authority(b"k0", 0)], &[authority(b"k1", 1), authority(b"m", 1)]), "authority duplicated");
    }

    #[test]
    fn soulbound_authority_generation_must_advance_by_one() {
        assert!(!ok(&[authority(b"k0", 3)], &[authority(b"k1", 3)]), "same generation");
        assert!(!ok(&[authority(b"k0", 3)], &[authority(b"k1", 2)]), "generation rolled back");
        assert!(!ok(&[authority(b"k0", 3)], &[authority(b"k1", 5)]), "generation skipped");
        assert!(!ok(&[authority(b"k0", u64::MAX)], &[authority(b"k1", 0)]), "no wrap");
        assert!(ok(&[authority(b"k0", 3)], &[authority(b"k1", 4)]), "a bare key rotation is allowed");
        assert!(ok(&[authority(b"k0", 3)], &[]), "retiring the authority closes the collection");
    }

    #[test]
    fn soulbound_changing_the_holder_fails() {
        let ins = vec![authority(b"k0", 0), ident(b"c1", 1, Tier::Identity, 9)];
        let moved = vec![authority(b"k1", 1), ident(b"c1", 1, Tier::Identity, 8)];
        assert!(!ok(&ins, &moved), "even the issuer cannot change the holder");
        // and without the issuer nobody can move the cell at all, not even to the same holder
        assert!(!ok(&[ident(b"c1", 1, Tier::Identity, 9)], &[ident(b"thief", 1, Tier::Identity, 9)]));
    }

    #[test]
    fn soulbound_issuer_retier_passes_and_nobody_else_can() {
        let ins = vec![authority(b"k0", 0), ident(b"c1", 1, Tier::Identity, 9)];
        for tier in [Tier::Member, Tier::Revoked, Tier::Identity] {
            let outs = vec![authority(b"k1", 1), ident(b"c2", 1, tier, 9)];
            assert!(ok(&ins, &outs), "issuer re-tier to {}", tier.name());
        }
        assert!(!ok(&[ident(b"c1", 1, Tier::Identity, 9)], &[ident(b"c1", 1, Tier::Member, 9)]), "no issuer");
    }

    #[test]
    fn soulbound_issuer_burn_passes_and_nobody_else_can() {
        let ins = vec![authority(b"k0", 0), ident(b"c1", 1, Tier::Revoked, 9)];
        assert!(ok(&ins, &[authority(b"k1", 1)]), "issuer burn");
        assert!(!ok(&[ident(b"c1", 1, Tier::Revoked, 9)], &[]), "a burn without the issuer");
    }

    #[test]
    fn soulbound_duplicate_serial_fails() {
        let ins = vec![authority(b"k0", 0)];
        let outs = vec![authority(b"k1", 1), ident(b"c1", 5, Tier::Identity, 9), ident(b"c2", 5, Tier::Identity, 8)];
        assert!(!ok(&ins, &outs), "two cells with one serial in a tx");
        // ledger half: minting a serial that is already live, outside this tx's inputs
        let live = vec![ident(b"c0", 5, Tier::Identity, 7)];
        let mint5 = vec![authority(b"k1", 1), ident(b"c1", 5, Tier::Identity, 9)];
        assert!(ok(&ins, &mint5), "the pure rule cannot see the ledger");
        assert!(!soulbound::serials_fresh(&live, &ins, &mint5, &SB, COL), "serial 5 is already live");
        // a re-tier of the live serial spends it first, so it is not a duplicate
        let retier_in = vec![authority(b"k0", 0), ident(b"c0", 5, Tier::Identity, 7)];
        let retier_out = vec![authority(b"k1", 1), ident(b"c1", 5, Tier::Member, 7)];
        assert!(soulbound::serials_fresh(&live, &retier_in, &retier_out, &SB, COL));
    }
}
