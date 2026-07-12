//! Phase 2 (stateless verification) — a compact, audited commitment to the UTXO set.
//!
//! The live value-UTXO set is [`crate::runtime::Ledger::token_cells`] (a `Vec<Cell>`). This module
//! commits it into a single 32-byte **root** via an audited Sparse Merkle Tree (the vendored Nervos
//! `sparse-merkle-tree`, its C blake2b removed — see `onchain/vendor/sparse-merkle-tree`), with the
//! `Hasher` supplied by Noesis's own pure-Rust `blake2b-ref`. A node can then prove **"this coin is
//! unspent"** (membership) or **"this coin is spent / never existed"** (non-membership) with a ~KB
//! proof, without shipping the whole set — the *nearly-free verification* the stateless-node design
//! is built around.
//!
//! ## Trust boundary (stated honestly, per the engagement rule)
//! The root is a **compact, self-verifying description** of the set: a proof checks by independent
//! re-hashing, so **no third party is trusted** (an SMT is a *verification* primitive, not a *trust*
//! one — its job is to let everyone else not trust you). What it is **not**: a *validity* proof. A
//! checkpoint ([`UtxoCommitment::verify_snapshot`]) is **trusted-until-Phase-3**, where a zkVM
//! receipt will prove the committed set is the result of a *valid history*. And validity is still
//! neither **canonicality** (which fork) nor **data availability** (a root is not the data).
//!
//! ## Scope
//! Purely **additive**: it derives from `token_cells` and touches no consensus / `apply` path, so
//! the Phase-1 replay parity is unaffected. Folding the root into `state_digest` (consensus
//! agreement on the commitment) + incremental per-block maintenance is a later step, noted in
//! `docs/phase2-commitment-report.md`.

use crate::{Cell, Script};
use sparse_merkle_tree::{
    default_store::DefaultStore, traits::Hasher, CompiledMerkleProof, SparseMerkleTree, H256,
};

/// Domain-separated blake2b personalization for the UTXO SMT's internal node hashing — distinct
/// from the novelty SMT (`noesis-smt-v1`), the tx digest (`noesis-tx-v1`), and the Lamport lock
/// (`noesis-lamp-v1`), so a UTXO node hash can never collide with any of them. 16 bytes (blake2b's
/// personal length), null-padded.
const UTXO_NODE_PERSONAL: &[u8; 16] = b"noesis-utxo-v1\0\0";
/// Personalization for the leaf-key derivation (a UTXO's consensus identity -> its 32-byte key).
const UTXO_KEY_PERSONAL: &[u8; 16] = b"noesis-utxoid\0\0\0";

/// Pure-Rust [`Hasher`] for the vendored SMT, backed by `blake2b-ref` (the same hasher the rest of
/// Noesis uses). Being pure Rust is exactly what lets the whole commitment cross-compile to RISC-V
/// for the Phase-3 zkVM guest with **no C toolchain**.
pub struct Blake2bRefHasher(blake2b_ref::Blake2b);

impl Default for Blake2bRefHasher {
    fn default() -> Self {
        Blake2bRefHasher(blake2b_ref::Blake2bBuilder::new(32).personal(UTXO_NODE_PERSONAL).build())
    }
}

impl Hasher for Blake2bRefHasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }
    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b]);
    }
    fn finish(self) -> H256 {
        let Blake2bRefHasher(h) = self;
        let mut out = [0u8; 32];
        h.finalize(&mut out);
        out.into()
    }
}

type UtxoSmt = SparseMerkleTree<Blake2bRefHasher, H256, DefaultStore<H256>>;

/// Length-prefixed (injective) append: no field-boundary ambiguity — `args=[1],data=[2,3]` can
/// never serialize the same as `args=[1,2],data=[3]`.
fn put(h: &mut blake2b_ref::Blake2b, bytes: &[u8]) {
    h.update(&(bytes.len() as u32).to_le_bytes());
    h.update(bytes);
}

fn put_script(h: &mut blake2b_ref::Blake2b, s: &Script) {
    put(h, &s.code_hash);
    put(h, &s.args);
}

/// The consensus identity of a UTXO -> its SMT leaf key. Keys on `(id, lock, type_script, data)` —
/// the SAME tuple [`crate::runtime::TokenTx::is_valid_in_ledger`] resolves existence on;
/// `parent`/`timestamp` are deliberately excluded (the ledger treats two cells equal under this
/// tuple as the same live cell, so the commitment must not distinguish them either).
pub fn utxo_key(cell: &Cell) -> H256 {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(UTXO_KEY_PERSONAL).build();
    put(&mut h, &cell.id.to_le_bytes());
    put_script(&mut h, &cell.lock);
    put_script(&mut h, &cell.type_script);
    put(&mut h, &cell.data);
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out.into()
}

/// A compact commitment to a UTXO set. A present (unspent) cell maps `key -> key` (a non-zero leaf
/// content-bound to its identity); an absent/spent cell maps `key -> H256::zero()` (the SMT's empty
/// leaf), so membership and non-membership are mutually exclusive and both provable.
pub struct UtxoCommitment {
    smt: UtxoSmt,
}

