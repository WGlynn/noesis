# PATENT FIGURE SPECIFICATIONS (PRIVATE)

> Specs for the formal drawings. Render as **black-and-white line art only** — no shading,
> greyscale, colour, or photographs (UKIPO/EPO/USPTO requirement). Each element is labelled
> with a reference numeral; the same numerals must be used in the description text when the
> formal specification is filed (Path B). Number sheets and label each figure "Fig. 1", etc.
> Five figures: 1 node/system architecture, 2 ledger data schema, 3 two-unit mint state
> transition, 4 temporal-novelty valuation flowchart, 5 eclipse-resistant finalisation flowchart.
> Numbering scheme: figure N uses reference signs in the N00 range.

---

## Fig. 1 — Distributed ledger system architecture (block diagram)

Boxes connected by plain lines; a central network cloud with node boxes around it; the ledger
state and the functional modules drawn as labelled rectangles.

- **100** distributed ledger system (outer frame)
- **102** contributor/validator node (draw three, 102a/102b/102c, identical boxes)
- **104** peer-to-peer network (central element joining the nodes)
- **106** cell store (ledger state), holding:
  - **108** capacity cells (transferable)
  - **110** standing cells (non-transferable)
- **112** type-script engine (evaluates state-transition validity rules)
- **114** lock-script engine (evaluates ownership and transfer)
- **116** consensus module (weighted-vote finalisation)
- **118** commit-reveal authorship module
- **120** valuation module (temporal-novelty)
- **122** attribution module (graph-restricted cooperative-game value)

Lines: each node 102 connects to network 104; network 104 connects to cell store 106; modules
112–122 act on cell store 106 (draw as arrows into 106). Caption: "system overview."

---

## Fig. 2 — Cell and block data schema (schema/structure diagram)

Two stacked record boxes (a capacity cell and a standing cell) with named fields, plus a block
box. Fields drawn as sub-boxes within each record.

- **200** cell record, with fields:
  - **202** identifier
  - **204** lock script, containing **206** current holder identifier
  - **208** type script, containing **210** contributor identifier and **212** program hash
  - **214** parent reference (provenance edge)
  - **216** commit-order timestamp
  - **218** data payload
- **220** capacity cell (an instance of 200; transferable; holder 206 may change)
- **222** standing cell (an instance of 200; non-transferable; type script 208 enumerates the
  only permitted operations: **224** accrue, **226** decay, **228** slash, **230** burn)
- **232** block, containing a set of cells/commitments **234** and a finalised-state hash **236**

Caption: "cell and block structure; the standing cell 222 and capacity cell 220 differ only in
the transitions their type script 208 permits."

---

## Fig. 3 — Two-unit mint and non-transferability (state-transition diagram)

A start node, a mint operation that forks into two cells, and the allowed/rejected transitions
off the standing cell. Use rectangles for states, diamonds for the validity decision, arrows for
transitions; mark the rejected path with an "X" cross-out box (no colour, just line-art X).

- **300** verified contribution (start)
- **302** mint operation (forks to 304 and 308)
- **304** capacity cell (transferable) → **306** transfer transition (holder → new holder) PERMITTED
- **308** standing cell (non-transferable) → **310** state-transition validity rule (decision diamond)
- permitted successor transitions off 310: **312** accrue, **314** decay, **316** slash, **318** burn
- **320** rejected transition: reassign holder or contributor → validity rule returns FAILURE (X box)
- **322** consensus weight is read from the contributor identifier 210, never from the holder 206

Caption: "the standing cell can evolve (312–318) but cannot change hands (320); franchise tracks
contributor 210, not holder 206."

---

## Fig. 4 — Strategyproof temporal-novelty valuation (flowchart)

Vertical flowchart, rounded process boxes, diamond decisions.

- **400** receive commitment = hash(contribution ‖ secret) + signature + timestamp
- **402** reveal contribution (decision: valid reveal?)
- **404** no → **406** slash (forfeit bond, zero value)
- **408** yes → establish canonical order from the revealed commitments
- **410** compute coverage(contribution) = set of content elements (e.g. fixed-length shingles)
- **412** compute union of coverage of all EARLIER-ordered contributions
- **414** value = | coverage(contribution) minus earlier-union | (count of novel elements)
- **416** duplicate / subset / recombination → value = 0 (annotation on 414)
- **418** optional: value ← value × (1 + learned quality q), q in [0,1]
- **420** output contribution value

Caption: "value is novel coverage only; duplicates, subsets, and recombinations score zero (416);
quality (418) can only multiply a non-zero novelty."

---

## Fig. 5 — Eclipse-resistant weighted finalisation (flowchart)

Vertical flowchart with a branch for the hybrid basis computation.

- **500** validators submit votes on a proposal
- **502** for each validator: base weight = weighted sum over dimensions
- **504** retention factor r = f(time since last heartbeat), applied SYMMETRICALLY to all dimensions
- **506** effective weight = base weight × r (this is the franchise/vote weight)
- **508** staked balance is NOT reduced by r (side annotation on 506)
- **510** weight_for = sum of effective weight of supporting validators
- **512** effective_total = sum of effective weight of all validators
- **514** quorum_floor = fixed fraction × total base weight
- **516** finalisation basis = max( effective_total 512, quorum_floor 514 )
- **518** decision: weight_for ≥ supermajority fraction × basis 516 ?
- **520** yes → FINALISE
- **522** no, and weight_against > basis − threshold → EARLY-REJECT
- **524** else → continue collecting votes (loop to 500)
- **526** annotation: dimensions composed conjunctively; no single dimension below the supermajority
  finalises alone

Caption: "the hybrid basis 516 closes the liveness halt (via 512) and the eclipse (via the floor
514) at once; decay reduces the vote 506 but not the stake 508."

---

## Notes for the draughtsperson

- Line art only: solid black lines on white, uniform stroke, no fills, no greyscale, no colour, no photographs.
- Every numbered element gets its reference numeral placed beside it (leader line if needed).
- Keep numerals consistent with this list; the description text will reference the same numerals.
- One figure per sheet is cleanest; label "Fig. 1" … "Fig. 5"; number sheets.
- Plain sans-serif labels are fine; keep text minimal (numerals + short tags).
