//! Missed-finality-vote decay (the inactivity-leak analog): `runtime::finality::VoteLeak`,
//! `finalizes_pos_pom_leaking`, `Node::checkpoint_round`, `Constitution.{leak_grace, leak_horizon}`.
//!
//! The gap it closes: heartbeat decay keys to liveness, not to voting, and the per-dimension floor
//! (`MIN_DIM_BPS`) reads base weights. So a holder of more than half of one dimension who stays
//! online and withholds its vote halts finality, and (found while building this) heartbeat decay
//! does not rescue the offline case either. Each test names the break that turns it RED.

use noesis::consensus::{Validator, TWO_THIRDS_BPS};
use noesis::runtime::finality::{finalizes_pos_pom, finalizes_pos_pom_leaking, VoteLeak};
use noesis::runtime::{Constitution, Node};

fn val(id: u64, pos: f64, pom: f64) -> Validator {
    Validator { id, pow: 0.0, pos, pom, last_heartbeat: 0, staked_balance: pos }
}

fn leaking(grace: u64, leak_horizon: u64) -> Constitution {
    Constitution { leak_grace: grace, leak_horizon, ..Constitution::default() }
}

/// The capture scenario. Honest validators hold all the capital and 40% of PoM; two captors hold
/// 60% of PoM, stay online (fresh heartbeats) and never vote. Base weights: honest
/// 100/3 + 2/3*40 = 60, captors 2/3*60 = 40, so the honest set fails the 2/3 bar (60%) and the PoM
/// floor (40% < 50%).
fn capture_set() -> (Vec<Validator>, Vec<Validator>) {
    let honest = vec![val(0, 50.0, 20.0), val(1, 50.0, 20.0)];
    let captors = [val(2, 0.0, 30.0), val(3, 0.0, 30.0)];
    let all: Vec<Validator> = honest.iter().chain(captors.iter()).cloned().collect();
    (honest, all)
}

/// Rounds until the first finalization, or None within `max`.
fn first_final(node: &mut Node, voters: &[Validator], all: &[Validator], max: u64) -> Option<u64> {
    (1..=max).find(|_| node.checkpoint_round(voters, all))
}

// ---- (a) inert by default ----

/// Default constitution: `checkpoint_round` must equal `checkpoint_finalizes` on every set and
/// leave `vote_leak` empty. And with the leak ON but no scores yet, the leaking predicate must
/// match `finalizes_pos_pom` exactly, including under quorum floors (this also pins the lib.rs
/// `finalizes_hybrid_floor_on` refactor). RED if the default path records state, or if the leaked
/// view at retention 1.0 is not the identity.
#[test]
fn leak_off_by_default_is_identical_to_checkpoint_finalizes() {
    assert_eq!(Constitution::default().leak_horizon, 0, "the leak must ship inert");
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        seed >> 33
    };
    let mut node = Node::new(0, vec![], Constitution::default());
    let empty = VoteLeak::default();
    let mut agreed = 0;
    for _ in 0..400 {
        let n = 1 + (next() % 6) as usize;
        let all: Vec<Validator> = (0..n as u64)
            .map(|id| Validator {
                last_heartbeat: next() % 8,
                ..val(id, (next() % 5) as f64 * 25.0, (next() % 5) as f64 * 25.0)
            })
            .collect();
        let voters: Vec<Validator> = all.iter().filter(|_| next() % 3 != 0).cloned().collect();
        let (now, horizon, floor) = (next() % 10, next() % 12, [0, 3333, 5000, 9000][(next() % 4) as usize]);

        assert_eq!(node.checkpoint_round(&voters, &all), node.checkpoint_finalizes(&voters, &all));
        assert_eq!(node.vote_leak, VoteLeak::default(), "default path wrote leak state");

        for decay_pos in [false, true] {
            let reference = finalizes_pos_pom(&voters, &all, now, horizon, decay_pos, TWO_THIRDS_BPS, floor);
            let leaked =
                finalizes_pos_pom_leaking(&voters, &all, &empty, 8, now, horizon, decay_pos, TWO_THIRDS_BPS, floor);
            assert_eq!(leaked, reference, "leak with no scores diverged from finalizes_pos_pom");
            agreed += reference as u32;
        }
    }
    assert!(agreed > 50 && agreed < 750, "sweep must hit both outcomes (got {agreed} finalizing of 800)");
}

