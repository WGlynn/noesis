//! Parity harness for the zk-score PoC (Fit 2 of `docs/ZK-INTEGRATION.md`).
//!
//! Runs the EXACT functions the RISC Zero guest proves — `noesis_core::zk_score_eval` +
//! `zk_score_public_digest` — on canonical fixtures, through the SAME single-sourced proof-wire
//! (`noesis_core::flatten_proofs`/`unflatten_proofs`) the guest decodes. No zkVM here: this is the
//! host-side ground truth. It reproduces the FULL journal tuple `(public_digest, nullifier, accepted,
//! value>=V_FLOOR)` the receipt must commit — digest included (same sha256 the guest uses, via core).
//!
//! It also demonstrates the VERIFIER contract, not just the prover: a receipt earns standing only if
//! `accepted && value>=V_FLOOR && digest == zk_score_public_digest(CANONICAL_root, thetas)`. The
//! empty-root standing-forgery is included precisely to show the digest-pin rejects it.
//!
//! The transparency<->privacy factoring, concretely: PUBLIC rule (zk_score_eval), PUBLIC corpus root,
//! PRIVATE content + proofs (never in the journal), verifiable verdict either way.

use noesis_core::{
    flatten_proofs, leaf, unflatten_proofs, unique_shingles, verify_member, verify_non_member,
    zk_score_eval, zk_score_nullifier, zk_score_public_digest, Hash, DEPTH, ZK_SCORE_V_FLOOR,
};

