# DESIGN — Crediting contributors who have no wallet yet (claimable attribution)

> Will 2026-06-23: *"how will we give people here credit if they don't have wallets yet?"* The answer
> falls out of separating two things Noesis conflates at its peril. Continuation of the standing
> reflexive-provenance rule (`DESIGN-reflexive-provenance.md`).

## The key distinction: provenance ≠ payout
- **Provenance needs NO wallet.** It is a recorded FACT — "this contribution came from X" — keyed to an
  identifier X already has: a GitHub repo URL, a juror ID, a dataset content-hash, an ENS, an email.
  Committed at ingestion (commit-reveal root + source). The moment it is recorded, they *have provenance*.
  No account, no signup, no wallet.
- **Standing / payout needs a wallet** only because value must be *received* by a controller. So it is
  held as a **claimable entitlement against the identifier**, not paid to a phantom.

## The mechanism: credit-by-identifier, claim-by-binding
1. **Attribute to the identifier** — standing accrues to `X` (the repo URL / juror ID / hash) in an
   UNCLAIMED state. The provenance graph references identifiers, not wallets.
2. **Bind later** — when the contributor wants the standing, they create a wallet and prove control of
   `X` (GitHub OAuth / a signed commit / DNS or `.well-known` / ENS / an EF-Gitcoin attestation). The
   binding tx maps `X → wallet`; the accrued standing is now theirs.
3. **Soulbound-consistent** — standing is non-transferable, so "held against an identifier, bound to a
   wallet on proof" is exactly the soulbound model: binding, never transfer. The wallet is just the
   eventual controller of an identity that already existed.

## Fair-launch / no-premine consistency (load-bearing)
- An unclaimed entitlement is **NOT minted or circulating** — it is a recorded RIGHT to mint on a valid
  claim. So crediting the whole pre-existing world does not premine tokens to phantoms (it respects the
  locked no-premine tokenomics).
- A **claim window** (then reclaim-to-commons / treasury) keeps unclaimed standing from dead-locking
  supply, and makes squatting an identifier you don't control worthless (claim requires proof).

## Why this is also the adoption engine (not just an accounting fix)
This is the whitepaper's "come join us / reverse-fork" made concrete: Noesis can **attribute the entire
existing contribution graph — every OSS repo, every dataset, every juror — before any of them has a
wallet**, and each onboards by *claiming what is already provably theirs*. Credit precedes, and pulls,
adoption. The first instance is the DeepFunding ingestion (`data/deepfunding/PROVENANCE.md`): those 117
repos + the jurors are attributed by identifier now; any becomes claimable on a live chain.

## Why it is powerful (the precise mechanism, not the slogan)
1. **It inverts the cold-start problem.** Every network dies at zero: no users → no value → no users.
   Noesis never starts at zero — it attributes the ENTIRE pre-existing contribution graph (all of open
   source, the DeepFunding jury work, every foreign useful-work chain's output) from day zero, keyed to
   identifiers that already exist. The value graph is fully populated *before* anyone joins. Adoption
   becomes *claiming*, not building-from-scratch.
2. **Credit precedes adoption and pulls it.** Normal: join, then earn. Here: you are *already credited,
   provably*, and you join to CLAIM. "There is standing with your name on it" is a far stronger pull than
   "come earn from zero."
3. **Permissionless attribution, permissioned claim.** Attribution is a fact, so it needs no one's
   consent to record; the claim is the only consent step. Noesis can map the world's contribution graph
   unilaterally and correctly, and let reality opt in.

## This is reverse-fork convergence at the contributor granularity
`docs/CONVERGENCE-REVERSE-FORK.md` states the thesis for *chains*: rival useful-work chains converge by
having their contributions attributed into one canonical graph (absorb, don't compete). Claimable
attribution is the SAME mechanism one level down — for *people, repos, datasets*. Two levels, one
geometry: **absorb by attribution, converge by claim.** A foreign chain's contributors, or the entire
OSS world's contributors, are credited by identifier and onboard by claiming. The chain-agnostic
contribution adapter (that doc's unbuilt part) and the identifier→wallet claim (this doc) are the same
import interface seen from the two ends.

## A value leg independent of the (currently unproven) moat
The learned-`v(S)` moat is, as of the 2026-06-23 DeepFunding test, **unsupported on real data**
(`data/deepfunding/RESULTS.md`). Claimable attribution does NOT depend on it: even with a *simple, fixed*
value function, pre-attributing the world's contribution graph and pulling adoption by claim is a
standalone value proposition. So Noesis has a second leg to stand on while the moat's faithful-feature
test is run. (Honest: it is a strong *design* claim, also unbuilt.)

## The guardrail that keeps it from being naive (consent / right-to-disclaim)
Attributing people without their action has real failure modes; name them, build the answer in:
- **Right to disclaim / opt out.** A provenance record is a fact, but a person may not want to be
  *associated* or to *receive* standing. The claim step is opt-IN by construction; add an explicit
  opt-OUT (disclaim an identifier → its standing reverts to commons, attribution marked declined).
- **Misattribution is reputational harm.** Computed credit can be wrong. Attribution is a *claim about*
  lineage, challengeable through the same dispute/refutation path as any contribution — never asserted
  as settled truth about a person.
- **No unconsented PAYOUT, ever.** Provenance (the record) is unconditional; standing (the receivable)
  is inert until claimed. Nothing is transacted on someone's behalf without their key.
- **Squatting resistance** is already handled: claiming requires PROOF of control of a pre-existing,
  externally-costly identifier (Cheng-Friedman anonymity-relaxation — a fresh identifier is worth zero).

## Scope / open
- Identifier→wallet proof methods are per-identifier-type (repo, person, dataset); start with GitHub
  control proof (covers the OSS-repo majority). 
- Sybil note: claiming requires PROOF of control of a *pre-existing, externally-costly* identifier (a
  real repo with history, a real juror record) — consistent with the Cheng-Friedman anonymity-relaxation
  (a fresh identifier is worth zero; an established one carries its own cost-of-existence).
