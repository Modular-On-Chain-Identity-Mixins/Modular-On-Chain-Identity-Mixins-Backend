# Contributing

Thanks for wanting to contribute to the Modular On-Chain Identity Mixins
workspace. This guide covers how to build, test, and open pull requests that
pass CI on the first try.

## Repository overview

| Directory | What it is |
|---|---|
| `soroban-compliance-kit/` | Reusable compliance library (rule engine, types, traits, macros) |
| `identity-registry/` | Identity/KYC/volume registry contract |
| `reference-defi-pool/` | Reference compliant pool contract |
| `scripts/` | `stellar`-CLI deploy/init scripts |
| `docs/` | Guides (quickstart, standards mapping) |

## Prerequisites

- Rust toolchain — pinned in [`rust-toolchain.toml`](rust-toolchain.toml).
  `rustup` picks it up automatically when you enter the workspace.
- The `wasm32v1-none` target for Soroban builds (also pinned in the toolchain
  file).
- Optional: `cargo-llvm-cov` (`cargo install cargo-llvm-cov`) for coverage.

## Local development

```bash
make all          # check + clippy + tests — run this before pushing
make build-wasm   # build the contracts for the Soroban runtime
make test         # full workspace test suite (unit + integration + property)
make lint         # clippy, warnings are errors
make fmt          # format the workspace (CI enforces `cargo fmt --check`)
make doc          # build API docs
make coverage     # line coverage with an 80% gate
```

Everything CI runs locally is listed in [`Makefile`](Makefile) and
[`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## CI gate (what must pass)

Every pull request runs the full CI matrix. In order of failure likelihood:

1. `cargo fmt --all -- --check` — run `make fmt` first.
2. `cargo clippy --workspace --all-targets --features testutils -- -D warnings`
   — note the `testutils` feature: test modules are linted too.
3. `cargo test --locked --workspace --features testutils` — full suite.
4. `cargo build --locked --target wasm32v1-none --release -p identity-registry -p reference-defi-pool`.
5. `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings`.
6. `cargo llvm-cov --workspace --features testutils --fail-under-lines 80`.
7. `cargo audit` — RustSec advisory scan of `Cargo.lock`.
8. `cargo deny check` — license allow-list, duplicate-version bans and
   advisory policy (see [`deny.toml`](deny.toml)).

**Run `make verify`** — it executes this exact sequence locally. If any of
these fail, CI is red — fix locally before asking for review.

## Branch naming

Use `kind/description` prefixes:

- `feat/` — new functionality
- `fix/` — bug fixes
- `docs/` — documentation
- `ci/` — CI/CD changes
- `refactor/` — structure changes with no behaviour change
- `security/` — hardening or vulnerability fixes

Examples: `feat/registry-volume-window`, `fix/pool-overflow`, `docs/api-reference`.

## Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short imperative summary>

<optional body explaining what and why>
```

Types: `feat`, `fix`, `docs`, `ci`, `refactor`, `test`, `chore`, `perf`,
`security`. Scopes: `kit`, `registry`, `pool`, `ci`, `scripts`, `docs`.

Keep each commit focused on one change; use multiple commits per PR when the
PR covers distinct steps (e.g. implementation, then tests).

## Opening a pull request

1. Create a branch from the latest `main`.
2. Make your changes and run `make all` locally.
3. If you added or changed contract entrypoints, update:
   - the affected tests (coverage is gated at 80% lines),
   - `docs/QUICKSTART.md` and/or `README.md` when public API changes,
   - `SEP57_COMPLIANCE.md` when events or errors change.
4. Open the PR against `main`, reference the issue it closes
   (`Closes #N`), and summarize what changed and why.

Reviewers check for: correct `require_auth` on every privileged path, typed
errors instead of panics on user-supplied input, overflow safety
(`saturating_*`), and tests for every new code path.

## Code style notes

- Editor/IDE defaults are pinned in [`.editorconfig`](.editorconfig)
  (UTF-8, LF, indentation per file type) — most editors pick it up
  automatically.
- Formatting is pinned in `rustfmt.toml` (import grouping enforced).
- Prefer typed `#[contractevent]`s over ad-hoc symbol events.
- Never `expect`/`panic` on user-supplied input — return a typed error.
- Keep privileged entrypoints behind an allow-list (admin, verifiers,
  authorized callers) with `require_auth`.

## Reporting issues

- Bugs and feature requests: open a normal issue.
- Security vulnerabilities: see [SECURITY.md](SECURITY.md) — do **not** open
  a public issue.