// ============ Personalized blake2b (drift-guarded mirror of core's SMT hasher) ============
// MUST match `noesis_core`'s internal SMT node hasher (personal "noesis-smt-v1"). Core does not
// export a 2-ary node hash, so the host-side corpus index replicates it here. Any drift is caught as
// a hard panic below: every honest proof this index emits is re-checked with core's OWN
// `verify_member` / `verify_non_member`. A silent miscount is impossible.
fn node_hash(a: &[u8; 32], b: &[u8; 32]) -> Hash {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(b"noesis-smt-v1\0\0\0").build();
    h.update(a);
    h.update(b);
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

// ============ Host-only corpus index (mirror of node::smt::NoveltyIndex) ============
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

fn index_of(contents: &[&[u8]]) -> NoveltyIndex {
    let mut idx = NoveltyIndex::new();
    for c in contents {
        for (k, _) in unique_shingles(c) {
            idx.insert(k);
        }
    }
    idx
}
fn proofs_for(idx: &NoveltyIndex, data: &[u8]) -> Vec<[Hash; DEPTH]> {
    unique_shingles(data).iter().map(|(k, _)| idx.proof(*k)).collect()
}

// Public thresholds (same Q16.16 constants the node's proven-intake tests use). V_FLOOR is NOT here —
// it is baked into `noesis_core::ZK_SCORE_V_FLOOR` (and thus the guest image id), never prover-chosen.
const SIM: u64 = 52429; // theta_sim  (~0.8)
const ENT: u64 = 62259; // theta_ent  (~0.95)

/// The journal a guest would commit, computed via the SAME core functions the guest calls.
struct Journal {
    digest: Hash,
    nullifier: Hash,
    accepted: bool,
    value_ge_floor: bool,
}

/// Decode-then-evaluate EXACTLY as the guest will: private content + flat proof wire + a claimed
/// `root` in, the public journal out. `tamper` corrupts one wire byte (forged path). The `root` the
/// prover supplies is used for BOTH the classification and the digest — so a lied root is bound into
/// the digest and caught by the verifier's pin.
fn run_guest(data: &[u8], root: Hash, proofs: &[[Hash; DEPTH]], tamper: bool) -> Journal {
    let mut flat = flatten_proofs(proofs);
    if tamper && !flat.is_empty() {
        flat[0] ^= 0xFF;
    }
    let n = unique_shingles(data).len();
    let verdict =
        unflatten_proofs(&flat, n).and_then(|p| zk_score_eval(data, root, &p, SIM, ENT));
    let (nullifier, accepted, value_ge_floor) = match verdict {
        Some((nf, value)) => (nf, true, value >= ZK_SCORE_V_FLOOR),
        None => ([0u8; 32], false, false),
    };
    Journal { digest: zk_score_public_digest(root, SIM, ENT), nullifier, accepted, value_ge_floor }
}

fn main() {
    let corpus: &[&[u8]] = &[
        b"alpha-bravo-charlie-delta-echo-foxtrot",
        b"golf-hotel-india-juliet-kilo-lima",
    ];
    let idx = index_of(corpus);
    let root = idx.root();
    let empty_idx = NoveltyIndex::new();
    let empty_root = empty_idx.root();

    // The verifier knows the canonical corpus root and pins the digest to it. A receipt earns
    // standing ONLY if it is accepted, clears the floor, AND its digest matches this.
    let canonical_digest = zk_score_public_digest(root, SIM, ENT);
    let verifier_grants = |j: &Journal| j.accepted && j.value_ge_floor && j.digest == canonical_digest;

    // --- DRIFT GUARD: core's OWN verifier accepts this host index's honest proofs. Panics on drift.
    {
        let present = unique_shingles(corpus[0])[0].0;
        let absent = 0xDEAD_BEEF_0BAD_F00Du64;
        assert!(verify_member(root, present, &idx.proof(present)), "drift: member proof rejected");
        assert!(verify_non_member(root, absent, &idx.proof(absent)), "drift: non-member proof rejected");
        assert!(!verify_non_member(root, present, &idx.proof(present)), "present key must not deny");
    }

    // --- NULLIFIER: same content => same nullifier (ledger dedups replay); different content differs.
    {
        let a1 = zk_score_nullifier(b"mike-november-oscar-papa-quebec-romeo-sierra-tango");
        let a2 = zk_score_nullifier(b"mike-november-oscar-papa-quebec-romeo-sierra-tango");
        let b = zk_score_nullifier(corpus[0]);
        assert_eq!(a1, a2, "nullifier must be deterministic for identical content (replay dedup)");
        assert_ne!(a1, b, "distinct content must yield distinct nullifiers");
    }

    let noise: Vec<u8> = (0u8..48).collect();

    // Each case: (label, content, root the prover uses, proofs, tamper, expect standing_granted, expect accepted)
    let fresh = b"mike-november-oscar-papa-quebec-romeo-sierra-tango".as_slice();
    let dup = corpus[0]; // a plagiarist copies existing corpus content

    struct Case<'a> {
        label: &'a str,
        data: &'a [u8],
        root: Hash,
        proofs: Vec<[Hash; DEPTH]>,
        tamper: bool,
        want_standing: bool,
        want_accepted: bool,
        note: &'a str,
    }
    let cases = vec![
        Case { label: "fresh novel work", data: fresh, root, proofs: proofs_for(&idx, fresh), tamper: false, want_standing: true, want_accepted: true, note: "novel vs corpus, clears both floors" },
        Case { label: "exact duplicate", data: dup, root, proofs: proofs_for(&idx, dup), tamper: false, want_standing: false, want_accepted: true, note: "similarity floor -> value 0" },
        Case { label: "high-entropy noise", data: &noise, root, proofs: proofs_for(&idx, &noise), tamper: false, want_standing: false, want_accepted: true, note: "entropy floor -> value 0" },
        Case { label: "empty content", data: b"", root, proofs: vec![], tamper: false, want_standing: false, want_accepted: false, note: "FIX F: empty cell rejected, not vacuously accepted" },
        Case { label: "tampered proof", data: fresh, root, proofs: proofs_for(&idx, fresh), tamper: true, want_standing: false, want_accepted: false, note: "forged path proves neither polarity -> reject" },
        Case { label: "forged member->absent", data: dup, root, proofs: proofs_for(&empty_idx, dup), tamper: false, want_standing: false, want_accepted: false, note: "FIX H: empty-root paths vs REAL root prove neither -> reject" },
        Case { label: "empty-root forgery", data: dup, root: empty_root, proofs: proofs_for(&empty_idx, dup), tamper: false, want_standing: false, want_accepted: true, note: "FIX A: scores against a fake corpus, but digest-pin REJECTS it" },
    ];

    println!("zk-score parity — zk_score_eval over PRIVATE content (Fit 2), V_FLOOR={ZK_SCORE_V_FLOOR}");
    println!("  public params: theta_sim={SIM} theta_ent={ENT}");
    println!("  canonical corpus root: {}", hex(&root));
    println!("  canonical digest (verifier pins this): {}\n", hex(&canonical_digest));

    let mut anchors_ok = true;
    for c in &cases {
        let j = run_guest(c.data, c.root, &c.proofs, c.tamper);
        let standing = verifier_grants(&j);
        let verdict = if standing {
            "STANDING"
        } else if !j.accepted {
            "REJECTED"
        } else if j.digest != canonical_digest {
            "ROOT-PIN" // accepted+scored, but the verifier's root pin rejects it
        } else {
            "NO-STAND"
        };
        // The nullifier is committed iff the cell is accepted (zero sentinel on reject); the ledger
        // dedups accepted receipts by it. Verify that invariant here so the field is load-bearing.
        let nullifier_ok = (j.nullifier != [0u8; 32]) == j.accepted;
        let ok = standing == c.want_standing && j.accepted == c.want_accepted && nullifier_ok;
        let nf_disp = if j.accepted { hex(&j.nullifier)[..8].to_string() } else { "--------".to_string() };
        // NOTE: content bytes never enter the journal line — that is the privacy property.
        println!(
            "  [{}] {:<22} journal=(accepted={}, value>=V={}, nullifier={}) standing={}   {:<8} {}",
            verdict,
            c.label,
            j.accepted,
            j.value_ge_floor,
            nf_disp,
            standing,
            if ok { "ok" } else { "MISMATCH" },
            c.note
        );
        if !ok {
            anchors_ok = false;
        }
    }

    assert!(anchors_ok, "anchor fixtures must match the ground truth");
    println!("\nanchors verified. journal = (public_digest, nullifier, accepted, value>=V_FLOOR) ONLY;");
    println!("content + exact score are private witness — they never enter the journal.");
    println!("standing requires the verifier's digest-pin to the CANONICAL root — a prover-chosen root");
    println!("(empty-root forgery) is scored but denied standing. These are what the receipt must commit.");
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
