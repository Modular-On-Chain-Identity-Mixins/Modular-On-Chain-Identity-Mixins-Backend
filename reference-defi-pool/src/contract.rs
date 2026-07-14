#![allow(deprecated)]

use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Bytes, Env, Vec};

use soroban_compliance_kit::rule_engine;
use soroban_compliance_kit::traits::ComplianceManager;
use soroban_compliance_kit::types::{
    ComplianceAction, ComplianceConfig, ComplianceError, ComplianceRule, IdentityRecord, KycStatus,
};
use soroban_compliance_kit::{
    compliance_deposit_check, compliance_transfer_check, compliance_withdraw_check,
};

use crate::storage::{self, PoolConfig};

use identity_registry::contract::IdentityRegistryContractClient;

type Contract = DefiPoolContract;

#[contract]
pub struct DefiPoolContract;

impl ComplianceManager for DefiPoolContract {
    /// Initialise the compliance module with an owner, registry, and limits.
    fn init_compliance(
        env: &Env,
        owner: Address,
        identity_registry: Address,
        required_tier: u32,
        daily_volume_limit: i128,
        monthly_volume_limit: i128,
        restricted_jurisdictions: Vec<Bytes>,
    ) {
        if storage::has_compliance_config(env) {
            panic!("compliance already initialized");
        }

        let config = ComplianceConfig {
            owner,
            paused: false,
            rules: Vec::new(env),
            identity_registry,
            required_tier,
            daily_volume_limit,
            monthly_volume_limit,
            restricted_jurisdictions,
        };
        storage::write_compliance_config(env, &config);
    }

    fn get_config(env: &Env) -> ComplianceConfig {
        storage::read_compliance_config(env)
    }

    fn set_config(env: &Env, config: ComplianceConfig) {
        let current = storage::read_compliance_config(env);
        current.owner.require_auth();
        storage::write_compliance_config(env, &config);
    }

    fn add_rule(env: &Env, rule: ComplianceRule) {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();
        config.rules.push_back(rule);
        storage::write_compliance_config(env, &config);
    }

    fn remove_rule(env: &Env, index: u32) {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();
        let mut new_rules: Vec<ComplianceRule> = Vec::new(env);
        for (i, rule) in config.rules.iter().enumerate() {
            if i as u32 != index {
                new_rules.push_back(rule);
            }
        }
        config.rules = new_rules;
        storage::write_compliance_config(env, &config);
    }

    fn pause(env: &Env) {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();
        config.paused = true;
        storage::write_compliance_config(env, &config);
    }

    fn unpause(env: &Env) {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();
        config.paused = false;
        storage::write_compliance_config(env, &config);
    }

    fn verify_auth(_env: &Env, user: &Address) {
        user.require_auth();
    }

    fn is_paused(env: &Env) -> bool {
        storage::read_compliance_config(env).paused
    }

    fn enforce_compliance(
        env: &Env,
        sender: &Address,
        recipient: &Address,
        amount: i128,
        action: ComplianceAction,
    ) -> Result<(IdentityRecord, IdentityRecord), ComplianceError> {
        let config = storage::read_compliance_config(env);

        if config.paused {
            return Err(ComplianceError::ContractPaused);
        }

        let registry_client = IdentityRegistryContractClient::new(env, &config.identity_registry);

        let sender_record = registry_client.get_identity_record(sender);

        if sender_record.kyc_status != KycStatus::Verified {
            return Err(ComplianceError::KycNotVerified);
        }

        if sender_record.tier < config.required_tier {
            return Err(ComplianceError::InsufficientTier);
        }

        rule_engine::check_jurisdiction_restriction(
            &sender_record,
            &config.restricted_jurisdictions,
        )?;

        let token_client = token::TokenClient::new(env, &storage::read_pool_config(env).token);
        let balance = token_client.balance(&sender);

        rule_engine::evaluate_rules(
            env,
            &sender_record,
            &config.rules,
            &action,
            amount,
            None,
            Some(balance),
        )?;

        rule_engine::check_volume_limits(
            &sender_record,
            amount,
            config.daily_volume_limit,
            config.monthly_volume_limit,
        )?;

        let recipient_record = if sender != recipient {
            let rec = registry_client.get_identity_record(recipient);
            if !config.restricted_jurisdictions.is_empty() {
                rule_engine::check_jurisdiction_restriction(
                    &rec,
                    &config.restricted_jurisdictions,
                )?;
            }
            rec
        } else {
            sender_record.clone()
        };

        Ok((sender_record, recipient_record))
    }

    fn evaluate_rules(
        env: &Env,
        user: &IdentityRecord,
        action: &ComplianceAction,
        amount: i128,
        total_supply: Option<i128>,
        balance: Option<i128>,
    ) -> Result<(), ComplianceError> {
        let config = storage::read_compliance_config(env);
        rule_engine::evaluate_rules(
            env,
            user,
            &config.rules,
            action,
            amount,
            total_supply,
            balance,
        )
    }
}

