#![cfg(all(test, feature = "testutils"))]

use soroban_sdk::{testutils::Address as _, token, Address, Bytes, Env, Vec};

use soroban_compliance_kit::types::{
    ComplianceAction, ComplianceRule, Jurisdiction, KycStatus, RuleField, RuleOperator, RuleValue,
};

use crate::contract::DefiPoolContract;
use identity_registry::contract::IdentityRegistryContract;

fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let registry_id = env.register(IdentityRegistryContract, (&admin,));
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&admin, &1_000_000_000_000);
    (env, admin, registry_id, token_id)
}

#[test]
fn test_deposit_and_withdraw() {
    let (env, admin, registry_id, token_id) = setup();
    let registry_client =
        identity_registry::contract::IdentityRegistryContractClient::new(&env, &registry_id);

    let user1 = Address::generate(&env);
    let did1 = Bytes::from_slice(&env, b"did:example:user1");
    let cc = Bytes::from_slice(&env, b"US");
    registry_client.register(&user1, &did1, &Jurisdiction::Us, &cc, &2u32);
    registry_client.add_verifier(&admin);
    registry_client.update_kyc(&admin, &user1, &KycStatus::Verified);

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    // The pool must be whitelisted by the registry before it can update
    // volume counters (the production flow for any regulated contract).
    registry_client.add_authorized_caller(&pool_id);

    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&user1, &1_000_000_000_000);

    pool_client.deposit(&user1, &100_000_000_000i128);
    assert_eq!(
        pool_client.get_pool_config().total_liquidity,
        100_000_000_000
    );

    pool_client.withdraw(&user1, &50_000_000_000i128);
    assert_eq!(
        pool_client.get_pool_config().total_liquidity,
        50_000_000_000
    );
}

#[test]
fn test_transfer() {
    let (env, admin, registry_id, token_id) = setup();
    let registry_client =
        identity_registry::contract::IdentityRegistryContractClient::new(&env, &registry_id);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let did1 = Bytes::from_slice(&env, b"did:example:user1");
    let did2 = Bytes::from_slice(&env, b"did:example:user2");
    let us = Bytes::from_slice(&env, b"US");
    let eu = Bytes::from_slice(&env, b"EU");
    registry_client.register(&user1, &did1, &Jurisdiction::Us, &us, &2u32);
    registry_client.register(&user2, &did2, &Jurisdiction::Eu, &eu, &2u32);
    registry_client.add_verifier(&admin);
    registry_client.update_kyc(&admin, &user1, &KycStatus::Verified);
    registry_client.update_kyc(&admin, &user2, &KycStatus::Verified);

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    // The pool must be whitelisted by the registry before it can update
    // volume counters (the production flow for any regulated contract).
    registry_client.add_authorized_caller(&pool_id);

    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&user1, &1_000_000_000_000);

    pool_client.transfer(&user1, &user2, &10_000_000_000i128);
}

#[test]
fn test_set_compliance_config() {
    let (env, admin, registry_id, token_id) = setup();

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    // The owner can replace the full compliance configuration (same
    // `owner.require_auth()` gate as add_rule / remove_rule / pause).
    let config = pool_client.get_compliance_config();
    let mut updated = config.clone();
    updated.daily_volume_limit = 42;
    pool_client.set_compliance_config(&updated);
    assert_eq!(pool_client.get_compliance_config().daily_volume_limit, 42);
    assert_eq!(
        pool_client.get_compliance_config().monthly_volume_limit,
        10_000_000_000_000
    );
}

#[test]
fn test_pause_unpause() {
    let (env, admin, registry_id, token_id) = setup();

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    pool_client.pause_contract();
    assert!(pool_client.get_compliance_config().paused);

    pool_client.unpause_contract();
    assert!(!pool_client.get_compliance_config().paused);
}

#[test]
fn test_deposit_below_minimum_fails() {
    let (env, admin, registry_id, token_id) = setup();
    let registry_client =
        identity_registry::contract::IdentityRegistryContractClient::new(&env, &registry_id);

    let user1 = Address::generate(&env);
    let did1 = Bytes::from_slice(&env, b"did:example:user1");
    let cc = Bytes::from_slice(&env, b"US");
    registry_client.register(&user1, &did1, &Jurisdiction::Us, &cc, &2u32);
    registry_client.add_verifier(&admin);
    registry_client.update_kyc(&admin, &user1, &KycStatus::Verified);

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&user1, &1_000_000_000_000);

    assert!(pool_client.try_deposit(&user1, &100_000i128).is_err());
}

#[test]
fn test_compliance_rule_enforcement() {
    let (env, admin, registry_id, token_id) = setup();

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    let rule = ComplianceRule {
        field: RuleField::Tier,
        operator: RuleOperator::Gte,
        value: RuleValue::Single(Bytes::from_slice(&env, &[0u8; 15])),
        action_filter: ComplianceAction::Any,
    };
    pool_client.add_compliance_rule(&rule);

    assert_eq!(pool_client.get_compliance_config().rules.len(), 1);
}

