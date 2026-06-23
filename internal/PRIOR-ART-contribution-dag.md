# Prior art: the contribution DAG, and how Noesis differentiates

> Related-work + positioning (feeds the whitepaper's related-work section and launch frame-defense).
> Honest discipline ([F·noesis-is-the-ultimate-blockchain] + AMD): name what we INHERIT vs what is
> NOVEL. We augment proven mechanisms; the novelty is the composition + the method inversions, not a
> claim to have invented value-flow or Hodge theory. PRIVATE.

## The space: "value flows along a DAG of what built on what"
Noesis is not the first to model contribution as a graph and flow value along it. The honest move is to
locate exactly where the lineage ends and the new work begins. Four prior-art clusters:

### 1. Graph value-flow / ranking (the math lineage)
- **PageRank / EigenTrust** — eigenvector centrality as value flow; importance accrues to nodes
  connected to important nodes. Noesis's `flow` module (eigenvector value-flow with damping) is this
  lineage, explicitly.
- **HodgeRank — Jiang, Lim, Yao, Ye, *Statistical ranking and combinatorial Hodge theory* (Math.
  Programming 2011, arXiv:0811.1067).** Decomposes a pairwise edge-flow into gradient (the ℓ²-optimal
  global ranking) ⊕ curl (locally cyclic) ⊕ harmonic (globally cyclic). Their PURPOSE: diagnose whether
  a consistent global ranking *exists* — a large cyclic component means "no good ranking, the data is
  intrinsically inconsistent." This is the exact math behind our `attribution_cycle_energy` (cc) +
  `collusion_residual_by_identity` (dd/ee).
- **Shapley / Myerson values; Shapley Flow (Wang et al.)** — axiomatic credit allocation (4 fairness
  axioms, sums exactly, no double-count); Shapley Flow allocates credit to graph EDGES across the causal
  graph, not just nodes. Radicchi et al. 2009 applied credit-diffusion to citation networks. Noesis's
  intra-block Myerson + pairwise→Shapley is this lineage.

### 2. Crypto contribution-funding (the application lineage)
- **SourceCred / CredRank** — THE closest prior system: a contribution graph (issues, commits, files,
  people as nodes) + a modified PageRank ("cred" flows along bidirectionally-weighted edges, no
  teleportation), payouts ("grain") from the scores.
- **Gitcoin Quadratic Funding** — prospective, crowd-driven, amplifies many-small-donors; sybil-prone.
- **Optimism RetroPGF** — retroactive: an expert *badgeholder* panel judges demonstrated impact ("easier
  to agree on what WAS useful than what will be"); evolving toward metrics; gaming-resistance is an
  acknowledged open problem.

### 3. Provenance / attestation
- Git DAG, W3C PROV, Ethereum Attestation Service — provenance recorded, but value/credit is not flowed
  or secured on it.

## How Noesis differentiates

### Axis 0 — COMPOSITION (the obvious one)
No prior system unifies **measurement + consensus + sybil-economics + a money layer + backwards-
enforcement** into one chain. The prior art each does ONE piece:
- SourceCred *measures* contribution — but it is a scoring/payout tool, not a chain; no consensus, no
  soulbound franchise, no money.
- RetroPGF *funds* contribution — but via a human jury, not a structural measure.
- HodgeRank / Shapley are *math* — no system, no economics, no adversary.
- PageRank *ranks* — and is gameable by construction (link-farming = cred-farming).

Noesis ties per-block ownership + pairwise→Shapley value + temporal-novelty + realized-flow + PoM
consensus + commit-reveal provenance + an Ergon money layer into a single object where **the measure IS
the consensus weight.** This is the "krabby patty": the coherent tying-together, not any single part.
*True, and the most legible differentiator — but the obvious one. The deeper differentiators are
method-level, and they survive even if a competitor copies the composition.*

### Axis 1 — MEASUREMENT IS THE ATTACK SURFACE (method)
Every prior system treats the value-measure as **given and trusted**: SourceCred trusts community-set
weights ("when the algorithm and the community disagree, the community wins"); RetroPGF trusts
badgeholders; PageRank trusts link topology. Noesis treats `v(S)` as the adversary's PRIMARY target and
hardens it with a standing **adversarial-gaming loop** (temporal-novelty, saturation damping, the Hodge
collusion detector, realized-flow gating, endorsement-slashing), each vector regression-pinned. The
*method* — measurement hardened by an adversary that never sleeps — is itself the contribution. No prior
contribution-DAG does adversarial hardening of the measure as a first-class, ongoing discipline.

### Axis 2 — THE HODGE INVERSION (method)
HodgeRank uses the harmonic/cyclic residual as a ranking-validity DIAGNOSTIC and *tolerates* it
("cyclic inconsistencies are intrinsic to real data and should be analyzed"). Noesis **inverts the
telos**: the same harmonic residual is the **collusion signature**, attributed per-identity and wired to
an economic **slash** (cc/dd/ee). Their question: "is this ranking consistent?" Ours: "WHO manufactured
this circulation, and how much do they lose?" Same Helmholtz–Hodge math, opposite purpose. Repurposing
the harmonic energy as a stake-slash signal is, as far as the prior art shows, new.
- Forward note: their curl/harmonic split (local vs global cyclicity) is a future refinement — distinguish
  local collusion (curl) from global rings (harmonic). Currently we use the full cycle-space residual.

### Axis 3 — ATTRIBUTION FUSED WITH FRANCHISE (structural)
Prior art *separates* "who contributed" (a score) from "who governs / gets paid" (tokens, votes, grants).
Noesis fuses them: PoM — the contribution measure — IS the **soulbound, unbuyable** consensus weight
(buy storage, not consensus). This fusion is what produces the **structural fork-resistance**
([[attribution-network-cannot-be-stolen-only-joined]]): copying the mechanism either strips attribution
(collapses to a worse chain) or keeps it (credits the origin). A scoring tool you can fork; a franchise
earned on the origin's graph you cannot.
- Forward note: this is the root of the non-zero-sum / reverse-fork thesis. Because copying honestly
  *adds* the copier to the same attribution graph, forking becomes contribution and rival chains
  converge in by accretion rather than competing (`docs/CONVERGENCE-REVERSE-FORK.md`); the same geometry
  one level down credits people/repos/datasets by identifier before they have a wallet
  (`internal/thesis/DESIGN-claimable-attribution.md`). Two levels, one geometry. Both are DESIGN theses:
  the conservation core is built+tested at the reference layer, the cross-chain adapter is unbuilt
  (`internal/STATUS-LEDGER.md`).

### Axis 4 — STRATEGYPROOF-BY-CONSTRUCTION, NOT BY HEURISTIC (method)
SourceCred's sybil-resistance is heuristic PageRank + community moderation; it is gameable and admits it
(community-as-backstop). Noesis's inter-block value is **temporal-novelty via commit-reveal order**,
strategyproof *by construction*: sybil-split, padding, and collusion-ring earn 0 because they add no new
coverage in commit order — proven in tests, not moderated after the fact. Commit-reveal binds
provenance so novelty cannot be front-run.

### Axis 5 — REALIZED-FLOW RETROACTIVE VESTING, STRUCTURAL NOT JURY (method)
RetroPGF is retroactive-by-vote (badgeholders, the open gaming problem). Noesis is
retroactive-by-realized-downstream-flow: value vests as a contribution is *actually built upon*, measured
structurally through the DAG (v5–v8), not voted. This closes RetroPGF's own stated weakness ("no way of
knowing how impact is measured") with structure instead of a panel.

### Axis 6 — SUBSTRATE: DAG ATTRIBUTION NATIVE TO THE STATE MODEL
SourceCred = off-chain compute; Gitcoin/RPGF = L1 contract + off-chain tally. Noesis runs the
attribution/value rules as **CKB cell type-scripts on-VM** — shardable, self-validating, the DAG is the
state model. Attribution is consensus-enforced, not computed in a trusted off-chain script.

## Honest summary (lineage vs novel)
- **Inherited (cited, not claimed):** eigenvector value-flow (PageRank), the Helmholtz–Hodge
  decomposition (HodgeRank), Shapley/Myerson credit allocation, retroactive-funding intuition (RetroPGF),
  the contribution-graph idea (SourceCred), the Ergon proportional money mechanism, the CKB cell model.
- **Novel (the bet):** the COMPOSITION into one chain where the measure is the consensus; and four
  method inversions — measurement-as-attack-surface, the Hodge harmonic-energy-as-slash inversion,
  attribution-fused-with-soulbound-franchise (⇒ structural fork-resistance), and strategyproof/realized-
  flow value that is structural not jury. Composition is the obvious differentiator; the inversions are
  the ones that survive a competitor copying the composition.

## Sources
- HodgeRank: Jiang, Lim, Yao, Ye, *Statistical ranking and combinatorial Hodge theory*, Math.
  Programming 2011 — https://arxiv.org/abs/0811.1067
- SourceCred algorithm — https://github.com/sourcecred/research/blob/master/algorithm.md ·
  https://research.protocol.ai/blog/2020/sourcecred-an-introduction-to-calculating-cred-and-grain/
- Optimism RetroPGF — https://medium.com/ethereum-optimism/retroactive-public-goods-funding-33c9b7d00f0c ·
  social-choice analysis https://arxiv.org/abs/2508.16285
- Shapley credit / Shapley Flow / citation credit — Radicchi et al., Phys. Rev. E 80:056103 (2009);
  Wang et al., Shapley Flow (edge-level credit) — https://arxiv.org/pdf/1804.05327 (Shapley attribution survey)
