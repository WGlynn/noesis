//! Phase-4 Step-1 — property-based invariants over the pure rulebook (stateless-verification, FV).
//!
//! Phases 0–3 make a receipt prove **the rulebook was FOLLOWED**. Phase 4 attacks the other
//! question: **is the rulebook RIGHT?** A perfectly-proven receipt for a rule that permits inflation
//! is worthless. This file pins the value-level invariants of `runtime::apply_block`. Plan:
//! `docs/phase4-fv-plan.md`.
//!
//! **No external test-crate dependency (deliberate).** The `proptest` route pulls
//! `getrandom → windows-sys`, which fail to link on this box's default `x86_64-pc-windows-gnu`
//! toolchain (no `dlltool.exe`), so plain `cargo test` would break. Instead we drive the properties
//! with a tiny in-file xorshift PRNG: pure Rust, zero deps, builds under the default toolchain, and
//! DETERMINISTIC — a failing case is reproducible from its fixed seed (fitting the repo's
//! replica-determinism ethos better than an OS-seeded RNG). Trade-off: no automatic shrinking; we
//! print the offending inputs on failure instead.
//!
//! The invariants (each with an anti-theater check — a property that cannot fail proves nothing):
//!   * **P1 (I1) value conservation** — a movement neither creates nor destroys value except an
//!     issuer-authorised mint/burn. Tx-level biconditional + block-gate accept/reject.
//!   * **P2 (I2/I3) no double-spend** — in-block (two txs consuming one identity) and cross-block (a
//!     retired UTXO cannot be respent).
//!   * **P3 (I4) no spend of a nonexistent output** — an input absent from the live set is rejected.
//!   * **P4 (I5) determinism** — `apply_block` is a pure function (same inputs ⇒ byte-identical
//!     `state_digest`); wire amounts round-trip (`decode∘encode == id`).
//!   * **P5 total supply** — over a random *sequence* of accepted conserving blocks, per-token supply
//!     never drifts.
//!
//! Scope (honest): these are the *value* invariants over the reference token layer, above the
//! commitment-level transition invariant (I6, host-tested in `utxo_commitment`). Auth/control
//! (lock-sig) is the deploy-coupled layer (`CONTROL_BINDING_ACTIVE == false` here) — see the
//! `is_valid_in_ledger` doc. The tx-digest canonicalisation (presentation-order invariance) is
//! covered by the lib unit test `tx_digest_is_invariant_to_input_output_presentation`; `digest()` is
//! `pub(crate)` so it is not re-exercised from this integration crate.

use noesis::commit_order::Committed;
use noesis::runtime::{apply_block, Block, Constitution, Ledger, TokenStandard, TokenTx, Violation};
use noesis::{tokens, Cell, Script};

// ============ deterministic PRNG (xorshift64*, no deps) ============

/// A tiny, deterministic PRNG. Same seed ⇒ same stream ⇒ a failing property is reproducible.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15) // avoid the all-zero fixed point
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// uniform-ish in `[0, n)`.
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
    fn below128(&mut self, n: u128) -> u128 {
        if n == 0 {
            0
        } else {
            self.u128() % n
        }
    }
    /// a full-range u128 (two draws) — exercises the whole wire domain for round-trips.
    fn u128(&mut self) -> u128 {
        ((self.next() as u128) << 64) | self.next() as u128
    }
}

/// case budgets: pure tx-level checks are cheap ⇒ many (10k). Block-apply checks each run a full
/// `apply_block` (incl. an O(chain) PoM recompute in a debug build), so they carry fewer cases (500)
/// to keep `cargo test` snappy — these invariants are not rare-event needles (a broken conservation
/// or double-spend check fails on the first case), and the anti-theater concretes pin the boundary.
const CASES_PURE: u64 = 10_000;
const CASES_BLOCK: u64 = 500;

// ============ fixtures ============

const FT: [u8; 32] = [20u8; 32]; // fungible standard program (type_script.code_hash)
const MT: [u8; 32] = [22u8; 32]; // multi standard program
const ISSUER: &[u8] = b"USD"; // the fungible token identity (type_script.args)
const OWNERS: [&[u8]; 3] = [b"al", b"bo", b"ca"]; // non-issuer owners ⇒ no mint authority

/// A fungible token cell: `lock.args` = owner, `type_script` = (FT, ISSUER), `data` = amount LE.
fn ft_cell(id: u64, owner: &[u8], amount: u128) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [0u8; 32], args: owner.to_vec() },
        type_script: Script { code_hash: FT, args: ISSUER.to_vec() },
        parent: None,
        timestamp: 0,
        data: tokens::fungible::encode(amount),
    }
}

