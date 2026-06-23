//! The Noesis lock script (onchain/locksig-typescript, no_std, riscv64imac) validated END TO END
//! under the host harness — DESIGN-onvm-locksig-program.md ((rr)): existence→CONTROL enforced INSIDE
//! the VM. The script reconstructs the value-movement from the served cells, recomputes the canonical
//! `tx_digest` in the SAME single-sourced serializer the node signs over (`noesis_core::tx`), and
//! verifies each input's post-quantum Lamport signature against the cell's `lock.args` root.
//!
//! Fixture: tests/fixtures/locksig-typescript — rebuild with
//!   cd onchain/locksig-typescript && cargo build --release --target riscv64imac-unknown-none-elf
//!   cp target/riscv64imac-unknown-none-elf/release/locksig-typescript ../../node/tests/fixtures/
//!
//! Exit-code contract (src/main.rs): 0 every input authorized · 41 malformed/short cell record or
//! empty input group · 42 a signature fails verification · 43 a lock.args not a 32-byte root · 44
//! unknown token-standard code_hash · 45 group spans more than one type-script.

mod common;

use common::run_typescript_t7;
use noesis::{Cell, Script};
use noesis_core::lamport;
use noesis_core::tx::{encode_cell_identity, parse_cell_identity, tx_digest, CellView};

const ELF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/locksig-typescript");
/// The Fungible standard's type-script code (matches the ELF's `standard_of` map; tag byte 0).
const FUNGIBLE: [u8; 32] = [0xF1; 32];

/// A logical token cell: `lock.args` carries the 32-byte Lamport public-key root (the owner),
/// `type_script` carries the token's `(code_hash = standard, args = issuer)`, `data` the amount.
fn cell(id: u64, lock_root: [u8; 32], code_hash: [u8; 32], issuer: &[u8], data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [2u8; 32], args: lock_root.to_vec() },
        type_script: Script { code_hash, args: issuer.to_vec() },
        parent: None,
        timestamp: 0,
        data: data.to_vec(),
    }
}

fn view(c: &Cell) -> CellView<'_> {
    CellView {
        id: c.id,
        lock_code_hash: &c.lock.code_hash,
        lock_args: &c.lock.args,
        type_code_hash: &c.type_script.code_hash,
        type_args: &c.type_script.args,
        data: &c.data,
    }
}

/// Wrap a logical cell into the cell the harness SERVES: its `data` is the encoded identity record
/// the ELF parses back (the pre-deploy model wire; at deploy these come from CKB cell-field syscalls).
fn served(c: &Cell) -> Cell {
    Cell {
        id: c.id,
        lock: c.lock.clone(),
        type_script: c.type_script.clone(),
        parent: None,
        timestamp: 0,
        data: encode_cell_identity(&view(c)),
    }
}

/// The host (wallet-side) digest over the logical movement — the single source the owner signs and
/// the ELF must reproduce on-VM byte-for-byte.
fn digest(standard: u8, code_hash: &[u8; 32], args: &[u8], inputs: &[Cell], outputs: &[Cell]) -> [u8; 32] {
    let iv: Vec<CellView> = inputs.iter().map(view).collect();
    let ov: Vec<CellView> = outputs.iter().map(view).collect();
    tx_digest(standard, code_hash, args, &iv, &ov)
}

/// A deterministic Lamport keypair from a one-byte seed: (seed, public-key root).
fn keypair(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
    let seed = [seed_byte; 32];
    (seed, lamport::keygen_root(&seed))
}

/// A valid one-input→one-output move owned by `seed`, the input signed over the canonical digest.
fn signed_move(seed_byte: u8) -> (Vec<Cell>, Vec<Cell>, Vec<Vec<u8>>) {
    let (seed, root) = keypair(seed_byte);
    let issuer = vec![9u8];
    let input = cell(1, root, FUNGIBLE, &issuer, &[5]);
    let output = cell(2, root, FUNGIBLE, &issuer, &[5]);
    let d = digest(0, &FUNGIBLE, &issuer, std::slice::from_ref(&input), std::slice::from_ref(&output));
    let sig = lamport::sign(&seed, &d);
    (vec![served(&input)], vec![served(&output)], vec![sig])
}

