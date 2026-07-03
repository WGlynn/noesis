# Noesis: what ZK is for (four applications)

Four places zero-knowledge proofs earn their place in Noesis. Two use ZK for succinctness (make a
verdict checkable by anyone, cheaply, without re-running it); three use it for privacy (prove a fact
about hidden data). Fit 1 is pure succinctness, Fit 2 is both, Fits 3 and 4 are privacy-first. That
is the whole map.

The reason it is a clean fit rather than a rewrite: the Noesis core is small, pure, and deterministic,
so a RISC-V zkVM proves the exact code the node already runs. No logic gets re-derived into a circuit.

## Fit 1 — proving the finality verdict (succinctness)
Secret: none. Verifier: any outside party (a light client, another chain, a partner).

Today a node computes "block X finalized" and asks you to trust that it ran the rule. ZK replaces the
"trust me" with a receipt: "I ran the canonical finality rule on these inputs and it returned
finalize," cheap to check without redoing the work or trusting the prover. The application is
trustless bridging and light-client finality: anyone verifies Noesis finalized something without
running a Noesis node. ZK earns its place because the alternative, re-executing to verify, does not
scale to external verifiers. This is the one that is built as a proof of concept.

## Fit 2 — private contribution scoring (privacy, and the zkML mile)
Secret: the contribution content. Verifier: the chain's value ledger.

Normally, to earn standing you reveal your contribution so the network can score it. That rules out
proprietary or pre-publication work; you cannot get credit for a secret you must publish. ZK flips
it: run the scoring inside the proof with the content as a hidden input, and publish only "this
scored at least V and cleared the novelty floor." The application is sovereign-data contribution:
earn Proof-of-Mind standing for work you never disclose. Proving a scoring computation over private
data is genuine zkML, and it is the most resume-credible of the four.

## Fit 3 — private novelty check (privacy)
Secret: your content and the corpus. Verifier: the chain.

Novelty in Noesis means your work barely overlaps what already exists. Proving that the plain way
would expose your content and the whole corpus. ZK proves "overlap below the threshold against the
committed corpus" while hiding both sides. The application is a private anti-copy gate: the novelty
check runs without a public content dump. It is a small dedicated circuit rather than a zkVM because
it reduces to membership over hashes, which is cheap and circuit-shaped.

## Fit 4 — private provenance and account-link (privacy)
Secret: the handle-to-key link and the work. Verifier: the chain, or a disputant.

The provenance layer binds a handle to a key and commits to work. Fit 4 proves "I hold the key for
this handle and I did this work" without publishing the link. Two applications fall out: pseudonymous
standing (build reputation without revealing which key is which handle), and private dispute or
slashing (prove misconduct, or prove you did the claimed work, without revealing the work). It
depends on Fit 3's commitment plumbing, so it comes last.

## The hierarchy in one line each
- Fit 1: make the verdict checkable by outsiders.
- Fit 2: make the score earnable on hidden work.
- Fit 3: make novelty checkable without exposure.
- Fit 4: make identity and provenance provable without linkage.
