use soroban_sdk::{contract, contracterror, contractevent, contractimpl, Address, Bytes, Env, Vec};

use soroban_compliance_kit::types::{CustomField, Jurisdiction, KycStatus};

use crate::storage::{self, IdentityData, VolumeData};

#[contract]
pub struct IdentityRegistryContract;

// ---------------------------------------------------------------------------
// Typed contract events (SEP-57 compliant on-chain logging)
// ---------------------------------------------------------------------------

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterEvent {
    pub user: Address,
    pub kyc_status: KycStatus,
    pub tier: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KycUpdatedEvent {
    pub user: Address,
    pub status: KycStatus,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierAddedEvent {
    pub verifier: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierRemovedEvent {
    pub verifier: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerAddedEvent {
    pub caller: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerRemovedEvent {
    pub caller: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JurisdictionsUpdatedEvent {
    pub jurisdictions: Vec<Bytes>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VolumeUpdatedEvent {
    pub user: Address,
    pub daily_volume: i128,
    pub monthly_volume: i128,
}

// ---------------------------------------------------------------------------
// Typed contract errors
// ---------------------------------------------------------------------------

/// Errors surfaced by the identity registry.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryError {
    IdentityAlreadyRegistered = 1,
    IdentityNotFound = 2,
    UnauthorizedVerifier = 3,
    UnauthorizedCaller = 4,
    InvalidDid = 5,
    InvalidCountryCode = 6,
    InvalidTier = 7,
    InvalidAmount = 8,
    DuplicateVerifier = 9,
    DuplicateCaller = 10,
    VerifierNotFound = 11,
    CallerNotFound = 12,
    InvalidCustomFieldKey = 13,
}

/// True when `cc` is a valid ISO 3166-1 alpha-2 country code (2 uppercase ASCII letters).
fn is_valid_country_code(cc: &Bytes) -> bool {
    cc.len() == 2 && cc.iter().all(|b| b.is_ascii_uppercase())
}

#[contractimpl]
impl IdentityRegistryContract {
    /// Initialise the registry with a single admin address.
    ///
    /// The constructor is invoked once at deployment; a second invocation is
    /// rejected so the admin can never be silently overwritten.
    pub fn __constructor(env: Env, admin: Address) {
        if storage::has_admin(&env) {
            panic!("registry already initialized");
        }
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
    ) -> Result<(), RegistryError> {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        if did.is_empty() {
            return Err(RegistryError::InvalidDid);
        }
        if !is_valid_country_code(&country_code) {
            return Err(RegistryError::InvalidCountryCode);
        }
        if tier == 0 {
            return Err(RegistryError::InvalidTier);
        }
        if storage::has_identity(&env, &user) {
            return Err(RegistryError::IdentityAlreadyRegistered);
        }

        let data =
            IdentityData { did, kyc_status: KycStatus::Pending, jurisdiction, country_code, tier };
        storage::write_identity(&env, &user, &data);

        RegisterEvent { user, kyc_status: data.kyc_status, tier: data.tier }.publish(&env);
        Ok(())
    }

    /// Update a user's KYC status.
    ///
    /// Only an authorised verifier may call this.
    pub fn update_kyc(
        env: Env,
        verifier: Address,
        user: Address,
        status: KycStatus,
    ) -> Result<(), RegistryError> {
        verifier.require_auth();

        if !storage::is_verifier(&env, &verifier) {
            return Err(RegistryError::UnauthorizedVerifier);
        }

        let mut data = match storage::read_identity(&env, &user) {
            Some(data) => data,
            None => return Err(RegistryError::IdentityNotFound),
        };
        data.kyc_status = status;
        storage::write_identity(&env, &user, &data);

        KycUpdatedEvent { user, status }.publish(&env);
        Ok(())
    }

    /// Add a new authorised verifier.
    ///
    /// The stored admin is the only caller permitted to do this.
    pub fn add_verifier(env: Env, verifier: Address) -> Result<(), RegistryError> {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        if storage::is_verifier(&env, &verifier) {
            return Err(RegistryError::DuplicateVerifier);
        }
        storage::add_verifier(&env, &verifier);

        VerifierAddedEvent { verifier }.publish(&env);
        Ok(())
    }

    /// Remove a verifier (admin-only). Revokes KYC-update privileges immediately.
    pub fn remove_verifier(env: Env, verifier: Address) -> Result<(), RegistryError> {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        if !storage::is_verifier(&env, &verifier) {
            return Err(RegistryError::VerifierNotFound);
        }
        storage::remove_verifier(&env, &verifier);

        VerifierRemovedEvent { verifier }.publish(&env);
        Ok(())
    }

    /// Authorise a contract (e.g. a DeFi pool) to update volume counters.
    ///
    /// Admin-only. This is the access-control gate for `update_volume`:
    /// only the admin and whitelisted callers may mutate volume state.
    pub fn add_authorized_caller(env: Env, caller: Address) -> Result<(), RegistryError> {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        if storage::is_authorized_caller(&env, &caller) {
            return Err(RegistryError::DuplicateCaller);
        }
        storage::add_authorized_caller(&env, &caller);

        CallerAddedEvent { caller }.publish(&env);
        Ok(())
    }

    /// Revoke a contract's permission to update volume counters (admin-only).
    pub fn remove_authorized_caller(env: Env, caller: Address) -> Result<(), RegistryError> {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        if !storage::is_authorized_caller(&env, &caller) {
            return Err(RegistryError::CallerNotFound);
        }
        storage::remove_authorized_caller(&env, &caller);

        CallerRemovedEvent { caller }.publish(&env);
        Ok(())
    }

    /// View the current verifier allow-list (who may update KYC status).
    pub fn get_verifiers(env: Env) -> Vec<Address> {
        storage::read_verifiers(&env)
    }

    /// View the current authorized-caller allow-list (who may update volume).
    pub fn get_authorized_callers(env: Env) -> Vec<Address> {
        storage::read_authorized_callers(&env)
    }

    /// Set the list of supported jurisdictions (country codes). Admin-only.
    pub fn set_supported_jurisdictions(env: Env, jurisdictions: Vec<Bytes>) {
        let admin = storage::read_admin(&env);
        admin.require_auth();
        storage::set_supported_jurisdictions(&env, jurisdictions.clone());

        JurisdictionsUpdatedEvent { jurisdictions }.publish(&env);
    }

    /// Update volume counters after a successful transaction.
    ///
    /// Only the admin or a whitelisted caller (see `add_authorized_caller`)
    /// may invoke this. `caller` is the address performing the update and must
    /// authenticate with `require_auth`; amounts must be non-negative.
    ///
    /// Resets daily volume if the last transaction was more than 24 hours ago,
    /// and monthly volume if more than 30 days ago.
    pub fn update_volume(
        env: Env,
        caller: Address,
        user: Address,
        amount: i128,
    ) -> Result<(), RegistryError> {
        if amount < 0 {
            return Err(RegistryError::InvalidAmount);
        }

        caller.require_auth();

        let admin = storage::read_admin(&env);
        if caller != admin && !storage::is_authorized_caller(&env, &caller) {
            return Err(RegistryError::UnauthorizedCaller);
        }

        if !storage::has_identity(&env, &user) {
            return Err(RegistryError::IdentityNotFound);
        }

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
                vol.daily_volume.saturating_add(amount)
            },
            monthly_volume: if monthly_reset {
                amount
            } else {
                vol.monthly_volume.saturating_add(amount)
            },
            last_tx_timestamp: now,
        };
        storage::write_volume(&env, &user, &updated);

        VolumeUpdatedEvent {
            user,
            daily_volume: updated.daily_volume,
            monthly_volume: updated.monthly_volume,
        }
        .publish(&env);
        Ok(())
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
    pub fn get_identity(env: Env, user: Address) -> Result<IdentityData, RegistryError> {
        storage::read_identity(&env, &user).ok_or(RegistryError::IdentityNotFound)
    }

    /// Get the KYC status for a user.
    pub fn get_kyc_status(env: Env, user: Address) -> Result<KycStatus, RegistryError> {
        storage::read_identity(&env, &user)
            .map(|data| data.kyc_status)
            .ok_or(RegistryError::IdentityNotFound)
    }

    /// Get the full identity record including live volume counters.
    ///
    /// Returns [`RegistryError::IdentityNotFound`] when `user` has no
    /// registered identity, so downstream contracts can handle the failure
    /// explicitly instead of relying on a cross-contract panic.
    pub fn get_identity_record(
        env: Env,
        user: Address,
    ) -> Result<soroban_compliance_kit::types::IdentityRecord, RegistryError> {
        storage::get_identity_record(&env, &user).ok_or(RegistryError::IdentityNotFound)
    }

    /// Set or remove a custom field on a user's identity.
    ///
    /// When `value` is empty the field is removed; otherwise it is upserted.
    /// Admin-only.
    pub fn set_custom_field(
        env: Env,
        user: Address,
        key: Bytes,
        value: Bytes,
    ) -> Result<(), RegistryError> {
        let admin = storage::read_admin(&env);
        admin.require_auth();

        if key.is_empty() {
            return Err(RegistryError::InvalidCustomFieldKey);
        }

        let add = !value.is_empty();
        storage::set_custom_field(&env, &user, &CustomField { key, value }, add);
        Ok(())
    }

    /// Get a single custom field value for a user.
    pub fn get_custom_field(env: Env, user: Address, key: Bytes) -> Option<Bytes> {
        let fields = storage::read_custom_fields(&env, &user);
        for f in fields.iter() {
            if f.key == key {
                return Some(f.value);
            }
        }
        None
    }

    /// Get all custom fields for a user.
    pub fn get_custom_fields(env: Env, user: Address) -> Vec<CustomField> {
        storage::read_custom_fields(&env, &user)
    }

    /// Check whether a country code is in the supported jurisdictions list.
    pub fn is_jurisdiction_supported(env: Env, country_code: Bytes) -> bool {
        storage::is_jurisdiction_supported(&env, &country_code)
    }
}
