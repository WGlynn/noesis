# Resource-DoS bounding — design + build status

> Closes the honest weak leg called out in `SECURITY.md` §2 and the reviewer's
> shortcut: *"the economic gate removes the incentive; a submission bond /
> rate-limit / commit-deposit that bounds the resource cost of evaluating junk is
> designed, not built."* This doc is the design; Bound A is built, Bound B is teed.

## The threat (precise)

`Node::submit(cell, coord)` (`node/src/runtime.rs`) pushes a proposal onto an
unbounded `mempool: Vec<(Cell, Committed)>` at **zero admission cost**. A cell only
has to be *well-formed* to be gossiped; it does not have to be *valuable*. So an
attacker gossips K cheap, well-formed-but-worthless cells and the node pays:

- **Memory** — O(K) mempool growth, attacker-controlled, unbounded.
- **Compute** — `propose` (`canonical_order` over the whole pool), `validate`, and
  the post-apply `pom_scores_with_similarity_floor_q16` novelty/attribution recompute
  all scale with admitted volume.

The class-1 gameability defense already makes junk score **0** (temporal-novelty +
the `theta_sim_q16` similarity floor) — so flooding earns no standing and is
**unprofitable**. But "scores 0" is computed *after* admission and inclusion: the
resource to evaluate the junk is **already spent**. Removing the *incentive* is not
the same as bounding the *resource*. That gap is this leg.

## Two structural bounds (defense-in-depth)

### Bound A — bounded mempool admission cap  ✅ BUILT (this session)

`Constitution.max_mempool` caps the local pool. `Node::submit` returns `bool` and
**rejects admission** once `mempool.len() >= max_mempool`. This is a deterministic,
O(1), **economics-independent** ceiling on both memory and the downstream
per-proposal compute — it holds even if every economic assumption fails, which is
exactly why it is the floor and goes in first. Reject-when-full (not
evict-by-quality) is the lean v1 choice: quality-prioritised eviction needs a
quality signal *at admission time*, which is precisely what Bound B's deposit
supplies — so eviction is a Bound-B-era refinement, not a v1 mechanism.

- **Where the gate lives:** mempool admission (`submit`), pre-consensus, per-replica.
  It is a *liveness/resource* guard, not a *safety* guard — it never changes which
  blocks are valid or final, so it is consensus-adjacent but not consensus-affecting
  (every honest replica caps its own pool; the cap does not enter `validate`).
- **Blast radius:** one new `Constitution` field (ripples to the single `Default`
  impl), `submit`'s return type (`()` → `bool`, non-breaking — all call sites ignore
  it), and new tests. No value/token/finality path touched.
- **Anti-theater:** the RED→GREEN probe floods past the cap and asserts admission
  stops exactly at the cap; setting `max_mempool = usize::MAX` re-admits the whole
  flood (the bound genuinely does the work, not a coincidental limit).

### Bound B — commit-deposit refunded on genuine contribution  🟡 DESIGNED (build next, fresh)

A submission carries a deposit `d ≥ Constitution.submission_deposit`. The deposit is
**refunded** iff the cell turns out to be a genuine contribution (banks novelty /
standing `> 0` when applied) and **forfeited (burned)** iff it scores 0 — the *same*
predicate the class-1 gate already computes. This attaches a cost to the *act* of
submission that only honest work gets back, so a K-junk flood costs `K·d`. The
structure does the work: the thing that decides genuineness (the novelty +
similarity-floor signal) is the thing that decides the refund — no new judgment, no
new oracle (SubstrateGeometryMatch).

**Build contract (teed for a fresh low-context session — touches the value path):**
1. Add `submission_deposit` to `Constitution` (Q-units, default `0` so it is inert
   until a network turns it on — same inert-shape-first discipline as the lock-sig
   binding).
2. Carry the deposit on the submission (a bonded value cell referenced by the
   proposal, *not* a free producer-asserted field — keep it in the
   `[P·dont-let-attacker-choose-critical-input]` class: the bond must be a real,
   consumed value cell).
3. On `apply`, partition included cells by the novelty outcome already computed in
   `pom_scores_with_similarity_floor_q16`: refund the bond for `pom > 0` cells, burn
   it for `pom == 0` cells.
4. RED→GREEN: a junk flood loses `K·d`; an honest contribution is made whole.
   Anti-theater: stub the refund predicate to "always refund" ⇒ the junk-flood
   accounting test goes RED.

Bound B is **consensus-adjacent** (forfeiture is value movement) ⇒ build it cold,
in fresh low-context, with the value-conservation tests in view — not bundled with
Bound A.

## Status line (truth, not rounded)

| Bound | Mechanism | Status | Where |
|---|---|---|---|
| A | bounded mempool admission cap | ✅ built & tested | `runtime.rs` `Node::submit` + `Constitution.max_mempool` |
| B | commit-deposit refunded on genuine contribution | 🟡 designed, build contract above | this doc §Bound B |

This does **not** touch the PoM↔finality decision (open, Will-reserved). The mempool
cap is a per-replica resource guard with no finality semantics.
