# Noesis — ZK integration design (core protocol)

Design doc, 2026-07-01. Maps the four ZK fits onto the ACTUAL core, with tooling, build order, and an
honest tractable-vs-research split. Status: all 🟡 designed / 🔬 to-build — nothing ZK ships today.
Grounded to `file:line`; verify before claiming.

## Why the fit is unusually clean

`onchain/noesis-core` is **`no_std` and already builds for `riscv64imac`** (`onchain/noesis-core/src/lib.rs:1-14`).
The verify-side primitives both worlds agree on live there: `coverage` (:88), `semantic_floor_q16` (:67),
`finalizes_pos_pom_fixed` (:481), value-movement identity (:703), wallet keygen/sign (:616). Because the
core is RISC-V, a **RISC-V zkVM proves the code we already run** — no re-implementation into a circuit.

---

## Fit 1 — zkVM proof of core execution (RISC Zero / SP1). ★ do first

**What.** Prove, succinctly, that `finalizes_pos_pom_fixed` / `coverage` / `semantic_floor_q16` executed
correctly on given inputs, verifiable cheaply anywhere. The scoring/finalization becomes **trustless**
(a proof, not an attestation).

**Why it's the strongest fit.** The core is `no_std` + `riscv64imac`. RISC Zero and SP1 are RISC-V
zkVMs; they execute an ELF and emit a STARK proof of that execution. We compile the *same* `noesis-core`
functions into the guest and prove them — near-zero new trusted code, and it directly hardens the
"`v(S)` computes off-chain, posts an attestation" path (make the attestation a proof).

**Tractable now.** The functions are pure, `no_std`, deterministic (Q32.32 fixed-point) — ideal zkVM
guests. Start with `finalizes_pos_pom_fixed` (small, pure) as the proof-of-concept, then `coverage` /
`semantic_floor_q16`.

**Risk.** Proving cost for the heavier novelty path; batch/recursion later. Fixed-point already avoids
float non-determinism (the reason Phase-3 used Q32.32).

## Fit 2 — private / confidential contribution scoring (zkML-adjacent)

**What.** Prove "my contribution scored value ≥ V and cleared the novelty floor" **without revealing the
content**. Content + secret are private witnesses; `semantic_floor_q16(novelty, data, theta)` (:67) runs
inside the proof; only the verdict + value bound are public.

**Why.** Unlocks proprietary / pre-publication work earning standing (the sovereign-data direction). The
`v(S)` frontier is ML-flavored, so this is genuine **zkML** territory — real, credible experience.

**Path.** Same zkVM as Fit 1 with the content as a *private* input (RISC Zero journal exposes only the
public outputs). No separate circuit language needed — reuse the core.

## Fit 3 — novelty as a ZK set-non-membership proof

**What.** `coverage(data)` emits `CovId`s (:88). Prove your submission's shingle/coverage set is
sufficiently *disjoint* from the corpus (overlap < `theta`) **without revealing content or the full
corpus** — i.e. prove novelty against a Merkle-committed corpus.

**Path.** Merkle root of corpus `CovId`s on-chain; a **non-membership / low-overlap proof** in Noir or
Circom. Smaller, self-contained circuit; good second artifact after the zkVM PoC.

## Fit 4 — private provenance / account-link (selective disclosure)

**What.** Prove the handle↔key binding (demo panel 06) and the commit-reveal **without publishing the
linkage** — selective-disclosure provenance; same for dispute/slashing (prove you did the claimed work
without revealing it).

**Path.** ZK proof of knowledge over the signed binding + commitment. Noir/Circom. Depends on Fit 3's
commitment plumbing.

---

## Where NOT to use ZK (avoid the sophistication trap)

The floor arithmetic (`finalizes_pos_pom_fixed`) is cheap and already runs on-VM and public — do **not**
hand-write a circuit for it. The only reason to "prove the floor" is succinct off-chain verification,
which is exactly **Fit 1's zkVM job**, not a bespoke circuit. Keep ZK where compute is heavy or inputs
are private; never where it's already light and public.

## Tooling

- **RISC Zero / SP1** (RISC-V zkVM) → Fits 1 & 2. Native match to `noesis-core` (riscv64imac). Rust guest.
- **Noir (Aztec) or Circom + snarkjs** → Fits 3 & 4 (small dedicated circuits: Merkle non-membership,
  proof-of-knowledge). Noir preferred (Rust-like, better DX).

## Build order

1. **RISC Zero PoC:** prove `finalizes_pos_pom_fixed` execution. Wraps existing `no_std` code; smallest
   real zkVM win. (Also the cleanest portfolio artifact.) ✅ **DONE (2026-07-15)** — `onchain/zk-finalize/`:
   guest wraps the real fn via the existing wire format, host proves+verifies the canonical fixtures,
   and a host-stable `parity/` harness (no risc0) locks the expected journal verdicts (whale-alone
   REJECTED, whale+2 FINALIZES). The real receipt is produced in CI on the free public-repo Linux
   runner (`.github/workflows/zk-receipt.yml`, run 29416024770): all three fixtures proven with a real
   STARK (`RISC0_DEV_MODE=0`), `receipt.verify(ZK_FINALIZE_ID)` passing, verdicts == parity. **⇒ ✅.**
2. **Private-input scoring proof (Fit 2):** content as private witness, value-bound + floor-pass public.
3. **Merkle non-membership novelty (Fit 3):** Noir circuit over `coverage` CovIds vs corpus root.
4. **Account-link selective disclosure (Fit 4):** Noir proof over the binding + commitment.

## Honest status

All four are 🟡 designed here, 🔬 unbuilt. Fit 1 is lowest-risk because it proves *existing* code rather
than re-deriving logic into a circuit. Do not claim ZK ships until a guest builds and a proof verifies.
This doc is the plan; implementation is a focused effort (Rust zkVM guest + Noir circuits), best run as
its own session with the core open.
