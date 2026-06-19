# Whitepaper Evolution Log — Proof of Mind

> The foundational whitepaper (`noesis-whitepaper.tex`) is a LIVING document. This log is its
> version-evolution loop: every reader reaction is captured verbatim, the signal is extracted, and
> it becomes a dated action item that drives the next version. `complete = ready-for-critique`
> ([[complete-as-ready-for-critique]]); reactions are the methodology working, not failing.
> Loop discipline: [[code-text-inspiration-loop]] · [[recursive-trp-rsi-on-docs]]. PRIVATE.

## Versions

| Version | Date | State | Notes |
|---|---|---|---|
| Draft 1.0 | 2026-06-19 | SHIPPED (`10402c4`) | First foundational scientific WP. 9pp PDF, Bitcoin cadence, equations + 4 figures (2 from real node data). For Will's dad + first external reads. |
| Draft 1.1 | 2026-06-19 | SHIPPED (12pp) | All 3 increments in: (1) "Measurement as a living mechanism" (answers FB-001) ✓; (2) standalone "Related work and what is novel" §9 written from RI-001+RI-002 — (a) NOVEL, Bittensor precise, (b) NOVEL vs Deep Funding (two-axis differentiator), (d) NOVEL-as-consensus-franchise vs SourceCred/DeSoc, (e)+decentralized-training hedged honestly ✓; (3) convergence/forwards-compatibility section ✓. Desktop PDF refreshed. |
| Draft 1.2 | 2026-06-19 | SHIPPED (14pp) | RI-003 (3rd pass) found real pre-emptions; §9 corrected defend-and-narrow: (e) cite DPoR (2019) for the Hodge-slashing construct, narrow to learned-model+endogenous-value pairing; (a) acknowledge Bittensor-Yuma + Fortytwo reward output-quality but via subjective peer-agreement (training family survives), differentiate on objective cooperative-game measurement as the finality object. +3 refs. Also: worked example, threat-model, notation, red-team hardening, airgap spine, synced abstract, label 1.1->1.2. |

---

## Research inputs

### RI-001 — Novelty audit vs the useful-PoW / contribution-consensus lineage — 2026-06-19
Adversarial deep-research (105 agents, 23 sources, 25 claims 3-vote-verified). Full report:
`docs/research/RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md`. Headline: claim (a) endogenous value as the
consensus object is **NOVEL** vs the whole corpus; **Bittensor** is the one dangerous competitor and
must be stated precisely (Yuma *attempts* contribution-scoring but is subjective stake-weighted opinion,
stake-dominated reward r≈0.5–0.95 — NOT "no attempt"). **Honest gap that gates the WP:** (b) Myerson-on-DAG,
(d) soulbound-standing, (e) v(S)+HodgeRank are novel *within the surveyed corpus* but were NOT cleared
against their natural families (EF Deep Funding, Data-Shapley, Myerson, SBT lit, HodgeRank lit,
decentralized-training: Gensyn/Prime Intellect). **A second research pass on those families is required
before Draft 1.1 asserts (b)/(d)/(e) novel.** Highest-risk open collision: does Deep Funding's
pairwise-distilled judgment over a dependency graph subsume (b)?

**Status:** report persisted; Draft 1.1 §9 rewrite + the 2nd-pass research are the open actions.

### RI-002 — Second-pass novelty audit (cleared the flagged claims) — 2026-06-19
Deep-research pass 2 (105 agents, 24/25 verified). Detail appended to
`docs/research/RELATED-WORK-NOVELTY-AUDIT-2026-06-19.md` (SECOND PASS section). **(b) NOVEL** — Deep
Funding decisively differentiated (scalar edge-weight funding oracle, not graph-restricted-Shapley
consensus object); **(d) NOVEL as consensus-franchise** (SourceCred/DeSoc have the split pattern, not the
consensus-weight). **(e) + (a)-vs-decentralized-training still OPEN** → Draft 1.2 third pass. §9 written
with honest hedges on the open two. **Status:** Draft 1.1 §9 complete; third pass is the next research action.

---

## Feedback entries

### FB-001 — CKBased.bit (CKB community) — 2026-06-19 ~09:43 CT
**Context:** First external read of Draft 1.0, shared by Will. Reader is a CKB-native (`.bit` name) — squarely the target audience.

**Reaction (verbatim, reconstructed from the live paste):**
- At the abstract line *"...we treat it as the load-bearing open problem rather than a solved one."* →
  *"you cheeky cunt 😂"*
- *"[you leav]e out the most interesting part"*
- *"[b]est approach I guess, not really something you can solve with a static solution"*

**Signal extracted:**
1. **The honest framing landed and worked as designed.** The "open problem, not solved" line pulled a
   sophisticated reader straight to the crux. This validates the honest demonstrated-vs-designed
   register (§9) and `complete = ready-for-critique` — the abstract did its job by being honest about
   the hard part.
2. **Gap (the actionable hit): the paper UNDER-TELLS the dynamic answer.** Draft 1.0 bounds the
   value-measurement discussion to prove it is *safe* (learned `v(S)` can only deny value never mint;
   HodgeRank residual; held-out chart) but does not expound the *solution* to un-gameable measurement.
   We showed the cage is sound; we did not show the living mechanism inside it. That mechanism is "the
   most interesting part" the reader noticed was missing.
3. **Convergent insight — the reader articulated our own thesis.** *"Not something you can solve with a
   static solution"* IS the design rationale: a static value rule is Goodhart-bait (any published fixed
   formula is gamed on publication); the only un-gameable measure is one that *adapts to the gaming*.
   Our actual answer is a control loop, not a function — learned `v(S)` retrained on realized outcomes
   (backwards-enforcement §8) + dynamic dispute/slashing adjudication + decay (no banking a stale score)
   + HodgeRank residual as a live break-detector + a verifier-gated, governance-mutable value-dimension
   matrix. The reader intuited the adaptive answer from its *absence*.

**Verdict:** Not a flaw found — a door to the sequel opened. Strong signal (reader engaged at depth and
converged on our design rationale unprompted).

**Action items → Draft 1.1:**
- [ ] New section **"Measurement as a living mechanism"**: state the dynamic/control-loop answer
      explicitly (retraining loop · dynamic dispute · decay · HodgeRank break-detector · mutable-but-
      verifier-gated value matrix). Frame the Goodhart argument: static rule ⇒ gamed; adaptive measure
      ⇒ the only un-gameable kind.
- [ ] Cross-reference the running deep-research dive (useful-PoW / proof-of-contribution lineage) — how
      Bittensor (Yuma) and the PoUW family handle, or fail to handle, the same dynamic-measurement
      problem. Sharpens both this section and Related Work (§9).

**Relationship action (Will-gated, Will delivers per [[jarvis-prep-not-delivery-for-partner-chat]]):**
prep reply talking points — lead with "you found the door to the sequel: it's a control loop, here's
the shape"; pull CKBased.bit in as a recurring critic. A CKB-native reader engaging this deeply is
worth keeping close.

**Status:** OPEN → queued for Draft 1.1.
