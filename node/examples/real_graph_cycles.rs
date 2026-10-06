//! Run Noesis's REAL cross-identity cycle detector on a REAL directed graph, WITH a null baseline.
//!
//! Default data: `data/deepfunding/dependency-graph/datasets/v2-graph/dependency-graph-v2.csv`
//! (real GitHub seed_repo -> dependency_repo edges). Generalized: pass any edge CSV.
//!   cargo run --release -p noesis --example real_graph_cycles
//!   cargo run --release -p noesis --example real_graph_cycles -- <csv> <from_col> <to_col> <skip_header:0|1>
//! e.g. a DEX trade graph:  ... -- trades.csv 0 1 1   (from_addr, to_addr columns)
//!
//! Detectors (built + tested, `node/src/lib.rs`):
//!   attribution_circulation  : MUTUAL (2-cycle) rings.
//!   attribution_cycle_energy : Helmholtz-Hodge harmonic energy -> DIRECTED k-cycles.
//!
//! NULL BASELINE: the raw energy is scale-dependent, so we degree-preservingly shuffle the graph
//! (permute the target endpoints, preserving every node's out-degree and the in-degree multiset)
//! K times and compare. observed >> null  =>  real cyclic structure, not a density artifact.

use noesis::{attribution_circulation, attribution_cycle_energy, Cell, Script};
use std::collections::{HashMap, HashSet};
use std::fs;

fn ident(i: usize) -> Vec<u8> { (i as u64).to_le_bytes().to_vec() }
fn script(args: Vec<u8>) -> Script { Script { code_hash: [0xB0u8; 32], args } }
fn anchor(id: u64, owner: Vec<u8>) -> Cell {
    Cell { id, lock: script(vec![]), type_script: script(owner), parent: None, timestamp: id, data: vec![] }
}
fn edge_cell(id: u64, owner: Vec<u8>, parent: u64) -> Cell {
    Cell { id, lock: script(vec![]), type_script: script(owner), parent: Some(parent), timestamp: id, data: vec![] }
}

// dep-free reproducible PRNG (xorshift64*)
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12; x ^= x << 25; x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn upto(&mut self, n: usize) -> usize { (self.next() % (n as u64)) as usize }
}

/// build cells from a directed edge list (n_repos anchors + one edge-cell per edge)
fn build(n_repos: usize, edges: &[(usize, usize)]) -> Vec<Cell> {
    let mut cells: Vec<Cell> = (0..n_repos).map(|i| anchor(i as u64, ident(i))).collect();
    let mut next = n_repos as u64;
    for &(s, d) in edges {
        cells.push(edge_cell(next, ident(s), d as u64));
        next += 1;
    }
    cells
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let path = a.get(1).map(|s| s.as_str())
        .unwrap_or("data/deepfunding/dependency-graph/datasets/v2-graph/dependency-graph-v2.csv");
    let from_col: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let to_col: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let skip_header: bool = a.get(4).map(|s| s != "0").unwrap_or(true);

    let text = fs::read_to_string(path).expect("read edge csv");
    let mut idx: HashMap<String, usize> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if skip_header && n == 0 { continue; }
        let cols: Vec<&str> = line.split(',').collect();
        let (src, dst) = (cols.get(from_col).unwrap_or(&"").trim(), cols.get(to_col).unwrap_or(&"").trim());
        if src.is_empty() || dst.is_empty() || src == dst { continue; }
        let mut get = |r: &str, names: &mut Vec<String>| -> usize {
            *idx.entry(r.to_string()).or_insert_with(|| { names.push(r.to_string()); names.len() - 1 })
        };
        let s = get(src, &mut names);
        let d = get(dst, &mut names);
        edges.push((s, d));
    }
    let n = names.len();

    // observed
    let cells = build(n, &edges);
    let circ = attribution_circulation(&cells);
    let energy = attribution_cycle_energy(&cells);

    // null baseline: degree-preserving target permutation, K trials
    const K: usize = 20;
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let targets: Vec<usize> = edges.iter().map(|&(_, d)| d).collect();
    let mut null_e: Vec<f64> = Vec::with_capacity(K);
    for _ in 0..K {
        let mut t = targets.clone();
        for i in (1..t.len()).rev() { let j = rng.upto(i + 1); t.swap(i, j); } // Fisher-Yates
        let shuffled: Vec<(usize, usize)> =
            edges.iter().enumerate().map(|(k, &(s, _))| (s, t[k])).filter(|&(s, d)| s != d).collect();
        null_e.push(attribution_cycle_energy(&build(n, &shuffled)));
    }
    let mean = null_e.iter().sum::<f64>() / K as f64;
    let var = null_e.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / K as f64;
    let std = var.sqrt();
    let z = if std > 0.0 { (energy - mean) / std } else { f64::INFINITY };

    // readable: mutual-dependency pairs
    let mut has: HashSet<(usize, usize)> = HashSet::new();
    for &(s, d) in &edges { has.insert((s, d)); }
    let mut mutual: Vec<(String, String)> = Vec::new();
    for &(s, d) in &edges { if s < d && has.contains(&(d, s)) { mutual.push((names[s].clone(), names[d].clone())); } }
    mutual.sort(); mutual.dedup();

    println!("=== Noesis cycle detector on: {}", path);
    println!("nodes: {}   directed edges: {}", n, edges.len());
    println!();
    println!("OBSERVED  circulation (mutual 2-cycles): {}", circ);
    println!("OBSERVED  cycle_energy (directed k-cycles): {:.2}", energy);
    println!();
    println!("NULL (degree-preserving shuffle, K={}):", K);
    println!("  null cycle_energy  mean={:.2}  std={:.2}  min={:.2}  max={:.2}",
        mean, std, null_e.iter().cloned().fold(f64::INFINITY, f64::min),
        null_e.iter().cloned().fold(0.0, f64::max));
    println!("  observed z-score vs null: {:.1}", z);
    println!("  => {}", if energy > mean + 3.0 * std {
        "observed cyclic structure is REAL SIGNAL (>3 sigma above degree-matched null)"
    } else {
        "observed is within null range -> energy is a density artifact, NOT signal"
    });
    println!();
    println!("mutual-dependency pairs: {}", mutual.len());
    for (x, y) in mutual.iter().take(12) { println!("  {}  <-->  {}", x, y); }
    if mutual.len() > 12 { println!("  ... and {} more", mutual.len() - 12); }
}
