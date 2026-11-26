# Threat model: if P = NP

Design note, 2026-11-25. Status: **designed and open**, nothing here is built or changes consensus. The questions and
the two central moves are Will's: whether P = NP should be treated like quantum and AI ("better safe than sorry"),
security from step complexity instead of search hardness, and taking crypto agility from Nervos.

## The question

Noesis is post-quantum by construction (hash-based signatures) and is designed against AI-capable adversaries. Both are
specific threats with a safe choice available: Shor's algorithm breaks factoring and discrete logarithms, so we use
hashes; an AI adversary is still bound by the same hardness. Should P = NP be assumed too?

## What a practical P = NP would break

P = NP means finding a solution is as easy as checking one. Every one-way function falls with it. In Noesis that is:

- **Signatures and identity.** XMSS rests on hash preimages being hard to find. They would not be.
- **Content ids, commitments, Merkle proofs.** Collisions become findable, so a hash no longer pins content and a
  commitment no longer binds.
- **Proof of work.** Finding a valid nonce is a search problem; its cost floor would vanish.

No choice of algorithm inside computational cryptography survives this. The honest caveats: it is unproven, most
complexity theorists expect P ≠ NP, and a proof of P = NP could be non-constructive or give an algorithm too slow to use.
So it cannot be hardened against the way quantum is. It can be planned around.

## The move: from "hard to find" to "hard to hurry"

Iterating a hash a billion times is not a search problem; each step needs the one before. Whether it can be shortcut
or parallelized is a different question that P = NP does not settle. Security from **step complexity** therefore rests
on its own assumption, not on P ≠ NP: that iterating a good hash has no shortcut, and that no one's hardware is more than
a small constant factor faster than everyone else's.

What it gives:
- **Time and order without signatures.** A sequential hash clock, with events mixed into one long chain, is a
  timestamp that needs no trusted party. Backdating an event means redoing every step since it, and nobody can do those
  faster than one at a time. Priority, "who had it first", is the record Noesis most needs, and it can rest here.
- **Cheap honest verification.** With checkpoints, different verifiers recompute different segments in parallel. The
  prover must go one step at a time; the checkers need not.

What it does not give, as far as we know:
- **Signatures** (proof of who). They need one-way functions. The nearest step-based substitute is identity by head
  start: a chain kept a little ahead of what is published, which nobody can catch without having started when you did.
  It holds only while hardware stays near parity, a physical assumption, not a mathematical one.
- **Succinct proofs** of long computations. Known verifiable delay functions need assumptions P = NP would break; hash
  based proofs of sequential work need collision resistance. In a P = NP world verification falls back to recomputation.

## Crypto agility: where we have it and where we do not

- **Lock level: built, as in Nervos.** The on-chain lock script maps a script's code hash to a signature standard
  (`standard_of`, `onchain/locksig-typescript/src/main.rs:60-72`). A new scheme is a new tag, not a fork: the identity
  authority is tag 3, the hot and cold identity key is tag 4. Honest floor (`ARCHITECTURE.md`): no on-VM script knows
  tag 4 yet, and the on-VM `CONTROL_ENFORCED` switch is off, so spend signatures are the reference node's check on the
  testnet, not yet the scripts'.
- **Id level: not agile.** Under the scripts, SHA-256 is fixed in the node itself: content ids, the UTXO commitment,
  gossip and sync, the score export (`node/src/factsync.rs`, `follow.rs`, `gossip.rs`, `lib.rs`, `pom_export.rs`,
  `identity_mint.rs`, `utxo_commitment.rs`). Changing it means a reset. CKB is built the same way at its consensus
  layer, with its hash and its proof of work fixed (from memory, not checked here).
- **Proposed: hash-tagged ids.** One leading byte naming the hash function, as multihash does, so a future migration is
  "new records carry the new tag", not a chain reset.

## Hash combiners

Bind ids under two unrelated hash families, so a break in one family alone forges nothing. Which pair, and what it costs
on every cell id, is open. Combiners protect against the far likelier event, one family breaking; they do not protect
against P = NP, which breaks both.

## The floor

- **The social fork** stays the final remedy, as it is today (`SECURITY.md` section 5).
- **Agreement without signatures exists for a known set.** Classic Byzantine agreement works among known parties with
  authenticated channels (pre-shared keys with information-theoretic MACs) when fewer than a third are faulty. A
  fallback, not a design.
- **Information-theoretic pieces where they are cheap.** Shamir secret sharing for cold-key recovery: below the
  threshold, no amount of computation rebuilds the key. A proposed change to the wallet's recovery design.

## Status

| Item | Status |
|---|---|
| Signature agility at the lock level (`standard_of`, tags) | built (tag 4 reference node only; on-VM enforcement off) |
| Hash-tagged content ids | designed, this note |
| Hash combiners for ids | designed, this note; pair and cost open |
| Sequential hash clock for priority and order | designed, this note |
| Shamir shares for cold-key recovery | proposed |
| Identity without one-way functions | open |
| Proof of sequential work as an alternative cost floor for the money layer | open |
