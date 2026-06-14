//! Noesis finalization type-script — PoM-weighted finalization running ON-VM (RISC-V / CKB-VM).
//! ON-VM-FINALIZATION.md build-order step 2: the consensus finalize rule, recomputed inside the
//! VM in the SAME Q32.32 integer form as the node reference (`finalization_fixed`, single-sourced
//! from `noesis_core::finalization`). The script gates a "finalization cell" — a cell asserting a
//! proposal finalized — and REFUSES to validate it unless the vote actually clears the bar.
//!
//! Validation (exit codes distinct for test triage; the 30s namespace keeps them apart from the
//! intake type-script's 0/11-23):
//!   0  = pass — every finalization cell in the group has a vote that truly finalizes
//!   30 = a finalization cell's vote does NOT clear the threshold (false claim, rejected)
//!   31 = malformed finalization cell data (missing, short header, bad record length) / empty group
//!   32 = malformed votes witness (odd length, or an index outside the validator set)
//!   33 = header missing/short ⇒ `now` cannot be consensus-sourced (mandatory)
//!   34 = validator-registry unbound/mismatch (forged `all` source; sentinel-gated inert pre-deploy)
//!
//! THE adversarial point (ON-VM-FINALIZATION.md §3): `now` is read from the block HEADER, never a
//! tx-chosen witness/arg. `effective_weight` decays with `now`, so a free `now` lets an attacker
//! pick the timestamp that decays opponents and forge finalization. The header-dep timestamp is
//! consensus-bound (a tx assembler cannot forge a real chain header) ⇒ the input is not
//! attacker-choosable. Same lesson as the index-dep binding (F1) and the temporal-order coords.

#![no_std]
#![no_main]

// NOTE: no `extern crate alloc` here — ckb_std::entry!/default_alloc! already declares it
// ([F·ckb-cell-build-recipe]); alloc::vec::Vec resolves through that declaration.

use ckb_std::{
    ckb_constants::Source,
    default_alloc,
    error::SysError,
    high_level::load_cell_data,
    syscalls,
};
use core::convert::TryInto;
use noesis_core::finalization::{finalizes_fixed, parse_finalization_cell, parse_votes, ValidatorQ};

ckb_std::entry!(program_entry);
default_alloc!();

/// Validator-set provenance (ON-VM-FINALIZATION.md §"Validator-set provenance"). When ACTIVE, the
/// `all` set is RE-DERIVED from the canonical validator-registry cell (type-id singleton + identity,
/// same mechanism as INDEX-DEP-CODEHASH-BINDING.md) and a caller-supplied set that omits honest
/// validators is REFUSED — a curated `all` shrinks the basis until a minority clears it. INACTIVE
/// pre-deploy (the registry cell isn't deployed yet): the set rides in the finalization cell data
/// and the curated-validator-set-rejected fixture is the deploy-coupled activated path — an HONEST
/// pin exactly like the index-dep F1/F2/F3 activated path, not a claim it is enforced today.
const REGISTRY_BINDING_ACTIVE: bool = false;

fn validator_set_bound() -> bool {
    if !REGISTRY_BINDING_ACTIVE {
        return true; // pre-deploy: cell-carried set (shape path); binding not yet activated
    }
    // post-deploy: load the registry cell-dep, assert type-id identity, re-derive `all` from it,
    // reject any witness/cell-supplied set. Lands with the registry deploy.
    false
}

/// `now` from the block header — CONSENSUS-sourced, never tx-chosen. `RawHeader` is a fixed CKB
/// struct (`version` u32, `compact_target` u32, `timestamp` u64, ...) so the u64 millisecond
/// timestamp sits at byte offset 8. Read the first 16 bytes of HeaderDep 0; take `[8..16)`. The
/// short read is expected (the full header is ~208 bytes ⇒ `LengthNotEnough`, but the 16 bytes we
/// asked for are copied). A header shorter than 16 bytes, or no header-dep, ⇒ `None` (exit 33).
fn header_now() -> Option<u64> {
    let mut buf = [0u8; 16];
    match syscalls::load_header(&mut buf, 0, 0, Source::HeaderDep) {
        Ok(n) if n >= 16 => {}
        Err(SysError::LengthNotEnough(avail)) if avail >= 16 => {}
        _ => return None,
    }
    Some(u64::from_le_bytes(buf[8..16].try_into().unwrap()))
}

/// Load the full vote witness at `index` (GroupInput-aligned) into an owned buffer via the
/// partial-load probe, exactly like the proof-blob streaming in the intake script.
fn load_witness_full(index: usize) -> Result<alloc::vec::Vec<u8>, i8> {
    let mut probe = [0u8; 1];
    let total = match syscalls::load_witness(&mut probe, 0, index, Source::GroupInput) {
        Ok(n) => n,
        Err(SysError::LengthNotEnough(avail)) => avail,
        Err(_) => return Err(32),
    };
    let mut buf = alloc::vec::Vec::new();
    buf.resize(total, 0u8);
    if total > 0 {
        match syscalls::load_witness(&mut buf, 0, index, Source::GroupInput) {
            Ok(_) | Err(SysError::LengthNotEnough(_)) => {}
            Err(_) => return Err(32),
        }
    }
    Ok(buf)
}

/// Validate finalization cell `index`: parse the set+params, read its vote witness, recompute the
/// Q32.32 inequality against the header `now`. Returns the cell-level exit code (0 = finalizes).
fn validate_cell(index: usize, data: &[u8], now: u64) -> i8 {
    let (mix, params, validators) = match parse_finalization_cell(data) {
        Some(t) => t,
        None => return 31,
    };
    let votes_blob = match load_witness_full(index) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let idxs = match parse_votes(&votes_blob, validators.len()) {
        Some(v) => v,
        None => return 32,
    };
    let voters_for: alloc::vec::Vec<ValidatorQ> = idxs.iter().map(|&i| validators[i].clone()).collect();
    if finalizes_fixed(
        &voters_for,
        &validators,
        mix,
        now,
        params.horizon,
        params.decay_pos,
        params.threshold_bps,
        params.quorum_floor_bps,
    ) {
        0
    } else {
        30 // claim false: refuse to validate a finalization cell whose vote does not clear the bar
    }
}

pub fn program_entry() -> i8 {
    // Validator-set provenance (sentinel-gated inert pre-deploy).
    if !validator_set_bound() {
        return 34;
    }
    // Iterate the WHOLE script group (GroupInput, witness[i] ↔ input[i]) so no second finalization
    // cell can smuggle a false claim past a single-cell check — the lesson the intake script's
    // input-0-only tick taught. `now` is sourced ONCE from the header and governs every cell.
    let mut now: Option<u64> = None;
    let mut checked = 0usize;
    let mut index = 0usize;
    loop {
        match load_cell_data(index, Source::GroupInput) {
            Ok(data) => {
                let t = match now {
                    Some(t) => t,
                    None => match header_now() {
                        Some(t) => {
                            now = Some(t);
                            t
                        }
                        None => return 33, // no consensus time source ⇒ cannot evaluate decay
                    },
                };
                let code = validate_cell(index, &data, t);
                if code != 0 {
                    return code;
                }
                index += 1;
                checked += 1;
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(_) => return 31,
        }
    }
    if checked == 0 {
        return 31; // empty finalization group: nothing to attest
    }
    0
}
