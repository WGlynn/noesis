# Session recap, 2026-10-20: Noesis applied to markets

Plain-English recap of a short design conversation. No code changed; one design note was added.

## Where it came from

Will had been testing whether a trading algorithm could make money from public data. It could not: every direction
rule tried did no better than a random baseline once costs were counted. His read of why: once everyone has the same
AI, an individual trader's edge is only as good as everyone else using the same tool, so the race eats itself.

His answer was not a better racer. It was to change the race: make trading a collective effort instead of a
competition. Many traders lock in their views before anyone sees the others', the views are combined into one trade,
and each trader is paid by what their view added. That is the Noesis rule, pay for real contribution, applied to
markets. Will confirmed that framing ("Noesis applied to markets is right").

## The two refinements that held

- **Define the pool by how it earns, not whom it beats.** You cannot choose who takes the other side of a trade, and
  many of them are ordinary people. So the pool earns by not being prey (trading once, on a venue where speed does
  not pay), by being paid for services markets need (the other side of leverage; buying from forced sellers), and by
  the combined call. Extractors lose because the venue removes their edge, not because the pool hunts them.
- **Copying cannot multiply pay.** Under Shapley, identical views split one view's credit. A view everyone shares earns
  each of them a small slice; a view only you bring keeps its full credit. Independent thinking is what earns.

## What the numbers said

A simple re-pricing of private test trades: keeping the spread and slippage turned a frequent strategy with a small
real edge from losing to about break-even. It could not rescue a signal with no edge. So the pool saves costs and
combines skill; it does not create skill.

## Where it lives

`docs/DESIGN-cooperative-trading-pool.md`: the sketch, guardrails (size limits, never trade to move a price, no house
cut), honest floor (nothing built; the outline resembles Numerai), and the open questions (scoring correlated views
at scale, near-duplicate sybils, capital versus views, regulation, adoption). Status: designed, not built.
