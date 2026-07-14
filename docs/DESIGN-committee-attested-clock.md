# DESIGN — The Committee-Attested Wall-Clock (Phase-2 timestamp source)

> Status labels: ✅ built · 🟡 designed-not-built · 🔬 open. Build-in-open; every claim labelled honestly.
> This spec is **ready-for-critique**, not finished. It resolves the Phase-2 timestamp source ratified in
> `DESIGN-M3-jul-economics.md` §2/§0.1 (branch (a), committee-attested form) into a concrete, buildable
> mechanism, and fixes its trust model. The reframe at its center — *time is an airgap instance, not an
> exogenous oracle* — is Will's (2026-07-14); this note traces the structural payload.

Companion reading: `research/bitcoins-missing-wall-clock.md` (why a chain has no clock), memory
`[[noesis-time-is-read-not-faked]]` (the two-clocks separation), `DESIGN-M3-jul-economics.md` §2.1
(the VibeSwap precedents), `DESIGN-pow-work-dimension.md` (`FINALITY_MIX.pow == 0`, the blast-radius fence).

---

## 0. What this is for

`noesis_core::pow::next_target` (inc-M3-1) takes an `observed: Option<u64>` — the observed elapsed time
since the retarget anchor. In Phase-1 that argument is `None` (no clock ⇒ exactly-on-schedule ⇒ difficulty
unchanged). This spec defines where the `Some(elapsed)` comes from in Phase-2, **trustlessly**.

It defines ONLY the physical clock. Noesis runs **two clocks** (`[[noesis-time-is-read-not-faked]]`):
- **Economic / ordering clock = cumulative work** (`now() == work`). Internal, endogenous, deterministic.
  The chain owns it. Unchanged by this spec.
- **Physical clock = time-of-day.** External reality humanity already agrees on (TAI/UTC via NTP/GPS,
  governed by IERS leap-second votes — atomic physics → committee → governance ≡ *Physics > Constitution
  > Governance*). The chain **reads** it; it never fakes it. This spec is how it reads it.

---

## 1. Trust model — as trustless as a double-spend

Time-of-day is an **airgap instance**, not a special exogenous-oracle dependency. It is an off-chain fact
the chain must import, structurally identical to every other off-chain-truth problem Noesis closes by
making dishonesty unprofitable (`[[honesty-as-structural-load-bearing-property]]`,
`[[airgap-problem-blockchain-vs-reality]]`). Three properties place its trustlessness at the ceiling the
design permits:

1. **The attesting set IS the chain's own consensus set** — the bonded PoS+PoM finalizer set, with the
   same bond and the same slashing. No new, smaller, weaker trust body is introduced. The clock inherits
   *exactly* the chain's finality trust model. (`[[recursive-self-consistency]]`: a chain that can't
   trustlessly import "what time is it" has no business claiming it can import "what work is valuable" —
   both are off-chain. Refusing to fake time is the same move as refusing to fake value.)

2. **Every node is an independent witness.** Each node has its own wall-clock and **validates the reported
   time against its own reading**. Falsifying network time therefore requires not fooling a committee but
   defeating the whole network's independent clocks — which requires a **bonded supermajority**, the *same
   threshold as a double-spend*. This is not a committee attesting to passive observers; it is a network of
   witnesses.

3. **Failure mode is identical to consensus failure, not additional to it.** Absolute time has no on-chain
   ground truth (you cannot prove "real UTC was X" in-protocol), so slashing punishes **deviation from the
   peer aggregate**, not deviation from truth. That closes the airgap against any *minority* liar and fails
   only against a *colluding supermajority* — which is the exact threshold at which finality itself fails.
   The clock breaks precisely when the chain breaks, and not one attacker sooner.

**Time is the *easy* airgap case.** The hard instance is *value* (PoM) — subjective, fuzzy, riding on the
open learned-`v(S)` moat (🔬). Time is a scalar, monotone, and peer-deviation is *objectively checkable*.
If the airgap thesis holds for the hard case, it holds for time *a fortiori*. The clock is the cleanest
demonstration of the closure, not an exception to it.

