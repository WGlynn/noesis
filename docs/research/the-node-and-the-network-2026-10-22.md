# The node and the network: thinking out loud about a solution we're still understanding

**In one line: the network is positive-sum exactly where its facts are non-rival.**

2026-10-22. Will asked for this framed as a solution we're trying to understand, not a problem. Nothing
here is decided or built unless it says so. Code pointers are the current sketch.

## 1. The solution, as plainly as I can put it

Many contribution chains, down to one per person, all running the same node. Each records what its owner
did and what they built on. Where two chains touch (I used your work, we traded), both record it, and the
overlaps reconcile into facts everyone shares. Worth is computed on top of the shared facts, by judges
frozen per round and anchored in use by people you don't control. Noesis is the node and the rules, not the
chain that wins.

## 2. Why this works where possession chains fight: rival and non-rival facts

I think this is what we've been circling. A possession fact ("this coin is mine") is rival: two chains
can't both be right, so merging them means picking a winner, and that is the war. A contribution fact
("I wrote this; you built on it") is mostly non-rival: it can be recorded in many places at once, and two
honest records of it agree instead of competing.

Merging non-rival facts is a union. It gives the same result in any order, and repeating it changes
nothing (the structure CRDTs use, a join-semilattice). That is Bernhard's order-independence result in its
easiest form: every merge only lowers the mismatch, it terminates, and any two merges commute
(*Agreement and Surprise*, p. 26). That holds by set algebra only for facts that never collide; a collision
(two incompatible records of the same thing) is exactly a rival fact, which is section 3.

So the network is positive-sum exactly to the extent its facts are non-rival. That is a structural reason,
not a slogan, and it can be checked.

Receipt: the ledger already splits the two. `Ledger.cells` holds contributions and `Ledger.token_cells`
holds tokens (`node/src/runtime.rs:206`, `:226`); finality weight folds over `cells` only, and JUL lives in
`token_cells`.

## 3. Where rivalry still lives in a contribution network

- **Money (JUL).** Rival by nature. It stays in per-chain consensus: proof of work for issuance, finality
  for settlement. Moving money between chains is a bridge problem, the hard kind.
- **Priority.** "Who was first" is rival when two people claim the same idea. It needs time that compares
  across chains: commit-reveal and the committee-attested clock design are where that lives.
- **Exclusive claims.** "I authored this, not you" is rival. It needs the dispute layer.
- **Finality votes.** Standing is not a stored rival fact; it is a view computed from facts, so once facts
  agree, standing agrees. Which validators vote on which chain's checkpoints is still per chain.

Rule of thumb: non-rival facts reconcile peer to peer; rival facts need consensus among the parties who
share them. That may be the cleanest layer boundary we have.

## 4. A personal chain is closer to a signed log than to a blockchain

