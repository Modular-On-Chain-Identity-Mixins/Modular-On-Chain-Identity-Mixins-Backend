#!/usr/bin/env bash
#
# Deploy both Soroban contracts to a network.
#
# Requires the Stellar CLI (`stellar`, formerly `soroban`) and a funded
# account. The target network is configured via environment variables
# (see .env.example — copy it to .env and fill in):
#
#   SOROBAN_RPC_URL             RPC endpoint, e.g. https://soroban-testnet.stellar.org
#   SOROBAN_NETWORK_PASSPHRASE  network passphrase, e.g. "Test SDF Network ; September 2015"
#   SOROBAN_SECRET_KEY          secret key (or identity name) of the deploying account
#   WASM_DIR                    override the wasm artifact directory (optional)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/.."
cd "$PROJECT_DIR"

# Load .env if present. Secrets live in .env only — never commit them.
if [ -f "$PROJECT_DIR/.env" ]; then
    # shellcheck disable=SC1091
    source "$PROJECT_DIR/.env"
fi

# --- CLI detection -----------------------------------------------------------
if command -v stellar >/dev/null 2>&1; then
    CLI="stellar"
    SOURCE_ARGS=(--source-account "${SOROBAN_SECRET_KEY:?SOROBAN_SECRET_KEY is required}")
elif command -v soroban >/dev/null 2>&1; then
    CLI="soroban"
    SOURCE_ARGS=(--secret-key "${SOROBAN_SECRET_KEY:?SOROBAN_SECRET_KEY is required}")
else
    echo "Error: neither 'stellar' nor 'soroban' CLI found." >&2
    echo "Install the Stellar CLI with: cargo install stellar-cli --locked" >&2
    exit 1
fi

NETWORK_ARGS=()
[ -n "${SOROBAN_RPC_URL:-}" ] && NETWORK_ARGS+=(--rpc-url "${SOROBAN_RPC_URL}")
[ -n "${SOROBAN_NETWORK_PASSPHRASE:-}" ] && NETWORK_ARGS+=(--network-passphrase "${SOROBAN_NETWORK_PASSPHRASE}")

WASM_DIR="${WASM_DIR:-$PROJECT_DIR/target/wasm32v1-none/release}"

echo "=== Building WASM contracts ==="
cargo build --target wasm32v1-none --release -p identity-registry -p reference-defi-pool

echo ""
echo "=== Deploying Identity Registry ==="
REGISTRY_ID="$("$CLI" contract deploy \
    "${NETWORK_ARGS[@]}" \
    "${SOURCE_ARGS[@]}" \
    --wasm "$WASM_DIR/identity_registry.wasm")"
echo "Identity Registry deployed at: $REGISTRY_ID"

echo ""
echo "=== Deploying Reference DeFi Pool ==="
POOL_ID="$("$CLI" contract deploy \
    "${NETWORK_ARGS[@]}" \
    "${SOURCE_ARGS[@]}" \
    --wasm "$WASM_DIR/reference_defi_pool.wasm")"
echo "Reference DeFi Pool deployed at: $POOL_ID"

echo ""
echo "=== Deployment Summary ==="
echo "Identity Registry: $REGISTRY_ID"
echo "Reference DeFi Pool: $POOL_ID"
echo ""
echo "Next steps:"
echo "  1. Record the IDs in .env (IDENTITY_REGISTRY_ID, POOL_ID, TOKEN_ID,"
echo "     IDENTITY_REGISTRY_ADMIN, POOL_ADMIN) and run: ./scripts/init.sh"
echo "  2. Authorize the pool as a volume caller of the registry:"
echo "     $CLI contract invoke --id \"\$IDENTITY_REGISTRY_ID\" \\"
echo "       add_authorized_caller --caller \"\$POOL_ID\""
