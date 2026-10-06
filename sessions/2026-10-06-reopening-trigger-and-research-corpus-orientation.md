# Session recap — 2026-10-06 — re-opening trigger + research-corpus orientation

Plain-English recap of a ~3h Noesis session (the kind that's easy to lose track of day to day).

## The through-line

It started with a conversation in the Pragma/OPH group. Jay gave our consensus-vs-value seam a better
name — the **admission invariant** (the rule for what's allowed to cross from the fluid value layer into
the irreversible ledger). Bernhard turned it toward brains: a world model that won't update (flat-earth)
is the same problem, and asked what should *trigger* re-evaluating a settled belief. That turned out to
be a real, well-posed Noesis design question, and most of the session grew from it.

## What we figured out about the re-opening trigger

The machinery to *re-do* standing once something is found fraudulent already exists in design (the
tombstone-mask + counterfactual clawback cascade). What was missing is the **trigger**: what admits a
finalized contribution to being re-opened — specifically the wash case where someone re-encodes the same
work under different bytes. The old CLAWBACK doc had explicitly left that case inadmissible.

Will's steer was the good part: take the shape from **biology** (nature already solves "commit hard, but
keep a bounded way to re-open a wrong commitment"), but close biology's own gameability with **game
theory**. That produced a clean two-tier answer:

- **Tier 1** — a fast, deterministic interior check (the relabel-invariance residual). It's a real
  trigger but *incomplete* — we confirmed from two of our own papers that interior measures are
  worth-blind to a well-made wash.
- **Tier 2** — the boundary (did a capital-independent mind actually build on it). This is the complete
  signal but it leans on the capital-cluster oracle that's still unbuilt.

The crisp line for Bernhard: "what triggers system-2" is **economic, not sensory** — a bonded challenge,
not a perfect detector. Written up as `docs/DESIGN-reopening-trigger-bio-priors-ungameable-game.md`
(design only; build stays cold/gated).

## What we did to the research folder

Read all 22 docs in `docs/research/` and found it's an unusually disciplined corpus: one kernel (value =
novel realized flow along provenance), one meta-law (role separation), and near-perfect agreement on
*where the single open problem sits* (the wash). Its main debts were cosmetic — drifted line-number
citations and some four-way-redundant positioning.

Shipped three new docs + a cleanup:
- **`how-to-read-noesis.md`** — a newcomer's orientation to how the project *thinks* (one bet; the
  built/designed/open status tags; the named hard problem; "real but uncomputable"). It's the "start
  here."
- **`META-ANALYSIS.md`** — the cross-analysis, with the centerpiece argument that the corpus's greatest
  asset is that it *predicts where its own hard problem sits* (the Lakatos "progressive program" idea,
  made concrete).
- **`INDEX.md`** — a map of all 22 docs.
- **Anchor re-pin** — fixed ~20 stale file:line citations (role-separation's core constants, the boundary
  independence-gate anchors, learned-value-seed's value-layer anchors) against current code.

## Honest state / what's left

- All of the above is **committed and pushed** to the public repo (HEAD 2d3c5849).
- The re-opening design is **design-only**, build gated.
- One small bit of drift remains un-fixed: the example-sim line anchors in the boundary paper
  (periphery_sim / peer_prediction_sim / wash_sim) — flagged, not re-verified.
- Two stale handoff claims got caught and corrected this session: the 2026-10-05 finality-fence work is
  actually committed+public (not local), and it partially discharges the non-interference item several
  papers list as "open."

## Not Noesis but same session (for continuity)
The Pragma reply + whiteboard pre-read are drafted on Will's Desktop but **not sent** — that's the live
partner thread to pick up.
