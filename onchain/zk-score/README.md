# zk-score — RISC Zero PoC (Fit 2 of `docs/ZK-INTEGRATION.md`)

Proves one execution of `noesis_core::zk_score_eval` in a RISC-V zkVM with the **contribution content
and its membership proofs as a PRIVATE witness**. Only the 4-tuple
`(public_digest, nullifier, accepted, value>=V_FLOOR)` is published. This is the *private /
confidential contribution scoring* fit — "earn Proof-of-Mind standing for work you never disclose."

It is the privacy-side twin of [`../zk-finalize`](../zk-finalize) (Fit 1, which proves the finality
verdict on **public** inputs). Together the two artifacts embody the transparency<->privacy
**factoring** (see `docs/transparency-privacy-grounding.md`): transparency belongs to the **rule**
(public, provable), privacy belongs to the **inputs** (a private witness), and ZK proves the
transparent rule ran on the private input.

## What is public vs private

| | value | role |
|---|---|---|
| **private witness** | contribution `data`, per-shingle membership `proofs` | consumed in-proof, never committed |
| **public input** | corpus `root`, `theta_sim`, `theta_ent` | bound into the journal digest |
| **baked constant** | `ZK_SCORE_V_FLOOR` (the bar) | in the guest image id — NOT a prover input |
| **public output (journal)** | `(public_digest, nullifier, accepted, value>=V_FLOOR)` | anyone verifies without the content |

- `public_digest = sha256(domain ‖ root ‖ theta_sim ‖ theta_ent ‖ V_FLOOR)` (`noesis_core::zk_score_public_digest`).
- `nullifier = sha256(domain ‖ sorted unique shingle keys)` (`noesis_core::zk_score_nullifier`) — binds
  the *content* (not identity; that is Fit 4), so the value ledger can dedup replays / double-claims.
- `accepted` = the cell was well-formed **and non-empty** and every shingle carried a valid
  membership/non-membership proof against the SUPPLIED root. Omission, padding, a forged path, or empty
  content rejects the whole cell.
- `value>=V_FLOOR` = the floored novelty score cleared the protocol floor. The exact score and the
  content never leave the proof.

## ⚠ The verifier contract (load-bearing — a receipt alone is NOT standing)

A receipt proves *"some hidden content scored `>= V_FLOOR` against root R, and here is its nullifier."*
It does **not** prove R is the real corpus. `root` is a prover-chosen public input; a malicious prover
can supply the empty-tree root (publicly computable) and forge full novelty for plagiarised content.
The proof is honest — it faithfully scored against the corpus it was handed.

**Therefore the consumer MUST, before granting standing:**
1. `receipt.verify(ZK_SCORE_ID)` — pins the code AND `V_FLOOR` (the bar cannot be prover-chosen).
2. `public_digest == zk_score_public_digest(canonical_corpus_root, policy_theta_sim, policy_theta_ent)`
   — pins `root` and the thresholds to consensus. This is what authenticates the novelty judgement.
3. `accepted && value>=V_FLOOR`.
4. nullifier unseen — reject a repeated nullifier (one standing grant per contribution).

Skipping step 2 is the empty-root standing forgery. The `parity`/`host` harnesses demonstrate the
check: the `empty-root forgery` fixture is *accepted and clears the floor* yet is denied standing
because its digest does not match the canonical one.

## Layout

- `parity/` — host-stable harness (NO risc0). Builds a corpus index, runs `zk_score_eval` on the
  fixtures through the SAME single-sourced proof-wire (`noesis_core::flatten_proofs`/`unflatten_proofs`)
  and digest (`zk_score_public_digest`, sha256) the guest uses, and applies the verifier contract.
  **Self-checking**: honest proofs are re-verified with core's OWN `verify_member`/`verify_non_member`
  (SMT drift = hard panic). Runs today.
- `methods/guest/` — the zkVM guest: reads content + flat proofs (private) + root/thetas (public), runs
  the real core verdict, commits the 4-tuple.
- `methods/` — `risc0_build::embed_methods()` exposes `ZK_SCORE_ELF` / `ZK_SCORE_ID`.
- `host/` — builds the corpus, feeds the content as a private input, proves, verifies each receipt
  against the image id, and applies the digest-pin verifier contract.

## Ground truth (verified on host stable, `cargo run` in `parity/`)

| Fixture | `(accepted, value>=V)` | Standing | Why |
|---|---|---|---|
| fresh novel work | `(true, true)` | **STANDING** | novel vs corpus, passes entropy floor |
| exact duplicate | `(true, false)` | NO-STAND | similarity floor zeroes the score |
| high-entropy noise | `(true, false)` | NO-STAND | entropy floor zeroes the score |
| empty content | `(false, false)` | REJECTED | empty cell is not well-formed |
| tampered proof | `(false, false)` | REJECTED | a forged path proves neither polarity |
| forged member→absent | `(false, false)` | REJECTED | empty-root paths vs the real root prove neither |
| empty-root forgery | `(true, true)` | **ROOT-PIN denied** | scores against a fake corpus; digest-pin rejects |

No fixture's content bytes ever appear in the journal line — that is the privacy property.

```
cd parity && cargo run --release      # host stable, no risc0 tooling
```

## Proving (env-gated)

RISC Zero's prover does not run natively on Windows — it needs Linux or WSL2. This box has neither, so
the receipt is produced under WSL2/Linux or CI, not here.

```
# one-time, on Linux / WSL2:
curl -L https://risczero.com/install | bash && rzup install
# then:
cd host && cargo run --release        # RISC0_DEV_MODE=1 for a fast (non-cryptographic) dry run
```

## Status (honest — do not round up)

- ✅ Guest wraps the real core verdict (`zk_score_eval`); wire + digest + nullifier + floor all
  single-sourced in `noesis-core`; parity harness GREEN incl. the three forgery-caught fixtures;
  content proven to stay out of the journal; the verifier's digest-pin contract is demonstrated.
- 🟡 Guest/host risc0 code written against the risc0 **1.2** line; **not yet compiled or proven** on
  this machine (no prover env). Versions may need a bump to match whatever `rzup` installs.
- 🔬 A verifying receipt has not been produced. Do not claim "private scoring ships" until `host`
  proves and `receipt.verify` passes. Next: Fit 3 (Noir Merkle non-membership) and Fit 4 (account-link
  selective disclosure), per `docs/ZK-INTEGRATION.md`.

## Boundary

This proves **the scoring rule ran correctly on some hidden content against a supplied corpus root**,
conditional on the verifier contract above. It never proves the content is "good," never data
availability (the root is not the data), and does not bind the content to an *identity* — the nullifier
binds the *content* for dedup; identity binding is Fit 4. Residual: cross-pseudonym work-splitting is
only fully closed when `root` is pinned to a consensus snapshot that already contains earlier fragments
(so they self-overlap) and/or a per-identity accumulator lives at the ledger layer.
