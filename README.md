# Soroban Compliance Kit — Modular On-Chain Identity Mixins

[![CI](https://github.com/Modular-On-Chain-Identity-Mixins/Modular-On-Chain-Identity-Mixins-Backend/actions/workflows/ci.yml/badge.svg)](https://github.com/Modular-On-Chain-Identity-Mixins/Modular-On-Chain-Identity-Mixins-Backend/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An open-source Rust workspace for **granular, programmable compliance on Stellar Soroban**,
implementing [SEP-57: On-Chain Identity Mixins](https://github.com/stellar/stellar-protocol/blob/master/sep-0057.md)
and the [ERC-3643 / T-REX](https://eips.ethereum.org/EIPS/eip-3643) permissioned-token model.

The workspace ships three crates:

| Crate | Kind | Purpose |
|---|---|---|
| `soroban-compliance-kit` | library | Reusable compliance primitives — rule engine, types, traits, gate macros |
| `identity-registry` | contract | On-chain identity store: KYC status, jurisdiction, tier, volume, custom fields |
| `reference-defi-pool` | contract | Reference implementation of a compliant pool (deposit/withdraw/transfer) |

## Architecture

```
User ──> DeFi Pool (reference-defi-pool)
              │
              ▼  compliance gate: paused? KYC verified? tier? jurisdiction? rules? volume caps?
     [Identity Registry Contract]  ──►  KYC / jurisdiction / tier / volume / custom fields
              │
    ┌─────────┴─────────┐
    ▼ (Pass)            ▼ (Fail)
 [Execute]         [Typed error returned, transaction reverts]
```

The compliance kit is a **mixin library**: any Soroban contract implements the
`ComplianceManager` trait (as `reference-defi-pool` does) and inserts one of the
gate macros (`compliance_transfer_check!`, `compliance_deposit_check!`,
`compliance_withdraw_check!`) at the top of its regulated functions. No
inheritance or proxy contracts are required.

## Repository layout

```
soroban-compliance-kit/        reusable library
  ├── src/types/               ComplianceRule, IdentityRecord, ComplianceConfig, errors…
  ├── src/traits/              ComplianceManager interface
  ├── src/macros/              compliance_*_check! gates
  ├── src/rule_engine.rs       rule evaluation, volume caps, jurisdiction restrictions
  └── tests/property_tests.rs  proptest fuzzing of the rule engine
identity-registry/             identity + KYC + volume registry contract
  ├── src/contract.rs          contract entrypoints, typed events & errors
  └── src/storage.rs           on-chain storage layout
reference-defi-pool/           compliant pool contract (reference implementation)
  ├── src/contract.rs          deposit / withdraw / transfer with full compliance
  └── src/storage.rs           pool + compliance configuration
scripts/                       deploy.sh / init.sh (stellar CLI)
```

## Prerequisites

- **Rust toolchain** — pinned in [`rust-toolchain.toml`](rust-toolchain.toml)
  (`stable`, with `wasm32v1-none` target for Soroban).
- **Stellar CLI** (only for deployment) — `cargo install stellar-cli --locked`,
  or the legacy `soroban` CLI (the scripts support both).
- **Docker** (optional) — for a containerized, zero-setup dev environment.

## Quick start (1 command)

```bash
make all          # check + clippy + full test suite
```

Or, without installing anything locally:

```bash
docker compose run --rm dev test
```

## Common commands

| Goal | Command |
|---|---|
| Full validation gate | `make all` |
| Build contracts to WASM | `make build-wasm` |
| Run all tests (unit + integration + property) | `make test` |
| Test a single crate | `make test-kit` / `test-registry` / `test-pool` |
| Lint (warnings are errors) | `make lint` |
| Verify formatting | `make fmt-check` |
| Build API docs | `make doc` |
| Line coverage (80% gate) | `make coverage` |

Under the hood the same commands are:

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --features testutils
cargo build --target wasm32v1-none --release -p identity-registry -p reference-defi-pool
```

## Testing

Tests require the `testutils` feature (mocked auth, `Address::generate`, ledger
snapshots), which the `make test` target enables workspace-wide.

- **Unit tests** — every registry and pool code path, including all error paths
  (`identity-registry/src/test.rs`, `reference-defi-pool/src/test.rs`).
- **Integration tests** — the pool exercising the registry cross-contract
  (KYC-gated deposits, volume tracking, jurisdiction rules).
- **Property tests** — proptest fuzzing of the rule engine
  (`soroban-compliance-kit/tests/property_tests.rs`).
- **Ledger snapshots** — committed under `test_snapshots/`; the SDK rewrites
  them on each run so contract/storage changes are visible in review.

Coverage (requires `cargo install cargo-llvm-cov`):

```bash
make coverage
```

Measured on the current test suite: **~94% line coverage** (registry contract
99%, pool contract 90%, rule engine 84%) — comfortably above the 80% gate
that `make coverage` enforces.

## Environment variables

All configuration is read from environment variables; copy
[`.env.example`](.env.example) to `.env` and fill it in (`.env` is git-ignored):

| Variable | Description | Required for |
|---|---|---|
| `SOROBAN_RPC_URL` | Soroban RPC endpoint | deploy |
| `SOROBAN_NETWORK_PASSPHRASE` | Network passphrase | deploy |
| `SOROBAN_SECRET_KEY` | Deploying account secret key / identity | deploy |
| `IDENTITY_REGISTRY_ID` / `POOL_ID` | Deployed contract IDs | init |
| `IDENTITY_REGISTRY_ADMIN` / `POOL_ADMIN` | Admin `G...` addresses | init |
| `TOKEN_ID` | Asset contract to wrap in the pool | init |
| `REQUIRED_TIER` / `DAILY_VOLUME_LIMIT` / `MONTHLY_VOLUME_LIMIT` | Pool compliance defaults | init |
| `RESTRICTED_JURISDICTIONS` | JSON array of restricted country codes | init |

**Secrets:** never commit `.env` or any `*.pem`/key material (see `.gitignore`).

## Deployment

Deploying to Soroban is a two-step process. Both scripts auto-detect the
`stellar` CLI (or fall back to `soroban`) and read `.env`.

```bash
# 1. Deploy both contracts, print their IDs
make deploy            # == ./scripts/deploy.sh

# 2. Fill the printed IDs + admin keys into .env, then initialize
make init              # == ./scripts/init.sh
```

Manual equivalents (testnet):

```bash
# Fund an account and build the contracts
stellar keys fund alice
make build-wasm

# Deploy
stellar contract deploy \
  --wasm target/wasm32v1-none/release/identity_registry.wasm \
  --source-account alice --network testnet

# Initialize the registry (admin)
stellar contract invoke \
  --id <REGISTRY_ID> --source-account alice --network testnet -- \
  __constructor --admin <ADMIN_G_ADDRESS>

# Deploy a wrapped asset and initialize the pool
stellar contract asset deploy --asset USD:<ISSUER> --source-account alice --network testnet
stellar contract invoke \
  --id <POOL_ID> --source-account alice --network testnet -- \
  __constructor \
  --token <TOKEN_ID> --admin <ADMIN_G_ADDRESS> --identity-registry <REGISTRY_ID> \
  --required-tier 1 --daily-volume-limit 1000000000 --monthly-volume-limit 10000000000 \
  --restricted-jurisdictions "[]"

# Authorize the pool to update volume counters in the registry
stellar contract invoke \
  --id <REGISTRY_ID> --source-account alice --network testnet -- \
  add_authorized_caller --caller <POOL_ID>
```

## Using the compliance kit in your own contract

```rust
use soroban_compliance_kit::traits::ComplianceManager;
use soroban_compliance_kit::compliance_transfer_check;

impl ComplianceManager for MyContract {
    fn enforce_compliance(/* ... */) -> Result<(IdentityRecord, IdentityRecord), ComplianceError> {
        // pause, KYC, tier, jurisdiction, rules, volume caps
    }
}

impl MyContract {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), ComplianceError> {
        compliance_transfer_check!(MyContract, env, from, to, amount);
        // ... business logic ...
        Ok(())
    }
}
```

The rule engine supports 8 operators (`Eq`, `Neq`, `Gt`, `Lt`, `Gte`, `Lte`,
`In`, `NotIn`) over 9 rule fields (KYC status, tier, jurisdiction, country
code, daily/monthly volume, total supply, balance, custom fields), each scoped
by action filter (`Any`, `Transfer`, `Deposit`, `Withdraw`, `Mint`, `Burn`).

## Design & standards

- **Typed errors** — all fallible entrypoints return `Result<_, ContractError>`
  (never opaque panics) so callers and tooling get structured failures.
- **Typed events** — every state change emits a `#[contractevent]` audit event
  (register, KYC updates, verifier/caller lifecycle, volume updates, pool ops).
- **Least privilege** — admin-only mutations, verifier allow-list for KYC,
  caller allow-list for volume updates, `require_auth` on every privileged path.
- **Input validation** — DIDs, ISO 3166-1 country codes, tiers, amounts.
- **Overflow safety** — `saturating_*` arithmetic on all volume counters.
- **No secrets in code** — keys are environment variables only.

See [SEP57_COMPLIANCE.md](SEP57_COMPLIANCE.md) for the full standards mapping.

## CI/CD

`.github/workflows/ci.yml` runs on every push/PR to `main`:

1. **quality** — `cargo fmt --check`, `cargo clippy -D warnings` (incl. tests), `cargo doc`
2. **test** — full workspace test suite with `testutils`
3. **build-wasm** — release WASM build for `wasm32v1-none` (hard gate) and
   artifact upload
4. **coverage** — `cargo llvm-cov` with an 80% line gate + lcov artifact

Tagging a version (`git tag v0.1.0 && git push --tags`) triggers
`.github/workflows/release.yml`, which attaches the two `.wasm` artifacts to
the GitHub release.

**Dependency updates** — [Dependabot](.github/dependabot.yml) opens grouped
PRs weekly: the `soroban-*` family stays in lockstep and non-breaking
updates are batched. Every change to `Cargo.lock` is scanned by the
`cargo-audit` CI job against the RustSec advisory DB (`make audit` locally).

For a full walkthrough of every contract entrypoint, see
[`docs/QUICKSTART.md`](docs/QUICKSTART.md).

## License

MIT — see [LICENSE](LICENSE). Built for the Stellar ecosystem.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and
[SECURITY.md](SECURITY.md) for the vulnerability disclosure policy.
