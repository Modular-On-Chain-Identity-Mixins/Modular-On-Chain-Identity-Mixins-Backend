use soroban_sdk::{contracttype, symbol_short, Address, Bytes, Env, Vec};

use soroban_compliance_kit::types::{IdentityRecord, Jurisdiction, KycStatus};

#[contracttype]
pub struct IdentityData {
    pub did: Bytes,
    pub kyc_status: KycStatus,
    pub jurisdiction: Jurisdiction,
    pub country_code: Bytes,
    pub tier: u32,
}

#[contracttype]
pub struct VolumeData {
    pub daily_volume: i128,
    pub monthly_volume: i128,
    pub last_tx_timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Identity(Address),
    Volume(Address),
}

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

pub fn get_identity_record(env: &Env, user: &Address) -> IdentityRecord {
    let data = read_identity(env, user).expect("identity not registered");
    let vol = read_volume(env, user);
    IdentityRecord {
        did: data.did,
        kyc_status: data.kyc_status,
        jurisdiction: data.jurisdiction,
        country_code: data.country_code,
        tier: data.tier,
        daily_volume: vol.daily_volume,
        monthly_volume: vol.monthly_volume,
        last_tx_timestamp: vol.last_tx_timestamp,
        custom_fields: Vec::new(env),
    }
}

pub fn write_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&symbol_short!("admin"), admin);
}

pub fn read_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&symbol_short!("admin"))
        .expect("admin not set")
}

pub fn add_verifier(env: &Env, verifier: &Address) {
    let mut verifiers: Vec<Address> = env
        .storage()
        .instance()
        .get(&symbol_short!("verifiers"))
        .unwrap_or(Vec::new(env));
    verifiers.push_back(verifier.clone());
    env.storage()
        .instance()
        .set(&symbol_short!("verifiers"), &verifiers);
}

pub fn is_verifier(env: &Env, verifier: &Address) -> bool {
    let verifiers: Vec<Address> = env
        .storage()
        .instance()
        .get(&symbol_short!("verifiers"))
        .unwrap_or(Vec::new(env));
    verifiers.iter().any(|v| v == *verifier)
}

pub fn set_supported_jurisdictions(env: &Env, jurisdictions: Vec<Bytes>) {
    env.storage()
        .instance()
        .set(&symbol_short!("jurisdict"), &jurisdictions);
}

pub fn is_jurisdiction_supported(env: &Env, country_code: &Bytes) -> bool {
    let jurisdictions: Vec<Bytes> = env
        .storage()
        .instance()
        .get(&symbol_short!("jurisdict"))
        .unwrap_or(Vec::new(env));
    jurisdictions.iter().any(|j| j == *country_code)
}