**The sharp version: PoW is the only cryptographic truth a blockchain produces; everything else IS the
oracle problem.** A proof-of-work is *self-evident* — `hash(header) ≤ target` is checkable by anyone
against nothing but the artifact itself. It proves an objective physical fact (energy was burned) with no
external referent and no appeal to consensus. It is the **unique self-verifying object** in the system.
Everything else is either *authorization* (a signature proves who signed, not who is *entitled*) or the
*oracle problem* (a fact the chain cannot self-verify — which history is canonical, who holds stake, what a
contribution is worth, and yes, what time it is), settled by economic/social consensus, not by mathematics.
So the clock does **not** add oracle-dependence to an otherwise cryptographically-pure chain. It joins the
~everything that was *always* oracle-shaped, sitting atop the one self-evident anchor beneath all of it.
The clock is not a new weak link — it is the *same epistemic category as finality itself* (Will, 2026-07-14).

**PoW is still an anchor to objective reality in consensus — but for on-chain data, not off-chain (Will,
2026-07-14).** Keeping PoW in the consensus mix (NCI `pow 0.10`) means the chain retains a tie to physics.
But that anchor secures **on-chain** facts — block production, Sybil-cost, JUL issuance (energy → coins) —
not **off-chain** veracity. It cannot prove what time it is. (Note Noesis excludes PoW from *finality*
(`FINALITY_MIX.pow == 0`) yet PoW remains the objective anchor for production/issuance — the anchor role and
the finality role are distinct.) So the epistemics are three layers, not two:
- **L0 — on-chain, self-evident (PoW):** physics/thermodynamic truth. Ordering, Sybil-cost, issuance.
- **L1 — on-chain, incentive-settled (PoS/PoM/finality/state):** *validity* cryptographic, *canonicity*
  incentive-based. The chain's internal consensus.
- **L2 — off-chain, imported (time, oracle data):** objectively real, but **not self-verifiable on-chain**;
  attestation *validity* cryptographic, *veracity* incentive-based. The clock lives here.

The clock (L2) tunes L0's difficulty without corrupting it — the oracle sets the bar's height; PoW's
self-evidence of clearing the bar is untouched.

**And the confluence with the blast-radius fence (§3): the one oracle input tunes the one self-evident
mechanism, but cannot corrupt it.** Time feeds the retarget, which sets PoW difficulty. Yet a seal that
meets its target is valid *on its own terms* regardless of whether the target was set by an honest or a
compromised clock. The clock can misconfigure difficulty; it can never make an invalid proof look valid, or
a valid one invalid. **The oracle sets the height of the bar; the cryptographic truth of clearing the bar is
untouched.** This is why time can ride on PoW without diluting the one thing that is not an oracle.

**Validity ≠ veracity — do not repeat the Chainlink marketing error.** Chainlink was marketed as
"cryptographic truth," but only its *validity* is cryptographic: the aggregated report is provably
well-formed and signed. Its *veracity* — whether the reported number matches reality — stayed
incentive-based (bonded nodes paid to report honestly). Conflating the two is a marketing error, not a
security property. Applied here: a BLS-signed attestation that validator *V* reported time *T* is
cryptographically **valid**; that *T* equals real UTC is **veracity**, and veracity is incentive-based
(relative-deviation slashing at the supermajority threshold). This spec claims only what is true — the
mechanism's *validity* is cryptographic, its *veracity* sits at the consensus-failure threshold — and we
market it as **trust-minimized, never as cryptographic truth** (Will, 2026-07-14). Per §1's sharp point,
this is not a weakness peculiar to the clock: outside PoW, *nothing* on any chain has cryptographic
veracity; validity is all cryptography ever delivers, and the rest is incentives.

**Honest residue (stated, not buried).** This trustlessness is therefore **incentive-based**
(game-theoretic: dishonesty unprofitable), not cryptographic (hash-certain) — the same *kind* of guarantee
as PoS finality and the PoM value layer, and, per the point above, the same kind as *everything* on the
chain except PoW. It sits at the consensus-failure threshold, closed by relative-deviation slashing and
fenced by blast radius (§3).

