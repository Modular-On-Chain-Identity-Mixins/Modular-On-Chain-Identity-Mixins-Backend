## Summary

<!-- What does this PR do, and why? Keep it short; link issues with "Closes #N". -->

## Changes

<!-- Bullet list of the main changes, per crate/module where relevant. -->

## Testing

- [ ] `make verify` passes locally (fmt, clippy `-D warnings`, tests, WASM build, docs, coverage ≥80%, audit)
- [ ] New or updated tests cover the change
- [ ] `docs/` (README, QUICKSTART, SEP57 mapping) updated if the public API changed

## Review checklist

- [ ] `require_auth` on every privileged path
- [ ] Typed errors instead of panics on user-supplied input
- [ ] Overflow-safe arithmetic (`saturating_*`) where applicable
- [ ] No secrets or `.env` files committed
