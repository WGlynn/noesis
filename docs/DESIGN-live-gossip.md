# DESIGN — live gossip (slice-5b, part 2) — ⚑ WILL SANITY-CHECK BEFORE BUILD

> Status: **ready-for-critique, NOT built.** slice-5b part 1 (the unify — `--listen` serves the
> durable log) shipped `cc81fe5`. This is the second half: new blocks propagating to
> already-joined peers so multiple nodes stay converged **live**. It touches multi-node consensus
> flow, so per the 2026-07-19 CONTINUE ("don't run blind on flow/standing semantics") it is
> written for Will to pick the topology BEFORE any code.

## 0. First, the honest framing: this is NOT a launch blocker

The public testnet is ONE `--serve-api` node and that is code-complete + turnkey (`fly auth login`
away). Live gossip only matters the moment you want a **second** node — a mirror for resilience, or a
friend running their own. Valuable ("the real next-code item"), but do not let it delay the deploy.

## 1. The crux: gossip ≠ consensus

Moving blocks between nodes is easy (we already have `net` framed TCP + `sync` replay + `gossip`
dedup/broadcast). The hard part is **agreeing on which blocks are canonical**. That is decided
entirely by *who is allowed to produce a block*, and the current node makes that choice for us by what
it does and doesn't support:

- **Single-proposer is assumed.** `chainspec.rs:177`: "single … proposer ⇒ every honest validator
  votes for the one valid proposal." `produce_block` (`chainspec.rs:161`) mines with one producer =
  the first bonded validator's contributor key.
- **PoS+PoM finality is per-block and FINAL.** Each block is finality-gated before the next
  (`checkpoint_finalizes`, `runtime.rs:1004`). PoW is the reorgeable layer (`runtime.rs:1559`); the
  PoS+PoM safety path is not.
- **There is no reorg / rollback / fork-choice in the node.** `Node::apply` only moves state forward.

⇒ Two producers each finalizing a block at height N = two "final" blocks the node **cannot reorg out
of**. An unresolvable split. So the topology is not a detail; it is the design.

## 2. The two topologies

### (A) Single-producer + live replicas  ·  **RECOMMENDED for testnet**

One node is the **producer** (mines + finalizes, exactly as today). Every other node is a **replica**:
it receives gossiped finalized blocks, re-validates against its own rulebook, applies, appends to its
durable store, re-broadcasts, and serves `/state` + the block log. It does **not** mine.

- **Submit routing:** a replica that gets `POST /submit` does not mine it (that would fork). It either
  (a) forwards the signed contribution to the producer's `/submit` and relays the result, or (b)
  returns `409 { producer: "<url>" }` telling the wallet where to submit. (a) is nicer UX; (b) is
  leaner and more honest about the topology. Lean recommendation: **(b)** for v0.
- **Block propagation:** on each finalized block the producer broadcasts `encode_block(b)` to its
  peers; a replica runs `Gossip::observe` (dedup), `Node::validate` (trust the rulebook, not the
  peer), `Node::apply`, `store::append_block`, then re-broadcasts. This is the exact
  validate-before-apply discipline `sync_from` and `load_chain` already use — no new trust.
- **Transport:** reuse the framed TCP `net`/`sync`/`gossip` stack + one persistent peer connection
  with a reader loop. (Alternative: piggyback HTTP with `GET /blocks?since=N` long-poll so it rides
  the one public port through a tunnel/fly — heavier duplication of `sync` but zero extra port. Fork
  for Will.)
- **Peer discovery:** a static `--peers host:port,host:port` seed list (Bitcoin-simple). No DHT.

**Guarantees:** all replicas converge to the producer's chain live; a replica restart replays its
durable store then re-syncs the gap. **Does NOT guarantee:** liveness if the producer dies (no
failover — the chain halts until it returns). That is the honest limit of v0, and it is *fine* for
"invite a few friends": it is one authoritative node with live mirrors, not a leaderless network.

This is small, reversible, and matches what the node already is. Estimated surface: a `--peers` flag,
a peer-connection manager + reader loop, broadcast-on-finalize wired into `rpc::submit_signed`, and
the replica apply path. All of it composes over existing primitives; none of it changes consensus
rules.

### (B) Multi-producer Nakamoto  ·  **research build, NOT a testnet increment**

Many nodes mine, gossip competing blocks, and a fork-choice rule (heaviest cumulative work) picks the
canonical tip; nodes reorg to it. This is the real decentralized chain. It requires, none of which
exists today: **rollback/reorg** in `Node`, a **fork-choice** rule over competing tips, **orphan/tip
management**, and **relaxing single-slot PoS+PoM finality** (or a leader-election layer so only one
node produces per height). Each of those is consensus-critical and independently large. This is a
Phase-2+ track, not the next code. Flag it, do not start it blind.

## 3. ⚑ Decisions for Will (the flow/standing semantics you wanted to check)

1. **Topology:** (A) single-producer + replicas [recommended], or (B) multi-producer [research]?
2. If (A) — **submit on a replica:** forward to producer (nicer UX) vs `409 + producer URL` (leaner)?
3. If (A) — **transport:** framed TCP gossip port (reuses `net`/`sync`/`gossip`) vs HTTP `GET
   /blocks?since=N` on the one public port (tunnel-friendly, duplicates `sync`)?
4. **Standing on replicas:** confirmed — PoM standing is a pure function of the finalized blocks, so a
   replica that applies the same blocks derives identical standing (no separate replication needed).
   Flagging only so you can confirm there is no producer-only standing state I am missing.

## 4. Autonomous-safe vs Will-gated

- **Safe to build once (A) + the two sub-forks are picked:** `--peers` flag, peer manager + reader
  loop, gossip dedup wiring, broadcast-on-finalize, replica apply-and-append, a two-producer-rejected
  regression test. None touch consensus rules.
- **Will-gated (do not touch without an explicit call):** anything that lets a second node PRODUCE, any
  reorg/rollback, any change to `finalizes_pos_pom` / the single-proposer assumption. That is
  topology (B) and it is off-limits for an autonomous increment.

## 5. Recommendation

Ship the deploy first (one node, turnkey). When a second node is wanted, build **(A)** with **(b)
submit-routing** and the **framed-TCP gossip** transport — the leanest path that keeps mirrors live
without pretending to be a leaderless network. Treat **(B)** as a named Phase-2 research track
(reorg + fork-choice + finality relaxation), not a slice.
