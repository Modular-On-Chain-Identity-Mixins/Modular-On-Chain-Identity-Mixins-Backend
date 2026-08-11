#![no_std]
#![deny(unsafe_code)]

//! # Soroban Compliance Kit
//!
//! A reusable library for building permissioned Soroban smart contracts.
//! Implements the compliance primitives described in SEP-57 and ERC-3643.
//!
//! ## Architecture
//!
//! - **types** — Data structures (`ComplianceRule`, `IdentityRecord`, etc.)
//! - **traits** — the `ComplianceManager` interface for regulated contracts
//! - **rule_engine** — Rule evaluation, volume checks, jurisdiction restrictions
//! - **macros** — Convenience gates (`compliance_transfer_check!`, etc.)

pub mod macros;
pub mod rule_engine;
pub mod traits;
pub mod types;

pub use rule_engine::bytes_to_u128;
