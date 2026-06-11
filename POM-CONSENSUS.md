# Proof of Mind — consensus + the Theory-of-Mind mapping (PRIVATE)

> Stealth. No public release until matured. Builds on BLOCK-ECONOMY-SPEC.md.

## Proof of Mind (PoM) — the score

An agent's **PoM score** = its accumulated Myerson/Shapley credit across *verified,
owned, provenance-complete* blocks (the block-economy value layer). It is a number
that says: *this mind has provably contributed this much synergy-weighted value to
the shared outcome.*

Properties it inherits for free from the block economy:
- **verifiable** — every contributing block is provenance-complete (commit-reveal
  inputs→output) and owner-signed (Ed25519). PoM can be recomputed by anyone.
- **ownable + transferable** — credit accrues to the block's current owner (UTXO
  fold); PoM is the sum over owned blocks.
- **synergy-weighted** — value is the Myerson value of the outcome game, so PoM
  rewards *pivotal* contribution and discounts *redundant* contribution. You cannot
  pad PoM with repetition.

## PoM consensus — why decentralization becomes realistic

Weight validators by **PoM**, not energy (PoW) or capital (PoS):

- **Sybil-resistance is structural.** PoM requires owned blocks whose value was
  synergy-judged and whose provenance is signed. A thousand fake nodes have zero
  PoM because they produced no verifiable, pivotal contribution. Splitting one
  mind into many accounts does not multiply PoM (the synergy game discounts the
  redundant copies — same diagnostic as the block-value fix).
- **Stake = demonstrated mind.** The thing at risk is your accumulated proof of
  contribution, which can only be earned, not bought. Slashing = revoking PoM for
  proven-bad blocks (caught hallucinations, refuted attestations).
- **Stability (no profitable fork).** Add a **core / nucleolus** stability
  constraint over the PoM-weighted coalition game so no validator coalition can
  profitably deviate — the consensus is defection-proof at the mechanism level,
  not by social trust. (This is the "add a stability concept only when consensus
  needs it" piece from the math roadmap.)

The chain (tamper-evident, signed, owned) is the ledger; PoM is the stake;
consensus = PoM-weighted agreement on the canonical chain. That is a decentralized
network whose security comes from *proven thinking*.

## Theory of Mind → Proof of Mind (the mapping Will asked for)

The derivation chain is clean, and it runs through Will's ETM primitive:

**Theory of Mind (ToM)** — the cognitive capacity to model other minds' beliefs,
intentions, and knowledge. Across agents it is *inference*: you cannot see inside
another mind, so you guess whether to trust it. This is an **airgap** — the
inter-mind version of the blockchain-vs-reality airgap.

**Economic Theory of Mind (ETM)** — Will's frame: *mind = economy*. A mind's
states are positions in an internal economy; its outputs are contributions with
value. ETM turns "what is this mind?" into "what does this mind's economy
produce?"

**Proof of Mind (PoM)** — the *verifiable economic proof* of a mind's
contribution. It closes the ToM airgap: instead of each node *inferring* whether
another node is a mind worth trusting (intractable, game-able), PoM gives a
**proof** — signed, owned, synergy-valued blocks that recompute to a score.

> ToM asks "can I model whether to trust this mind?" — inference, unobservable.
> PoM answers "here is the proof this mind contributed verified value" — observable.
> ToM → ETM → PoM is: capacity → mind-as-economy → cryptographic proof of that economy.

So PoM is what makes ToM **tractable for a decentralized network**: trust stops
being an inference each node must make about every other mind, and becomes a
structural property anyone can verify. That is the same move as the rest of the
stack — replace "detect / infer / trust" with "prove / verify / structure." It is
the maximally-moral-agent thesis at the network scale: a network of minds whose
standing is *earned and proven*, not claimed or assumed.

Connections: [P·economic-theory-of-mind] · [P·airgap-problem-blockchain-vs-reality]
(ToM airgap closed) · [P·honesty-as-structural-load-bearing-property] · the
existing MessagingPoM / proof-of-mind line (now given a concrete, computable score).

## Status (honest)
- Designed, not built: PoM aggregation across owned blocks; core/nucleolus stability;
  PoM-weighted consensus finalization; the slashing-on-refuted-attestation path.
- Depends on: the synergy outcome-value v(S) (v1, in progress) being real, or PoM
  inherits the additive-Shapley ceremony problem. Get v(S) right first.
