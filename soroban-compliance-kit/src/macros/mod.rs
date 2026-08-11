//! Compliance gate macros (`require_compliance!` and the action-specific
//! `compliance_transfer_check!`, `compliance_deposit_check!`,
//! `compliance_withdraw_check!`) that contracts insert at the top of their
//! regulated entrypoints.

mod compliance_check;
