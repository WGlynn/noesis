//! Shared CKB-VM host harness for integration tests: the Noesis syscall
//! environment (Cell model behind the VM) + an end-to-end runner. Extracted from
//! ckb_vm_syscalls.rs; the ABI provenance notes live there.
#![allow(dead_code)]

use ckb_vm::machine::{CoreMachine, DefaultCoreMachine, DefaultMachineBuilder, SupportMachine, VERSION2};
use ckb_vm::memory::{sparse::SparseMemory, wxorx::WXorXMemory, Memory};
use ckb_vm::registers::{A0, A1, A2, A3, A4, A7};
use ckb_vm::{Bytes, Error, Register, Syscalls, ISA_A, ISA_B, ISA_IMC, ISA_MOP};
use noesis::{Cell, Script};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub const SYS_LOAD_SCRIPT: u64 = 2052;
pub const SYS_LOAD_CELL_DATA: u64 = 2092;
pub const INDEX_OUT_OF_BOUND: u64 = 1;
pub const SOURCE_INPUT_LOW: u64 = 1;

/// Hand-encoded molecule `Script` table (3 fields: code_hash, hash_type, args).
/// Layout per molecule spec: u32 total size, 3 × u32 field offsets, then the fields;
/// `args` is a fixvec (u32 length + bytes).
pub fn molecule_script(code_hash: &[u8; 32], hash_type: u8, args: &[u8]) -> Vec<u8> {
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
pub struct NoesisSyscalls {
    #[allow(dead_code)]
    pub script: Vec<u8>,
    pub inputs: Vec<Cell>,
    pub served: Arc<AtomicU64>,
}

impl NoesisSyscalls {
    pub fn for_cell(cell: &Cell, inputs: Vec<Cell>, served: Arc<AtomicU64>) -> Self {
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

pub fn input_cell(id: u64, contrib: u8, data: &[u8]) -> Cell {
    Cell {
        id,
        lock: Script { code_hash: [1u8; 32], args: vec![contrib] },
        type_script: Script { code_hash: [0xB0; 32], args: vec![contrib] },
        parent: None,
        timestamp: id,
        data: data.to_vec(),
    }
}


/// Load an ELF and run it inside a full machine with the Noesis environment bound.
/// Returns the VM result and how many syscalls the program actually consumed.
pub fn run_typescript(
    elf_path: &str,
    script_cell: &Cell,
    inputs: Vec<Cell>,
) -> (Result<i8, Error>, u64) {
    let program = Bytes::from(std::fs::read(elf_path).expect("fixture ELF present"));
    let served = Arc::new(AtomicU64::new(0));
    let handler = NoesisSyscalls::for_cell(script_cell, inputs, served.clone());
    let core = DefaultCoreMachine::<u64, WXorXMemory<SparseMemory<u64>>>::new_with_memory(
        ISA_IMC | ISA_A | ISA_B | ISA_MOP,
        VERSION2,
        u64::MAX,
        ckb_vm::RISCV_MAX_MEMORY,
    );
    let mut machine = DefaultMachineBuilder::new(core).syscall(Box::new(handler)).build();
    machine
        .load_program(&program, [].iter().map(|b: &Bytes| Ok(b.clone())))
        .unwrap();
    (machine.run(), served.load(std::sync::atomic::Ordering::SeqCst))
}