A chain with one owner doesn't need two-key finality: there is nobody inside it to disagree with. It needs
a signed append-only log (the wallet's XMSS signatures already give that) plus witnesses, other chains that
recorded the overlaps. Its facts become network facts only when someone else records them too; private
writes alone can't make a public fact (p. 26). Two-key finality belongs to shared chains: a project or
community that holds rival things together.

Prior art (read 2026-10-22, `docs/research/prior-art-agent-centric-chains-2026-10-22.md`): one chain per
person and the rival / non-rival split are NOT new. Holochain's 2024 whitepaper states the split almost
word for word and calls a double-spend an agent forking its own chain, and Kleppmann and Howard (2020) prove
union merge tolerates any number of bad peers exactly when no merge can break an application rule.
Correction to my first draft: Holochain says countersigning is not for preventing double-spends, and SSB
never handled rival data at all. What the prior art lacks is pricing contributions and paying witnesses;
app-level contribution accounting does exist on Holochain (ValueFlows, Sensorica), and SourceCred-style
contribution scoring is not yet surveyed, so no novelty claim on the value layer either until it is.

## 5. What reconciliation needs that the code doesn't have yet

- **Content-addressed identity.** Today a cell's id is a local counter and its parent is that counter
  (`node/src/lib.rs`, `id: u64`, `parent: Option<u64>`). "Cell 17 on my chain" means nothing to your chain.
  Cross-chain provenance needs ids derived from content: a hash of the author's key, the data and the
  parent's hash. That is the first concrete change a reconciliation spec implies, and it is small.
- **The overlap record.** When I build on your cell, both chains should record it, so the shared record
  pins down one version (the p. 26 condition). A one-sided citation is a claim, not a fact. Attacks: the
  user may hide a source to look more novel (the near-duplicate screen catches copied content, and the source
  can post its own claim and dispute it); two colluding chains can record fake overlaps on both sides (that is
  wash, and it belongs to the value layer's independence test, not to reconciliation).
- **A rule for rival collisions.** Two claims of the same content: earliest commitment, using commit-reveal
  and a shared clock; disputes otherwise.

## 6. Value across many chains: order travels, rates float

EXP-62b and EXP-62h found that judges built differently agree on which contribution is worth more and
disagree on how much more. Across many chains with different judges, that becomes: the order of worth
reconciles, the exchange rate between chains' units does not. That looks like currencies. Each judge
defines a unit, and rates between units are prices. Bernhard's warning applies: iterative price adjustment
can cycle (Scarf, p. 28), so rates shouldn't be forced to converge by repair; freeze them per round or let
a market set them. Will's 10-16 idea fits here: a disagreement in order, not rate, across judges is the
obstruction worth flagging.

## 7. The bootstrap incentive and the anti-wash anchor are the same mechanism

This is the part that surprised me. Why would my chain record overlaps with yours? Because my work earns
only when someone else uses it (value vests on realized use), and that someone lives on another chain.
Reconciling isn't an altruistic interop chore; it is how a contribution gets paid.

And the anchor we have been looking for against wash is use by minds the contributor doesn't control. In
a network of personal chains, that is use recorded on chains you don't control. The interop incentive and
the outside anchor are one mechanism: the network is the periphery.

Floor: a ring of chains that only use each other is the same wash problem one level up. Cheap chains are
cheap identities, and independence across chains still has to cost something (capital clusters, layered
anchors). If nothing can make it cost enough, the residual is priced rather than excluded, the same
51%-class framing as today. This doesn't dissolve the irreducibles; it says where they live.

## 8. The layer map as it falls out (proposal)

1. **Records.** Each agent's signed append-only log; content-addressed cells. Built: signed cells, XMSS.
   Not built: content addressing, key rotation.
2. **Fact reconciliation.** Non-rival facts merged by union, overlaps recorded by both sides, order-free.
   Not built; the conservation core is the nearest piece.
3. **Rival facts.** Money and exclusive claims, settled by consensus among the parties: countersigning
   between two, two-key finality for shared chains, proof of work for issuance. Finality built in the
   reference; countersigning not.
4. **Value.** Frozen judges over the reconciled facts, anchored by use on chains you don't control;
   standing is a derived view. Seam built; judges designed; transfer measured on toy tasks.
5. **Money.** JUL, issued by proof of work, rival, per chain. Issuance built.

Roles still sharing one carrier:
- PoM standing is both a finality vote (layer 3) and a measure of contribution (layer 4). The finality
  fence keeps value out of it, but it is one number doing two jobs.
- The devnet node is records, consensus and API at once.
- "Noesis" names both the protocol and a chain.

## 9. The smallest thing that would test this

Two independent devnet nodes, each with a few contributions and some cross-references, merged in both
orders. If the merged fact sets are identical, the non-rival claim holds in code; if a tie-break is needed,
we have found the first rival fact. It needs content-addressed ids first. Roughly 100 to 200 lines;
Will-gated.

## 10. What I'm still unsure about

- Whether "contribution facts are non-rival" survives real disputes (plagiarism, contested authorship)
  often enough to matter.
- Whether personal chains need any consensus at all, or whether witnessing is enough.
- Whether standing should be one network-wide view or a view per judge; the exchange-rate point suggests
  per judge. The objection is that per-judge standing fragments the franchise. A likely answer: each shared
  chain names its judge in its own constitution, so finality there uses that chain's view, and no single
  network-wide view is needed.
- How much of sections 3 to 5 Holochain already answers.

## 11. Second pass: arguing with myself

**"Non-rival facts make the network positive-sum."** Counter: the facts are non-rival, but payouts can be
rival. If contributors are paid from a fixed pot (a fixed emission per block, a fixed grant), dividing it is
a split, and the fight comes back over the pot. So the claim needs a condition: payouts have to scale with
value created, not divide a fixed amount. Where Noesis rewards actually come from is still open in the
tokenomics (JUL is issued for work, standing is not money), and that choice decides whether positive-sum
facts stay positive-sum in money.

**"Recording overlaps is how my work gets paid, so chains will reconcile."** Counter: if my income depends on
your chain recording that you used my work, your chain can withhold it, and gatekeeping returns as refusal to
witness. The answer has the same shape as Bernhard's stability result (p. 33): a reciprocal coupling is
stable and a one-sided one can be broken by a single change. If witnessing pays only the cited side, it is
one-sided and fragile; if it also pays the witness (credit for being an early user, or for evaluating),
both sides want the record. Design rule candidate: every overlap record pays both parties.

**"A personal chain needs no consensus, only witnesses."** Counter: a single owner can fork their own log,
showing one history to you and another to me. Signatures don't stop that. What stops it is the overlap: two
witnesses who compare their copies catch the fork, which is local repair doing exactly its job. So personal
chains need witnesses who compare, and an equivocation found that way should cost the owner standing. That
makes "witnessing" a protocol duty with a reward, not a courtesy, which fits the previous point.

What survives all three: the rival/non-rival split as the layer boundary, the overlap as the unit of
reconciliation, and reciprocity as the incentive rule. What it adds to the open list: where payouts come
from, and how witnessing is rewarded.

## 12. Similar work can choose to become a coalition (Will, 2026-10-22; proposal)

Will: wherever the seam is that measures similarity between code, it can be a "via governance" merge
technique where it makes sense, not to be conflated with outright plagiarism: software that is similar
enough has the option to form Shapley coalitions.

Read against this note, it is a third way to resolve a rival collision. When two records carry the same or
nearly the same work under different authors (`reconcile::FactSet::rival_claims` surfaces exactly this),
today's options both pick a winner: priority (earliest commitment) or a dispute (plagiarism). A coalition
picks no winner. The parties agree to hold the work jointly and split its value by Shapley, which turns a
rival fact back into a non-rival one: shared authorship. It is the chain-level reverse fork at the scale of
one piece of work, and the same positive-sum move.

Three things keep it from being a free-ride channel:
- **Similarity only nominates.** The measure flags candidates; it never merges anything. Similarity
  measures are gameable (and today's is a byte-overlap screen that misses paraphrase), so a merge needs an
  explicit act: the parties' consent, or a governance decision where consent can't be had.
- **Shapley pays a copy nothing.** A member who adds nothing to any coalition gets zero (the null-player
  axiom), so joining a valuable project with a near-copy earns nothing; only a member whose work adds
  something gets a share.
- **It is separate from plagiarism.** A copy that strips attribution is still a dispute, not a coalition.

Receipts and floors: our transfer experiments already priced coalitions this way on toy tasks (EXP-18,
data-channel synergy, Shapley split it about evenly while leave-one-out overpaid about 1.7x; EXP-19, learned
sequential composition, Shapley about even). Not built: a coalition record type, the consent or governance
path (the amendment activation point doesn't exist yet), a similarity measure suited to code (AST or
behaviour, not bytes), and a real v(S) to price members, which is the open value-layer problem.