impl UtxoCommitment {
    pub fn new() -> Self {
        UtxoCommitment { smt: UtxoSmt::default() }
    }

    /// Build a commitment over a set of live cells. Order-independent — the SMT root is a commitment
    /// to the SET, not the insertion order. This is the shadow/checkpoint constructor; incremental
    /// maintenance (insert/remove per block) is the deploy-time optimization.
    pub fn from_cells(cells: &[Cell]) -> Self {
        let leaves: Vec<(H256, H256)> = cells
            .iter()
            .map(|c| {
                let k = utxo_key(c);
                (k, k)
            })
            .collect();
        let mut smt = UtxoSmt::default();
        if !leaves.is_empty() {
            smt.update_all(leaves).expect("in-memory SMT update_all is infallible");
        }
        UtxoCommitment { smt }
    }

    /// Mark a cell UNSPENT (present).
    pub fn insert(&mut self, cell: &Cell) {
        let k = utxo_key(cell);
        self.smt.update(k, k).expect("in-memory SMT update is infallible");
    }

    /// Retire a cell on spend (present -> empty). Idempotent.
    pub fn remove(&mut self, cell: &Cell) {
        let k = utxo_key(cell);
        self.smt.update(k, H256::zero()).expect("in-memory SMT update is infallible");
    }

    /// The 32-byte commitment root.
    pub fn root(&self) -> [u8; 32] {
        (*self.smt.root()).into()
    }

    /// A compiled proof for `cell` — the ~KB witness a light node verifies against the root. The
    /// proof is polarity-agnostic: the verifier decides membership vs non-membership by the value it
    /// checks (see [`UtxoCommitment::verify`]).
    pub fn prove(&self, cell: &Cell) -> Vec<u8> {
        let key = utxo_key(cell);
        let proof = self.smt.merkle_proof(vec![key]).expect("merkle_proof");
        proof.compile(vec![key]).expect("compile proof").0
    }

    /// Verify a compiled proof against a root. `unspent = true` checks MEMBERSHIP (the coin is in
    /// the committed set); `unspent = false` checks NON-MEMBERSHIP (spent / never existed). Pure
    /// re-hashing against the caller's own hasher — no trust in whoever produced the proof.
    pub fn verify(root: &[u8; 32], cell: &Cell, proof: &[u8], unspent: bool) -> bool {
        let key = utxo_key(cell);
        let value = if unspent { key } else { H256::zero() };
        CompiledMerkleProof(proof.to_vec())
            .verify::<Blake2bRefHasher>(&(*root).into(), vec![(key, value)])
            .unwrap_or(false)
    }

    /// assumeutxo-style checkpoint check: does this snapshot of cells reproduce `root`? A new node
    /// bootstraps from {snapshot, committed root}; if they match it can start operating on the
    /// snapshot — TRUSTED until Phase 3 proves the snapshot is the result of a valid history.
    pub fn verify_snapshot(cells: &[Cell], root: &[u8; 32]) -> bool {
        &Self::from_cells(cells).root() == root
    }
}

