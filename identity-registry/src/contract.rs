use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Vec};

use soroban_compliance_kit::types::{Jurisdiction, KycStatus};

use crate::storage::{self, IdentityData};

#[contract]
pub struct IdentityRegistryContract;

#[contractimpl]
impl IdentityRegistryContract {
    pub fn __constructor(env: Env, admin: Address) {
        storage::write_admin(&env, &admin);
    }

    pub fn register(
        env: Env,
        user: Address,
        did: Bytes,
        jurisdiction: Jurisdiction,
        country_code: Bytes,
        tier: u32,
    ) {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        if storage::has_identity(&env, &user) {
            panic!("identity already registered");
        }

        let data = IdentityData {
            did,
            kyc_status: KycStatus::Pending,
            jurisdiction,
            country_code,
            tier,
        };
        storage::write_identity(&env, &user, &data);
    }

    pub fn update_kyc(env: Env, verifier: Address, user: Address, status: KycStatus) {
        verifier.require_auth();

        if !storage::is_verifier(&env, &verifier) {
            panic!("unauthorized verifier");
        }

        let mut data = storage::read_identity(&env, &user).expect("identity not found");
        data.kyc_status = status;
        storage::write_identity(&env, &user, &data);
    }

    pub fn add_verifier(env: Env, admin: Address, verifier: Address) {
        admin.require_auth();
        let stored_admin = storage::read_admin(&env);
        if admin != stored_admin {
            panic!("only admin can add verifiers");
        }
        storage::add_verifier(&env, &verifier);
    }

    pub fn set_supported_jurisdictions(env: Env, admin: Address, jurisdictions: Vec<Bytes>) {
        admin.require_auth();
        let stored_admin = storage::read_admin(&env);
        if admin != stored_admin {
            panic!("only admin can set jurisdictions");
        }
        storage::set_supported_jurisdictions(&env, jurisdictions);
    }

    pub fn verify(env: Env, user: Address) -> bool {
        let data = storage::read_identity(&env, &user);
        match data {
            Some(identity) => identity.kyc_status == KycStatus::Verified,
            None => false,
        }
    }

    pub fn get_identity(env: Env, user: Address) -> IdentityData {
        storage::read_identity(&env, &user).expect("identity not found")
    }

    pub fn get_kyc_status(env: Env, user: Address) -> KycStatus {
        let data = storage::read_identity(&env, &user).expect("identity not found");
        data.kyc_status
    }

    pub fn get_identity_record(env: Env, user: Address) -> soroban_compliance_kit::types::IdentityRecord {
        storage::get_identity_record(&env, &user)
    }

    pub fn is_jurisdiction_supported(env: Env, country_code: Bytes) -> bool {
        storage::is_jurisdiction_supported(&env, &country_code)
    }
}
