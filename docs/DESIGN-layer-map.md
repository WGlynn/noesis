# DESIGN: The layer map (one node among many)

**In one line: the layers split where rivalry starts. Non-rival facts merge by union with no one in charge,
rival facts are settled by the parties who share them, worth is computed on top, and money stays per chain.**

> Status: **PROPOSED, ready for critique.** Nothing here is built unless a line says so. Code pointers are
> the current sketch, not law. `ARCHITECTURE.md` stays the canonical record of what is built.
> Written 2026-10-23 from `research/the-node-and-the-network-2026-10-22.md` (the thinking),
> `research/prior-art-agent-centric-chains-2026-10-22.md` (what others already learned) and
> `node/src/reconcile.rs` (the first slice), plus the day-30 devnet bar agreed with Boardy on 2026-10-22.

## 0. Why a map now

Will's method is to cram everything into one layer until it is understood, then split it into its best
form. The peer-node reframe (`55e9753c`: Noesis is one node in a network of contribution chains, not
the hub) changes which jobs can share a carrier, so it is time to split. Each layer below names what it
settles, who settles it, the component that does it today, and what the prior art already learned
about it. That last part is deliberate: we are speedrunning what blockchain learned about itself, and
the traps listed here come from the prior-art survey, either a system that hit them or a theorem that
predicts them.

## 1. The map

| # | Layer | Settles | Rival? | Who settles it | Current sketch | Status |
|---|---|---|---|---|---|---|
| 1 | Records | what one agent did | no | the owner signs; witnesses compare copies | XMSS-signed submit (`rpc.rs:73-77`); content ids (`reconcile.rs`) | signing built at the API; content ids shadow only; signatures do not travel with records |
| 2 | Fact reconciliation | which non-rival facts the network shares | no | nobody: union merge | `reconcile::merge`, `rival_claims`, `dangling` | shadow, 4 tests (`node/tests/reconcile_merge.rs`); no network path |
| 3 | Rival facts | balances, finality, authorship collisions | yes | the parties who share the thing | `FINALITY_MIX` (`runtime.rs:1664`), `MIN_DIM_BPS` (`runtime.rs:1676`), `commit_order` | two-key finality built in the reference; no countersigning, no witness meeting point, no authorship dispute |
| 4 | Value | what the facts are worth | the order of worth travels; rates float | frozen judges per round, anchored by use on chains you don't control | `ValueOracle` seam, vesting gate, endorsement dispute | seam built; judges designed; learned v(S) open |
| 5 | Money | JUL | yes | proof of work issues it, finality settles it, per chain | `jul.rs`, coinbase (`runtime.rs:80`) | issuance and coinbase built in the reference; economics unset |

Derived views sit across layers 2 to 4: **standing** is computed from facts, so once facts agree, standing
agrees (section 6 has the catch: today it needs one commit order).

## 2. The rule that draws the boundaries

A fact is non-rival when no merge of honest records can break a rule. Those facts go in layer 2 and
need no consensus. Kleppmann and Howard (2020) prove that, in their model, a peer-to-peer store tolerates
any number of bad peers exactly when every operation has this property. Uniqueness by content hash and
derived views are among the cases they list as safe. A fact is rival when two records of it can't both stand. It needs the parties who can
spend it, and only them. Guerraoui et al. (2019) show money out of a single-owner account needs no
consensus, only an owner who can't equivocate, while an account with k owners needs consensus among k.

Receipt that the ledger already draws this line: `Ledger.cells` holds contributions and
`Ledger.token_cells` holds tokens (`runtime.rs:206`, `:226`).

The boundary is prior art (Holochain whitepaper 2.0 states it almost word for word). What the agent-centric
systems lack is layer 4 and paying witnesses. Contribution scoring outside that family (SourceCred,
retroactive public-goods funding) is not surveyed yet, so we make no novelty claim on layer 4 until it is.

## 3. Layer by layer

### 3.1 Records

- **Built:** the wallet signs `xmss::contribution_digest(address, index, data)` and the node verifies it
  at submit (`rpc.rs:73-77`).
- **Not built:** content ids in the live `Cell`. Its `id` is a local `u64` counter and `parent` points at
  that counter (`lib.rs:138-149`), so no chain can name another chain's work. The signature is checked
  at the door and then dropped. Key rotation, several devices and restoring from backup are not built.
