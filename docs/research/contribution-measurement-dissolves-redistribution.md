# Measured Contribution Dissolves the Redistribution Dichotomy

*Noesis Research · draft v0.2 · build-in-the-open · honest labels throughout (✅ built · 🟡 designed · 🔬 open)*

> Supersedes the SEED (`…-SEED.md`). Still a working draft — run the marginal-contribution loop before
> publishing. Every mechanism claim carries its status; nothing is rounded up.

## Abstract

On-chain public-goods funding — bug bounties, security, documentation, infrastructure, retroactive
grants — is almost always built as a *redistributive* program: a treasury taxes value here and
reallocates it there. In crypto's political grammar that reads as socialism, and it triggers a reflexive
aversion that no amount of good design fully removes. We argue the aversion is not answered by a better
redistribution mechanism; it is **dissolved** by removing the redistribution step entirely. If
contribution can be *measured*, then funding a public good is not a reallocation at all — it is a
*purchase of measured value*, a market transaction the same community already endorses. The treasury
vote, the allocation policy, the tax: gone. What remains is a price on contribution. We show this is one
instance of a general design law (separate two conflated roles), that a token architecture designed so
capital cannot buy consensus weight (built) or the definition of contribution (built as classification,
enforcement designed) keeps the resulting market from being recaptured by capital, and that its strength
is bounded precisely by one open research problem — the un-gameability of the measure — which the whole
system already stakes everything on.

## 1. The dichotomy and the aversion

Two phrases carry opposite charges in crypto. "Payment for value" reads as market, meritocratic,
legitimate. "Redistribution" reads as socialist, coercive, suspect. This is not merely rhetorical: it
shapes what gets built. Public goods — the security researchers, the documentation, the client
diversity, the infrastructure nobody owns — are chronically under-funded because the only available
funding shape is a *program*: a treasury that collects (a tax, an inflation cut, a fee skim) and
disburses by some governance process. However well-run, that shape is a reallocation, and inspection
always reveals the reallocation. The aversion returns.

The usual responses try to make the redistribution more palatable — quadratic matching, retroactive
rounds, conviction voting, better grants committees. Each is an improvement; none escapes the category.
A matching subsidy is still a subsidy; a retroactive round is still a treasury deciding allocations. The
thing being inspected is still a mechanism that *takes and gives*.

## 2. Measurement changes the category

The escape is not a better redistribution. It is a change of category, and it turns on one capability:
**native measurement of contribution.** Suppose the protocol can score the value a contribution adds —
not perfectly, but adversarially and on-chain. Then "should we fund this public good?" is no longer a
policy question put to a treasury. It is the same question the protocol already answers for every
contribution: *what is this worth?* And the answer is paid as a price, not disbursed as a grant.

There is no separate bug-bounty program; the bug bounty is the contribution reward. There is no
public-goods treasury; the public good is a contribution the measure recognizes. The social-policy
*outcome* — funded security, funded infrastructure — is delivered by a *mechanism* — payment for
measured value — that is not socially coded as redistribution, because it isn't one.

## 3. Why this is a dissolution, not a rebranding

A rebranding relabels redistribution as "payment" while keeping a redistributive mechanism underneath —
a treasury still decides allocations, and the aversion returns on inspection. A **dissolution** removes
the mechanism. The distinction is concrete and inspectable:

- **Rebrand:** treasury + allocation policy + a nicer name. Inspect it → find the reallocation.
- **Dissolve:** no treasury, no allocation vote, no tax. Inspect it → find a measurement and a price.
  There is nothing left to call redistribution because there is no reallocation *pathway* — only a
  measurement pathway and a payment.

This is the structure doing the work, not the branding. You cannot argue someone out of the socialism
frame; you can build a system in which the frame has no referent.

## 4. The design-law spine: one conflated role, split

The dissolution is a specific case of a pattern that recurs far outside economics. **When a single
mechanism is forced to carry two roles whose optimization pressures conflict, the conflation is the
bottleneck; separating the roles into independent pathways dissolves it.** In the treasury, the two
welded roles are *measuring* which contributions deserve funding and *reallocating* value to them — and
it is the reallocation half that reads as socialist. Separate them: let the protocol *measure* natively
and *pay a price*, and the reallocation step is simply gone.

That the same move appears in unrelated substrates is the evidence it is a real abstraction and not a
convenient framing: in machine-learning architecture, the State-Prediction Separation Hypothesis splits
a transformer's conflated "store state" and "predict next token" into two computation streams [1]; in
consensus, Nervos CKB's NC-Max splits a Bitcoin block's welded "disseminate transactions" and "propagate
the block / prove work" so throughput stops being hostage to security [2]; and across Noesis's own stack
the same law separates money from governance from capital, and separates proof-of-work's issuance role
from the finality-safety it is excluded from. The redistribution dissolution is a named instance of that
law, not a one-off argument.

## 5. What makes it capture-resistant: the token architecture

A skeptic's sharpest objection is not "can you measure?" (§7) but "even if you can, won't capital just
capture the measure or the payment, turning your 'market' into plutocratic redistribution by another
name?" The answer is that the token architecture is built precisely to make that impossible, and the
guarantees are structural, not policy.

