//! zk-score host — prove + verify the canonical Fit-2 fixtures.
//!
//! Builds the corpus index and the per-shingle proofs, feeds the guest the content + proofs as a
//! PRIVATE input, proves, verifies the receipt against the guest image id, decodes the journal
//! `(public_digest, nullifier, accepted, value>=V_FLOOR)`, and then applies the VERIFIER contract:
//! standing is granted only if `accepted && value>=V_FLOOR && digest == canonical digest`. The
//! printed verdicts must equal the `parity` harness (host stable, no zkVM).
//!
//! The corpus index is a compact mirror of `parity/`'s (and of `node::smt::NoveltyIndex`). This host
//! only runs where a RISC Zero prover exists (Linux / WSL2); it is not compiled on Windows.

use risc0_zkvm::{default_prover, ExecutorEnv};
use zk_score_methods::{ZK_SCORE_ELF, ZK_SCORE_ID};

use noesis_core::{
    flatten_proofs, leaf, unique_shingles, zk_score_public_digest, Hash, DEPTH, ZK_SCORE_V_FLOOR,
};

const SIM: u64 = 52429;
const ENT: u64 = 62259;

fn node_hash(a: &[u8; 32], b: &[u8; 32]) -> Hash {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-smt-v1\0\0\0").build();
    h.update(a);
    h.update(b);
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

struct NoveltyIndex {
    nodes: std::collections::HashMap<(usize, u64), Hash>,
    empty: [Hash; DEPTH + 1],
}
impl NoveltyIndex {
    fn new() -> Self {
        let mut empty = [[0u8; 32]; DEPTH + 1];
        for d in 1..=DEPTH {
            empty[d] = node_hash(&empty[d - 1], &empty[d - 1]);
        }
        NoveltyIndex { nodes: std::collections::HashMap::new(), empty }
    }
    fn node(&self, height: usize, prefix: u64) -> Hash {
        *self.nodes.get(&(height, prefix)).unwrap_or(&self.empty[height])
    }
    fn root(&self) -> Hash {
        self.node(DEPTH, 0)
    }
    fn proof(&self, key: u64) -> [Hash; DEPTH] {
        let mut sib = [[0u8; 32]; DEPTH];
        for (i, s) in sib.iter_mut().enumerate() {
            *s = self.node(i, (key >> i) ^ 1);
        }
        sib
    }
    fn insert(&mut self, key: u64) {
        let mut acc = leaf(key);
        self.nodes.insert((0, key), acc);
        for i in 0..DEPTH {
            let prefix = key >> i;
            let sib = self.node(i, prefix ^ 1);
            acc = if prefix & 1 == 0 { node_hash(&acc, &sib) } else { node_hash(&sib, &acc) };
            self.nodes.insert((i + 1, prefix >> 1), acc);
        }
    }
}

/// Prove one case. `root` is what the prover feeds the guest (honest = canonical; forgery = empty).
/// Returns the decoded journal 4-tuple. The digest + nullifier are decoded as the SAME `Hash`
/// (`[u8; 32]`) type the guest commits — no risc0 `Digest` hop, so the decode round-trips.
fn prove_case(data: &[u8], root: Hash, proofs_flat: Vec<u8>) -> (Hash, Hash, bool, bool) {
    let env = ExecutorEnv::builder()
        .write(&data.to_vec())
        .unwrap()
        .write(&proofs_flat)
        .unwrap()
        .write(&root)
        .unwrap()
        .write(&SIM)
        .unwrap()
        .write(&ENT)
        .unwrap()
        .build()
        .unwrap();

    let receipt = default_prover().prove(env, ZK_SCORE_ELF).unwrap().receipt;
    receipt.verify(ZK_SCORE_ID).unwrap();
    receipt.journal.decode().unwrap()
}

fn main() {
    let corpus: &[&[u8]] = &[
        b"alpha-bravo-charlie-delta-echo-foxtrot",
        b"golf-hotel-india-juliet-kilo-lima",
    ];
    let mut idx = NoveltyIndex::new();
    for c in corpus {
        for (k, _) in unique_shingles(c) {
            idx.insert(k);
        }
    }
    let root = idx.root();
    let empty_idx = NoveltyIndex::new();
    let empty_root = empty_idx.root();

    // The verifier pins the digest to the canonical corpus root.
    let canonical_digest: Hash = zk_score_public_digest(root, SIM, ENT);

    let flat = |data: &[u8], src: &NoveltyIndex| {
        let p: Vec<[Hash; DEPTH]> = unique_shingles(data).iter().map(|(k, _)| src.proof(*k)).collect();
        flatten_proofs(&p)
    };

    // (label, content, root fed to guest, proof source index, expect standing, expect accepted)
    let fresh = b"mike-november-oscar-papa-quebec-romeo-sierra-tango".as_slice();
    let dup = corpus[0];
    let cases: Vec<(&str, &[u8], Hash, &NoveltyIndex, bool, bool)> = vec![
        ("fresh novel work", fresh, root, &idx, true, true),
        ("exact duplicate", dup, root, &idx, false, true),
        ("empty-root forgery", dup, empty_root, &empty_idx, false, true),
    ];

    println!("zk-score host — proving zk_score_eval in the RISC Zero zkVM (V_FLOOR={ZK_SCORE_V_FLOOR})\n");
    for (label, data, prover_root, src, exp_standing, exp_accepted) in cases {
        let (digest, _nullifier, accepted, value_ge_floor) =
            prove_case(data, prover_root, flat(data, src));
        let digest_ok = digest == canonical_digest;
        // Ledger consumes the nullifier ONLY on a fully-granted standing (never on a root-pin-denied
        // or below-floor receipt), so a forged-root receipt cannot burn the honest author's nullifier.
        let standing = accepted && value_ge_floor && digest_ok;
        assert_eq!(accepted, exp_accepted, "proven 'accepted' for '{label}' disagrees with parity");
        assert_eq!(standing, exp_standing, "verifier standing for '{label}' disagrees with parity");
        let verdict = if standing {
            "STANDING"
        } else if !accepted {
            "REJECTED"
        } else if !digest_ok {
            "ROOT-PIN"
        } else {
            "NO-STAND"
        };
        println!(
            "  [{}] {:<22} proven+verified  journal=(accepted={}, value>=V={}, digest_pinned={})",
            verdict, label, accepted, value_ge_floor, digest_ok
        );
    }
    println!("\nall receipts verified against the guest image id; content stayed a private witness;");
    println!("standing granted only when the journal digest matches the canonical corpus root.");
}
