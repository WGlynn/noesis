# Noesis — the papers

Pick your depth. Each accessible doc is also available as PDF, TXT, and HTML in [`dist/`](dist/).

| Doc | For | Length |
|---|---|---|
| [One page](NOESIS-ONEPAGER.md) | a single-screen pitch | ~1 page |
| [For normal people](NOESIS-FOR-DUMMIES.md) | the no-math, no-jargon explainer | ~5 min |
| [FAQ](NOESIS-FAQ.md) | the questions people actually ask (honest answers) | skim |
| [Litepaper](NOESIS-LITEPAPER.md) | the whole idea, none of the heavy math | ~10 min |
| Whitepaper (`whitepaper/`, **v5.3**) | the full technical paper, math + measured results | the deep end |

All four accessible papers say the same true things at different depths, and all mark honestly what is built versus what is still designed. Start wherever you like and go deeper when you want.

## Design theses (forward-looking)

These state where the protocol is headed and are marked as design theses, not built features. They sit behind the accessible papers and the whitepaper v5.3 capstone:

- **Non-zero-sum protocol** — the headline framing: Noesis is the first blockchain whose competitive relationship to other chains is non-zero-sum, absorbing rivals by conserving their contributions into one attribution graph (reverse-fork = accretion). The contribution-conservation core is built and tested at the reference layer; the cross-chain adapter that enacts convergence is designed, not yet built.
- **Reverse-fork convergence** ([`CONVERGENCE-REVERSE-FORK.md`](CONVERGENCE-REVERSE-FORK.md)) — the substrate other useful-work chains converge into, at the chain granularity.
- **Claimable attribution** — the adoption engine: the existing contribution graph is attributed by identifier before anyone has a wallet, so onboarding is claiming what is provably yours. Reverse-fork at the contributor granularity (two levels, one geometry). Provenance is a fact; standing stays inert until claimed (opt-in), with a right to disclaim and no unconsented payout.
- **Reflexive provenance** — any external data Noesis ingests is itself attributed; the first instance is the DeepFunding ingestion.

Status note: where these theses rely on a *learned* value function being un-gameable, that property is unproven — its first real-data test came back null (unsupported, not refuted). The papers carry a status ledger that marks each claim as built, designed, or open.

## Reference

- **Tokenomics** ([`TOKENOMICS.md`](TOKENOMICS.md)) — the canonical token model: three tokens (JUL = money, VIBE = governance, state-bytes = capital) plus **soulbound PoM-standing** as the unbuyable consensus weight. Clarifies that *Ergon* is a design model for JUL, not a token, and marks honestly what is built (soulbound PoM + transferable state-bytes) versus designed (the JUL money layer).
