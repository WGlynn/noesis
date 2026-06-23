# Noēsis

### Every blockchain is fighting the same war: to be THE chain. Noesis is the first one that ends it.

Other chains compete winner-take-all to be the money, the standard, the one that wins. Builders pick
sides. Communities split in forks. Rivals claw at the same liquidity and attention. Underneath the
"we are all cooperative" branding, the base game is zero-sum, and most of the value created is just
value moved around, or burned outright.

Noesis changes what is being competed for. Its consensus does not reward owning a scarce slot. It
measures **contribution**, and contribution adds up instead of running out. So a rival chain does not
lose to Noesis. It converges in, and keeps everything it built. We call it a **reverse fork**: merging
in instead of splitting off. The adoption war is not won. It is dissolved.

**And your contribution is already here.** Noesis maps the existing contribution graph, every
open-source repo and every contributor, before anyone joins. You do not start from zero. You claim what
is provably already yours. Claiming creates your wallet, so adoption is built in, and because you can
only claim what you actually did, it pulls real builders, not airdrop farmers. Credit comes first, and
pulls adoption behind it.

Underneath, it is a **Proof-of-Mind value chain**: blocks are owned, value flows along the graph of
what builds on what, and the right to finalize is earned by demonstrated contribution rather than
bought with capital. It prices *minds*, not hashes or stake.

---

## What is real today (we tell you out loud)

The most important thing a young protocol can be is honest about its own status. Here is the exact line
between what is built, what is designed, and what is still open. A single status ledger is the authority.

- **Built and tested** at the reference layer: the contribution-conservation core (value flows along
  provenance, sybil / padding / collusion drive to zero), Proof-of-Mind weighted finalization, the
  dispute and slashing mechanics, and the on-chain rules running as type-scripts inside CKB-VM. The
  Rust implementation is exercised by **316 passing tests** (253 reference + 63 integration).
- **Designed, not yet built**: the cross-chain *convergence adapter* that actually lets a rival chain
  reverse-fork in. The non-zero-sum claim is a design thesis resting on a built conservation core and an
  unbuilt adapter. Claimable attribution is likewise designed, with consent guardrails (provenance is a
  fact, standing is inert until claimed, an explicit right to disclaim, and no unconsented payout).
- **Open, and honestly inconclusive**: where value is scored by a *learned* function, that learned
  measure has **not** yet been shown to beat a fixed structural proxy on real data. Its first real-data
  test (DeepFunding) came back null: unsupported, not refuted. The faithful version of that test is the
  next real experiment, not a settled result.

There is no public network yet. This is a reference implementation, pre-launch. We would rather you back
an honest design than an overclaimed demo.

---

## Why it exists

Bitcoin made *scarcity* objective. It did not make *value* objective: proof-of-work prices energy, not
contribution. Noesis closes that gap. A block's value is sourced from the realized downstream flow it
enables, identity is earned and soulbound rather than purchasable, and finalization weight comes from a
Proof-of-Mind score. The cheapest way to gain influence becomes actually contributing. The attack
surface is dissolved, not patched.

```mermaid
flowchart TD
  CONTRIB["Block of thought (contribution)"] --> VAL["Value: temporal-novelty × learned quality<br/>sybil / padding / collusion → 0"]
  VAL --> POM["Proof-of-Mind score<br/>(accumulated synergy value)"]
  POM --> STAND["SOULBOUND standing<br/>consensus weight + right-to-mint"]
  POM --> BYTES["TRANSFERABLE state-bytes<br/>medium of exchange (1 Noeum = 1 byte)"]
  STAND --> CONS["PoM-weighted finalization<br/>(2/3, retention-decay, quorum floor)"]
  CONS --> ENF["The chain disciplines the model"]
  ENF -.-> CONTRIB
  classDef sb fill:#1f2937,stroke:#60a5fa,color:#e5e7eb;
  classDef tr fill:#1f2937,stroke:#34d399,color:#e5e7eb;
  class STAND sb
  class BYTES tr
```

## How it stays honest under attack

Every security-critical input the chain consumes is re-derived from consensus, never accepted as the
transaction assembler claims it. This is the recurring *do not let the attacker choose the input*
invariant. Collusion rings are caught on graph topology alone and slashed. Theft is made structurally
hard by commit-reveal timestamp priority: the record exists before any claim, so a fresh identity is
worth zero. The design goal throughout is structural honesty, where dishonesty is unprofitable by
construction rather than discouraged by policy.

## Architecture

```
Execution    on-VM type-scripts (RISC-V / CKB-VM): intake floors, proven novelty, finalization
Value        novelty -> similarity/semantic floors -> realized-flow gate -> priced identity -> dispute
Consensus    PoM-weighted finalization (2/3, retention-decay, quorum floor), AND-composed proof mix
State        Cell model (UTXO-style), sparse-merkle novelty index, commit-reveal ordering
```

The substrate is Nervos CKB's design: Rust, the RISC-V CKB-VM, and the Cell model, with Proof-of-Mind
as the consensus and value mechanism on top.

## Repository layout

```
node/            Reference implementation (Rust, host): consensus, value, dispute, the SMT
                 novelty index, the fixed-point arithmetic cores, and the full test suite.
onchain/
  noesis-core/   no_std verify-side core shared by the node and the type-scripts:
                 ONE source of truth for the arithmetic both sides must agree on.
  pom-typescript/          On-VM intake type-script (novelty/semantic floors, proofs).
  finalization-typescript/ On-VM PoM-weighted finalization (header-sourced `now`).
docs/            Whitepaper, protocol specs, the accessible-tier explainers, positioning.
marketing/       The plain-language pitch (deck, "Noesis for Humans" paper, visuals).
research/        Prototype models (Python) the Rust implementation is derived from.
scripts/         Repo-hygiene tooling (doc-coherence, study guide).
```

## Build & test

```bash
make test        # host suite: node + noesis-core
make fmt         # rustfmt
make clippy      # clippy, warnings-as-errors
make elf         # build the RISC-V type-scripts (nightly + riscv64imac target)
```

The on-VM type-scripts compile to RISC-V and are validated end-to-end against a host harness that serves
the Cell environment, so the same rule produces the same verdict on-VM and off-VM. Cross-boundary
determinism comes from canonical fixed-point arithmetic.

## Start here

- **New to it?** [`docs/NOESIS-FOR-DUMMIES.md`](docs/NOESIS-FOR-DUMMIES.md) and the plain-language
  [`marketing/paper/noesis-for-humans.md`](marketing/paper/noesis-for-humans.md).
- **The pitch:** [`marketing/deck/index.html`](marketing/deck/index.html) (open in a browser).
- **The full design:** [`docs/WHITEPAPER.md`](docs/WHITEPAPER.md).
- **The mechanism:** [`docs/POM-CONSENSUS.md`](docs/POM-CONSENSUS.md) and
  [`docs/BLOCK-ECONOMY-SPEC.md`](docs/BLOCK-ECONOMY-SPEC.md).
- **The honest status of every claim:** [`internal/STATUS-LEDGER.md`](internal/STATUS-LEDGER.md).
- **The plan:** [`ROADMAP.md`](ROADMAP.md).

> Naming: **Noēsis** is the network (the act of mind). **Noeum** is the unit (1 Noeum = 1 byte of state
> = 1 PoM unit). Core inspiration: Nervos CKB.

## License

Proprietary and confidential during the pre-release period. See [`LICENSE`](LICENSE). An open-source
license will be designated at public release.
