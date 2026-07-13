# DESIGN — JUL, the money layer (design note; build to follow)

> Status: **designed, not built.** This note reconciles JUL's design across the two places it
> lives — the noesis tokenomics docs (`TOKENOMICS.md`, `CRYPTOECONOMICS.md`) and the prior
> money-stability work (`TreasuryStabilizer.sol`, a sibling codebase) — into a single
> build plan. Nothing here ships yet; where a number or mechanism is owed, it says so.
> Grounded per [[never-assert-noesis-protocol-numbers-from-memory]] — read the cited sources, not memory.

## 1. What JUL is, and why it exists

JUL is the **money / medium-of-exchange** token: transferable, PoW-issued, energy-pegged, designed to
be **spent, not hoarded.** It is one of the three separated powers (money / governance / capital) and
is deliberately the mirror image of PoM-standing: standing is scarce, inelastic, and unbuyable; JUL is
elastic and made to circulate.

The one job a medium of exchange must do that state-bytes cannot is **price stability** — volatile
bytes make poor money (`TOKENOMICS.md` §JUL, `CRYPTOECONOMICS.md`). Delivering that stability is JUL's
whole reason to exist, and it is the piece that is currently *designed but unbuilt*.

Two hard constraints from the surrounding design:
- **JUL is PoW-objective.** Adding JUL is exactly what *brings Proof of Work back* into Noesis — at the
  money layer, where energy turns into sound money, which is the one job PoW does better than anything
  (`CRYPTOECONOMICS.md`: "GEV-conservation — PoW is conserved, relocated to where it belongs").
- **JUL is excluded from finality.** PoW is reorgeable, so it stays off the safety path (`FINALITY_MIX`).
  JUL secures *money, production, ordering, and Sybil-cost* — never the finality decision.

## 2. JUL's role at genesis (per the settled genesis decision, 2026-07-13)

Locked in [[project_noesis-genesis-bootstrap-decision]]: **PoW starts genesis, bonded PoS finalizes,
handoff to PoM.** JUL is the concrete form of "PoW starts genesis":

- At t=0 the endogenous powers (PoM, and PoS-derived-from-PoM) are zero. JUL is the only value that can
  be issued from **outside** the ledger with **no pre-mine and no allocation** — energy → JUL,
  permissionlessly, Bitcoin-style. So JUL issuance is the fair-launch money bootstrap.
- A small bonded PoS set carries genesis *finality* in parallel (JUL cannot — PoW is finality-excluded).
- As PoM accrues, finality migrates to PoS+PoM; JUL recedes from any security role and settles into its
  steady-state job: being the money.

So JUL is issued from block zero (money + liveness), never finalizes anything, and is the exogenous
half of the dynamic genesis mix.

## 3. Stability — the owed mechanism — is two levers, not one

The reconciliation of the two source designs is that they solve **different time-horizons** of the same
problem, and compose:

### Lever A — production-cost anchor (Ergon-style; the long-run value)

The primary anchor is the same one Ergon uses: **JUL's value gravitates to the marginal cost of
producing it.** PoW difficulty ties issuance to real energy expended, so:
- when JUL's price rises above production cost, mining is profitable → hashrate rises → difficulty rises
  → the cost floor rises toward the price;
- when price falls below cost, miners leave → difficulty falls → issuance slows.

Supply is therefore **elastic to demand through an energy cost anchor**, which resists *both* failure
modes of naive designs: the unbounded inflation of a printable token and the deflationary hoarding of a
hard cap. "Ergon-style" is a **design descriptor**, not a dependency and not a fourth token
(`TOKENOMICS.md` §Naming). This lever is what makes JUL *fundamentally* sound money.

### Lever B — counter-cyclical backstop (ported from `TreasuryStabilizer`; the short-run smoother)

The production anchor sets the long-run level; it does not damp short-run volatility around that level.
That is exactly what the prior money-stability work does, and it ports directly. `TreasuryStabilizer.sol`
is a **counter-cyclical treasury**: it assesses market conditions via TWAP (short/long) plus a
volatility oracle, detects drawdowns (`isBearMarket` / `_calculateTrend`), and deploys bounded treasury
reserves as a backstop during bear markets (`shouldDeployBackstop` → `executeDeployment`), withdrawing on
recovery (`withdrawDeployment`, with durable failure/retry). Its guardrails are already thought through:
`MIN_ASSESSMENT_PERIOD = 1h`, `MAX_DEPLOYMENT_PERIOD = 7d`, per-token config, pause + emergency mode.

