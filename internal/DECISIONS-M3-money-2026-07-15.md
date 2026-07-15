# M3 money-layer decisions — RATIFIED (Will, 2026-07-15)

> Authoritative ruling set from the M3 economics sitting. Supersedes conflicting text in
> `docs/DESIGN-M3-jul-economics.md` §0.1/§4/§10 and `internal/DESIGN-elastic-pow-money.md`.
> Next window wires from THIS file. Sources verified this session (not memory): Ergon whitepaper
> (Trzeszczkowski 2021, `ergon.moe/prop-reward.pdf`) + VibeSwap Trinomial Stability Theorem
> (`vibeswap/docs/research/theorems/TRINOMIAL_STABILITY_THEOREM.md`).

## The one-token / three-mechanism vs three-token distinction (DO NOT CONFLATE — [[no-false-pattern-matching]])
- **TSS = 3 stability MECHANISMS inside ONE token (JUL)** — L1 anchor / L2 PI / L3 rebase. Money's stability layers.
- **3-token economy = JUL / VIBE / CKB-native** — three FUNCTIONS (money / governance / state-rent capital). Orthogonal.
- These are different threes. The Session-067 false-match connected them; never repeat it.

## RULED

### 1. Money peg = TSS **Layer 1 ONLY** at launch (Ergon anchor + Moore's-law correction)
- `reward = e^(−a_estim·t) · difficulty` — proportional PoW reward (Ergon §2/§5), scaled by a **smooth
  per-block exponential Moore's-law decay** `e^(−a_estim·t)` (Ergon §5.3 `f(h)=e^(−a_estim·t)·h(t)`).
- **1 JUL ≡ one genesis-block of ENERGY** (not one block of hashes — that's the whole point of Moore's):
  `reward_num = 1e8` (fixed, `jul.rs:31`), `reward_den = W_g` (genesis expected work/block, measured at build).
- **Moore's is CORE to the peg, not deferred** — a flat (no-decay) reward pegs to hashes, and since hashes/joule
  rises with hardware, that silently inflates JUL against energy. The decay holds the ENERGY peg. This
  **supersedes** DESIGN-M3 §4/§10#9 ("Moore's DEFERRED/OFF") and the 2026-07-14 "flat proportional is final".
- `a_estim` = estimated hardware-efficiency rate, **governable**, revisable. Illustrative starting figure:
  efficiency ~doubling every 3 years (Ergon §5.3; ASIC 5.0M→15 J/TH 2009–2025 ≈ 333,000×). NOT a hardcoded
  half-life; over-estimate → mild inflation, under → mild deflation, both only at decade timescales.
- **Keyed to the attested wall-clock** (committee clock / block timestamp), NOT cumulative work — Moore's law
  is calendar-based. (Minor: puts the timestamp into issuance = small bounded grind surface; note at wiring.)
- **No launch ramp** (dropped 2026-07-14 — JUL elastic ⇒ no deep-capital windfall). **No discrete halving**
  (the smooth exponential replaces Bitcoin-style halving).
- **RATIONALE (the plain-English *why*):** Will's blog `docs/research/stable-base-money-ergon-rationale.md`
  — the deflationary-base paradox (defi builds stability on hyper-deflationary collateral ⇒ boom/bust),
  and Ergon's fix: **costliness ≠ deflation** (keep gold/BTC costliness, drop the scarcity-deflation via a
  proportional/elastic reward). L1 = the stable base; public-goods "building blocks" compose on top.

### 2. TSS **Layer 2 (PI controller) + Layer 3 (elastic rebase) = OFF the base layer** (DeFi-derivative, not consensus)
- **Reason (Will):** L2 and L3 both depend on the **CPI + electricity-price oracles** (TSS §5.2 dual-oracle;
  our AMD upgrade = bonded-verified-compute dispute). That oracle design is trust-minimization-hard and should
  NOT be built solo/prematurely, and MUST NOT sit in base consensus. Defer until enough people are on it.
- **PREFERRED ARCHITECTURE (Will 2026-07-15, "worst case we just stick with L1"):** L1 is the **permanent base
  money**; the stability tiers become **elastic/stable pegged DERIVATIVES built on the DeFi layer on top of L1**,
  NOT baked into Noesis consensus. Lower risk: oracle dependency is **opt-in at the derivative layer**, never
  consensus-critical. This is the natural home anyway — the TSS is a **VibeSwap paper**, and L2/L3 (RAI PI +
  AMPL rebase) are **DeFi mechanisms**; VibeSwap (or any DeFi layer) builds the pegged stable on JUL-L1. The
  full trinomial still exists as a system — correctly LAYERED (base = honest energy money, app = stability
  derivatives) instead of monolithic. Matches the "pure-utility base, no bolted-on speculative token" thesis.
