use soroban_compliance_kit::rule_engine;
use soroban_compliance_kit::types::{
    ComplianceAction, ComplianceError, ComplianceRule, CustomField, IdentityRecord, Jurisdiction,
    KycStatus, RuleField, RuleOperator, RuleValue,
};
use soroban_sdk::{Bytes, Env, Vec};

fn identity_record(env: &Env, tier: u32, kyc: KycStatus, country: &[u8]) -> IdentityRecord {
    let mut fields: Vec<CustomField> = Vec::new(env);
    let key = Bytes::from_slice(env, b"risk_score");
    let val = Bytes::from_slice(env, b"low");
    fields.push_back(CustomField { key, value: val });

    IdentityRecord {
        did: Bytes::from_slice(env, b"did:example:prop"),
        kyc_status: kyc,
        jurisdiction: Jurisdiction::Us,
        country_code: Bytes::from_slice(env, country),
        tier,
        daily_volume: 0,
        monthly_volume: 0,
        last_tx_timestamp: 0,
        custom_fields: fields,
    }
}

fn single_rule_value(env: &Env, val: &[u8]) -> RuleValue {
    RuleValue::Single(Bytes::from_slice(env, val))
}

#[test]
fn rule_all_tiers_above_minimum() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    for tier in 1..=10u32 {
        let record = identity_record(&env, tier, KycStatus::Verified, b"US");
        let rules: Vec<ComplianceRule> = Vec::new(&env);

        let result = rule_engine::evaluate_rules(&env, &record, &rules, &action, 1000, None, None);
        assert!(result.is_ok(), "tier {} should pass empty rules", tier);
    }
}

#[test]
fn rule_tier_threshold_exact_match() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let rule = ComplianceRule {
        field: RuleField::Tier,
        operator: RuleOperator::Gte,
        value: single_rule_value(&env, &3u128.to_be_bytes()),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    for tier in 1..=5u32 {
        let record = identity_record(&env, tier, KycStatus::Verified, b"US");
        let result = rule_engine::evaluate_rules(&env, &record, &rules, &action, 1000, None, None);
        if tier >= 3 {
            assert!(result.is_ok(), "tier {} should pass Gte 3", tier);
        } else {
            assert_eq!(result, Err(ComplianceError::RuleEvaluationFailed));
        }
    }
}

#[test]
fn rule_jurisdiction_matching() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let rule = ComplianceRule {
        field: RuleField::Jurisdiction,
        operator: RuleOperator::Eq,
        value: single_rule_value(&env, b"US"),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    let us_record = IdentityRecord {
        did: Bytes::from_slice(&env, b"did:us"),
        kyc_status: KycStatus::Verified,
        jurisdiction: Jurisdiction::Us,
        country_code: Bytes::from_slice(&env, b"US"),
        tier: 1,
        daily_volume: 0,
        monthly_volume: 0,
        last_tx_timestamp: 0,
        custom_fields: Vec::new(&env),
    };
    assert!(
        rule_engine::evaluate_rules(&env, &us_record, &rules, &action, 1000, None, None).is_ok()
    );

    let eu_record = IdentityRecord {
        did: Bytes::from_slice(&env, b"did:eu"),
        kyc_status: KycStatus::Verified,
        jurisdiction: Jurisdiction::Eu,
        country_code: Bytes::from_slice(&env, b"EU"),
        tier: 1,
        daily_volume: 0,
        monthly_volume: 0,
        last_tx_timestamp: 0,
        custom_fields: Vec::new(&env),
    };
    assert_eq!(
        rule_engine::evaluate_rules(&env, &eu_record, &rules, &action, 1000, None, None),
        Err(ComplianceError::RuleEvaluationFailed)
    );
}

#[test]
fn rule_jurisdiction_other_matches() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let china = Bytes::from_slice(&env, b"CN");
    let rule = ComplianceRule {
        field: RuleField::Jurisdiction,
        operator: RuleOperator::Eq,
        value: single_rule_value(&env, b"CN"),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    let cn_record = IdentityRecord {
        did: Bytes::from_slice(&env, b"did:cn"),
        kyc_status: KycStatus::Verified,
        jurisdiction: Jurisdiction::Other(china.clone()),
        country_code: Bytes::from_slice(&env, b"CN"),
        tier: 1,
        daily_volume: 0,
        monthly_volume: 0,
        last_tx_timestamp: 0,
        custom_fields: Vec::new(&env),
    };
    assert!(
        rule_engine::evaluate_rules(&env, &cn_record, &rules, &action, 1000, None, None).is_ok(),
        "Jurisdiction::Other should match rule value"
    );
}

