#![cfg(feature = "testutils")]

use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, Vec};

use soroban_compliance_kit::types::{Jurisdiction, KycStatus};

use crate::contract::IdentityRegistryContract;

fn setup_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let verifier = Address::generate(&env);
    let user = Address::generate(&env);
    (env, admin, verifier, user)
}

#[test]
fn test_register_identity() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let did = Bytes::from_slice(&env, b"did:example:123");
    let country_code = Bytes::from_slice(&env, b"US");

    client.register(&user, &did, &Jurisdiction::Us, &country_code, &1u32);

    let identity = client.get_identity(&user);
    assert_eq!(identity.kyc_status, KycStatus::Pending);
    assert_eq!(identity.did, did);
    assert_eq!(identity.tier, 1);
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
}

#[test]
#[should_panic(expected = "identity already registered")]
fn test_duplicate_registration_fails() {
    let (env, admin, _verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let did = Bytes::from_slice(&env, b"did:example:111");
    let country_code = Bytes::from_slice(&env, b"US");

    client.register(&user, &did, &Jurisdiction::Us, &country_code, &1u32);
    client.register(&user, &did, &Jurisdiction::Us, &country_code, &1u32);
}

#[test]
#[should_panic(expected = "unauthorized verifier")]
fn test_unauthorized_verifier_rejected() {
    let (env, admin, _verifier, user) = setup_env();
    let unauthorized = Address::generate(&env);

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let did = Bytes::from_slice(&env, b"did:example:222");
    let country_code = Bytes::from_slice(&env, b"US");

    client.register(&user, &did, &Jurisdiction::Us, &country_code, &1u32);
    client.update_kyc(&unauthorized, &user, &KycStatus::Verified);
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
fn test_update_volume() {
    let (env, admin, verifier, user) = setup_env();

    let contract_id = env.register(IdentityRegistryContract, (&admin,));
    let client = crate::contract::IdentityRegistryContractClient::new(&env, &contract_id);

    let did = Bytes::from_slice(&env, b"did:example:vol");
    let country_code = Bytes::from_slice(&env, b"US");

    client.register(&user, &did, &Jurisdiction::Us, &country_code, &1u32);
    client.add_verifier(&verifier);
    client.update_kyc(&verifier, &user, &KycStatus::Verified);

    client.update_volume(&user, &1000i128);
    let record = client.get_identity_record(&user);
    assert_eq!(record.daily_volume, 1000);
    assert_eq!(record.monthly_volume, 1000);

    client.update_volume(&user, &500i128);
    let record = client.get_identity_record(&user);
    assert_eq!(record.daily_volume, 1500);
    assert_eq!(record.monthly_volume, 1500);
}
