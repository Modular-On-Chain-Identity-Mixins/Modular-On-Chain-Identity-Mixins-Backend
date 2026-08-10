#![no_std]
// This crate links the `identity-registry` contract crate (for its generated
// client). Both crates export a `__constructor` entrypoint, so wasm-ld reports
// a duplicate-symbol / signature-mismatch message when the pool's WASM is
// linked. The pool's own export is the one the host invokes at runtime, making
// the message benign; the `linker_messages` lint silences it (the lint also
// ignores `-D warnings`, so this is purely cosmetic).
#![allow(linker_messages)]

pub mod contract;
mod storage;
mod test;