// ---- (b) online withholders: stall without the leak, resume with it ----

/// Without the leak the captors hold the stall forever. With grace 2 and leak horizon 10 the
/// expected first finalization is round 7: the binding constraint is the PoM floor,
/// 40 >= 0.5*(40 + 60r) <=> r <= 2/3 <=> missed >= 3.34 <=> missed = 4, and misses accrue from
/// stalled round 3 (stall > grace), so round k decides on k - 3 misses, giving k = 7. The 2/3 bar
/// clears earlier (r <= 0.75). RED if the leak never fires, fires during grace, or decays voters.
#[test]
fn online_withholding_pom_captors_stall_without_the_leak_and_finality_resumes_with_it() {
    let (honest, all) = capture_set();

    let mut off = Node::new(0, all.clone(), Constitution::default());
    assert_eq!(first_final(&mut off, &honest, &all, 200), None, "precondition: the captors hold the stall");

    let mut node = Node::new(0, all.clone(), leaking(2, 10));
    let before = node.ledger.state_digest();
    assert_eq!(first_final(&mut node, &honest, &all, 200), Some(7));
    assert_eq!(node.vote_leak, VoteLeak::default(), "a finalized round must clear the stall and every score");
    assert_eq!(node.ledger.state_digest(), before, "the leak touched the ledger (credit/attribution)");
    for v in &all {
        assert_eq!(v.staked_balance, v.pos, "stake is never touched");
    }
}

/// The finding behind (b): heartbeat decay does not relax the dimension floor, because `dim_ok`
/// reads base weights. Captors fully decayed (offline, heartbeat 0, now = horizon, symmetric decay)
/// still block it; the leak resumes finality. RED if heartbeat decay starts reaching `dim_ok` (then
/// update `SECURITY.md` section 5, which states this limit).
#[test]
fn heartbeat_decay_alone_never_relaxes_the_dimension_floor() {
    let (honest, all) = capture_set();
    let horizon = 4;
    let fresh: Vec<Validator> =
        honest.iter().map(|v| Validator { last_heartbeat: horizon, ..v.clone() }).collect();
    let all: Vec<Validator> = all
        .iter()
        .map(|v| if v.id < 2 { Validator { last_heartbeat: horizon, ..v.clone() } } else { v.clone() })
        .collect();
    assert!(
        !finalizes_pos_pom(&fresh, &all, horizon, horizon, true, TWO_THIRDS_BPS, 0),
        "offline captors fully decayed, yet the PoM floor still blocks: base weights decide dim_ok"
    );

    let mut leak = VoteLeak::default();
    let mut rounds = 0;
    while !finalizes_pos_pom_leaking(&fresh, &all, &leak, 10, horizon, horizon, true, TWO_THIRDS_BPS, 0) {
        leak.record(&all, &fresh, false, 0, 10);
        rounds += 1;
        assert!(rounds < 50, "the leak must restore finality for offline captors too");
    }
    assert_eq!(rounds, 4, "grace 0: four misses take r to 0.6, the first value under 2/3");
}

// ---- (c) honest voters are never decayed ----

/// Every honest voter keeps retention exactly 1.0 and never appears in the score map, through the
/// whole stall. RED if `record` scores voters or the decrement underflows into a score.
#[test]
fn honest_voters_are_never_decayed() {
    let (honest, all) = capture_set();
    let mut node = Node::new(0, all.clone(), leaking(2, 10));
    for _ in 0..6 {
        assert!(!node.checkpoint_round(&honest, &all));
        for v in &honest {
            assert!(!node.vote_leak.missed.contains_key(&v.id), "an honest voter was scored");
            assert_eq!(node.vote_leak.retention(v.id, 10), 1.0);
        }
    }
    assert_eq!(node.vote_leak.missed.get(&2), Some(&4), "the captors, and only they, carry misses");
}

// ---- (d) normal operation never decays anyone ----