For JUL this becomes a protocol-owned reserve that leans against the wind: buy support in drawdowns,
rebuild reserves in strength, always bounded, never a promise to hold a fixed peg. It is a smoother, not
a peg — which keeps it honest (no Terra-style "defend the peg to zero" failure mode).

### Why two levers

Lever A answers "what is JUL fundamentally worth?" (cost of energy to make it). Lever B answers "how do
we keep the market price from thrashing around that value day to day?" (counter-cyclical reserve). A is
structural and unkillable; B is operational and bounded. Neither alone is sufficient; together they are a
money layer that is both anchored and calm.

## 4. Build plan (design-first, then code — per Will 2026-07-13)

Increment order, cheapest-first, each shippable and testable on its own:

1. **JUL issuance core — ✅ BUILT (`e51e164`).** `node/src/jul.rs`: the pure integer issuance rule
   `reward_for_work(work, num/den)` (Lever A; pre-PoW degrades to a flat per-block subsidy, becomes
   difficulty-proportional when `block_work` returns mined difficulty — no rule change) + `JulSupply`
   (monotone, no cap by design). Built ADDITIVE/SHADOW — imports nothing from consensus, called from no
   consensus path, so replay-parity is provably unaffected. 9 tests (6 integration + 3 unit), each with
   a named anti-theater break; clippy-clean; `apply_block_parity`/`two_node_join` unregressed. Numbers
   are v0 unit-definitions, NOT pinned economics (§5). Two Fable-5 planners scoped it (mechanism +
   build-safety); the leaner "no difficulty-retarget until real PoW inputs exist" scope was taken.
