//! JUL money layer, increment 1 — the issuance core (Lever A: the production-cost anchor).
//!
//! HONEST SCOPE: a deterministic, integer-only JUL issuance primitive, built in ISOLATION. It imports
//! NOTHING from `runtime` / consensus and is called from no consensus function, so it is provably
//! incapable of affecting replay-parity (the same additive/shadow discipline as `utxo_commitment`). It
//! ships the issuance RULE, not the economics: pre-PoW the work signal is a constant
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

// ============ Denomination ============

/// Sub-unit resolution: 1 JUL = 10^8 base units. HARDCODED (never governable) — changing decimals
/// would rewrite the meaning of every JUL balance. 10^8 is the Bitcoin-simple choice and keeps a
/// single block's reward well inside `u64` even at large mined difficulties, with `u128` cumulative
/// supply then carrying ~10^21 JUL of headroom.
pub const JUL_BASE_UNITS: u128 = 100_000_000;

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
}
