//! RO:WHAT — Readiness/shedding-adjacent contract tests for writes under pressure.
//! RO:WHY  — Pillar 12; Concerns: RES/PERF/ECON. Degraded services must fail closed for writes.
//! RO:INTERACTS — errors and WalletConfig hardening knobs.
//! RO:INVARIANTS — BUSY maps to 429; RETRY_LATER maps to 503; both are retryable.
//! RO:METRICS — future busy_rejections_total and wallet_rejects_total.
//! RO:CONFIG — max_inflight is bounded and non-zero.
//! RO:SECURITY — no token material.
//! RO:TEST — cargo test -p svc-wallet --test readiness_and_shedding.

use svc_wallet::{
    config::WalletConfig,
    errors::{WalletError, WalletErrorCode},
};

#[test]
fn busy_and_retry_later_have_stable_shedding_semantics() {
    let busy = WalletError::new(WalletErrorCode::Busy, "busy");
    let retry_later = WalletError::new(WalletErrorCode::RetryLater, "retry later");

    assert_eq!(busy.http_status(), 429);
    assert!(busy.retryable());

    assert_eq!(retry_later.http_status(), 503);
    assert!(retry_later.retryable());
}

#[test]
fn max_inflight_is_present_and_bounded_by_default() {
    let cfg = WalletConfig::default();

    assert_eq!(cfg.max_inflight, 512);
    assert!(cfg.max_inflight > 0);
    cfg.validate()
        .expect("default max_inflight should validate");
}

#[test]
fn zero_inflight_config_is_invalid() {
    let cfg = WalletConfig {
        max_inflight: 0,
        ..WalletConfig::default()
    };

    assert!(cfg.validate().is_err());
}
