#![no_std]
#![deny(unsafe_code)]
// This crate links the `identity-registry` contract crate (for its generated
// client). Both crates export a `__constructor` entrypoint, so wasm-ld reports
// a duplicate-symbol / signature-mismatch message when the pool's WASM is
// linked. The pool's own export is the one the host invokes at runtime, making
// the message benign; the `linker_messages` lint silences it (the lint also
// ignores `-D warnings`, so this is purely cosmetic).
#![allow(linker_messages)]

//! # Reference DeFi Pool
//!
//! A reference implementation of a compliance-gated liquidity pool built on
//! the [`soroban_compliance_kit`]. Every `deposit`, `withdraw` and `transfer`
//! runs the full compliance pipeline against the identity registry (pause,
//! KYC, tier, jurisdiction, programmable rules, volume caps) before any token
//! movement, and reports failures as typed [`ComplianceError`]s.
//!
//! [`ComplianceError`]: soroban_compliance_kit::types::ComplianceError

pub mod contract;
mod storage;
mod test;