/// The DIGEST-PARITY check at the serializer level: a cell's identity round-trips through
/// encode/parse, and the digest over the PARSED reconstruction equals the digest over the original.
/// This is what makes the on-VM reconstruction faithful; the end-to-end Lamport proof (below) is the
/// other half — a host-signed sig only verifies on-VM if the two digests are byte-equal.
#[test]
fn cell_identity_round_trips_and_preserves_the_digest() {
    let c = cell(42, [3u8; 32], FUNGIBLE, &[1, 2, 3], &[9, 9, 9]);
    let blob = encode_cell_identity(&view(&c));
    let parsed = parse_cell_identity(&blob).expect("a clean record parses");
    assert_eq!(parsed.id, 42);
    assert_eq!(parsed.lock_args, vec![3u8; 32]);
    assert_eq!(parsed.type_args, vec![1u8, 2, 3]);
    assert_eq!(parsed.data, vec![9u8, 9, 9]);
    let d_original = tx_digest(0, &FUNGIBLE, &[1, 2, 3], &[view(&c)], &[]);
    let d_parsed = tx_digest(0, &FUNGIBLE, &[1, 2, 3], &[parsed.view()], &[]);
    assert_eq!(d_original, d_parsed, "digest is invariant under encode→parse — reconstruction is faithful");
    // Trailing bytes / short reads are rejected (no silent partial parse).
    let mut trailing = blob.clone();
    trailing.push(0);
    assert!(parse_cell_identity(&trailing).is_none(), "trailing bytes ⇒ not a clean record");
    assert!(parse_cell_identity(&blob[..blob.len() - 1]).is_none(), "short read ⇒ rejected");
}

#[test]
fn valid_owner_signature_authorizes_the_spend_on_vm() {
    let (inputs, outputs, wits) = signed_move(7);
    let (res, served_n) = run_typescript_t7(ELF, &inputs[0], inputs.clone(), outputs, vec![], wits);
    assert!(served_n >= 1, "script must consume at least one cell-data load (got {served_n})");
    assert_eq!(res.unwrap(), 0, "a valid owner Lamport sig authorizes the spend on-VM");
}

/// THE adversarial fixture: a signature under a DIFFERENT key over the SAME digest cannot move the
/// cell. Existence ≠ control — a real cell can be named by anyone, only its key-holder can spend it.
/// This is also the on-VM analog of the node's (nn) wrong-key test, and the anti-theater anchor: a
/// stubbed `verify→true` ELF would let this through.
#[test]
fn a_wrong_key_signature_over_the_same_digest_is_refused_on_vm() {
    let (_seed_a, root_a) = keypair(7); // the cell owner
    let (seed_b, _root_b) = keypair(8); // the attacker — NOT the owner
    let issuer = vec![9u8];
    let input = cell(1, root_a, FUNGIBLE, &issuer, &[5]);
    let output = cell(2, root_a, FUNGIBLE, &issuer, &[5]);
    let d = digest(0, &FUNGIBLE, &issuer, std::slice::from_ref(&input), std::slice::from_ref(&output));
    let forged = lamport::sign(&seed_b, &d); // correct message, wrong key
    let (res, _) = run_typescript_t7(
        ELF,
        &served(&input),
        vec![served(&input)],
        vec![served(&output)],
        vec![],
        vec![forged],
    );
    assert_eq!(res.unwrap(), 42, "a sig under a different key cannot move the owner's cell");
}

#[test]
fn a_tampered_signature_is_refused_on_vm() {
    let (inputs, outputs, mut wits) = signed_move(7);
    wits[0][0] ^= 0xFF; // flip a byte in the otherwise-valid signature
    let (res, _) = run_typescript_t7(ELF, &inputs[0], inputs.clone(), outputs, vec![], wits);
    assert_eq!(res.unwrap(), 42, "a tampered signature does not verify");
}

#[test]
fn empty_auth_is_inert_pre_deploy() {
    // No witness at all ⇒ empty auth ⇒ inert pass (CONTROL_ENFORCED = false): honest empty-auth
    // flows are unchanged, exactly like the node's CONTROL_BINDING_ACTIVE gate.
    let (seed, root) = keypair(7);
    let _ = seed;
    let issuer = vec![9u8];
    let input = cell(1, root, FUNGIBLE, &issuer, &[5]);
    let output = cell(2, root, FUNGIBLE, &issuer, &[5]);
    let (res, _) = run_typescript_t7(
        ELF,
        &served(&input),
        vec![served(&input)],
        vec![served(&output)],
        vec![],
        vec![], // no auths
    );
    assert_eq!(res.unwrap(), 0, "pre-deploy an empty auth authorizes — honest flows unchanged");
}

#[test]
fn a_non_32_byte_lock_args_cannot_authorize() {
    let issuer = vec![9u8];
    // lock.args is 31 bytes — not a Lamport root. A PRESENTED auth forces the 32-byte check.
    let mut input = cell(1, [0u8; 32], FUNGIBLE, &issuer, &[5]);
    input.lock.args = vec![0u8; 31];
    let output = cell(2, [0u8; 32], FUNGIBLE, &issuer, &[5]);
    let (res, _) = run_typescript_t7(
        ELF,
        &served(&input),
        vec![served(&input)],
        vec![served(&output)],
        vec![],
        vec![vec![1u8; 64]], // any presented auth
    );
    assert_eq!(res.unwrap(), 43, "a lock.args that is not a 32-byte root cannot authorize");
}

