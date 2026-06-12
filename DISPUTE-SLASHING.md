# Dispute-window endorsement-slashing — design (pre-implementation)

> Status: IMPLEMENTED (2026-06-12, same day) — `node/` `dispute` module, full §6 plan
> green, §5b critical-qa hardenings applied, **§7 escalation court + juror accountability
> SHIPPED** (the judge-cartel structural counter: round-1 veto overturned at the
> AND-composed full-mix tribunal; overturned jurors slashed; bonds double; conflicted
> jurors excluded). The cartel pin stays as round-1 surface documentation; the remaining
> ceiling is the system's GLOBAL assumption, stated in code:
> `full_consensus_capture_defeats_the_escalation_court_global_assumption` (never flips —
> ≥2/3 cross-dimension capture is the consensus layer's own ceiling).
> Parent docs: ROADMAP Phase 1 (gate hardening), POM-CONSENSUS (finalization machinery),
> COHERENCE-LAWS (L-invariants referenced below).

## 1. The residual this closes

value_v6 priced identity: an all-fresh sybil ring earns 0 because unvested identities pump
no flow. The surviving attack: a contributor with EARNED standing builds a novel-garbage
child on a fresh-key garbage parent. The certifier clears the floor, the gate opens, the
attacker's fresh key collects. v6 made this accountable (the endorser is a real, earned,
soulbound identity) but not yet costly. This design makes it NEGATIVE-EV.

## 2. Mechanism

### Objects
- **Vesting entry**: value paid to cell X by the v6 gate does not become spendable at
  intake. Flow realized in epoch E vests (becomes withdrawable state-bytes) at E + W.
- **Challenge cell**: any vested-standing holder may post `Challenge(X, bond B)` while any
  of X's value is still unvested (i.e., within W of the flow that paid it).
- **Verdict**: PoM-weighted finalization over vested standing, REUSING the consensus
  machinery already in code (`consensus::finalizes_hybrid`: 2/3 supermajority + eclipse
  quorum-floor), open for D epochs after the challenge.

### Parameters
| param | meaning | first-cut |
|---|---|---|
| W | dispute window (epochs between flow-realization and vesting) | calibrate; ≥ D |
| B | challenge bond (state-bytes) | fraction of X's unvested value, floor B_min |
| λ | restitution multiplier on certifier slash | 1.0 (full restitution) |
| α | deterrence penalty on top of restitution | > 0, see §4 inequality |
| β | challenger bounty share of slashed amount | e.g. 0.5; remainder BURNED (sink) |
| γ | nuisance compensation to author on failed challenge | small fraction of B |

### Refuted path (verdict = garbage)
1. X's UNVESTED value → canceled. Already-vested value (older than W) is untouchable —
   that is the price of finality; W bounds the exposure by construction.
2. Every certifier whose EXTERNAL edge seeded flow into X is slashed
   `λ × (their causal share of X's canceled value) + α`, via `soulbound::Op::Slash` on
   their standing cell. Causal share is DETERMINISTIC: recompute the v6 flow with that
   certifier's seeds zeroed; their share = the difference (Shapley-style marginal on the
   flow graph; exact, no oracle — the flow module already computes both runs).
3. Challenger: bond B returned + β × total slashed. Remainder burned (mint↔sink balance,
   COHERENCE-LAWS).
4. Authors with standing are slashed like certifiers (self-endorsement through a second
   vested identity gains nothing). Fresh-key authors have nothing to slash — which is
   exactly why the liability rides on the certifier, the only earned identity in the loop.

### Upheld path (verdict = genuine)
- Challenger loses B: γ × B to the challenged author (nuisance compensation), rest burned.
- X's vesting clock is NOT reset (no griefing-by-delay).

## 3. Why the certifier, not (only) the author

The author key is free; the certifier's standing is earned and soulbound. Slashing must
attach to the scarce, unforgeable thing. This is the same inversion as v6 itself: identity
pricing moved from "who wrote it" to "who vouched for it". Endorsement = underwriting.
Building on a cell IS the certification act that pumped the gate; the slash makes that
underwriting literal.

## 4. Incentive analysis (the un-gameability inequality)

Attacker (vested certifier C, pocket key P, garbage X):
- Gain: V = value vested to X caused by C's certification (≤ C's causal share, by §2.2).
- Loss if refuted (probability p): λV + α, with λ = 1 ⇒ V + α.
- EV = (1−p)·V − p·(V + α) < 0  ⇔  α > V·(1−2p)/p.
  - p ≥ 1/2 ⇒ ANY α > 0 suffices.
  - The bounty (β share to the challenger, paid from the slash) is what makes p large:
    refuting garbage is PROFITABLE work, and the causal-share computation is mechanical,
    so detection cost is low. TESTED INVARIANT (next increment): for the implemented
    parameters, ring EV < 0 across the adversary suite's attack shapes.

Griefer (challenger spamming honest cells):
- EV = −B + p_false·(β × slash). With the 2/3 + quorum-floor bar, p_false requires
  capturing the vested-standing supermajority — the consensus-capture problem, already
  priced by NCI economics. Sub-capture, EV = −B + γ leakage ⇒ strictly negative.

Honest certifier:
- Slashed only on a 2/3 vested-standing verdict that the thing they built on is garbage.
  Exposure is bounded by λ × causal share — they can never lose more than the value their
  endorsement minted (plus α). Building on plausible work remains positive-EV; the gate
  prices CARELESS certification, not certification.

## 5. Honest tensions (recorded, not hidden)

1. **The verdict is judgment, not math.** "Genuinely valueless" is socially legible but
   not mechanically provable — this is the airgap, surfacing exactly where it must: at
   the boundary between coverage-proxy and meaning. The design contains it with bond
   asymmetry + supermajority + bounty, it does not dissolve it. The learned
   outcome-evaluator (Phase 1 open bet) is EVIDENCE submitted to the verdict, never the
   verdict itself.