- ⇒ Base-layer L2/L3 is not merely "deferred pending a team" — it likely **belongs on a different layer entirely**.
  Noesis base ships **L1 only, oracle-free, forever-viable**; stability is an application-layer product.
- L1 alone needs **no external oracle** (endogenous PoW difficulty only) ⇒ self-contained, launchable, honest.
- Verified constants for when L2/L3 are built (TSS paper, carry the VibeSwap audit fixes MED-6):
  - **L2 PI:** Kp = 7.5×10⁻⁸, Ki = 2.4×10⁻¹⁴, leaky-integrator α = 0.9999997112 = **120-day half-life** (§4.2).
    Carry fix: clamp per-tick, cap integrator windup.
  - **L3 rebase:** ±5% equilibrium band, rebaseLag = 10, O(1) global scalar, dual oracle (§5). Carry fix:
    MAX_REBASE_SCALAR per-rebase clamp (~100bps), multi-oracle MEDIAN (not owner-set).
  - Theorem (§7.2): the three composed converge `Var(p) → σ²_elec` (~2–5%/yr). L1 ALONE oscillates.

### 3. HONESTY CONSTRAINT (launch copy)
- Launch claims **"energy-anchored money"** only — NEVER the full stability theorem / σ²_elec floor / "stable".
- L1 alone = energy-pegged **and oscillating**. Honest label: "energy-pegged; oscillation-damping tiers L2/L3
  designed-not-built, deferred pending oracle design + contributors." ✅ built · 🟡 designed · never round up.

### 4. infra_bps = **NONE** (no coinbase public-goods skim, ever, by design)
- Infrastructure is funded as **measured contribution** (PoM standing earns rewards for infra work) + **VIBE**
  governance direction — not a redistributive coinbase tax. This IS the "measured contribution dissolves
  redistribution" thesis applied to infra. `coinbase_split` stays empty (current default), permanently.
- Supersedes DESIGN-M3 §4/§10#7 ("recommend infra_bps ∈ [200,500]").

### 5. `vesting_w` (finality vote-weight cliff) = **STAYS** — clarification, no change
- `vesting_w` gates `finality_pom_weight` ONLY (`runtime.rs:1058`, applied `:147-160`) — when freshly-earned
  **PoM finality VOTE weight** activates. It does NOT touch JUL, rewards, or any money. PoM standing is
  soulbound/non-transferable ⇒ nothing is redistributed. It's the anti-circularity guard (stops fresh standing
  finalizing the block that minted it). JUL money has ZERO vesting (flat per §1). Confirmed stays.

### 6. Deferred / my-lead (unchanged from the §10 table)
- **Reserve activation set** (skim/deadband/caps): all 0 until the inc-4 game-theory pass. Deferred.
- **`work_clock_ceiling K`** (my lead): finite + generous, never clamps an honest block; the only param that
  MUST be non-inert to enable `pow_enforced`. Since `vesting_w` launches at 0, K can be loose now, tighten if W
  activates. `assert!` precondition already enforces finite-K ⇒ pow_enforced (`runtime.rs:902`).
- **ASERT retarget window** (my lead): block-count first cut, Phase-2 only, inert at launch. Half-life = 2d
  (ratified). δ=120s, max_staleness=300s, max-forward-skew=7200s (ratified 2026-07-14).

## NEXT-WINDOW WIRING PLAN (autopilot, no ⚑ left)
1. **Reconcile the docs to this file** — DESIGN-M3 §0.1/§4/§10 (Moore's IN, L1-alone, infra none) + fix
   DESIGN-elastic-pow-money's "~2.3yr half-life" hallucination (real = a_estim ~3yr-doubling illustrative on L1
   + 120d PI on L2; two different half-lives) + reconcile the "Lever A/B reserve" framing to the TSS 3-tier.
2. **Build L1 Moore's decay** behind the existing `ERGON SEAM` (`jul.rs:96`): `reward_for_work` gains an
   `e^(−a_estim·t)` coefficient keyed to the attested clock; `a_estim` a governable Constitution field; ships
   with a decay of 0 ⇒ byte-identical until set (inert-default precedent). RED-first: identical energy → identical
   JUL across a simulated efficiency-doubling; flat-vs-decayed divergence test.
3. **Leave L2/L3 unbuilt** — a one-paragraph DESIGN stub citing the verified constants + the deferral reason, so
   the team that picks up the oracle work has the spec. Do NOT build the oracles.
4. **K + genesis chain-spec** — set a generous finite K, wire the Phase-1 genesis (pow_enforced from block 0,
   genesis_bits measured at build) — the deploy-pole item.