#[test]
fn rule_custom_field_matching() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let key = Bytes::from_slice(&env, b"risk_score");
    let val = Bytes::from_slice(&env, b"low");
    let rule = ComplianceRule {
        field: RuleField::Custom(key),
        operator: RuleOperator::Eq,
        value: RuleValue::Single(val),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    let mut fields: Vec<CustomField> = Vec::new(&env);
    let fk = Bytes::from_slice(&env, b"risk_score");
    let fv = Bytes::from_slice(&env, b"low");
    fields.push_back(CustomField { key: fk, value: fv });

    let record = IdentityRecord {
        did: Bytes::from_slice(&env, b"did:custom"),
        kyc_status: KycStatus::Verified,
        jurisdiction: Jurisdiction::Us,
        country_code: Bytes::from_slice(&env, b"US"),
        tier: 1,
        daily_volume: 0,
        monthly_volume: 0,
        last_tx_timestamp: 0,
        custom_fields: fields,
    };
    assert!(rule_engine::evaluate_rules(&env, &record, &rules, &action, 1000, None, None).is_ok());
}

#[test]
fn rule_custom_field_missing_returns_field_not_available() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let key = Bytes::from_slice(&env, b"nonexistent");
    let rule = ComplianceRule {
        field: RuleField::Custom(key),
        operator: RuleOperator::Eq,
        value: RuleValue::Single(Bytes::from_slice(&env, b"any")),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);
    let record = identity_record(&env, 1, KycStatus::Verified, b"US");

    assert_eq!(
        rule_engine::evaluate_rules(&env, &record, &rules, &action, 1000, None, None),
        Err(ComplianceError::FieldNotAvailable)
    );
}

#[test]
fn rule_balance_and_total_supply() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let balance_rule = ComplianceRule {
        field: RuleField::Balance,
        operator: RuleOperator::Gte,
        value: single_rule_value(&env, &1000i128.to_be_bytes()),
        action_filter: ComplianceAction::Any,
    };
    let supply_rule = ComplianceRule {
        field: RuleField::TotalSupply,
        operator: RuleOperator::Gte,
        value: single_rule_value(&env, &10000i128.to_be_bytes()),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [balance_rule, supply_rule]);

    let record = identity_record(&env, 1, KycStatus::Verified, b"US");

    assert_eq!(
        rule_engine::evaluate_rules(&env, &record, &rules, &action, 1000, None, None),
        Err(ComplianceError::FieldNotAvailable),
        "without total_supply param it should error"
    );
    assert_eq!(
        rule_engine::evaluate_rules(&env, &record, &rules, &action, 1000, Some(20000), None),
        Err(ComplianceError::FieldNotAvailable),
        "without balance param it should error"
    );
    assert!(
        rule_engine::evaluate_rules(
            &env,
            &record,
            &rules,
            &action,
            1000,
            Some(20000),
            Some(5000)
        )
        .is_ok(),
        "with both params it should pass"
    );
}

#[test]
fn rule_action_filtering() {
    let env = Env::default();

    let rule = ComplianceRule {
        field: RuleField::Tier,
        operator: RuleOperator::Gte,
        value: single_rule_value(&env, &3u128.to_be_bytes()),
        action_filter: ComplianceAction::Withdraw,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);
    let record = identity_record(&env, 1, KycStatus::Verified, b"US");

    assert!(
        rule_engine::evaluate_rules(
            &env,
            &record,
            &rules,
            &ComplianceAction::Transfer,
            1000,
            None,
            None
        )
        .is_ok(),
        "rule scoped to Withdraw should not apply to Transfer"
    );
    assert_eq!(
        rule_engine::evaluate_rules(
            &env,
            &record,
            &rules,
            &ComplianceAction::Withdraw,
            1000,
            None,
            None
        ),
        Err(ComplianceError::RuleEvaluationFailed),
        "rule scoped to Withdraw should apply to Withdraw"
    );
}

