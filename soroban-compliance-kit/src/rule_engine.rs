use soroban_sdk::{Bytes, Env, Vec};

use crate::types::{
    ComplianceAction, ComplianceError, ComplianceRule, IdentityRecord, Jurisdiction, KycStatus,
    RuleField, RuleOperator, RuleValue,
};

fn bytes_to_u128(b: &Bytes) -> u128 {
    let len = b.len() as usize;
    if len == 0 {
        return 0;
    }
    let mut buf = [0u8; 16];
    let start = 16_usize.saturating_sub(len);
    for i in 0..len {
        if start + i < 16 {
            buf[start + i] = b.get(i as u32).unwrap_or(0);
        }
    }
    u128::from_be_bytes(buf)
}

fn check_numeric_comparison(field_val: u128, rule_val: &RuleValue, op: &RuleOperator) -> bool {
    match op {
        RuleOperator::In | RuleOperator::NotIn => {
            let values = match rule_val {
                RuleValue::Multiple(vals) => vals,
                _ => return false,
            };
            let matched = values.iter().any(|v| field_val == bytes_to_u128(&v));
            matches!(op, RuleOperator::In) == matched
        }
        _ => {
            let rv = match rule_val {
                RuleValue::Single(b) => bytes_to_u128(b),
                _ => return false,
            };
            match op {
                RuleOperator::Eq => field_val == rv,
                RuleOperator::Neq => field_val != rv,
                RuleOperator::Gt => field_val > rv,
                RuleOperator::Lt => field_val < rv,
                RuleOperator::Gte => field_val >= rv,
                RuleOperator::Lte => field_val <= rv,
                _ => false,
            }
        }
    }
}

fn check_numeric_comparison_signed(
    field_val: i128,
    rule_val: &RuleValue,
    op: &RuleOperator,
) -> bool {
    match op {
        RuleOperator::In | RuleOperator::NotIn => {
            let values = match rule_val {
                RuleValue::Multiple(vals) => vals,
                _ => return false,
            };
            let matched = values
                .iter()
                .any(|v| field_val == bytes_to_u128(&v) as i128);
            matches!(op, RuleOperator::In) == matched
        }
        _ => {
            let rv = match rule_val {
                RuleValue::Single(b) => bytes_to_u128(b) as i128,
                _ => return false,
            };
            match op {
                RuleOperator::Eq => field_val == rv,
                RuleOperator::Neq => field_val != rv,
                RuleOperator::Gt => field_val > rv,
                RuleOperator::Lt => field_val < rv,
                RuleOperator::Gte => field_val >= rv,
                RuleOperator::Lte => field_val <= rv,
                _ => false,
            }
        }
    }
}

fn check_bytes_comparison(field_val: &Bytes, rule_val: &RuleValue, op: &RuleOperator) -> bool {
    match op {
        RuleOperator::In | RuleOperator::NotIn => {
            let values = match rule_val {
                RuleValue::Multiple(vals) => vals,
                _ => return false,
            };
            let matched = values.iter().any(|v| *field_val == v);
            matches!(op, RuleOperator::In) == matched
        }
        _ => {
            let rv = match rule_val {
                RuleValue::Single(b) => b,
                _ => return false,
            };
            match op {
                RuleOperator::Eq => field_val == rv,
                RuleOperator::Neq => field_val != rv,
                _ => false,
            }
        }
    }
}

fn get_custom_field(record: &IdentityRecord, key: &Bytes) -> Option<Bytes> {
    for field in record.custom_fields.iter() {
        if &field.key == key {
            return Some(field.value.clone());
        }
    }
    None
}