---

## 2. Mechanism — optimistic, challenge-on-deviation

The happy path is **silent and free**; the machinery activates only as a **dispute**. This is Noesis's
existing commit-reveal / refutation shape applied to time, and it is leaner than continuous aggregation.

1. **Propose.** The block proposer stamps the block with a reported wall-clock time `t`. (Data-model:
   a `timestamp` on the block/seal; excluded from `state_digest` — it never touches replay-determinism.)
2. **Validate universally.** Every validating node checks `|t − local_clock| ≤ δ` against *its own* clock.
   Within the band ⇒ silently accepted. This is a **node-local admission rule** (like Bitcoin's future-time
   bound), NOT a replayable state rule — see §4.
3. **Challenge on deviation.** A node seeing `|t − local| > δ` **gossips a deviation challenge**. No gossip
   traffic in the common case; the network only speaks up when a clock is out of line.
4. **Adjudicate by bonded stake.** The dispute resolves by **bonded-stake supermajority**, NOT node count
   (§5 — Sybil). Beyond δ, the proposer needs a supermajority to make a fake `t` stick ⇒ the double-spend
   threshold. A liar (proposer whose `t` the bonded supermajority rejects) is slashed; the block's `t` is
   rejected.
5. **Feed the retarget.** The accepted times feed `next_target`'s `observed` term as
   `observed = attested_now − attested_at_anchor` (monotone; see `wallclock::observed_elapsed`). Nowhere
   else. Ordering-only parts use block-count, never the clock (`expected = ideal_interval · height_delta`).

**Why not continuous BLS-median?** An earlier draft had the committee BLS-aggregate a median every block.
That is heavier and unnecessary: universal per-node validation + challenge-on-deviation gives the same
supermajority guarantee with zero happy-path overhead. A canonical aggregate (median / BLS) is only needed
*inside* a fired dispute, to compute the reference the proposer's `t` is judged against — not every block.

---

## 3. Blast-radius fence — the decisive lever

Do not only ask *who* attests; fence *what the clock can touch*. The reported time feeds **only**
`next_target` (difficulty), and difficulty is **finality-excluded** (`FINALITY_MIX.pow == 0`, locked by
the `pow_stays_out_of_finality` test). So the blast radius of a *fully compromised* clock is bounded to
**difficulty mis-tuning — cadence, never safety**. It cannot reverse finality, cannot steal, cannot
double-spend. And a mistuned difficulty is caught by the never-halt min-difficulty floor (🔬
designed-not-built): bad clock → bad difficulty → floor keeps blocks coming. `[[structure-does-the-work]]`:
you do not need the clock to be *perfectly* trustless if the architecture structurally forbids it from
hurting anything past cadence.

---

## 4. Determinism boundary (load-bearing)

The timestamp splits cleanly:

- **Node-local, non-deterministic (admission):** the `|t − local_clock| ≤ δ` check. Each node uses its own
  clock at validation time; this is NOT part of replay and NOT in `state_digest` — exactly like Bitcoin's
  "reject a block > 2h in my future" rule. Two honest nodes may briefly disagree at the δ boundary; the
  challenge/adjudication path resolves it. **Never fold `local_clock` into a state transition.**
- **Deterministic, replayable:** monotonicity of `t` vs the previous accepted `t`, and the
  `observed_elapsed` value fed to `next_target`. Because the timestamp is excluded from `state_digest` and
  only sets the *next* block's target (difficulty, finality-excluded), replay determinism is preserved:
  the same block sequence yields the same state regardless of any node's live wall-clock.

This is the two-clocks argument in code form: the economic clock (`work`) stays the sole determinant of
state; the physical clock only tunes difficulty.

---

## 5. The two ⚑ parameters + slashing

- **⚑ δ (tolerance band).** Real tradeoff: too tight ⇒ honest nodes with legitimate clock skew (NTP drift,
  latency, no GPS) false-alarm and gossip constantly (liveness noise); too loose ⇒ a proposer has a free
  ±δ nudge, and δ-per-block can slow-timewarp. The double-spend equivalence is **exact outside ±δ**; inside
  it there is a **bounded, finality-safe** grinding margin (the §7 bounded-timewarp residual of DESIGN-M3).
  Recommendation: δ on the order of the ideal block interval (minutes), pinned against real validator clock
  skew in testnet. Governable, bounded away from 0 and from "hours."
