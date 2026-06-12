//! The Noesis PoM type-script (onchain/pom-typescript, no_std, riscv64imac) validated
//! END TO END under the host harness — CKB-VM-PORT.md increment #3 complete: our own
//! mechanism code runs INSIDE the VM against an environment served from our Cell model.
//!
//! Fixture: tests/fixtures/pom-typescript — rebuild with
//!   cd onchain/pom-typescript && cargo build --release --target riscv64imac-unknown-none-elf
//!
//! Exit-code contract (src/main.rs): 0 pass · 11 empty soulbound args · 13 semantic floor.

mod common;

use common::{input_cell, run_typescript};
use noesis::value_fixed;

const ELF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pom-typescript");
const THETA_ENT_Q16: u64 = 62259;

#[test]
fn content_cell_passes_the_on_vm_intake_floor() {
    let cell = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    let (result, served) = run_typescript(ELF, &cell.clone(), vec![cell]);
    assert!(served >= 2, "script must consume load_script + load_cell_data (got {served})");
    assert_eq!(result.unwrap(), 0, "structured content passes on-VM");
}

#[test]
fn noise_cell_is_floored_on_vm_exit_13() {
    let noise: Vec<u8> = (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
    let cell = input_cell(0, 9, &noise);
    let (result, _) = run_typescript(ELF, &cell.clone(), vec![cell]);
    assert_eq!(result.unwrap(), 13, "the semantic floor fires INSIDE the VM");
}

#[test]
fn missing_soulbound_identity_is_rejected_on_vm_exit_11() {
    let mut cell = input_cell(0, 7, b"alpha-bravo-charlie-delta");
    cell.type_script.args = vec![]; // no contributor identity
    let (result, _) = run_typescript(ELF, &cell.clone(), vec![cell]);
    assert_eq!(result.unwrap(), 11, "soulbound identity is not optional on-VM");
}

#[test]
fn on_vm_verdicts_agree_with_the_host_side_value_fixed_mirror() {
    // The same Q16.16 arithmetic must produce the same verdict on both sides of the VM
    // boundary — this is the determinism claim the fixed-point port exists for.
    let payloads: Vec<Vec<u8>> = vec![
        b"alpha-bravo-charlie-delta".to_vec(),
        b"the-quick-brown-fox-says-nothing-of-value-today".to_vec(),
        (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect(),
        (0u8..32).map(|i| i.wrapping_mul(67).wrapping_add(29)).collect(), // keyish airgap
        (0u8..64).map(|i| i.wrapping_mul(37).wrapping_add(11)).flat_map(|b| format!("{b:02x}").into_bytes()).collect(), // hexed evasion
    ];
    for p in payloads {
        let cell = input_cell(0, 7, &p);
        let (result, _) = run_typescript(ELF, &cell.clone(), vec![cell]);
        let on_vm_floored = result.unwrap() == 13;
        let host_floored = value_fixed::is_incompressible_q16(&p, THETA_ENT_Q16);
        assert_eq!(on_vm_floored, host_floored, "VM and host disagree on {:?}…", &p[..8.min(p.len())]);
    }
}
