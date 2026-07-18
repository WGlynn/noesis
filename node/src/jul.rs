//! JUL money layer, increment 1 — the issuance core (Lever A: the production-cost anchor).
//!
//! HONEST SCOPE: a deterministic, integer-only JUL issuance primitive. It imports only the root
//! `Cell`/`Script` types, never runtime's consensus internals, and the dependency arrow is ONE-WAY:
//! runtime's `apply_transition` calls into jul (increment 2 wires the coinbase mint via
//! `reward_for_work` + the identity constants below); jul never calls runtime. It ships the issuance
//! RULE, not the economics: pre-PoW the work signal is a constant
//! (`runtime::WORK_PER_BLOCK == 1`), so the anchor is economically INERT until real mined difficulty
//! flows through `runtime::block_work` — a later increment. Settling JUL as a transferable `Fungible`
//! token (increment 2), the counter-cyclical reserve (increment 3), and consensus/genesis wiring +
//! governable `Constitution` params (increment 4) are NOT here.
//!
//! Numbers here are v0 UNIT definitions, not pinned economics — the design note flags Ergon fidelity
//! and parameter values as open (`docs/DESIGN-jul-money-layer.md` §5). Do not mistake a placeholder
//! for a decision.
//!
//! Lever A in one line: JUL is minted at a fixed integer PRICE OF WORK — `reward = work · num/den` — so
//! that when `block_work` later returns a block's mined difficulty instead of the pre-PoW constant,
//! issuance becomes difficulty-proportional (the production-cost anchor) with NO change to this rule.
//! This reuses the codebase's own "right interface, degenerate constant" pattern: pre-PoW the work
//! clock degrades to a height clock, and here JUL issuance degrades to a flat per-block subsidy.

use crate::Cell;

// ============ Denomination ============

/// Sub-unit resolution: 1 JUL = 10^8 base units. HARDCODED (never governable) — changing decimals
/// would rewrite the meaning of every JUL balance. 10^8 is the Bitcoin-simple choice and keeps a
/// single block's reward well inside `u64` even at large mined difficulties, with `u128` cumulative
/// supply then carrying ~10^21 JUL of headroom.
pub const JUL_BASE_UNITS: u128 = 100_000_000;

// ============ JUL token identity + coinbase id space (increment 2) ============

/// The fungible type-script PROGRAM id for JUL cells. v0 PLACEHOLDER — a reserved, nothing-up-my-sleeve
/// tag; becomes the hash of the real on-VM RISC-V type-script when that port lands. Stable identity,
/// program hash owed.
pub const JUL_CODE_HASH: [u8; 32] = *b"NOESIS-JUL-FUNGIBLE-TYPESCRIPT-0";

/// The JUL issuer identity (`type_script.args`). A reserved constant that is NOBODY's key: no Lamport
/// keypair with this root is known, and no cell with `lock.args == JUL_ISSUER` is ever seeded. Combined
/// with the JUL-conserve-only clause in `runtime::token_txs_conserve_and_single_use`, the block coinbase
/// is the STRUCTURALLY UNIQUE JUL inflation channel (closes the pay-to-issuer-lock → mint hole).
pub const JUL_ISSUER: &[u8] = b"NOESIS-JUL-ISSUER-v0-nobody-holds-this-key";

/// Reserved id space for coinbase cells: top bit set, low bits = height ⇒ one deterministic id per
/// height, forever. Token-tx outputs are barred from this space, so no producer-chosen output can
/// collide with a coinbase identity and grief its retirement.
pub const COINBASE_ID_BIT: u64 = 1 << 63;

/// The deterministic coinbase cell id for a block at `height` — the PRODUCER's reward. Never sets
/// [`SPLIT_SLICE_BIT`] (requires `height < 2^62`, astronomical), so it is disjoint from every slice id.
pub fn coinbase_id(height: u64) -> u64 {
    COINBASE_ID_BIT | (height & 0x3fff_ffff_ffff_ffff)
}

/// Second reserved bit, set ONLY on coinbase SLICE cells (the N-way split recipients, inc-M3-3). Keeps a
/// slice id disjoint from every producer [`coinbase_id`] (which never sets it) while staying inside the
/// [`COINBASE_ID_BIT`] reserved half ⇒ token-tx outputs remain barred from slice ids too.
pub const SPLIT_SLICE_BIT: u64 = 1 << 62;

