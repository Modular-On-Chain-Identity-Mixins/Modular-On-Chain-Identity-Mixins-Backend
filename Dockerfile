# syntax=docker/dockerfile:1
#
# Development/CI image for the Modular On-Chain Identity Mixins workspace.
#
#   docker compose run --rm dev        # run `make check` in the pinned toolchain
#   docker compose run --rm dev test   # run `make test`
#
# The builder stage runs the full quality gate (fmt, clippy, tests) and
# produces the deployable WASM artifacts.

FROM rust:1.97-slim AS builder

WORKDIR /workspace

# Base tooling needed by the Makefile. The rust:1.97-slim image ships the
# pinned toolchain; add the Soroban WASM target and make.
RUN apt-get update \
    && apt-get install -y --no-install-recommends make ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt clippy \
    && rustup target add wasm32v1-none

COPY . .

# Quality gate: format, lint (incl. test modules), full test suite, docs,
# then the WASM artifacts. Mirrors the build/lint/test/doc jobs of
# .github/workflows/ci.yml (coverage and audit run in CI only — they need
# cargo-llvm-cov / cargo-audit, which are not installed in this image).
# `--locked` keeps the build reproducible against the committed Cargo.lock.
RUN cargo fmt --all -- --check \
    && cargo clippy --workspace --all-targets --features testutils -- -D warnings \
    && cargo test --locked --workspace \
    && cargo test --locked --workspace --features testutils \
    && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps \
    && cargo build --locked --target wasm32v1-none --release -p identity-registry -p reference-defi-pool

# Minimal stage containing only the deployable contracts.
FROM scratch AS wasm
COPY --from=builder /workspace/target/wasm32v1-none/release/identity_registry.wasm /wasm/
COPY --from=builder /workspace/target/wasm32v1-none/release/reference_defi_pool.wasm /wasm/

# Default dev target: re-runs the check gate with the caller's sources mounted.
FROM builder AS dev
ENTRYPOINT ["make"]
CMD ["check"]
