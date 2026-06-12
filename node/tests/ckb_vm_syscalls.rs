//! Register-level tests of the Noesis syscall host (tests/common/mod.rs).
//! ABI provenance (all read from source, not assumed):
//!   - `Syscalls` trait: ckb-vm `src/syscalls/mod.rs` (`ecall -> Result<bool>`,
//!     false = pass to next handler).
//!   - a7 = syscall number; a0 buf, a1 len-ptr (capacity in, actual length out),
//!     a2 offset, a3 index, a4 source — ckb-std `src/syscalls/native.rs::syscall_load`.
//!   - SYS_LOAD_SCRIPT = 2052, SYS_LOAD_CELL_DATA = 2092 — ckb-std `src/ckb_constants.rs`.
//!   - molecule Script table layout hand-encoded and invariant-checked in-test.

mod common;

use ckb_vm::machine::{CoreMachine, DefaultCoreMachine, DefaultMachineBuilder, SupportMachine, VERSION2};
use ckb_vm::memory::{sparse::SparseMemory, wxorx::WXorXMemory, Memory};
use ckb_vm::registers::{A0, A1, A2, A3, A4, A7};
use ckb_vm::{Bytes, Error, Syscalls, ISA_A, ISA_B, ISA_IMC, ISA_MOP};
use common::{input_cell, NoesisSyscalls, INDEX_OUT_OF_BOUND, SYS_LOAD_CELL_DATA, SYS_LOAD_SCRIPT};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

type UnitMachine = DefaultCoreMachine<u64, SparseMemory<u64>>;

fn unit_machine() -> UnitMachine {
    DefaultCoreMachine::new_with_memory(ISA_IMC | ISA_A | ISA_B | ISA_MOP, VERSION2, u64::MAX, 1 << 20)
}

const BUF: u64 = 0x1000;
const LEN_PTR: u64 = 0x2000;

fn call(
    handler: &mut NoesisSyscalls,
    m: &mut UnitMachine,
    syscall: u64,
    capacity: u64,
    offset: u64,
    index: u64,
    source: u64,
) -> bool {
    m.memory_mut().store64(&LEN_PTR, &capacity).unwrap();
    m.set_register(A0, BUF);
    m.set_register(A1, LEN_PTR);
    m.set_register(A2, offset);
    m.set_register(A3, index);
    m.set_register(A4, source);
    m.set_register(A7, syscall);
    Syscalls::<UnitMachine>::ecall(handler, m).unwrap()
}

#[test]
fn load_cell_data_serves_the_cell_model_with_partial_load_semantics() {
    let cell = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let served = Arc::new(AtomicU64::new(0));
    let mut h = NoesisSyscalls::for_cell(&cell.clone(), vec![cell], served.clone());
    let mut m = unit_machine();

    // Full read: capacity ≥ data ⇒ whole payload lands in VM memory, a0 = 0.
    assert!(call(&mut h, &mut m, SYS_LOAD_CELL_DATA, 64, 0, 0, 1));
    assert_eq!(m.registers()[A0], 0, "success return code");
    assert_eq!(m.memory_mut().load64(&LEN_PTR).unwrap(), 25, "actual length written back");
    assert_eq!(
        m.memory_mut().load_bytes(BUF, 25).unwrap().as_ref(),
        b"alpha-bravo-charlie-delta"
    );

    // Partial read: capacity 4, offset 6 ⇒ 4 bytes copied, FULL remaining length (19)
    // written back — the caller learns the true size, per the CKB protocol.
    assert!(call(&mut h, &mut m, SYS_LOAD_CELL_DATA, 4, 6, 0, 1));
    assert_eq!(m.memory_mut().load64(&LEN_PTR).unwrap(), 19);
    assert_eq!(m.memory_mut().load_bytes(BUF, 4).unwrap().as_ref(), b"brav");

    // Out-of-bounds index ⇒ INDEX_OUT_OF_BOUND in a0, still handled (true).
    assert!(call(&mut h, &mut m, SYS_LOAD_CELL_DATA, 64, 0, 9, 1));
    assert_eq!(m.registers()[A0], INDEX_OUT_OF_BOUND);

    // Foreign syscall ⇒ false (pass to the next handler), untouched a0 semantics.
    assert!(!call(&mut h, &mut m, 9999, 64, 0, 0, 1));

    assert_eq!(served.load(Ordering::SeqCst), 2, "exactly the two successful loads served");
}

#[test]
fn load_script_serves_wellformed_molecule_with_the_contributor_args() {
    let cell = input_cell(3, 42, b"payload");
    let served = Arc::new(AtomicU64::new(0));
    let mut h = NoesisSyscalls::for_cell(&cell.clone(), vec![cell], served);
    let mut m = unit_machine();

    assert!(call(&mut h, &mut m, SYS_LOAD_SCRIPT, 128, 0, 0, 0));
    let total = m.memory_mut().load64(&LEN_PTR).unwrap();
    let raw = m.memory_mut().load_bytes(BUF, total).unwrap();
    // Molecule invariants: declared total == served length; offsets are monotone;
    // code_hash and args round-trip from the Cell's soulbound type_script.
    let total_decl = u32::from_le_bytes(raw[0..4].try_into().unwrap()) as u64;
    assert_eq!(total_decl, total, "molecule total size matches served length");
    let o0 = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let o2 = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
    assert_eq!(&raw[o0..o0 + 32], &[0xB0u8; 32], "code_hash round-trips");
    let args_len = u32::from_le_bytes(raw[o2..o2 + 4].try_into().unwrap()) as usize;
    assert_eq!(&raw[o2 + 4..o2 + 4 + args_len], &[42u8], "contributor identity in args");
}

#[test]
fn pom_lock_script_consumes_syscalls_and_progresses_past_bare_vm_state() {
    // The prebuilt vibeswap PoM lock script under a machine WITH our syscalls bound.
    // The honest assertion is the counter: the program must actually CONSUME at least
    // one served syscall (so it demonstrably progressed past where the bare smoke-test
    // machine stopped), and the run must end in program-level territory — a native exit
    // (any code; it is validating a foreign environment) or a further, still-unbound
    // CKB syscall — never a VM-level fault like InvalidInstruction.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/proof-of-mind-lock-script");
    let program = Bytes::from(std::fs::read(path).expect("fixture ELF present"));
    let cell = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let served = Arc::new(AtomicU64::new(0));
    let handler = NoesisSyscalls::for_cell(&cell.clone(), vec![cell], served.clone());

    let core = DefaultCoreMachine::<u64, WXorXMemory<SparseMemory<u64>>>::new_with_memory(
        ISA_IMC | ISA_A | ISA_B | ISA_MOP,
        VERSION2,
        u64::MAX,
        ckb_vm::RISCV_MAX_MEMORY,
    );
    let mut machine = DefaultMachineBuilder::new(core).syscall(Box::new(handler)).build();
    machine.load_program(&program, [].iter().map(|b: &Bytes| Ok(b.clone()))).unwrap();
    let result = machine.run();

    assert!(
        served.load(Ordering::SeqCst) >= 1,
        "the script consumed none of our syscalls — environment not actually exercised"
    );
    match result {
        Ok(code) => println!("script ran to native exit with code {code}"),
        Err(Error::InvalidEcall(code)) => {
            assert_ne!(code, SYS_LOAD_SCRIPT, "load_script is bound; must not resurface");
            assert!(
                (2000..6000).contains(&code),
                "stopped at non-CKB ecall {code} — not program-level territory"
            );
            println!("script progressed to next unbound CKB syscall {code}");
        }
        Err(e) => panic!("VM-level fault, not program-level: {e:?}"),
    }
}