/// The deterministic id of the `index`-th coinbase SLICE cell at `height`. Unique per `(height, index)`
/// and disjoint from every `coinbase_id`. Bounds (astronomical): `height < 2^54`, `index < 256`.
/// The 8-bit `index` field is the hard cap on split size: an `index ≥ 256` would `& 0xff`-WRAP and
/// collide with an earlier slice's id. That bound is enforced at Constitution admission via
/// [`MAX_COINBASE_SPLIT`] (fail-loud on a misconfigured genesis) — this function stays total.
pub fn coinbase_slice_id(height: u64, index: u64) -> u64 {
    COINBASE_ID_BIT | SPLIT_SLICE_BIT | ((height & 0x003f_ffff_ffff_ffff) << 8) | (index & 0xff)
}

/// Hard cap on `Constitution.coinbase_split` length: the [`coinbase_slice_id`] index field is 8 bits,
/// so beyond this a slice id would wrap and collide with an earlier one. Enforced fail-loud when a
/// [`crate::runtime::Constitution`] is admitted at genesis (a governance/genesis misconfiguration must
/// be caught before launch, NOT silently mint two coinbase cells sharing an id).
pub const MAX_COINBASE_SPLIT: usize = 256;

/// Is this cell a JUL cell (matches the JUL type-script program + issuer)?
pub fn is_jul(cell: &Cell) -> bool {
    cell.type_script.code_hash == JUL_CODE_HASH && cell.type_script.args.as_slice() == JUL_ISSUER
}

// ============ Issuance parameters (v0; migrate to a governable Constitution field at wiring) ============

/// The issuance RATE as an exact rational: `reward = work · num / den` base units. Held as a plain
/// value in increment 1 (the module is deliberately consensus-isolated); it migrates to a governable
/// `runtime::Constitution` field when JUL is wired into `apply` (increment 4). The defaults define the
/// UNIT, not the economics — `1 JUL per unit of work` — and deliberately refuse to cosplay Bitcoin's
/// 50/block or any unverified Ergon number (`docs/DESIGN-jul-money-layer.md` §5: parameters owed).
///
/// A rational pair (not Q16.16) is used on purpose: the in-repo Q16.16 convention is for fractions in
/// `[0,1]`, whereas an issuance rate spans magnitudes in both directions once real difficulty (hashes)
/// lands, and an explicit `num/den` is exact and arbitrarily rangeable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JulParams {
    /// Base units minted per `reward_den` units of work.
    pub reward_num: u64,
    /// Work units per `reward_num` base units. Must be ≥ 1 (clamped fail-safe in [`reward_for_work`];
    /// governance will bound it away from 0 at wiring).
    pub reward_den: u64,
}

impl Default for JulParams {
    fn default() -> Self {
        // 1 JUL (= JUL_BASE_UNITS base units) per unit of work.
        JulParams { reward_num: JUL_BASE_UNITS as u64, reward_den: 1 }
    }
}

// ============ The issuance rule (Lever A) ============

/// JUL base units minted for `work` units of contribution, at the rate in `params`. Pure, total, and
/// integer-only. Floor division: deterministic, replica-identical, biased DOWN — the chain never mints
/// more than the exact rational product, and the sub-unit remainder (< 1 base unit) is discarded, not
/// carried (a remainder accumulator would add a state word and an invariant for < 1 base unit/block —
/// the lean rule wins).
///
/// ERGON SEAM: v0 proportionality is LINEAR — the minimal rule consistent with the design note's stated
/// direction ("difficulty falls → issuance slows"). If pinning Ergon's public design yields a nonlinear
/// curve `f(work)`, it replaces THIS BODY behind THIS SIGNATURE; nothing else changes.
///
/// Totality: `u64 as u128 · u64 as u128` is at most `(2^64 − 1)^2 = 2^128 − 2^65 + 1 < u128::MAX`, so
/// the multiply is exact and `saturating_mul` is belt-and-braces (it never actually saturates). `den`
/// is clamped to `≥ 1`, so the function is total even on an impossible zero denominator (fail-closed).
pub fn reward_for_work(work: u64, params: JulParams) -> u128 {
    (work as u128).saturating_mul(params.reward_num as u128) / (params.reward_den.max(1) as u128)
}