- **Traps:** per-author counters as identity can be corrupted (Kleppmann-Howard), so use hash links.
  SSB killed honest identities when a second device or a backup restore forked the log
  ("irrecoverably stuck"). Multi-device keys and a signed "abandon this branch" record belong in the
  first design, not a later patch.

### 3.2 Fact reconciliation

- **Built (shadow):** `ContentId` = blake2b over the author, the data hash and the parent's ContentId, and
  it leaves out the local id, timestamp and lock. `merge` is a union keyed by ContentId: commutative,
  associative, idempotent. `insert` refuses a record whose id doesn't re-derive from its fields.
  `rival_claims` lists the same data under more than one author. `dangling` lists records whose parent
  is missing (a claim waiting for its witness).
- **A fork here is harmless.** Both branches are valid signed facts, so take both. Penalties belong only
  to forks that disagree on rival facts (3.3). Punishing a fork of non-rival facts punishes accidents
  (a backup restore, a second device) and buys no safety.
- **Gaps:** `rival_claims` matches exact data hashes only, and paraphrase slips past (the submit
  screen, `screen.rs`, is a node-local byte-overlap check). A record has one parent, but real work cites
  many. An overlap that both sides record is not a record type yet, so a one-sided citation stays a
  claim.

### 3.3 Rival facts

- **Single-owner chains:** no consensus. The owner must not equivocate. That needs (a) a **meeting point**
  for heads, because SSB's fork detection failed when two counterparties never compared copies (Holochain's
  answer is the author's own neighborhood: a role many nodes play, not a hub); (b) **cosigned heads
  before acceptance** where a rival fact is at stake, because catching a fork afterwards doesn't
  prevent it (C2SP witnesses); (c) witnesses that are **costly to fake**: stake, or a standing snapshot
  frozen per epoch, never standing earned from the facts being witnessed (that loop is circular).
- **Shared chains:** two-key finality. PoW off the safety path, PoS 1/3 : PoM 2/3 (`runtime.rs:1664`),
  each side at least 50% of its own dimension (`runtime.rs:1676`).
- **Order inside one chain:** commit-reveal ordering, height then an XOR-seeded in-block slot
  (`commit_order`, enforced at `runtime.rs:1262`). **There is no order across chains, and the merge
  deliberately never creates one.**
