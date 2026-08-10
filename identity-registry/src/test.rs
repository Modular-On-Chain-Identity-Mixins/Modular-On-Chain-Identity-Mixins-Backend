#![cfg(all(test, feature = "testutils"))]

use soroban_sdk::{
    testutils::Address as _, testutils::Events, testutils::Ledger, Address, Bytes, Env,
    InvokeError, Vec,
};

use soroban_compliance_kit::types::{Jurisdiction, KycStatus};

use crate::contract::{IdentityRegistryContract, RegistryError};

/// Extract the typed contract error from a `try_<fn>` client call result.
///
/// Generated clients return `Result<Result<T, CE>, Result<E, InvokeError>>`
/// where the contract's typed error `E` sits in the outer `Err`'s `Ok` slot.
/// This helper unwraps that and panics on any other outcome (host errors,
/// conversion errors, or an unexpected success).
fn contract_err<T, CE, E: core::fmt::Debug>(
    result: Result<Result<T, CE>, Result<E, InvokeError>>,
) -> E {
    match result {
        Err(Ok(e)) => e,
        Err(Err(ie)) => panic!("expected a contract error, got an invoke error: {ie:?}"),
        Ok(_) => panic!("expected a contract error, got a success result"),
    }
}

/// Test fixture: a fresh env with mocked auth, an admin, a verifier and a user.
fn setup_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let verifier = Address::generate(&env);
    let user = Address::generate(&env);
    (env, admin, verifier, user)
}

fn us_cc(env: &Env) -> Bytes {
    Bytes::from_slice(env, b"US")
}

fn register_user(
    client: &crate::contract::IdentityRegistryContractClient,
    env: &Env,
    user: &Address,
    tier: u32,
) {
    let did = Bytes::from_slice(env, b"did:example:123");
    client.register(user, &did, &Jurisdiction::Us, &us_cc(env), &tier);
}

#[test]
fn test_register_identity() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let did = Bytes::from_slice(&env, b"did:example:123");
    let country_code = Bytes::from_slice(&env, b"US");

    client.register(&user, &did, &Jurisdiction::Us, &country_code, &1u32);

    // A registration emits an audit event. Note: assert immediately after the
    // emitting call — subsequent client calls in the same test can roll back
    // the events buffer, so a later assertion would see an empty list.
    assert_eq!(env.events().all().events().len(), 1);

    let identity = client.get_identity(&user);
    assert_eq!(identity.kyc_status, KycStatus::Pending);
    assert_eq!(identity.did, did);
    assert_eq!(identity.tier, 1);
}

#[test]
fn test_register_validation_errors() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    // Empty DID is rejected.
    let empty_did = Bytes::from_slice(&env, b"");
    let err = contract_err(client.try_register(
        &user,
        &empty_did,
        &Jurisdiction::Us,
        &us_cc(&env),
        &1u32,
    ));
    assert_eq!(err, RegistryError::InvalidDid);

    // Lower-case / malformed country codes are rejected.
    let bad_cc = Bytes::from_slice(&env, b"us");
    let did = Bytes::from_slice(&env, b"did:example:lower");
    let err = contract_err(client.try_register(&user, &did, &Jurisdiction::Us, &bad_cc, &1u32));
    assert_eq!(err, RegistryError::InvalidCountryCode);

    let long_cc = Bytes::from_slice(&env, b"USA");
    let err = contract_err(client.try_register(&user, &did, &Jurisdiction::Us, &long_cc, &1u32));
    assert_eq!(err, RegistryError::InvalidCountryCode);

    // Tier 0 is rejected.
    let err =
        contract_err(client.try_register(&user, &did, &Jurisdiction::Us, &us_cc(&env), &0u32));
    assert_eq!(err, RegistryError::InvalidTier);

    // Nothing was persisted.
    let err = contract_err(client.try_get_identity(&user));
    assert_eq!(err, RegistryError::IdentityNotFound);
}

#[test]
fn test_duplicate_registration_fails() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    let err = contract_err(client.try_register(
        &user,
        &Bytes::from_slice(&env, b"did:example:123"),
        &Jurisdiction::Us,
        &us_cc(&env),
        &1u32,
    ));
    assert_eq!(err, RegistryError::IdentityAlreadyRegistered);
}

