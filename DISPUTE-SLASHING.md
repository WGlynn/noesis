# Dispute-window endorsement-slashing — design (pre-implementation)

> Status: IMPLEMENTED (2026-06-12, same day) — `node/` `dispute` module, full §6 plan
> green. Closes the residual pinned by
> `adversary::vested_certifier_endorsing_garbage_open_gap` (negative-EV at the dispute
> layer; the gate-level pin stays as surface documentation). §5.3's judge-cartel residual
> is now PINNED IN CODE: `adversary::judge_cartel_protects_its_own_garbage_open_gap`
> (a >1/3 vested-standing bloc vetoes refutation; flips when a structural counter lands).
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

## 6. Test plan (implementation gate — all must exist before the pinned test flips)

- Windowed vesting: value realized at E spendable only at E+W; refutation inside W cancels.
- Challenge/verdict state machine: open → voting (reuse `finalizes_hybrid`) → refuted/upheld.
- Causal-share slash math: zero-seed recomputation; Σ shares ≤ canceled value; deterministic.
- `vested_certifier_endorsing_garbage_open_gap` FLIPS: post-slashing ring EV < 0.
- Griefing: failed challenge costs B; honest cell's vesting unharmed.
- Honest-certifier safety: no refutation ⇒ no slash, ever (regression).
- Inequality invariant: α, λ, β, B satisfy §4 for the implemented attack shapes.
