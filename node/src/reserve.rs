//! JUL money layer, increment 3 — the counter-cyclical reserve (Lever B: the short-run smoother).
//!
//! HONEST SCOPE: a deterministic, integer-only reserve state machine. It is the mechanism port of
//! `TreasuryStabilizer.sol` (`vibeswap/contracts/governance/TreasuryStabilizer.sol`) — the Solidity
//! contract's POLICY (trend classification, bounded sizing, cooldown/period budgets) carried onto a
//! venue-less deterministic UTXO replay chain; everything that was Solidity-environment scaffolding
//! dissolves (see below). It imports NOTHING — not even the root `Cell`/`Script` types — so it is even
//! leaner than `jul.rs`, and it is called from NO consensus path. Increment 3 is therefore ADDITIVE /
//! SHADOW: `validate_block`, `apply_transition`, `apply_block`, `token_txs_conserve_and_single_use`,
//! and `state_digest` are all untouched, so replay-parity is unaffected BY CONSTRUCTION (proven three
//! ways in `node/tests/jul_reserve.rs`).
//!
//! INERT UNTIL REAL INPUTS (the `jul.rs` honesty idiom, jul.rs:6-9). Two independent inerting facts:
//!   1. Pre-PoW `runtime::block_work` is the constant `WORK_PER_BLOCK == 1` (runtime.rs:495), so the v0
//!      work-series signal is flat ⇒ `trend_bps == 0` ⇒ the market never reads bear ⇒ nothing releases.
//!   2. [`ReserveParams::default()`] is ALL ZERO ⇒ the skim funds nothing and the release rate is nil ⇒
//!      the reserve never accumulates and never deploys. The mechanism ships provably OFF; every number
//!      is OWED, not invented (`docs/DESIGN-jul-money-layer.md` §5). Do not mistake a placeholder for a
//!      decision, and do not read a nonzero test value as pinned economics.
//!
//! WHY NO CELL IN INCREMENT 3 (a load-bearing safety fact, not a shortcut). Representing the reserve as
//! a real JUL cell requires a reserved nobody's-key lock — but `CONTROL_BINDING_ACTIVE == false`
//! (runtime.rs:411) means an empty-auth spend currently AUTHORIZES, so a keyless reserve-lock cell would
//! be anyone-spendable in the reference model today. A real reserve cell therefore needs a new
//! protocol-spend-only validate clause (consensus-affecting) and is deferred to increment 3b. For now the
//! reserve is a module-local `u128` balance + bookkeeping — the same "plain value now, `Constitution`
//! field at wiring" posture as `jul.rs` increment 1 (jul.rs:63-66).
//!
//! ASYMMETRY BY DESIGN — the withdraw round-trip DISSOLVES (a deliberate re-derivation, NOT an omission).
//! `TreasuryStabilizer` deploys reserves as reversible pool liquidity and later WITHDRAWS them on recovery
//! (sol:322-394). Noesis has no AMM and no reversible position: a release is spent irreversibly (its
//! destination — coinbase subsidy top-up vs burn — is a seam, below), and "rebuild reserves in strength"
//! is simply the ongoing skim in non-bear periods. So there is no `withdrawDeployment`, no LP unwind, no
//! failure-flag/retry machinery — the reserve is an ASYMMETRIC accumulate-then-subsidize smoother, never a
//! peg (`docs/DESIGN-jul-money-layer.md` §3). This is destination-chain physics, documented so a later
//! reader does not mistake it for a porting error.
//!
//! WILL-GATED SEAMS (undecided policy shipped as seams, the increment-1/2 discipline):
//!   * SIGNAL SOURCE — the v0 proxy is the difficulty/work trend ([`Signal::from_work_series`] over
//!     `runtime::block_work` history), a native, replay-deterministic, oracle-free reading of miners'
//!     revealed price-vs-production-cost (`DESIGN` §3, `tokens.rs:10-13` "NO PRICE/ATTESTATION ORACLE
//!     LAYER"). It is a PROXY with named confounders — passive/exogenous (energy-price shocks,
//!     hardware-efficiency jumps, lag) AND, more dangerously, ENDOGENOUS: the work-trend is a
//!     revealed-preference signal the PAID party can MANIPULATE — a producer throttling hashrate to trip
//!     the bear flag and harvest a release — which an exogenous price feed cannot be. That reflexivity is
//!     the first-order reason the source is a seam, and it gates any nonzero params on the inc-4
//!     game-theory pass. The work-trend is also SIGN-AMBIGUOUS (it cannot separate a JUL-price drawdown,
//!     where a release is correctly counter-cyclical, from an input-cost shock, where it is mis-targeted).
//!     So the source is swappable: [`assess`] takes an `Option<Signal>`; a future price feed is a
//!     different `Signal` at the call site, ZERO mechanism change. Absent a signal ⇒ NO assessment
//!     (fail-closed) — a synthesized trend is never invented (the `sol:447-456` volatility fallback is
//!     deliberately NOT ported).
//!   * FUNDING — the reserve is fed ONLY by a protocol-fixed slice of newly-constructed coinbase issuance
//!     ([`Reserve::accrue`], `skim_bps`): new energy money that was never any participant's property, so
//!     funding buys NOTHING — no PoM standing, no PoS weight, no governance voice (the anti-plutocracy
//!     invariant, `DESIGN` §5, proven executable in `jul_reserve.rs`). The RATE and turn-on are owed
//!     economics ⇒ `skim_bps` defaults to 0; at wiring it becomes one `Constitution` field beside
//!     `Constitution.jul` (runtime.rs:79-82).
//!   * RELEASE DESTINATION — what a released amount is spent ON (coinbase subsidy top-up vs burn) is
//!     policy; [`Reserve::release`] only computes and debits the bounded AMOUNT. The consumer decides the
//!     destination at increment 3b's coinbase-mint site (runtime.rs:822-837), exactly the increment-2
//!     recipient-policy seam.
//!
//! CONSERVATION is a THEOREM PROVEN BY TEST (`jul_reserve.rs`), never a runtime check — the established
//! bar (jul_settlement.rs:196-224). The coinbase stays the structurally unique JUL inflation channel
//! (runtime.rs:746-768): this module can only move a `u128` it was handed, and cannot mint.

