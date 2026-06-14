# Contributing to noesis

noesis is in a pre-release / stealth period: development is currently closed and
external contributions are not yet open. This document describes how the codebase is
built and the discipline it is held to, so the workflow is legible now and ready for
outside contributors at public release.

## Build & test

```bash
make test        # host suite — node + noesis-core
make fmt         # rustfmt (check with `cargo fmt --all -- --check`)
make clippy      # clippy with warnings denied
make elf         # build the RISC-V type-scripts (nightly + riscv64imac target)
```

The host crates (`node`, `onchain/noesis-core`) share one workspace and lockfile. The
on-VM type-scripts (`onchain/pom-typescript`, `onchain/finalization-typescript`) pin their
own nightly toolchain and the `riscv64imac-unknown-none-elf` target and are built
standalone; rebuilt ELFs are checked in under `node/tests/fixtures/` so the integration
tests run them.

## Engineering discipline

- **Single source of truth.** Arithmetic both the host and the on-VM scripts must agree
  on lives once in `onchain/noesis-core`; the node re-exports it and a drift-guard test
  asserts equivalence. Don't duplicate a core — re-export it.
- **Determinism on-VM.** No floating point in any rule that runs inside the VM. Use the
  fixed-point cores (Q16.16 / Q32.32); add a drift-guard against the f64 reference.
- **Every security-critical input is consensus-sourced.** Never accept a value the
  transaction assembler can choose for a check; re-derive it and reject what you can't
  reconstruct.
- **Adversarial-first.** New mechanisms ship with a hostile pass that tries to break
  them; pin survivors as named open-gap tests rather than hiding them.
- **Honest scope.** Distinguish *demonstrated* from *designed*. Deploy-coupled paths are
  pinned as honest TODOs, not claimed as enforced.

## Repo hygiene gates (enforced on commit)

- `scripts/doc-coherence.py` — documentation test-count / claim coherence with the code.
- `scripts/study-guide.py` — keeps `STUDY-GUIDE.md` regenerated from the repo.

Run `python scripts/doc-coherence.py --stamp` and `python scripts/study-guide.py` before
committing if either reports drift.
