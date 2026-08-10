# Quickstart — end-to-end usage

This guide walks through a complete flow on **Stellar testnet** using the
`stellar` CLI (the legacy `soroban` CLI works with equivalent flags — see
`scripts/deploy.sh`). All commands assume:

```bash
export IDENTITY_REGISTRY_ID=<from deploy.sh>
export POOL_ID=<from deploy.sh>
export TOKEN_ID=<asset contract id>
export ADMIN=<G... address that owns the contracts>
export ALICE=<G... address of a user you will onboard>
export SIGNER=<your secret key or identity name, e.g. "alice">
```

> **Argument formats** (verified against `soroban-cli` argument parsing):
> enums accept a bare variant name (`Us`, `Verified`); `Vec<Bytes>` and
> contract-type structs use JSON (`'["IR","KP"]'`, `'{...}'`); addresses are
> `G...` strings or identity names.

---

## 1. Deploy & initialize

```bash
# One command each — see scripts/deploy.sh and scripts/init.sh
make deploy
make init
```

Then **authorize the pool** to update volume counters in the registry
(without this, deposits/withdraws/transfers fail with `UnauthorizedCaller`):

```bash
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$SIGNER" --network testnet -- \
  add_authorized_caller \
  --caller "$POOL_ID"
```

---

## 2. Onboard a user (registry)

```bash
# 1. Register Alice with a DID, US jurisdiction, country code "US", tier 1
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$SIGNER" --network testnet -- \
  register \
  --user "$ALICE" \
  --did "did:example:alice" \
  --jurisdiction Us \
  --country-code "US" \
  --tier 1

# 2. Grant a verifier the right to update KYC status (verifier is a G... address)
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$SIGNER" --network testnet -- \
  add_verifier \
  --verifier "$VERIFIER"

# 3. Verify Alice's KYC (only an authorized verifier may do this)
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$VERIFIER" --network testnet -- \
  update_kyc \
  --verifier "$VERIFIER" \
  --user "$ALICE" \
  --status Verified
```

Check the result:

```bash
# Read-only views (no signature needed)
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --network testnet -- \
  get_identity_record --user "$ALICE"

stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --network testnet -- \
  verify --user "$ALICE"          # -> true

stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --network testnet -- \
  get_kyc_status --user "$ALICE"  # -> Verified
```

### Optional identity features

```bash
# Set supported jurisdictions (admin)
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$SIGNER" --network testnet -- \
  set_supported_jurisdictions --jurisdictions '["US","EU"]'

# Attach a custom field (e.g. a risk score) — empty value removes the field
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$SIGNER" --network testnet -- \
  set_custom_field --user "$ALICE" --key "risk_score" --value "low"

stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --network testnet -- \
  get_custom_field --user "$ALICE" --key "risk_score"

# Manage the allow-lists (admin)
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$SIGNER" --network testnet -- \
  remove_verifier --verifier "$VERIFIER"
stellar contract invoke \
  --id "$IDENTITY_REGISTRY_ID" --source-account "$SIGNER" --network testnet -- \
  remove_authorized_caller --caller "$POOL_ID"
```

---

## 3. Use the pool (compliance-enforced)

```bash
# Alice deposits 1,000,000 units — requires her signature (KYC/tier/volume gates apply)
stellar contract invoke \
  --id "$POOL_ID" --source-account "$ALICE" --network testnet -- \
  deposit --from "$ALICE" --amount 1000000

# Transfer 500,000 from Alice to Bob (Bob must also be a verified identity)
stellar contract invoke \
  --id "$POOL_ID" --source-account "$ALICE" --network testnet -- \
  transfer --from "$ALICE" --to "$BOB" --amount 500000

# Withdraw 250,000 back to Alice
stellar contract invoke \
  --id "$POOL_ID" --source-account "$ALICE" --network testnet -- \
  withdraw --to "$ALICE" --amount 250000

# Inspect pool state (read-only)
stellar contract invoke \
  --id "$POOL_ID" --network testnet -- get_pool_config
stellar contract invoke \
  --id "$POOL_ID" --network testnet -- get_compliance_config
```

After each operation the registry's volume counters move (visible via
`get_identity_record --user "$ALICE"`) and a typed event is emitted.

### Compliance governance (owner)

```bash
# Add a rule: tier must be >= 2 for any action
stellar contract invoke \
  --id "$POOL_ID" --source-account "$SIGNER" --network testnet -- \
  add_compliance_rule \
  --rule '{"field":"Tier","operator":"Gte","value":{"Single":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2]},"action_filter":"Any"}'

# Remove a rule by index
stellar contract invoke \
  --id "$POOL_ID" --source-account "$SIGNER" --network testnet -- \
  remove_compliance_rule --index 0

# Replace the whole compliance config (limits, registry, restricted countries)
stellar contract invoke \
  --id "$POOL_ID" --source-account "$SIGNER" --network testnet -- \
  set_compliance_config \
  --config '{"owner":"'"$ADMIN"'","paused":false,"rules":[],"identity_registry":"'"$IDENTITY_REGISTRY_ID"'","required_tier":1,"daily_volume_limit":1000000000,"monthly_volume_limit":10000000000,"restricted_jurisdictions":["IR","KP"]}'

# Emergency pause / resume
stellar contract invoke \
  --id "$POOL_ID" --source-account "$SIGNER" --network testnet -- pause_contract
stellar contract invoke \
  --id "$POOL_ID" --source-account "$SIGNER" --network testnet -- unpause_contract
```

---

## 4. What "compliance enforced" means

Every `deposit` / `withdraw` / `transfer` runs the full pipeline before any
token movement:

1. Amount is positive (and ≥ `min_deposit` for deposits, ≤ liquidity for
   withdrawals).
2. Contract is not paused.
3. Caller authenticated (`require_auth`).
4. Sender has a **Verified** identity in the registry.
5. Sender's tier meets `required_tier`.
6. Sender's country code is not on the restricted-jurisdictions list.
7. All programmable `ComplianceRule`s pass.
8. Daily/monthly volume caps are not exceeded.

If any gate fails, the transaction reverts with a typed `ComplianceError`
(e.g. `KycNotVerified`, `InsufficientTier`, `DailyVolumeExceeded`).

---

## 5. Verifying events & errors

Typed contract events are emitted for every state change
(see `SEP57_COMPLIANCE.md` for the full table). On failure, `stellar` prints
the contract error; the unit/integration test suite
(`make test`) exercises every error path locally without a network.
