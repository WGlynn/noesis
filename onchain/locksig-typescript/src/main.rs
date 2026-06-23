//! Noesis lock script — existence→CONTROL enforced ON-VM (RISC-V / CKB-VM).
//! DESIGN-onvm-locksig-program.md ((rr)): the on-VM twin of the node's `spend_is_authorized` ((nn)).
//! It reconstructs the value-movement INSIDE the VM, recomputes the canonical `tx_digest` in the
//! SAME single-sourced serializer the node signs over (`noesis_core::tx`), and refuses to authorize
//! a spend unless each consumed input presents a valid post-quantum Lamport signature over that
//! digest under the input's own `lock.args` public-key root. A real cell can be NAMED by anyone;
//! only the holder of its one-time key can MOVE it.
//!
//! THE adversarial point (the digest's `standard` byte and every cell field are CONSENSUS-sourced,
//! never attacker-chosen, [P·dont-let-attacker-choose-critical-input]):
//!   - cell identities (id, lock, type, data) come from the served cell set, not a free witness;
//!   - the token `(code_hash, args)` is the single type-script governing the group (asserted unified);
//!   - `standard` is DERIVED from `code_hash` via a fixed map — an unknown code is rejected, so the
//!     digest's standard tag can never be picked by the spender;
//!   - `auth` is `witness[input_index]` and is NOT part of the digest (no circularity).
//!
//! Exit codes (40s namespace; per-binary, so the reuse vs commit-order's 40s is fine — triage is
//! per script): 0 every input authorized · 41 malformed/short cell record or empty input group ·
//! 42 a signature fails verification · 43 a `lock.args` that is not a 32-byte root · 44 unknown
//! token-standard code_hash · 45 group spans more than one type-script (out of scope by construction).
//!
//! Honest scope (the deploy-coupled boundary, same as the finalization registry binding and the
//! commit-order coord binding): pre-deploy each cell's identity is parsed from the served model
//! record (`noesis_core::tx::parse_cell_identity`); at deploy it is sourced from real CKB cell-field
//! syscalls (`CELL_FIELDS_BOUND`). And `CONTROL_ENFORCED` is INERT pre-deploy: an EMPTY auth still
//! authorizes (honest empty-auth flows unchanged, consistent with the node's `CONTROL_BINDING_ACTIVE`);
//! a PRESENTED auth is verified FOR REAL today. At deploy the flag flips and every spend must sign.

#![no_std]
#![no_main]

// NOTE: no `extern crate alloc` — ckb_std::entry!/default_alloc! already declares it
// ([F·ckb-cell-build-recipe]); `alloc::vec::Vec` resolves through that declaration.

use ckb_std::{
    ckb_constants::Source,
    default_alloc,
    error::SysError,
    high_level::load_cell_data,
    syscalls,
};
use core::convert::TryFrom;
use noesis_core::lamport;
use noesis_core::tx::{parse_cell_identity, tx_digest, CellView, OwnedCellView};

ckb_std::entry!(program_entry);
default_alloc!();

/// Control-binding deploy flag — never an overloaded sentinel (the QA-port-2 lesson; mirrors the
/// node's `CONTROL_BINDING_ACTIVE`). Pre-deploy an EMPTY `auth` authorizes (inert, honest flows
/// unchanged); a PRESENTED `auth` is verified for real. At deploy the flag flips: an empty auth no
/// longer authorizes and every spend must carry a signature.
const CONTROL_ENFORCED: bool = false;

/// Map the token type-script `code_hash` → its `TokenStandard` tag byte (Fungible 0 / Nft 1 /
/// Multi 2, matching the node's `standard as u8`). DERIVED from consensus state, never a free field:
/// an unknown code is rejected so `standard` can never be attacker-chosen. The three constants are
/// the per-standard type-script codes; pre-deploy the reference + tests use these sentinels.
fn standard_of(code_hash: &[u8; 32]) -> Option<u8> {
    const FUNGIBLE: [u8; 32] = [0xF1; 32];
    const NFT: [u8; 32] = [0x71; 32];
    const MULTI: [u8; 32] = [0x11; 32];
    if code_hash == &FUNGIBLE {
        Some(0)
    } else if code_hash == &NFT {
        Some(1)
    } else if code_hash == &MULTI {
        Some(2)
    } else {
        None
    }
}

