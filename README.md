# Soroban Compliance Kit — Modular On-Chain Identity Mixins

An open-source Rust library and reference contracts for granular, programmable compliance on Stellar Soroban, implementing SEP-57 / T-REX (ERC-3643) standards for permissioned token operations.

## Architecture

```
User ──> DeFi Pool / Token Contract
              │
              ▼ (Intercepted by Compliance Mixin)
     [Identity Registry Contract] ──> [Validates KYC/Jurisdiction/Tier/Volume]
              │
    ┌─────────┴─────────┐
    ▼ (Pass)            ▼ (Fail)
 [Execute]         [Panic: Unauthorized]
```

## Project Structure

```
soroban-compliance-kit/     # Reusable library crate (traits, types, macros, rule engine)
  ├── traits/               # ComplianceManager, IdentityVerifier traits
  ├── types/                # ComplianceConfig, IdentityRecord, ComplianceRule, etc.
  ├── macros/               # #[soroban_compliance], compliance_transfer_check!, etc.
  └── rule_engine/          # Rule evaluation engine

identity-registry/          # Reference Identity Registry contract
  ├── register/verify identities
  ├── KYC status management
  ├── Verifier authorization
  └── Jurisdiction whitelist

reference-defi-pool/        # Example DeFi pool using the compliance kit
  ├── Deposit/withdraw with compliance checks
  ├── Transfer with identity verification
  └── Volume limits & jurisdiction restrictions
```

## Usage

**Add the compliance kit to a contract:**

```rust
use soroban_compliance_kit::traits::ComplianceManager;
use soroban_compliance_kit::macros::{compliance_transfer_check, compliance_deposit_check};

impl ComplianceManager for MyContract {
    fn enforce_compliance(/* ... */) -> Result<(), ComplianceError> {
        // Identity + rule enforcement
    }
}

impl MyContract {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), ComplianceError> {
        compliance_transfer_check!(MyContract, env, from, to, amount);
        // ... proceed with transfer
        Ok(())
    }
}
```

**Use the macros for inline compliance gates:**

- `compliance_transfer_check!(Contract, env, from, to, amount)`
- `compliance_deposit_check!(Contract, env, from, amount)`
- `compliance_withdraw_check!(Contract, env, from, to, amount)`

## Building

```bash
cargo build
```

## Testing

Tests require the `testutils` feature:

```bash
cargo test --features testutils
```

Note: The `testutils` feature enables mock auth, `Address::generate`, and other Soroban test utilities. If testutils fails to compile with `ed25519-dalek` compatibility errors, update your `soroban-sdk` dependency or try a different SDK version.

## License

MIT