/// Finality succeeding every round means no stall, so even a validator that never votes keeps full
/// weight: the leak punishes stalls, not silence. RED if misses accrue outside a stall.
#[test]
fn normal_operation_never_decays_anyone() {
    let all: Vec<Validator> = (0..4).map(|id| val(id, 25.0, 25.0)).collect();
    let voters = &all[..3]; // id 3 never votes; 75% clears 2/3 and both floors
    let mut node = Node::new(0, all.clone(), leaking(0, 5));
    for _ in 0..100 {
        assert!(node.checkpoint_round(voters, &all));
        assert_eq!(node.vote_leak, VoteLeak::default(), "a non-voter was decayed while finality was live");
    }
}

// ---- (e) known risk: partitions ----

/// The same risk Ethereum's leak carries. Two halves (50% of each dimension each) split; each
/// replica sees only its own half vote. With no quorum floor, each side leaks the other and BOTH
/// finalize, on conflicting checkpoints. A quorum floor above 3/4 stops that (two disjoint sides
/// can both clear 2/3 of the floor only if Q <= 1/(2 * 2/3) = 3/4), while still letting the capture
/// scenario resume (honest 60% of base >= 2/3 * 76%). RED if the floor follows the leaked view.
#[test]
fn partition_leaks_both_sides_and_a_quorum_floor_above_three_quarters_stops_double_finality() {
    let all: Vec<Validator> = (0..4).map(|id| val(id, 25.0, 25.0)).collect();
    let (side_a, side_b) = (&all[..2], &all[2..]);

    let mut a = Node::new(0, all.clone(), leaking(2, 10));
    let mut b = Node::new(1, all.clone(), leaking(2, 10));
    let (fa, fb) = (first_final(&mut a, side_a, &all, 100), first_final(&mut b, side_b, &all, 100));
    assert!(fa.is_some() && fb.is_some(), "known risk: with no floor both sides of a partition finalize");

    let floored = Constitution { quorum_floor_bps: 7600, ..leaking(2, 10) };
    let mut a = Node::new(0, all.clone(), floored.clone());
    let mut b = Node::new(1, all.clone(), floored.clone());
    assert_eq!(first_final(&mut a, side_a, &all, 200), None, "floor > 3/4 must safe-halt side A");
    assert_eq!(first_final(&mut b, side_b, &all, 200), None, "floor > 3/4 must safe-halt side B");

    let (honest, cap_all) = capture_set();
    let mut node = Node::new(0, cap_all.clone(), floored);
    assert_eq!(first_final(&mut node, &honest, &cap_all, 200), Some(7), "the floor must not block the capture fix");
}

// ---- one axis never stands in for the other ----

/// All capital withholds. The leak drains the capital axis to zero in the view, where the plain
/// floor would read "dimension absent, cannot gate"; the registered-total floor keeps PoM from
/// finalizing alone. RED if the `co_signed` check is dropped from `finalizes_pos_pom_leaking`.
#[test]
fn leak_never_lets_one_axis_finalize_alone() {
    let pom_side = vec![val(0, 0.0, 50.0), val(1, 0.0, 50.0)];
    let capital = val(2, 100.0, 0.0);
    let all: Vec<Validator> = pom_side.iter().cloned().chain([capital]).collect();
    let mut node = Node::new(0, all.clone(), leaking(0, 4));
    assert_eq!(first_final(&mut node, &pom_side, &all, 200), None, "PoM finalized without any capital");
    assert_eq!(node.vote_leak.retention(2, 4), 0.0, "precondition: the capital axis is fully leaked");
}