/// Reconstruct every cell on one side of the tx by iterating `load_cell_data(i, source)` until
/// `IndexOutOfBound`, parsing each served record into an [`OwnedCellView`]. A malformed/short record
/// or any other syscall error ⇒ Err(41).
fn load_cells(source: Source) -> Result<alloc::vec::Vec<OwnedCellView>, i8> {
    let mut out = alloc::vec::Vec::new();
    let mut i = 0usize;
    loop {
        match load_cell_data(i, source) {
            Ok(d) => match parse_cell_identity(&d) {
                Some(c) => {
                    out.push(c);
                    i += 1;
                }
                None => return Err(41),
            },
            Err(SysError::IndexOutOfBound) => break,
            Err(_) => return Err(41),
        }
    }
    Ok(out)
}

/// The one token type-script governing the whole movement: every input AND output must share one
/// `(type_code_hash, type_args)`. A Noesis token tx is single-type by construction; a mixed group is
/// out of scope and rejected (None ⇒ exit 45), never silently digested under the first cell's type.
fn unify_type<'a>(
    inputs: &'a [OwnedCellView],
    outputs: &'a [OwnedCellView],
) -> Option<(&'a [u8; 32], &'a [u8])> {
    let first = inputs.first()?;
    let ch = &first.type_code_hash;
    let ar = first.type_args.as_slice();
    for c in inputs.iter().chain(outputs.iter()) {
        if &c.type_code_hash != ch || c.type_args.as_slice() != ar {
            return None;
        }
    }
    Some((ch, ar))
}

/// Load `witness[index]` (input-aligned) into an owned buffer via the partial-load probe, exactly
/// like the finalization script's vote-witness streaming. An ABSENT witness (no auth for this input)
/// or a zero-length one ⇒ the empty auth (the pre-deploy inert path), NOT an error.
fn load_auth(index: usize) -> Result<alloc::vec::Vec<u8>, i8> {
    let mut probe = [0u8; 1];
    let total = match syscalls::load_witness(&mut probe, 0, index, Source::Input) {
        Ok(n) => n,
        Err(SysError::LengthNotEnough(avail)) => avail,
        Err(SysError::IndexOutOfBound) => return Ok(alloc::vec::Vec::new()),
        Err(_) => return Err(41),
    };
    if total == 0 {
        return Ok(alloc::vec::Vec::new());
    }
    let mut buf = alloc::vec::Vec::new();
    buf.resize(total, 0u8);
    match syscalls::load_witness(&mut buf, 0, index, Source::Input) {
        Ok(_) | Err(SysError::LengthNotEnough(_)) => Ok(buf),
        Err(_) => Err(41),
    }
}

pub fn program_entry() -> i8 {
    // Reconstruct the movement from consensus state (no field is a free witness).
    let inputs = match load_cells(Source::Input) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if inputs.is_empty() {
        return 41; // empty input group authorizes nothing
    }
    let outputs = match load_cells(Source::Output) {
        Ok(v) => v,
        Err(code) => return code,
    };

    // The token identity + standard, both consensus-derived.
    let (code_hash, args) = match unify_type(&inputs, &outputs) {
        Some(t) => t,
        None => return 45,
    };
    let standard = match standard_of(code_hash) {
        Some(s) => s,
        None => return 44,
    };

    // The canonical digest — the SAME bytes the node signs/verifies over (single source).
    let in_views: alloc::vec::Vec<CellView> = inputs.iter().map(|c| c.view()).collect();
    let out_views: alloc::vec::Vec<CellView> = outputs.iter().map(|c| c.view()).collect();
    let digest = tx_digest(standard, code_hash, args, &in_views, &out_views);

    // CONTROL: every consumed input must be authorized by its owner over this digest.
    for (i, input) in inputs.iter().enumerate() {
        let auth = match load_auth(i) {
            Ok(a) => a,
            Err(code) => return code,
        };
        if auth.is_empty() {
            if CONTROL_ENFORCED {
                return 42; // deploy: an empty auth no longer authorizes a spend
            }
            continue; // pre-deploy inert: honest empty-auth flows unchanged
        }
        let root = match <&[u8; 32]>::try_from(input.lock_args.as_slice()) {
            Ok(r) => r,
            Err(_) => return 43, // a lock.args that is not a 32-byte root cannot authorize
        };
        if !lamport::verify(root, &digest, &auth) {
            return 42; // existence ≠ control: a wrong-key / wrong-message / garbage auth is refused
        }
    }
    0
}
