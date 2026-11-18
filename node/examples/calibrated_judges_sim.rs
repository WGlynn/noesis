//! Calibrated judges: does weighting value judges by their calibration track record make a wash ring's
//! influence decay over rounds, or hold? Pre-registered 2026-11-18 in
//! `docs/research/calibrated-judges-2026-11-18.md` (pre-registration commit 50ef8b86, before any data).
//!
//! WHY. Judges of value are weighted by standing, and standing is what the value layer issues: a circle. A
//! circle has more than one resting point (an honest community, or a ring valuing its own members), and
//! `wash_sim.rs` found no worth-blind graph signal that tells them apart. The proposal tested here: weight
//! each judge by how well its past valuations predicted later independent use, so influence flows to
//! whoever keeps predicting the outside world.
//!
//! WHAT THIS IS. A model of judges, weights and minting, not of the node: no cells, novelty, `v(S)` or
//! vesting. Calibration-weighted judging is designed, not built; nothing in `node/src` computes it. The
//! only thing read from the library is `consensus::TWO_THIRDS_BPS`, for the dispute-style aggregator (a
//! reported variant). It touches no consensus code.
//!
//! Arms: A standing alone (today's circle), B calibration alone, C standing x calibration.
//! Adversaries: plain ring, selective ring, patient ring, bought anchor (fraction f of the use signal).
//! Every number below is fixed by the pre-registration; see the doc for the reasons.
//!
//! Run, in a separate target directory so the live node's binary is never touched:
//! `CARGO_TARGET_DIR=target/calib-sim cargo run --release -p noesis --example calibrated_judges_sim`

use noesis::consensus::TWO_THIRDS_BPS;
use std::collections::VecDeque;
use std::time::Instant;

// ============ Pre-registered parameters ============

const N: usize = 100; // identities; each is a contributor and a judge
const T: usize = 60; // rounds
const BUDGET: f64 = 10.0; // minted per round (10% of the starting total standing of 100)
const V_MAX: f64 = 3.0; // report bound, three times the honest mean worth
const SIGMA_J: f64 = 0.3; // honest judge noise
const SIGMA_U: f64 = 0.3; // independent-use noise
const DELAY: usize = 3; // items of round t are revealed at the end of round t + 3
const SEEDS: usize = 32;
const ETA: f64 = 5.0; // exponential-weights rate
const GAMMA: f64 = 0.9; // per-reveal decay of the calibration loss
const DEFECT_AFTER: usize = 20; // patient ring behaves through round 20
const SHARES: [f64; 4] = [0.10, 0.30, 0.49, 0.60];
const DELTAS: [f64; 6] = [0.1, 0.25, 0.5, 1.0, 2.0, 3.0];
const FS: [f64; 10] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.45, 0.5, 0.55, 0.6, 0.8];
const REFUTE_BELOW: f64 = 0.5; // dispute variant: a judge votes to refute iff its report is below this
const CLAIMED: f64 = 1.0; // dispute variant: the worth-blind claimed value of every item
const REPORT_ROUNDS: [usize; 8] = [1, 5, 10, 20, 30, 40, 50, 60];
const WORKERS: usize = 2; // at most two threads on this shared machine
const ARM_NAMES: [&str; 3] = ["A standing", "B calibration", "C standing x calib"];

// ============ RNG (splitmix64 + Box-Muller), seeded from the seed index only ============

