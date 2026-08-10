use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractevent, contractimpl, token, vec, Address, Bytes, Env, IntoVal, Symbol, Vec,
};

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

// ---------------------------------------------------------------------------
// Cross-contract helpers (identity registry)
// ---------------------------------------------------------------------------

/// Fetch a user's identity record from the registry, treating any failure
/// (unregistered identity or a registry-side error) as `KycNotVerified`.
///
/// The generated client's `try_get_identity_record` returns a nested result
/// (`Ok(Ok(record))` on success); every other outcome is collapsed here.
fn get_identity_record(
    registry_client: &IdentityRegistryContractClient,
    user: &Address,
) -> Result<IdentityRecord, ComplianceError> {
    registry_client
        .try_get_identity_record(user)
        .map_err(|_| ComplianceError::KycNotVerified)?
        .map_err(|_| ComplianceError::KycNotVerified)
}

/// Authorise this contract (as the current contract) with the registry and
/// update `user`'s volume counters by `amount`.
///
/// The registry requires the calling contract to authenticate via
/// `require_auth`; a contract can only satisfy that by emitting a
/// self-authorisation entry with [`Env::authorize_as_current_contract`]. Any
/// registry failure is surfaced as [`ComplianceError::VolumeUpdateFailed`].
fn update_volume(
    env: &Env,
    registry: &Address,
    user: &Address,
    amount: i128,
) -> Result<(), ComplianceError> {
    let pool = env.current_contract_address();
    env.authorize_as_current_contract(vec![
        env,
        InvokerContractAuthEntry::Contract(SubContractInvocation {
            context: ContractContext {
                contract: registry.clone(),
                fn_name: Symbol::new(env, "update_volume"),
                args: vec![
                    env,
                    pool.clone().into_val(env),
                    user.clone().into_val(env),
                    amount.into_val(env),
                ],
            },
            sub_invocations: vec![env],
        }),
    ]);
    IdentityRegistryContractClient::new(env, registry)
        .try_update_volume(&pool, user, &amount)
        .map_err(|_| ComplianceError::VolumeUpdateFailed)?
        .map_err(|_| ComplianceError::VolumeUpdateFailed)
}