#[test]
fn rule_kyc_status_comparison() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let rule = ComplianceRule {
        field: RuleField::KycStatus,
        operator: RuleOperator::Eq,
        value: single_rule_value(&env, &2u128.to_be_bytes()),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    let statuses = [
        (KycStatus::None, false),
        (KycStatus::Pending, false),
        (KycStatus::Verified, true),
        (KycStatus::Rejected, false),
        (KycStatus::Expired, false),
    ];

    for item in &statuses {
        let record = identity_record(&env, 1, item.0, b"US");
        let result = rule_engine::evaluate_rules(&env, &record, &rules, &action, 1000, None, None);
        if item.1 {
            assert!(
                result.is_ok(),
                "{:?} should pass KycStatus == Verified",
                item.0
            );
        } else {
            assert_eq!(
                result,
                Err(ComplianceError::RuleEvaluationFailed),
                "{:?} should fail KycStatus == Verified",
                item.0
            );
        }
    }
}

#[test]
fn volume_limit_edge_cases() {
    let env = Env::default();
    let mut record = identity_record(&env, 1, KycStatus::Verified, b"US");

    assert!(rule_engine::check_volume_limits(&record, 100, 0, 0).is_ok());
    assert!(rule_engine::check_volume_limits(&record, 100, 1000, 0).is_ok());

    record.daily_volume = 900;
    assert!(rule_engine::check_volume_limits(&record, 100, 1000, 0).is_ok());
    assert_eq!(
        rule_engine::check_volume_limits(&record, 200, 1000, 0),
        Err(ComplianceError::DailyVolumeExceeded)
    );

    record.daily_volume = 0;
    record.monthly_volume = 9000;
    assert!(rule_engine::check_volume_limits(&record, 500, 0, 10000).is_ok());
    assert_eq!(
        rule_engine::check_volume_limits(&record, 2000, 0, 10000),
        Err(ComplianceError::MonthlyVolumeExceeded)
    );
}

#[test]
fn jurisdiction_restriction_multi_country() {
    let env = Env::default();
    let us = Bytes::from_slice(&env, b"US");
    let cn = Bytes::from_slice(&env, b"CN");
    let ir = Bytes::from_slice(&env, b"IR");
    let restricted = Vec::from_array(&env, [us.clone(), cn.clone(), ir.clone()]);

    let record_us = identity_record(&env, 1, KycStatus::Verified, b"US");
    let record_de = identity_record(&env, 1, KycStatus::Verified, b"DE");
    let record_cn = identity_record(&env, 1, KycStatus::Verified, b"CN");

    assert_eq!(
        rule_engine::check_jurisdiction_restriction(&record_us, &restricted),
        Err(ComplianceError::JurisdictionRestricted)
    );
    assert!(rule_engine::check_jurisdiction_restriction(&record_de, &restricted).is_ok());
    assert_eq!(
        rule_engine::check_jurisdiction_restriction(&record_cn, &restricted),
        Err(ComplianceError::JurisdictionRestricted)
    );
}

#[test]
fn country_code_inequality_rules() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let rule = ComplianceRule {
        field: RuleField::CountryCode,
        operator: RuleOperator::Neq,
        value: single_rule_value(&env, b"US"),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    let us_record = identity_record(&env, 1, KycStatus::Verified, b"US");
    assert_eq!(
        rule_engine::evaluate_rules(&env, &us_record, &rules, &action, 1000, None, None),
        Err(ComplianceError::RuleEvaluationFailed),
        "US should fail Neq US"
    );

    let de_record = identity_record(&env, 1, KycStatus::Verified, b"DE");
    assert!(
        rule_engine::evaluate_rules(&env, &de_record, &rules, &action, 1000, None, None).is_ok(),
        "DE should pass Neq US"
    );
}

#[test]
fn rule_in_operator_multi_value() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let us = Bytes::from_slice(&env, b"US");
    let eu = Bytes::from_slice(&env, b"EU");
    let uk = Bytes::from_slice(&env, b"UK");
    let rule = ComplianceRule {
        field: RuleField::CountryCode,
        operator: RuleOperator::In,
        value: RuleValue::Multiple(Vec::from_array(&env, [us, eu, uk])),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    let us_record = identity_record(&env, 1, KycStatus::Verified, b"US");
    let de_record = identity_record(&env, 1, KycStatus::Verified, b"DE");

    assert!(
        rule_engine::evaluate_rules(&env, &us_record, &rules, &action, 1000, None, None).is_ok()
    );
    assert_eq!(
        rule_engine::evaluate_rules(&env, &de_record, &rules, &action, 1000, None, None),
        Err(ComplianceError::RuleEvaluationFailed)
    );
}