- **⚑ max_staleness (origin-freshness).** Bound a reported/attested time's age against its **origin**, not
  against last-seen state (VibeSwap C49-F1: `now ≤ origin_deadline + MAX_STALENESS`, 5 min). Rejects a
  replayed old-but-valid attestation. Only load-bearing once attestations become referenced objects (the
  dispute path); documented here, wired in Phase-2.
- **Slashing.** On **relative deviation from the bonded aggregate beyond δ** — objectively checkable
  in-protocol (attestations compared to each other), because "wrong vs real UTC" is not. Penalty per the
  bonded-set slashing precedent. Detection is by-every-node (many witnesses); **adjudication is
  stake-weighted** (§ next).

---

## 6. Sybil — detection is by-node, adjudication is by-stake

"Every node validates" is right for **detection** — a swarm of independent witnesses. But when the
deviation-gossip fires, **who wins the dispute must resolve by bonded stake, not node count** — else an
attacker spins up cheap nodes that all "attest" the fake time and drown the honest gossip. A double-spend
is also resolved by stake/work-weight, not headcount, so this keeps the equivalence honest. This is the one
place a naive "count the nodes" implementation silently reopens the hole.

---

## 6.1 What Pragma coherence adds — the mechanism, not the fact (🔬 / partner)

Pragma's Coherence engine (`[[pragma-os-crys-contact]]`; Tom Lindeman / Bernhard Mueller — integration is
terms-first, POC-offered, **not built**) helps here, but only in a bounded way that must not be overclaimed
into the Nazarov error one layer up.

**What it CAN'T do:** give the external fact cryptographic veracity. No amount of rule-coherence-checking
proves `T == real UTC`. The airgap for the *imported fact* stays incentive-based (L2). Claiming Pragma
"makes the clock cryptographically true" would be the exact validity≠veracity error from §1, moved up a level.

**What it CAN do — strengthen the veracity of the *mechanism* (L1 rules that process the L2 fact):**
- **Confluence** — machine-check that the deviation-challenge / adjudication rules are *confluent*: every
  honest node, given the same attestations, reaches the **same** accept/reject/slash verdict regardless of
  gossip order. That removes a real failure mode — "the mechanism told different nodes different things,"
  which is a veracity failure of the *mechanism* even when each attestation is individually valid.
- **Reconcile** — Pragma's agent-state reconciliation is, structurally, the clock's dispute-convergence
  (all nodes converge on one accepted time). A direct fit.
- **Confluence on amendments** — verify a governance change to δ / the slashing rule stays *axiom-preserving*
  (e.g. does not silently flip adjudication from stake-weighted to node-count and reopen the §6 Sybil hole).
  This is Tom's dangerous "confluent + axiom-breaking" quadrant applied to the clock's own rules, hooked
  through the built amendment socket (`node/src/amendment.rs`).

**Net:** Pragma raises the *floor* of the incentive-veracity layer — it turns "we argue the incentives hold"
into "the rule-processing is machine-checked coherent," which is the honest §1 obligation (*show* the
incentives hold structurally) discharged rigorously. It does **not** convert incentive-veracity into
cryptographic veracity. It makes the bridge provably coherent; it never crosses the airgap.

## 7. Liveness / never-halt

The clock is **optional by construction** — the `observed: Option<u64>` seam. No usable attestation (a
stalled committee, a network partition) ⇒ `None` ⇒ exactly-on-schedule ⇒ no retarget ⇒ the chain continues
on the current difficulty (Phase-1 behavior). Combined with the min-difficulty floor, clock unavailability
degrades *difficulty regulation quality*, never liveness. The chain NEVER halts for lack of a clock
(`[[noesis-never-halt-chain]]`). The stall-*detector* (a chain that has stopped producing) is a **separate**
concern — the gossiped committee wall-clocks that keep ticking through a halt — and belongs to the
never-halt liveness item, cross-referenced here, not built in this spec.

