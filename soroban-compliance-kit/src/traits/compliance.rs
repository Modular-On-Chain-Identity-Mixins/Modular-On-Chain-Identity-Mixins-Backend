use soroban_sdk::{Address, Env, Vec};

use crate::types::{
    ComplianceAction, ComplianceConfig, ComplianceError, ComplianceRule, IdentityRecord,
};

pub trait ComplianceManager {
    fn init_compliance(
        env: &Env,
        owner: Address,
        identity_registry: Address,
        required_tier: u32,
        daily_volume_limit: i128,
        monthly_volume_limit: i128,
        restricted_jurisdictions: Vec<soroban_sdk::Bytes>,
    );

    fn get_config(env: &Env) -> ComplianceConfig;

    fn set_config(env: &Env, config: ComplianceConfig);

    fn add_rule(env: &Env, rule: ComplianceRule);

    fn remove_rule(env: &Env, index: u32);

    fn pause(env: &Env);

    fn unpause(env: &Env);

    fn verify_auth(env: &Env, user: &Address);

    fn is_paused(env: &Env) -> bool;

    fn enforce_compliance(
        env: &Env,
        sender: &Address,
        recipient: &Address,
        amount: i128,
        action: ComplianceAction,
    ) -> Result<(IdentityRecord, IdentityRecord), ComplianceError>;

    fn evaluate_rules(
        env: &Env,
        user: &IdentityRecord,
        action: &ComplianceAction,
        amount: i128,
        total_supply: Option<i128>,
        balance: Option<i128>,
    ) -> Result<(), ComplianceError>;
}
