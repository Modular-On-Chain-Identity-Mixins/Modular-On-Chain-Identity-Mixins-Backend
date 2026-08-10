use soroban_sdk::{contracttype, Address, Env};

use soroban_compliance_kit::types::ComplianceConfig;

/// Pool-specific configuration (token, liquidity, admin).
#[contracttype]
pub struct PoolConfig {
    pub token: Address,
    pub total_liquidity: i128,
    pub min_deposit: i128,
    pub admin: Address,
}

/// Single namespace for every storage key used by the pool.
#[contracttype]
pub enum DataKey {
    PoolConfig,
    ComplianceConfig,
}

pub fn write_pool_config(env: &Env, config: &PoolConfig) {
    env.storage().instance().set(&DataKey::PoolConfig, config);
}

/// Read the pool configuration.
///
/// Panics only if the contract was never initialized; the host enforces that
/// `__constructor` runs exactly once at deployment, so this is unreachable in
/// practice.
pub fn read_pool_config(env: &Env) -> PoolConfig {
    env.storage()
        .instance()
        .get(&DataKey::PoolConfig)
        .expect("pool not initialized")
}

pub fn has_pool_config(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::PoolConfig)
}

pub fn write_compliance_config(env: &Env, config: &ComplianceConfig) {
    env.storage()
        .instance()
        .set(&DataKey::ComplianceConfig, config);
}

/// Read the compliance configuration.
///
/// Panics only if the contract was never initialized; the host enforces that
/// `__constructor` runs exactly once at deployment, so this is unreachable in
/// practice.
pub fn read_compliance_config(env: &Env) -> ComplianceConfig {
    env.storage()
        .instance()
        .get(&DataKey::ComplianceConfig)
        .expect("compliance not initialized")
}

pub fn has_compliance_config(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::ComplianceConfig)
}
