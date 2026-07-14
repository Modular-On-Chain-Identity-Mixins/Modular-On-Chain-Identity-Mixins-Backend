use soroban_sdk::{Address, Env, Vec};

use crate::types::{
    ComplianceAction, ComplianceConfig, ComplianceError, ComplianceRule, IdentityRecord,
};

/// Interface for contracts that manage on-chain compliance.
///
/// Implementations store a [`ComplianceConfig`] and enforce rules on every
/// regulated action (transfer, deposit, withdraw, etc.).
pub trait ComplianceManager {
    /// Initialise the compliance module with an owner, registry, and limits.
    fn init_compliance(
        env: &Env,
        owner: Address,
        identity_registry: Address,
        required_tier: u32,
        daily_volume_limit: i128,
        monthly_volume_limit: i128,
        restricted_jurisdictions: Vec<soroban_sdk::Bytes>,
    );

    /// Read the full compliance configuration.
    fn get_config(env: &Env) -> ComplianceConfig;

    /// Replace the compliance configuration (owner-only).
    fn set_config(env: &Env, config: ComplianceConfig);

    /// Append a new compliance rule (owner-only).
    fn add_rule(env: &Env, rule: ComplianceRule);

    /// Remove a rule by index (owner-only).
    fn remove_rule(env: &Env, index: u32);

    /// Pause all compliance-checked operations (owner-only).
    fn pause(env: &Env);

    /// Unpause operations (owner-only).
    fn unpause(env: &Env);

    /// Assert that the caller has authorised the operation.
    fn verify_auth(env: &Env, user: &Address);

    /// Returns `true` if the contract is currently paused.
    fn is_paused(env: &Env) -> bool;

    /// Full compliance enforcement pipeline.
    ///
    /// Checks pause, KYC, tier, jurisdiction, programmable rules, volume
    /// limits, and (for transfers) recipient jurisdiction. Returns both
    /// sender and recipient identity records for downstream use.
    fn enforce_compliance(
        env: &Env,
        sender: &Address,
        recipient: &Address,
        amount: i128,
        action: ComplianceAction,
    ) -> Result<(IdentityRecord, IdentityRecord), ComplianceError>;

    /// Evaluate configured rules against a single identity record.
    fn evaluate_rules(
        env: &Env,
        user: &IdentityRecord,
        action: &ComplianceAction,
        amount: i128,
        total_supply: Option<i128>,
        balance: Option<i128>,
    ) -> Result<(), ComplianceError>;
}
