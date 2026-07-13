//! Equivocation slashing on the LIVE finality path (A4 accountable safety).
//!
//! The guard (`finality::finalizes_with_equivocation_guard` / `epoch_equivocators`) was built but
//! only reachable off the live path — `Node::checkpoint_finalizes` called the bare, history-less
//! rule. These tests prove `Node::checkpoint_finalizes_guarded` wires the guard in:
//!   (1) honest epoch  -> identical verdict to bare `checkpoint_finalizes`, no equivocators reported;
//!   (2) a double-vote -> the equivocator's weight is stripped from BOTH sets before counting, which
//!       FLIPS a would-finalize checkpoint to not-final, and the offender is reported. Anti-theater:
//!       the bare path (no vote history) still finalizes on the tainted weight, so the guard is what
//!       changes the outcome;
//!   (3) accountability -> slashing the reported id zeros its vote weight and cuts its stake.

use noesis::consensus::{slash, Validator};
use noesis::runtime::{Constitution, Node};

fn val(id: u64, pos: f64, pom: f64, stake: f64) -> Validator {
    Validator { id, pow: 0.0, pos, pom, last_heartbeat: 0, staked_balance: stake }
}

const P1: u64 = 100;
const P2: u64 = 200;

/// A (id 0) is pivotal: with A the supporters clear 2/3 (6/8 = 75%); strip A from both the supporting
/// set and the basis and the honest remainder (B,C) falls to 2/4 = 50% < 2/3. D (id 3) abstains from
/// this checkpoint, so it keeps the basis from collapsing onto the supporters. pos == pom per validator,
/// so the PoS:PoM finality mix reduces to that scalar and both anti-concentration dims move together.
fn scenario() -> (Node, Vec<Validator>, Vec<Validator>) {
    let a = val(0, 4.0, 4.0, 1000.0);
    let b = val(1, 1.0, 1.0, 100.0);
    let c = val(2, 1.0, 1.0, 100.0);
    let d = val(3, 2.0, 2.0, 100.0);
    let all = vec![a.clone(), b.clone(), c.clone(), d.clone()];
    let voters_for = vec![a, b, c]; // D abstains from THIS checkpoint
    let node = Node::new(0, all.clone(), Constitution::default());
    (node, voters_for, all)
}

#[test]
fn honest_epoch_matches_bare_path_and_reports_no_equivocators() {
    let (node, voters_for, all) = scenario();
    // one ballot per validator, all for the same proposal -> no double-vote.
    let ballots = vec![(0, P1), (1, P1), (2, P1)];
    let (guarded, equivocators) = node.checkpoint_finalizes_guarded(&voters_for, &all, &ballots);
    assert_eq!(
        guarded,
        node.checkpoint_finalizes(&voters_for, &all),
        "with no equivocation the guarded path must match the bare finality verdict"
    );
    assert!(equivocators.is_empty(), "an honest epoch has no equivocators");
}

#[test]
fn a_double_vote_is_stripped_before_counting_and_flips_finality() {
    let (node, voters_for, all) = scenario();

    // baseline (anti-theater): the checkpoint DOES finalize when A's weight is counted, so the guard
    // is provably what changes the outcome — not the scenario finalizing on its own.
    assert!(
        node.checkpoint_finalizes(&voters_for, &all),
        "baseline: honest weight finalizes -> the guard must be what flips it"
    );

    // A (id 0) double-votes: P1 then P2 in the same epoch.
    let ballots = vec![(0, P1), (1, P1), (2, P1), (0, P2)];
    let (guarded, equivocators) = node.checkpoint_finalizes_guarded(&voters_for, &all, &ballots);

    assert!(
        !guarded,
        "the equivocator's tainted weight is stripped from both sets, dropping the honest remainder \
         below 2/3 -> the checkpoint must NOT finalize"
    );
    assert_eq!(equivocators, vec![0], "the double-signer must be reported for slashing");
}

#[test]
fn a_reported_equivocator_can_be_slashed() {
    let (node, voters_for, all) = scenario();
    let ballots = vec![(0, P1), (1, P1), (2, P1), (0, P2)];
    let (_finalizes, equivocators) = node.checkpoint_finalizes_guarded(&voters_for, &all, &ballots);

    // the caller applies the stake slash to the reported ids on its (persistent) validator set.
    let mut set = all;
    for id in &equivocators {
        if let Some(v) = set.iter_mut().find(|v| v.id == *id) {
            slash(v, 500.0);
        }
    }
    let a = set.iter().find(|v| v.id == 0).unwrap();
    assert_eq!(a.staked_balance, 500.0, "slash cuts the stake by the amount");
    assert_eq!((a.pos, a.pom, a.pow), (0.0, 0.0, 0.0), "slash zeros all vote weight");
}
