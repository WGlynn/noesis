# Phase 3 — Provable History (the ZK core): status + plan

> Stateless-verification engagement. Phase 3 = prove the state transition in a zkVM, honest cost
> report (a human go/no-go). **This box cannot run a STARK prover** (no WSL2/Docker/`r0vm`, and no C
> compiler), so the engagement's *no-mocked-benchmarks* rule forbids fabricating numbers here. What
> IS achievable here — and is done — is the **guest logic, host-verified green.** Real proving is
> **Option B, deferred until a Linux env exists** (Will, 2026-07-12).

## What is DONE (Option A, host-verified — no prover needed)

**Milestone 1 guest logic — "prove one block's transition against commitments" — is written and
green.** `node/src/utxo_commitment.rs`:
- `UtxoCommitment::transition(spends, creates) -> TransitionWitness` — applies a block's value
  movement and emits a **bounded** witness (touched keys + one compiled SMT multi-proof; O(touched ·
  depth), independent of |UTXO set|).
- `verify_transition(&TransitionWitness) -> bool` — **the guest program itself.** Pure re-hashing;
  `no_std`-portable as written. It exploits the SMT property that a single compiled multi-proof
  verifies the touched keys at their **old** values under `old_root` *and* their **new** values under
  `new_root` — so one inclusion proof doubles as a transition proof.
- Test `utxo_transition_is_zk_verifiable_against_both_roots` (green): the honest witness verifies; and
  three attacks are rejected — a forged `new_root`, a corrupted proof, and **claiming a spent coin
  survived** (the double-spend attempt). This is exactly the check a RISC Zero guest commits to its
  journal.

Suite after this increment: `cargo test -p noesis --lib utxo_commitment` → **8 passed / 1 ignored**
(the ignored one is the Phase-2 measurement).

## What is NOT done — and why (honest)

A **real STARK receipt** + real proving-cost numbers. Blocked on this Windows box: **no prover**
(RISC Zero needs Linux/WSL2/Docker) and **no C compiler**. Per the engagement's honesty rule, no
numbers are invented; the milestone is code + a host proof of the *logic*, not a receipt.

The existing `onchain/zk-finalize/` sits at exactly this boundary already (guest for the finalization
verdict written, parity green, **no receipt**) — Phase 3 extends that pattern to the state transition.

## Option B — do this when a Linux / WSL2 / CI env exists

1. **Share the guest logic `no_std`.** Move `verify_transition` + `Blake2bRefHasher` (+ the leaf-key
   derivation) into `noesis-core` (`no_std`). The vendored SMT and `blake2b-ref` are already
   `no_std`, so host + on-VM + guest link ONE definition (single-source; no drift).
2. **Scaffold `onchain/zk-utxo/`** mirroring `onchain/zk-finalize/`:
   - `parity/` — host-stable (no risc0): builds a UTXO set, runs `transition`, records
     `(old_root, new_root)` ground truth the receipt must reproduce. (Runnable anywhere — build it
     here too as the cross-check.)
   - `methods/guest/` — the guest: read `(old_root, new_root, leaves, proof)`, call
     `verify_transition`, commit `(sha256(inputs), ok)` to the journal.
   - `methods/` + `host/` — `risc0_build::embed_methods`, prove + `receipt.verify`.
3. **Prove on Linux** and record REAL numbers **here** (`docs/phase3-zk-plan.md`): proving time + cost
   per transition, projected one-time backfill over history, tip-pace, receipt size, verify time.
   **Do NOT claim "ZK ships" until a receipt verifies.**
4. **Then milestones 2–3:** recursion (fold K transition-proofs into one receipt — RISC Zero
   composition) and the **sync artifact** (a receipt asserting "genesis → state root X is valid" that
   a light node verifies in ms, combined with a Phase-2 snapshot to start).

**Cost footnote (measure, don't assume):** RISC Zero hardware-accelerates **SHA-256, not blake2b**, so
the blake2b commitment proves slower there. This is a real Phase-3 cost input — and the go/no-go
turns on the measured number, not a guess.

## Boundaries (restated)

A validity receipt proves **validity only** — never **canonicality** (a valid fork is still a fork;
fork-choice untouched) and never **data availability** (a root is not the data). "Trustless full
verification" is never claimed without naming both.
