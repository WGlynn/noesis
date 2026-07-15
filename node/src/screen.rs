//! screen — the bootstrap ingress filter (advisory, node-local): a cheap quality + originality gate
//! at `POST /submit`, BEFORE a contribution enters the mempool. Scaffolding for the early network,
//! before the structural defenses are robust (`docs/DESIGN-authorityless-contribution-value.md` §5).
//!
//! WHAT IT IS: a spam filter. It rejects trivial/low-information content (the "booger‑chain") and
//! content that overlaps what this node has already seen (imported plagiarism the on‑chain θ_sim floor
//! cannot catch because the source was never on‑chain).
//!
//! WHAT IT IS NOT (load‑bearing honesty, `[[airgap-problem]]`): it is NOT a consensus rule and NOT the
//! value function `v(S)`. Nodes may run different screens; a screen NEVER changes which blocks are
//! valid or final. Baking an off‑chain judgment into consensus would re‑introduce the exact authority
//! we deliberately removed. This is a heuristic that CONVERGES to the structural defense (Myerson value
//! + collusion slash + dispute market + learned‑v(S)) as the network robustifies — it buys time, it is
//! not the endgame. Reuses the built `coverage` shingling (`lib.rs:147`); no LLM, deterministic.

use std::collections::HashSet;

use crate::{coverage, CovId};

/// Minimum submission length in bytes. Below this there is no room for a genuine contribution.
const MIN_LEN: usize = 16;
/// Minimum number of DISTINCT content shingles — an absolute floor for very short content.
const MIN_DISTINCT_SHINGLES: usize = 6;
/// Minimum ratio (basis points) of DISTINCT to TOTAL shingles. Repetitive padding ("booger booger
/// booger …") produces many total shingles but few distinct ones (a low ratio), while genuine text is
/// nearly all‑distinct (ratio → 1). This catches long‑but‑repetitive content that the absolute distinct
/// floor alone misses — the word‑boundary transitions of a repeated short token clear the absolute
/// floor but not the ratio. HONEST LIMIT: a determined attacker can still pad with *varied* junk to
/// beat both floors; the ratio raises the cost, it does not prove value (that is `v(S)`, §4 of the doc).
const MIN_UNIQUE_RATIO_BPS: u64 = 5000; // 50%
/// Maximum fraction (in basis points) of a submission's distinct shingles that may already be in this
/// node's seen‑set. Above this the content is a near‑copy of something seen (dup / imported plagiarism).
const MAX_OVERLAP_BPS: u64 = 8000; // 80%

/// Why a submission was screened out (advisory — returned to the submitter, never consensus).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reject {
    TooShort,
    LowInformation,
    Unoriginal { overlap_bps: u64 },
}

impl Reject {
    pub fn message(&self) -> String {
        match self {
            Reject::TooShort => "contribution is too short to carry value".into(),
            Reject::LowInformation => "contribution is trivial / too repetitive (low information)".into(),
            Reject::Unoriginal { overlap_bps } => {
                format!("contribution overlaps content already seen ({:.0}% — near-duplicate/plagiarism)", *overlap_bps as f64 / 100.0)
            }
        }
    }
}

/// A node's ingress screen: the set of content shingles it has ever accepted (optionally seeded with an
/// external reference corpus so imported plagiarism is caught from block zero). Lives on the node, not
/// the chain.
#[derive(Default)]
pub struct Screen {
    seen: HashSet<CovId>,
}

impl Screen {
    pub fn new() -> Self {
        Screen { seen: HashSet::new() }
    }

    /// Seed the seen‑set with an external reference corpus (e.g. bundled public text/code shingles) so
    /// off‑chain plagiarism is rejected before the chain has any history of its own.
    pub fn seed(&mut self, corpus: &[u8]) {
        self.seen.extend(coverage(corpus));
    }

    /// Screen a candidate submission. `Ok(())` ⇒ admit to the mempool; `Err(reason)` ⇒ advise the
    /// submitter and drop it (node‑local, never a consensus verdict). Pure: does not mutate the seen‑set
    /// (call [`record`](Self::record) only once a submission is actually accepted + finalized).
    pub fn check(&self, data: &[u8]) -> Result<(), Reject> {
        if data.len() < MIN_LEN {
            return Err(Reject::TooShort);
        }
        let cov = coverage(data);
        let total = cov.len().max(1);
        let distinct: HashSet<CovId> = cov.into_iter().collect();
        if distinct.len() < MIN_DISTINCT_SHINGLES {
            return Err(Reject::LowInformation);
        }
        // Repetitiveness: distinct/total ratio. Low ⇒ padded with repeats (catches "booger booger …").
        if (distinct.len() as u64 * 10_000 / total as u64) < MIN_UNIQUE_RATIO_BPS {
            return Err(Reject::LowInformation);
        }
        if !self.seen.is_empty() {
            let overlap = distinct.iter().filter(|c| self.seen.contains(c)).count();
            let overlap_bps = (overlap as u64 * 10_000) / distinct.len() as u64;
            if overlap_bps > MAX_OVERLAP_BPS {
                return Err(Reject::Unoriginal { overlap_bps });
            }
        }
        Ok(())
    }

    /// Fold an accepted submission's shingles into the seen‑set (so later copies of it are caught).
    pub fn record(&mut self, data: &[u8]) {
        self.seen.extend(coverage(data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booger_is_rejected_as_low_information_or_too_short() {
        let s = Screen::new();
        assert_eq!(s.check(b"booger"), Err(Reject::TooShort)); // 6 bytes < MIN_LEN
        // long but trivially repetitive ⇒ few distinct shingles ⇒ low information
        assert_eq!(s.check(b"booger booger booger booger booger booger"), Err(Reject::LowInformation));
        assert_eq!(s.check(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), Err(Reject::LowInformation));
    }

    #[test]
    fn a_genuine_contribution_passes() {
        let s = Screen::new();
        assert!(s.check(b"a genuinely novel account of winter light over cold northern rivers at dawn").is_ok());
    }

    #[test]
    fn a_near_copy_of_seen_content_is_rejected_as_unoriginal() {
        let mut s = Screen::new();
        let original = b"the treaty of westphalia reorganized european sovereignty in sixteen forty eight";
        assert!(s.check(original).is_ok());
        s.record(original);
        // resubmitting the same content ⇒ ~100% overlap ⇒ unoriginal
        match s.check(original) {
            Err(Reject::Unoriginal { overlap_bps }) => assert!(overlap_bps > MAX_OVERLAP_BPS),
            other => panic!("expected Unoriginal, got {other:?}"),
        }
        // a genuinely different contribution still passes against the same seen-set
        assert!(s.check(b"an unrelated meditation on desert stars and the silence of midnight dunes").is_ok());
    }

    #[test]
    fn external_seed_catches_imported_plagiarism_from_block_zero() {
        let mut s = Screen::new();
        // pretend this is a bundled public-corpus passage the attacker will try to copy
        let external = b"it was the best of times it was the worst of times it was the age of wisdom";
        s.seed(external);
        // copying it verbatim is rejected even though it was never on-chain
        match s.check(external) {
            Err(Reject::Unoriginal { .. }) => {}
            other => panic!("expected Unoriginal from external seed, got {other:?}"),
        }
    }
}
