//! zk-finalize guest — proves one execution of `finalizes_pos_pom_fixed`.
//!
//! Reads the finalization cell + vote witness + clock from the host, decodes them with the
//! core's own wire format, runs the REAL finalize function, and commits (input digest, verdict)
//! to the journal. The proof therefore attests: "on inputs whose sha256 is D, the canonical
//! PoS+PoM anti-concentration finalize rule returns V" — no trust in the prover required.

use risc0_zkvm::guest::env;
use risc0_zkvm::sha::{Impl, Sha256};

use noesis_core::finalization::{
    finalizes_pos_pom_fixed, parse_finalization_cell, parse_votes, ValidatorQ,
};

risc0_zkvm::guest::entry!(main);

fn main() {
    // Host inputs. `now` is the finalize-time clock (not carried in the cell).
    let cell: Vec<u8> = env::read();
    let votes: Vec<u8> = env::read();
    let now: u64 = env::read();

    // Bind the journal to the inputs: sha256(cell || votes || now_le). A verifier that trusts
    // the digest trusts the verdict is for exactly these inputs.
    let mut buf = cell.clone();
    buf.extend_from_slice(&votes);
    buf.extend_from_slice(&now.to_le_bytes());
    let digest = *Impl::hash_bytes(&buf);

    // Decode with the core's own parsers. A malformed cell/vote set is a hard failure — the
    // guest must not silently "finalize" garbage (matches parse_* returning None on bad input).
    let (_mix, params, validators) =
        parse_finalization_cell(&cell).expect("malformed finalization cell");
    let idxs = parse_votes(&votes, validators.len()).expect("malformed vote witness");
    let voters_for: Vec<ValidatorQ> =
        idxs.iter().map(|&i| validators[i].clone()).collect();

    let verdict = finalizes_pos_pom_fixed(
        &voters_for,
        &validators,
        now,
        params.horizon,
        params.decay_pos,
        params.threshold_bps,
    );

    env::commit(&(digest, verdict));
}
