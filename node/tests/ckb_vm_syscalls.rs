//! Noesis syscalls behind CKB-VM — CKB-VM-PORT.md increment #3, first half.
//!
//! The smoke test proved a real RISC-V type-script EXECUTES under `ckb_vm::run`; this
//! file gives the VM an environment: the CKB syscall ABI served from OUR Cell model.
//! Everything here is implemented against source, not assumption:
//!   - `Syscalls` trait: `initialize` + `ecall -> Result<bool>` (false = not mine,
//!     try next handler) — ckb-vm `src/syscalls/mod.rs`.
//!   - ABI: a7 = syscall number; a0 = buf ptr, a1 = ptr to u64 length (read as capacity,
//!     written back as ACTUAL available length), a2 = offset, a3 = index, a4 = source —
//!     ckb-std `src/syscalls/native.rs::syscall_load`.
//!   - Numbers: SYS_LOAD_SCRIPT = 2052, SYS_LOAD_CELL_DATA = 2092 — ckb-std
//!     `src/ckb_constants.rs`. Source::Input = 1, GroupInput = 0x0100000000000001.
//!   - `load_script` consumers parse MOLECULE bytes, so the host serves a real molecule
//!     `Script` table (hand-encoded below: 16-byte header, code_hash[32], hash_type u8,
//!     args fixvec), letting a stock ckb-std program proceed legitimately.

use ckb_vm::machine::{CoreMachine, DefaultCoreMachine, DefaultMachineBuilder, SupportMachine, VERSION2};
use ckb_vm::memory::{sparse::SparseMemory, wxorx::WXorXMemory, Memory};
use ckb_vm::registers::{A0, A1, A2, A3, A4, A7};
use ckb_vm::{Bytes, Error, Register, Syscalls, ISA_A, ISA_B, ISA_IMC, ISA_MOP};
use noesis::{Cell, Script};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const SYS_LOAD_SCRIPT: u64 = 2052;
const SYS_LOAD_CELL_DATA: u64 = 2092;
const INDEX_OUT_OF_BOUND: u64 = 1;
const SOURCE_INPUT_LOW: u64 = 1;

/// Hand-encoded molecule `Script` table (3 fields: code_hash, hash_type, args).
/// Layout per molecule spec: u32 total size, 3 × u32 field offsets, then the fields;
/// `args` is a fixvec (u32 length + bytes).
fn molecule_script(code_hash: &[u8; 32], hash_type: u8, args: &[u8]) -> Vec<u8> {
    let header = 16u32;
    let o0 = header;
    let o1 = o0 + 32;
    let o2 = o1 + 1;
    let total = o2 + 4 + args.len() as u32;
    let mut out = Vec::with_capacity(total as usize);
    for v in [total, o0, o1, o2] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(code_hash);
    out.push(hash_type);
    out.extend_from_slice(&(args.len() as u32).to_le_bytes());
    out.extend_from_slice(args);
    out
}

/// The Noesis data source: serves the executing script + input-cell data from our model.
struct NoesisSyscalls {
    script: Vec<u8>,
    inputs: Vec<Cell>,
    served: Arc<AtomicU64>,
}

impl NoesisSyscalls {
    fn for_cell(cell: &Cell, inputs: Vec<Cell>, served: Arc<AtomicU64>) -> Self {
        let code_hash: [u8; 32] = cell.type_script.code_hash;
        NoesisSyscalls {
            script: molecule_script(&code_hash, 0, &cell.type_script.args),
            inputs,
            served,
        }
    }

    /// CKB partial-load protocol: copy min(capacity, avail) from `data[offset..]`,
    /// write back the FULL available length, return 0 in a0.
    fn serve<Mac: SupportMachine>(&self, machine: &mut Mac, data: &[u8]) -> Result<bool, Error> {
        let addr = machine.registers()[A0].to_u64();
        let len_ptr = machine.registers()[A1].clone();
        let offset = machine.registers()[A2].to_u64() as usize;
        let capacity = machine.memory_mut().load64(&len_ptr)?.to_u64() as usize;
        let avail = data.len().saturating_sub(offset);
        let copy = avail.min(capacity);
        if copy > 0 {
            machine.memory_mut().store_bytes(addr, &data[offset..offset + copy])?;
        }
        machine
            .memory_mut()
            .store64(&len_ptr, &Mac::REG::from_u64(avail as u64))?;
        machine.set_register(A0, Mac::REG::from_u64(0));
        self.served.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for NoesisSyscalls {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        match machine.registers()[A7].to_u64() {
            SYS_LOAD_SCRIPT => {
                let script = self.script.clone();
                self.serve(machine, &script)
            }
            SYS_LOAD_CELL_DATA => {
                let index = machine.registers()[A3].to_u64() as usize;
                let source = machine.registers()[A4].to_u64();
                // This harness serves INPUT cells (plain or group-scoped).
                if source & 0xFF == SOURCE_INPUT_LOW {
                    if let Some(cell) = self.inputs.get(index) {
                        let data = cell.data.clone();
                        return self.serve(machine, &data);
                    }
                }
                machine.set_register(A0, Mac::REG::from_u64(INDEX_OUT_OF_BOUND));
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn input_cell(id: u64, contrib: u8, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [1u8; 32], args: vec![contrib] },
        type_script: Script { code_hash: [0xB0; 32], args: vec![contrib] },
        parent: None,
        timestamp: id,
        data: data.to_vec(),
    }
}

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