#[test]
fn test_kyc_verification_flow() {
    let (env, admin, verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let did = Bytes::from_slice(&env, b"did:example:456");
    let country_code = Bytes::from_slice(&env, b"EU");

    client.register(&user, &did, &Jurisdiction::Eu, &country_code, &2u32);
    client.add_verifier(&verifier);

    assert!(!client.verify(&user));

    client.update_kyc(&verifier, &user, &KycStatus::Verified);

    assert!(client.verify(&user));
    assert_eq!(client.get_kyc_status(&user), KycStatus::Verified);
}

#[test]
fn test_unauthorized_verifier_rejected() {
    let (env, admin, _verifier, user) = setup_env();
    let unauthorized = Address::generate(&env);

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    let err = contract_err(client.try_update_kyc(&unauthorized, &user, &KycStatus::Verified));
    assert_eq!(err, RegistryError::UnauthorizedVerifier);

    // The status was not changed.
    assert!(!client.verify(&user));
}

#[test]
fn test_update_kyc_unknown_identity_fails() {
    let (env, admin, verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    client.add_verifier(&verifier);

    let err = contract_err(client.try_update_kyc(&verifier, &user, &KycStatus::Verified));
    assert_eq!(err, RegistryError::IdentityNotFound);
}

#[test]
fn test_verifier_lifecycle() {
    let (env, admin, verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    // Duplicate verifier is rejected.
    client.add_verifier(&verifier);
    let err = contract_err(client.try_add_verifier(&verifier));
    assert_eq!(err, RegistryError::DuplicateVerifier);

    // Removing a verifier that is not present is rejected.
    let ghost = Address::generate(&env);
    let err = contract_err(client.try_remove_verifier(&ghost));
    assert_eq!(err, RegistryError::VerifierNotFound);

    // Revoking the verifier immediately blocks KYC updates.
    client.remove_verifier(&verifier);
    register_user(&client, &env, &user, 1);
    let err = contract_err(client.try_update_kyc(&verifier, &user, &KycStatus::Verified));
    assert_eq!(err, RegistryError::UnauthorizedVerifier);
}

#[test]
fn test_jurisdiction_support() {
    let (env, admin, _verifier, _user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let us = Bytes::from_slice(&env, b"US");
    let eu = Bytes::from_slice(&env, b"EU");

    let jurisdictions: Vec<Bytes> = Vec::from_array(&env, [us.clone(), eu.clone()]);
    client.set_supported_jurisdictions(&jurisdictions);

    let cn = Bytes::from_slice(&env, b"CN");
    assert!(client.is_jurisdiction_supported(&us));
    assert!(client.is_jurisdiction_supported(&eu));
    assert!(!client.is_jurisdiction_supported(&cn));
}

#[test]
fn test_get_identity_record() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let did = Bytes::from_slice(&env, b"did:example:333");
    let country_code = Bytes::from_slice(&env, b"US");

    client.register(&user, &did, &Jurisdiction::Us, &country_code, &1u32);

    let record = client.get_identity_record(&user);
    assert_eq!(record.did, did);
    assert_eq!(record.kyc_status, KycStatus::Pending);
    assert_eq!(record.tier, 1);
    assert_eq!(record.daily_volume, 0);
    assert_eq!(record.monthly_volume, 0);
}

#[test]
fn test_get_identity_record_unknown_user() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let err = contract_err(client.try_get_identity_record(&user));
    assert_eq!(err, RegistryError::IdentityNotFound);
}

#[test]
fn test_update_volume_authorized() {
    let (env, admin, verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);
    client.add_verifier(&verifier);
    client.update_kyc(&verifier, &user, &KycStatus::Verified);

    // The admin may always update volume counters.
    client.update_volume(&admin, &user, &1000i128);
    let record = client.get_identity_record(&user);
    assert_eq!(record.daily_volume, 1000);
    assert_eq!(record.monthly_volume, 1000);

    client.update_volume(&admin, &user, &500i128);
    let record = client.get_identity_record(&user);
    assert_eq!(record.daily_volume, 1500);
    assert_eq!(record.monthly_volume, 1500);
}

#[test]
fn test_update_volume_requires_authorized_caller() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    // A random contract (the user) is not allowed to bump volume counters.
    let err = contract_err(client.try_update_volume(&user, &user, &1000i128));
    assert_eq!(err, RegistryError::UnauthorizedCaller);
}

#[test]
fn test_authorized_caller_lifecycle() {
    let (env, admin, _verifier, user) = setup_env();
    let pool = Address::generate(&env);

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    // Adding a caller enables it to update volume.
    client.add_authorized_caller(&pool);
    client.update_volume(&pool, &user, &250i128);
    assert_eq!(client.get_identity_record(&user).daily_volume, 250);

    // Duplicates are rejected.
    let err = contract_err(client.try_add_authorized_caller(&pool));
    assert_eq!(err, RegistryError::DuplicateCaller);

    // Removing a caller that is not present is rejected.
    let ghost = Address::generate(&env);
    let err = contract_err(client.try_remove_authorized_caller(&ghost));
    assert_eq!(err, RegistryError::CallerNotFound);

    // Revocation immediately blocks volume updates.
    client.remove_authorized_caller(&pool);
    let err = contract_err(client.try_update_volume(&pool, &user, &100i128));
    assert_eq!(err, RegistryError::UnauthorizedCaller);
}

#[test]
fn test_update_volume_unknown_identity_fails() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let err = contract_err(client.try_update_volume(&admin, &user, &100i128));
    assert_eq!(err, RegistryError::IdentityNotFound);
}

