#![allow(deprecated)]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Bytes, Env, Vec};

use soroban_compliance_kit::types::{CustomField, Jurisdiction, KycStatus};

use crate::storage::{self, IdentityData, VolumeData};

#[contract]
pub struct IdentityRegistryContract;

#[contractimpl]
impl IdentityRegistryContract {
    /// Initialise the registry with a single admin address.
    pub fn __constructor(env: Env, admin: Address) {
        storage::write_admin(&env, &admin);
    }

    /// Register a new identity.
    ///
    /// Admin-only. The new identity starts with `KycStatus::Pending`.
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

        env.events().publish(
            (symbol_short!("evt"), symbol_short!("register")),
            (user, data.kyc_status, data.tier),
        );
    }

    /// Update a user's KYC status.
    ///
    /// Only an authorised verifier may call this.
    pub fn update_kyc(env: Env, verifier: Address, user: Address, status: KycStatus) {
        verifier.require_auth();

        if !storage::is_verifier(&env, &verifier) {
            panic!("unauthorized verifier");
        }

        let mut data = storage::read_identity(&env, &user).expect("identity not found");
        data.kyc_status = status;
        storage::write_identity(&env, &user, &data);

        env.events().publish(
            (symbol_short!("evt"), symbol_short!("kyc_updt")),
            (user, status),
        );
    }

    /// Add a new authorised verifier.
    ///
    /// The stored admin is the only caller permitted to do this.
    pub fn add_verifier(env: Env, verifier: Address) {
        let admin = storage::read_admin(&env);
        admin.require_auth();
        storage::add_verifier(&env, &verifier);

        env.events()
            .publish((symbol_short!("evt"), symbol_short!("add_vrfy")), verifier);
    }

    /// Set the list of supported jurisdictions (country codes).
    ///
    /// Admin-only.
    pub fn set_supported_jurisdictions(env: Env, jurisdictions: Vec<Bytes>) {
        let admin = storage::read_admin(&env);
        admin.require_auth();
        storage::set_supported_jurisdictions(&env, jurisdictions);
    }

    /// Update volume counters after a successful transaction.
    ///
    /// Resets daily volume if the last transaction was more than 24 hours
    /// ago, and monthly volume if more than 30 days ago.
    pub fn update_volume(env: Env, user: Address, amount: i128) {
        let vol = storage::read_volume(&env, &user);
        let now = env.ledger().timestamp();

        let daily_reset =
            vol.last_tx_timestamp > 0 && now.saturating_sub(vol.last_tx_timestamp) >= 86400;
        let monthly_reset =
            vol.last_tx_timestamp > 0 && now.saturating_sub(vol.last_tx_timestamp) >= 2_592_000;

        let updated = VolumeData {
            daily_volume: if daily_reset {
                amount
            } else {
                vol.daily_volume + amount
            },
            monthly_volume: if monthly_reset {
                amount
            } else {
                vol.monthly_volume + amount
            },
            last_tx_timestamp: now,
        };
        storage::write_volume(&env, &user, &updated);

        env.events().publish(
            (symbol_short!("evt"), symbol_short!("vol_upd")),
            (user, updated.daily_volume, updated.monthly_volume),
        );
    }

    /// Quick check: is the user's KYC status `Verified`?
    pub fn verify(env: Env, user: Address) -> bool {
        let data = storage::read_identity(&env, &user);
        match data {
            Some(identity) => identity.kyc_status == KycStatus::Verified,
            None => false,
        }
    }

    /// Get the stored identity data for a user.
    pub fn get_identity(env: Env, user: Address) -> IdentityData {
        storage::read_identity(&env, &user).expect("identity not found")
    }

    /// Get the KYC status for a user.
    pub fn get_kyc_status(env: Env, user: Address) -> KycStatus {
        let data = storage::read_identity(&env, &user).expect("identity not found");
        data.kyc_status
    }

    /// Get the full identity record including live volume counters.
    pub fn get_identity_record(
        env: Env,
        user: Address,
    ) -> soroban_compliance_kit::types::IdentityRecord {
        storage::get_identity_record(&env, &user)
    }

    /// Set or remove a custom field on a user's identity.
    ///
    /// When `value` is `None` the field is removed; otherwise it is upserted.
    /// Admin-only.
    pub fn set_custom_field(env: Env, user: Address, key: Bytes, value: Bytes) {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        let is_empty = value.len() == 0;
        let field = CustomField { key, value };
        storage::set_custom_field(&env, &user, &field, !is_empty);
    }

    /// Get a single custom field value for a user.
    pub fn get_custom_field(env: Env, user: Address, key: Bytes) -> Option<Bytes> {
        let fields: Vec<CustomField> = env
            .storage()
            .instance()
            .get(&crate::storage::DataKey::CustomFields(user))
            .unwrap_or(Vec::new(&env));
        for f in fields.iter() {
            if f.key == key {
                return Some(f.value);
            }
        }
        None
    }

    /// Get all custom fields for a user.
    pub fn get_custom_fields(env: Env, user: Address) -> Vec<CustomField> {
        env.storage()
            .instance()
            .get(&crate::storage::DataKey::CustomFields(user))
            .unwrap_or(Vec::new(&env))
    }

    /// Check whether a country code is in the supported jurisdictions list.
    pub fn is_jurisdiction_supported(env: Env, country_code: Bytes) -> bool {
        storage::is_jurisdiction_supported(&env, &country_code)
    }
}