- **Traps:** treating detection as prevention (Holochain countersigning "is not for preventing
  double-spends"); self-declared timestamps deciding anything rival (Nostr `created_at`, SSB gatherings);
  "earliest anchor wins" while the content stays secret (Ceramic's own spec calls its data-withholding
  case a double spend).

### 3.4 Value

- The order of worth travels and the exchange rates between judges' units float (EXP-62b, 62h). Freeze
  rates per round or let a market set them; forcing them to converge by repair can cycle (Scarf).
- The anchor against wash is use by minds you don't control. In a network of chains that means use
  recorded on chains you don't control, so the interop incentive and the anti-wash anchor are one
  mechanism. Floor: a ring of chains that only use each other is wash one level up.
- Payouts must scale with value created, not divide a fixed pot, or the fight returns over the pot.
  Where rewards come from is still open in the tokenomics.
- **Every overlap record pays both parties** (witness and cited). This is untested, and no system in the
  survey does it. Without the independence test it becomes a wash subsidy.
- **Built:** the `ValueOracle` seam, the vesting gate, endorsement disputes. The fence holds: standing
  cannot read the value layer (test at `runtime.rs:3209`). **Open:** a learned v(S) on real labels.

### 3.5 Money

JUL is rival, issued by proof of work and settled by finality, per chain. Moving it between chains is
the bridge problem, the hard kind, and this map doesn't solve it. Issuance and coinbase are built in
the reference; the stability reserve is built but unwired; economics are unset (`ARCHITECTURE.md` money row).

## 4. Rival authorship: the split is the rule, and first commit pays nobody (proposal)

`rival_claims` surfaces the collision; this section is what happens next. The 10-22 note listed three
resolutions: priority, a plagiarism dispute, or a coalition. Boardy objected (10-22) that
first-commit-wins fights the thesis: it rewards racing, lets a rough early version beat a good later
one, and rebuilds winner-take-all with a timestamp on it. He proposed the coalition as the default with
first commit as the fallback. This map goes one step further, because a fallback that hands the whole
thing to the first committer gives that person a reason to refuse, and the fallback quietly becomes
the rule. So:

1. **Classify first, by access.** Commit-reveal proves you held the content when you committed. If A's
   content was **published before B committed**, B could have copied: presumed copy, burden on B. If
   **both committed before either was published**, neither could have copied: independent. Timestamps
   enter only here, as evidence of who could see what. They never decide a payout.
   - "Before" comes from the causal past first: if B's record hash-links to a head containing A's
     publication, B provably had access (the converse fails, since B could have read it elsewhere).
     Otherwise it comes from attested time (`DESIGN-committee-attested-clock.md`, designed), never from a
     self-declared timestamp.
   - A commitment counts as evidence only if its content is published within an enforced reveal window
     (the Ceramic lesson).
2. **Copy: dispute.** The copy leaves the player set and earns nothing. Stripping attribution is
   plagiarism, a dispute. The nearest built machinery is the endorsement dispute (`lib.rs:4677`). An
   authorship dispute is not built.
3. **Independent: coalition.** Shapley split, by the parties' consent, or governance imposes the same
   split if one party refuses. Being first earns no premium.

**Correction to the 10-22 note, section 12 ("Shapley pays a copy nothing").** Not as stated. Under a v that
looks only at outcomes, an exact copy is not a null player: alone it delivers the full value, so
v({copy}) = v({original}) = v({both}) = V, and Shapley's symmetry splits V in half. The null-player axiom
needs v(S + copy) = v(S) for every S, including the empty coalition. A copy earns zero only if step 1
removes it, or if v credits the original for the copy existing at all (a counterfactual v). So the access
test is load-bearing: without it, "copy anything valuable under a fresh key and claim independent
discovery" earns half.

**A useful accident of the built surface.** `rival_claims` matches exact data hashes, and exact duplicates
are symmetric players under any v. So for every collision the code can surface today, the Shapley split
is the equal split, computable with no value oracle at all. That means coalition resolution for exact
duplicates (access test, then an equal split) could be built before the value layer exists. Near-duplicates
of different quality need a real v(S), the open problem. (For independent rough-early vs good-later work: early gets
v(early)/2 + (v(both) - v(later))/2, and later gets more.)

**Tension with the built value rule.** `temporal_novelty` (`lib.rs:183`) values each cell by coverage
novel against every earlier cell in canonical commit order (`lib.rs:225-245`), and standing is computed
from it (`runtime.rs:1534`). That is first-commit-wins by construction. Its job is to stop padding: a
later sybil or duplicate cell earns 0, and inside one chain with one order it does that job. Two
consequences:
- It doesn't lift to the network. There is no order across chains, so standing is a view per shared
  chain, which fits the 10-22 note's suggestion that each shared chain names its own judge.
- Inside one chain, two people who commit the same work in the same block provably couldn't have copied
  each other, yet the XOR-seeded slot decides which one earns the standing. That is a lottery between
  independent authors. Whether standing should split like value does is an open question for Will
  (section 9).

## 5. The minimal reconciliation spec: what a contribution chain must record

This is the interface another chain implements to join the network as a peer. It is the "written spec"
half of the devnet bar.

1. **Name every contribution by content:** ContentId = H(author key, H(data), parent ContentId or root),
   domain-separated (`reconcile::content_id`). Never a local counter, height or timestamp in the id.
2. **Carry the author's signature over the ContentId** with the record. Accept a record if and only if
   the id re-derives and the signature verifies. *(Not built.)*
3. **Record overlaps on both sides.** A citation names a foreign ContentId as parent
   (`FactRecord::new` allows it); until the cited side records it too, it is a claim (`dangling`).
   *(Multiple parents not built.)*
4. **Publish signed heads (latest id and length) to a meeting point** so forks shown to different
   counterparties get compared. *(Not built.)*
5. **Commit before reveal, and publish within the reveal window,** or the commitment is not evidence.
6. **Never decide a rival question from a self-declared timestamp.**

Not required: a shared consensus, a shared clock for non-rival facts, a shared value function, or
Noesis itself. Any chain that records these six things is a node.

## 6. Roles still sharing one carrier

- **`temporal_novelty` does two jobs:** it stops padding (needed) and awards priority between distinct
  authors (what section 4 says should not pay). Splitting them is hard because a sybil looks exactly
  like a second author. The access test is the only separator on the table, so this split waits on it.
- **PoM standing** is both a finality vote and a measure of contribution. It is fenced from the value
  layer (`runtime.rs:3209`), so it is one number with one input but two uses. Keep it together until
  the value layer is live; splitting it early would be splitting before understanding.
- **The devnet node** holds records, reconciliation, consensus and the API in one process, and
  `noesisd` runs either `--listen` or `--serve-api`, one mode per process (`bin/noesisd.rs:288`, `:299`). So a
  node can't serve the wallet and talk to peers at the same time yet.
- **"Noesis"** names both the protocol and a chain.

## 7. The day-30 devnet bar is the test of layers 1 to 3

Agreed with Boardy, 2026-10-22. A devnet is team-run and resettable. The bar:

| Bar item | Layer | Where it stands |
|---|---|---|
| 2+ team-run nodes, separate programs on separate machines, swapping records over the network as they go | 1-2 | `net`, `gossip`, `sync` exist; not wired into a running node alongside the API |
| content ids in the live record | 1 | shadow only (`reconcile.rs`) |
| seeded shuffles of arrival order, from a published seed list anyone can rerun, all land on one hash | 2 | unit test checks all 6 orders of 3 small sets; nothing shuffles real traffic |
| a node taken offline catches up | 2 | `sync` replays a block log; fact catch-up not built |
| rival authorship **flagged, not resolved**, and the post says that is the line | 2 | `rival_claims` surfaces it in the shadow |
| a forged record (someone else's key) is rejected, **and the genuine record arriving after it is still accepted** | 1-2 | not testable yet: records carry no signature. Gossip dedups on a hash of the whole frame (`gossip.rs:35-48`) and `insert` keeps only accepted records, so the denial case should not bite, but that is a reading, not a test |
| rebuildable from a tagged release and a written spec | all | section 5 is the start of the spec |
| resettable | ops | not started |

Out of scope for the devnet, by design: strangers' nodes, a faucet, an explorer, incentives, real
pricing, contributors voting in finality. Those belong to the public and incentivized testnets.

## 8. Second pass: arguing with this map

**"The access test separates copies from independent work."** Counter: access that happens off the
protocol (a leak, a private demo, a talk) is invisible. A copier with early private access commits
before publication and passes as independent, earning half. Answer: the test catches copying from
public work, the cheap and common case. Copying with private access needs the dispute layer and outside
evidence, and what is left gets priced rather than excluded, the same treatment as wash. A narrow reveal
window also limits committing early and waiting to see.

**"Governance imposes the split when someone refuses."** Counter: imposing a split needs v(S), which is
the open problem. Until then governance prices by hand, which is Sensorica's social valuation, and that
can be captured. Answer: for exact duplicates, the only case the code surfaces today, the split is
equal under any v, so nothing is priced by hand. For near-duplicates, imposed splits wait for v(S); until
then the collision stays flagged.

**"Witnesses need a meeting point."** Counter: a meeting point is a hub, which is what Noesis says it is
not. Answer: a meeting point per author (the nodes nearest the author's key) is a role many nodes play,
not one hub. It still means someone stores and serves heads, which is the unpaid-infrastructure trap
unless witnessing pays. That is H3, untested.

What survives: the rival / non-rival split as the layer boundary, the reconciliation spec as the
interface, and the access test as what makes the coalition rule safe. What it adds to the open list:
whether standing splits too, and who stores heads.

## 9. Open questions for Will

1. Should standing also split between independent co-authors, or stay first in commit order? Today a
   same-block tie is decided by the XOR-seeded slot.
2. Is "presumed copy after publication, with the burden on the later committer" an acceptable cost for
   honest people who rediscover something later?
3. Multiple parents per record: needed before the devnet, or after?

## 10. Next (Will-gated)

- Survey SourceCred and retroactive public-goods funding before any novelty claim on layer 4.
- Smallest build toward the devnet bar, in order: signatures on `FactRecord` plus the forgery-then-genuine
  test; a seeded-shuffle harness over `merge`; content ids in the live `Cell` (this touches consensus, so
  it goes to Will first).
