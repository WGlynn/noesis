# DESIGN — block logistics (size, interval) under the PoM/PoW/PoS hybrid

> **STATUS: design-first, nothing built. WORKING draft, decisions PENDING Will's ratify.**
> Consensus surgery = highest blast radius ⇒ design before code (repo convention).
> Grounded 2026-07-10 against ARCHITECTURE.md, POM-FINALITY-TEMPORALITY.md, node code, and the
> CKB NC-Max prior art. Lean-like-Bitcoin constraint applies: every knob must earn its place.

## 0. Ground truth (why this doc exists)
Block logistics are **unspecified today.** Verified in code:
- No block-size cap, no block-interval, no difficulty adjustment, no block-time target. The only
  resource bound is `Constitution.max_mempool` (DoS Bound A).
- Ordering is **commit/slice position, not wall-clock** (`lib.rs:765`
  `temporal_order_is_consensus_critical_and_timestamp_is_not_the_lever`). The `timestamp` field
  exists but is not the lever for anything consensus-critical.
So this is a blank surface. We lock it in now.

## 1. The master insight: production and finality are ALREADY decoupled
This is the fact that reshapes the whole problem. Noesis separates two weightings (ARCHITECTURE.md
§Consensus vs Finality):
- **Production / ordering / Sybil-cost / money = PoW** (NCI `pow 0.10`, `lib.rs:3289`), **EXCLUDED
  from finality** (`FINALITY_MIX` `pow 0.0`, `runtime.rs:584`) because PoW is probabilistic/reorgeable.
- **Finality / safety = PoS + PoM** (`pos 1/3 : pom 2/3` renormalized), 2/3 bar, anti-concentration
  floor ≥50% per dimension (`MIN_DIM_BPS 5000`, `runtime.rs:596`).

**Consequence for block logistics:** a produced-block fork/orphan is **NOT a safety event** here.
In pure Nakamoto (Bitcoin), the heaviest chain *is* the security, so a short interval → orphans →
security loss. In Noesis, PoM+PoS finality selects the canonical block regardless of PoW orphans. So
the production layer has *more* latitude to run fast/big than Bitcoin or even CKB — the penalty for a
production orphan is wasted JUL issuance + producer griefing surface, not lost safety. **Block size
and interval live on the PRODUCTION clock; finality is a separate cadence that confirms it.**

## 2. Prior art: CKB NC-Max is Will's instinct, already formalized
Noesis is CKB-inspired; NC-Max is the "re-derive from the prototype" lineage (not invention). Its three
moves (RFC-0020; "Breaking the Throughput Limit of Nakamoto Consensus"):
1. **Two-step propose/commit** decouples *tx* propagation from *block* propagation ⇒ allows short
   intervals. **Noesis already has this for free: commit-reveal IS the two-step.** They compose.
