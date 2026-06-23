# noesis — developer task runner. Host crates build under the workspace; the on-VM
# type-scripts build standalone for RISC-V (their own pinned nightly toolchain).

ELF_TARGET := riscv64imac-unknown-none-elf
SCRIPTS    := pom-typescript finalization-typescript commit-order-typescript locksig-typescript

.PHONY: all test fmt fmt-check clippy elf check docs clean

all: check

## Host test suite (node + noesis-core).
test:
	cargo test

## Format all host code.
fmt:
	cargo fmt --all

## Verify formatting (CI).
fmt-check:
	cargo fmt --all -- --check

## Lint with warnings denied.
clippy:
	cargo clippy --workspace --all-targets -- -D warnings

## Build the RISC-V type-scripts and copy the ELFs into the test fixtures.
elf:
	@for s in $(SCRIPTS); do \
		echo ">> building $$s"; \
		( cd onchain/$$s && cargo build --release --target $(ELF_TARGET) ); \
		cp onchain/$$s/target/$(ELF_TARGET)/release/$$s node/tests/fixtures/$$s; \
	done

## Full local gate (what CI runs): format, lint, test.
check: fmt-check clippy test

## Regenerate the repo-hygiene artifacts.
docs:
	python scripts/doc-coherence.py --stamp
	python scripts/study-guide.py

clean:
	cargo clean
	@for s in $(SCRIPTS); do ( cd onchain/$$s && cargo clean ); done