// ============ Signal (the market reading) ============

/// A two-horizon market signal: the short-window and long-window means of the underlying series. The
/// counter-cyclical trend is their ratio ([`trend_bps`]). Kept as an explicit input so the SOURCE is a
/// seam — v0 derives it from the work series ([`Signal::from_work_series`]); a future price feed supplies
/// the same shape with no mechanism change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signal {
    /// Short-horizon mean (recent).
    pub short: u64,
    /// Long-horizon mean (baseline).
    pub long: u64,
}

impl Signal {
    /// The v0 deterministic proxy: means of the last `short_window` and `long_window` per-block work
    /// values (`runtime::block_work` history). `None` — fail-closed — when a window is zero or history is
    /// shorter than `long_window` (insufficient data is NOT a synthesized signal). Pure and integer-only;
    /// pre-PoW the series is the constant `WORK_PER_BLOCK`, so `short == long` and [`trend_bps`] is 0.
    pub fn from_work_series(works: &[u64], short_window: usize, long_window: usize) -> Option<Signal> {
        if short_window == 0 || long_window == 0 || works.len() < long_window || short_window > long_window {
            return None;
        }
        let mean = |w: usize| -> u64 {
            let slice = &works[works.len() - w..];
            let sum: u128 = slice.iter().map(|x| *x as u128).sum();
            (sum / w as u128) as u64
        };
        Some(Signal { short: mean(short_window), long: mean(long_window) })
    }
}