// ---------------------------------------------------------------------------
// Typed contract events (on-chain audit log)
// ---------------------------------------------------------------------------

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolInitializedEvent {
    pub token: Address,
    pub owner: Address,
    pub identity_registry: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositEvent {
    pub from: Address,
    pub amount: i128,
    pub total_liquidity: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawEvent {
    pub to: Address,
    pub amount: i128,
    pub total_liquidity: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleAddedEvent {
    pub index: u32,
    pub rule: ComplianceRule,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleRemovedEvent {
    pub index: u32,
}

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

    fn add_rule(env: &Env, rule: ComplianceRule) -> Result<(), ComplianceError> {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();
        config.rules.push_back(rule.clone());
        let index = config.rules.len().saturating_sub(1);
        storage::write_compliance_config(env, &config);

        RuleAddedEvent { index, rule }.publish(env);
        Ok(())
    }

    fn remove_rule(env: &Env, index: u32) -> Result<(), ComplianceError> {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();

        if index as usize >= config.rules.len() as usize {
            return Err(ComplianceError::RuleIndexOutOfBounds);
        }

        let mut new_rules: Vec<ComplianceRule> = Vec::new(env);
        for (i, rule) in config.rules.iter().enumerate() {
            if i as u32 != index {
                new_rules.push_back(rule);
            }
        }
        config.rules = new_rules;
        storage::write_compliance_config(env, &config);

        RuleRemovedEvent { index }.publish(env);
        Ok(())
    }

    fn pause(env: &Env) {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();
        config.paused = true;
        storage::write_compliance_config(env, &config);

        PauseEvent { paused: true }.publish(env);
    }

    fn unpause(env: &Env) {
        let mut config = storage::read_compliance_config(env);
        config.owner.require_auth();
        config.paused = false;
        storage::write_compliance_config(env, &config);

        PauseEvent { paused: false }.publish(env);
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

        // An unknown identity is treated exactly like an unverified one: the
        // operation is refused. This keeps unregistered users from ever
        // passing the compliance gate.
        let sender_record = get_identity_record(&registry_client, sender)?;

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
        let balance = token_client.balance(sender);

        <DefiPoolContract as ComplianceManager>::evaluate_rules(
            env,
            &sender_record,
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
            let rec = get_identity_record(&registry_client, recipient)?;
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
    ///
    /// `too_many_arguments` is allowed here because the parameter list mirrors
    /// the on-chain constructor ABI consumed by the `stellar contract invoke`
    /// CLI; grouping them would change the deployed interface.
    #[allow(clippy::too_many_arguments)]
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
            token: token.clone(),
            total_liquidity: 0,
            min_deposit: 1_000_000,
            admin: admin.clone(),
        };
        storage::write_pool_config(&env, &pool_config);

        <DefiPoolContract as ComplianceManager>::init_compliance(
            &env,
            admin.clone(),
            identity_registry.clone(),
            required_tier,
            daily_volume_limit,
            monthly_volume_limit,
            restricted_jurisdictions,
        );

        PoolInitializedEvent {
            token,
            owner: admin,
            identity_registry,
        }
        .publish(&env);
    }

    /// Deposit tokens into the pool after compliance checks.
    ///
    /// On success, the sender's volume counters are updated in the registry.
    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), ComplianceError> {
        let pool_config = storage::read_pool_config(&env);
        if amount <= 0 {
            return Err(ComplianceError::InvalidAmount);
        }
        if amount < pool_config.min_deposit {
            return Err(ComplianceError::AmountBelowMinimum);
        }

        compliance_deposit_check!(Contract, env, from, amount);

        let token_client = token::TokenClient::new(&env, &pool_config.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&from, &contract_address, &amount);

        let mut pool_config = storage::read_pool_config(&env);
        pool_config.total_liquidity = pool_config.total_liquidity.saturating_add(amount);
        storage::write_pool_config(&env, &pool_config);

        let config = storage::read_compliance_config(&env);
        update_volume(&env, &config.identity_registry, &from, amount)?;

        DepositEvent {
            from,
            amount,
            total_liquidity: pool_config.total_liquidity,
        }
        .publish(&env);

        Ok(())
    }

    /// Withdraw tokens from the pool after compliance checks.
    ///
    /// On success, the sender's volume counters are updated in the registry.
    pub fn withdraw(env: Env, to: Address, amount: i128) -> Result<(), ComplianceError> {
        let pool_config = storage::read_pool_config(&env);
        if amount <= 0 {
            return Err(ComplianceError::InvalidAmount);
        }
        if amount > pool_config.total_liquidity {
            return Err(ComplianceError::InsufficientLiquidity);
        }

        compliance_withdraw_check!(Contract, env, to, to, amount);

        let token_client = token::TokenClient::new(&env, &pool_config.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &to, &amount);

        let mut pool_config = storage::read_pool_config(&env);
        pool_config.total_liquidity = pool_config.total_liquidity.saturating_sub(amount);
        storage::write_pool_config(&env, &pool_config);

        let config = storage::read_compliance_config(&env);
        update_volume(&env, &config.identity_registry, &to, amount)?;

        WithdrawEvent {
            to,
            amount,
            total_liquidity: pool_config.total_liquidity,
        }
        .publish(&env);

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
        if amount <= 0 {
            return Err(ComplianceError::InvalidAmount);
        }
        compliance_transfer_check!(Contract, env, from, to, amount);

        let pool_config = storage::read_pool_config(&env);
        let token_client = token::TokenClient::new(&env, &pool_config.token);
        token_client.transfer(&from, &to, &amount);

        let config = storage::read_compliance_config(&env);
        update_volume(&env, &config.identity_registry, &from, amount)?;

        TransferEvent { from, to, amount }.publish(&env);

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

    /// Replace the full compliance configuration (owner-only).
    ///
    /// Exposes [`ComplianceManager::set_config`] so a deployed pool's limits,
    /// registry and restricted jurisdictions can be governed without a
    /// redeploy. The supplied config replaces the current one wholesale;
    /// prefer `add_compliance_rule` / `remove_compliance_rule` for
    /// incremental rule changes.
    pub fn set_compliance_config(env: Env, config: ComplianceConfig) {
        <DefiPoolContract as ComplianceManager>::set_config(&env, config);
    }

    /// Add a compliance rule (owner-only).
    pub fn add_compliance_rule(env: Env, rule: ComplianceRule) -> Result<(), ComplianceError> {
        <DefiPoolContract as ComplianceManager>::add_rule(&env, rule)
    }

    /// Remove a compliance rule by index (owner-only).
    pub fn remove_compliance_rule(env: Env, index: u32) -> Result<(), ComplianceError> {
        <DefiPoolContract as ComplianceManager>::remove_rule(&env, index)
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
