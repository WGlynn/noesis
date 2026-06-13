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
};

ckb_std::entry!(program_entry);
default_alloc!();

const THETA_ENT_Q16: u64 = 62259; // same constant as node value_fixed / noesis-core

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
    index = 0;
    loop {
        match load_cell_data(index, Source::GroupOutput) {
            Ok(data) => {
                if is_incompressible(&data) {
                    return 14; // MINT-side floor — distinct code for triage
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
