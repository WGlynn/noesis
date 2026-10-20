# Noesis applied to markets: many minds, one trade

> **Status: DESIGN / idea, ready-for-critique (2026-10-20). Nothing here is built.** Idea from Will (2026-10-20);
> this page drafted with JARVIS. Status discipline: built · designed · open. External facts asserted from memory are
> marked `[verify]`.

## The problem

The working premise (Will's): when every trader runs the same AI on the same public data, they chase the same edges,
and each edge gets competed down toward zero. The race wastes effort (the same work done a thousand times, the costs paid a thousand times),
and the small trader acting alone is often the prey: front-run, filled at worse prices, hunted at their liquidation
level.
In our own private testing, no direction rule built from public data beat a random baseline after costs.

## The idea in one line

**Many minds, one trade, each paid by what they add.** The trading race becomes a collective effort instead of a
competition. It is the Noesis rule, pay for real contribution, applied to markets.

## How it would work (sketch)

1. **Commit.** Each member locks in a view (direction, size, horizon) as a hash commitment before the period opens.
   Views are revealed only after the commit window closes, so nobody can copy the consensus after seeing it (the
   VibeSwap commit-reveal pattern).
2. **Combine.** The revealed views are combined into one call, weighted by each member's scored track record.
3. **Execute once.** Members' views net out inside the pool before any order is sent, so members do not trade against
   each other or race each other for the same fills. The net order goes to a venue where speed does not pay (a
   uniform-price batch auction).
4. **Pay by contribution.** Each member is paid the Shapley value of their view to the pool's realized result,
   scored out of sample. Identical views split the credit for that view between them (the symmetry axiom), so a view
   many members share earns each of them a small slice, and a view only you bring keeps its full credit. Copying
   cannot multiply pay; independent thinking is what earns.

## How it earns, and whom it does not target

An open market does not let you choose who is on the other side of a trade, and many of them are ordinary people,
not villains. So the pool is defined by how it earns, not by whom it beats:

- **Stop being prey.** Pooled execution on a fair venue keeps what front-running, spreads and slippage used to take.
  In our private testing, keeping those costs turned a frequent strategy with a small real edge from losing to about
  break-even. It could not rescue a signal with no edge.
- **Get paid for services markets need.** Taking the other side of traders who want leverage (the funding they pay),
  and buying from forced sellers in a liquidation cascade, when nobody else will.
- **The combined call.** Many independent views, each only slightly better than chance, combine into a better call
  than any one of them alone.

Extractors lose because the venue removes their edge, not because the pool hunts them. Same move as VibeSwap:
change the venue so the extractive game stops paying.

## Guardrails

- **Size cuts both ways.** A large pool moves prices against itself; capacity is finite and should be measured.
- **Trade on views only.** Every trade moves prices a little; trading in order to move a price is manipulation, and the
  pool may never do it.
- **No house cut.** Members keep what the pool earns, split by contribution (the no-extraction rule).

## Honest floor

- **Built:** nothing. **Designed:** this sketch. Commit-reveal, batch auctions and Shapley attribution exist as
  separate pieces in VibeSwap and Noesis; the pool combining them does not.
- **Pooling views with no edge gives no edge.** The pool works only if members bring small, genuinely independent
  edges. It saves costs and combines skill; it does not create skill.
- **The outline is not new.** Numerai pays people for predictions, partly by what each adds to its combined model
  `[verify]`. What would be new is the combination: commit-reveal against copying, Shapley pay, no house cut, a fair
  execution venue.

## Open

- Scoring contribution when views are correlated, at scale (exact Shapley is exponential; approximations and their
  gaming surface). Related: `DESIGN-harberger-peer-prediction-theorems.md`.
- Sybil members (one person, many accounts): symmetry splits a duplicated view's credit, so duplicates should not
  gain, but this needs a proof against near-duplicates.
- Capital versus views: who supplies capital, how it is paid, and how that stays separate from contribution standing.
- Regulation: pooled trading for members looks like a fund in most places. Needs real legal review before anything
  touches money.
- Adoption: whether traders join depends on whether a view earns more inside the pool than alone.