/// Counter-cyclical trend in basis points, the exact `_calculateTrend` shape
/// (`TreasuryStabilizer.sol:436-441`): `(short − long) · 10000 / long`. Signed: negative = the recent
/// horizon sits BELOW baseline (a drawdown — the miner-revealed signal that JUL price is below production
/// cost, `DESIGN` §3), positive = above. Fail-closed on a zero baseline (`long == 0 ⇒ 0`). Computed in
/// `i128` so no intermediate overflows, then SATURATED (never a wrapping `as i64`) into `i64`. A bps trend
/// is usually small, but `Signal`'s fields are a swappable seam (a future price feed can report
/// large-magnitude values), and a wrapping cast could INVERT the sign — reading a boom as a deep bear and
/// deploying counter-cyclically backwards. Clamping preserves sign: an extreme rally saturates to
/// `i64::MAX`, an extreme drawdown to `i64::MIN` (still "deep bear").
pub fn trend_bps(sig: Signal) -> i64 {
    if sig.long == 0 {
        return 0;
    }
    let short = sig.short as i128;
    let long = sig.long as i128;
    let raw = ((short - long) * 10_000) / long;
    raw.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

// ============ Parameters (v0; migrate to a governable Constitution field at wiring) ============

/// Reserve parameters. ALL DEFAULT TO ZERO ⇒ the mechanism ships OFF (see the module INERT note). Every
/// value is OWED (`docs/DESIGN-jul-money-layer.md` §5); the defaults define no economics. All time-like
/// bounds are denominated in WORK units on the cumulative-work clock (`Ledger::now`, runtime.rs:196-202) —
/// NEVER wall-clock; pre-PoW work == height, so they degrade exactly to block counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ReserveParams {
    /// Accumulation rate: base units skimmed into the reserve per 10000 base units of coinbase issuance
    /// (the funding lever). 0 ⇒ the reserve never funds.
    pub skim_bps: u64,
    /// Bear threshold: the market reads bear when `trend_bps < -bear_threshold_bps` (`sol:157`), the
    /// comparison done in `i128` so the negation is TOTAL across the full `u64` domain (a naive
    /// `-(bps as i64)` panics in debug at `1<<63` and mis-signs at `u64::MAX`). NOTE: at the 0 default any
    /// negative trend classifies bear, but with `release_rate_bps == 0` no release can follow ⇒ still inert;
    /// once live it must be set as a NOISE DEADBAND (a post-PoW difficulty random walk trips a 0 threshold
    /// ~half the time), an inc-4 number.
    pub bear_threshold_bps: u64,
    /// Per-release cap as basis points of the CURRENT reserve balance (`sol:225`). 0 ⇒ never releases.
    pub release_rate_bps: u64,
    /// Absolute cap on total releases within one work-period (`sol:228-231`); releases clamp to the
    /// remaining budget.
    pub max_release_per_period: u128,
    /// Minimum work elapsed between assessments (`MIN_ASSESSMENT_PERIOD`, sol:45, 1h → work units).
    pub min_assessment_work: u64,
    /// Minimum work elapsed between releases (`deploymentCooldown`, sol:204-211) — hysteresis vs thrash.
    pub cooldown_work: u64,
    /// The work-denominated period window (`MAX_DEPLOYMENT_PERIOD`, sol:46, 7d → work units); the
    /// per-period release budget resets when it elapses. `0` DISABLES the per-period budget (releases are
    /// then bounded only by `release_rate_bps` and the balance floor) — like the other `0 ⇒ off` params.
    pub period_work: u64,
}

// ============ Reserve state (module-local; NOT a Ledger field, NOT in state_digest) ============

