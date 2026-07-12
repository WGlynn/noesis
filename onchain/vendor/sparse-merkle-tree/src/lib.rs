//! Constructs a new `SparseMerkleTree<H, V, S>`.
//!
//! # Examples
//!
//! ```ignore
//! // (original upstream example — used the removed C `blake2b` module; kept for reference only)
//! use sparse_merkle_tree::{
//!     blake2b::Blake2bHasher, default_store::DefaultStore,
//!     error::Error, MerkleProof,
//!     SparseMerkleTree, traits::Value, H256
//! };
//! use blake2b_rs::{Blake2b, Blake2bBuilder};
//!
//! // define SMT
//! type SMT = SparseMerkleTree<Blake2bHasher, Word, DefaultStore<Word>>;
//!
//! // define SMT value
//! #[derive(Default, Clone)]
//! pub struct Word(String);
//! impl Value for Word {
//!    fn to_h256(&self) -> H256 {
//!        if self.0.is_empty() {
//!            return H256::zero();
//!        }
//!        let mut buf = [0u8; 32];
//!        let mut hasher = new_blake2b();
//!        hasher.update(self.0.as_bytes());
//!        hasher.finalize(&mut buf);
//!        buf.into()
//!    }
//!    fn zero() -> Self {
//!        Default::default()
//!    }
//! }
//!
//! // helper function
//! fn new_blake2b() -> Blake2b {
//!     Blake2bBuilder::new(32).personal(b"SMT").build()
//! }
//!
//! fn construct_smt() {
//!     let mut tree = SMT::default();
//!     for (i, word) in "The quick brown fox jumps over the lazy dog"
//!         .split_whitespace()
//!         .enumerate()
//!     {
//!         let key: H256 = {
//!             let mut buf = [0u8; 32];
//!             let mut hasher = new_blake2b();
//!             hasher.update(&(i as u32).to_le_bytes());
//!             hasher.finalize(&mut buf);
//!             buf.into()
//!         };
//!         let value = Word(word.to_string());
//!         // insert key value into tree
//!         tree.update(key, value).expect("update");
//!     }
//!
//!     println!("SMT root is {:?} ", tree.root());
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
// The upstream crate gates a `trie` variant behind `#[cfg(feature = "trie")]`; the Noesis vendor
// removed that feature (and the `trie_tree` module), leaving a few inert cfg checks in merge.rs /
// merkle_proof.rs. Allow them rather than re-advertise a half-removed feature.
#![allow(unexpected_cfgs)]

// VENDORED + STRIPPED for Noesis (2026-07-12): the `blake2b` (C via blake2b-rs), `ckb_smt`
// (C-backed SMT + build.rs), and `trie_tree` modules and the crate's own tests/benches were
// removed so this builds PURE-RUST / no_std with no C toolchain — cross-compilable to RISC-V for
// the on-VM / zkVM guest. The audited tree/merge/proof math is UNCHANGED. The consumer supplies the
// `Hasher` (Noesis plugs in `blake2b-ref`). Upstream: nervosnetwork/sparse-merkle-tree v0.6.1 (MIT).
pub mod default_store;
pub mod error;
pub mod h256;
pub mod merge;
pub mod merkle_proof;
pub mod traits;
mod tree;

pub use h256::H256;
pub use merkle_proof::{CompiledMerkleProof, MerkleProof};
pub use tree::{BranchKey, BranchNode, SparseMerkleTree};

/// Expected path size: log2(256) * 2, used for hint vector capacity
pub const EXPECTED_PATH_SIZE: usize = 16;
// Max stack size can be used when verify compiled proof
pub(crate) const MAX_STACK_SIZE: usize = 257;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::collections;
        use std::vec;
        use std::string;
    } else {
        extern crate alloc;
        use alloc::collections;
        use alloc::vec;
        use alloc::string;
    }
}