2. **JUL as a transferable token — 🔨 DESIGNED (two-planner-vetted), building.** Settle each block's
   issuance as a `Fungible` coinbase. CONSENSUS-AFFECTING (mints into `token_cells` → changes state), so
   NOT additive-shadow. Converged design:
   - **Coinbase = a `Block.coinbase: Option<Script>` field carrying ONLY the recipient lock** (Bitcoin
     "coinbase in the block"). The amount is CONSTRUCTED in `apply_transition` as
     `reward_for_work(block_work(b), constitution.jul)` — a forged amount is *unrepresentable*, not merely
     rejected (construct-don't-validate). `None` ⇒ no mint ⇒ all pre-existing blocks/tests unchanged.
   - **⚠ SECURITY FINDING (both planners, independently): JUL mint-via-token-tx hole.** The token model
     derives mint authority from a consumed input whose `lock.args == issuer`. A holder could pay JUL to a
     cell with `lock.args == JUL_ISSUER`, then consume it next block as an "authority input" and mint
     UNBOUNDED JUL through the ordinary token path — making the coinbase decorative. **Fix (load-bearing):
     a JUL-conserve-only clause in `token_txs_conserve_and_single_use` — any JUL-identity tx must have
     `Σ outputs ≤ Σ inputs`, regardless of authority.** ⇒ the coinbase is the *structurally unique* JUL
     inflation channel; also keeps the FV spec-oracle (`Σout>Σin ⇒ reject`) honest. Plus a reserved
     coinbase-id space (`1<<63 | height`) barred from token-tx outputs (prevents retirement-collision griefing).
   - **`Ledger.jul_supply` (u128) EXCLUDED from `state_digest`** (finalized_at precedent — replay-derivable;
     keeps the digest tuple unchanged ⇒ zero churn in noesisd/sync/parity tests). Conservation
     (`jul_issued == Σ live JUL + replay-derived burns`) is a theorem of the transition, proven by test, not a runtime check.
   - **Recipient = producer-named** (the field a real miner fills at the PoW increment; amount is
     protocol-fixed so recipient choice can only direct the fixed subsidy, never enlarge it). One-line
     Will-gated seam for alternatives (treasury / burn).
   - **JUL never touches finality** — it lives in `token_cells`, finality weight folds only over `cells`.
   - `Constitution.jul: JulParams` added now (the "wiring" jul.rs anticipated; governance-*bounding* it
     stays increment 4). v0 identity constants are placeholders until the on-VM type-script hash lands.
3. **Counter-cyclical reserve (Lever B) — ✅ MECHANISM BUILT, SHADOW (inc-3, `node/src/reserve.rs`).** The
   `TreasuryStabilizer` POLICY ported to a pure, integer-only, consensus-isolated node module: `trend_bps`
   (the exact `_calculateTrend` shape over a work-series `Signal`), bear classification, and bounded
   `release` with per-release-rate + per-period caps + cooldown, all on the cumulative-work clock
   (`Ledger::now`), NEVER wall-clock. Funded by a coinbase skim (`accrue`). What DISSOLVED from the
   Solidity original (documented, not omitted): the AMM/TWAP/volatility-oracle (Noesis has no on-chain
   price — the work-trend proxy replaces it, and it is a swappable seam), the DAOTreasury/owner/pause/UUPS
   (all-zero fail-closed params ARE the pause), and `withdrawDeployment`+retry (no reversible LP position ⇒
   release is irreversible subsidy and "rebuild" is the ongoing skim ⇒ an ASYMMETRIC accumulate-then-
   subsidize smoother). Additive/shadow: no `Ledger` field, `state_digest` byte-identical, replay-parity
   untouched — proven (full lib suite + `apply_block_parity`/`two_node_join`/`jul_settlement` green; 11
   anti-theater tests incl. the shadow-parity + `money_never_buys_standing` theorems in
   `node/tests/jul_reserve.rs`). Ships provably OFF (`ReserveParams::default()` all-zero). **NOT YET a live
   lever:** the consensus wiring (skim/top-up at the coinbase-mint site + a protocol-spend-only reserve
   cell — required because `CONTROL_BINDING_ACTIVE == false`, runtime.rs:411, makes a keyless reserve cell
   anyone-spendable today) is **inc-3b**; governance-bounding all params + the miner-reflexivity
   game-theory pass is **inc-4**. Both gated on real numbers Will has not set.
4. **Genesis wiring** — issue JUL from block zero on the PoW path; keep it out of `FINALITY_MIX`.
5. **Honest tests** — issuance responds to difficulty in the modeled direction; the backstop deploys in a
   simulated drawdown and withdraws on recovery, always within bounds; JUL never enters the finality
   weight. Anti-theater RED checks on each.

## 5. Open questions / honest gaps

- **Parameters unset.** Difficulty-adjustment cadence, issuance rate, reserve size, deployment bounds —
  all owed. This note fixes the *mechanism*, not the *numbers*.
- **NCI weight reconciliation.** The implemented NCI uses fixed weights (PoM / state-stake / PoW); the
  capital/compute/cognition framing puts JUL/PoW as one leg of the cycle. Reconciling the fixed weights
  with the framing is a tracked open item (`CRYPTOECONOMICS.md` "DIVERGES from implemented NCI";
  `CONSENSUS-REVIEW.md`).
- **Reserve funding source — MECHANISM DECIDED (coinbase skim), rate/activation OWED (inc-3).** Lever B is
  funded ONLY by a protocol-fixed slice (`skim_bps`) of newly-constructed coinbase issuance — the sole
  non-plutocratic source that exists (fees are unbuilt; external funding IS the forbidden backdoor). It is
  new energy money that was never any participant's property, so funding buys NOTHING: no PoM standing, no
  PoS weight, no governance voice — enforced structurally (JUL lives in `token_cells`; PoM folds over
  `cells` only; `FINALITY_MIX` excludes PoW) and made executable by the `money_never_buys_standing` test.
  The RATE (`skim_bps`) and turn-on remain owed economics: `skim_bps` defaults to 0 ⇒ zero funding until
  Will sets it; at wiring it becomes one `Constitution` field beside `Constitution.jul`. Any issuance-slice
  must fold INTO the coinbase (never a second mint channel) so the coinbase stays the structurally unique
  inflation channel (runtime.rs:746-768).
- **Signal is a manipulable, sign-ambiguous proxy (inc-4 game-theory gate).** The work-trend is (a)
  ENDOGENOUS — a producer can throttle hashrate to trip bear and harvest a release (reflexivity, a
  manipulation, not a passive confounder like energy shocks / hardware jumps / lag); (b) SIGN-AMBIGUOUS —
  it cannot separate a JUL-price drawdown (release is correctly counter-cyclical) from an input-cost shock
  (release is mis-targeted, mildly pro-cyclical); (c) it needs `bear_threshold_bps` set as a NOISE DEADBAND
  (a post-PoW difficulty random walk trips a 0 threshold ~half the time). All three are why the signal
  SOURCE is a swappable seam and why any nonzero params await the inc-4 miner-reflexivity pass. Ships OFF
  until then.
- **Ergon fidelity.** "Ergon-style" is the target; the exact difficulty→issuance proportionality to adopt
  should be pinned against Ergon's actual public design before Lever A is finalized.

## 6. One-line summary

JUL is PoW money that **starts the genesis by being issued from energy**, **never finalizes anything**,
and stays stable through **two levers** — a production-cost anchor (Ergon-style, long-run) and a bounded
counter-cyclical reserve (TreasuryStabilizer-style, short-run). Design reconciled; build is the next step.