/// The counter-cyclical reserve's state machine. A pure, module-local value — it is not a `Ledger` field
/// and never enters `state_digest` (runtime.rs:188-194), so exercising it cannot perturb replay (the
/// increment-3 headline theorem, `jul_reserve.rs`). Holds a `u128` balance, not JUL cells, per the module
/// "WHY NO CELL" note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Reserve {
    /// Base units currently held by the reserve. Only [`accrue`](Self::accrue) grows it and only
    /// [`release`](Self::release) shrinks it, so `balance == Σ skims − Σ releases` is a conservation
    /// invariant proven by test.
    balance: u128,
    /// Base units released within the current work-period (reset on period expiry and on the bear→bull
    /// transition). Bounds the per-period budget.
    deployed_this_period: u128,
    /// Work-clock stamp of the current period's start.
    period_start: u64,
    /// Work-clock stamp of the last release (`0` = none yet).
    last_release: u64,
    /// Work-clock stamp of the last assessment (`0` = none yet).
    last_assessment: u64,
    /// Whether the market last read as a bear (drawdown) market.
    bear: bool,
}

impl Reserve {
    /// A fresh, empty reserve (fair launch: no pre-fund).
    pub fn new() -> Self {
        Reserve::default()
    }

    /// Current balance in base units.
    pub fn balance(&self) -> u128 {
        self.balance
    }

    /// Whether the market last read bear.
    pub fn is_bear(&self) -> bool {
        self.bear
    }

    /// FUNDING (the accumulate half). Skim `reward · skim_bps / 10000` base units of a block's coinbase
    /// issuance into the reserve and return the skimmed amount. This is the sole funding source — new
    /// energy money, never bought (the anti-plutocracy invariant). Floor division (biased down); the skim
    /// IS the "rebuild reserves in strength" clause. `skim_bps == 0 ⇒ 0` (inert).
    pub fn accrue(&mut self, coinbase_reward: u128, params: ReserveParams) -> u128 {
        let skim = coinbase_reward.saturating_mul(params.skim_bps as u128) / 10_000;
        self.balance = self.balance.saturating_add(skim);
        skim
    }

    /// ASSESS the market from an optional signal (`assessMarketConditions`, sol:133-168). Fail-closed:
    /// refused (state unchanged) if the assessment cooldown has not elapsed, or if `sig` is `None` (a
    /// missing signal is NEVER a synthesized trend). Otherwise recompute the bear flag and, on a bear→bull
    /// transition, reset the per-period release budget (sol:161-165).
    pub fn assess(&mut self, sig: Option<Signal>, now: u64, params: ReserveParams) {
        if params.min_assessment_work > 0
            && self.last_assessment != 0
            && now.saturating_sub(self.last_assessment) < params.min_assessment_work
        {
            return; // assessment cooldown
        }
        let sig = match sig {
            Some(s) => s,
            None => return, // no signal ⇒ no assessment (fail-closed)
        };
        let trend = trend_bps(sig);
        let was_bear = self.bear;
        // Compare in `i128`: the negation is total across the FULL `u64` domain of `bear_threshold_bps`
        // (`-(i64::MIN)` overflows — a naive `-(bps as i64)` panics in debug at `1<<63` and mis-signs at
        // `u64::MAX`). `i128` holds `-(u64::MAX)` with room to spare.
        self.bear = (trend as i128) < -(params.bear_threshold_bps as i128);
        self.last_assessment = now;
        if was_bear && !self.bear {
            // recovery: reset the period budget (sol:161-165)
            self.deployed_this_period = 0;
            self.period_start = now;
        }
    }

    /// Has the current work-period elapsed? (`period_work == 0` ⇒ never — the disabled default.)
    fn period_expired(&self, now: u64, params: ReserveParams) -> bool {
        params.period_work > 0 && now > self.period_start.saturating_add(params.period_work)
    }

    /// Release budget already spent this period, accounting for period expiry WITHOUT mutating — this is
    /// the deterministic fix for the Solidity view-function wart (sol:214-217, "can't modify state in a
    /// view function"): the query computes the correct number; [`release`](Self::release) persists the
    /// reset.
    fn effective_deployed(&self, now: u64, params: ReserveParams) -> u128 {
        if self.period_expired(now, params) {
            0
        } else {
            self.deployed_this_period
        }
    }

