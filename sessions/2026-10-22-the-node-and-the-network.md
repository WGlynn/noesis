# Session recap — 2026-10-22 — Noesis as one node in a network of contribution chains

Plain-English recap. Reading artifact, not an operator handoff.

## Will's frame
Noesis exists to end the blockchain wars. The future it invites is one personal chain per person, and
Noesis is the same node every chain runs, not the hub others converge into ("the claim isn't superiority").
Its incentives have to make joining worth it from the first node on. Because the network is positive-sum, a
similar idea shipping first is one more node, not a loss, provided its chain can reconcile with ours. Will's
read on method: it was natural to cram everything into one layer until we understood enough to see each
part's own form.

## What we worked out
- The network is positive-sum exactly where its facts are non-rival. "Who did what, who built on what" can sit
  in many chains at once and merges by plain union in any order; money, priority and exclusive claims are rival
  and need consensus among the parties. The ledger already separates the two (`cells` vs `token_cells`).
- Bernhard Mueller's *Agreement and Surprise* (PhilPapers, private draft of 22 October) supports agreement on
  facts under stated conditions (records that separate the consistent states; repairs that lower mismatch,
  terminate and are locally confluent reach one result in any order; reciprocal coupling tolerates different
  speeds) and gives nothing for agreeing on value (price adjustment can cycle).
- Prior art: one chain per person and the rival / non-rival split are not new (Holochain's 2024 whitepaper,
  Kleppmann and Howard 2020, Linera). What none of them has is pricing contributions or paying witnesses.
  `docs/research/prior-art-agent-centric-chains-2026-10-22.md`.
- The thinking note: `docs/research/the-node-and-the-network-2026-10-22.md` (layer map as it falls out,
  bootstrap incentive = anti-wash anchor, reciprocity rule, open questions).

## What got built
`node/src/reconcile.rs`, a consensus-isolated shadow module: content-addressed contribution ids (author,
data and the parent's id, never the local counter) and an order-free union merge of independently run chains'
facts, with rival claims and dangling references surfaced. `node/tests/reconcile_merge.rs`: two nodes from
separate geneses merge to the same digest in either order, a cross-chain reference resolves after the merge,
merge is associative, commutative and idempotent, and a rival claim is surfaced once. Library suite unchanged
(370 pass). Not wired, no adversary model, no signatures or witness checks yet.

## Next
A layer-map proposal with the prior-art traps folded in, and Will's calls on the README peer framing (done) and
on what to build next.
