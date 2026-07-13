# 2026-07-13 (PM) — The AI became a swappable part, double-voters get caught, and we figured out where the AI actually lives

**In one line:** we turned the "AI" inside Noesis into a clean, swappable component (so shipping the
honest simple version today doesn't lock out the smart version later), wired up the rule that punishes
a validator for voting two ways at once, and answered the question that had been nagging — *is the AI
on-chain, or is it just the nodes?* Plus we made a real start on protecting the whole project from ever
being erased or captured.

---

## What shipped today (in human terms)

### 1. An honesty gate that can't be skipped
There's a personal faith-frame in the project (the "base case is God" idea) that must never leak into
the technical surfaces — the code, the whitepaper, the formal proofs — because rounding a private
belief up into a technical claim would poison the honesty that is the whole moat. Until today that rule
lived in our heads. Now it's a **pre-commit check**: before any commit lands, a small script scans the
formal surfaces, and if the forbidden term is there, the commit fails. We tested it by deliberately
planting a violation and confirming it blocked. The lesson we keep relearning: for a boundary that
matters, put it in the *structure*, not your willpower. (Also fixed a cosmetic display glitch in that
tool.) **Built.**

### 2. The AI is now a swappable part (the big one)
Noesis is an AI/blockchain system: contributions flow through a "value function" `v(S)` that scores
them, and that score becomes voting weight over finalizing blocks. The temptation is to weld the AI
into consensus. We did the opposite. We named the boundary explicitly — a `ValueOracle` interface with
a hard contract (must be deterministic, integer, identical on every machine). The current scorer (a
transparent novelty heuristic) plugs in behind it and is honestly labeled *designed, not learned*. A
future learned model plugs into the **same** slot later, with no rebuild.

Why this matters: you were torn between "get it right" (the hard, data-gated smart model) and "get it
done" (ship something usable now). This dissolves the conflict — they're now sequential, not a
trade-off. Ship the honest simple version today; the smart version is a drop-in swap when the data
exists. We proved it works with two tests: one showing the new path is byte-for-byte identical to the
old (no consensus change), and one showing a *different* scorer genuinely changes the output (the swap
is real, not decorative). **Built.**

### 3. Double-voters now get caught on the live path
A validator that votes for two different blocks at once (equivocation) was a known hole: the guard
existed but nothing on the live finality decision actually called it. Now it does — a double-signer's
weight is **stripped before any weight is counted**, which we proved flips a block from "would
finalize" to "does not finalize," and reports the offender for slashing. **Built.** (Honest limit:
making the punishment *stick* across time needs the validator registry that comes with the network
build — for now the decision is protected and the offender is flagged.)

### 4. We figured out where the AI actually lives
Sparked by a paper you sent (small models + a good "harness" match big models at ~4% of the cost) and
your own design instinct. The answer: a real language model **can't** run inside consensus, because
consensus needs every machine to compute the identical bit and model inference isn't deterministic. So
the models aren't on-chain and aren't quite "just nodes" either — they're **node-level oracles** whose
fuzzy judgments ("is contribution A better than B?") reach agreement through Tim Cotten's **CRPC**
(commit-reveal pairwise comparison) over the naturally-sharded contribution graph. The chain is the
*harness* around the fallible models — the same pattern that lets a small model punch above its weight,
lifted to the protocol level. We wrote this synthesis into the CRPC sketch and credited Tim properly.
**Designed / sketched** (the load-bearing open piece is proving the fuzzy layer can never corrupt the
hard rules).

### 5. Starting to make the project un-erasable and un-capturable
You named the real deadline — not money, but the risk that someone cracks down on the project. Two
moves: we triggered a **permanent public archive** of the whole repo (so a takedown can't erase it),
and we recorded a new standing priority — **run JARVIS on free-tier models** as the defense against
corporate capture (grounded in last night's billing mess and the paper proving free-tier is viable, not
a downgrade). One thing left for you: add a second mirror of the repo somewhere besides GitHub.

---

## Honest status

| Thing | Status |
|---|---|
| Honesty pre-commit gate | ✅ built |
| v(S) swappable oracle + honest v0 | ✅ built |
| Equivocation slashing on the live path | ✅ built (persistent slashing waits on the network build) |
| Where-the-AI-lives (CRPC + node-oracles) | 🔬 designed/sketched — non-interference proof is the open safety piece |
| Constitutional dimension-set amendments | ⛔ blocked — needs a dimension matrix that doesn't exist yet, and its coherence is partner-deferred |
| Learned v(S) (the real moat) | 🔬 open — data-gated, not effort-gated |
| T1 slice-1 (persistence + wire codec) | ✅ built — restart from disk → byte-identical state |
| T1 slice-2 (transport) + slice-3 (gossip) | ✅ built — framed TCP + dedup broadcast |
| T1 slice-4 (sync = the join) + slice-5 (demo) | next |

## What's next

- **T1 slices 1-3 shipped** (persistence+codec `621c2d3`, transport `55bd8be`, gossip `9aa02e0`): a
  node restarts from its on-disk block log to byte-identical state; two peers exchange framed messages
  over TCP; gossip broadcast + dedup makes a mesh flood terminate. **Next is slice-4 (sync = the join)**:
  a fresh node pulls a peer's block log, decodes + replays, and converges — then slice-5 is the
  two-node local-join demo wired into noesisd.
- The dimension-set amendment surface is genuinely blocked; don't spend effort there yet.
- The learned v(S) is still the moat, and it's waiting on real data, not more code.

**Commits today:** `815cbdf`, `acbfcca`, `ddec2ce` (honesty gate), `6fe4552` (oracle seam), `536d6e2`
(equivocation), `7181969` (CRPC synthesis) — all pushed. Plus a permanent Software Heritage archive
request accepted.