impl Default for UtxoCommitment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: u64, owner: &[u8], data: &[u8]) -> Cell {
        Cell {
            id,
            lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
            type_script: Script { code_hash: [1u8; 32], args: b"tok".to_vec() },
            parent: None,
            timestamp: 0,
            data: data.to_vec(),
        }
    }

    fn sample() -> Vec<Cell> {
        vec![
            cell(1, b"alice", b"100"),
            cell(2, b"bob", b"250"),
            cell(3, b"carol", b"7"),
        ]
    }

    #[test]
    fn root_is_a_set_commitment_order_independent() {
        let mut fwd = sample();
        let mut rev = fwd.clone();
        rev.reverse();
        assert_eq!(
            UtxoCommitment::from_cells(&fwd).root(),
            UtxoCommitment::from_cells(&rev).root(),
            "root must depend on the SET, not insertion order"
        );
        // a different set => a different root (anti-theater).
        fwd.push(cell(4, b"dave", b"1"));
        assert_ne!(
            UtxoCommitment::from_cells(&fwd).root(),
            UtxoCommitment::from_cells(&sample()).root(),
        );
        // empty set has a fixed (all-zero) root distinct from any non-empty set.
        assert_ne!(UtxoCommitment::new().root(), UtxoCommitment::from_cells(&sample()).root());
    }

    #[test]
    fn unspent_cell_has_a_verifiable_membership_proof() {
        let set = sample();
        let c = UtxoCommitment::from_cells(&set);
        let root = c.root();
        for cl in &set {
            let proof = c.prove(cl);
            assert!(UtxoCommitment::verify(&root, cl, &proof, true), "unspent coin must prove membership");
            // the SAME proof must NOT verify as non-membership — polarity is bound to the value.
            assert!(!UtxoCommitment::verify(&root, cl, &proof, false));
        }
    }

    #[test]
    fn absent_cell_has_a_verifiable_non_membership_proof() {
        let set = sample();
        let c = UtxoCommitment::from_cells(&set);
        let root = c.root();
        let ghost = cell(999, b"mallory", b"0"); // never inserted
        let proof = c.prove(&ghost);
        assert!(UtxoCommitment::verify(&root, &ghost, &proof, false), "absent coin must prove NON-membership");
        assert!(!UtxoCommitment::verify(&root, &ghost, &proof, true), "absent coin must NOT prove membership");
    }

    #[test]
    fn spending_a_cell_retires_it_from_the_commitment() {
        let set = sample();
        let mut c = UtxoCommitment::new();
        for cl in &set {
            c.insert(cl);
        }
        let root_before = c.root();
        assert_eq!(root_before, UtxoCommitment::from_cells(&set).root(), "incremental == batch");

        // spend bob (id 2).
        let bob = &set[1];
        c.remove(bob);
        let root_after = c.root();
        assert_ne!(root_before, root_after, "spending must move the root");

        // bob now proves NON-membership; alice/carol still prove membership.
        let bob_proof = c.prove(bob);
        assert!(UtxoCommitment::verify(&root_after, bob, &bob_proof, false), "spent coin => non-membership");
        assert!(!UtxoCommitment::verify(&root_after, bob, &bob_proof, true));
        for cl in [&set[0], &set[2]] {
            let p = c.prove(cl);
            assert!(UtxoCommitment::verify(&root_after, cl, &p, true), "unspent coin still a member");
        }
    }

    #[test]
    fn snapshot_checkpoint_verifies_and_rejects_tampering() {
        let set = sample();
        let root = UtxoCommitment::from_cells(&set).root();
        assert!(UtxoCommitment::verify_snapshot(&set, &root), "honest snapshot must verify");

        // drop a coin -> different set -> must fail against the committed root.
        let missing = &set[..2];
        assert!(!UtxoCommitment::verify_snapshot(missing, &root), "a snapshot missing a coin must be rejected");

        // add a coin -> must fail.
        let mut extra = set.clone();
        extra.push(cell(4, b"dave", b"1"));
        assert!(!UtxoCommitment::verify_snapshot(&extra, &root), "a snapshot with an extra coin must be rejected");

        // flip one byte of the root -> must fail (anti-theater).
        let mut bad_root = root;
        bad_root[0] ^= 0x01;
        assert!(!UtxoCommitment::verify_snapshot(&set, &bad_root));
    }

    #[test]
    fn membership_proof_is_kilobyte_scale() {
        // a realistic-ish set; the proof is O(log n) hashes, so it stays small regardless of |set|.
        let big: Vec<Cell> = (0..256u64).map(|i| cell(i, b"x", &i.to_le_bytes())).collect();
        let c = UtxoCommitment::from_cells(&big);
        let root = c.root();
        let proof = c.prove(&big[123]);
        assert!(UtxoCommitment::verify(&root, &big[123], &proof, true));
        // hard ceiling: a compiled SMT proof for a 256-leaf set is well under 4 KB.
        assert!(proof.len() < 4096, "proof unexpectedly large: {} bytes", proof.len());
    }

    #[test]
    fn a_tampered_proof_does_not_verify() {
        let set = sample();
        let c = UtxoCommitment::from_cells(&set);
        let root = c.root();
        let mut proof = c.prove(&set[0]);
        assert!(UtxoCommitment::verify(&root, &set[0], &proof, true));
        if let Some(b) = proof.last_mut() {
            *b ^= 0xFF; // corrupt the proof
        }
        assert!(!UtxoCommitment::verify(&root, &set[0], &proof, true), "a corrupted proof must not verify");
    }

    /// Phase-2 measurement (NOT a correctness gate). Reproduce with:
    ///   cargo test -p noesis --lib utxo_commitment::tests::report_metrics -- --ignored --nocapture
    /// Numbers land in `docs/phase2-commitment-report.md`; re-run there before quoting.
    #[test]
    #[ignore = "measurement, not a correctness gate — run with --ignored --nocapture"]
    fn report_metrics() {
        use std::time::Instant;
        for &n in &[100usize, 1_000, 10_000] {
            let cells: Vec<Cell> = (0..n as u64).map(|i| cell(i, b"acct", &i.to_le_bytes())).collect();
            let t0 = Instant::now();
            let c = UtxoCommitment::from_cells(&cells);
            let build = t0.elapsed();
            let root = c.root();
            let mid = &cells[n / 2];
            let t1 = Instant::now();
            let proof = c.prove(mid);
            let prove = t1.elapsed();
            let ok = UtxoCommitment::verify(&root, mid, &proof, true);
            // cost of one incremental spend/insert on top of an n-set (the per-block delta):
            let mut inc = UtxoCommitment::from_cells(&cells);
            let t2 = Instant::now();
            inc.insert(&cell(n as u64, b"acct", b"new"));
            let one_insert = t2.elapsed();
            println!(
                "n={:>6}  build_full_root={:>10.2?}  one_insert={:>9.2?}  proof_bytes={:>4}  prove={:>9.2?}  verify_ok={}",
                n, build, one_insert, proof.len(), prove, ok
            );
        }
    }
}
