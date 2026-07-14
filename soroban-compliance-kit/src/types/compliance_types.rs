use soroban_sdk::{contracterror, contracttype, Address, Bytes, Vec};

/// Geographic jurisdiction for identity classification.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Jurisdiction {
    Us,
    Eu,
    Uk,
    Other(Bytes),
}

/// KYC/onboarding status of an identity.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KycStatus {
    None,
    Pending,
    Verified,
    Rejected,
    Expired,
}

/// The type of compliance-checked action being performed.
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

/// Comparison operator for a compliance rule.
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

/// The identity field a rule evaluates against.
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
    Custom(Bytes),
}

/// A single value or set of values for a compliance rule.
///
/// `Single` is used with comparison operators (`Eq`, `Neq`, `Gt`, etc.).
/// `Multiple` is used with set operators (`In`, `NotIn`) to test membership.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleValue {
    Single(Bytes),
    Multiple(Vec<Bytes>),
}

/// A programmable compliance rule.
///
/// The rule is evaluated against a user's [`IdentityRecord`]. It only applies
/// when `action_filter` matches the current action (or is `Any`).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceRule {
    pub field: RuleField,
    pub operator: RuleOperator,
    pub value: RuleValue,
    pub action_filter: ComplianceAction,
}

/// On-chain identity record returned by the identity registry.
///
/// Includes KYC status, jurisdiction, tier, tracked volumes, and custom fields.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityRecord {
    pub did: Bytes,
    pub kyc_status: KycStatus,
    pub jurisdiction: Jurisdiction,
    pub country_code: Bytes,
    pub tier: u32,
    pub daily_volume: i128,
    pub monthly_volume: i128,
    pub last_tx_timestamp: u64,
    pub custom_fields: Vec<CustomField>,
}

/// A key-value pair for extensible identity metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomField {
    pub key: Bytes,
    pub value: Bytes,
}

/// Configuration for a contract's compliance module.
///
/// Stored per-contract and governs all compliance checks.
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
    pub restricted_jurisdictions: Vec<Bytes>,
}

/// Errors returned by compliance checks and rule evaluation.
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
    VolumeUpdateFailed = 103,
}
