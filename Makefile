.PHONY: all build build-wasm test test-all clean lint check

# Default target
all: build test

# Build the workspace (debug)
build:
	cargo build

# Build for WebAssembly (Soroban contract target)
# Uses Rust 1.81 or earlier with wasm32-unknown-unknown, or Rust 1.84+ with wasm32v1-none
# The `stellar contract build` CLI handles toolchain selection automatically
build-wasm:
	stellar contract build -p identity-registry -p reference-defi-pool 2>/dev/null || \
		echo "stellar CLI not found. Install with: cargo install stellar-cli"
	@ls -lh target/wasm32-unknown-unknown/release/*.wasm 2>/dev/null || \
		echo "No WASM artifacts found. Use: stellar contract build --wasm <CRATE>"

# Build with wasm32v1-none target directly (requires Rust 1.84+)
build-wasm-raw:
	cargo build --target wasm32v1-none --release -p identity-registry -p reference-defi-pool

# Run library tests (without testutils feature - works in all environments)
test:
	cargo test -p soroban-compliance-kit --test property_tests

# Run all tests including integration tests (requires testutils feature)
test-all:
	cargo test --features testutils -p soroban-compliance-kit -p identity-registry -p reference-defi-pool

# Run tests for a specific crate
test-kit:
	cargo test --features testutils -p soroban-compliance-kit --test property_tests

test-registry:
	cargo test --features testutils -p identity-registry

test-pool:
	cargo test --features testutils -p reference-defi-pool

# Clean build artifacts
clean:
	cargo clean

# Check for compilation errors
check:
	cargo check --workspace

# Lint with clippy
lint:
	cargo clippy --workspace -- -D warnings

# Deploy contracts to a local/test network (requires Soroban CLI)
deploy-registry:
	soroban contract deploy \
		--wasm target/wasm32-unknown-unknown/release/identity_registry.wasm

deploy-pool:
	soroban contract deploy \
		--wasm target/wasm32-unknown-unknown/release/reference_defi_pool.wasm

# Full deploy pipeline
deploy: build-wasm deploy-registry deploy-pool

# Print dependency tree
deps:
	cargo tree

# Help
help:
	@echo "Targets:"
	@echo "  build        - Build workspace (debug)"
	@echo "  build-wasm   - Build contracts for WASM target"
	@echo "  test         - Run library property tests"
	@echo "  test-all     - Run all tests (with testutils)"
	@echo "  test-kit     - Run compliance-kit tests"
	@echo "  test-registry- Run identity-registry tests"
	@echo "  test-pool    - Run defi-pool tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  deploy       - WASM build + deploy contracts"
	@echo "  lint         - Run clippy"
	@echo "  deps         - Show dependency tree"
