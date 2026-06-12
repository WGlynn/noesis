//! CKB-VM host smoke harness — CKB-VM-PORT.md code increment #2.
//!
//! Proves the execution substrate is REAL on this machine: a genuine RISC-V type-script
//! ELF (built from Will's vibeswap `contracts-ckb` work — `proof-of-mind-lock-script`,
//! riscv64imac, ckb-std) is loaded and executed by `ckb_vm::run` (0.24, the same crate
//! the Nervos node embeds).
//!
//! Honest scope of the assertion: with NO syscall handlers registered, a ckb-std program
//! must run its startup code and then stop at its first CKB environment call. Verified
//! against ckb-vm source (`machine/mod.rs:585-604`): ecall 93 exits natively; any other
//! ecall without a handler returns `Error::External`/`InvalidEcall(code)`. Reaching a
//! CKB-numbered ecall therefore PROVES the VM decoded and executed the program's real
//! instructions up to its first intentional environment interaction — exactly what a
//! smoke test should establish. Wiring real syscalls (load_cell_data etc. backed by our
//! Cell model) is increment #3.

use ckb_vm::{run, Bytes, Error, SparseMemory, RISCV_MAX_MEMORY};

#[test]
fn prebuilt_riscv_type_script_loads_and_executes_under_ckb_vm() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/proof-of-mind-lock-script");
    let program = Bytes::from(std::fs::read(path).expect("fixture ELF present in-repo"));
    let result = run::<u64, SparseMemory<u64>>(&program, &[], RISCV_MAX_MEMORY);
    match result {
        // If the program managed to exit on its own, the VM ran it end to end.
        Ok(code) => {
            println!("program ran to native exit, code {code}");
        }
        // Expected path: startup executed, first CKB syscall reached, no handler bound.
        Err(Error::InvalidEcall(code)) => {
            assert!(
                (2000..6000).contains(&code),
                "stopped at ecall {code}, which is not a CKB syscall number — \
                 the program did not reach intentional environment code"
            );
        }
        Err(e) => panic!("VM failed before reaching program logic: {e:?}"),
    }
}
