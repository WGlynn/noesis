# VIBESWAP → NOESIS THESIS FIDELITY

> **PRIVATE — Noesis, leak-gated.** Synthesis audit, 2026-07-03. Origin: Will's concern —
> *"I can't afford to abandon vibeswap's original thesis just because we're making a sovereign
> chain."* The risk named: sovereignty tempts drift into standard-chain patterns the thesis
> rejects — possession-ledger, PoS plutocracy, fee extraction, MEV. Noesis must be the thesis
> realized DEEPER (at the substrate), not abandoned.
>
> **Status discipline (HONEST-NUMBER, never round up):**
> **preserved** = enforced in code/design with evidence · **at-risk** = weak/implicit ·
> **drifted** = design contradicts or abandons the tenet.
> Every claim below carries a `file:line` or verbatim doc quote. Line numbers verified against
> the working tree 2026-07-03; a handful re-verified and corrected during synthesis.

---

## 1. THE THESIS (north star)

The six tenets VibeSwap exists for. Noesis is only legitimate as their deeper realization.

| # | Tenet | VibeSwap anchor |
|---|-------|-----------------|
| T1 | **A coordination primitive, not a casino** | Locked tagline [J·vibeswap-tagline]; infrastructure, not a speculation venue |
| T2 | **Contribution chain, not possession chain** | PoM standing = source of franchise; who-holds-what is never the organizing fact |
| T3 | **MEV / ordering-extraction elimination** | CommitRevealAuction + DeterministicShuffle + uniform clearing; "we eliminate ordering-based extraction" (vibeswap/WHITEPAPER.md:3,40,190) |
| T4 | **GEV elimination — the extraction CLASS dissolved** | GEV_RESISTANCE.md:10-18; from-mev-to-gev.md; dissolve structurally, never patch per-instance |
| T5 | **Cooperative capitalism — mutualized risk + free-market competition** | cooperative-capitalism.md; W_coop > W_extractive; insurance pools + priority markets |
| T6 | **Value to contributors, not extractors / airgap closed via structural honesty** | Soulbound un-buyable standing; dishonesty structurally unprofitable; no trusted bridge to reality |

---

## 2. FIDELITY LEDGER

**Scoreboard: 4 preserved · 2 at-risk · 0 drifted.** Every "preserved" carries recorded
residuals (listed); none is rounded up to "solved."

### T1 — Coordination primitive, not a casino — **preserved**

- Coordination IS the block content in built code: `node/src/runtime.rs:15-17` — "The block
  shape (a commit-reveal batch ordered by a consensus-sourced shuffle) is derived from
  VibeSwap's CommitRevealAuction; the per-contributor PoM attribution it settles..." The block
  loop settles **attribution**, not trades or bets.
- Intent verbatim: `docs/MANIFESTO.md:24-27` — "Noesis is where cooperative capitalism stops
  being a set of contracts and becomes the substrate."
- Anti-casino money design explicit: `docs/TOKENOMICS.md:85-88` — JUL "designed to stay
  roughly stable and to be **spent, not hoarded**."
- No extraction surface: grep `fee|rake|house|priority_bid` over `node/src` is clean (only
  'feed'/`0xFEED_FACE` false positives). **No protocol fee path exists in code.**
- Franchise off the speculative layer: `node/src/lib.rs:44-49` — contributor identity "NEVER
  reassigned on transfer — this is what keeps consensus franchise off the transferable byte."
- Fair launch ratified: `docs/COHERENCE-LAWS.md:179-182` — genesis-burn, "no creator
  pre-launch advantage; neutralization must be on-chain-provable, not asserted" (candidate L14).
- Product framing: `docs/TOKENOMICS.md:148-149` — "The product is allocative efficiency —
  getting value to the people who created it."

**Residuals:** JUL stability mechanism "still owed" (`TOKENOMICS.md:157` open items); the two
transferable layers (state-bytes, JUL) are the casino aperture — see §3.

### T2 — Contribution chain, not possession chain — **preserved**

Enforced at three code layers:
1. `node/src/lib.rs:41-49` — Cell splits transferable `lock` (owner) from soulbound
   `type_script` (contributor, set at mint, never reassigned).