struct Rng {
    s: u64,
    spare: Option<f64>,
}
impl Rng {
    fn new(seed_idx: u64) -> Self {
        Rng {
            s: seed_idx.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            spare: None,
        }
    }
    fn next_u64(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    fn normal(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let u1 = self.uniform().max(1e-300);
        let u2 = self.uniform();
        let r = (-2.0 * u1.ln()).sqrt();
        let th = 2.0 * std::f64::consts::PI * u2;
        self.spare = Some(r * th.sin());
        r * th.cos()
    }
}

/// Every random number a run uses, drawn once per seed in the registered order (per round, per item:
/// worth uniform, use noise, one noise per judge), so every configuration sees the same stream.
struct Draws {
    worth_u: Vec<f64>,
    use_n: Vec<f64>,
    judge_e: Vec<f64>,
}
fn draws_for(seed_idx: usize) -> Draws {
    let mut rng = Rng::new(seed_idx as u64);
    let mut d = Draws {
        worth_u: Vec::with_capacity(T * N),
        use_n: Vec::with_capacity(T * N),
        judge_e: Vec::with_capacity(T * N * N),
    };
    for _t in 0..T {
        for _c in 0..N {
            d.worth_u.push(rng.uniform());
            d.use_n.push(rng.normal());
            for _j in 0..N {
                d.judge_e.push(rng.normal());
            }
        }
    }
    d
}

// ============ Configuration ============

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Adv {
    Plain,
    Selective,
    Patient,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Agg {
    Mean,
    Median,
    Dispute,
}
#[derive(Clone, Copy, Debug)]
struct Cell {
    adv: Adv,
    share: f64,
    f: f64,
    agg: Agg,
    eta: f64,
    gamma: f64,
    concentrated: bool,
}
impl Cell {
    fn base(adv: Adv, share: f64) -> Self {
        Cell {
            adv,
            share,
            f: 0.0,
            agg: Agg::Mean,
            eta: ETA,
            gamma: GAMMA,
            concentrated: false,
        }
    }
}

// ============ One run ============

#[derive(Clone)]
struct RunOut {
    infl: [[f64; T]; 3], // ring share of judge weight per round, per arm
    m: [f64; 3],         // ring share of value minted in the scoring window, per arm
    tv: [f64; 3],        // mean total-variation misallocation over rounds in the window that minted
}

fn aggregate(agg: Agg, r: &[f64; N], w: &[f64; N], thr: f64, scratch: &mut Vec<(f64, f64)>) -> f64 {
    match agg {
        Agg::Mean => r.iter().zip(w.iter()).map(|(a, b)| a * b).sum::<f64>().max(0.0),
        Agg::Median => {
            scratch.clear();
            scratch.extend(r.iter().copied().zip(w.iter().copied()));
            scratch.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let mut cum = 0.0;
            for &(v, wt) in scratch.iter() {
                cum += wt;
                if cum >= 0.5 {
                    return v.max(0.0);
                }
            }
            scratch.last().map(|p| p.0).unwrap_or(0.0).max(0.0)
        }
        Agg::Dispute => {
            let refute: f64 = r
                .iter()
                .zip(w.iter())
                .filter(|(v, _)| **v < REFUTE_BELOW)
                .map(|(_, wt)| wt)
                .sum();
            if refute >= thr {
                0.0
            } else {
                CLAIMED
            }
        }
    }
}

fn run(cell: &Cell, delta: f64, d: &Draws) -> RunOut {
    let n_r = if cell.concentrated {
        10
    } else {
        (cell.share * N as f64).round() as usize
    };
    let ring = |i: usize| i < n_r;
    // Standing per identity for the two arms that read it: index 0 = arm A, 1 = arm C.
    let mut st = [[0.0f64; N]; 2];
    for i in 0..N {
        let s0 = if cell.concentrated {
            if ring(i) {
                cell.share * N as f64 / n_r as f64
            } else {
                (1.0 - cell.share) * N as f64 / (N - n_r) as f64
            }
        } else {
            1.0
        };
        st[0][i] = s0;
        st[1][i] = s0;
    }
    let thr = TWO_THIRDS_BPS as f64 / 10_000.0;
    let win_start = if cell.adv == Adv::Patient {
        DEFECT_AFTER + 1
    } else {
        1
    };

    let mut loss = [0.0f64; N]; // decayed calibration loss, shared by arms B and C (reports do not depend on weights)
    let mut pending: VecDeque<[f64; N]> = VecDeque::new();
    let mut out = RunOut {
        infl: [[0.0; T]; 3],
        m: [0.0; 3],
        tv: [0.0; 3],
    };
    let mut ring_mint = [0.0f64; 3];
    let mut tot_mint = [0.0f64; 3];
    let mut tv_sum = [0.0f64; 3];
    let mut tv_n = [0usize; 3];
    let mut w = [[0.0f64; N]; 3];
    let mut agg = [[0.0f64; N]; 3];
    let mut worth = [0.0f64; N];
    let mut reports = [0.0f64; N];
    let mut scratch: Vec<(f64, f64)> = Vec::with_capacity(N);

    for t in 1..=T {
        // ---- weights at the start of the round ----
        let lmin = loss.iter().cloned().fold(f64::INFINITY, f64::min);
        for j in 0..N {
            let k = (-cell.eta * (loss[j] - lmin)).exp();
            w[0][j] = st[0][j];
            w[1][j] = k;
            w[2][j] = st[1][j] * k;
        }
        for a in 0..3 {
            let s: f64 = w[a].iter().sum();
            for j in 0..N {
                w[a][j] /= s;
            }
            out.infl[a][t - 1] = (0..n_r).map(|j| w[a][j]).sum();
        }

        let honest_mode = cell.adv == Adv::Patient && t <= DEFECT_AFTER;
        let plain_mode = !honest_mode && matches!(cell.adv, Adv::Plain | Adv::Patient);

        // ---- items, reports, use, aggregates ----
        let mut batch = [0.0f64; N];
        for c in 0..N {
            let base = (t - 1) * N + c;
            let ring_item = ring(c);
            let wash = ring_item && !honest_mode;
            let wv = if wash { 0.0 } else { 0.5 + d.worth_u[base] };
            worth[c] = wv;
            let honest_use = wv + SIGMA_U * d.use_n[base];
            // The ring's bought fraction of use mirrors its reports: Delta on its own items, 0 on others
            // (plain), honest on others (selective).
            let ring_use = if honest_mode {
                honest_use
            } else if ring_item {
                delta
            } else if plain_mode {
                0.0
            } else {
                honest_use
            };
            let u = (1.0 - cell.f) * honest_use + cell.f * ring_use;
            let e = &d.judge_e[base * N..base * N + N];
            for j in 0..N {
                let honest_r = (wv + SIGMA_J * e[j]).clamp(-V_MAX, V_MAX);
                reports[j] = if !ring(j) || honest_mode {
                    honest_r
                } else if ring_item {
                    delta
                } else if plain_mode {
                    0.0
                } else {
                    honest_r
                };
                let err = reports[j] - u;
                batch[j] += err * err;
            }
            for a in 0..3 {
                agg[a][c] = aggregate(cell.agg, &reports, &w[a], thr, &mut scratch);
            }
        }
        for b in batch.iter_mut() {
            *b /= N as f64;
        }

        // ---- mint ----
        let wsum: f64 = worth.iter().sum();
        for a in 0..3 {
            let s: f64 = agg[a].iter().sum();
            if s <= 0.0 {
                continue;
            }
            let mut tv = 0.0;
            for c in 0..N {
                let m = BUDGET * agg[a][c] / s;
                if a == 0 {
                    st[0][c] += m;
                } else if a == 2 {
                    st[1][c] += m;
                }
                if t >= win_start {
                    if ring(c) {
                        ring_mint[a] += m;
                    }
                    tot_mint[a] += m;
                }
                tv += (agg[a][c] / s - worth[c] / wsum).abs();
            }
            if t >= win_start {
                tv_sum[a] += 0.5 * tv;
                tv_n[a] += 1;
            }
        }

        // ---- reveal the batch from three rounds back ----
        pending.push_back(batch);
        if t > DELAY {
            let b = pending.pop_front().unwrap();
            for j in 0..N {
                loss[j] = cell.gamma * loss[j] + b[j];
            }
        }
    }
    for a in 0..3 {
        out.m[a] = if tot_mint[a] > 0.0 {
            ring_mint[a] / tot_mint[a]
        } else {
            0.0
        };
        out.tv[a] = if tv_n[a] > 0 {
            tv_sum[a] / tv_n[a] as f64
        } else {
            f64::NAN
        };
    }
    out
}

// ============ Statistics and verdicts ============

#[derive(Clone, Copy, Debug)]
struct Stat {
    mean: f64,
    se: f64,
}
fn stat(xs: &[f64]) -> Stat {
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    Stat {
        mean,
        se: (var / n).sqrt(),
    }
}

/// One arm of one cell at the ring's best response (the Delta that maximizes its mean mint share).
struct ArmSum {
    delta: f64,
    infl: [f64; T],
    i_end: Stat,
    m: Stat,
    m_seeds: Vec<f64>,
    tv: Stat,
}
fn arm_summary(res: &[Vec<RunOut>], arm: usize) -> ArmSum {
    // res[delta_idx][seed]
    let mut best = 0;
    let mut best_m = f64::NEG_INFINITY;
    for (di, runs) in res.iter().enumerate() {
        let m = runs.iter().map(|r| r.m[arm]).sum::<f64>() / runs.len() as f64;
        if m > best_m + 1e-12 {
            best_m = m;
            best = di;
        }
    }
    let runs = &res[best];
    let mut infl = [0.0; T];
    for r in runs {
        for t in 0..T {
            infl[t] += r.infl[arm][t] / runs.len() as f64;
        }
    }
    let i_end_seeds: Vec<f64> = runs
        .iter()
        .map(|r| r.infl[arm][T - 10..].iter().sum::<f64>() / 10.0)
        .collect();
    let m_seeds: Vec<f64> = runs.iter().map(|r| r.m[arm]).collect();
    let tv_seeds: Vec<f64> = runs.iter().map(|r| r.tv[arm]).filter(|x| x.is_finite()).collect();
    ArmSum {
        delta: DELTAS[best],
        infl,
        i_end: stat(&i_end_seeds),
        m: stat(&m_seeds),
        m_seeds,
        tv: if tv_seeds.is_empty() {
            Stat {
                mean: f64::NAN,
                se: f64::NAN,
            }
        } else {
            stat(&tv_seeds)
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outcome {
    Works,
    Partial,
    Fails,
}
struct CellJudgement {
    h1: bool,
    h2: bool,
    outcome: Outcome,
    undecided: bool,
    worse_than_a: bool, // M(X) > M(A) + 2 SE, paired
}
fn judge(share: f64, a: &ArmSum, x: &ArmSum) -> CellJudgement {
    let bar = 0.5 * share;
    let h1 = x.i_end.mean <= bar;
    let h1_undecided = (x.i_end.mean - bar).abs() < 2.0 * x.i_end.se;
    let diff: Vec<f64> = a.m_seeds.iter().zip(&x.m_seeds).map(|(ma, mx)| ma - mx).collect();
    let ds = stat(&diff);
    let half: Vec<f64> = a
        .m_seeds
        .iter()
        .zip(&x.m_seeds)
        .map(|(ma, mx)| 0.5 * ma - mx)
        .collect();
    let hs = stat(&half);
    let c1 = x.m.mean <= 0.5 * a.m.mean;
    let c2 = ds.mean > 2.0 * ds.se;
    let c3 = x.m.mean < 0.005;
    let h2 = (c1 && c2) || c3;
    // H2 decided true: c3 holds by 2 SE, or c1 holds by 2 SE together with c2. Decided false: c3 fails by
    // 2 SE and (c2 fails, or c1 fails by 2 SE). Anything else is undecided.
    let c3_sure = x.m.mean + 2.0 * x.m.se < 0.005;
    let c3_sure_not = x.m.mean - 2.0 * x.m.se >= 0.005;
    let c1_sure = hs.mean - 2.0 * hs.se >= 0.0;
    let c1_sure_not = hs.mean + 2.0 * hs.se < 0.0;
    let h2_decided = c3_sure || (c1_sure && c2) || (c3_sure_not && (!c2 || c1_sure_not));
    let outcome = match (h1, h2) {
        (true, true) => Outcome::Works,
        (false, false) => Outcome::Fails,
        _ => Outcome::Partial,
    };
    let worse_than_a = -ds.mean > 2.0 * ds.se;
    CellJudgement {
        h1,
        h2,
        outcome,
        undecided: h1_undecided || !h2_decided,
        worse_than_a,
    }
}

fn verdict(js: &[CellJudgement]) -> &'static str {
    let undecided = js.iter().filter(|j| j.undecided).count();
    if undecided >= 2 {
        "INCONCLUSIVE"
    } else if js.iter().all(|j| j.outcome == Outcome::Works) {
        "CONFIRMED"
    } else if js.iter().all(|j| j.outcome == Outcome::Fails) {
        "REFUTED"
    } else {
        "BOUNDARY"
    }
}

fn oc(o: Outcome) -> &'static str {
    match o {
        Outcome::Works => "WORKS",
        Outcome::Partial => "PARTIAL",
        Outcome::Fails => "FAILS",
    }
}

// ============ Printing ============

fn print_cell_table(title: &str, cells: &[(f64, [ArmSum; 3])], patient: bool) -> [Vec<CellJudgement>; 2] {
    println!("\n### {title}");
    let mut js: [Vec<CellJudgement>; 2] = [Vec::new(), Vec::new()];
    for (share, arms) in cells {
        println!("\n  ring share s = {:.2}", share);
        print!("    arm                 | Delta* |");
        for r in REPORT_ROUNDS {
            print!("  I({:>2}) |", r);
        }
        println!("  I_end (+-SE)    |  M ring mint (+-SE) | honest share | misalloc TV");
        for (a, s) in arms.iter().enumerate() {
            print!("    {:<19} | {:>6.2} |", ARM_NAMES[a], s.delta);
            for r in REPORT_ROUNDS {
                print!(" {:>6.3} |", s.infl[r - 1]);
            }
            println!(
                " {:.3} +- {:.3} | {:.4} +- {:.4}    | {:>12.4} | {:.4}",
                s.i_end.mean,
                s.i_end.se,
                s.m.mean,
                s.m.se,
                1.0 - s.m.mean,
                s.tv.mean
            );
        }
        if patient {
            for (a, s) in arms.iter().enumerate() {
                let i20 = s.infl[DEFECT_AFTER - 1];
                let half = (DEFECT_AFTER..T)
                    .find(|&t| s.infl[t] <= 0.5 * i20)
                    .map(|t| t + 1 - DEFECT_AFTER);
                println!(
                    "    {:<19} I(20) = {:.3}; rounds after defection until mean I(t) <= half of I(20): {}",
                    ARM_NAMES[a],
                    i20,
                    half.map(|h| h.to_string())
                        .unwrap_or_else(|| "not within the horizon".into())
                );
            }
        }
        for x in 1..3 {
            let j = judge(*share, &arms[0], &arms[x]);
            println!(
                "    {} vs A: H1 (I_end <= {:.3}) {} | H2 (M <= half of A's, or < 0.005) {} | {}{}{}",
                &ARM_NAMES[x][..1],
                0.5 * share,
                if j.h1 { "holds" } else { "fails" },
                if j.h2 { "holds" } else { "fails" },
                oc(j.outcome),
                if j.undecided {
                    " (UNDECIDED within 2 SE)"
                } else {
                    ""
                },
                if j.worse_than_a {
                    " [worse than A by > 2 SE]"
                } else {
                    ""
                }
            );
            js[x - 1].push(j);
        }
    }
    js
}

fn summaries(res: &[Vec<RunOut>]) -> [ArmSum; 3] {
    [arm_summary(res, 0), arm_summary(res, 1), arm_summary(res, 2)]
}

fn main() {
    let t0 = Instant::now();

    // ---- the cells, in registered groups ----
    let mut cells: Vec<Cell> = Vec::new();
    let mut idx_plain_f = vec![[0usize; FS.len()]; SHARES.len()]; // adversaries 1 (f = 0) and 4
    let mut idx_sel = [0usize; 4];
    let mut idx_pat = [0usize; 4];
    let mut idx_r1 = Vec::new(); // (agg, adv, share_idx, cell)
    let mut idx_r2 = Vec::new(); // (label, share_idx, cell)
    let mut idx_r3 = [0usize; 4];
    let mut idx_r4 = vec![[0usize; FS.len()]; SHARES.len()];
    for (si, &s) in SHARES.iter().enumerate() {
        for (fi, &f) in FS.iter().enumerate() {
            idx_plain_f[si][fi] = cells.len();
            cells.push(Cell {
                f,
                ..Cell::base(Adv::Plain, s)
            });
        }
        idx_sel[si] = cells.len();
        cells.push(Cell::base(Adv::Selective, s));
        idx_pat[si] = cells.len();
        cells.push(Cell::base(Adv::Patient, s));
        for agg in [Agg::Median, Agg::Dispute] {
            for adv in [Adv::Plain, Adv::Selective, Adv::Patient] {
                idx_r1.push((agg, adv, si, cells.len()));
                cells.push(Cell {
                    agg,
                    ..Cell::base(adv, s)
                });
            }
        }
        for (label, eta, gamma) in [
            ("eta 1, gamma 0.9", 1.0, GAMMA),
            ("eta 20, gamma 0.9", 20.0, GAMMA),
            ("eta 5, gamma 1.0", ETA, 1.0),
        ] {
            idx_r2.push((label, si, cells.len()));
            cells.push(Cell {
                eta,
                gamma,
                ..Cell::base(Adv::Selective, s)
            });
        }
        idx_r3[si] = cells.len();
        cells.push(Cell {
            concentrated: true,
            ..Cell::base(Adv::Selective, s)
        });
        for (fi, &f) in FS.iter().enumerate() {
            if f == 0.0 {
                idx_r4[si][fi] = idx_sel[si];
            } else {
                idx_r4[si][fi] = cells.len();
                cells.push(Cell {
                    f,
                    ..Cell::base(Adv::Selective, s)
                });
            }
        }
    }

    println!("== Calibrated judges (pre-registered 2026-11-18, commit 50ef8b86) ==");
    println!(
        "   N {N}, rounds {T}, budget {BUDGET}/round, report cap {V_MAX}, sigma_J {SIGMA_J}, sigma_U {SIGMA_U}, reveal delay {DELAY}, \
         eta {ETA}, gamma {GAMMA}, seeds {SEEDS}"
    );
    println!(
        "   TWO_THIRDS_BPS read from noesis::consensus = {} (dispute variant threshold {:.4})",
        TWO_THIRDS_BPS,
        TWO_THIRDS_BPS as f64 / 10_000.0
    );
    println!(
        "   cells {}, Delta grid {}, runs {}",
        cells.len(),
        DELTAS.len(),
        cells.len() * DELTAS.len() * SEEDS
    );

    // ---- run: seeds split across WORKERS threads; results land by index, so output is deterministic ----
    // res[cell][delta][seed]
    let mut res: Vec<Vec<Vec<Option<RunOut>>>> = vec![vec![vec![None; SEEDS]; DELTAS.len()]; cells.len()];
    let per_worker: Vec<Vec<(usize, Vec<Vec<RunOut>>)>> = std::thread::scope(|sc| {
        let handles: Vec<_> = (0..WORKERS)
            .map(|wk| {
                let cells = &cells;
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for seed in (wk..SEEDS).step_by(WORKERS) {
                        let d = draws_for(seed);
                        let per_cell: Vec<Vec<RunOut>> = cells
                            .iter()
                            .map(|c| DELTAS.iter().map(|&dl| run(c, dl, &d)).collect())
                            .collect();
                        out.push((seed, per_cell));
                    }
                    out
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for wk in per_worker {
        for (seed, per_cell) in wk {
            for (ci, per_delta) in per_cell.into_iter().enumerate() {
                for (di, r) in per_delta.into_iter().enumerate() {
                    res[ci][di][seed] = Some(r);
                }
            }
        }
    }
    let res: Vec<Vec<Vec<RunOut>>> = res
        .into_iter()
        .map(|pd| {
            pd.into_iter()
                .map(|ps| ps.into_iter().map(|r| r.unwrap()).collect())
                .collect()
        })
        .collect();
    let run_secs = t0.elapsed().as_secs_f64();

    println!("\nReading the tables: I(t) = ring share of judge weight in round t (mean over seeds); I_end = mean over");
    println!("rounds 51-60; M = ring share of value minted in the scoring window (every ring item there is wash, so the");
    println!("honest contributors' share is 1 - M); TV = mean total-variation distance between minted shares and true-worth");
    println!("shares. Each arm faces the ring's best response for that arm (Delta*, the value it reports on its own items).");

    // ---- adversaries 1-3 ----
    let mut verdict_lines = Vec::new();
    let groups: [(&str, Vec<usize>, bool); 3] = [
        (
            "Adversary 1: plain ring (Delta on own items, 0 on others)",
            (0..4).map(|si| idx_plain_f[si][0]).collect(),
            false,
        ),
        (
            "Adversary 2: selective ring (Delta on own items, honest on others)",
            idx_sel.to_vec(),
            false,
        ),
        (
            "Adversary 3: patient ring (honest rounds 1-20, then plain; window rounds 21-60)",
            idx_pat.to_vec(),
            true,
        ),
    ];
    for (title, idxs, patient) in groups.iter() {
        let sums: Vec<(f64, [ArmSum; 3])> = idxs
            .iter()
            .enumerate()
            .map(|(si, &ci)| (SHARES[si], summaries(&res[ci])))
            .collect();
        let js = print_cell_table(title, &sums, *patient);
        let name = title.split(':').next().unwrap();
        for x in 0..2 {
            let cells_str: Vec<String> = js[x]
                .iter()
                .enumerate()
                .map(|(si, j)| {
                    format!(
                        "s={:.2} {}{}",
                        SHARES[si],
                        oc(j.outcome),
                        if j.undecided { "?" } else { "" }
                    )
                })
                .collect();
            verdict_lines.push(format!(
                "{name}, arm {}: {}  [{}]",
                &ARM_NAMES[x + 1][..1],
                verdict(&js[x]),
                cells_str.join(", ")
            ));
        }
    }

    // ---- adversary 4: bought anchor on the plain base, and R4 on the selective base ----
    let mut f_lines = Vec::new();
    for (label, grid, is_primary) in [
        (
            "Adversary 4: bought anchor, plain base (ring controls fraction f of the use signal)",
            &idx_plain_f,
            true,
        ),
        (
            "R4 (reported, not decisive): bought anchor, selective base",
            &idx_r4,
            false,
        ),
    ] {
        println!("\n### {label}");
        let mut h4_cells: [Vec<(Option<f64>, Option<f64>, bool)>; 2] = [Vec::new(), Vec::new()];
        for (si, &s) in SHARES.iter().enumerate() {
            println!("\n  ring share s = {:.2}", s);
            println!("      f   | A: Delta*  I_end   M      | B: Delta*  I_end   M      outcome   | C: Delta*  I_end   M      outcome");
            let mut fb: [Option<f64>; 2] = [None, None];
            let mut ff: [Option<f64>; 2] = [None, None];
            let mut und_at_break = [false; 2];
            let mut prev_und = [false; 2];
            for (fi, &f) in FS.iter().enumerate() {
                let sm = summaries(&res[grid[si][fi]]);
                let jb = judge(s, &sm[0], &sm[1]);
                let jc = judge(s, &sm[0], &sm[2]);
                println!(
                    "    {:>5.2} | {:>5.2}  {:.3}  {:.4}  | {:>5.2}  {:.3}  {:.4}  {:<8}{}{} | {:>5.2}  {:.3}  {:.4}  {:<8}{}{}",
                    f,
                    sm[0].delta,
                    sm[0].i_end.mean,
                    sm[0].m.mean,
                    sm[1].delta,
                    sm[1].i_end.mean,
                    sm[1].m.mean,
                    oc(jb.outcome),
                    if jb.undecided { "?" } else { " " },
                    if jb.worse_than_a { "!" } else { " " },
                    sm[2].delta,
                    sm[2].i_end.mean,
                    sm[2].m.mean,
                    oc(jc.outcome),
                    if jc.undecided { "?" } else { " " },
                    if jc.worse_than_a { "!" } else { " " },
                );
                for (x, j) in [jb, jc].iter().enumerate() {
                    if fb[x].is_none() && j.outcome != Outcome::Works {
                        fb[x] = Some(f);
                        und_at_break[x] = j.undecided || prev_und[x];
                    }
                    if ff[x].is_none() && j.worse_than_a {
                        ff[x] = Some(f);
                    }
                    prev_und[x] = j.undecided;
                }
            }
            for x in 0..2 {
                println!(
                    "    arm {}: f_break = {}, f_flip = {}",
                    &ARM_NAMES[x + 1][..1],
                    fb[x]
                        .map(|v| format!("{v:.2}"))
                        .unwrap_or_else(|| "none up to 0.8".into()),
                    ff[x]
                        .map(|v| format!("{v:.2}"))
                        .unwrap_or_else(|| "none up to 0.8".into())
                );
                h4_cells[x].push((fb[x], ff[x], und_at_break[x]));
            }
        }
        for x in 0..2 {
            let v = &h4_cells[x];
            let verdict = if v.iter().any(|c| c.0.is_none()) {
                "REFUTED"
            } else if v.iter().filter(|c| c.2).count() >= 2 {
                "INCONCLUSIVE"
            } else if v
                .iter()
                .all(|c| c.0.map(|f| (0.4..=0.55).contains(&f)).unwrap_or(false))
            {
                "CONFIRMED"
            } else {
                "BOUNDARY"
            };
            let detail: Vec<String> = v
                .iter()
                .enumerate()
                .map(|(si, c)| {
                    format!(
                        "s={:.2} f_break {} f_flip {}",
                        SHARES[si],
                        c.0.map(|f| format!("{f:.2}")).unwrap_or_else(|| "-".into()),
                        c.1.map(|f| format!("{f:.2}")).unwrap_or_else(|| "-".into())
                    )
                })
                .collect();
            let line = format!(
                "{}, arm {}: H4 {}  [{}]",
                label.split(':').next().unwrap(),
                &ARM_NAMES[x + 1][..1],
                verdict,
                detail.join("; ")
            );
            if is_primary {
                verdict_lines.push(line);
            } else {
                f_lines.push(line);
            }
        }
    }

    // ---- R1: other aggregators ----
    println!("\n### R1 (reported, not decisive): other aggregators");
    for agg in [Agg::Median, Agg::Dispute] {
        for adv in [Adv::Plain, Adv::Selective, Adv::Patient] {
            println!("\n  {:?} aggregator, {:?} ring", agg, adv);
            println!("      s    | A: Delta*  I_end   M      | B: Delta*  I_end   M      outcome   | C: Delta*  I_end   M      outcome");
            for &(ag, ad, si, ci) in idx_r1.iter() {
                if ag != agg || ad != adv {
                    continue;
                }
                let sm = summaries(&res[ci]);
                let jb = judge(SHARES[si], &sm[0], &sm[1]);
                let jc = judge(SHARES[si], &sm[0], &sm[2]);
                println!(
                    "    {:>5.2} | {:>5.2}  {:.3}  {:.4}  | {:>5.2}  {:.3}  {:.4}  {:<8}{}{} | {:>5.2}  {:.3}  {:.4}  {:<8}{}{}",
                    SHARES[si],
                    sm[0].delta,
                    sm[0].i_end.mean,
                    sm[0].m.mean,
                    sm[1].delta,
                    sm[1].i_end.mean,
                    sm[1].m.mean,
                    oc(jb.outcome),
                    if jb.undecided { "?" } else { " " },
                    if jb.worse_than_a { "!" } else { " " },
                    sm[2].delta,
                    sm[2].i_end.mean,
                    sm[2].m.mean,
                    oc(jc.outcome),
                    if jc.undecided { "?" } else { " " },
                    if jc.worse_than_a { "!" } else { " " },
                );
            }
        }
    }

    // ---- R2: calibration settings, selective ring ----
    println!("\n### R2 (reported, not decisive): calibration settings, selective ring");
    println!("    setting            |   s   | A: Delta*  I_end   M      | B: Delta*  I_end   M      outcome   | C: Delta*  I_end   M      outcome");
    for &(label, si, ci) in idx_r2.iter() {
        let sm = summaries(&res[ci]);
        let jb = judge(SHARES[si], &sm[0], &sm[1]);
        let jc = judge(SHARES[si], &sm[0], &sm[2]);
        println!(
            "    {:<18} | {:>5.2} | {:>5.2}  {:.3}  {:.4}  | {:>5.2}  {:.3}  {:.4}  {:<8}{}{} | {:>5.2}  {:.3}  {:.4}  {:<8}{}{}",
            label,
            SHARES[si],
            sm[0].delta,
            sm[0].i_end.mean,
            sm[0].m.mean,
            sm[1].delta,
            sm[1].i_end.mean,
            sm[1].m.mean,
            oc(jb.outcome),
            if jb.undecided { "?" } else { " " },
            if jb.worse_than_a { "!" } else { " " },
            sm[2].delta,
            sm[2].i_end.mean,
            sm[2].m.mean,
            oc(jc.outcome),
            if jc.undecided { "?" } else { " " },
            if jc.worse_than_a { "!" } else { " " },
        );
    }

    // ---- R3: concentrated selective ring ----
    let sums: Vec<(f64, [ArmSum; 3])> = (0..4)
        .map(|si| (SHARES[si], summaries(&res[idx_r3[si]])))
        .collect();
    let js = print_cell_table(
        "R3 (reported, not decisive): concentrated selective ring (10 identities hold standing share s; B starts at 0.10)",
        &sums,
        false,
    );
    for x in 0..2 {
        f_lines.push(format!(
            "R3 concentrated selective, arm {}: {} (reading only)",
            &ARM_NAMES[x + 1][..1],
            verdict(&js[x])
        ));
    }

    println!("\n== VERDICTS (pre-registered criteria) ==");
    for l in &verdict_lines {
        println!("  {l}");
    }
    println!("\n== Reported, not decisive ==");
    for l in &f_lines {
        println!("  {l}");
    }
    println!("\n  ('?' marks a cell UNDECIDED within 2 SE; '!' marks an arm whose ring mint share M is above arm A's by more than 2 SE, paired.)");
    println!(
        "\nRuntime: {:.1} s for the runs, {:.1} s total.",
        run_secs,
        t0.elapsed().as_secs_f64()
    );
}
