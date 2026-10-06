# Resolving worth-blindness — you don't measure worth, you price lying about it

> **Status (read first): design directions, ready-for-critique, NOT built.** Everything below is 🟡
> designed / 🔬 open. Nothing in this doc is deployed or tested. It is the forward companion to
> `boundary-measurement-ungameable-game.md` (which states the problem honestly) and
> `something-from-nothing-oracle-free-content-value.md` (the oracle-free stack). It records a design
> framework agreed 2026-10-06; adversarial gating of the open pieces has NOT been run. Do not cite any
> part as a result.

## 0.5 The spot where the design already says "this works" (stated flatly, no hedge)

One result is demonstrated, on real data, and is allowed to be stated without a caveat, because it earned
it:

> **Value vests only on independent use.** Given a signal of independence, this cleanly zeroes a closed
> wash ring and pays genuine work. Demonstrated on **111,689 real crates.io crates** over 1,940,612
> reverse-dependency edges: crates whose reuse is 100% same-owner (closed rings) vest **0.00**; crates
> with cross-owner reuse (genuine) vest **69.36**. Real closed rings exist in the wild and the gate
> zeroes them (owner 373951: 2,875 self-edges, 0 external → vests 0); genuine large projects vest high
> (owner 980: 7,023 external → high). Rule unchanged from the built gate *logic* (which is tested but not
> yet wired into consensus). Figures **verified 2026-10-06 against the cached result**
> (`data/crates/graph/periphery_grounding.json`); not re-run from the raw dump this pass (not local).

That is the floor to stand on. Its one dependency — "given a signal of independence" — is the entire open
frontier below, and it is a *forge-cost* question (make independence cost capital, not a free identity),
not a missing mechanism. Everything in §1–§4 is about hardening that one input. The result above does not
wait on any of it.

## 0. The question, and why it has no interior answer

"How do we solve worth-blindness?" — the fact that no graph-internal signal separates a genuine
collaboration from a competently-built acyclic wash (`wash_sim`, 0.0%). The honest answer is that it is
**provably unsolvable as posed.** `DESIGN-attestation-exogeneity.md` carries the class-level result:
worth cannot be built from endogenous data. Genuine and wash differ *only* in content-worth, and worth
is not a property of the graph — so every purely interior measure is worth-blind **by construction**, not
by insufficient cleverness. Stop searching the interior. It is a proven dead end.

The answerable question is narrower: **which exogenous signal do you import, and how do you make forging
it cost more than the wash earns?**

## 1. Worth is imported, never measured — and every source has a toll

Worth means "does this matter to minds other than its maker" — a fact about the world *outside* the
ledger (other minds freely choosing to use, build on, or pay for a contribution). There are exactly three
places to import it from, and each extracts a different, unavoidable tax:

| Exogenous coupling | What it reads | Unpayable-for-free toll | Characteristic failure |
|---|---|---|---|
| **Realized external use** (current bet) | did independent minds build on it | **time** + recursion | "independent" recurses one hop down; patient wash waits it out |
| **Market / stake** (Harberger, peer-prediction) | what someone risks money on | **capital** | price ≠ worth; the capital that games everything sets the price; collusion equilibria coexist |
| **Oracle** (LLM / human panel, CRPC sketch) | a judge reads it and rules | **authority** | capturable, centralizing, adversarially foolable; who judges the judge |

This is a trilemma. Worth is never free; each source taxes **time, capital, or authority.** Noesis bets
on the first (capital-rooted realized-use) and refuses the third (an oracle is a capturable authority).
That is the right instinct: capital-cost is the hardest to fake cheaply *and* does not centralize trust.

## 2. The four agreed design directions

### 2.1 Operationalize worth; abandon intrinsic worth
There is no god's-eye worth to measure. Define it: **worth ≡ costly, independent, external adoption.**
Once accepted, worth-blindness stops being a metrology failure — the graph genuinely contains no worth,
and that is *fine*, because worth was never supposed to live there. The directive that falls out: stop
auditing the interior, price the boundary.