#[contractimpl]
impl DefiPoolContract {
    /// Initialise the pool with a token, admin, and compliance config.
    pub fn __constructor(
        env: Env,
        token: Address,
        admin: Address,
        identity_registry: Address,
        required_tier: u32,
        daily_volume_limit: i128,
        monthly_volume_limit: i128,
        restricted_jurisdictions: Vec<Bytes>,
    ) {
        if storage::has_pool_config(&env) {
            panic!("already initialized");
        }

        let pool_config = PoolConfig {
            token,
            total_liquidity: 0,
            min_deposit: 1_000_000,
            admin: admin.clone(),
        };
        storage::write_pool_config(&env, &pool_config);

        <DefiPoolContract as ComplianceManager>::init_compliance(
            &env,
            admin,
            identity_registry,
            required_tier,
            daily_volume_limit,
            monthly_volume_limit,
            restricted_jurisdictions,
        );
    }

    /// Deposit tokens into the pool after compliance checks.
    ///
    /// On success, the sender's volume counters are updated in the registry.
    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), ComplianceError> {
        compliance_deposit_check!(Contract, env, from, amount);

        let mut pool_config = storage::read_pool_config(&env);
        if amount < pool_config.min_deposit {
            panic!("deposit below minimum");
        }

        let token_client = token::TokenClient::new(&env, &pool_config.token);
        token_client.transfer(&from, &env.current_contract_address(), &amount);

        pool_config.total_liquidity += amount;
        storage::write_pool_config(&env, &pool_config);

        let config = storage::read_compliance_config(&env);
        let registry_client = IdentityRegistryContractClient::new(&env, &config.identity_registry);
        registry_client.update_volume(&from, &amount);

        env.events().publish(
            (symbol_short!("evt"), symbol_short!("deposit")),
            (from, amount, pool_config.total_liquidity),
        );

        Ok(())
    }

    /// Withdraw tokens from the pool after compliance checks.
    ///
    /// On success, the sender's volume counters are updated in the registry.
    pub fn withdraw(env: Env, to: Address, amount: i128) -> Result<(), ComplianceError> {
        let pool_config = storage::read_pool_config(&env);
        compliance_withdraw_check!(Contract, env, to, to, amount);

        if amount > pool_config.total_liquidity {
            panic!("insufficient liquidity");
        }

        let token_client = token::TokenClient::new(&env, &pool_config.token);
        token_client.transfer(&env.current_contract_address(), &to, &amount);

        let mut pool_config = storage::read_pool_config(&env);
        pool_config.total_liquidity -= amount;
        storage::write_pool_config(&env, &pool_config);

        let config = storage::read_compliance_config(&env);
        let registry_client = IdentityRegistryContractClient::new(&env, &config.identity_registry);
        registry_client.update_volume(&to, &amount);

        env.events().publish(
            (symbol_short!("evt"), symbol_short!("withdraw")),
            (to, amount, pool_config.total_liquidity),
        );

        Ok(())
    }

    /// Transfer tokens between users after compliance checks.
    ///
    /// On success, the sender's volume counters are updated in the registry.
    pub fn transfer(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), ComplianceError> {
        compliance_transfer_check!(Contract, env, from, to, amount);

        let pool_config = storage::read_pool_config(&env);
        let token_client = token::TokenClient::new(&env, &pool_config.token);
        token_client.transfer(&from, &to, &amount);

        let config = storage::read_compliance_config(&env);
        let registry_client = IdentityRegistryContractClient::new(&env, &config.identity_registry);
        registry_client.update_volume(&from, &amount);

        env.events().publish(
            (symbol_short!("evt"), symbol_short!("transfer")),
            (from, to, amount),
        );

        Ok(())
    }

    /// Get the pool configuration.
    pub fn get_pool_config(env: Env) -> PoolConfig {
        storage::read_pool_config(&env)
    }

    /// Get the compliance configuration.
    pub fn get_compliance_config(env: Env) -> ComplianceConfig {
        <DefiPoolContract as ComplianceManager>::get_config(&env)
    }

    /// Add a compliance rule (owner-only).
    pub fn add_compliance_rule(env: Env, rule: ComplianceRule) {
        <DefiPoolContract as ComplianceManager>::add_rule(&env, rule);
    }

    /// Remove a compliance rule by index (owner-only).
    pub fn remove_compliance_rule(env: Env, index: u32) {
        <DefiPoolContract as ComplianceManager>::remove_rule(&env, index);
    }

    /// Pause all pool operations (owner-only).
    pub fn pause_contract(env: Env) {
        <DefiPoolContract as ComplianceManager>::pause(&env);
    }

    /// Unpause pool operations (owner-only).
    pub fn unpause_contract(env: Env) {
        <DefiPoolContract as ComplianceManager>::unpause(&env);
    }
}
