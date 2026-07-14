#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/.."

if [ -f "$PROJECT_DIR/.env" ]; then
    source "$PROJECT_DIR/.env"
fi

command -v soroban >/dev/null 2>&1 || { echo "Error: soroban CLI not found"; exit 1; }

NETWORK_ARGS=()
[ -n "${SOROBAN_RPC_URL:-}" ] && NETWORK_ARGS+=(--rpc-url "$SOROBAN_RPC_URL")
[ -n "${SOROBAN_NETWORK_PASSPHRASE:-}" ] && NETWORK_ARGS+=(--network-passphrase "$SOROBAN_NETWORK_PASSPHRASE")
[ -n "${SOROBAN_SECRET_KEY:-}" ] && NETWORK_ARGS+=(--secret-key "$SOROBAN_SECRET_KEY")

echo "=== Initializing Identity Registry ==="
soroban contract invoke \
    "${NETWORK_ARGS[@]}" \
    --id "${IDENTITY_REGISTRY_ID:?IDENTITY_REGISTRY_ID not set}" \
    -- \
    init \
    --admin "${IDENTITY_REGISTRY_ADMIN:?IDENTITY_REGISTRY_ADMIN not set}"

echo "Identity Registry initialized."

echo ""
echo "=== Initializing Reference DeFi Pool ==="
soroban contract invoke \
    "${NETWORK_ARGS[@]}" \
    --id "${POOL_ID:?POOL_ID not set}" \
    -- \
    __constructor \
    --token "${TOKEN_ID:?TOKEN_ID not set}" \
    --admin "${POOL_ADMIN:?POOL_ADMIN not set}" \
    --identity-registry "${IDENTITY_REGISTRY_ID}" \
    --required-tier "${REQUIRED_TIER:-1}" \
    --daily-volume-limit "${DAILY_VOLUME_LIMIT:-1000000000}" \
    --monthly-volume-limit "${MONTHLY_VOLUME_LIMIT:-10000000000}"

echo "Reference DeFi Pool initialized."