2. `node/src/lib.rs:156-161` — `pom_scores` keyed by `type_script.args`, **NOT** `lock.args`:
   "Consensus franchise therefore tracks soulbound standing, never bought bytes (buy storage,
   not consensus). Resolves the POM-CONSENSUS 'transferable credit' vs CRYPTOECONOMICS
   'soulbound standing' contradiction in favor of the latter" — i.e., this exact drift already
   surfaced once in docs and was resolved TOWARD the tenet.
3. `node/src/lib.rs:~453-520` `soulbound::valid_transition` + regression tests
   (`reassign_owner_is_rejected`, `reattribute_contributor_is_rejected`): moving standing to
   a new owner key and reattributing the contributor are both REJECTED — "that rejection IS
   the soulbound guarantee."
- Possession's voice bounded: PoS = 0.30 of NCI (`lib.rs:3683`), 1/3 of `FINALITY_MIX`
  (`runtime.rs:583-587`), floored by `MIN_DIM_BPS = 5000` — "A CONSTITUTIONAL constant...
  not governance-tunable" (`runtime.rs:588-595`). Capital participates in finality but cannot
  organize it.
- Doc anchor: `docs/COMPETITIVE-POSITION.md:15-26` ("Every standard chain is a possession
  chain... Noesis is a value chain"); `docs/TOKENOMICS.md:113-124` ("you cannot buy a say in
  what counts as contribution").

**Residuals (sharp — see §3.3):** `Constitution::default()` sets `decay_pos: false`
(`runtime.rs:76`) — contribution decays, capital doesn't; the code names the drift itself:
"NCI's **non-decaying PoS** drifts the effective mix toward capital under staleness"
(`lib.rs:3668`). Fix coded and tested but NOT default. NCI weight reconciliation open
(`TOKENOMICS.md:159-161`). p+ε vote-rental side channel open (`COMPETITIVE-POSITION.md:146-152`).

### T3 — MEV / ordering-extraction elimination — **AT-RISK**

**Ordering half enforced:**
- `runtime.rs:353-357` — "Block — a commit-reveal batch in canonical order... no producer can
  bias which cell lands in which slot"; `runtime.rs:451` `is_canonical_order(&b.coords)` in
  `validate` — producer-favorable reorder rejected before any state math.
- `lib.rs:9181-9187` — intra-block Fisher-Yates "seeded by the XOR of EVERY revealing
  participant's secret — the VibeSwap DeterministicShuffle primitive... DISSOLVES
  producer-favorable ordering"; pinned by tests at `lib.rs:9225-9312`.
- Ordering rule runs on-VM: `docs/TEMPORAL-ORDER-ONCHAIN.md:92-96`, `onchain/commit-order-typescript`.
- No fee path: grep `fee|priority|tip|gas` over `node/src` = zero hits; Bound-B deposit is
  refund-or-burn, paid to no one (`docs/RESOURCE-DOS-BOUNDING.md:52-56`).

**The at-risk gaps (self-recorded in the repo):**
1. **Coord provenance NOT bound** — on-VM re-derivation of (height, secret) is "NOT YET
   (deploy-coupled)... Gated INERT behind COORDS_BOUND" (`TEMPORAL-ORDER-ONCHAIN.md:97-101`);
   the repo's own pin proves "a falsely-earlier claimed height steals novelty"
   (`lib.rs:9139-9162`).
2. **Leader inclusion discretion unconstrained** — `propose()` assembles from its OWN mempool
   (`runtime.rs:432-435`); `validate` checks order, not completeness. Mempool novelty
   front-running excluded only "BY COMPOSITION" with commit-reveal plumbing that is itself
   unbuilt (`docs/T7-CROSS-CELL-SIMILARITY.md:54-56` — the reference mempool holds
   already-revealed cells).
3. **No uniform-clearing trade settlement on-chain yet** — blocks settle attribution, not
   trades (`docs/RELEASE-PLAN-VIBESWAP-ON-NOESIS.md:19`).

**Honest claim until gaps close:** "producer reorder rejected; producer inclusion and
coord-forgery contained by honest-validator assumption" — **never** "MEV eliminated."

### T4 — GEV elimination at the protocol level — **preserved**

Verified at code level across every extraction surface:
1. **No fee path** — word-boundary grep for fee/tip/gas across `node/src` clean; the only
   "rent" is the decay supply sink (`lib.rs:472`).
2. **Token layer = pure conservation, no skim expressible** — `tokens.rs:10-13` "there is no
   off-chain feed to attest, nothing to bridge, nothing to trust"; output supply == input
   supply (`tokens.rs:53-56`); mint only by issuer (`tokens.rs:58-75`).
3. **Ordering extraction dissolved in code** — `runtime.rs:355-357, 439-441`; consensus-sourced
   ordering so "earlier" is not producer-arrangeable (`lib.rs:9188-9199`).
4. **No producer revenue lever** — grep coinbase/block-reward/producer-reward = zero hits.
5. **No inclusion fee market** — mempool reject-when-full, explicitly not evict-by-priority
   (`runtime.rs:416-430`); designed admission price = commit-deposit REFUNDED on genuine
   contribution, BURNED on junk, paid to no one (`RESOURCE-DOS-BOUNDING.md:50-58`).
6. **All sinks burn, no payee** — decay reclaims to the state commons
   (`CRYPTOECONOMICS.md:17,22-37`); slash remainder "BURNED (sink)" after restitution +
   challenger bounty β (`DISPUTE-SLASHING.md:40,51`); claims waterfall in code makes
   restitution senior, deficits land on the risk-taker, "never recovered from honest third
   parties" (`lib.rs:6202-6213`, regression `lib.rs:6263-6278`).
7. **Doctrine explicit** — "Dissolution, not detection... make attack classes unprofitable by
   structure" (`COMPETITIVE-POSITION.md:41-47`); GEV vocabulary live
   (`CRYPTOECONOMICS.md:138` "GEV-conservation: PoW is conserved, relocated").

**Residuals (self-recorded, not rounded up):** p+ε out-of-band bribery "a shared open
weakness... Do not list bribery-resistance as differentiation" + self-report rings open
(`COMPETITIVE-POSITION.md:146-152`); Bound B designed-NOT-built
(`RESOURCE-DOS-BOUNDING.md:50`); JUL stabilizer owed (`TOKENOMICS.md:157`). The no-fee
property is currently **true-by-absence, not invariant** — see §4.

### T5 — Cooperative capitalism (mutualized risk + free-market competition) — **AT-RISK**

**Intent preserved verbatim:** `MANIFESTO.md:24-27` — "non-zero-sum cooperative economics,
distilled into a protocol stack... Noesis is where cooperative capitalism stops being a set of
contracts and becomes the substrate."

**Free-market half designed/built:** state-bytes "transferable — this is the liquid commodity
layer" (`TOKENOMICS.md:74-76`); JUL "elastic and made to circulate" (`TOKENOMICS.md:85-89`);
competitive verification market — "refuting garbage is PROFITABLE work"
(`DISPUTE-SLASHING.md:83-84`), "the bounty makes refutation positive-EV at every grid V —
p >= 1/2 is PURCHASED by beta, not assumed" (`DISPUTE-SLASHING.md:200-201`); zero
fee-extraction paths.

**Mutualization half THIN — the gap:** all risk instruments are **individual** bonds —
per-certifier causal-share liability ("Endorsement = underwriting",
`DISPUTE-SLASHING.md:65-67`), challenge bond B, appeal bonds 2^k
(`DISPUTE-SLASHING.md:189-190`) — and slash surplus is BURNED (`DISPUTE-SLASHING.md:51-52`),
i.e. deflation accruing pro-rata to possession. Repo-wide grep for
insurance/mutual/risk-pool: **zero mechanism hits** — no ILProtection/VolatilityInsurancePool-class
mechanism exists in `node/src` or `docs`. Nearest analogs: PoM-dilution as shared cartel cost
(`DISPUTE-SLASHING.md:110-111`), γ nuisance compensation (`DISPUTE-SLASHING.md:41,58`),
TSS-JUL rebase designed-not-built with the firewall "minting JUL touches no PoM, no standing"
(`internal/DESIGN-elastic-pow-money.md:133-135,154-156`). The cooperative test is alive as a
design gate — `ROADMAP.md:1062-1064`: Harberger variant "must pair with quadratic weighting OR
redistribute the tax as a commons-dividend... (cooperative-capitalist, not pay-to-play)" — but
un-instantiated. Anti-winner-take-all floors ARE constitutional (`runtime.rs:588-595`;
`TOKENOMICS.md:76-79` "No capital gate: you contribute your way in").

### T6 — Value to contributors, not extractors / airgap closed structurally — **preserved**

- **Closed deeper than VibeSwap-on-EVM could reach:** `tokens.rs:10-13` — "NO PRICE /
  ATTESTATION ORACLE LAYER... the airgap is closed structurally... nothing to bridge, nothing
  to trust"; `runtime.rs:150-156` — token validity "a PURE function of the tx (no oracle)",
  mint authority DERIVED from consumed inputs, never producer-asserted. Noesis **deletes the
  oracle class VibeSwap itself still carries** (`vibeswap/oracle/` Kalman price feed).
- **Dishonesty negative-EV is TESTED, not asserted:**
  `lib.rs:5276` `endorsement_slashing_makes_the_vested_certifier_ring_negative_ev`;
  `lib.rs:5331` `encoded_noise_endorsement_is_negative_ev_slashing_is_content_agnostic`;
  `lib.rs:6045` calibration sweep pins shipped params inside the feasible region
  (`DISPUTE-SLASHING.md:80-85,192-204`); adversary cells "earn 0 novel coverage by
  construction" (`lib.rs:7503-7506`).
- **Franchise un-buyable, unrepresentable to reassign:** `lib.rs:156-161` +
  `soulbound::valid_transition` + both rejection tests; slash-evasion exit closed by
  `valid_transition_under_dispute` (`lib.rs:528-553`); paraphrase-ring closed by
  `pom_scores_with_similarity_floor_q16` (`lib.rs:171-193`).
- **Capital cannot finalize without contribution's consent:** `runtime.rs:583-595`
  FINALITY_MIX pow=0 + MIN_DIM_BPS, wired live per the anti-theater test
  (`runtime.rs:1490-1520`), plus `pom_alone_cannot_finalize_anti_concentration`
  (`runtime.rs:1523`) and `pow_is_excluded_from_finality` (`runtime.rs:1539`).
- **Sovereign surfaces don't reopen the gap:** JUL peg is endogenous Ergon-style PoW, no
  energy-price feed (`TOKENOMICS.md:80-86`); convergence import boundary explicitly named "an
  airgap, and must be treated as one... bonds and disputes imported attestations...
  re-measures" (`docs/CONVERGENCE-REVERSE-FORK.md:89-94`).
- Invariant verbatim: `TOKENOMICS.md:117-122` — "you cannot buy consensus weight... This is
  what separates Noesis from proof-of-stake, where influence simply is wealth."

**Residuals (pinned, never rounded up):** meaning boundary "contained... it does not dissolve
it" (`DISPUTE-SLASHING.md:100-105`); semantic-airgap never-flip pins (`lib.rs:6754,1702`);
p+ε shared open weakness; un-gameability "unsupported, NOT refuted" post-MOAT-1-null
(`COMPETITIVE-POSITION.md:69-79`); consensus-capture ceiling stated in code as
`full_consensus_capture_defeats_the_escalation_court_global_assumption`; off-chain private-key
sale = the one channel where standing is effectively buyable (`MANIFESTO.md:80-81` "Identity
assumptions... where that cannot hold, the guarantees weaken"). **Missing named test:**
`capital_alone_cannot_finalize` does not exist in `runtime.rs` (verified 2026-07-03) — the
anti-plutocracy direction is enforced by code-path symmetry, not pinned by its own test.

---

## 3. TOP DRIFT RISKS (ranked)

The standard-chain patterns sovereignty tempts, ranked by (proximity-to-code × thesis-centrality).

### #1 — T3: The PBS / inclusion-market slide (MEV re-entry) — AT-RISK TODAY
The tenet VibeSwap is literally known for, and the only tenet where the enforcement is half
missing **in the built path**. The trajectory: (a) "quality-prioritised eviction"
(`RESOURCE-DOS-BOUNDING.md:35-37`) + the `d >= submission_deposit` floor hardens into
deposit-size-ranked admission — a priority-fee auction in deposit clothing; (b) unconstrained
leader inclusion (`runtime.rs:432-435`) becomes a builder market selling inclusion/exclusion
of reveals — **novelty-theft-by-censorship**, the sovereign analog of sandwich MEV (exclude a
rival's reveal, bank the contested coverage at your own height); (c) COORDS_BOUND ships
"temporarily" producer-asserted at launch and never activates, leaving the strategyproofness
proof pointing at an unenforced input. Each step is the block producer becoming an auctioneer
of inclusion/ordering — exactly what commit-reveal + uniform clearing was built to dissolve.

### #2 — T5: Risk-layer plutocracy (bonded-individualism + burn-to-holders)
The Ethereum-validator-slashing / EIP-1559 shape. Every Noesis risk instrument is an
individual bond; every surplus burns (deflationary subsidy to passive POSSESSION). Without
mutualization, **risk-bearing capacity itself becomes capital-gated**: only deep pockets can
certify boldly (λV+α exposure), fight a 2^k appeal escalation, or absorb the honest-juror
chilling slash. The bonded free-market half quietly eats the cooperative half. This is Will's
plutocracy fear relocated from the consensus layer (closed by MIN_DIM_BPS + soulbound
standing) to the risk layer (where nothing closes it yet).

### #3 — T2: The decay asymmetry — sharpest single line in code today
`Constitution::default()` → `decay_pos: false` (`runtime.rs:76`): contribution standing decays
under staleness while capital stake never does. The code names the consequence itself:
"NCI's non-decaying PoS drifts the effective mix toward capital under staleness"
(`lib.rs:3668`). The symmetric-decay fix is coded and tested but NOT the default — meaning the
effective consensus mix silently re-organizes around who-holds-what as validators go stale.
The exact possession-chain regression, one boolean away.

### #4 — T4: Congestion monetization / fee-market vacuum
Inclusion is unpriced (reject-when-full mempool; Bound B unbuilt) and block production earns
nothing. At launch pressure ("validators need revenue", "mempool is full") the standard moves
are: producer-paid priority tips (re-importing MEV through the fee door even with canonical
shuffle intact), fee-based mempool eviction, or giving any of the three burn-sinks (Bound B
forfeits, slash remainders, decay reclaim) a payee — the instant a sink gets a payee, protocol
rent exists and the extractor class is re-instantiated. Today no-fee is true-by-absence only.

### #5 — T1: JUL-before-stabilizer + sequencing inversion
An energy-pegged PoW coin WITHOUT the proportional-supply stabilizer ("still owed",
`TOKENOMICS.md:157`) is just another volatile speculation token — the exact casino the tagline
rejects. Compounded by the standard L1 launch playbook: token liquidity shipping before live
PoM attribution settlement, so the first thing markets price is the ticker, not the primitive.
Secondary: state-bytes are deliberately "the liquid commodity layer" (`TOKENOMICS.md:72-77`) —
the EIP-1559/blob-futures slide is storage price action eclipsing the attribution product.

### #6 — T6/T2: Parameter-layer and oracle-door re-entry
(a) NCI 10/30/60 (`lib.rs:3683`) is a plain constant with NO constitutional marker (unlike
MIN_DIM_BPS); the owed reconciliation (`TOKENOMICS.md:159-161`) is a retuning surface where
pos could rise or MIN_DIM_BPS become tunable. (b) Three unbuilt doors for the trusted-feed
pattern: a JUL price oracle (stablecoin-oracle pattern), a convergence adapter accepting
foreign attestations instead of bond-and-dispute re-measurement (the LayerZero-style bridge
VibeSwap already ditched once), and outcome-evaluator promotion from evidence-to-the-verdict
into a trusted scoring oracle. (c) p+ε vote-rental: capital RENTS standing-holders' votes
without buying the soulbound token — a de facto transferable franchise through a side channel
the type-script cannot see (shared open weakness, not a differentiator).

---

## 4. HARD-INVARIANT ACTIONS

Fidelity enforced, not hoped: each action converts a comment/absence/assumption into a
test, gate, or constitutional constant. Per [AA#2: claim-needs-structural-enforcer].

### T3 (at-risk) — three launch-gates, constitutionalized like MIN_DIM_BPS
1. **COORDS_BOUND activation = hard LAUNCH-CHECKLIST item.** No network boots while the ELF
   trusts claimed (height, secret); on-VM re-derivation from commit-block header + reveal set
   (`TEMPORAL-ORDER-ONCHAIN.md:58-64`) must be live, with the forged-coord pin
   (`lib.rs:9139-9162`) flipped from documenting-the-hole to asserting-the-rejection.
2. **Build the real commit-reveal block plumbing** (commit = hash(cell‖secret) published
   pre-content, `POM-CONSENSUS.md:88-92`) so pre-reveal gossip is content-blind — converting
   T7's "by composition" defense from asserted to structural — plus an
   **inclusion-completeness rule in `validate`**: a validator votes NO on a block omitting
   reveals it has seen for that height's commits.
3. **Pin "deposit = flat constitutional floor, refund-or-burn only, NEVER a rank-ordering or
   producer-income signal"** as a constant + anti-theater RED test when Bound B is built, so
   the commit-deposit can never mutate into a priority-fee market.
   *Until (1)+(2) land, all public claims say "contained under honest-validator assumption,"
   never "MEV eliminated."*

### T5 (at-risk) — instantiate the mutualization half, in the codebase's own idioms
1. **Spec `docs/MUTUALIZATION.md`** (or a TOKENOMICS section): (a) bond-syndication for
   challenges/appeals — pooled bonds with pro-rata β shares, so dispute access is
   contribution-gated not capital-gated; (b) a certifier mutual pool covering honest-certifier
   slash on overturned close-call verdicts (today the §7 chilling mitigation is only
   "slash rate < 1", `DISPUTE-SLASHING.md:186`); (c) route the burned slash-remainder per the
   rule Noesis already wrote for itself (`ROADMAP.md:1063`) — commons-dividend weighted by
   PoM-STANDING, never by holdings, preserving the P-001 firewall (no dividend touches
   consensus weight).
2. **Pin the gap in the suite** the way every honest residual is pinned — an `*_open_gap`
   test (idiom: `judge_cartel_protects_its_own_garbage_open_gap`), e.g.
   `risk_bearing_is_capital_gated_without_bond_pooling_open_gap` — so the missing half is
   recorded in code and cannot silently ship as done.
3. **Land the TSS-JUL port** (`internal/DESIGN-elastic-pow-money.md:171-173` — "plausibly
   in-scope for launch") with the firewall invariant and MED-6 fixes carried.
4. **Merge-gate question for every new bonded mechanism:** "who can afford to bear this
   risk?" — if the answer is "capital", it needs a pooling counterpart before merge.

### T2 (preserved, guard the residuals)
1. **Close the decay asymmetry** — flip `decay_pos: true` in `Constitution::default()`
   (`runtime.rs:76`), OR if the asymmetry is a deliberate constitutional choice, pin it with a
   regression asserting effective-mix-under-staleness never pushes pos above its
   constitutional 0.30. Today the drift is documented (`lib.rs:3668`) but unbounded by any test.
2. **Freeze the possession ceiling at physics:** encode in genesis Constitution + regression
   that NCI reconciliation can never raise pos above pom and can never make MIN_DIM_BPS
   governance-tunable — today "constitutional" lives in a comment (`runtime.rs:592-594`), not
   an enforced invariant.
3. **CI lint: no consensus/finality-path function may key by `lock.args`** — makes the
   `pom_scores` claim (`lib.rs:156`) structurally impossible to regress. Keep both soulbound
   rejection tests as permanent fixtures.

### T4 (preserved, make no-fee an invariant instead of an absence)
1. **Constitutional no-payee invariant at MIN_DIM_BPS tier:** every protocol flow either makes
   a harmed party whole (restitution/β bounty from the slash) or terminates in a burn-sink; no
   discretionary payee, no producer revenue, no inclusion fee — constant + named regression in
   the style of `restitution_is_senior_shortfall_junior_decay_last` (`lib.rs:6263`), so a
   future fee path breaks CI, not memory.
2. **Build Bound B BEFORE launch pressure** — it is the already-designed GEV-free answer to
   the congestion problem a fee market would otherwise "solve"; shipping it first removes the
   vacuum the drift lives in.
3. **Anti-theater CI probe** on `node/src` (mirror of the RESOURCE-DOS-BOUNDING pattern)
   failing loud on fee/tip/payee-introducing constructs; keep p+ε and self-report-ring pinned
   as open ledger items so they are never re-marketed as solved.

### T1 (preserved, fence the two unbuilt transferable layers)
1. **Promote candidate L14 to a standing COHERENCE-LAW** covering the full anti-casino launch
   posture: genesis-burn on-chain-provable AND **no JUL mainnet until the spent-not-hoarded
   stabilizer ships** — the stabilizer as constitutional launch-gate, not post-launch patch.
2. **Lock no-protocol-fee as machine-checked** (shared with T4 action 1/3).
3. **Sequencing rule in ROADMAP:** attribution settlement live BEFORE any transferable-token
   public market — the coordination primitive must be what the market prices first.

### T6 (preserved, keep the doors welded)
1. **JUL build gate:** peg must be a pure function of chain state (work/difficulty); any
   external feed in a JUL design doc or module = automatic drift alarm; require a
   tokens.rs-style pure-conservation pinned test before JUL code merges.
2. **Extend the dont-let-attacker-choose-critical-input discipline** (currently 8 sites) to
   every new input — any producer-asserted value re-derived from consensus or rejected.
3. **EV-test gate:** no new mechanism ships without its own `..._negative_ev` regression +
   membership in the calibration feasibility sweep (`lib.rs:5276/5331/6045` pattern). No new
   mechanism on detect-and-slash monitoring in place of a proven negative-EV inequality.
4. **Evaluator-is-evidence-not-verdict** encoded as a code-level pin when the
   outcome-evaluator module lands (mirroring `full_consensus_capture_..._global_assumption`).
5. **Add the missing `capital_alone_cannot_finalize` regression** (PoS whale with zero PoM
   participation must fail `dim_ok(pom)`), mirroring `pom_alone_cannot_finalize_anti_concentration`
   (`runtime.rs:1523`).
6. **Public-claim rule:** wherever "cannot be bought" is claimed, state the recorded residual —
   the type-script makes on-chain reassignment unrepresentable; it does not and cannot prevent
   off-chain account sale.

---

## 5. THE FRAME — the honest verdict

**Noesis, as designed, IS VibeSwap's thesis realized deeper — with two tenets at-risk in the
unbuilt layers, and fidelity currently held in several places by absence and comments rather
than by tests.**

Where the substrate goes deeper than VibeSwap-on-EVM ever could:
- **T1/T2:** attribution is not an app on the chain — it IS the block
  (`runtime.rs:15-17`); the franchise/possession split is not a contract convention but the
  cell type itself (`lib.rs:41-49`), with reassignment made *unrepresentable*, not merely
  forbidden.
- **T6:** VibeSwap carries an oracle (`vibeswap/oracle/` Kalman feed); Noesis **deletes the
  oracle class** (`tokens.rs:10-13`) — the airgap is not bridged better, it is closed
  structurally. Dishonesty-is-unprofitable is a *test suite*, not a whitepaper claim.
- **T4:** there is no fee, no tip, no coinbase, no payee-bearing sink anywhere in `node/src` —
  the extraction class has no expressible surface in the built code.

Where the honest number is lower:
- **T3 is at-risk**, and it is the tenet VibeSwap is named for: producer *reordering* is
  dissolved in code, but producer *inclusion discretion* and *coord provenance* are contained
  only by an honest-validator assumption plus commit-reveal plumbing that is designed, not
  built. The honest claim today is containment, not elimination.
- **T5 is at-risk:** the free-market half is built and sharp; the mutualization half — the
  thing that makes it *cooperative* capitalism rather than bonded individualism — has zero
  mechanism instances in the repo. Every surplus burns to possession; every risk is borne alone.

**Verdict: thesis-faithful (4 preserved / 2 at-risk / 0 drifted) — no tenet contradicted or
abandoned; the drift Will fears has not happened, and in one case (transferable-credit vs
soulbound-standing, `lib.rs:158-161`) it already appeared and was resolved TOWARD the thesis.
But faithfulness in the built core is not faithfulness at launch: the at-risk halves sit
exactly in the layers sovereignty is about to force decisions on (token launch, congestion,
validator revenue, dispute economics). The §4 actions are what turn "faithful so far" into
"faithful by construction." Until they land, do not claim MEV elimination, do not launch JUL
without its stabilizer, and do not let any sink acquire a payee.**

---

*Audit trail: per-tenet audits synthesized 2026-07-03; key file:line claims re-verified
against the working tree during synthesis (`runtime.rs:76,583-595,1523,1539`;
`lib.rs:41-49,156-161,3668,3683`; `tokens.rs:10-13`; `TOKENOMICS.md:157-161`;
`COHERENCE-LAWS.md:179-182`). Not pushed to git per instruction.*
