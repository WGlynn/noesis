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
| Draft 1.2 (legibility rev) | 2026-06-19 | SHIPPED (14pp) | Will read: "everything is great, it's just a ~200 IQ gate on understanding what's going on." Surgical 5% legibility pass — NOT a rewrite. Added a one-clause "in plain terms" gloss immediately after each of the three highest-wall formal objects: Myerson value (§5.3), HodgeRank decomposition (§5.6), finalization hybrid (Consensus §). Math, claims, and structure untouched; each object now states what it *means* the moment after what it *is*. Abstract / worked example / proofs deliberately left alone (already carry their weight). Version label stays 1.2 (no content/claim change). [[make-the-mechanism-legible-not-just-the-analogy]]. |
| Version 1.4 (privacy section) | 2026-06-19 | SHIPPED (16pp) | Will: "needs a privacy section for sure" + "homomorphic encryption and compute-to-data? ... do what is logical." New §"Privacy: what a value chain can and cannot hide" (p11). Stance: provenance-complete BY NECESSITY (PoM recomputable = anti-privacy); 3 layers — identity pseudonymous-not-anonymous (sybil + soulbound personhood-binding), content committable-without-publish (coverage = shingle hashes in SMT), value = the hard case. Hard case resolved by COMPUTE-TO-DATA (Will's own VibeSwap research §9.2 sweet-spot): evaluator model→content, content never leaves (CKB cell referenced read-only, never consumed), only score+proof on-chain; +ZK-proof-of-execution (honest run) +differential-privacy (bound score leak) +FHE frontier. + data-governance para. designed-not-built; gates consume commitment/proof/noised-score, never plaintext. Version 1.3→1.4. Economítra + C2D papers ABSORBED not cited (stealth; explicit-cite deferred to Will). |
| Draft 1.2 (Economítra opening) | 2026-06-19 | SHIPPED (15pp) | Will: old abstract opening "too abstract, doesn't draw people in." Reworked on the Economítra thesis (Will's own info-theoretic economics paper, `~/Desktop/Economitra/`): markets as Shannon channels, extraction = noise, "you optimize for what you measure." New abstract opener (tighter version, Will: "straight fire"): *"You optimize for what you measure, and a blockchain measures the wrong thing... the gap fills with extraction. This paper specifies the missing organ."* Lineage line's middle rung made explicit: "information as the work, value measured by the signal a contribution carries, not the energy it burns" (= Economítra, absorbed not cited; explicit-cite decision deferred to Will). Connection mapped (not all folded): Economítra Shannon-noise → Noesis HodgeRank/circulation detector; Shapley+time-neutrality → Myerson+temporal-novelty; commit-reveal + coop>extraction → consensus authorship + honesty-load-bearing. |
| Draft 1.2 (economic-frame fold-in) | 2026-06-19 | SHIPPED (14pp) | Will: "fold in" the Cooperative-Capitalism↔Noesis comparison. New section "The economic frame: cooperative capitalism, made consensus-native" before the Conclusion. Maps Will's Medium thesis (*Cooperative Capitalism is the last coherent economic path crypto has left*) onto the design: the four exhausted paths (libertarian/clearinghouse/commons/governance) → consensus defenses; the article's maxim "mutualize the risk layer, compete on the value layer / risk layer = where we're alike, value layer = where we differ" → the soulbound-franchise vs transferable-capacity split ("buy storage, not consensus"); and the article's own escape hatch ("if it fails, blame the substrate") → Noesis IS that substrate (value endogenous + consensus-securing vs redistributed atop a possession chain). Frames Noesis as cooperative capitalism moved from application-property to consensus-property. No claim/math change. |
| Draft 1.2 (legibility rev 2) | 2026-06-19 | SHIPPED (14pp) | Will: "dumb it down another 5% — the whole thing." Second legibility pass, spread ACROSS the paper (rev 1 hit only the 3 hardest formal objects). +6 one-clause "in plain terms" / "put simply" glosses: abstract landing sentence (pays-for-verified-non-duplicate-contribution / value-by-what-builds-on-it / earned-not-bought), Bradley-Terry (chess-rating/RLHF intuition), temporal novelty (paid only for what no earlier block covered), saturation (geometric ceiling, can't run away), seed gate (chain of 0-1 dials, can only turn payout down), PoM definition (summed verified value of work you own, unbuyable). Math/claims/structure still untouched; version stays 1.2. Cumulative legibility delta now ~10% over Draft 1.2 base. |
| Version 1.5 | 2026-06-19 | SHIPPED (`d4c886e`) | Readability pass: intro rewritten to a viral reading level (also: em-dashes removed from the abstract per Will, `1878c0e`). This is the version `HANDOFF-readability-v1.5.md` documented. |
| Version 2.2 | 2026-06-19 | SHIPPED (`b41fa70`) | Abstract reframed to lead with the two grounded big claims. |
| Version 2.4 | 2026-06-19 | SHIPPED (`7046ca9`) | Satoshi-voice reframe: lineage not diss; cryptoeconomics framed as medium-creation (mint what the market needs, the market follows). |
| Version 2.5 | 2026-06-19 | SHIPPED (`3d08a1f`) | De-narration pass + Will's Equilibrium design note + resume pointer. |
| Version 2.6 | 2026-06-19 | SHIPPED (`bcd49ee`) | Conceptual core sharpened to its thesis: "the chain IS the market for contribution." |
| Version 2.8 | 2026-06-19 | SHIPPED (`5b4062d`) | Unify-under-one-rule (novel realized downstream flow along provenance) + the named equilibrium + the honesty-propagation section + plain-up pass. |
| Version 2.9 | 2026-06-20 | SHIPPED (`228f2ac`) | Three Powers rewrite: cognition (PoM) leads; compute + capital are the value-inert liveness floor ("keep the lights on, cannot buy consensus"). |
| Version 3.0 | 2026-06-20 | SHIPPED (`a603d85`) | Plain-down pass on the densest sections (contents unchanged). |
| Version 3.2 | 2026-06-20 | SHIPPED (`c5d452e`) | PoW reframed honestly (anti-spam + energy-oracle money, not waste) + the endogenous-value point, across the whitepaper and the accessible tier. |
| Version 3.3 | 2026-06-20 | SHIPPED (`e5cd58b`) | Updated to locked decisions + 7x "vine-prune" (converged, redundancy cut). |
| Version 3.4 | 2026-06-20 | SHIPPED (`edf094f`) | Abstract reframe: lead with the thesis, drop the Bitcoin-grading opener. |
| Version 3.5 | 2026-06-20 | SHIPPED (`d670129`) | Abstract capstone: "the explanation is the contribution" (any one property is easy alone; the paper is how they hold together). |
| Version 3.6 | 2026-06-20 | SHIPPED (`206bb49`) | Doubled the references (15 -> 33), each tied to an existing claim (positioned, not pretended-invented). |
| Version 3.7 | 2026-06-20 | SHIPPED (`934e890`) | Grounded endogenous value in marginal value theory (the 1870s marginal revolution; value at the margin, revealed by use). |
| Version 3.8 | 2026-06-20 | SHIPPED (`6843dd8`) | Bibliography corrections (web-verified, 3 fixes). |
| Version 3.9 | 2026-06-20 | SHIPPED (`a76334c`) | Cut the redundant economic-frame section (former §15); kept value-at-margin. |
| Version 4.0 | 2026-06-20 | SHIPPED (`42c304c`) | Eliminated the named conjecture (the "Glynn equilibrium") in favor of the honest labeled-conjecture framing (the Honest-Contribution Equilibrium, demonstrated core marked, open theorems named). |
| Version 5.0 | 2026-06-21 | SHIPPED (`dfb8fc1`) | 17 -> 13pp trim + a coherence / fact / tokenomics audit (16 fixes). Audit report: `~/Desktop/noesis-whitepaper-audit-2026-06-21.md`. |
| Version 5.2 | 2026-06-23 | SHIPPED (`f14f156`) | Honesty pass + thesis consolidation: 6 overclaims fixed + 2 more self-caught; the moat-NULL result propagated; the STATUS-LEDGER made the single authority for demonstrated-vs-designed. The DeepFunding moat experiment returned NULL (learned v(S) did not beat a fixed proxy on real jury labels), logged as unsupported-not-refuted under the reflexive-provenance rule. Internal thesis cluster added (non-zero-sum paradigm capstone, claimable-attribution adoption engine, moat-stack). |
| Version 5.3 | 2026-07-10 | SHIPPED (15pp, `a30c932`) | Two strengthenings, no new design (sources absorbed, not cited). (1) Differential-incompleteness woven into "Measurement as a living mechanism" as a "Detecting and completing, not averaging" passage: a persistent residual is a diagnostic value dispute pointing at a missing dimension; resolution is completion not compromise; this is the exact sense in which the measure is objective (whether a basis omits a dimension its participants track is a fact of the matter). Grounds the load-bearing word "objective." (2) Patent E/A invariant into Cryptoeconomics: consensus authority is produced/preserved by protocol, independent of transferable ownership, across every valid state transition, so "cannot buy consensus" is a consequence not a policy. PAPERS.md synced to v5.3. |

> **Reconstruction note.** Rows 1.5 through 5.3 were reconstructed 2026-07-10 from the authoritative git history (`git log --follow` on `noesis-whitepaper.tex`); the commit messages carry the what / when / why and each row cites its commit hash. The 1.0-1.4 rows above keep their live-capture detail (reader reactions, in-flight reasoning); the reconstructed rows are authoritative on change-content but thinner on live reaction, since that lived in-session, not in git. Going forward the log is maintained per version bump again. (Skipped version numbers, e.g. 2.3 / 2.7 / 3.1 / 5.1, were passed over during iteration and never shipped as distinct `\date` labels.)

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
