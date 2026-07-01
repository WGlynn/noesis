# zk-finalize — RISC Zero PoC (Fit 1 of `docs/ZK-INTEGRATION.md`)

Proves one execution of `noesis_core::finalization::finalizes_pos_pom_fixed` in a RISC-V zkVM.
The scoring/finalization verdict becomes a **proof, not an attestation** — the "`v(S)` computes
off-chain, posts an attestation" path hardened into "posts a proof."

Why the fit is clean: `noesis-core` is `no_std + alloc` and already builds for RISC-V, and it
already ships a flat wire format (`parse_finalization_cell` / `parse_votes` +
`encode_finalization_cell` / `encode_votes`). The guest proves the **same code the node runs** —
no circuit re-implementation.

## Layout

- `parity/` — host-stable harness (NO risc0). Runs `finalizes_pos_pom_fixed` on the canonical
  fixtures through the wire format and records the verdicts the proof must reproduce. Runs today.
- `methods/guest/` — the zkVM guest: reads cell + votes + clock, decodes, calls the real finalize
  fn, commits `(sha256(inputs), verdict)` to the journal.
- `methods/` — `risc0_build::embed_methods()` compiles the guest and exposes `ZK_FINALIZE_ELF` /
  `ZK_FINALIZE_ID`.
- `host/` — encodes the fixtures with the core's producer, proves, verifies each receipt against
  the image id, decodes the journal, and asserts the verdict equals the parity ground truth.

## Ground truth (verified on host stable, `cargo run` in `parity/`)

| Fixture | Verdict |
|---|---|
| whale alone (pure capital) | REJECTED |
| whale + 1 contributor | REJECTED |
| whale + 2 contributors | FINALIZES |

Anti-capture: capital cannot finalize without the contribution (PoM) axis clearing its floor.

```
cd parity && cargo run --release      # host stable, no risc0 tooling
```

## Proving (env-gated)

RISC Zero's prover does not run natively on Windows — it needs Linux or WSL2. This box has neither
a WSL distro nor Docker installed, so the receipt is produced under WSL2/Linux or CI, not here.

```
# one-time, on Linux / WSL2:
curl -L https://risczero.com/install | bash && rzup install
# then:
cd host && cargo run --release        # RISC0_DEV_MODE=1 for a fast (non-cryptographic) dry run
```

`RISC0_DEV_MODE=1` executes the guest and fills the journal without generating a STARK — use it to
confirm the guest logic + journal shape match `parity/` before paying for a real proof.

## Status (honest)

- ✅ Guest wraps the real core via the existing wire format; parity harness GREEN on host stable.
- 🟡 Guest/host risc0 code written against the risc0 **1.2** line; **not yet compiled or proven**
  on this machine (no prover env). Versions may need a bump to match whatever `rzup` installs.
- 🔬 A verifying receipt has not been produced. Do not claim "ZK ships" until `host` proves and
  `receipt.verify` passes. Next after this: Fit 2 (private-input scoring), Fit 3 (Noir novelty
  non-membership), per `docs/ZK-INTEGRATION.md`.