    /// The RELEASE DECISION (`shouldDeployBackstop`, sol:194-234), pure/non-mutating: releases only in a
    /// bear market, only past the cooldown, bounded by both the per-release rate and the remaining period
    /// budget, and never more than the balance held. Returns `(should, amount)`.
    pub fn should_release(&self, now: u64, params: ReserveParams) -> (bool, u128) {
        if !self.bear {
            return (false, 0);
        }
        if params.cooldown_work > 0
            && self.last_release != 0
            && now.saturating_sub(self.last_release) < params.cooldown_work
        {
            return (false, 0); // release cooldown
        }
        let by_rate = self.balance.saturating_mul(params.release_rate_bps as u128) / 10_000;
        let remaining = params
            .max_release_per_period
            .saturating_sub(self.effective_deployed(now, params));
        let amount = by_rate.min(remaining).min(self.balance);
        (amount > 0, amount)
    }

    /// EXECUTE a release (`executeDeployment`, sol:241-314): persist any period-expiry reset (sol:263-266),
    /// then debit the bounded amount computed by [`should_release`](Self::should_release), stamp the
    /// cooldown, and accrue the per-period budget. Returns the amount released (`0` if nothing is due).
    /// Fail-closed by construction: the amount is `≤ balance`, so the balance can never go negative and the
    /// reserve can never mint. The released JUL's DESTINATION is a seam (see the module note).
    pub fn release(&mut self, now: u64, params: ReserveParams) -> u128 {
        if self.period_expired(now, params) {
            self.deployed_this_period = 0;
            self.period_start = now;
        }
        let (should, amount) = self.should_release(now, params);
        if !should {
            return 0;
        }
        self.balance -= amount; // amount ≤ balance (should_release clamps with `.min(self.balance)`)
        self.deployed_this_period = self.deployed_this_period.saturating_add(amount);
        self.last_release = now;
        amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_is_zero_on_a_flat_series() {
        // Pre-PoW: block_work ≡ WORK_PER_BLOCK ⇒ a flat series ⇒ trend 0 ⇒ never bear.
        // RED break: divide by the short mean instead of the long, or an off-by-one in the window slice.
        let works = [1u64; 8];
        let sig = Signal::from_work_series(&works, 2, 4).unwrap();
        assert_eq!(sig, Signal { short: 1, long: 1 });
        assert_eq!(trend_bps(sig), 0);
    }

    #[test]
    fn trend_sign_matches_the_twap_shape_both_directions() {
        // Falling series ⇒ negative bps; rising ⇒ positive; the exact (short−long)·10000/long shape.
        // RED break: flip the sign convention.
        // long mean (last 4) = (10+10+6+6)/4 = 8, short mean (last 2) = 6 ⇒ (6−8)*10000/8 = −2500.
        let falling = [10u64, 10, 6, 6];
        let s = Signal::from_work_series(&falling, 2, 4).unwrap();
        assert_eq!(s, Signal { short: 6, long: 8 });
        assert_eq!(trend_bps(s), -2500);
        // rising: long = 8, short (last 2) = 10 ⇒ (10−8)*10000/8 = +2500.
        let rising = [6u64, 6, 10, 10];
        let r = Signal::from_work_series(&rising, 2, 4).unwrap();
        assert_eq!(r, Signal { short: 10, long: 8 });
        assert_eq!(trend_bps(r), 2500);
    }

    #[test]
    fn insufficient_history_yields_no_signal() {
        // Fail-closed: fewer than long_window samples ⇒ None ⇒ assess is a no-op.
        // RED break: return Some with a partial/zero-padded window.
        assert_eq!(Signal::from_work_series(&[1, 2, 3], 2, 4), None);
        assert_eq!(Signal::from_work_series(&[1, 2, 3, 4], 0, 4), None);
        assert_eq!(Signal::from_work_series(&[1, 2, 3, 4], 3, 2), None); // short > long
    }

    #[test]
    fn zero_params_are_fully_inert() {
        // ReserveParams::default() all-zero ⇒ no funding, no release, even in a crafted deep bear.
        // RED break for the accrue assertion: a nonzero `skim_bps` default. RED break for the release
        // assertions: nonzero `release_rate_bps` AND `max_release_per_period` defaults together — either
        // alone stays inert (a zero budget OR a zero rate each floors the release to 0).
        let p = ReserveParams::default();
        let mut r = Reserve::new();
        assert_eq!(r.accrue(1_000_000, p), 0, "no skim at zero rate");
        assert_eq!(r.balance(), 0);
        // force a deep bear directly, then confirm nothing releases.
        r.balance = 1_000_000; // (test-only: simulate a funded reserve)
        r.bear = true;
        assert_eq!(r.should_release(999, p), (false, 0), "zero release rate ⇒ nothing due");
        assert_eq!(r.release(999, p), 0);
        assert_eq!(r.balance(), 1_000_000, "inert: balance untouched");
    }

    #[test]
    fn trend_bps_saturates_and_preserves_sign_on_large_magnitudes() {
        // `Signal`'s fields are a swappable seam (a future price feed can report huge values). A wrapping
        // `as i64` would INVERT the sign (a rally reading as a deep bear); saturation preserves it.
        // RED break: revert trend_bps to a bare `((short-long)*10000/long) as i64` cast.
        let rally = Signal { short: u64::MAX, long: 1 };
        assert_eq!(trend_bps(rally), i64::MAX, "an extreme rally saturates to +max, never wraps negative");
        let crash = Signal { short: 1, long: u64::MAX };
        assert!(trend_bps(crash) < 0, "an extreme drawdown stays negative");
    }

    #[test]
    fn assess_is_total_across_the_full_threshold_domain() {
        // `-(bear_threshold_bps as i64)` would panic (debug) / mis-sign at `1<<63` and `u64::MAX`; the
        // i128 comparison is total. RED break: negate the threshold as i64.
        let mut r = Reserve::new();
        for thr in [0u64, 1 << 63, u64::MAX] {
            let p = ReserveParams { bear_threshold_bps: thr, ..Default::default() };
            r.assess(Some(Signal { short: 99, long: 100 }), 1, p); // must not panic across the domain
        }
        // at an astronomically-high threshold a mild −100 bps drawdown must NOT read bear.
        let p = ReserveParams { bear_threshold_bps: 1 << 40, ..Default::default() };
        let mut r2 = Reserve::new();
        r2.assess(Some(Signal { short: 99, long: 100 }), 1, p);
        assert!(!r2.is_bear(), "a mild drawdown must not trip a very high bear threshold");
    }

    #[test]
    fn assess_respects_the_assessment_cooldown() {
        // Two assessments within `min_assessment_work` ⇒ the second is refused (the first's reading
        // persists); at the boundary the new signal applies.
        // RED break: drop the cooldown early-return, compare with `<=` off-boundary, or stamp
        // `last_assessment` from anything but `now`.
        let p = ReserveParams { bear_threshold_bps: 100, min_assessment_work: 10, ..Default::default() };
        let mut r = Reserve::new();
        r.assess(Some(Signal { short: 90, long: 100 }), 100, p); // −1000 bps ⇒ bear
        assert!(r.is_bear());
        r.assess(Some(Signal { short: 110, long: 100 }), 105, p); // 5 < 10 ⇒ refused
        assert!(r.is_bear(), "assessment within the cooldown is refused ⇒ bear persists");
        r.assess(Some(Signal { short: 110, long: 100 }), 110, p); // now−last == 10 ⇒ applies
        assert!(!r.is_bear(), "assessment at the cooldown boundary applies");
    }
}