/// A fungible movement with the pre-deploy inert auth path (`auths` empty ⇒ CONTROL_BINDING off).
fn ft_tx(inputs: Vec<Cell>, outputs: Vec<Cell>) -> TokenTx {
    TokenTx {
        standard: TokenStandard::Fungible,
        code_hash: FT,
        args: ISSUER.to_vec(),
        inputs,
        outputs,
        auths: Vec::new(),
    }
}

/// One attribution cell + coordinate, so a block is well-formed (≥1 cell, canonical order). Its id
/// lives in a high range disjoint from token-cell ids so the two cell sets never confuse a reader.
fn attr(height: u64) -> (Cell, Committed) {
    let c = Cell {
        id: 1_000_000 + height,
        lock: Script { code_hash: [0u8; 32], args: b"lk".to_vec() },
        type_script: Script { code_hash: [1u8; 32], args: b"contrib".to_vec() },
        parent: None,
        timestamp: height,
        // per-height-unique payload ⇒ disjoint coverage ⇒ never zeroed by the similarity floor.
        data: vec![b'A' + (height % 26) as u8; 48],
    };
    (c, Committed { height, secret: [height as u8; 32] })
}

/// A canonical-ordered block at `height` carrying `txs` (via `Block::assemble` ⇒ ordering valid by
/// construction).
fn block_with(height: u64, txs: Vec<TokenTx>) -> Block {
    Block::assemble(height, &[attr(height)]).with_token_txs(txs)
}

/// A fresh ledger seeded with a live token-cell set (all `Ledger` fields are `pub`).
fn seeded(token_cells: Vec<Cell>) -> Ledger {
    Ledger { token_cells, ..Ledger::new() }
}

fn ft_total(l: &Ledger) -> u128 {
    tokens::fungible::total(&l.token_cells, &FT, ISSUER)
}

fn con() -> Constitution {
    Constitution::default()
}

// ============ P1 (I1) — value conservation ============

#[test]
fn p1_fungible_conservation_biconditional() {
    // With NO mint authority present (owners are non-issuers), `is_valid()` ⟺ `Σout ≤ Σin`
    // (conserve or burn allowed; an unauthorised supply increase rejected). A false biconditional
    // here is a real I1 bug. Anti-theater: over 10k cases the generator produces BOTH branches (see
    // the coverage assert at the end).
    let mut rng = Rng::new(0xF00D_1111);
    let (mut saw_valid, mut saw_invalid) = (false, false);
    for _ in 0..CASES_PURE {
        let n_in = 1 + rng.below(5);
        let ins: Vec<u128> = (0..n_in).map(|_| rng.below128(1000)).collect();
        let n_out = rng.below(6);
        let outs: Vec<(usize, u128)> =
            (0..n_out).map(|_| (rng.below(3) as usize, rng.below128(1000))).collect();
        let inputs: Vec<Cell> =
            ins.iter().enumerate().map(|(i, &a)| ft_cell(i as u64 + 1, OWNERS[i % 3], a)).collect();
        let outputs: Vec<Cell> =
            outs.iter().enumerate().map(|(i, &(o, a))| ft_cell(2000 + i as u64, OWNERS[o], a)).collect();
        let total_in: u128 = ins.iter().sum();
        let total_out: u128 = outs.iter().map(|&(_, a)| a).sum();
        let valid = ft_tx(inputs, outputs).is_valid();
        assert_eq!(
            valid,
            total_out <= total_in,
            "conservation biconditional broke: in={total_in} out={total_out} ins={ins:?} outs={outs:?}"
        );
        saw_valid |= valid;
        saw_invalid |= !valid;
    }
    assert!(saw_valid && saw_invalid, "anti-theater: both accept and reject cases must occur");
}

