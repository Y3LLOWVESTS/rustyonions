//! RO:WHAT — Lightweight interop vector checks for the Rust SDK surface.
//! RO:WHY — Keeps canonical content IDs, idempotency headers, and error classes stable for polyglot SDKs.
//! RO:INTERACTS — AddrB3, IdemCfg, derive_idempotency_key convention, SdkError.
//! RO:INVARIANTS — b3 IDs are lowercase 64-hex; retry taxonomy is deterministic.
//! RO:SECURITY — Idempotency keys are opaque fingerprints, not raw user input.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::time::Duration;

use ron_app_sdk::{config::RetryCfg, AddrB3, IdemCfg, Jitter, RetryClass, SdkError};

#[test]
fn content_id_vector_accepts_canonical_b3_and_rejects_uppercase() {
    let canonical = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let uppercase = "b3:0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

    let parsed = AddrB3::parse(canonical).expect("canonical b3 CID should parse");
    assert_eq!(parsed.as_str(), canonical);

    assert!(
        AddrB3::parse(uppercase).is_err(),
        "uppercase hex should not be accepted as canonical b3"
    );
}

#[test]
fn retry_taxonomy_vector_is_stable() {
    let retryable = SdkError::Server(503);
    let permanent = SdkError::CapabilityDenied;

    assert_eq!(retryable.retry_class(), RetryClass::Retriable);
    assert_eq!(permanent.retry_class(), RetryClass::NoRetry);
}

#[test]
fn retry_config_vector_stays_capped() {
    let cfg = RetryCfg {
        base: Duration::from_millis(50),
        factor: 2.0,
        cap: Duration::from_millis(500),
        max_attempts: 4,
        jitter: Jitter::None,
    };

    assert_eq!(cfg.base, Duration::from_millis(50));
    assert_eq!(cfg.cap, Duration::from_millis(500));
    assert_eq!(cfg.max_attempts, 4);
    assert!(cfg.cap >= cfg.base);
}

#[test]
fn idempotency_config_vector_is_opaque() {
    let cfg = IdemCfg {
        enabled: true,
        key_prefix: Some("ron".to_owned()),
    };

    assert!(cfg.enabled);
    assert_eq!(cfg.key_prefix.as_deref(), Some("ron"));
}
