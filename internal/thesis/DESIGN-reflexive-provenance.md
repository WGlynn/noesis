# DESIGN — Reflexive provenance (standing rule, Will-ratified 2026-06-23)

> Will: *"we give them provenance for their data?"* → *"continue to abide by this rule."* A standing
> design law for Noesis, not a one-off. First applied to the DeepFunding ingestion
> (`data/deepfunding/PROVENANCE.md`).

## The rule
**Any external data or contribution Noesis ingests is recorded with provenance, and its providers are
attributed contributors.** The contribution-attribution chain attributes its OWN inputs — anything less
is self-refuting. Concretely, on every ingestion:
1. **Commit the source** — content hash (commit-reveal root) + origin + licence, before use, so what was
   ingested is verifiable not asserted (the same timestamp-priority discipline any contribution gets).
2. **Attribute the providers** — the data creators (e.g. the DeepFunding jurors, the OSS repos, EF/Gitcoin)
   are first-class contributors; in a live chain they earn standing for the downstream value their
   contribution adds, measured by the very mechanism their data helped build.
3. **Cite the record** — any artifact trained on / derived from the data (e.g. learned-`v(S)` weights)
   names its provenance record as lineage.

## Why it is load-bearing (not just ethics)
- **Self-consistency / dogfood:** a chain that measures contribution but does not attribute the
  contributions that built it fails its own thesis. Applying the mechanism reflexively is the strongest
  proof it works.
- **Licence + trust:** attribution is also licence compliance (DeepFunding = MIT) and is what earns the
  right to keep ingesting community data.
- **It composes with the moat:** the learned-`v(S)` trained on ingested labels carries that data's
  provenance, so the value it later assigns is auditable back to its training lineage — closing the
  airgap between "the measure" and "where the measure came from."

## Scope
Applies to ALL external ingestion (datasets, OSS dependency graphs, partner data, scraped corpora). Does
NOT apply to internally-generated chain state (that already has native provenance). Candidate to harden
into an ingestion-time gate once the live data pipeline exists (per [[primitive_universal-coverage-hook]]:
an "always" belongs in a gate, not just a doc). Sibling: `data/deepfunding/PROVENANCE.md` (first instance).