#[test]
fn p1_block_rejects_inflation_accepts_conserving() {
    // Block gate. Inputs are seeded LIVE so a rejection is conservation, not non-existence (isolates
    // I1 from I4).
    let mut rng = Rng::new(0xF00D_2222);
    for _ in 0..CASES_BLOCK {
        let base = 1 + rng.below128(1000);
        // (a) unauthorised inflation ⇒ REJECTED.
        let extra = 1 + rng.below128(1000);
        let k = ft_cell(1, b"al", base);
        let bad = block_with(1, vec![ft_tx(vec![k.clone()], vec![ft_cell(2, b"al", base + extra)])]);
        assert!(
            matches!(apply_block(seeded(vec![k.clone()]), &bad, &con()), Err(Violation::TokenTxInvalidOrDoubleSpend)),
            "inflation {base}->{} accepted",
            base + extra
        );
        // (b) conserving split ⇒ ACCEPTED, supply preserved, input retired, outputs live.
        let split = rng.below128(base + 1);
        let good = block_with(
            1,
            vec![ft_tx(vec![k.clone()], vec![ft_cell(2, b"al", base - split), ft_cell(3, b"bo", split)])],
        );
        let l = apply_block(seeded(vec![k]), &good, &con())
            .unwrap_or_else(|v| panic!("conserving {base}->{}+{split} rejected: {v:?}", base - split));
        assert_eq!(ft_total(&l), base, "supply drifted on a conserving transfer");
        assert!(!l.token_cells.iter().any(|c| c.id == 1), "input not retired");
        assert!(l.token_cells.iter().any(|c| c.id == 2), "output missing");
    }
}

// ============ P2 (I2/I3) — no double-spend ============

#[test]
fn p2_in_block_double_spend_rejected() {
    let mut rng = Rng::new(0xF00D_3333);
    for _ in 0..CASES_BLOCK {
        let amt = 1 + rng.below128(1000);
        let k = ft_cell(1, b"al", amt);
        // two txs each consume identity (1, al, FT) in ONE block.
        let blk = block_with(
            1,
            vec![
                ft_tx(vec![k.clone()], vec![ft_cell(2, b"bo", amt)]),
                ft_tx(vec![k.clone()], vec![ft_cell(3, b"ca", amt)]),
            ],
        );
        assert!(
            matches!(apply_block(seeded(vec![k]), &blk, &con()), Err(Violation::TokenTxInvalidOrDoubleSpend)),
            "in-block double-spend of amt={amt} was accepted"
        );
    }
}

#[test]
fn p2_cross_block_double_spend_rejected() {
    let mut rng = Rng::new(0xF00D_4444);
    for _ in 0..CASES_BLOCK {
        let amt = 1 + rng.below128(1000);
        let k = ft_cell(1, b"al", amt);
        // block 1 spends K (retires it).
        let blk1 = block_with(1, vec![ft_tx(vec![k.clone()], vec![ft_cell(2, b"bo", amt)])]);
        let l = apply_block(seeded(vec![k.clone()]), &blk1, &con()).expect("block 1 valid");
        assert!(!l.token_cells.iter().any(|c| c.id == 1), "K not retired by block 1");
        // block 2 tries to respend K ⇒ not live ⇒ rejected.
        let blk2 = block_with(2, vec![ft_tx(vec![k], vec![ft_cell(3, b"ca", amt)])]);
        assert!(
            matches!(apply_block(l, &blk2, &con()), Err(Violation::TokenTxInvalidOrDoubleSpend)),
            "cross-block respend of amt={amt} was accepted"
        );
    }
}

// ============ P3 (I4) — no spend of a nonexistent output ============

#[test]
fn p3_rejects_phantom_input() {
    let mut rng = Rng::new(0xF00D_5555);
    for _ in 0..CASES_BLOCK {
        let amt = 1 + rng.below128(1000);
        // conserving tx (so the failure is non-existence, not conservation), but the input was never
        // seeded into the live set.
        let phantom = ft_cell(1, b"al", amt);
        let blk = block_with(1, vec![ft_tx(vec![phantom], vec![ft_cell(2, b"bo", amt)])]);
        assert!(
            matches!(apply_block(seeded(vec![]), &blk, &con()), Err(Violation::TokenTxInvalidOrDoubleSpend)),
            "a phantom (never-finalized) input of amt={amt} was spent"
        );
    }
}

// ============ P4 (I5) — determinism + round-trip ============

#[test]
fn p4_apply_block_is_deterministic() {
    let mut rng = Rng::new(0xF00D_6666);
    for _ in 0..CASES_BLOCK {
        let amt = rng.below128(1000);
        let k = ft_cell(1, b"al", amt);
        let blk = block_with(1, vec![ft_tx(vec![k.clone()], vec![ft_cell(2, b"bo", amt)])]);
        let d1 = apply_block(seeded(vec![k.clone()]), &blk, &con()).unwrap().state_digest();
        let d2 = apply_block(seeded(vec![k]), &blk, &con()).unwrap().state_digest();
        assert_eq!(d1, d2, "apply_block is not a pure function on amt={amt}");
    }
}

