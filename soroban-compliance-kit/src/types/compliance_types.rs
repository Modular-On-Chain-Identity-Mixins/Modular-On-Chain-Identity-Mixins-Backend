use soroban_sdk::{contracterror, contracttype, Address, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Jurisdiction {
    Us,
    Eu,
    Uk,
    Other(soroban_sdk::Bytes),
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KycStatus {
    None,
    Pending,
    Verified,
    Rejected,
    Expired,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComplianceAction {
    Any,
    Transfer,
    Deposit,
    Withdraw,
    Mint,
    Burn,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleOperator {
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
    In,
    NotIn,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleField {
    Jurisdiction,
    KycStatus,
    Tier,
    CountryCode,
    DailyVolume,
    MonthlyVolume,
    TotalSupply,
    Balance,
    Custom(soroban_sdk::Bytes),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceRule {
    pub field: RuleField,
    pub operator: RuleOperator,
    pub value: soroban_sdk::Bytes,
    pub action_filter: ComplianceAction,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityRecord {
    pub did: soroban_sdk::Bytes,
    pub kyc_status: KycStatus,
    pub jurisdiction: Jurisdiction,
    pub country_code: soroban_sdk::Bytes,
    pub tier: u32,
    pub daily_volume: i128,
    pub monthly_volume: i128,
    pub last_tx_timestamp: u64,
    pub custom_fields: Vec<CustomField>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomField {
    pub key: soroban_sdk::Bytes,
    pub value: soroban_sdk::Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceConfig {
    pub owner: Address,
    pub paused: bool,
    pub rules: Vec<ComplianceRule>,
    pub identity_registry: Address,
    pub required_tier: u32,
    pub daily_volume_limit: i128,
    pub monthly_volume_limit: i128,
    pub restricted_jurisdictions: Vec<soroban_sdk::Bytes>,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComplianceError {
    RuleEvaluationFailed = 1,
    DailyVolumeExceeded = 2,
    MonthlyVolumeExceeded = 3,
    JurisdictionRestricted = 4,
    FieldNotAvailable = 5,
    ContractPaused = 100,
    KycNotVerified = 101,
    InsufficientTier = 102,
}
