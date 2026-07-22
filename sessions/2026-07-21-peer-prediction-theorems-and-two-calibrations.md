# Session recap — 2026-07-21 — peer-prediction theorems, and the discipline of catching my own overclaims

Plain-English recap of a long Noesis session. The through-line: I worked the two open
peer-prediction theorems, and twice built an elegant-but-overstated claim that an adversarial
calibration caught and forced me to downgrade. The honesty machinery worked on its own author.

## What we set out to do

You picked "#2 — the Harberger theorems" from a menu of four next moves (the other three — wiring
the vest gate into consensus, retraining the value model, and deploying the testnet — are all gated on
your input or your fly.io card). #2 is the deploy-independent research one: the two open theorems for
using peer-prediction (the network scoring each other's contributions, with no privileged judge) as the
oracle-free content signal in the value layer.

## What actually happened, in order

1. **Pinned the earlier periphery result** into the test suite so it can't silently regress
   (`discernment.rs`), then wrote the two theorems up honestly (`DESIGN-harberger-peer-prediction-theorems.md`):
   - **T1 (graph-generalization):** peer-prediction needs peers whose signals are independent given the
     contribution's true worth. On our graph the thing that breaks that is a shared controller (a wash
     ring). So peer-prediction works on the *capital-independent* part of the graph.
   - **T2 (uniqueness):** I had to strike it — peer-prediction alone *always* has a lazy "everyone says
     nothing" equilibrium (a known impossibility). What's true is that the Harberger stake + dispute
     market kill that lazy equilibrium, so honesty becomes the only *surviving* one.

2. **Built a numeric sim** (`peer_prediction_sim.rs`) so the theorems are backed by running numbers, not
   just prose. Caught a real bug by running it (a mislabeled EV sign).

3. **Ran an adversarial calibration on the load-bearing T1 claim** (10 independent agents told to refute,
   not confirm). It caught my elegant line — "the two mechanisms need the same condition, so the protocol
   buys it once" — as an **overclaim**: capital-independence is *necessary* but *not sufficient*. A shared
   public prior, herding, semantic copying, or a third party's sybils all break the independence
   peer-prediction needs, and my gate touches none of them. Downgraded the doc + sim.

4. **Advanced the sharpened residual** with another sim section: "detail-free" scoring (subtracting the
   cross-task baseline) *closes* a task-constant common bias but *not* a task-specific external
   correlation (herding/copying), which beats genuine work past ~70% coordination. So the open problem is
   now named and measured, not a vague list.

5. **Tried to close it with a backstop** ("require the vesting gate too, and cheap coordination earns
   zero") — and **ran a second calibration on that**. It caught three things: (a) the conjunctive value
   function I assumed is **not built** — the shipped value layer gates on accrued *standing*, not on my
   capital-independence gate, which has zero callers; (b) a **semi-funded ring** renting one outside
   identity beats it cheaply; (c) I conflated a per-identity cost with the network-51% cost. Downgraded
   again.

## The real reconciliation (the useful part)

Reading the actual code (not my memory of it) showed the punchline: **peer-prediction is not a new
mechanism I need to add — it's a candidate for the value layer's existing v8 "outcome factor"**, an
oracle-free content floor that already multiplies into the score and can only ever lower it. The built
anti-coordination defense is already there too: the *standing*-vest gate ("an unvested identity certifies
nothing"). My careful parallel derivation had quietly rebuilt a composition the code already ships,
under different names. The genuinely-open residual is the built system's own pinned gap — a *vested*
identity certifying garbage — which only the learned value-model-on-real-labels closes (the known
crown-jewel-open).

## Honest status (no rounding up)

- **Built + tested:** the value layer (v5–v8, multiplicative floors, standing-vest gate), the periphery
  vesting gate as a pure function, the discernment pins.
- **Designed, not built:** the peer-prediction wrapper, the conjunctive vest composition, the Harberger
  self-assessed price.
- **Open:** the task-specific-correlation residual; the vested-certifier-garbage gap; the learned v(S) on
  real realized-use labels (still the crown jewel, still null on synthetic labels).

## Commits (all pushed, HEAD `3462a30`)

`3402ebf` periphery pin · `5a0fa57` theorems + first calibration + CI-2 downgrade · `5650740` detail-free
CA residual partition · `6fc2884` backstop hypothesis · `3462a30` backstop calibrated + downgraded.

## The meta-lesson worth keeping

Twice this session the *prettiest* version of a claim was the wrong one, and the tell was the elegance —
"same set, bought once", "the two residuals compose perfectly". Both are the shape "X's weakness is
exactly Y's strength", which is seductive and usually false in the details. The calibration workflow (spawn
refuters, keep what survives) caught both before anything was built on them. And execute-to-verify (read
the running code, not my model of it) caught that I'd been re-deriving shipped mechanisms. Those two
disciplines did more for correctness this session than the cleverness did.

## Still on your plate (the gated unlocks)

- **#1** wire the vesting/value composition into consensus — needs your explicit "we're wiring consensus" go.
- **#4** permanent testnet deploy — one blocker, your fly.io card.
- **#3** retrain v(S) on real labels — data-gated on #1 shipping.
