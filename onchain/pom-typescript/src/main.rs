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
//!   12 = input cell data unavailable
//!   13 = semantic floor: payload is incompressible noise (entropy ≥ θ = 62259/2^16)
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

const Q: u32 = 16;
const THETA_ENT_Q16: i128 = 62259; // floor(0.95 · 2^16) — same constant as node value_fixed

/// Q16.16 log2 — line-for-line the node `value_fixed::log2_q16` algorithm
/// (shift-and-square, 16 bounded iterations, pure integer).
fn log2_q16(x: u64) -> u64 {
    let ip = 63 - u64::from(x.leading_zeros());
    let mut m: u128 = ((x as u128) << 32) >> ip;
    let mut frac: u64 = 0;
    let mut i = Q;
    while i > 0 {
        i -= 1;
        m = (m * m) >> 32;
        if m >= (2u128 << 32) {
            m >>= 1;
            frac |= 1 << i;
        }
    }
    (ip << Q) | frac
}

/// Mirror of node `value_fixed::is_incompressible_q16` at the suite theta.
fn is_incompressible(data: &[u8]) -> bool {
    let n = data.len() as u64;
    if n < 2 {
        return false; // theta > 0; zero-entropy payload passes
    }
    let mut counts = [0u64; 256];
    let mut i = 0;
    while i < data.len() {
        counts[data[i] as usize] += 1;
        i += 1;
    }
    let mut sum_clog: i128 = 0;
    let mut b = 0;
    while b < 256 {
        if counts[b] > 0 {
            sum_clog += (counts[b] as i128) * (log2_q16(counts[b]) as i128);
        }
        b += 1;
    }
    let lhs: i128 = (n as i128) * (log2_q16(n) as i128) - sum_clog;
    let m = n.min(256);
    let rhs: i128 = (THETA_ENT_Q16 * (n as i128) * (log2_q16(m) as i128)) >> Q;
    lhs >= rhs
}

pub fn program_entry() -> i8 {
    let script = match load_script() {
        Ok(s) => s,
        Err(_) => return 10,
    };
    if script.as_reader().args().raw_data().is_empty() {
        return 11; // soulbound identity is not optional
    }
    // Iterate the WHOLE script group until INDEX_OUT_OF_BOUND — a tx cannot smuggle a
    // failing cell at a later index (flips on_vm_floor_checks_only_input_zero_open_gap).
    let mut index = 0usize;
    loop {
        match load_cell_data(index, Source::GroupInput) {
            Ok(data) => {
                if is_incompressible(&data) {
                    return 13; // semantic floor, on-VM, every group input
                }
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(_) => return 12,
        }
    }
    if index == 0 {
        return 12; // a type-script with no group inputs has nothing to attest
    }
    0
}