#[test]
fn p4_wire_amounts_roundtrip() {
    let mut rng = Rng::new(0xF00D_7777);
    for _ in 0..CASES_PURE {
        // fungible amount over the FULL u128 range.
        let a = rng.u128();
        assert_eq!(tokens::fungible::amount(&ft_cell(1, b"al", a)), a, "fungible amount round-trip");
        // multi (id, amount).
        let id = rng.next() as u32;
        let amt = rng.u128();
        let c = Cell {
            id: 1,
            lock: Script { code_hash: [0u8; 32], args: b"al".to_vec() },
            type_script: Script { code_hash: MT, args: b"game".to_vec() },
            parent: None,
            timestamp: 0,
            data: tokens::multi::encode(id, amt),
        };
        assert_eq!(tokens::multi::entry(&c), Some((id, amt)), "multi entry round-trip");
    }
}

// ============ P5 — total supply conserved over a sequence ============

#[test]
fn p5_total_supply_conserved_over_sequence() {
    // Over a random sequence of accepted conserving blocks (each merges the whole live set and
    // re-splits it), the token's total supply never drifts from its issuance.
    let mut rng = Rng::new(0xF00D_8888);
    const INIT: u128 = 200;
    for _ in 0..CASES_BLOCK {
        let mut l = seeded(vec![ft_cell(1, b"al", INIT)]);
        let mut next_id = 2u64;
        let rounds = 1 + rng.below(11);
        for round in 0..rounds {
            let live: Vec<Cell> = l
                .token_cells
                .iter()
                .filter(|c| c.type_script.code_hash == FT && c.type_script.args == ISSUER)
                .cloned()
                .collect();
            let total: u128 = live.iter().map(tokens::fungible::amount).sum();
            let a = rng.below128(total + 1);
            let out1 = ft_cell(next_id, OWNERS[(round % 3) as usize], a);
            let out2 = ft_cell(next_id + 1, OWNERS[((round + 1) % 3) as usize], total - a);
            next_id += 2;
            let blk = block_with(round + 1, vec![ft_tx(live, vec![out1, out2])]);
            l = apply_block(l, &blk, &con())
                .unwrap_or_else(|v| panic!("round {round}: conserving block rejected: {v:?}"));
            assert_eq!(ft_total(&l), INIT, "supply drifted at round {round}");
        }
    }
}

// ============ anti-theater — concrete accept/reject boundary cases ============
//
// A biconditional/negative property is only as strong as the existence of BOTH outcomes in its input
// space. These hand-built cases pin the exact accept/reject boundary, so if a conservation or
// mint-authority check were deleted, a NAMED test goes RED (not just "coverage dropped").

#[test]
fn anti_theater_inflation_rejected_transfer_accepted() {
    let k = ft_cell(1, b"al", 10);
    // 10 -> 11 with no authority ⇒ rejected.
    let bad = block_with(1, vec![ft_tx(vec![k.clone()], vec![ft_cell(2, b"al", 11)])]);
    assert!(matches!(
        apply_block(seeded(vec![k.clone()]), &bad, &con()),
        Err(Violation::TokenTxInvalidOrDoubleSpend)
    ));
    // 10 -> 6 + 4 ⇒ accepted, supply preserved.
    let good =
        block_with(1, vec![ft_tx(vec![k.clone()], vec![ft_cell(2, b"al", 6), ft_cell(3, b"bo", 4)])]);
    let l = apply_block(seeded(vec![k]), &good, &con()).expect("conserving transfer accepted");
    assert_eq!(ft_total(&l), 10);
}

#[test]
fn anti_theater_issuer_may_mint_nonissuer_may_not() {
    // P1's mint-authority branch (the ONLY sanctioned supply increase): authorised iff the issuer
    // spends an authority cell it owns (type == (FT, ISSUER) AND lock.args == ISSUER).
    let authority = ft_cell(1, ISSUER, 0); // issuer owns this input
    let mint_out = ft_cell(2, b"al", 1000);
    assert!(
        ft_tx(vec![authority], vec![mint_out.clone()]).is_valid(),
        "issuer-owned authority input authorises the mint"
    );
    // same amounts, but the input is owned by a non-issuer ⇒ no authority ⇒ mint rejected.
    assert!(
        !ft_tx(vec![ft_cell(1, b"al", 0)], vec![mint_out]).is_valid(),
        "no authority input ⇒ unauthorised mint rejected"
    );
}
