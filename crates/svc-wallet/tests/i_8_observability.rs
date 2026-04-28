//! RO:WHAT — Observability contract tests for stable error labels and metric-friendly DTOs.
//! RO:WHY  — Pillar 12; Concerns: PERF/RES/DX/GOV. Dashboards and SDKs need stable machine labels.
//! RO:INTERACTS — errors and dto::errors.
//! RO:INVARIANTS — stable code strings; retryability is deterministic; corr_id can be echoed.
//! RO:METRICS — validates labels used by wallet_rejects_total.
//! RO:CONFIG — none.
//! RO:SECURITY — error envelopes contain no Authorization/token material.
//! RO:TEST — cargo test -p svc-wallet --test i_8_observability.

use svc_wallet::{
    dto::errors::ErrorResponse,
    errors::{WalletError, WalletErrorCode},
};

#[test]
fn error_response_preserves_stable_machine_code_and_corr_id() {
    let err = WalletError::new(WalletErrorCode::InsufficientFunds, "insufficient funds");
    let body = ErrorResponse::from_error(&err, Some("corr-123".to_string()));

    assert_eq!(body.code, "INSUFFICIENT_FUNDS");
    assert_eq!(body.http, 409);
    assert!(!body.retryable);
    assert_eq!(body.corr_id.as_deref(), Some("corr-123"));
}

#[test]
fn retryable_error_classes_are_stable() {
    assert!(WalletErrorCode::Busy.retryable());
    assert!(WalletErrorCode::RetryLater.retryable());
    assert!(WalletErrorCode::UpstreamUnavailable.retryable());

    assert!(!WalletErrorCode::BadRequest.retryable());
    assert!(!WalletErrorCode::Forbidden.retryable());
    assert!(!WalletErrorCode::InsufficientFunds.retryable());
}

#[test]
fn code_strings_are_dashboard_safe() {
    let labels = [
        WalletErrorCode::BadRequest.as_str(),
        WalletErrorCode::Unauthorized.as_str(),
        WalletErrorCode::Forbidden.as_str(),
        WalletErrorCode::LimitsExceeded.as_str(),
        WalletErrorCode::InsufficientFunds.as_str(),
        WalletErrorCode::NonceConflict.as_str(),
        WalletErrorCode::IdempotencyConflict.as_str(),
        WalletErrorCode::Busy.as_str(),
        WalletErrorCode::RetryLater.as_str(),
        WalletErrorCode::UpstreamUnavailable.as_str(),
        WalletErrorCode::NotFound.as_str(),
    ];

    for label in labels {
        assert!(label.chars().all(|c| c == '_' || c.is_ascii_uppercase()));
    }
}
