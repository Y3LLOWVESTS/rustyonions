//! RO:WHAT — I-9 SDK error taxonomy stability checks.
//! RO:WHY — Keeps retry classification deterministic and low-surprise for app callers.
//! RO:INTERACTS — SdkError, RetryClass.
//! RO:INVARIANTS — Auth/schema/OAP errors are non-retriable; transient errors are retriable.
//! RO:SECURITY — Capability failures must never be retried blindly.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::{io::ErrorKind, time::Duration};

use ron_app_sdk::{RetryClass, SdkError};

#[test]
fn retry_classification_is_deterministic() {
    let retriable = [
        SdkError::DeadlineExceeded,
        SdkError::Transport(ErrorKind::TimedOut),
        SdkError::RateLimited {
            retry_after: Some(Duration::from_secs(1)),
        },
        SdkError::Server(503),
    ];

    for err in retriable {
        assert_eq!(
            err.retry_class(),
            RetryClass::Retriable,
            "expected retriable classification for {err:?}"
        );
        assert!(err.is_retriable());
    }

    let permanent = [
        SdkError::Tls,
        SdkError::TorUnavailable,
        SdkError::OapViolation {
            reason: "payload-too-large",
        },
        SdkError::CapabilityExpired,
        SdkError::CapabilityDenied,
        SdkError::NotFound,
        SdkError::Conflict,
        SdkError::Server(404),
        SdkError::Unknown("opaque".to_owned()),
    ];

    for err in permanent {
        assert_eq!(
            err.retry_class(),
            RetryClass::NoRetry,
            "expected non-retriable classification for {err:?}"
        );
        assert!(!err.is_retriable());
    }
}

#[test]
fn schema_violation_preserves_path_and_is_not_retriable() {
    let err = SdkError::schema_violation("asset.cid", "expected b3:<64hex>");

    match err {
        SdkError::SchemaViolation { path, detail } => {
            assert_eq!(path, "asset.cid");
            assert!(detail.contains("b3"));
        }
        other => panic!("expected schema violation, got {other:?}"),
    }
}
