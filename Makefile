.PHONY: all build build-wasm test test-all test-kit test-registry test-pool \
        clean check lint fmt fmt-check doc coverage deploy init help

# Default: everything a reviewer should run to validate the workspace.
all: check lint test

# Build the workspace (debug).
build:
	cargo build --workspace

# Build the contracts to WebAssembly for the Soroban runtime.
# The wasm32v1-none target is pinned in rust-toolchain.toml.
build-wasm:
	cargo build --target wasm32v1-none --release -p identity-registry -p reference-defi-pool
	@ls -lh target/wasm32v1-none/release/*.wasm

# Run the full test suite (unit + integration + property tests).
test:
	cargo test --workspace --features testutils

test-all: test

# Run tests for a single crate.
test-kit:
	cargo test -p soroban-compliance-kit --features testutils

test-registry:
	cargo test -p identity-registry --features testutils

test-pool:
	cargo test -p reference-defi-pool --features testutils

# Clean build artifacts.
clean:
	cargo clean

# Compile-check everything (including tests and examples).
check:
	cargo check --workspace --all-targets

# Lint with clippy; warnings are treated as errors.
# `--features testutils` ensures the cfg-gated test modules are linted too.
lint:
	cargo clippy --workspace --all-targets --features testutils -- -D warnings

# Format the whole workspace.
fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

# Build API documentation.
doc:
	cargo doc --workspace --no-deps

# Line coverage with a gate (requires: cargo install cargo-llvm-cov).
coverage:
	cargo llvm-cov --workspace --features testutils --fail-under-lines 80

# Deployment helpers (require the stellar CLI, see scripts/ and .env.example).
deploy:
	./scripts/deploy.sh

init:
	./scripts/init.sh

help:
	@echo "Targets:"
	@echo "  all           - check + lint + test (default)"
	@echo "  build         - cargo build (debug)"
	@echo "  build-wasm    - build contracts for wasm32v1-none"
	@echo "  test          - full test suite (workspace, testutils)"
	@echo "  test-kit      - soroban-compliance-kit tests"
	@echo "  test-registry - identity-registry tests"
	@echo "  test-pool     - reference-defi-pool tests"
	@echo "  check         - cargo check (all targets)"
	@echo "  lint          - clippy with -D warnings"
	@echo "  fmt / fmt-check - format / verify formatting"
	@echo "  doc           - build API docs"
	@echo "  coverage      - llvm-cov with a 80% line gate"
	@echo "  deploy / init - run scripts/deploy.sh / scripts/init.sh"
	@echo "  clean         - cargo clean"
