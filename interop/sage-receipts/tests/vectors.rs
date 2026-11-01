//! Parity with the reference: every vector in tests/vectors/ was produced by SAGE's own Go code
//! (l33tdawg/sage at ae7f8a7, Go 1.26.10, `internal/outcomereceipt`: Decode + VerifyEnvelope, and
//! DigestHex / CanonicalJSON / TaskSetDigest for accepted cases). For each one this crate must reach
//! the same verdict, and for accepted ones the same digest and the same canonical bytes.
//!
//! The only allowed disagreements are the deliberate, fail-closed ones listed in STRICTER (see the
//! crate docs). Any other mismatch fails, and so does an entry in STRICTER that stops being a
//! mismatch, so the list cannot go stale.

use sage_receipts::{canonical_json, decode, task_set_digest, verify, verify_to_label};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vector {
    name: String,
    input: String,
    accept: bool,
    #[serde(default)]
    digest: String,
    #[serde(default)]
    canonical: String,
    #[serde(default)]
    task_set_digest: String,
    #[serde(default)]
    go_error: String,
}

/// Inputs the reference accepts and this verifier rejects on purpose.
const STRICTER: [&str; 1] = [
    // Go matches object keys case-insensitively, so "Version" fills `version`. Exact keys only here.
    "key in another case",
];

fn vectors() -> Vec<Vector> {
    let text = include_str!("vectors/sage-ae7f8a7.json");
    serde_json::from_str(text).expect("vector file parses")
}

#[test]
fn every_reference_vector_gets_the_same_verdict_digest_and_bytes() {
    let mut mismatches = Vec::new();
    for v in vectors() {
        let ours = decode(v.input.as_bytes()).and_then(|env| verify(&env).map(|d| (env, d)));
        match (&ours, v.accept) {
            (Ok((env, digest)), true) => {
                assert_eq!(digest, &v.digest, "{}: digest", v.name);
                assert_eq!(
                    canonical_json(&env.receipt).unwrap(),
                    v.canonical,
                    "{}: canonical bytes",
                    v.name
                );
                assert_eq!(
                    task_set_digest(&env.receipt.task_set.tasks),
                    v.task_set_digest,
                    "{}: task-set digest",
                    v.name
                );
            }
            (Err(_), false) => {}
            (Ok(_), false) => mismatches.push(format!(
                "{}: we accept, the reference rejects ({})",
                v.name, v.go_error
            )),
            (Err(e), true) => mismatches.push(format!("{}: we reject ({e}), the reference accepts", v.name)),
        }
    }
    let unexpected: Vec<_> = mismatches
        .iter()
        .filter(|m| !STRICTER.iter().any(|s| m.starts_with(&format!("{s}:"))))
        .collect();
    assert!(
        unexpected.is_empty(),
        "disagreements with the reference:\n{}",
        unexpected
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    for s in STRICTER {
        assert!(
            mismatches.iter().any(|m| m.starts_with(&format!("{s}:"))),
            "{s} no longer differs from the reference: drop it from STRICTER"
        );
    }
}

#[test]
fn the_base_vector_reproduces_sages_golden_digest() {
    // The digest SAGE's own test pins for its validReceipt fixture (receipt_test.go).
    let base = vectors()
        .into_iter()
        .find(|v| v.name.starts_with("base: "))
        .unwrap();
    let env = decode(base.input.as_bytes()).unwrap();
    assert_eq!(
        verify(&env).unwrap(),
        "84d860249ab99478850a1dd91248887ff9a9950882d029d0449b3308ad9f0d54"
    );
}

#[test]
fn the_vectors_cover_both_verdicts() {
    let vs = vectors();
    assert!(vs.iter().filter(|v| v.accept).count() >= 8, "accepted cases");
    assert!(vs.iter().filter(|v| !v.accept).count() >= 20, "rejected cases");
}

#[test]
fn a_valid_receipt_becomes_a_label_with_independence_facts() {
    let base = vectors()
        .into_iter()
        .find(|v| v.name.starts_with("base: "))
        .unwrap();
    let label = verify_to_label(base.input.as_bytes()).unwrap();
    assert_eq!(
        label.receipt_digest,
        "84d860249ab99478850a1dd91248887ff9a9950882d029d0449b3308ad9f0d54"
    );
    assert_eq!(
        (label.status.as_str(), label.delta.as_str(), label.pairs),
        ("positive", "0.25", 4)
    );
    assert_eq!(label.candidate.memory_id, "mem-candidate");
    assert_eq!(label.signers.len(), 1);
    assert!(!label.independence.evaluator_is_candidate_author);
    assert!(!label.independence.evaluator_is_task_author);
    assert_eq!(label.independence.task_authors, 2);
}

#[test]
fn an_evaluator_grading_its_own_memory_is_valid_for_sage_and_flagged_in_the_label() {
    // SAGE's contract leaves evaluator independence to the consumer, so the reference accepts this
    // receipt. The label must carry the fact so a written evaluator policy can refuse it.
    let v = vectors()
        .into_iter()
        .find(|v| v.name.starts_with("self-evaluation"))
        .unwrap();
    assert!(v.accept, "the reference accepts a self-evaluated receipt");
    let label = verify_to_label(v.input.as_bytes()).unwrap();
    assert!(label.independence.evaluator_is_candidate_author);
    assert!(!label.independence.evaluator_is_task_author);
}
