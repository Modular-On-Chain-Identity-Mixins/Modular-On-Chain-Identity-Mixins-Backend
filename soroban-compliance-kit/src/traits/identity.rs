use soroban_sdk::{Address, Env};

use crate::types::{ComplianceAction, IdentityRecord, Jurisdiction, KycStatus};

pub trait IdentityVerifier {
    fn register_identity(
        env: &Env,
        registry: &Address,
        user: &Address,
        did: soroban_sdk::Bytes,
        jurisdiction: Jurisdiction,
        country_code: soroban_sdk::Bytes,
        tier: u32,
    );

    fn update_kyc_status(env: &Env, registry: &Address, user: &Address, status: KycStatus);

    fn get_identity(env: &Env, registry: &Address, user: &Address) -> IdentityRecord;

    fn verify_identity(
        env: &Env,
        registry: &Address,
        user: &Address,
        action: ComplianceAction,
    ) -> Result<IdentityRecord, soroban_sdk::Error>;

    fn is_authorized(
        env: &Env,
        registry: &Address,
        user: &Address,
        action: ComplianceAction,
    ) -> bool;
}
