# Security Policy

The contracts in this workspace manage identity, KYC, and volume state on
Stellar Soroban. Bugs in smart-contract code can have irreversible financial
consequences, so we ask that vulnerabilities be reported privately.

## Supported versions

| Version | Supported |
|---|---|
| `main` (latest) | ✅ |
| Tagged releases (`v0.x.y`) | ✅ — critical fixes backported on request |
| Older releases | ❌ |

## Reporting a vulnerability

**Do not open a public issue for security problems.** Use GitHub's private
vulnerability reporting instead:

1. Open the **Security** tab of this repository.
2. Click **Report a vulnerability** and fill in the form.

> **Security contact email** — to receive notifications for private reports,
> set a contact email under **Repository settings → Security → Security
> contact**. No contact email is stored anywhere in this codebase, so this
> file never goes stale and requires no secret management.

Please include:

- Which contract/crate and version is affected.
- A minimal, reproducible test case (prefer a Soroban unit test).
- The impact (funds at risk, access-control bypass, storage bloat, etc.).
- Any suggested fix, if you have one.

## Disclosure process

- **Acknowledgement** — within 3 business days of a valid report.
- **Triage & fix** — a fix (with regression tests) is normally prepared within
  14 days, depending on severity.
- **Disclosure** — after a fix is deployed, we publish an advisory and credit
  the reporter (unless they prefer to stay anonymous).

## Scope

- `identity-registry/` and `reference-defi-pool/` contracts (on-chain logic).
- `soroban-compliance-kit/` (rule engine, macros, traits used by on-chain code).

Out of scope: deployment scripts (no secrets stored), documentation issues
(report those as normal issues), and dependencies with their own advisories
(see `cargo audit` / the RustSec advisory database).
