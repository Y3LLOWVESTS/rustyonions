//! RO:WHAT — Transport-bound and hardening config tests.
//! RO:WHY  — Pillar 12; Concerns: SEC/RES/PERF. Bounds must hold before heavy work or ledger IO.
//! RO:INTERACTS — WalletConfig constants and validation.
//! RO:INVARIANTS — body≤1MiB; decompression≤10x; timeout>0; inflight bounded.
//! RO:METRICS — future wallet_rejects_total{reason="LIMITS_EXCEEDED"}.
//! RO:CONFIG — validates config hardening defaults.
//! RO:SECURITY — rejects oversized/decompression-abuse config.
//! RO:TEST — cargo test -p svc-wallet --test i_11_transport_bounds.

use svc_wallet::config::{
    WalletConfig, DEFAULT_MAX_BODY_BYTES, DEFAULT_MAX_DECOMP_RATIO, DEFAULT_MAX_INFLIGHT,
    DEFAULT_REQ_TIMEOUT_MS,
};

#[test]
fn default_transport_bounds_match_hardening_contract() {
    let cfg = WalletConfig::default();

    assert_eq!(cfg.max_body_bytes, DEFAULT_MAX_BODY_BYTES);
    assert_eq!(cfg.max_body_bytes, 1_048_576);
    assert_eq!(cfg.max_decomp_ratio, DEFAULT_MAX_DECOMP_RATIO);
    assert_eq!(cfg.max_decomp_ratio, 10);
    assert_eq!(cfg.req_timeout_ms, DEFAULT_REQ_TIMEOUT_MS);
    assert_eq!(cfg.req_timeout_ms, 5_000);
    assert_eq!(cfg.max_inflight, DEFAULT_MAX_INFLIGHT);
    assert_eq!(cfg.max_inflight, 512);
    cfg.validate()
        .expect("default transport bounds should validate");
}

#[test]
fn config_rejects_body_cap_above_one_mib() {
    let cfg = WalletConfig {
        max_body_bytes: DEFAULT_MAX_BODY_BYTES + 1,
        ..WalletConfig::default()
    };

    assert!(cfg.validate().is_err());
}

#[test]
fn config_rejects_decompression_ratio_above_ten() {
    let cfg = WalletConfig {
        max_decomp_ratio: DEFAULT_MAX_DECOMP_RATIO + 1,
        ..WalletConfig::default()
    };

    assert!(cfg.validate().is_err());
}
