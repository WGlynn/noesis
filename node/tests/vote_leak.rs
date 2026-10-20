//! Missed-finality-vote decay (the inactivity-leak analog): `runtime::finality::VoteLeak`,
//! `finalizes_pos_pom_leaking`, `Node::checkpoint_round`, `Constitution.{leak_grace, leak_horizon}`.
//!
//! The gap it closes: heartbeat decay keys to liveness, not to voting, so a group holding more than
//! a third of the combined finality weight that stays online and withholds its vote stalls finality
//! on the 2/3 bar indefinitely. The leak decays the 2/3 bar only. Ruling (Will 2026-10-20,
//! provisional pending peer feedback): it never reaches the per-dimension floor, so a refusal by more
//! than half of either dimension keeps its veto; finality stalls, block production continues, and
//! the remedy is the social fork in `SECURITY.md` section 5. Each test names the break that turns
//! it RED.

use noesis::commit_order::Committed;
use noesis::consensus::{Validator, TWO_THIRDS_BPS};
use noesis::runtime::finality::{finalizes_pos_pom, finalizes_pos_pom_leaking, VoteLeak};
use noesis::runtime::{Block, Constitution, Node};
use noesis::{Cell, Script};

fn val(id: u64, pos: f64, pom: f64) -> Validator {
    Validator { id, pow: 0.0, pos, pom, last_heartbeat: 0, staked_balance: pos }
}

fn leaking(grace: u64, leak_horizon: u64) -> Constitution {
    Constitution { leak_grace: grace, leak_horizon, ..Constitution::default() }
}

/// The case the leak fixes. Two withholders hold 40% of EACH dimension, stay online and never vote.
/// The voters hold 60% of each dimension (both floors pass) but only 60% of the combined weight, under
/// the 2/3 bar.
fn withholding_set() -> (Vec<Validator>, Vec<Validator>) {
    let voters = vec![val(0, 30.0, 30.0), val(1, 30.0, 30.0)];
    let all: Vec<Validator> = voters.iter().cloned().chain([val(2, 20.0, 20.0), val(3, 20.0, 20.0)]).collect();
    (voters, all)
}

/// The case the leak deliberately does NOT fix. Captors hold 60% of PoM and no capital; the voters
/// hold all the capital and 40% of PoM, so the PoM floor fails on raw weights.
fn capture_set() -> (Vec<Validator>, Vec<Validator>) {
    let voters = vec![val(0, 50.0, 20.0), val(1, 50.0, 20.0)];
    let all: Vec<Validator> = voters.iter().cloned().chain([val(2, 0.0, 30.0), val(3, 0.0, 30.0)]).collect();
    (voters, all)
}

/// Rounds until the first finalization, or None within `max`.
fn first_final(node: &mut Node, voters: &[Validator], all: &[Validator], max: u64) -> Option<u64> {
    (1..=max).find(|_| node.checkpoint_round(voters, all))
}

fn block_at(h: u64) -> Block {
    let cell = Cell {
        id: h,
        lock: Script { code_hash: [0u8; 32], args: b"owner".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: b"producer".to_vec() },
        parent: None,
        timestamp: h,
        data: format!("work produced at height {h} while finality is stalled").into_bytes(),
    };
    Block::assemble(h, &[(cell, Committed { height: h, secret: [h as u8; 32] })])
}

// ---- (a) inert by default ----

/// Default constitution: `checkpoint_round` must equal `checkpoint_finalizes` on every set and leave
/// `vote_leak` empty. With the leak ON but no scores yet, the leaking predicate must match
/// `finalizes_pos_pom` exactly, including under quorum floors (this also pins the lib.rs
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

// ---- (b) online withholders below half of each dimension: stall without the leak, resume with it ----

/// Without the leak the withholders stall finality forever. With grace 2 and leak horizon 10 the
/// first finalization is round 6: the voters need 60 >= 2/3 * (60 + 40r), i.e. r <= 0.75, i.e.
/// missed >= 2.5, so missed = 3; misses accrue from stalled round 3 (stall > grace), so round k
/// decides on k - 3 misses, giving k = 6. RED if the leak never fires, fires during grace, or decays
/// voters.
#[test]
fn online_withholders_stall_finality_without_the_leak_and_it_resumes_with_it() {
    let (voters, all) = withholding_set();

    let mut off = Node::new(0, all.clone(), Constitution::default());
    assert_eq!(first_final(&mut off, &voters, &all, 200), None, "precondition: the withholders stall finality");

    let mut node = Node::new(0, all.clone(), leaking(2, 10));
    let before = node.ledger.state_digest();
    assert_eq!(first_final(&mut node, &voters, &all, 200), Some(6));
    assert_eq!(node.vote_leak, VoteLeak::default(), "a finalized round must clear the stall and every score");
    assert_eq!(node.ledger.state_digest(), before, "the leak touched the ledger (credit/attribution)");
    for v in &all {
        assert_eq!(v.staked_balance, v.pos, "stake is never touched");
    }
}