---

## 8. Status

- ✅ **built (kernel):** the pure validation kernel `node/src/wallclock.rs` — `within_tolerance`,
  `advances_monotonically`, `observed_elapsed` — with RED-first tests. Consensus-isolated shadow module
  (the `jul.rs`/`reserve.rs` precedent); no consensus wiring, no `state_digest` touch, additive.
- ✅ **built (inc-CLK-1, INERT ADDITIVE — the M2a-1 precedent):** the block `timestamp: Option<u64>`
  field carried on the wire (serde-default; legacy logs decode `None`) + the node-local admission
  SEMANTICS `runtime::timestamp_admissible` shipped as a pure, tested helper, + the
  `observed_elapsed → next_target` seam proven by test. **No consensus path reads or enforces the
  timestamp** — it never enters `validate_block`/`state_digest`, replays deterministically even for
  absurd values. `node/tests/clock_wiring.rs` (6 RED-first tests). Never a live ordering rule without
  its magnitude guard (a monotonicity rule shipped without the admission bound bricks the channel on one
  `Some(u64::MAX)` block — the Council finding that scoped inc-CLK-1 down to pure additive data).
- ✅ **built (inc-CLK-2 — the deterministic enforcement bundle, flag-gated on `Constitution.clock_enforced`,
  default inert):** (1) `header_digest` binds the `timestamp` (option-tagged like `coinbase`/`parent`) ⇒ a
  solved seal cannot replay onto an altered time; (2) `validate_block` enforces a PRESENT + NON-DECREASING
  timestamp (`clock_check`, single-sourced from `wallclock::advances_monotonically` — `≥`, not strict, so
  same-tick blocks pass) with typed `Violation::{TimestampMissing, TimestampNotMonotone}`, baseline
  `Ledger::last_timestamp` advanced in `apply_transition` (excluded from `state_digest`, history-derivable);
  (3) the NODE-LOCAL admission ingress `Node::admits(b, local_now, δ)` = deterministic `validate` + the
  forward-skew bound, kept OUT of the replay path. Genesis-admission gate `clock_enforced ⇒ pow_enforced`
  (`Node::new`). `node/tests/clock_enforcement.rs` (6 RED-first tests, anti-theater breaks named).
  NEVER-HALT: non-strict `≥` means a far-future stamp cannot brick production (successors may reuse it) —
  it would only freeze forward progress, and the admission bound rejects it at production so the freeze
  never lands. The ordering rule + its admission guard ship as ONE unit (the inc-CLK-1 Council lesson).
- 🟡 **still Phase-2 (deploy-coupled):** the LIVE retarget activation (`next_target` on the block path with
  anchor state + genesis bits — needs the committee-attested time SOURCE, so wiring it now would fabricate
  `observed`, violating read-not-faked); the daemon accept-loop call-site for `admits`; the
  deviation-challenge/gossip path + stake-weighted adjudication + slashing.
- ✅ **⚑ ratified (Will 2026-07-14):** δ = 120 s (≈ the 120 s ordering-block interval), node-local config
  (NOT a Constitution field); max_staleness = 300 s; max-forward-skew = 7200 s; ASERT half-life = 2 d.
- 🔬 **open:** the never-halt stall-detector; whether the in-dispute reference aggregate is a plain median
  or BLS-signed (only matters inside a fired dispute); the numbers pinned against real validator skew at testnet.

## 9. Open questions for critique

1. **δ selection** — interval-scale vs a fixed wall value; governable range.
2. **Attestation cadence** — is `t` per-block (proposer-stamped, as specced) sufficient, or does the
   retarget want a periodic checkpoint attestation independent of block production?
3. **In-dispute reference** — plain median of bonded readings vs BLS-signed median; the cost only lands
   inside a (rare) dispute, so leanest-that-adjudicates wins.
4. **Committee = full finalizer set vs a rotating subset** — full set maximizes trustlessness; a subset
   reduces attestation load. Recommendation: full set (load is happy-path-free anyway).