2. **W delays honest liquidity.** Every honest contributor waits W for spendability.
   W is the price of clawback-ability; calibrate, don't hide.
3. **Judge collusion residual (NEW PIN for the adversary suite):** a vested-standing
   supermajority could refuse to refute its own ring. Bounded by: 2/3 bar (capture cost),
   bounty (defection from the cartel pays), and PoM-dilution (protecting garbage inflates
   value supply, diluting every honest holder — the cartel pays the inflation it shelters).
   Pin as `judge_cartel_protects_its_own_garbage_open_gap` when the module lands.

## 5b. Critical-qa hardenings (2026-06-12, post-implementation hostile review)

Four weaknesses found by self-adversarial review and FIXED same-session, each with a
regression test:
1. **Exposure snapshots at challenge-open** — slow resolution can no longer vest value
   out from under a live dispute (`open_challenge_snapshots_exposure_...`).
2. **α attaches to causation, not adjacency** — zero-share certifiers are skipped
   entirely (`zero_share_certifier_is_never_alpha_taxed`).
3. **Param clamping** — β, γ clamped to [0,1]; the resolver can never become a mint by
   misconfiguration (`misconfigured_params_cannot_turn_the_resolver_into_a_mint`).
4. **Slash-evasion-by-exit blocked** — `standing_exit_blocked`: while a challenge is
   open, any contributor with a provenance edge into the challenged target is denied
   burn/decay-exit (`standing_exit_is_blocked_while_a_challenge_names_your_edge`).
   **WIRED at the cell layer (same session): `soulbound::valid_transition_under_dispute`**
   — burn rejected while exposed; `pom` may decrease only by the settlement-authorized
   amount (the slash itself always lands; voluntary drain dressed as decay cannot);
   accrual unaffected; no exposure ⇒ defers to the plain rule. Three regression tests.
   The plain `valid_transition` carries a NOTE that it is correct only for standing with
   no dispute exposure.

Annotated (not fixed): the judge set's `pom` values and the value layer's standing map
are bound by caller convention in the reference spec; on-chain the type-script reads
standing cells directly.

## 6. Test plan (implementation gate — all must exist before the pinned test flips)

- Windowed vesting: value realized at E spendable only at E+W; refutation inside W cancels.
- Challenge/verdict state machine: open → voting (reuse `finalizes_hybrid`) → refuted/upheld.
- Causal-share slash math: zero-seed recomputation; Σ shares ≤ canceled value; deterministic.
- `vested_certifier_endorsing_garbage_open_gap` FLIPS: post-slashing ring EV < 0.
- Griefing: failed challenge costs B; honest cell's vesting unharmed.
- Honest-certifier safety: no refutation ⇒ no slash, ever (regression).
- Inequality invariant: α, λ, β, B satisfy §4 for the implemented attack shapes.

## 7. Judge-cartel structural counter — escalation court + juror accountability

Closes §5.3's pinned residual (`judge_cartel_protects_its_own_garbage_open_gap`): a >1/3
vested-standing bloc vetoing refutation of its own ring.

### Mechanism (three pieces, one load-bearing)
1. **Juror-exclusion (hygiene, not load-bearing):** jurors with a provenance edge into the
   challenged target are excluded from the round-1 denominator — no one judges their own
   case. Evadeable by separating worker-identities from judge-identities, so this is a
   cheap filter only.
2. **Escalation court (structural):** a failed refutation may be APPEALED with a doubled
   bond. The appeal tribunal is NOT PoM-only — it is the AND-composed full mix
   (`consensus::NCI`: PoW 10 / PoS 30 / PoM 60, 2/3 bar + quorum floor). A PoM cartel
   holding >1/3 of standing holds only ~0.6×its-share of the appeal weight; vetoing there
   requires cross-dimension capture, which is the consensus layer's already-priced global
   assumption. NO NEW capture surface is introduced — the value layer borrows the
   strongest tribunal the system already has.
3. **Juror accountability (load-bearing):** when a verdict is overturned on appeal, the
   jurors who voted with the overturned majority are slashed proportionally to their
   vote weight. The cartel's round-1 veto is therefore a STANDING BET that anyone can
   call by escalating. Equilibrium: the veto doesn't fire. This attaches to the VOTE,
   not the edge — so worker/judge identity separation (which evades exclusion) does not
   evade accountability.

### Why this dissolves the class (not detects instances)
The cartel's veto power becomes a delay plus a bonded liability; conviction authority
terminates at a tribunal whose capture is the system's global assumption anyway. The
profitable-trajectory test fails for every instance of the class: veto ⇒ slashed on
appeal; don't veto ⇒ ring is refuted. Detection-independence per
[class-dissolution-vs-case-defeat].

### Honest tensions
- **Ceiling stated plainly:** if the FULL three-dimension tribunal is captured (2/3 of
  the AND-composed mix), no appeal exists. That is full consensus capture — the global
  assumption every layer already rests on; pinned in code as
  `full_consensus_capture_defeats_the_escalation_court_global_assumption` (passes today,
  BY DESIGN — it documents the ceiling, it does not flip).
- **Chilling honest jurors:** slashing wrong-but-honest votes discourages participation.
  Mitigations: slash rate < 1 and proportional; overturn requires a 2/3 supermajority at
  a STRONGER court (close calls don't overturn); appeal bonds double (frivolous
  escalation is expensive).
- **Griefing via escalation spam:** bounded by doubling bonds; a griefer pays 2^k × B to
  drag a dispute k rounds with no new evidence.