#[test]
fn test_withdraw_insufficient_liquidity_fails() {
    let (env, admin, registry_id, token_id) = setup();
    let registry_client =
        identity_registry::contract::IdentityRegistryContractClient::new(&env, &registry_id);

    let user1 = Address::generate(&env);
    let did1 = Bytes::from_slice(&env, b"did:example:user1");
    let cc = Bytes::from_slice(&env, b"US");
    registry_client.register(&user1, &did1, &Jurisdiction::Us, &cc, &2u32);
    registry_client.add_verifier(&admin);
    registry_client.update_kyc(&admin, &user1, &KycStatus::Verified);

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    assert!(pool_client
        .try_withdraw(&user1, &999_999_999_999_999i128)
        .is_err());
}

#[test]
fn test_deposit_without_kyc_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user_no_kyc = Address::generate(&env);

    let registry_id = env.register(IdentityRegistryContract, (&admin,));
    let registry_client =
        identity_registry::contract::IdentityRegistryContractClient::new(&env, &registry_id);

    let did = Bytes::from_slice(&env, b"did:example:nokyc");
    let country_code = Bytes::from_slice(&env, b"US");

    registry_client.register(&user_no_kyc, &did, &Jurisdiction::Us, &country_code, &1u32);
    registry_client.add_verifier(&admin);
    registry_client.update_kyc(&admin, &user_no_kyc, &KycStatus::Pending);

    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&user_no_kyc, &1_000_000_000_000);

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    assert!(pool_client
        .try_deposit(&user_no_kyc, &100_000_000_000i128)
        .is_err());
}

#[test]
fn test_negative_amounts_rejected() {
    let (env, admin, registry_id, token_id) = setup();
    let registry_client =
        identity_registry::contract::IdentityRegistryContractClient::new(&env, &registry_id);

    let user1 = Address::generate(&env);
    let did1 = Bytes::from_slice(&env, b"did:example:neg1");
    let cc = Bytes::from_slice(&env, b"US");
    registry_client.register(&user1, &did1, &Jurisdiction::Us, &cc, &2u32);
    registry_client.add_verifier(&admin);
    registry_client.update_kyc(&admin, &user1, &KycStatus::Verified);

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);
    registry_client.add_authorized_caller(&pool_id);

    let user2 = Address::generate(&env);
    let did2 = Bytes::from_slice(&env, b"did:example:neg2");
    let eu = Bytes::from_slice(&env, b"EU");
    registry_client.register(&user2, &did2, &Jurisdiction::Eu, &eu, &2u32);
    registry_client.update_kyc(&admin, &user2, &KycStatus::Verified);

    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&user1, &1_000_000_000_000);

    assert!(pool_client.try_deposit(&user1, &(-100i128)).is_err());
    assert!(pool_client.try_withdraw(&user1, &(-100i128)).is_err());
    assert!(pool_client
        .try_transfer(&user1, &user2, &(-100i128))
        .is_err());
}

#[test]
fn test_volume_tracking_after_deposit() {
    let (env, admin, registry_id, token_id) = setup();
    let registry_client =
        identity_registry::contract::IdentityRegistryContractClient::new(&env, &registry_id);

    let user1 = Address::generate(&env);
    let did1 = Bytes::from_slice(&env, b"did:example:vol1");
    let cc = Bytes::from_slice(&env, b"US");
    registry_client.register(&user1, &did1, &Jurisdiction::Us, &cc, &2u32);
    registry_client.add_verifier(&admin);
    registry_client.update_kyc(&admin, &user1, &KycStatus::Verified);

    let pool_id = env.register(
        DefiPoolContract,
        (
            &token_id,
            &admin,
            &registry_id,
            &1u32,
            &1_000_000_000_000i128,
            &10_000_000_000_000i128,
            &Vec::<Bytes>::new(&env),
        ),
    );
    let pool_client = crate::contract::DefiPoolContractClient::new(&env, &pool_id);

    // The pool must be whitelisted by the registry before it can update
    // volume counters (the production flow for any regulated contract).
    registry_client.add_authorized_caller(&pool_id);

    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&user1, &1_000_000_000_000);

    pool_client.deposit(&user1, &100_000_000_000i128);

    let record = registry_client.get_identity_record(&user1);
    assert_eq!(record.daily_volume, 100_000_000_000);
    assert_eq!(record.monthly_volume, 100_000_000_000);

    pool_client.deposit(&user1, &50_000_000_000i128);

    let record = registry_client.get_identity_record(&user1);
    assert_eq!(record.daily_volume, 150_000_000_000);
    assert_eq!(record.monthly_volume, 150_000_000_000);
}
