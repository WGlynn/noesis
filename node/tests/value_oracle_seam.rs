//! v(S) value-oracle seam tests (the blockchain <-> AI boundary).
//!
//! Two properties, both anti-theater:
//!   (1) PARITY — routing the consensus attribution path through the `NoveltyOracleV0` oracle is
//!       byte-identical to calling the named floor entry directly, so introducing the seam changed
//!       no consensus behaviour.
//!   (2) SWAPPABILITY — a *different* oracle plugged into the same aggregator genuinely changes the
//!       scores. If it did not, the seam would be decorative. This is the property that makes
//!       "get the learned v(S) right later" a component swap, not a rebuild.
//!
//! See docs/DESIGN-value-oracle-seam.md.

use noesis::{
    pom_scores_with_oracle, pom_scores_with_similarity_floor_q16, Cell, NoveltyOracleV0, Script,
    ValueOracle,
};

/// A production-like similarity floor (~0.95 in Q16.16); distinct-content cells sit well below it,
/// so honest novel work survives. Parity holds for ANY theta; this value just mirrors the default.
const THETA_SIM_Q16: u64 = 62259;

fn cell(id: u64, contributor: &[u8], data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: contributor.to_vec() },
        parent: None,
        timestamp: id,
        data: data.to_vec(),
    }
}

/// Distinct-content cells (real novelty), two authored by alice, one each by bob and carol.
fn workload() -> Vec<Cell> {
    vec![
        cell(1, b"alice", b"the quick brown fox jumps high over meadows"),
        cell(2, b"bob", b"lorem ipsum dolor sit amet consectetur now"),
        cell(3, b"alice", b"an entirely separate account of cold winter light"),
        cell(4, b"carol", b"fresh unrelated subject matter appears in this cell"),
    ]
}

#[test]
fn v0_oracle_path_is_byte_identical_to_the_named_floor_entry() {
    let cells = workload();
    let direct = pom_scores_with_similarity_floor_q16(&cells, THETA_SIM_Q16);
    let via_oracle = pom_scores_with_oracle(&NoveltyOracleV0, &cells, THETA_SIM_Q16);
    assert_eq!(
        direct, via_oracle,
        "the v0 oracle must reproduce the consensus attribution exactly (seam introduced no drift)"
    );
}

/// A trivial replacement oracle: value 1 per cell regardless of novelty. Stands in for "a different
/// v(S)" (e.g. the future learned model) to prove the seam actually swaps.
struct ConstantOracle;
impl ValueOracle for ConstantOracle {
    fn cell_values(&self, cells_in_commit_order: &[Cell], _theta_sim_q16: u64) -> Vec<u64> {
        vec![1; cells_in_commit_order.len()]
    }
}

#[test]
fn a_different_oracle_swaps_in_and_actually_changes_attribution() {
    let cells = workload();

    // Constant-1 oracle => each contributor scores their cell count: alice=2, bob=1, carol=1.
    let swapped = pom_scores_with_oracle(&ConstantOracle, &cells, THETA_SIM_Q16);
    assert_eq!(swapped.get(&b"alice".to_vec()).copied(), Some(2));
    assert_eq!(swapped.get(&b"bob".to_vec()).copied(), Some(1));
    assert_eq!(swapped.get(&b"carol".to_vec()).copied(), Some(1));

    // ...and it genuinely differs from the novelty oracle, or the swap would be a no-op (theater).
    let novelty = pom_scores_with_oracle(&NoveltyOracleV0, &cells, THETA_SIM_Q16);
    assert_ne!(
        swapped, novelty,
        "swapping the oracle must change the scores, else the seam is decorative"
    );
}
