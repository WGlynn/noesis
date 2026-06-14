# Security Policy

## Reporting a vulnerability

noesis is a pre-launch protocol; there is no public network and no funds at risk.
If you find a flaw in the consensus, value, dispute, or execution layers, please
report it privately rather than opening a public issue. Open a
[GitHub security advisory](https://github.com/WGlynn/noesis/security/advisories/new)
or contact the maintainers directly. We aim to acknowledge within 72 hours.

Please include: the component (`node` module or on-VM type-script), a minimal
reproduction (ideally a failing test against the suite), and the impact you believe
it has on a deployed network.

## Security model

noesis is designed so that the cheapest path to influence is to contribute, not to
attack — the attack surface is dissolved structurally rather than patched. Two
invariants carry most of the weight:

- **Don't let the attacker choose a security-critical input.** Every value the chain
  acts on — index identity, commit ordering, the finalization clock `now`, the
  validator set — is re-derived from consensus on-VM and rejected if it cannot be
  reconstructed. A free, transaction-chosen value is self-assertion, not a check.
- **History is verified, not trusted.** Novelty, similarity, and finalization are
  proven against committed state (a sparse-merkle novelty index, header-sourced
  time, the bonded validator registry), with classification that makes omission and
  stale-state attacks structurally impossible rather than merely detectable.

A standing internal audit of these surfaces lives in the design docs (see the
attacker-choosable-input audit and the per-mechanism critical-QA notes in `docs/`).

## Scope

In scope: the Rust reference implementation (`node/`, `onchain/`) and the on-VM
type-scripts. Out of scope (pre-launch): deployment infrastructure, third-party
dependencies' own advisories, and the prototype models in `research/`.