// ---- the ruling: a dimension majority keeps its veto ----

/// Captors holding 60% of PoM stall finality whether they are offline (heartbeat decay) or online
/// and withholding (the leak): neither decay reaches the per-dimension floor, which reads raw
/// weights. RED if either decay starts relaxing `dim_ok` (then the 2026-10-20 ruling and
/// `SECURITY.md` section 5 must change with it).
#[test]
fn a_dimension_majority_keeps_its_veto_offline_or_online() {
    let (voters, all) = capture_set();
    let horizon = 4;
    let fresh: Vec<Validator> =
        voters.iter().map(|v| Validator { last_heartbeat: horizon, ..v.clone() }).collect();
    let offline: Vec<Validator> = all
        .iter()
        .map(|v| if v.id < 2 { Validator { last_heartbeat: horizon, ..v.clone() } } else { v.clone() })
        .collect();
    assert!(
        !finalizes_pos_pom(&fresh, &offline, horizon, horizon, true, TWO_THIRDS_BPS, 0),
        "offline captors fully decayed, yet the PoM floor must still hold"
    );

    let mut node = Node::new(0, all.clone(), leaking(0, 10));
    assert_eq!(first_final(&mut node, &voters, &all, 300), None, "the leak must not dissolve a dimension veto");
    assert_eq!(node.vote_leak.retention(2, 10), 0.0, "precondition: the captors are fully leaked on the 2/3 bar");
}

/// "Halt" in Noesis only ever means finality. While the capture above stalls finality, blocks keep
/// being validated and applied: production is never gated on finality. RED if any part of the leak
/// path starts gating `validate`/`apply`.
#[test]
fn block_production_continues_during_a_finality_stall() {
    let (voters, all) = capture_set();
    let mut node = Node::new(0, all.clone(), leaking(2, 10));
    for h in 1..=50u64 {
        assert!(!node.checkpoint_round(&voters, &all), "finality should stay stalled");
        let b = block_at(h);
        assert!(node.validate(&b), "production refused a valid block during a finality stall");
        node.apply(&b);
    }
    assert_eq!(node.ledger.height, 50, "the chain kept producing through 50 stalled finality rounds");
}

/// The veto is permanent however small the other side's co-signers are. Honest capital refusing a
/// gamed PoM checkpoint holds when the gamed side brings a dust of capital (the case that broke the
/// first version of this build, which relaxed the floor) and when it brings 40% of capital.
/// RED if the leak reaches the dimension floor.
#[test]
fn a_refusal_by_more_than_half_of_a_dimension_is_a_permanent_veto() {
    let gamed_pom = [val(10, 0.0, 50.0), val(11, 0.0, 50.0)];
    let checkpoint = |capital_voting: Validator, capital_refusing: Validator| {
        let voters: Vec<Validator> = gamed_pom.iter().cloned().chain([capital_voting]).collect();
        let all: Vec<Validator> = voters.iter().cloned().chain([capital_refusing]).collect();
        let mut node = Node::new(0, all.clone(), leaking(2, 10));
        first_final(&mut node, &voters, &all, 300)
    };
    assert_eq!(checkpoint(val(0, 1.0, 0.0), val(1, 100.0, 0.0)), None, "dust capital finalized a gamed PoM");
    assert_eq!(checkpoint(val(0, 40.0, 0.0), val(1, 60.0, 0.0)), None, "a 60% capital refusal lost its veto");
}

/// All capital withholds: PoM can't finalize alone, however long the leak runs.
#[test]
fn leak_never_lets_one_axis_finalize_alone() {
    let pom_side = vec![val(0, 0.0, 50.0), val(1, 0.0, 50.0)];
    let all: Vec<Validator> = pom_side.iter().cloned().chain([val(2, 100.0, 0.0)]).collect();
    let mut node = Node::new(0, all.clone(), leaking(0, 4));
    assert_eq!(first_final(&mut node, &pom_side, &all, 200), None, "PoM finalized without any capital");
}

