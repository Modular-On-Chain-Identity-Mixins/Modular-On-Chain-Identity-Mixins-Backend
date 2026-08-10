# Changelog

All notable changes to this workspace are documented here, following
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `SECURITY.md` — vulnerability disclosure policy.
- `CONTRIBUTING.md` — development workflow and CI gate guide.
- `CHANGELOG.md` — this file.
- `LICENSE` — MIT license text matching the workspace metadata.

## [0.1.0] - 2026-08-10

Initial production-ready release.

### Added

- **soroban-compliance-kit**
  - Rule engine with 8 operators (`Eq`, `Neq`, `Gt`, `Lt`, `Gte`, `Lte`,
    `In`, `NotIn`) over 9 rule fields, action-scoped (`Any`, `Transfer`,
    `Deposit`, `Withdraw`, `Mint`, `Burn`).
  - `ComplianceManager` trait and `compliance_*_check!` gate macros.
  - Typed `ComplianceError` surface (`InvalidAmount`, etc.).
  - Proptest property tests for the rule engine.

- **identity-registry**
  - Typed `#[contracterror]` surface for every fallible entrypoint.
  - Typed `#[contractevent]` audit events for register, KYC updates,
    verifier/caller lifecycle, jurisdiction updates, and volume updates.
  - Verifier allow-list (KYC updates) and authorized-caller allow-list
    (volume updates), admin-only governance, `require_auth` on all
    privileged paths.
  - Input validation: DIDs, ISO 3166-1 alpha-2 country codes, tiers,
    amounts; constructor re-init guard.
  - Composite `IdentityRecord` (identity + volume + custom fields) with
    explicit `IdentityNotFound` instead of cross-contract panics.

- **reference-defi-pool**
  - Compliance-gated deposit/withdraw/transfer against the identity
    registry (KYC, tier, jurisdiction, rules, volume caps, pause).
  - Cross-contract volume updates via `authorize_as_current_contract`
    self-auth and nested `try_` result handling.
  - Owner-governed `set_compliance_config`, negative-amount guards.

- **CI/CD & tooling**
  - GitHub Actions: fmt, clippy (`-D warnings`, incl. test modules), full
    test suite, hard wasm32v1-none build gate with artifact upload,
    llvm-cov coverage gate (≥80% lines), and a tag-triggered release
    workflow attaching `.wasm` artifacts.
  - `Makefile` targets for build/test/lint/wasm/coverage/deploy.
  - `stellar`-CLI deploy/init scripts (with `soroban` fallback), `.env`
    support and `.env.example`.
  - Dockerfile + docker-compose for a 1-command containerized dev env.
  - Pinned toolchain (`rust-toolchain.toml`), refreshed dependencies
    (soroban-sdk 27.0.5).
  - README rewrite, `docs/QUICKSTART.md`, `SEP57_COMPLIANCE.md` mapping.

[Unreleased]: https://github.com/Modular-On-Chain-Identity-Mixins/Modular-On-Chain-Identity-Mixins-Backend/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Modular-On-Chain-Identity-Mixins/Modular-On-Chain-Identity-Mixins-Backend/releases/tag/v0.1.0
