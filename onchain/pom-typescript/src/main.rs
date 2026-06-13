//! Noesis PoM type-script — the INTAKE FLOORS running ON-VM (RISC-V / CKB-VM).
//! CKB-VM-PORT.md increment #3, second half.
//!
//! This is the first Noesis mechanism code that executes inside the VM instead of
//! beside it: the semantic/compressibility floor in the SAME Q16.16 integer form as
//! `node` `value_fixed` (the canonical on-chain arithmetic — bit-identical, no floats).
//!
//! Validation (exit codes are distinct for test triage, vibeswap-cell convention):
//!   0  = pass
//!   11 = soulbound contributor identity missing (empty type-script args)
//!   12 = empty script group (nothing to attest) or unexpected load error
//!   13 = semantic floor: a group INPUT is incompressible noise (θ = 62259/2^16)
//!   14 = semantic floor: a group OUTPUT is incompressible noise — the MINT side
//!   20 = index root missing/malformed (cell-dep 0) or witness missing/short
//!   21 = a proof fails to classify its shingle (omission/padding/tamper/stale root)
//!   22 = mint denied: proven floored novelty is ZERO (redundant or floored content)
//!
//! T7 #4 second half: every group OUTPUT must additionally PROVE its novelty against
//! the live novelty-index root (cell-dep 0, 32 raw bytes). Witness i (GroupOutput) is
//! the canonical proof blob: concatenated 64x32-byte sibling paths, one per unique
//! shingle of output i's data, in sorted-unique order — nothing else. Proofs are
//! verified STREAMING (one 2KB path at a time, fixed buffer, no large allocation)
//! via noesis_core::classify; the floor arithmetic is noesis_core::floored_from_counts
//! — the SAME single-source functions the node drift-guards.
//!
//! Honest scope: this enforces the SEMANTIC floor on-VM, over EVERY input in the
//! script group (closes the input-0-only smuggling gap found by the 2026-06-12
//! adversarial tick). The similarity floor still needs cross-cell state (the
//! seen-shingle set) served via syscalls — that is the next piece, not claimed here.

#![no_std]
#![no_main]

use ckb_std::{
    ckb_constants::Source,
    default_alloc,
    error::SysError,
    high_level::{load_cell_data, load_script},
    syscalls,
};

ckb_std::entry!(program_entry);
default_alloc!();

const THETA_ENT_Q16: u64 = 62259; // same constants as node value_fixed / noesis-core
const THETA_SIM_Q16: u64 = 52429;
const PATH_BYTES: usize = noesis_core::DEPTH * 32;

/// Verify output `index`'s proven novelty: stream witness `index` one sibling path at a
/// time against the index root. Returns the floored novelty or an exit code.
fn proven_mint_value(index: usize, data: &[u8], root: noesis_core::Hash) -> Result<u64, i8> {
    let uniq = noesis_core::unique_shingles(data);
    let expected = uniq.len() * PATH_BYTES;
    let mut buf = [0u8; PATH_BYTES];
    // probe total witness length via a 1-byte read (partial-load writes back full size)
    let total = match syscalls::load_witness(&mut buf[..1], 0, index, Source::GroupOutput) {
        Ok(n) => n,
        Err(SysError::LengthNotEnough(actual)) => actual,
        Err(_) => return Err(20),
    };
    if total != expected {
        return Err(20); // omission or padding is rejected before any verification
    }
    let mut novelty_occ = 0u64;
    let mut overlap_uniq = 0u64;
    for (i, (key, mult)) in uniq.iter().enumerate() {
        match syscalls::load_witness(&mut buf, i * PATH_BYTES, index, Source::GroupOutput) {
            Ok(_) | Err(SysError::LengthNotEnough(_)) => {}
            Err(_) => return Err(20),
        }
        let mut path = [[0u8; 32]; noesis_core::DEPTH];
        for (j, chunk) in buf.chunks_exact(32).enumerate() {
            path[j].copy_from_slice(chunk);
        }
        match noesis_core::classify(root, *key, &path) {
            Some(noesis_core::Class::Member) => overlap_uniq += 1,
            Some(noesis_core::Class::Absent) => novelty_occ += mult,
            None => return Err(21),
        }
    }
    Ok(noesis_core::floored_from_counts(
        novelty_occ,
        overlap_uniq,
        uniq.len() as u64,
        data,
        THETA_SIM_Q16,
        THETA_ENT_Q16,
    ))
}

/// Single-source verify core (T7 #4 first half): the floor logic now comes from
/// noesis-core — the SAME crate the node drift-guards against its own lib.
fn is_incompressible(data: &[u8]) -> bool {
    noesis_core::is_incompressible_q16(data, THETA_ENT_Q16)
}

pub fn program_entry() -> i8 {
    let script = match load_script() {
        Ok(s) => s,
        Err(_) => return 10,
    };
    if script.as_reader().args().raw_data().is_empty() {
        return 11; // soulbound identity is not optional
    }
    // Iterate the WHOLE script group in BOTH directions until INDEX_OUT_OF_BOUND —
    // neither a consumed nor a freshly-MINTED cell can smuggle noise past the floor
    // (closes ROADMAP T6: mint-side was the survivor of the group-input fix).
    let mut checked = 0usize;
    let mut index = 0usize;
    loop {
        match load_cell_data(index, Source::GroupInput) {
            Ok(data) => {
                if is_incompressible(&data) {
                    return 13; // consumed-side floor
                }
                index += 1;
                checked += 1;
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(_) => return 12,
        }
    }
    // Mint side: semantic floor AND the proven history floors (T7 #4). The index root
    // rides in cell-dep 0 — exactly 32 raw bytes — and is only demanded when the tx
    // actually mints.
    index = 0;
    let mut root: Option<noesis_core::Hash> = None;
    loop {
        match load_cell_data(index, Source::GroupOutput) {
            Ok(data) => {
                if is_incompressible(&data) {
                    return 14; // MINT-side floor — distinct code for triage
                }
                let r = match root {
                    Some(r) => r,
                    None => match load_cell_data(0, Source::CellDep) {
                        Ok(rd) if rd.len() == 32 => {
                            let mut h = [0u8; 32];
                            h.copy_from_slice(&rd);
                            root = Some(h);
                            h
                        }
                        _ => return 20,
                    },
                };
                match proven_mint_value(index, &data, r) {
                    Ok(0) => return 22, // proven worthless: redundant against history
                    Ok(_) => {}
                    Err(code) => return code,
                }
                index += 1;
                checked += 1;
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(_) => return 12,
        }
    }
    if checked == 0 {
        return 12; // empty group: nothing to attest (burn-only and mint-only are fine)
    }
    0
}
