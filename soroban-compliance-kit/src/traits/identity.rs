use soroban_sdk::{Address, Bytes, Env};

use crate::types::{IdentityRecord, Jurisdiction, KycStatus};

/// Interface for interacting with an on-chain identity registry.
pub trait IdentityVerifier {
    /// Register a new identity in the given registry.
    fn register_identity(
        env: &Env,
        registry: &Address,
        user: &Address,
        did: Bytes,
        jurisdiction: Jurisdiction,
        country_code: Bytes,
        tier: u32,
    );

    /// Update the KYC status for a user.
    fn update_kyc_status(env: &Env, registry: &Address, user: &Address, status: KycStatus);

    /// Fetch the full identity record for a user.
    fn get_identity(env: &Env, registry: &Address, user: &Address) -> IdentityRecord;

    /// Update volume counters after a successful transaction.
    fn update_volume(env: &Env, registry: &Address, user: &Address, amount: i128);
}