Noesis separates the functions money systems fuse — money, capital, governance — into three *tradeable*
instruments (an energy-pegged money, a state/capital unit, a governance token) and holds the one thing
that must never be for sale — **consensus weight** — out of all of them as a *soulbound, non-transferable*
franchise (Proof-of-Mind standing) earned only by verified contribution. From this, four structural
properties follow — carried by structure rather than a committee's restraint, three enforced in code
today and the fourth built as classification with its enforcement designed:

1. **Consensus weight is unbuyable.** Weight is soulbound standing; money never converts into weight.
   (✅ reference layer.)
2. **A say over the *measure* is unbuyable.** Amending what counts as contribution requires earned
   contribution authority, not a bought token. (🟡 authority classification built; the vote layer is
   designed-not-built.)
3. **Capital can never outweigh contribution.** The consensus mix is governance-tunable but bounded by
   `pos ≤ pom` — capital's share can never exceed contribution's. Number-free; enforced in code. (✅)
4. **No amendment can silently break the rules.** Every rule change passes an axiom-preservation gate
   regardless of who voted. (✅ socket built; the full confluence proof is an external attach point.)

The consequence for the thesis: the "payment for measured contribution" cannot decay into plutocratic
redistribution, because capital cannot buy the weight that decides outcomes, cannot buy the definition
of contribution, and cannot tilt the system toward itself. The market that replaces the treasury is one
capital is structurally barred from capturing. (Sybil resistance — the other way to fake "contribution"
— is inherited from the same soulbound standing: you cannot split an earned, non-transferable identity
across wallets, so faking N contributors reduces to the un-gameability of the measure itself, §7.)

## 6. The generalization, and a corollary

Once "fund X?" becomes "is X a contribution the measure values?", the special cases collapse into one
mechanism: security/bug-bounties, retroactive public-goods funding (the Deep-Funding / RPGF shape),
documentation, tooling, infrastructure, and protocol code are all just contributions with a price.

A corollary follows for token design itself. If native consensus already measures and rewards
contribution, an application does not need a bolted-on incentive token *outside* consensus. The default
reasons to mint one — bootstrap incentives, speculative alignment, a VC/exit vehicle — dissolve for the
large class of applications whose value is measurable contribution. The chain trends toward pure utility
instead of fragmenting into thousands of thin-liquidity speculative islands. **Honest calibration:** this
dissolves the *speculative-incentive* token, not the *functional asset* — a purpose-specific medium of
exchange, in-game items, or a stablecoin remain legitimate state, not redundant bolt-ons.

## 7. The two maturities, and the honest boundary

The argument is downstream of measurability, and we split the claim on exactly that line:

- **Fraud / gamed-contribution — ✅ already built.** A challenger who proves a contribution fraudulent
  earns a bounty from the slashed value (`dispute::resolve_refuted` + the β-bounty). This is a
  self-funding bug bounty for the contribution graph: the fraudster pays the finder, not a treasury —
  the dissolution, demonstrated, for the adversarial case.
- **General value — 🔬 open research.** "Was this a good contribution?" for code, docs, and
  infrastructure depends on a learned value-oracle `v(S)` that is, honestly, not yet demonstrated on
  real-outcome labels. The elegance of the whole argument is *exactly* as real as the measure's accuracy
  and un-gameability — no more.

We state the limit plainly: this does not "solve social policy." It **reduces** good public-goods
funding to a single open problem — a measure of contribution that is accurate and un-gameable — and
stakes the claim only *to the extent contribution is measurable*. That the same open problem also
secures consensus, sybil resistance, and governance is not a weakness hidden across four systems; it is
one bet with several payoffs. What would falsify the thesis is precise: a demonstration that the measure
is gameable at scale, such that "payment for measured contribution" can be captured for value not
contributed. Absent measurement, the argument is a design direction, not a delivered result — and we
label it as such.

## 8. Positioning (we did not invent the pieces)

- **Quadratic / matching funding** (Buterin–Hitzig–Weyl): a matching subsidy over donations — still a
  reallocation. We measure value directly rather than match donations.
- **Retroactive PGF** (Optimism): "fund what already proved valuable" — the same intuition; we supply
  the missing measurement substrate and make it consensus-native rather than a grants council.
- **Deep Funding** (Ethereum Foundation): the value-of-a-contribution measurement lineage this builds on.
- **Cooperative Capitalism / Augmented Mechanism Design**: the parent frames — math-enforced fairness as
  a byproduct of a market, not a policy overlay.

Our marginal contribution over these is the *dissolution* (removing the redistributive step, not
improving it), the *capture-resistant token architecture* that keeps capital out of the measure and the
payment, and the *design-law framing* that makes the move a named, cross-substrate pattern rather than a
crypto-local trick.

## References

1. G. Monea, N. Godey, K. Brantley, Y. Artzi. *The State-Prediction Separation Hypothesis.* arXiv:2607.01218.
2. K. Ren et al. *NC-Max: Breaking the Security-Performance Tradeoff in Nakamoto Consensus.* NDSS; RFC 0020 (CKB).

---

*Companion: `role-separation-as-design-law.md` (the general law), `DESIGN-governance-authority-tiering.md`
(the capture-resistance mechanism), `internal/STUDY-GUIDE-TOKEN-FLOW.md` (the token architecture).*
