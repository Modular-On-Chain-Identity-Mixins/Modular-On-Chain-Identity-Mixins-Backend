#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/.."

# Source .env if present
if [ -f "$PROJECT_DIR/.env" ]; then
    source "$PROJECT_DIR/.env"
fi

# Check prerequisites
command -v soroban >/dev/null 2>&1 || { echo "Error: soroban CLI not found. Install it with: cargo install soroban-cli"; exit 1; }

NETWORK_ARGS=()
if [ -n "${SOROBAN_RPC_URL:-}" ]; then
    NETWORK_ARGS+=(--rpc-url "$SOROBAN_RPC_URL")
fi
if [ -n "${SOROBAN_NETWORK_PASSPHRASE:-}" ]; then
    NETWORK_ARGS+=(--network-passphrase "$SOROBAN_NETWORK_PASSPHRASE")
fi
if [ -n "${SOROBAN_SECRET_KEY:-}" ]; then
    NETWORK_ARGS+=(--secret-key "$SOROBAN_SECRET_KEY")
fi

echo "=== Building WASM contracts ==="
cargo build --target wasm32-unknown-unknown --release -p identity-registry -p reference-defi-pool

echo ""
echo "=== Deploying Identity Registry ==="
REGISTRY_ID=$(soroban contract deploy \
    "${NETWORK_ARGS[@]}" \
    --wasm "$PROJECT_DIR/target/wasm32-unknown-unknown/release/identity_registry.wasm" \
)
echo "Identity Registry deployed at: $REGISTRY_ID"

echo ""
echo "=== Deploying Reference DeFi Pool ==="
POOL_ID=$(soroban contract deploy \
    "${NETWORK_ARGS[@]}" \
    --wasm "$PROJECT_DIR/target/wasm32-unknown-unknown/release/reference_defi_pool.wasm" \
)
echo "Reference DeFi Pool deployed at: $POOL_ID"

echo ""
echo "=== Deployment Summary ==="
echo "Identity Registry: $REGISTRY_ID"
echo "Reference DeFi Pool: $POOL_ID"

echo ""
echo "Next steps:"
echo "  1. Initialize the identity registry with:"
echo "     soroban contract invoke --id $REGISTRY_ID -- init --admin <ADMIN_ADDRESS>"
echo ""
echo "  2. Deploy a Stellar Asset Contract and initialize the pool with:"
echo "     soroban contract invoke --id $POOL_ID -- __constructor \\"
echo "       --token <TOKEN_ID> --admin <ADMIN_ADDRESS> \\"
echo "       --identity-registry $REGISTRY_ID \\"
echo "       --required-tier 1 \\"
echo "       --daily-volume-limit 1000000000 \\"
echo "       --monthly-volume-limit 10000000000"
