# HANDOFF — v1.5 viral-readability rewrite (resume here)

**Goal (Will, 2026-06-19):** the Noesis whitepaper must be able to **go viral**, so its prose needs the
reading level of the **Bitcoin original whitepaper** — short, plain, declarative sentences, one idea
each. BUT keep the **brilliant math and technical specs fully intact** — Will: *"i don't want to dim the
brilliant math... i like the technical specs, the internal value."* The rigor IS the value; only the
connective prose changes.

## North-star voice (already approved)
The abstract opener and the rewritten Introduction §1 are the exemplar. Match them exactly:
- BEFORE: "A miner proves it expended energy; the network agrees on a transaction ordering; the coin's worth is set off-chain by a market."
- AFTER: "A miner proves it burned energy. The network agrees on an order for the transactions. What the coin is worth is decided somewhere else, by a market the chain never sees."

## State
- Version **1.5**, committed + pushed: `317f7a3` (origin/master in sync). Intro §1 done; margin 0.8in.
- Current length: **14pp**. Target: **≤12pp** (comes from the prose tightening, NOT typography).
- Build: `bash ~/noesis/docs/whitepaper/build-wp.sh` (stamps a dated Desktop PDF; never same-name overwrite).

## THE RULES (do / don't)
DO: rewrite connective prose into short plain Bitcoin-WP sentences; cut hedge-stacking and Latinate
filler; merge any "in plain terms: X" gloss into one plain sentence; prefer plain words.
DON'T: change ANY equation, number, statistic (e.g. `$285$`, `1.06\times`, `$\ge 0.9$`, `$\varphi^2$`,
`2.618`, `0.26`, `17.66`), figure/tikz block, section/paragraph heading, the Notation table, references,
or any "demonstrated vs designed" honesty caveat. No typography/preamble changes. Keep all the math.

## Sections still to rewrite (intro = DONE/exemplar)
Blocks & provenance · Ownership · Endogenous value (+ subsections: elicitation, additivity trap,
Myerson, strategyproofness, saturation, worked example, HodgeRank, learned evaluator) · Measurement as a
living mechanism · Proof of Mind (+ ToM) · Consensus (+ 3 subsections) · Cryptoeconomics ·
Backwards-enforcement · Security & honest limitations · Privacy · Fair launch · Forwards-compatibility ·
Related work · Economic frame · Conclusion.

## Process per pass
1. Rewrite prose section-by-section (keep math verbatim).
2. `bash build-wp.sh`; check `pdfinfo` Pages ≤ 12.
3. `git diff` review: confirm NO number/equation/claim/caveat changed (this is the safety gate).
4. Commit + push (verify HEAD advanced + origin == HEAD per [[verify-commit-landed-never-trust-background-exit]]; the pre-commit runs the full test suite, ~70s, so the commit backgrounds — wait for it).

## Open decisions (Will)
- Cite Economítra (`~/Desktop/Economitra/ECONOMITRA_V1.2.md`) + the compute-to-data paper
  (`~/JARVIS/papers/data-marketplace-compute-to-data.md`) explicitly in the WP, or keep absorbed (stealth)? Currently absorbed.

## Full session context
`~/.claude/projects/C--Users-Will/memory/project_session-2026-06-19-noesis-marathon.md` (UPDATE ~21:10 block).