2. **Dynamic block interval targeting a fixed ORPHAN RATE, not a fixed block time.** Orphan rate =
   the network-connectivity / decentralization signal. Below target ⇒ lower difficulty ⇒ shorter
   interval ⇒ higher throughput ("network can sync faster, raise throughput without harming
   decentralization"); above target ⇒ back off. **This IS "dynamic on both to balance scalability and
   decentralization"** — orphan rate is the exact balance variable. Block reward ∝ 1/interval so
   emission per period is fixed.
3. **All-blocks (incl. orphans) in difficulty adjustment** ⇒ selfish-mining resistance (difficulty
   made independent of orphan rate, so an attacker can't lower difficulty by inflating orphans).

NC-Max governs the **production clock**. It is directly portable. What it does NOT cover is Noesis's
finality layer and value measure — that is where our specific work is.

## 3. Noesis is a THREE-clock system (block params must respect all three)
| Clock | Driven by | Bounds | Signal |
|---|---|---|---|
| **Production** — mint cadence + size | PoW | propagation bandwidth/latency | orphan rate (NC-Max) |
| **Finality** — confirm cadence | PoM + PoS | v(S) eval compute + validator breadth | finality lag; anti-concentration health |
| **Vesting `W`** — realized-value clears into franchise | temporality model | fraud-catch window vs responsiveness | dispute-window closure (POM-FINALITY-TEMPORALITY.md) |

The production and finality clocks should be **decoupled** (fast micro-production, batched finality
epochs — Bitcoin-NG / slot-vs-epoch lineage). The vesting clock `W` is orthogonal but interacts (see 4D).

## 4. The five Noesis-specific delicacies (what NC-Max does NOT solve for us)

### 4A. Block SIZE has a SECOND ceiling: v(S) evaluation compute
Bitcoin/CKB bound block size by propagation bandwidth. Noesis must ALSO re-price every contribution in
a block at finality: Myerson value (exponential, sampled), HodgeRank Laplacian solve + collusion
detectors (`attribution_circulation`/`attribution_cycle_energy`), learned-outcome scoring per cell.
More contributions ⇒ superlinear finality compute. **Dynamic size is bounded by
min(propagation-bandwidth, v(S)-compute-budget), whichever binds first** — often the compute wall, not
bandwidth. This is unique to a value-measuring chain.

### 4B. Anti-concentration breadth is a SAFETY floor, not an ideal
The anti-plutocracy property needs each dimension (PoM, PoS) to have enough INDEPENDENT participants to
keep neither past the 50% floor (`MIN_DIM_BPS`). If dynamic params raise the hardware/bandwidth bar
enough to price out small finalizers, the PoM and PoS sets THIN ⇒ the anti-concentration floor gets
harder to satisfy ⇒ liveness risk AND the anti-plutocracy guarantee weakens. So **decentralization has
a hard load-bearing floor here that generic chains lack.** ⇒ TWO decentralization signals are needed,
not one: orphan rate (producer breadth) AND finalizer-set breadth (the bar to be a PoM/PoS validator
must stay low).

### 4C. Commit-reveal ordering strategyproofness ↔ the reveal window
Temporal-novelty strategyproofness rests on commit ORDER (`lib.rs:765`). NC-Max's propose/commit maps
onto our commit-reveal — good — but the commit→reveal gap is a propagation buffer, and the reveal
window must be ≥ propagation time so honest reveals aren't dropped. **That window is the floor on how
short the interval can go**; pushing the interval below it would grief honest revealers and let a
well-connected producer manipulate novelty pricing.

### 4D. Dynamic interval breaks any block-height-denominated window
`W` (vesting), franchise decay, and dispute windows must be measured against a **stable clock**
(wall-clock or a cumulative-work clock), NOT block height. If interval floats (NC-Max) and these are
counted in blocks, they wobble in real time as throughput changes — a fraud-catch window `W` could
silently shrink to unsafe wall-clock duration during a throughput spike. **Correctness requirement, not
a preference.**

### 4E. Who sets the knob = capture-resistance (the blocksize-war lesson)
A dynamic block param is a governance surface. If PoW miners choose size, they bloat it to price out
finalizers; if capital (PoS) chooses, it raises the bar to concentrate. Separation-of-powers answer:
the knob moves by a **structural rule** (orphan rate + finality lag, automatic like difficulty
adjustment), never any single power's discretion. Where governance (VIBE) touches it, verifier-gated
**bounded ranges** (mirror the value-matrix mutation rule: move within bounds, nothing zeroable).
Natural Noesis move: gate block-space growth on **contribution demand (PoM breadth) + network health
(orphan rate)** — what earns more space is verified contribution + a healthy network, never capital.

## 5. The unifying frame: block logistics = a control loop (same shape as the money layer)
Maximize throughput subject to staying in the decentralization-safe region, where "safe" = three
signals held at target: **orphan rate** (producer breadth), **finality lag / v(S) compute** (finality
keeps up), **finalizer-set breadth** (anti-concentration healthy). Proportional response, damped
against oscillation. This is the SAME control-loop shape as the money layer's Trinomial Stability
System (proportional anchor + damping) — one controller geometry across money AND blocks
([[substrate-geometry-match]]). And it is LEANER than Bitcoin: orphan-rate-targeting replaces the
arbitrary fixed-interval magic constant with a value derived from measured network physics.

## 5.5 Design lens — "a blockchain built for AI, by AI" (Will, 2026-07-10; CORRECTED)
NOT an exclusivity claim — humans are welcome. The frame is an ADOPTION dynamic: **AI building the
chain imprints an AI-legible shape on it, which lowers the adoption barrier for other AI agents**
(builder-nature → adopter-ease; a self-reinforcing convergence). What actually follows for this design:
- **Adoptability / legibility is a first-class goal:** an AI agent should natively produce, verify, and
  reason about the mechanism. The block-as-thought abstraction, commit-reveal, and provenance already
  match how agents track their own work — that is *why* it is easy for AI to adopt.
- **Choices that stand on their own merits** (NOT because "agents only"): the cumulative-work logical
  clock (censorship-robust + floating-interval-safe, rec 4); the self-tuning controller (physics over
  magic constants); structural-over-discretionary governance (ANY participant games a discretionary
  knob, rec 5); pricing size in v(S)-compute (the block IS a batch of thought that finality re-prices —
  true for any participant, rec 2).
- **CORRECTION (supersedes an earlier over-read):** decentralization is NOT reweighted to
  "agent-operators only," and 4B's anti-concentration floor is not an "agents tolerate high throughput"
  license. Anti-plutocracy / broad independent participation matters for ALL adopters, human or AI. The
  Boundary-2 Sybil-robustness gap (mechanism spec §8) is a general security open problem, not a
  referendum on the AI-native thesis.

## 6. DECISIONS — RATIFIED (Will, 2026-07-10; all six recommendations approved)
1. **Interval control signal:** orphan-rate alone (NC-Max), or **orphan-rate + finality-lag** (two
   signals, because production/finality are decoupled here). — *Rec: two-signal.*
2. **Size control signal:** bandwidth vs v(S)-compute-budget. — *Rec: bound by min(both); the compute
   wall usually binds first.*
3. **Production/finality coupling:** micro-blocks (fast produce) + finality epochs (batched), or one
   cadence? — *Rec: decouple — it's the scalability unlock and safety already tolerates it.*
4. **Clock for `W` / decay / dispute windows:** wall-clock or cumulative-work, NOT block-height. —
   *Rec: cumulative-work clock (censorship-robust, matches the PoW production substrate).*
5. **Knob governance:** pure structural rule vs VIBE bounded-range vs hybrid. — *Rec: structural rule,
   VIBE only sets bounded outer limits (verifier-gated).*
6. **JUL emission ↔ interval:** reconcile NC-Max's reward∝1/interval with Ergon's reward∝difficulty
   (both are functions of difficulty ⇒ may already be consistent; needs working out with the money-layer
   design). — *Rec: derive jointly with `DESIGN-elastic-pow-money.md` so the two controllers don't fight.*

## 7. Honest status / lean check
Nothing here is built. The production-clock port (NC-Max) is well-trodden; the finality-clock governor
(4A/4B) and the stable-clock requirement (4D) are the genuinely-ours design work. Lean verdict:
dynamic-both is LEANER than fixed constants (fewer magic numbers, signals derived from physics), and
commit-reveal already gives us NC-Max's step 1 for free — so this earns its place. Next: ratify §6,
then either write the mechanism spec or run a design-and-gate pass to adversarially stress the candidate
against the safety invariants (no gameable input on the safety path; capture-resistance; anti-
concentration preservation).
