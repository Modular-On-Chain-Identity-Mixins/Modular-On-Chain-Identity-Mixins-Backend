/// Core compliance gate: verify auth then enforce all compliance rules.
///
/// Should be called inside business-logic functions after pause checks.
/// Used internally by the action-specific macros below.
#[macro_export]
macro_rules! require_compliance {
    ($contract:ty, $env:expr, $sender:expr, $recipient:expr, $amount:expr, $action:expr) => {{
        <$contract as $crate::traits::ComplianceManager>::verify_auth(&$env, &$sender);
        <$contract as $crate::traits::ComplianceManager>::enforce_compliance(
            &$env,
            &$sender,
            &$recipient,
            $amount,
            $action,
        )?;
    }};
}

/// Check pause state and enforce compliance for a transfer action.
#[macro_export]
macro_rules! compliance_transfer_check {
    ($contract:ty, $env:expr, $from:expr, $to:expr, $amount:expr) => {{
        if <$contract as $crate::traits::ComplianceManager>::is_paused(&$env) {
            return Err($crate::types::ComplianceError::ContractPaused);
        }
        $crate::require_compliance!(
            $contract,
            $env,
            $from,
            $to,
            $amount,
            $crate::types::ComplianceAction::Transfer
        )
    }};
}

/// Check pause state and enforce compliance for a deposit action.
#[macro_export]
macro_rules! compliance_deposit_check {
    ($contract:ty, $env:expr, $from:expr, $amount:expr) => {{
        if <$contract as $crate::traits::ComplianceManager>::is_paused(&$env) {
            return Err($crate::types::ComplianceError::ContractPaused);
        }
        let _recipient = $from.clone();
        $crate::require_compliance!(
            $contract,
            $env,
            $from,
            _recipient,
            $amount,
            $crate::types::ComplianceAction::Deposit
        )
    }};
}

/// Check pause state and enforce compliance for a withdraw action.
#[macro_export]
macro_rules! compliance_withdraw_check {
    ($contract:ty, $env:expr, $from:expr, $to:expr, $amount:expr) => {{
        if <$contract as $crate::traits::ComplianceManager>::is_paused(&$env) {
            return Err($crate::types::ComplianceError::ContractPaused);
        }
        $crate::require_compliance!(
            $contract,
            $env,
            $from,
            $to,
            $amount,
            $crate::types::ComplianceAction::Withdraw
        )
    }};
}
