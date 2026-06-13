//! Adversarial tick on the VM arc (iterations 4-8) — method-standard: every new layer
//! meets the adversary the moment it lands. Targets: the syscall host's register-driven
//! attack surface, runaway-script economics, and the on-VM program's honest scope gaps.

mod common;

use ckb_vm::machine::{CoreMachine, DefaultCoreMachine, SupportMachine, VERSION2};
use ckb_vm::memory::{sparse::SparseMemory, Memory};
use ckb_vm::registers::{A0, A1, A2, A3, A4, A7};
use ckb_vm::{Error, Syscalls, ISA_IMC};
use common::{input_cell, run_typescript, run_typescript_metered, NoesisSyscalls, SYS_LOAD_CELL_DATA};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

const ELF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pom-typescript");

#[test]
fn hostile_register_values_cannot_break_the_host() {
    // The guest controls a0-a4 completely. Three hostile shapes against serve():
    // capacity = u64::MAX (grab everything), offset beyond the data (read past end),
    // and the two combined (wrap attempt on offset+copy). The host must stay
    // protocol-correct and never panic or over-copy.
    let cell = input_cell(0, 7, b"alpha-bravo-charlie-delta"); // 25 bytes
    let served = Arc::new(AtomicU64::new(0));
    let mut h = NoesisSyscalls::for_cell(&cell.clone(), vec![cell], served);
    let mut m =
        DefaultCoreMachine::<u64, SparseMemory<u64>>::new_with_memory(ISA_IMC, VERSION2, u64::MAX, 1 << 20);

    let mut call = |capacity: u64, offset: u64| -> (u64, u64) {
        m.memory_mut().store64(&0x2000, &capacity).unwrap();
        m.set_register(A0, 0x1000);
        m.set_register(A1, 0x2000);
        m.set_register(A2, offset);
        m.set_register(A3, 0);
        m.set_register(A4, 1);
        m.set_register(A7, SYS_LOAD_CELL_DATA);
        Syscalls::<DefaultCoreMachine<u64, SparseMemory<u64>>>::ecall(&mut h, &mut m).unwrap();
        (m.registers()[A0], m.memory_mut().load64(&0x2000).unwrap())
    };

    let (ret, len) = call(u64::MAX, 0);
    assert_eq!((ret, len), (0, 25), "huge capacity: copy bounded by the data, not the ask");

    let (ret, len) = call(64, 1_000_000);
    assert_eq!((ret, len), (0, 0), "offset past end: zero available, protocol-correct");

    let (ret, len) = call(u64::MAX, u64::MAX);
    assert_eq!((ret, len), (0, 0), "offset+capacity wrap attempt: saturating math holds");
}

#[test]
fn cycle_metering_bounds_runaway_scripts() {
    // The unmetered harness runs at max_cycles = u64::MAX — fine for protocol tests,
    // NOT a production posture: a loop{} script would hang the host. This proves the
    // defense exists and engages: same ELF, real estimate_cycles, starvation budget.
    let cell = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let (result, _) = run_typescript_metered(ELF, &cell.clone(), vec![cell.clone()], 500);
    match result {
        Err(Error::CyclesExceeded) => {} // the budget bit
        other => panic!("expected CyclesExceeded at 500 cycles, got {other:?}"),
    }
    // And with a sane budget the same program still completes (metering ≠ breakage).
    let (result, _) = run_typescript_metered(ELF, &cell.clone(), vec![cell], 50_000_000);
    assert_eq!(result.unwrap(), 0, "metered run completes within a realistic budget");
}

#[test]
fn hexed_noise_passes_on_vm_too_open_gap() {
    // PINNED (inherited, not new): the encoding-evasion gap
    // (`encoded_noise_evades_the_entropy_floor_open_gap`) crosses the VM boundary intact —
    // hex-encoded garbage exits 0 on-VM. Same containment as host-side: structured-but-
    // valueless is the out-of-band frontier; economic layers stay the binding defense.
    let raw: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
    let hexed: Vec<u8> = raw.iter().flat_map(|b| format!("{b:02x}").into_bytes()).collect();
    let cell = input_cell(0, 9, &hexed);
    let (result, _) = run_typescript(ELF, &cell.clone(), vec![cell]);
    assert_eq!(result.unwrap(), 0, "OPEN GAP: encoded noise passes the on-VM floor too");
}

#[test]
fn on_vm_floor_now_checks_every_group_input_gap_closed() {
    // FLIPPED (roadmap-advance increment, same day the tick pinned it): the program now
    // iterates its whole script group until INDEX_OUT_OF_BOUND. The EXACT smuggling tx
    // that passed before — noise at input index 1 — is floored at 13.
    let content = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
    let smuggled = input_cell(1, 7, &noise);
    let (result, _) = run_typescript(ELF, &content.clone(), vec![content, smuggled]);
    assert_eq!(result.unwrap(), 13, "smuggled noise at index 1 is now floored on-VM");
}

#[test]
fn multi_input_all_content_group_still_passes() {
    // Regression for the fix: iterating the group must not break the honest path —
    // three content cells, exit 0; and an EMPTY group is rejected (nothing to attest).
    let a = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let b = input_cell(1, 7, b"echo-foxtrot-golf-hotel");
    let c = input_cell(2, 7, b"india-juliet-kilo-lima");
    let (result, served) = run_typescript(ELF, &a.clone(), vec![a.clone(), b, c]);
    assert_eq!(result.unwrap(), 0, "honest multi-input group passes");
    assert!(served >= 4, "script + 3 cells + the out-of-bound probe all served");
    let (result, _) = run_typescript(ELF, &a, vec![]);
    assert_eq!(result.unwrap(), 12, "empty group rejected — nothing to attest");
}

#[test]
fn mint_side_noise_is_floored_t6_closed() {
    // FLIPS the T6 doc-pin: a tx MINTING noise (clean inputs, noise in the produced
    // group outputs) was invisible to the program before the output loop. Now: 14.
    let consumed = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let minted_ok = input_cell(10, 7, b"echo-foxtrot-golf-hotel");
    let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
    let minted_noise = input_cell(11, 7, &noise);
    // (T7 update: mints now also need the index dep + proof witnesses; the semantic
    // floor still fires FIRST, so the noise case needs no proofs at all.)
    let (result, _) = common::run_typescript_tx(
        ELF,
        &consumed.clone(),
        vec![consumed.clone()],
        vec![minted_noise, minted_ok.clone()],
    );
    assert_eq!(result.unwrap(), 14, "mint-side noise floored with its own exit code");
    // Honest mint passes WITH proofs; mint-only (genesis-ish) likewise.
    let idx = noesis::smt::NoveltyIndex::new();
    let blob = common::proof_blob(&idx, &minted_ok.data);
    let (result, _) = common::run_typescript_t7(
        ELF,
        &consumed.clone(),
        vec![consumed],
        vec![minted_ok.clone()],
        vec![common::root_dep(&idx)],
        vec![blob.clone()],
    );
    assert_eq!(result.unwrap(), 0, "honest proven mint passes both loops");
    let (result, _) = common::run_typescript_t7(
        ELF,
        &minted_ok.clone(),
        vec![],
        vec![minted_ok],
        vec![common::root_dep(&idx)],
        vec![blob],
    );
    assert_eq!(result.unwrap(), 0, "mint-only group is valid, floored, and proven");
}
