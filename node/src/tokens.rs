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
            .sum()
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
                *m.entry(id).or_insert(0) += amt;
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
}