/// The protocol cannot tell an honest veto from a captor's halt: both are "did not vote". So the
/// line is drawn by size, through `LEAK_DIM_FLOOR_BPS` (a third of a dimension's registered total
/// must still co-sign). Honest capital refusing a gamed PoM checkpoint keeps its veto when the
/// gamed side brings only a dust of capital, even with the quorum floor at 0 (where nothing else
/// would stop it, since PoM is 2/3 of the finality mix). A refusal of 60% of capital, between half
/// and two thirds, is treated as a halt and leaks out: round 7 with grace 2 and horizon 10, by the
/// same r <= 2/3 arithmetic as the capture test. A refusal of 70% keeps its veto. RED if
/// `LEAK_DIM_FLOOR_BPS` is lowered toward 0 (the dust case finalizes) or raised to half (the 60%
/// case never leaks).
#[test]
fn leak_cannot_tell_a_veto_from_a_halt_so_the_registered_floor_draws_the_line() {
    use noesis::runtime::finality::LEAK_DIM_FLOOR_BPS;
    assert_eq!(LEAK_DIM_FLOOR_BPS, 3333);
    let gamed_pom = [val(10, 0.0, 50.0), val(11, 0.0, 50.0)];
    let checkpoint = |capital_voting: &[Validator], capital_refusing: &[Validator]| {
        let voters: Vec<Validator> = gamed_pom.iter().chain(capital_voting).cloned().collect();
        let all: Vec<Validator> = voters.iter().chain(capital_refusing).cloned().collect();
        let mut node = Node::new(0, all.clone(), leaking(2, 10));
        first_final(&mut node, &voters, &all, 300)
    };
    assert_eq!(checkpoint(&[val(0, 1.0, 0.0)], &[val(1, 100.0, 0.0)]), None, "dust capital finalized a gamed PoM");
    assert_eq!(checkpoint(&[val(0, 40.0, 0.0)], &[val(1, 60.0, 0.0)]), Some(7), "a 60% refusal must leak out");
    assert_eq!(checkpoint(&[val(0, 30.0, 0.0)], &[val(1, 70.0, 0.0)]), None, "a 70% refusal must keep its veto");
}

// ---- rotation cannot hold the stall ----

/// One vote lowers a score by one; it does not wipe it. RED if `record` resets on a single vote.
#[test]
fn a_single_vote_does_not_reset_a_missed_score() {
    let all = vec![val(0, 50.0, 50.0), val(1, 50.0, 50.0)];
    let mut leak = VoteLeak::default();
    for _ in 0..5 {
        leak.record(&all, &all[..1], false, 0, 40);
    }
    assert_eq!(leak.missed.get(&1), Some(&5));
    leak.record(&all, &all, false, 0, 40);
    assert_eq!(leak.missed.get(&1), Some(&4), "one vote must take one miss off, not all of them");
}

/// Seven captors of 10% PoM take turns: exactly one votes per round (honest 30% PoM + one captor
/// stays under the PoM floor and the 2/3 bar). If any vote restored full weight, each captor would
/// reset every seventh round, carry scores 0..6 of a 40-round horizon, and hold the stall
/// indefinitely; the counterfactual below emulates that and holds 300 rounds. With one-step
/// recovery each captor nets +5 misses per seven rounds, so finality resumes. (Not necessarily
/// later than plain withholding: the rotating vote also counts toward the bar.) RED if `record`
/// resets a score on a single vote.
#[test]
fn leak_rotation_cannot_hold_the_stall() {
    let honest = vec![val(0, 50.0, 15.0), val(1, 50.0, 15.0)];
    let captors: Vec<Validator> = (2..9).map(|id| val(id, 0.0, 10.0)).collect();
    let all: Vec<Validator> = honest.iter().chain(captors.iter()).cloned().collect();
    let (grace, lh) = (2, 40);
    let voters_in = |round: u64| {
        let mut v = honest.clone();
        v.push(captors[(round % 7) as usize].clone());
        v
    };

    // counterfactual: instant recovery (the voter's score is wiped after each round).
    let mut instant = VoteLeak::default();
    for round in 1..=300u64 {
        let voters = voters_in(round);
        let ok = finalizes_pos_pom_leaking(&voters, &all, &instant, lh, 0, 0, false, TWO_THIRDS_BPS, 0);
        assert!(!ok, "precondition: instant recovery should let rotation hold the stall (round {round})");
        instant.record(&all, &voters, false, grace, lh);
        instant.missed.remove(&voters[2].id);
    }

    let mut node = Node::new(0, all.clone(), leaking(grace, lh));
    let rotated = (1..=300u64).find(|&r| node.checkpoint_round(&voters_in(r), &all));
    assert!(rotated.is_some(), "rotation held the stall under one-step recovery");
    println!("rotation: finality resumed at round {}", rotated.unwrap());
}