/// The full Ergon-style issuance rule: the proportional reward, scaled by the **Moore's-law calendar
/// decay** `e^(−a_estim·t)` that holds JUL's ENERGY peg (`DECISIONS-M3-money-2026-07-15.md` §1). The
/// proportional part (`reward_for_work`) pegs JUL to a fixed amount of *work* (hashes); the decay
/// corrects for hardware getting cheaper per hash over calendar time, so a fixed amount of *energy*
/// keeps minting a fixed amount of JUL — the actual invariant.
///
/// `elapsed` = calendar time since genesis (seconds, from the attested wall-clock — Moore's law is
/// calendar-based, not work-based). `halflife` = the hardware-efficiency doubling period (the governable
/// `a_estim` expressed as an exact integer period, `= ln2 / a_estim`).
///
/// **INERT DEFAULT:** `halflife == 0` ⇒ `noesis_core::pow::moore_decay_q32` returns the identity
/// multiplier `2^32`, so this is BYTE-IDENTICAL to `reward_for_work` until a nonzero period is governed
/// in (the same default-off seam as `pow_enforced`). The decay only reduces issuance — `decay ≤ 1` always
/// ⇒ this never mints more than `reward_for_work`, preserving the `Σout ≤ Σin` conservation oracle.
///
/// Totality: `base ≤ ~2^91` (work·1e8) and `decay ≤ 2^32`, so the Q32 product `≤ 2^123 < u128::MAX`;
/// `saturating_mul` is belt-and-braces. Floor `>> 32` biases DOWN (never over-mints), matching
/// `reward_for_work`'s floor-division discipline.
pub fn reward_with_decay(work: u64, params: JulParams, elapsed: u64, halflife: u64) -> u128 {
    let base = reward_for_work(work, params);
    let decay = noesis_core::pow::moore_decay_q32(elapsed, halflife) as u128;
    base.saturating_mul(decay) >> 32
}

// ============ Issuance state ============

/// Cumulative JUL ever issued, in base units — the ONLY issuance state. The per-block subsidy is NOT
/// stored: it is a pure function of the block's work + params, recomputable by any replica (the same
/// "don't persist what replay derives" discipline as `runtime::Ledger::finalized_at`).
///
/// Invariants: `issued` is monotone nondecreasing; there is NO supply cap by design — the energy-cost
/// anchor replaces the cap (`docs/DESIGN-jul-money-layer.md`); a cap, if ever wanted, is a new explicit
/// field, never a silent edit. The `u128` width matches the `tokens::fungible` amount representation, so
/// increment 2's settlement into transferable cells needs no conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JulSupply {
    issued: u128,
}

impl JulSupply {
    /// Fair launch: zero supply, no pre-mine (`docs/DESIGN-jul-money-layer.md` §2).
    pub fn new() -> Self {
        JulSupply { issued: 0 }
    }

    /// Total JUL issued so far, in base units.
    pub fn issued(&self) -> u128 {
        self.issued
    }

