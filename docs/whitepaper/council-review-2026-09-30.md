# Network whitepaper - adversarial council review (2026-09-30)

> Five-seat adversarial persona council run on the Section-1-through-16 draft of
> `network-whitepaper-draft.md`. Verdict and prioritized revision map below; this file is the
> provenance record for the revision the current draft implements. Seats: skeptical crypto-economics
> referee, cynical mechanism-design skeptic (code-grounded), QF/RetroPGF practitioner, mechanism-design
> economist, adversarial whitepaper editor.

## Verdict: needs-major-revision

Four of five seats converged on the same two structural defects:

1. **The teleological spine** ("logical conclusion / completion of the sequence / arguably forced") is
   unfalsifiable and retrofits intent onto QF/RetroPGF/Deep Funding, which were not designed as steps
   toward an attribution ledger. Worse, it contradicts the paper's own honest-floor discipline, which is
   its explicit credibility asset.
2. **The flagship novelty claim** ("attribution IS the consensus object") survived only via an unshown
   "surveyed corpus" hedge that omitted the nearest competitors: Bittensor/Yuma (consensus object already
   IS a weighted attribution), SourceCred, Colony, Coordinape, EigenTrust, and proof-of-useful-work.

The deepest single reframing (one seat, adopted into the synthesis): the paper never names
**accounting-versus-value**. Given the measured 0% graph-internal wash separation, the built L0 is a
tamper-proof *accounting* ledger, not a ledger of *value*. The honesty was real and code-matched but
diffused across nine "honest floor" asides, so no single reader assembled the full discount.

## Prioritized revisions and disposition

| # | Priority | Revision | Applied in this draft |
|---|---|---|---|
| 1 | must-fix | Demote teleology; falsifiable claim + defeat conditions; name rejected alternatives; "structural bet" framing throughout | Yes - new 1.1 (claims/not-claims + defeat conditions), 7 retitled "one design that closes the gap" with alternatives named, 8 "preferred not forced" with optimistic/off-chain alternatives, 16 bet framing |
| 2 | must-fix | Rescue novelty claim by naming nearest competitors + precise delta each | Yes - 14 adds Bittensor/Yuma, SourceCred, Colony, Coordinape, EigenTrust, PoUW; claim downgraded to "not aware of... invite counterexamples" |
| 3 | must-fix | Consolidated built-vs-bet / accounting-vs-value artifact up front with the 0% number + comparison table | Yes - new 1.2 + comparison table |
| 4 | must-fix | Fix QF: F=(sum sqrt c)^2, match = F - sum(c); condition welfare claim in same sentence; verify Pasquini citation | Yes - 4 rewritten; Pasquini flagged for verification in references |
| 5 | should-fix | Stop rounding "strategyproof"; state Sybil/padding-resistant-under-commit-order, NOT wash-resistant; cite Broder 1997 | Yes - 13 + 9 |
| 6 | should-fix | Qualify "bounded evaluator harmless to safety": factor-vs-seed, suppression attack, depth-axis pump | Yes - 13 |
| 7 | should-fix | Reconcile "attribution IS consensus object" with FINALITY_MIX + capital floor; formalize/rename non-interference | Yes - 12 ("first-class larger-weight input conjoined with capital floor"; property restated as "no learned output reaches finality"; Goguen-Meseguer term-collision noted) |
| 8 | should-fix | Bounded-residual sentence after 12's caveat stack (gaming-OR-capital -> gaming-AND-capital, 51%-class); reconcile 9 "tested" vs 12 "unproved" | Yes - 12 + 9 |
| 9 | should-fix | Situate oracle/elicitation machinery (SchellingCoin/Chainlink/Augur/Blum; Bradley-Terry/Elo/Kemeny/HodgeRank/RLHF); scope zkML to execution-faithfulness, drop "endgame" | Yes - 10, 11 |
| 10 | should-fix | Rewrite 7.1: drop "non-rivalrous by construction" + the ledger-self-reference; normative not descriptive | Yes - 7.1 retitled "Predecessors as upstream contributions, not rivals (a positioning stance)" |
| 11 | should-fix | Update RetroPGF (5) and Deep Funding (6) to current state; pin Deep Funding to 2025 round | Yes - 5, 6 |
| 12 | nice-to-have | State structural restriction for closed-form Shapley (#P-hard general, Deng-Papadimitriou); relabel Hodge as inconsistency decomposition | Yes - 9 |
| 13 | nice-to-have | Concrete wash worked example + plain-English glosses of what each L0 primitive prevents | Yes - 15 worked example; 9 glosses |
| 14 | nice-to-have | Earn the signature line as a conclusion of 2; lead 11 with the PoW analogy | Yes - 2, 11 |

## Carried as honest-floor, not "fixed"

Per status discipline, the following were made *more visible*, not resolved: the finality-safety
property is defined but not proved (the gate before L1); L1 is a design sketch; the learned value on real
labels is data-gated; the depth-axis lineage-laundering gap is open; the exogeneity terminus is a
boundary, not a bug.

## Not asserted into the public draft (anti-hallucination hold)

The council cited specific code line numbers (runtime.rs:726 FINALITY_MIX, :738 MIN_DIM_BPS=5000) and a
specific pump figure (v8 +16.7). These were NOT written into the public prose as figures. The finality
mix (PoM larger weight ~2/3, PoS ~1/3) and the 50%-per-dimension floor are stated qualitatively, matching
canonical architecture notes; the depth-axis pump is stated qualitatively ("seed pumpable via lineage
laundering") without a figure. Re-verify against `node/src/runtime.rs` and `ISOMORPHISM-INVARIANCE-VS.md`
before any figure is published.
