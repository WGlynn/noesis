# CKB-VM PORT — the on-VM PoM type-script (verified-API design, PRIVATE)

> Frontier item #2 (HANDOFF). Rule honored: **no assumed APIs** — everything below was
> read from crate source on this machine (`~/.cargo/registry/src/`), versions pinned.
> Status: DESIGN VERIFIED, code increment not started. The fixed-point requirement
> (critical-qa 2026-06-12) is designed here, implementable off-VM first.

## Verified API surfaces (read 2026-06-12)

### ckb-vm 0.24.14 — the host-side VM (what our node embeds to run type-scripts)
- `ckb_vm::run::<R, M>(program: &Bytes, args: &[Bytes], memory_size: usize) -> Result<i8, Error>`
  — simplest entry; i8 exit code, 0 = success. (`src/lib.rs:41`)
- Default machine recipe used by `run` itself (`src/lib.rs`):
  `DefaultCoreMachine::<R, WXorXMemory<M>>::new_with_memory(ISA_IMC | ISA_A | ISA_B | ISA_MOP, machine::VERSION2, u64::MAX, memory_size)`
  wrapped in `TraceMachine::new(DefaultMachineBuilder::new(core).build())`, then
  `machine.load_program(program, args)?; machine.run()`.
- Custom syscalls: `DefaultMachineBuilder::syscall(Box<dyn Syscalls<Inner>>)` (`machine/mod.rs:794`)
  — this is where Noesis-specific syscalls (load standing cell, load provenance edges) plug in.
- Cycle metering: `DefaultMachineBuilder::instruction_cycle_func` (`machine/mod.rs:786`);
  max_cycles is a constructor param. Memory backends exported: `FlatMemory`, `SparseMemory`,
  `WXorXMemory` (W^X enforcement).

### ckb-std 0.16.4 — the in-script side (no_std, what the type-script program itself uses)
- `high_level::load_cell_data(index, Source) -> Result<Vec<u8>, SysError>` (`high_level.rs:487`)
- `high_level::load_script() -> Result<Script, SysError>` (`:498`) — gives the type-script
  its own `args` (= contributor identity in our cell model).
- `load_witness_args` (`:179`), `load_cell_data_hash` (`:262`).
- Module layout: `entry.rs` (entry! macro), `global_alloc_macro/` (default_alloc!),
  `syscalls/`, `since.rs`, `type_id.rs`. Matches the proven vibeswap build recipe
  [F·ckb-cell-build-recipe]: ws features `ckb-types`+`allocator`, blake2b-ref for hashing.

### ckb-script 1.1.0 — how consensus actually invokes scripts (reference for our chain fork)
- `TransactionScriptsVerifier::new(rtx: Arc<ResolvedTransaction>, data_loader: DL, consensus: Arc<Consensus>, tx_env: Arc<TxVerifyEnv>)`
  where `DL: CellDataProvider + HeaderProvider + ExtensionProvider + ...` (`verify.rs:55-86`)
- `.verify(max_cycles: Cycle) -> Result<Cycle, Error>` (`verify.rs:197`).

## What runs on-VM vs off (the authority split)
| Piece | Where | Why |
|---|---|---|
| `soulbound::valid_transition[_under_dispute]` | ON-VM type-script | per-cell transition invariant — exactly type-script shaped |
| temporal-novelty + similarity floor + semantic floor (intake `production_value`) | ON-VM | per-transaction, content-local, deterministic |
| flow/eigenvector (v5–v7 settlement), dispute verdicts, tribunal | OFF-VM (consensus layer) | graph-global state; scripts see one tx, not the graph |
| learned outcome model | OFF-VM, role-bounded (`evaluator`) | already bounded to advance-timing + evidence, never mint |

## Fixed-point conversion map (the f64 problem, per-function)
- `temporal_novelty` — already u64. No change.
- similarity floor (Jaccard vs theta) — compare `|A∩B| / |A∪B| ≥ θ` as cross-multiplied
  integers: `inter * DEN ≥ THETA_NUM * union`. Exact, no floats.
- `semantic::normalized_entropy ≥ θ` — Shannon needs log2. Plan: integer comparison via a
  fixed Q16.16 `log2` lookup over byte counts (256 entries max), or equivalently compare
  `Σ c·log2(c)` against a θ-derived bound. Deterministic, bounded table. (Design only —
  needs an exactness argument at the θ boundary; flag for its own adversarial tick.)
- `flow_gate f/(f+half) — only ever consumed as comparisons/multipliers; keep flow itself
  in Q32.32 and the gate as a rational pair.
- eigenvector flow — fixed iteration count (already `iters` param) over Q32.32 with
  saturating ops ⇒ deterministic across platforms. Convergence tolerance becomes an exact
  integer epsilon.

## Next code increments (in order, each tested)
1. **Fixed-point `value` mirror OFF-VM first**: `value_fixed` module mirroring intake
   `production_value` in pure integer math + equivalence tests vs the f64 form over the
   existing corpora (exact agreement on all current fixtures, divergence bounded on random
   inputs). No new deps.
2. **Host harness**: add `ckb-vm = "0.24"` dev-dep; run a precompiled trivial RISC-V ELF
   via `ckb_vm::run` as a smoke test. BLOCKER to note honestly: building our own ELF needs
   the `riscv64imac-unknown-none-elf` target + ckb-std scaffold (proven recipe exists in
   vibeswap `contracts-ckb`, [J·vibeswap-ckb-chain-alive]).
3. **The PoM type-script crate** (separate `scripts/pom-typescript/` workspace member,
   no_std + ckb-std 0.16): intake floors + soulbound transition, compiled to RISC-V,
   executed under the host harness with Noesis syscalls.
