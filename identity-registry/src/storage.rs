use soroban_sdk::{contracttype, Address, Bytes, Env, Vec};

use soroban_compliance_kit::types::{CustomField, IdentityRecord, Jurisdiction, KycStatus};

/// On-chain identity data (excluding volume counters and custom fields).
#[contracttype]
pub struct IdentityData {
    pub did: Bytes,
    pub kyc_status: KycStatus,
    pub jurisdiction: Jurisdiction,
    pub country_code: Bytes,
    pub tier: u32,
}

/// Per-user volume counters, reset on a 24h / 30d cadence.
#[contracttype]
pub struct VolumeData {
    pub daily_volume: i128,
    pub monthly_volume: i128,
    pub last_tx_timestamp: u64,
}

/// Single namespace for every storage key used by the registry.
#[contracttype]
pub enum DataKey {
    Identity(Address),
    Volume(Address),
    CustomFields(Address),
    Admin,
    Verifiers,
    AuthorizedCallers,
    SupportedJurisdictions,
}

// ---------------------------------------------------------------------------
// Identity records
// ---------------------------------------------------------------------------

pub fn has_identity(env: &Env, user: &Address) -> bool {
    env.storage()
        .instance()
        .has(&DataKey::Identity(user.clone()))
}

pub fn write_identity(env: &Env, user: &Address, data: &IdentityData) {
    env.storage()
        .instance()
        .set(&DataKey::Identity(user.clone()), data);
}

pub fn read_identity(env: &Env, user: &Address) -> Option<IdentityData> {
    env.storage()
        .instance()
        .get(&DataKey::Identity(user.clone()))
}

// ---------------------------------------------------------------------------
// Volume counters
// ---------------------------------------------------------------------------

pub fn read_volume(env: &Env, user: &Address) -> VolumeData {
    env.storage()
        .instance()
        .get(&DataKey::Volume(user.clone()))
        .unwrap_or(VolumeData {
            daily_volume: 0,
            monthly_volume: 0,
            last_tx_timestamp: 0,
        })
}

pub fn write_volume(env: &Env, user: &Address, volume: &VolumeData) {
    env.storage()
        .instance()
        .set(&DataKey::Volume(user.clone()), volume);
}

// ---------------------------------------------------------------------------
// Composite identity record (identity + volume + custom fields)
// ---------------------------------------------------------------------------

/// Composite identity record (identity + volume + custom fields).
///
/// Returns `None` when the user has no registered identity, so callers can
/// surface a typed error instead of panicking across contract boundaries.
pub fn get_identity_record(env: &Env, user: &Address) -> Option<IdentityRecord> {
    let data = read_identity(env, user)?;
    let vol = read_volume(env, user);
    let custom_fields = read_custom_fields(env, user);
    Some(IdentityRecord {
        did: data.did,
        kyc_status: data.kyc_status,
        jurisdiction: data.jurisdiction,
        country_code: data.country_code,
        tier: data.tier,
        daily_volume: vol.daily_volume,
        monthly_volume: vol.monthly_volume,
        last_tx_timestamp: vol.last_tx_timestamp,
        custom_fields,
    })
}

// ---------------------------------------------------------------------------
// Custom fields
// ---------------------------------------------------------------------------

pub fn read_custom_fields(env: &Env, user: &Address) -> Vec<CustomField> {
    env.storage()
        .instance()
        .get(&DataKey::CustomFields(user.clone()))
        .unwrap_or(Vec::new(env))
}

/// Upsert (`add == true`) or remove (`add == false`) a custom field.
pub fn set_custom_field(env: &Env, user: &Address, field: &CustomField, add: bool) {
    let fields = read_custom_fields(env, user);
    let mut replaced = false;
    let mut new_fields: Vec<CustomField> = Vec::new(env);
    for f in fields.iter() {
        if f.key == field.key {
            if add {
                new_fields.push_back(field.clone());
            }
            replaced = true;
        } else {
            new_fields.push_back(f);
        }
    }
    if !replaced && add {
        new_fields.push_back(field.clone());
    }
    env.storage()
        .instance()
        .set(&DataKey::CustomFields(user.clone()), &new_fields);
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

pub fn write_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

/// Read the registry admin.
///
/// Panics only if the contract was never initialized; the host enforces that
/// `__constructor` runs exactly once at deployment, so this is unreachable in
/// practice.
pub fn read_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("admin not set")
}

// ---------------------------------------------------------------------------
// Verifiers
// ---------------------------------------------------------------------------

fn read_verifiers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Verifiers)
        .unwrap_or(Vec::new(env))
}

pub fn is_verifier(env: &Env, verifier: &Address) -> bool {
    read_verifiers(env).iter().any(|v| v == *verifier)
}

/// Adds a verifier. Callers must check `is_verifier` first to avoid duplicates.
pub fn add_verifier(env: &Env, verifier: &Address) {
    let mut verifiers = read_verifiers(env);
    verifiers.push_back(verifier.clone());
    env.storage()
        .instance()
        .set(&DataKey::Verifiers, &verifiers);
}

pub fn remove_verifier(env: &Env, verifier: &Address) {
    let verifiers = read_verifiers(env);
    let mut remaining: Vec<Address> = Vec::new(env);
    for v in verifiers.iter() {
        if v != *verifier {
            remaining.push_back(v);
        }
    }
    env.storage()
        .instance()
        .set(&DataKey::Verifiers, &remaining);
}

// ---------------------------------------------------------------------------
// Authorized callers (e.g. pool contracts allowed to update volume)
// ---------------------------------------------------------------------------

fn read_authorized_callers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::AuthorizedCallers)
        .unwrap_or(Vec::new(env))
}

pub fn is_authorized_caller(env: &Env, caller: &Address) -> bool {
    read_authorized_callers(env).iter().any(|c| c == *caller)
}

/// Adds an authorized caller. Callers must check `is_authorized_caller` first.
pub fn add_authorized_caller(env: &Env, caller: &Address) {
    let mut callers = read_authorized_callers(env);
    callers.push_back(caller.clone());
    env.storage()
        .instance()
        .set(&DataKey::AuthorizedCallers, &callers);
}

pub fn remove_authorized_caller(env: &Env, caller: &Address) {
    let callers = read_authorized_callers(env);
    let mut remaining: Vec<Address> = Vec::new(env);
    for c in callers.iter() {
        if c != *caller {
            remaining.push_back(c);
        }
    }
    env.storage()
        .instance()
        .set(&DataKey::AuthorizedCallers, &remaining);
}

// ---------------------------------------------------------------------------
// Supported jurisdictions
// ---------------------------------------------------------------------------

pub fn set_supported_jurisdictions(env: &Env, jurisdictions: Vec<Bytes>) {
    env.storage()
        .instance()
        .set(&DataKey::SupportedJurisdictions, &jurisdictions);
}

pub fn is_jurisdiction_supported(env: &Env, country_code: &Bytes) -> bool {
    let jurisdictions: Vec<Bytes> = env
        .storage()
        .instance()
        .get(&DataKey::SupportedJurisdictions)
        .unwrap_or(Vec::new(env));
    jurisdictions.iter().any(|j| j == *country_code)
}