#[test]
fn an_unknown_token_standard_is_rejected() {
    let issuer = vec![9u8];
    let unknown = [0xAB; 32]; // not in the standard_of map
    let input = cell(1, [0u8; 32], unknown, &issuer, &[5]);
    let output = cell(2, [0u8; 32], unknown, &issuer, &[5]);
    let (res, _) = run_typescript_t7(
        ELF,
        &served(&input),
        vec![served(&input)],
        vec![served(&output)],
        vec![],
        vec![],
    );
    assert_eq!(res.unwrap(), 44, "an unknown token-standard code_hash is rejected (standard not attacker-chosen)");
}

#[test]
fn a_group_spanning_two_type_scripts_is_rejected() {
    let issuer = vec![9u8];
    let nft: [u8; 32] = [0x71; 32];
    let a = cell(1, [0u8; 32], FUNGIBLE, &issuer, &[5]);
    let b = cell(2, [0u8; 32], nft, &issuer, &[5]); // different type-script
    let (res, _) = run_typescript_t7(
        ELF,
        &served(&a),
        vec![served(&a), served(&b)],
        vec![],
        vec![],
        vec![vec![], vec![]],
    );
    assert_eq!(res.unwrap(), 45, "a mixed-type group is out of scope and rejected, not silently digested");
}

#[test]
fn a_malformed_cell_record_is_rejected() {
    // A served cell whose data is not a valid identity record.
    let bad = Cell {
        id: 1,
        lock: Script { code_hash: [2u8; 32], args: vec![] },
        type_script: Script { code_hash: FUNGIBLE, args: vec![] },
        parent: None,
        timestamp: 0,
        data: vec![1, 2, 3], // far too short to be a record
    };
    let (res, _) = run_typescript_t7(ELF, &bad, vec![bad.clone()], vec![], vec![], vec![vec![]]);
    assert_eq!(res.unwrap(), 41, "a malformed cell record is rejected");
}

#[test]
fn an_empty_input_group_is_rejected() {
    let (res, _) = run_typescript_t7(ELF, &served(&cell(1, [0u8; 32], FUNGIBLE, &[9], &[5])), vec![], vec![], vec![], vec![]);
    assert_eq!(res.unwrap(), 41, "an empty input group authorizes nothing");
}

/// Build a two-owner movement (inputs owned by keys A and B over one shared digest) + the per-input
/// witnesses. `corrupt_second` signs input 1 with A's key instead of B's (the smuggle attempt).
fn two_owner_move(corrupt_second: bool) -> (Vec<Cell>, Vec<Cell>, Vec<Vec<u8>>) {
    let (seed_a, root_a) = keypair(7);
    let (seed_b, root_b) = keypair(8);
    let issuer = vec![9u8];
    let in_a = cell(1, root_a, FUNGIBLE, &issuer, &[5]);
    let in_b = cell(2, root_b, FUNGIBLE, &issuer, &[3]);
    let out = cell(3, root_a, FUNGIBLE, &issuer, &[8]);
    let inputs = vec![in_a.clone(), in_b.clone()];
    let d = digest(0, &FUNGIBLE, &issuer, &inputs, std::slice::from_ref(&out));
    let sig_a = lamport::sign(&seed_a, &d);
    // input 1 is owned by B; the smuggle signs it with A's key (wrong owner for that cell).
    let sig_1 = if corrupt_second { lamport::sign(&seed_a, &d) } else { lamport::sign(&seed_b, &d) };
    (
        vec![served(&in_a), served(&in_b)],
        vec![served(&out)],
        vec![sig_a, sig_1],
    )
}

/// Every consumed input is authorized independently: two cells owned by different keys, each correctly
/// signed over the shared digest, both clear.
#[test]
fn each_input_is_authorized_independently() {
    let (inputs, outputs, wits) = two_owner_move(false);
    let (res, _) = run_typescript_t7(ELF, &inputs[0], inputs.clone(), outputs, vec![], wits);
    assert_eq!(res.unwrap(), 0, "two distinct owners each signing their own input both authorize");
}

/// The per-input gate checks EVERY input, not just index 0: a valid input-0 signature cannot smuggle a
/// wrong-key input-1 past the loop (the on-VM analog of the finalization "second cell can't smuggle").
#[test]
fn a_wrong_signature_on_a_later_input_cannot_smuggle_past_the_first() {
    let (inputs, outputs, wits) = two_owner_move(true);
    let (res, _) = run_typescript_t7(ELF, &inputs[0], inputs.clone(), outputs, vec![], wits);
    assert_eq!(res.unwrap(), 42, "input 1 signed by the wrong owner is caught — the loop gates every input");
}
