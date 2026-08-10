#!/usr/bin/env bash
#
# Initialize the deployed contracts (constructors, admin, compliance limits).
#
# Run this AFTER scripts/deploy.sh. Required environment variables
# (see .env.example):
#
#   IDENTITY_REGISTRY_ID        contract id from deploy.sh
#   POOL_ID                     contract id from deploy.sh
#   TOKEN_ID                    asset contract id to wrap in the pool
#   IDENTITY_REGISTRY_ADMIN     G... public key that owns the registry
#   POOL_ADMIN                  G... public key that owns the pool
#   REQUIRED_TIER / DAILY_VOLUME_LIMIT / MONTHLY_VOLUME_LIMIT / RESTRICTED_JURISDICTIONS
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

echo "=== Initializing Identity Registry ==="
"$CLI" contract invoke \
    "${NETWORK_ARGS[@]}" \
    "${SOURCE_ARGS[@]}" \
    --id "${IDENTITY_REGISTRY_ID:?IDENTITY_REGISTRY_ID not set}" \
    -- \
    __constructor \
    --admin "${IDENTITY_REGISTRY_ADMIN:?IDENTITY_REGISTRY_ADMIN not set}"
echo "Identity Registry initialized."

echo ""
echo "=== Initializing Reference DeFi Pool ==="
"$CLI" contract invoke \
    "${NETWORK_ARGS[@]}" \
    "${SOURCE_ARGS[@]}" \
    --id "${POOL_ID:?POOL_ID not set}" \
    -- \
    __constructor \
    --token "${TOKEN_ID:?TOKEN_ID not set}" \
    --admin "${POOL_ADMIN:?POOL_ADMIN not set}" \
    --identity-registry "${IDENTITY_REGISTRY_ID}" \
    --required-tier "${REQUIRED_TIER:-1}" \
    --daily-volume-limit "${DAILY_VOLUME_LIMIT:-1000000000}" \
    --monthly-volume-limit "${MONTHLY_VOLUME_LIMIT:-10000000000}" \
    --restricted-jurisdictions "${RESTRICTED_JURISDICTIONS:-[]}"
echo "Reference DeFi Pool initialized."

echo ""
echo "Next: authorize the pool as a volume caller of the registry:"
echo "  $CLI contract invoke --id \"\$IDENTITY_REGISTRY_ID\" \\"
echo "    add_authorized_caller --caller \"\$POOL_ID\""
