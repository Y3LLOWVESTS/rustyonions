//! RO:WHAT — I-2 capability-required contract checks for `ron-app-sdk`.
//! RO:WHY — Ensures storage-plane calls require an explicit capability argument at the SDK boundary.
//! RO:INTERACTS — Capability, storage_get, storage_put, SdkError.
//! RO:INVARIANTS — No ambient auth; expired capabilities are non-retriable.
//! RO:SECURITY — Capability failures must not be retried blindly.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::time::Duration;

use bytes::Bytes;
use ron_app_sdk::{
    planes::storage::{storage_get, storage_put},
    transport::TransportHandle,
    AddrB3, Capability, NoopSdkMetrics, RetryClass, SdkConfig, SdkError,
};

fn test_capability(scope: &str) -> Capability {
    Capability {
        subject: "passport:test:alice".to_owned(),
        scope: scope.to_owned(),
        issued_at: 1_700_000_000,
        expires_at: 1_700_003_600,
        caveats: Vec::new(),
    }
}

#[test]
fn capability_error_taxonomy_is_non_retriable() {
    let denied = SdkError::CapabilityDenied;
    let expired = SdkError::CapabilityExpired;

    assert_eq!(denied.retry_class(), RetryClass::NoRetry);
    assert_eq!(expired.retry_class(), RetryClass::NoRetry);
    assert!(!denied.is_retriable());
    assert!(!expired.is_retriable());
}

#[tokio::test]
async fn storage_get_still_requires_explicit_capability_parameter() {
    let transport = TransportHandle::new(SdkConfig::default());
    let metrics = NoopSdkMetrics;
    let cap = test_capability("storage:read");
    let addr = AddrB3::parse("b3:0000000000000000000000000000000000000000000000000000000000000000")
        .expect("valid test b3 address");

    let err = storage_get(&transport, &metrics, cap, &addr, Duration::ZERO)
        .await
        .expect_err("zero deadline should fail before any transport call");

    match err {
        SdkError::SchemaViolation { path, .. } => assert_eq!(path, "storage_get.deadline"),
        other => panic!("expected SchemaViolation, got {other:?}"),
    }
}

#[tokio::test]
async fn storage_put_still_requires_explicit_capability_parameter() {
    let transport = TransportHandle::new(SdkConfig::default());
    let metrics = NoopSdkMetrics;
    let cap = test_capability("storage:write");

    let err = storage_put(
        &transport,
        &metrics,
        cap,
        Bytes::from_static(b"capability-bound-write"),
        Duration::ZERO,
        None,
    )
    .await
    .expect_err("zero deadline should fail before any transport call");

    match err {
        SdkError::SchemaViolation { path, .. } => assert_eq!(path, "storage_put.deadline"),
        other => panic!("expected SchemaViolation, got {other:?}"),
    }
}