/// Evaluate a single compliance rule against an identity record.
///
/// Returns `Ok(true)` if the rule passes (or is skipped due to action filter),
/// `Ok(false)` if it fails, or `Err` if a required field is unavailable.
fn evaluate_single_rule(
    env: &Env,
    record: &IdentityRecord,
    rule: &ComplianceRule,
    action: &ComplianceAction,
    _amount: i128,
    total_supply: Option<i128>,
    balance: Option<i128>,
) -> Result<bool, ComplianceError> {
    if rule.action_filter != ComplianceAction::Any && rule.action_filter != *action {
        return Ok(true);
    }

    let result = match &rule.field {
        RuleField::KycStatus => {
            let val = match record.kyc_status {
                KycStatus::None => 0u128,
                KycStatus::Pending => 1,
                KycStatus::Verified => 2,
                KycStatus::Rejected => 3,
                KycStatus::Expired => 4,
            };
            check_numeric_comparison(val, &rule.value, &rule.operator)
        }
        RuleField::Tier => {
            check_numeric_comparison(record.tier as u128, &rule.value, &rule.operator)
        }
        RuleField::CountryCode => {
            check_bytes_comparison(&record.country_code, &rule.value, &rule.operator)
        }
        RuleField::DailyVolume => {
            check_numeric_comparison_signed(record.daily_volume, &rule.value, &rule.operator)
        }
        RuleField::MonthlyVolume => {
            check_numeric_comparison_signed(record.monthly_volume, &rule.value, &rule.operator)
        }
        RuleField::Jurisdiction => {
            let j_bytes = match &record.jurisdiction {
                Jurisdiction::Us => Bytes::from_slice(env, b"US"),
                Jurisdiction::Eu => Bytes::from_slice(env, b"EU"),
                Jurisdiction::Uk => Bytes::from_slice(env, b"UK"),
                Jurisdiction::Other(b) => b.clone(),
            };
            check_bytes_comparison(&j_bytes, &rule.value, &rule.operator)
        }
        RuleField::TotalSupply => match total_supply {
            Some(ts) => check_numeric_comparison_signed(ts, &rule.value, &rule.operator),
            None => return Err(ComplianceError::FieldNotAvailable),
        },
        RuleField::Balance => match balance {
            Some(b) => check_numeric_comparison_signed(b, &rule.value, &rule.operator),
            None => return Err(ComplianceError::FieldNotAvailable),
        },
        RuleField::Custom(key) => match get_custom_field(record, key) {
            Some(val) => check_bytes_comparison(&val, &rule.value, &rule.operator),
            None => return Err(ComplianceError::FieldNotAvailable),
        },
    };

    Ok(result)
}

/// Evaluate all configured compliance rules against an identity record.
///
/// Iterates through `rules` and returns `Ok(())` only when every applicable
/// rule passes. Returns `RuleEvaluationFailed` if any rule fails, or a
/// domain-specific error if a required field is missing.
pub fn evaluate_rules(
    env: &Env,
    record: &IdentityRecord,
    rules: &Vec<ComplianceRule>,
    action: &ComplianceAction,
    amount: i128,
    total_supply: Option<i128>,
    balance: Option<i128>,
) -> Result<(), ComplianceError> {
    for rule in rules.iter() {
        let passed =
            evaluate_single_rule(env, record, &rule, action, amount, total_supply, balance)?;
        if !passed {
            return Err(ComplianceError::RuleEvaluationFailed);
        }
    }
    Ok(())
}

/// Check whether an operation `amount` would exceed daily or monthly volume caps.
///
/// Limits of `0` are treated as unlimited.
pub fn check_volume_limits(
    record: &IdentityRecord,
    amount: i128,
    daily_limit: i128,
    monthly_limit: i128,
) -> Result<(), ComplianceError> {
    if daily_limit > 0 && record.daily_volume + amount > daily_limit {
        return Err(ComplianceError::DailyVolumeExceeded);
    }
    if monthly_limit > 0 && record.monthly_volume + amount > monthly_limit {
        return Err(ComplianceError::MonthlyVolumeExceeded);
    }
    Ok(())
}

/// Check whether an identity's country code is in a restricted list.
///
/// Returns `JurisdictionRestricted` if the country code matches any entry.
pub fn check_jurisdiction_restriction(
    record: &IdentityRecord,
    restricted: &Vec<Bytes>,
) -> Result<(), ComplianceError> {
    for restricted_country in restricted.iter() {
        if record.country_code == restricted_country {
            return Err(ComplianceError::JurisdictionRestricted);
        }
    }
    Ok(())
}