    /// Mint `reward` base units for one finalized block and return the new cumulative total. Monotone
    /// by construction (`reward` is unsigned; `saturating_add` never wraps).
    pub fn credit(&mut self, reward: u128) -> u128 {
        self.issued = self.issued.saturating_add(reward);
        self.issued
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_division_discards_sub_unit_remainder() {
        // 7 · 10 / 3 = 70/3 = 23 (floor); the 1/3 remainder is discarded, never carried.
        let p = JulParams { reward_num: 10, reward_den: 3 };
        assert_eq!(reward_for_work(7, p), 23);
    }

    #[test]
    fn default_rate_is_one_jul_per_unit_of_work() {
        assert_eq!(reward_for_work(1, JulParams::default()), JUL_BASE_UNITS);
        assert_eq!(reward_for_work(0, JulParams::default()), 0);
    }

    #[test]
    fn zero_denominator_is_total_not_a_panic() {
        // Clamp to den ≥ 1 ⇒ the function is total on the full input domain (fail-closed).
        let p = JulParams { reward_num: 100, reward_den: 0 };
        assert_eq!(reward_for_work(5, p), 500);
    }

    // ============ Moore's-law decay (ERGON SEAM) ============

    #[test]
    fn decay_off_is_byte_identical_to_flat_reward() {
        // halflife == 0 ⇒ inert seam ⇒ reward_with_decay ≡ reward_for_work at ANY elapsed time.
        let p = JulParams::default();
        for work in [0u64, 1, 7, 1_000, u32::MAX as u64] {
            for elapsed in [0u64, 1, 1_000_000, u64::MAX] {
                assert_eq!(
                    reward_with_decay(work, p, elapsed, 0),
                    reward_for_work(work, p),
                    "decay-off must equal flat reward (work={work}, elapsed={elapsed})"
                );
            }
        }
    }

    #[test]
    fn identical_energy_mints_identical_jul_across_an_efficiency_doubling() {
        // THE PEG PROPERTY. Hardware efficiency doubles every `halflife` seconds ⇒ the same ENERGY buys
        // 2× the work (hashes). With the decay, that 2× work at t=halflife must mint ~the SAME JUL as
        // 1× work at t=0 — energy-pegged, not hash-pegged.
        let p = JulParams::default();
        let halflife = 94_608_000u64; // ~3 years in seconds (illustrative a_estim)
        let work_t0 = 1_000_000u64; // energy E buys this many hashes at genesis
        let work_t1 = 2_000_000u64; // same energy E buys 2× hashes after one efficiency doubling

        let jul_t0 = reward_with_decay(work_t0, p, 0, halflife);
        let jul_t1 = reward_with_decay(work_t1, p, halflife, halflife);

        // Equal within the fixed-point rounding floor (the >>32 + cubic approximation).
        let diff = jul_t0.abs_diff(jul_t1);
        assert!(diff <= 2, "energy peg broken: t0={jul_t0} t1={jul_t1} diff={diff}");
        // And sanity: a FLAT reward would have paid DOUBLE at t1 (the bug the decay fixes).
        assert_eq!(reward_for_work(work_t1, p), 2 * reward_for_work(work_t0, p));
    }

    #[test]
    fn decay_halves_issuance_each_halflife_and_never_over_mints() {
        let p = JulParams::default();
        let hl = 1_000_000u64;
        let work = 1_000_000u64;
        let flat = reward_for_work(work, p);
        // One period ⇒ ½, two ⇒ ¼ (within rounding).
        assert!(reward_with_decay(work, p, hl, hl).abs_diff(flat / 2) <= 1);
        assert!(reward_with_decay(work, p, 2 * hl, hl).abs_diff(flat / 4) <= 1);
        // decay ≤ 1 ALWAYS ⇒ never mints more than the flat reward (conservation-safe).
        for elapsed in [0u64, hl / 2, hl, 5 * hl, 100 * hl] {
            assert!(reward_with_decay(work, p, elapsed, hl) <= flat);
        }
    }

    // ============ Coinbase id-space disjointness ============

    #[test]
    fn coinbase_id_never_collides_with_slice_id_at_extreme_heights() {
        // A producer coinbase_id must NEVER set SPLIT_SLICE_BIT (bit 62), or it could collide with a
        // slice id and let two coinbase cells share an id. Regression for heights ≥ 2^62, where the
        // raw `COINBASE_ID_BIT | height` would leak height's bit 62 into the slice-id half.
        for height in [0u64, 1, 1 << 53, (1 << 62) - 1, 1 << 62, (1 << 62) + 7, u64::MAX] {
            let pid = coinbase_id(height);
            assert_eq!(pid & SPLIT_SLICE_BIT, 0, "coinbase_id set SPLIT_SLICE_BIT at height {height}");
            assert_ne!(pid & COINBASE_ID_BIT, 0, "coinbase_id must stay in the reserved half");
            // Explicit collision witness: at height 2^62 the raw impl equalled slice 0's id.
            assert_ne!(pid, coinbase_slice_id(height, 0), "producer id collided with slice 0 at height {height}");
        }
    }
}