#[test]
fn rule_notin_operator_multi_value() {
    let env = Env::default();
    let action = ComplianceAction::Transfer;

    let ir = Bytes::from_slice(&env, b"IR");
    let kp = Bytes::from_slice(&env, b"KP");
    let rule = ComplianceRule {
        field: RuleField::CountryCode,
        operator: RuleOperator::NotIn,
        value: RuleValue::Multiple(Vec::from_array(&env, [ir, kp])),
        action_filter: ComplianceAction::Any,
    };
    let rules: Vec<ComplianceRule> = Vec::from_array(&env, [rule]);

    let us_record = identity_record(&env, 1, KycStatus::Verified, b"US");
    let ir_record = identity_record(&env, 1, KycStatus::Verified, b"IR");

    assert!(
        rule_engine::evaluate_rules(&env, &us_record, &rules, &action, 1000, None, None).is_ok()
    );
    assert_eq!(
        rule_engine::evaluate_rules(&env, &ir_record, &rules, &action, 1000, None, None),
        Err(ComplianceError::RuleEvaluationFailed)
    );
}

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_bytes_to_u128_roundtrip(val: u128) {
        let env = Env::default();
        let bytes = Bytes::from_slice(&env, &val.to_be_bytes());
        let decoded = soroban_compliance_kit::bytes_to_u128(&bytes);
        assert_eq!(decoded, val);
    }

    #[test]
    fn prop_bytes_to_u128_short_slice(high in 0u128..u128::MAX) {
        let env = Env::default();
        let full = high.to_be_bytes();
        let short = &full[8..];
        let bytes = Bytes::from_slice(&env, short);
        let decoded = soroban_compliance_kit::bytes_to_u128(&bytes);
        assert_eq!(decoded, high & 0x00000000_00000000_FFFFFFFF_FFFFFFFF);
    }

    #[test]
    fn prop_volume_limits_not_exceeded(
        daily_vol in 0i128..i128::MAX,
        monthly_vol in 0i128..i128::MAX,
        amount in 0i128..i128::MAX,
    ) {
        let env = Env::default();
        let record = IdentityRecord {
            did: Bytes::from_slice(&env, b"did:prop"),
            kyc_status: KycStatus::Verified,
            jurisdiction: Jurisdiction::Us,
            country_code: Bytes::from_slice(&env, b"US"),
            tier: 1,
            daily_volume: daily_vol,
            monthly_volume: monthly_vol,
            last_tx_timestamp: 0,
            custom_fields: Vec::new(&env),
        };
        let daily_limit = daily_vol.saturating_add(amount);
        let monthly_limit = monthly_vol.saturating_add(amount);

        let result = rule_engine::check_volume_limits(&record, amount, daily_limit, monthly_limit);
        assert!(result.is_ok(), "volume within limits should pass");
    }

    #[test]
    fn prop_volume_limits_zero_limit(
        daily_vol in 0i128..i128::MAX,
        monthly_vol in 0i128..i128::MAX,
        amount in 0i128..i128::MAX,
    ) {
        let env = Env::default();
        let record = IdentityRecord {
            did: Bytes::from_slice(&env, b"did:prop"),
            kyc_status: KycStatus::Verified,
            jurisdiction: Jurisdiction::Us,
            country_code: Bytes::from_slice(&env, b"US"),
            tier: 1,
            daily_volume: daily_vol,
            monthly_volume: monthly_vol,
            last_tx_timestamp: 0,
            custom_fields: Vec::new(&env),
        };

        let result = rule_engine::check_volume_limits(&record, amount, 0, 0);
        assert!(result.is_ok(), "limit of 0 should always pass");
    }

    #[test]
    fn prop_jurisdiction_restriction_not_restricted(
        country in "[A-Z]{2}",
    ) {
        let env = Env::default();
        let restricted: Vec<Bytes> = Vec::new(&env);
        let record = IdentityRecord {
            did: Bytes::from_slice(&env, b"did:prop"),
            kyc_status: KycStatus::Verified,
            jurisdiction: Jurisdiction::Us,
            country_code: Bytes::from_slice(&env, country.as_bytes()),
            tier: 1,
            daily_volume: 0,
            monthly_volume: 0,
            last_tx_timestamp: 0,
            custom_fields: Vec::new(&env),
        };

        let result = rule_engine::check_jurisdiction_restriction(&record, &restricted);
        assert!(result.is_ok(), "empty restricted list should always pass");
    }
}
