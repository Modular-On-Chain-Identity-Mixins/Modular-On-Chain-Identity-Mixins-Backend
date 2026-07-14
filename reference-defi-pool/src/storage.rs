use soroban_sdk::{contracttype, symbol_short, Address, Env};

use soroban_compliance_kit::types::ComplianceConfig;

#[contracttype]
pub struct PoolConfig {
    pub token: Address,
    pub total_liquidity: i128,
    pub min_deposit: i128,
    pub admin: Address,
}

pub fn write_pool_config(env: &Env, config: &PoolConfig) {
    env.storage().instance().set(&symbol_short!("poolcfg"), config);
}

pub fn read_pool_config(env: &Env) -> PoolConfig {
    env.storage().instance().get(&symbol_short!("poolcfg")).expect("pool not initialized")
}

pub fn has_pool_config(env: &Env) -> bool {
    env.storage().instance().has(&symbol_short!("poolcfg"))
}

pub fn write_compliance_config(env: &Env, config: &ComplianceConfig) {
    env.storage().instance().set(&symbol_short!("compcfg"), config);
}

pub fn read_compliance_config(env: &Env) -> ComplianceConfig {
    env.storage()
        .instance()
        .get(&symbol_short!("compcfg"))
        .expect("compliance not initialized")
}

pub fn has_compliance_config(env: &Env) -> bool {
    env.storage().instance().has(&symbol_short!("compcfg"))
}
