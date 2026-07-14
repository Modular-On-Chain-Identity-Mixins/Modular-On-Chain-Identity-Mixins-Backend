# SEP-57 / ERC-3643 Compliance Mapping

This document maps the `soroban-compliance-kit` library to [SEP-57: On-Chain Identity Mixins](https://github.com/stellar/stellar-protocol/blob/master/sep-0057.md) and the [ERC-3643 T-REX](https://eips.ethereum.org/EIPS/eip-3643) standard it extends.

## SEP-57 Core Concepts

| SEP-57 Concept | Implementation | File |
|---|---|---|
| **Identity Registry** | `identity-registry` contract — stores KYC status, jurisdiction, tier per `Address` | `identity-registry/src/contract.rs` |
| **Compliance Module** | `ComplianceManager` trait — 12-method interface for any Soroban contract | `soroban-compliance-kit/src/traits/compliance.rs` |
| **Rule Evaluation** | `rule_engine::evaluate_rules` — programmable gate logic with 8 operators | `soroban-compliance-kit/src/rule_engine.rs` |
| **Volume Limits** | `rule_engine::check_volume_limits` — daily/monthly caps | `soroban-compliance-kit/src/rule_engine.rs` |
| **Jurisdiction Control** | `rule_engine::check_jurisdiction_restriction` — country-code allow/deny | `soroban-compliance-kit/src/rule_engine.rs` |
| **Gate Macros** | `compliance_transfer_check!`, `compliance_deposit_check!`, `compliance_withdraw_check!` | `soroban-compliance-kit/src/macros/compliance_check.rs` |
| **Mixin Architecture** | Library crates (`soroban-compliance-kit`) imported by contracts — no inheritance needed | `Cargo.toml` (workspace) |

## Identity Record Structure (SEP-57 §3)

| Field | Type | Description |
|---|---|---|
| `did` | `Bytes` | Decentralized Identifier |
| `kyc_status` | `KycStatus` | None / Pending / Verified / Rejected / Expired |
| `jurisdiction` | `Jurisdiction` | Us / Eu / Uk / Other(Bytes) |
| `country_code` | `Bytes` | ISO 3166-1 alpha-2 |
| `tier` | `u32` | Numeric access tier |
| `daily_volume` | `i128` | Cumulative daily volume (reset periodically) |
| `monthly_volume` | `i128` | Cumulative monthly volume |
| `custom_fields` | `Vec<CustomField>` | Extensible key-value map |

## ERC-3643 (T-REX) Compatibility

| ERC-3643 Feature | Soroban Equivalent |
|---|---|
| `_identityRegistry` | `ComplianceConfig.identity_registry` (contract address) |
| `_compliance` | `ComplianceManager` trait + `rule_engine` |
| `_token` | Stellar Asset Contract (SAC) — `token::TokenClient` |
| `onlyOwner` | `config.owner.require_auth()` |
| `_canTransfer` | `require_compliance!` macro → `enforce_compliance` |
| `_isVerified` | `registry_client.get_identity_record(sender).kyc_status == Verified` |
| `_isModifier` | Rust trait defaults — no modifier logic |
| `_signed` | Stellar native signature verification via `require_auth` |
| Identity storage | `Env::storage().persistent()` / `temporary()` |

## Rule Engine Operators

| Operator | Comparison |
|---|---|
| `Eq` | `==` |
| `Neq` | `!=` |
| `Gt` | `>` |
| `Lt` | `<` |
| `Gte` | `>=` |
| `Lte` | `<=` |
| `In` | `==` (set membership via `Vec`) |
| `NotIn` | `!=` (set exclusion) |

## Rule Fields

| Field | Source |
|---|---|
| `Jurisdiction` | `IdentityRecord.jurisdiction` |
| `KycStatus` | `IdentityRecord.kyc_status` (numeric mapping) |
| `Tier` | `IdentityRecord.tier` |
| `CountryCode` | `IdentityRecord.country_code` |
| `DailyVolume` | `IdentityRecord.daily_volume` |
| `MonthlyVolume` | `IdentityRecord.monthly_volume` |
| `TotalSupply` | External parameter (optional, e.g. from SAC) |
| `Balance` | External parameter (optional, e.g. from SAC) |
| `Custom(Bytes)` | `IdentityRecord.custom_fields` key lookup |

## Action Filtering

Each `ComplianceRule` carries an `action_filter: ComplianceAction`:
- `Any` — rule applies to all actions
- `Transfer`, `Deposit`, `Withdraw`, `Mint`, `Burn` — scoped rule

Three gate macros map to these actions:
- `compliance_transfer_check!` → `ComplianceAction::Transfer`
- `compliance_deposit_check!` → `ComplianceAction::Deposit`
- `compliance_withdraw_check!` → `ComplianceAction::Withdraw`

## Deployed Contracts

| Contract | Purpose |
|---|---|
| `identity-registry` | Stores identity records, manages KYC flow, verifier authorization |
| `reference-defi-pool` | Reference implementation of `ComplianceManager` — deposit/withdraw/transfer with full compliance enforcement |
