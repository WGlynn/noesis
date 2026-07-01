// risc0_build writes the ELF bytes + image id into OUT_DIR/methods.rs at build time.
// Downstream (the host) uses `ZK_FINALIZE_ELF` and `ZK_FINALIZE_ID` from here.
include!(concat!(env!("OUT_DIR"), "/methods.rs"));