#[test]
fn test_update_volume_rejects_negative_amount() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    let err = contract_err(client.try_update_volume(&admin, &user, &(-1i128)));
    assert_eq!(err, RegistryError::InvalidAmount);
}

#[test]
fn test_volume_reset_after_24h() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    env.ledger().set_timestamp(1_000_000);
    client.update_volume(&admin, &user, &1000i128);

    // 25 hours later the daily counter resets, monthly does not (30 days).
    env.ledger().set_timestamp(1_000_000 + 90_000);
    client.update_volume(&admin, &user, &500i128);

    let record = client.get_identity_record(&user);
    assert_eq!(record.daily_volume, 500, "daily volume resets after 24h");
    assert_eq!(record.monthly_volume, 1500, "monthly volume accumulates");
}

#[test]
fn test_volume_reset_after_30d() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    env.ledger().set_timestamp(1_000_000);
    client.update_volume(&admin, &user, &1000i128);

    // 31 days later both counters reset.
    env.ledger().set_timestamp(1_000_000 + 31 * 86_400);
    client.update_volume(&admin, &user, &500i128);

    let record = client.get_identity_record(&user);
    assert_eq!(record.daily_volume, 500);
    assert_eq!(record.monthly_volume, 500);
}

#[test]
fn test_custom_fields() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    let key = Bytes::from_slice(&env, b"risk_score");
    let val = Bytes::from_slice(&env, b"low");
    client.set_custom_field(&user, &key, &val);

    let result = client.get_custom_field(&user, &key);
    assert_eq!(result, Some(val));

    let all_fields = client.get_custom_fields(&user);
    assert_eq!(all_fields.len(), 1);

    let record = client.get_identity_record(&user);
    assert_eq!(record.custom_fields.len(), 1);
    assert_eq!(
        record.custom_fields.get(0).unwrap().value,
        Bytes::from_slice(&env, b"low")
    );

    // Upserting the same key replaces it.
    let val2 = Bytes::from_slice(&env, b"high");
    client.set_custom_field(&user, &key, &val2);
    assert_eq!(client.get_custom_fields(&user).len(), 1);
    assert_eq!(
        client.get_custom_field(&user, &key),
        Some(Bytes::from_slice(&env, b"high"))
    );

    // An empty value removes the field.
    let empty_val = Bytes::from_slice(&env, b"");
    client.set_custom_field(&user, &key, &empty_val);
    let result = client.get_custom_field(&user, &key);
    assert_eq!(result, None);
}

#[test]
fn test_custom_field_empty_key_rejected() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    register_user(&client, &env, &user, 1);

    let empty_key = Bytes::from_slice(&env, b"");
    let val = Bytes::from_slice(&env, b"x");
    let err = contract_err(client.try_set_custom_field(&user, &empty_key, &val));
    assert_eq!(err, RegistryError::InvalidCustomFieldKey);
}

#[test]
fn test_verify_unregistered_user_returns_false() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    assert!(!client.verify(&user));
}
