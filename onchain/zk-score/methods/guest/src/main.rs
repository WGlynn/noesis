//! zk-score guest — proves one execution of the canonical `zk_score_eval` over PRIVATE content.
//!
//! Reads the contribution content + its membership proofs as PRIVATE witnesses, the corpus root +
//! thresholds as PUBLIC inputs, runs the REAL core verdict (`noesis_core::zk_score_eval` — the same
//! function the node scores with), and commits ONLY the 4-tuple
//!   (public_digest, nullifier, accepted, value>=V_FLOOR)
//! to the journal. The content and proofs are read but NEVER committed: that omission is the privacy.
//!
//! Two soundness properties the caller/verifier must honour (they cannot live in the proof alone):
//!   * `root` is UNTRUSTED here. The guest cannot know the canonical corpus root, so it binds `root`
//!     into `public_digest`; the VERIFIER must reject any receipt whose digest is not
//!     `zk_score_public_digest(canonical_root, thetas)`. That comparison pins the novelty judgement
//!     to the real corpus (defeats the empty/attacker-root standing forgery).
//!   * The bar `V_FLOOR` is BAKED into this image (`ZK_SCORE_V_FLOOR`), not read as an input, so a
//!     prover cannot pick `v_min=0` to mint standing for a zero score, nor binary-search the exact
//!     score by re-proving at different bounds.
//! The `nullifier` binds the content (not the identity — that is Fit 4); the value ledger rejects a
//! repeated nullifier to stop replay / double-claim of one contribution.

use risc0_zkvm::guest::env;

use noesis_core::{
    unflatten_proofs, unique_shingles, zk_score_eval, zk_score_public_digest, Hash, ZK_SCORE_V_FLOOR,
};

risc0_zkvm::guest::entry!(main);

fn main() {
    // PRIVATE witnesses — read, used, never committed.
    let data: Vec<u8> = env::read();
    let proofs_flat: Vec<u8> = env::read();
    // PUBLIC inputs — bound into the journal digest. (V_FLOOR is baked into the image, not read.)
    let root: Hash = env::read();
    let theta_sim_q16: u64 = env::read();
    let theta_ent_q16: u64 = env::read();

    // Decode the proof wire (single-sourced) and run the canonical verdict. A malformed witness,
    // an empty cell, or a forged path all collapse to None => rejected (accepted = false).
    let n = unique_shingles(&data).len();
    let verdict = unflatten_proofs(&proofs_flat, n)
        .and_then(|proofs| zk_score_eval(&data, root, &proofs, theta_sim_q16, theta_ent_q16));

    let (nullifier, accepted, value_ge_floor) = match verdict {
        Some((nf, value)) => {
            let ge = value >= ZK_SCORE_V_FLOOR;
            // Commit the nullifier LIVE only when the cell clears the floor (is standing-eligible), so
            // a below-floor or forged-root receipt cannot pre-register / burn another author's
            // nullifier. The verifier consumes it only AFTER the digest-pin passes (see README contract).
            (if ge { nf } else { [0u8; 32] }, true, ge)
        }
        None => ([0u8; 32], false, false),
    };

    // Bind the journal to the PUBLIC inputs ONLY (never the private witness). The verifier
    // authenticates `root` by recomputing this digest from the canonical corpus root.
    let public_digest = zk_score_public_digest(root, theta_sim_q16, theta_ent_q16);

    env::commit(&(public_digest, nullifier, accepted, value_ge_floor));
}