// ---- (c) honest voters are never decayed ----

/// Every voter keeps retention exactly 1.0 and never appears in the score map through the stall.
/// RED if `record` scores voters or the decrement underflows into a score.
#[test]
fn honest_voters_are_never_decayed() {
    let (voters, all) = withholding_set();
    let mut node = Node::new(0, all.clone(), leaking(2, 10));
    for _ in 0..5 {
        assert!(!node.checkpoint_round(&voters, &all));
        for v in &voters {
            assert!(!node.vote_leak.missed.contains_key(&v.id), "a voter was scored");
            assert_eq!(node.vote_leak.retention(v.id, 10), 1.0);
        }
    }
    assert_eq!(node.vote_leak.missed.get(&2), Some(&3), "the withholders, and only they, carry misses");
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

/// The same risk Ethereum's leak carries, narrowed by the ruling. Each side of a split sees the other
/// as not voting and leaks it on the 2/3 bar. Because the dimension floor reads raw weights, a side
/// must hold at least half of EACH dimension to finalize, so two disjoint sides can both finalize
/// only on the knife edge of an exact half split in both dimensions. An uneven split finalizes on the
/// majority side only. A quorum floor above 3/4 removes even the knife edge (two sides can both clear
/// 2/3 of the floor only if Q <= 3/4) without blocking the withholding fix. RED if the floor follows
/// the leaked view, or if the leak reaches the dimension floor.
#[test]
fn partition_double_finality_needs_an_exact_half_split_and_a_floor_above_three_quarters_removes_it() {
    let even: Vec<Validator> = (0..4).map(|id| val(id, 25.0, 25.0)).collect();
    let mut a = Node::new(0, even.clone(), leaking(2, 10));
    let mut b = Node::new(1, even.clone(), leaking(2, 10));
    let (fa, fb) = (first_final(&mut a, &even[..2], &even, 100), first_final(&mut b, &even[2..], &even, 100));
    assert!(fa.is_some() && fb.is_some(), "knife edge: an exact half split finalizes on both sides with no floor");

    let uneven = vec![val(0, 26.0, 26.0), val(1, 26.0, 26.0), val(2, 24.0, 24.0), val(3, 24.0, 24.0)];
    let mut a = Node::new(0, uneven.clone(), leaking(2, 10));
    let mut b = Node::new(1, uneven.clone(), leaking(2, 10));
    assert!(first_final(&mut a, &uneven[..2], &uneven, 100).is_some(), "the 52% side should resume");
    assert_eq!(first_final(&mut b, &uneven[2..], &uneven, 300), None, "the 48% side must not finalize");

    let floored = Constitution { quorum_floor_bps: 7600, ..leaking(2, 10) };
    let mut a = Node::new(0, even.clone(), floored.clone());
    let mut b = Node::new(1, even.clone(), floored.clone());
    assert_eq!(first_final(&mut a, &even[..2], &even, 200), None, "floor > 3/4 must stall side A");
    assert_eq!(first_final(&mut b, &even[2..], &even, 200), None, "floor > 3/4 must stall side B");

    let (voters, all) = withholding_set();
    let mut node = Node::new(0, all.clone(), floored);
    assert_eq!(first_final(&mut node, &voters, &all, 200), Some(6), "the floor must not block the withholding fix");
}

// ---- recovery and rotation ----

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

/// Nine withholders of 5% each (45% of each dimension) take turns: exactly one votes per round, so the
/// voters hold 60% of the combined weight, under the 2/3 bar, while both dimension floors pass. If any
/// vote restored full weight, each withholder would reset every ninth round, carry scores 0..8 of a
/// 40-round horizon, and hold the stall indefinitely; the counterfactual below emulates that and holds
/// 300 rounds. With one-step recovery each nets +7 misses per nine rounds, so finality resumes.
/// RED if `record` resets a score on a single vote.
#[test]
fn leak_rotation_cannot_hold_the_stall() {
    let honest = vec![val(0, 27.5, 27.5), val(1, 27.5, 27.5)];
    let rotating: Vec<Validator> = (2..11).map(|id| val(id, 5.0, 5.0)).collect();
    let all: Vec<Validator> = honest.iter().chain(rotating.iter()).cloned().collect();
    let (grace, lh) = (2, 40);
    let voters_in = |round: u64| {
        let mut v = honest.clone();
        v.push(rotating[(round % 9) as usize].clone());
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
