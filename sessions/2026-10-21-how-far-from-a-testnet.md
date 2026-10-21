# Session recap — 2026-10-21 (afternoon) — how far from a testnet, and do Cadence brains box anyone out

Plain-English recap. Reading artifact, not an operator handoff.

## Will's questions
How far is a launchable testnet? How much does it depend on Cadence? And if Noesis used Cadence brains
as nodes in the contribution graph, could ordinary people still contribute, or have we boxed ourselves in?

## Testnet: a hosting decision for one node, weeks of build for many
- **One public node.** Code-complete since 2026-07-18; it ran publicly for one day (07-19) through a
  temporary tunnel. Checked today on current master: the release node builds, serves the wallet, accepts
  two signed contributions (the second naming the first as its parent; the parent's downstream count went
  to 1), rejects an exact repeat as a near-duplicate, and resumes at the same height after a restart.
  The main thing it lacks is an always-on host. fly.io needs a payment card; a new `scripts/vm-deploy.sh`
  puts the same Docker image on an Ubuntu VM, so the host no longer has to be fly.io (written for Oracle's
  Always-Free ARM shape; syntax-checked only, not yet run on a real VM). As we understand Oracle's
  terms (not checked today), it also asks for a card at signup to verify identity but doesn't bill
  Always-Free resources. Smaller gaps: HTTPS needs a
  hostname, and the demo still shows a single lineage.
- **Honest floor of one node:** we are the only producer, so it shows contributing, scoring, provenance
  and JUL mining, not decentralization or the two-key finality rule.
- **Many nodes.** Fork choice by finality support (ratified 07-19) is not built: the code still prefers
  the chain with the most work (`node/src/reorg.rs` `try_reorg`). Finality is not yet wired to that
  choice, new blocks don't propagate live, and the five on-VM enforcement switches are off.

## Cadence: not on the testnet path
Nothing in `node/` or `onchain/` references the Cadence library (checked by search; the word appears
only as block timing); the live scorer is `NoveltyOracleV0`. Cadence's role is the value layer's
research instrument (the transfer program) and, later, possibly the measuring device behind a learned
judge, swapped in through the value-oracle seam by amendment.

## Cadence brains as nodes: nobody is boxed out
In the transfer design the brains are the measuring instrument, standing in for "someone used this":
a contribution is fed to a copy of a brain, and its worth is how much the copy improves over its untaught
twin on a task the contributor can't move. People are still the contributors. The brains hold no
standing and cast no votes. Their cheapness is what makes the twin comparison and the many-judge
agreement checks affordable. The cheap-mind worry belonged only to the older idea of counting brains as
outside witnesses (docs/DESIGN-attestation-exogeneity.md), which the layered-anchor design replaced.

The real limit is scope, and it already has a rule: the transfer measure prices only what a brain can
learn from on a judged task. Everything else falls to the structural floor and the other anchors, and
the rule is that it earns under stricter terms, never zero (Will, 2026-10-19). Those graded terms are designed, not built:
the deposit and vesting window are still single global settings.

A correction on the record: the first answer given to Will today got this wrong. It treated the brains as
cheap members who could flood the graph. That came from reading ARCHITECTURE.md's v(S) section, which
still frames the answer as mind scarcity plus the periphery and doesn't yet describe the transfer program
or the layered anchors.
