# DESIGN — genesis & value-chain lineages: an empty forest, not a rooted tree — ✅ RATIFIED (Will 2026-07-20)

> **Supersedes** the 2026-07-19 "genesis root = the codebase snapshot" framing in this same file.
> That framing **drifted** Noesis from a *general attribution protocol* toward *an on-chain GitHub for
> Noesis* — Will caught it 2026-07-20. This doc records the correction and the architecture it implies.
> The drifted version's *durable* insights (roots are unscored; earned-not-premined) survive here,
> re-applied per-lineage instead of to one privileged root.

## 0. The drift, named

The prior version equated the genesis root with **the constitution** (general — the *rules* that make
contribution measurable) and then with **the codebase** (particular — *this repo's source*). That
equivocation is the whole error. The moment "the root" became "the Noesis codebase," every contribution
frames as descending from Noesis's own source ⇒ the protocol reads as a ledger of contributions *to
Noesis*, not a substrate for measuring contribution to **anything**.

**The structural tell (substrate-geometry match).** A general contribution substrate — value across
*any* domain — is a **forest**: many independent lineages, each with its own origin, heavy-tailed.
Seeding one privileged root imposes a **single-tree topology on a forest substrate** — a geometry
mismatch, the First-Available-Trap. A general protocol cannot have one universal genesis contribution
without becoming about that one thing.

## 1. The architecture (ratified)

- **Noesis is a substrate hosting a FOREST of independent value-chain lineages.** The provenance graph
  (`Cell.parent`) is many disjoint/loosely-joined lineages, each grown from its own `parent = None`
  contributions. There is no single tree and no universal root.
- **A value chain = a connected component of the provenance graph — EMERGENT, not namespaced.** No
  registered domains, no privileged roots, no per-domain objects. "Domains" are simply what the graph
  grows into. This is the maximally-general choice (Will 2026-07-20): the substrate privileges nothing.
- **Empty genesis.** The block-zero state seeds **no contribution cell**. The rules — `chain_id`, the
  consensus mix, the finality rule, the bonded validator set — live in `ChainSpec` (`node/src/
  chainspec.rs`) and are the **consensus-genesis**. They are NOT a contribution in the DAG. (This is why
  the ledger genuinely starts empty, `genesis_node()` unchanged.)
- **Noesis's own development is just ONE lineage** on the substrate (dogfooding — [[voluntary-noesis]]),
  never the substrate's identity. That is the exact line the codebase-root crossed.

## 2. What survives from the prior framing (re-applied per-lineage)

The unscored-root reasoning was correct; it just applies to **every** lineage's roots, not one:

- **Every `parent = None` root is UNSCORED at origin (standing 0).** Nobody is paid for *being* a root.
  Value flows only to what is *built upon* (downstream realized flow — `value_v5`/`flow_gate`,
  `rpc.rs:239-240`). A root earns standing exactly as later contributions in its lineage name it as
  `parent`. Fair-launch (earned-not-premined) holds by construction — no premine anywhere in the forest.
- **The "seed the anchor, not the standing" theorem generalizes:** if any root were paid for authoring a
  lineage's frame, a downstream-flow measure would route that lineage's value back to it — a per-lineage
  premine. So roots stay unscored. (Mirrors PoW-excluded-from-finality: the frame is not scored inside
  its own game.)

## 3. `v(S)` — one protocol, not N (universal floor + domain-adaptive learning)

The measure is what keeps Noesis *general* rather than a framework of per-domain protocols:

- **Universal structural floor (domain-blind, built):** novelty / Sybil / collusion / "was this realized
  downstream over time" — `temporal_novelty` → `pom_scores` → the layered defense (253/253 vs constructed
  adversaries). Nothing here reads a domain.
- **Domain-adaptive learned prediction (the open moat):** the learned `v(S)` that estimates *realized
  value* may calibrate to a domain's signal, but it is bounded to advance/evidence roles — it can never
  *mint*, only estimate. This is where domain enters, on top of a universal floor. Honest status: the
  learned predictive `v(S)` is null-on-structural-features / ~0.60-rich-feature (upside, not the moat);
  the moat is the structural floor.

⇒ **One substrate, one floor, adaptive learning** — not one protocol per domain.

## 4. Bootstrapping without a privileged root (the fair-launch question, re-posed)

Removing the privileged root re-opens the honest genesis-bootstrap question generally: *how do the first
lineages start fairly when standing is earned, not premined, and no root is privileged?* This is the
`first-citizens-ai-genesis-contributors.md` / `DESIGN-bootstrap-admission.md` problem, now correctly
framed as **per-lineage cold-start**, not "seed one universal root." Open; do NOT solve it by privileging.

## 5. The live testnet

The contribution seeded on the live testnet 2026-07-19 (`/contributions` id 1, the "codebase" cell) is
**just one ordinary contribution in one (Noesis) lineage** — submitted through the normal `/submit`
path, earning ordinary novelty standing, `parent = None` like any lineage root. It is **not** a protocol
genesis and carries no special status. Reframed, not privileged. (The demo would show the general case
better with *diverse* lineages, not only a Noesis-about-Noesis cell — a UX follow-up, not an architecture
change.)

## 6. Net

Nothing in the engine (consensus mix, finality, three tokens, soulbound PoM, `v(S)`) was Noesis-specific;
the drift lived only at the genesis/DAG-seeding + framing layer. The fix is: **empty forest genesis,
emergent lineages, no privileged root, Noesis-as-one-lineage.** No code change is required to *be*
correct (genesis is already empty); the correction is to NOT bake a root, to keep the docs/demo general,
and to hold the two ratified calls (emergent lineages · universal-floor + adaptive-learned `v(S)`).
