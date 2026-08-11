#![no_std]
#![deny(unsafe_code)]

//! # Identity Registry
//!
//! On-chain identity store for SEP-57 / ERC-3643 permissioned systems.
//!
//! Tracks KYC status, jurisdiction, tier, volume counters and custom fields
//! per user, with admin-only governance, a verifier allow-list for KYC
//! updates and an authorized-caller allow-list for volume updates. Every
//! state change emits a typed audit event; every fallible entrypoint
//! returns a typed [`contract::RegistryError`] instead of panicking.

pub mod contract;
mod storage;
mod test;