### 2.2 The real unlock — multiple *uncorrelated* exogenous anchors, required to agree
Today everything touching the funded wash hangs off ONE anchor (capital-cluster independence), so it is a
single dependency chain, not depth (see `WASH-PROBLEM-INDEX.md` §2.5). The move: require a contribution
to clear **time** (realized use accrued over N intervals) **AND capital** (independent-cluster adoption)
**AND decorrelation** (adopters are not herding on a shared signal). An attacker must then pay *all three
uncorrelated tolls at once* — wait, fund genuinely independent capital, **and** manufacture real
diversity. This is **genuinely additive**, because the forgery costs do not share a factor. Contrast the
built interior stack, which composes multiplicatively over *worth-blind* factors (1×1×1 = 1 on the
acyclic wash): deep in count, flat in effect. Exogenous-anchor conjunction is the opposite — few factors,
but each with a real, independent cost.
- Status: 🔬 the **decorrelation anchor is the genuinely open piece** — capital-independence does not
  touch herding / shared-prior / semantic-copy (the A7 column, blank in §2.5). This is where the next
  real design cycle goes. The time and capital anchors exist in designed form (re-open trigger; Layer A).

### 2.3 Accept the time-toll: provisional finality + clawback
If you cannot know worth at mint, do not finalize worth at mint. Mint provisionally, let realized-use
accrue, claw back what turns out to be wash. The re-opening trigger
(`DESIGN-reopening-trigger-bio-priors-ungameable-game.md`) is exactly this, and it is **underweighted** —
it is the only mechanism that *accepts* "we can't tell yet" instead of pretending the interior can.
Patient wash beats time-filters only when finality is premature; make finality track worth-accrual and
patience stops being an attack.

### 2.4 Reframe v(S) as an amortizer, not a solution
The learned `v(S)` does **not** solve worth-blindness and cannot. It is a **predictor of the exogenous
oracle** — it learns to predict the expensive, late realized-use measurement from cheap features so you
need not wait the time-lock every time. It inherits the gameability of its labels *exactly*: that is why
it is ~0.60 on honest rich-feature labels (it predicts honest worth) and NULL on structural features (it
has no independent adversarial power). Treating `v(S)` as *the* answer is the trap; it is a latency
optimization layered on the real signal, never the real signal. It can only ever be as good, and as
forge-resistant, as the exogenous labels under it.

## 3. The deepest reframe: it is an incentive problem, not a measurement problem

You never needed to measure worth. You need to make **claiming false worth unprofitable.** This is the
"un-gameable *game*, not un-gameable *measure*" thesis taken to its conclusion. The resolution of
worth-blindness is: **do not measure worth at all; structure the game so the three exogenous tolls
(time, capital, decorrelation) are load-bearing, uncorrelated, and required jointly, with provisional
finality so late-arriving truth can claw back the wash.**

## 4. Honest unpaid tolls (what this does and does not buy)

- **Open:** the decorrelation anchor (§2.2) — the one blank column. Capital-independence does not close
  herding / shared-prior; this is the open frontier.
- **Not "un-gameable."** The ceiling is a **funded-majority attacker who manufactures genuine diversity**
  — the 51%-class residual, now *priced* across three uncorrelated axes rather than one. You buy "no free
  move, and every expensive move named," not impossibility.
- **Nothing here is built.** The time anchor (clawback) and capital anchor (Layer A) are 🟡 designed; the
  decorrelation anchor is 🔬 un-designed; the joint-conjunction mechanism is 🔬 un-designed. The next
  step is to adversarially gate §2.2 before any of it is trusted.

## 5. One line

Worth-blindness is not solved; it is dissolved — by giving up on measuring worth and instead pricing the
lie across time, capital, and decorrelation jointly, late-bound and clawback-able, so that the only
surviving wash is a funded majority that also manufactures real diversity, and even that pays full price.
