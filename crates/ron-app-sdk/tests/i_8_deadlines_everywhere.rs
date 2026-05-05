//! RO:WHAT — I-8 deadline discipline checks for SDK calls and config.
//! RO:WHY — Ensures external I/O is bounded and zero deadlines are rejected.
//! RO:INTERACTS — SdkConfig, Timeouts, storage plane validation.
//! RO:INVARIANTS — Valid configs keep overall timeout >= read/write timeouts; per-call deadlines must be nonzero.
//! RO:SECURITY — Prevents hanging calls and unbounded waits.
//! RO:TEST — cargo test -p ron-app-sdk --test i_8_deadlines_everywhere.

use std::time::Duration;

use ron_app_sdk::{
    transport::TransportHandle, AddrB3, Capability, NoopSdkMetrics, SdkConfig, SdkError, Timeouts,
    Transport,
};

fn dummy_capability() -> Capability {
    Capability {
        subject: "deadline-test".to_owned(),
        scope: "storage:read".to_owned(),
        issued_at: 0,
        expires_at: u64::MAX,
        caveats: Vec::new(),
    }
}

fn valid_sdk_config() -> SdkConfig {
    SdkConfig {
        transport: Transport::Tls,
        overall_timeout: Duration::from_secs(60),
        timeouts: Timeouts {
            connect: Duration::from_secs(3),
            read: Duration::from_secs(10),
            write: Duration::from_secs(10),
        },
        ..Default::default()
    }
}

#[test]
fn explicit_valid_config_has_bounded_deadlines() {
    let cfg = valid_sdk_config();

    assert!(cfg.overall_timeout >= Duration::from_secs(1));
    assert!(cfg.timeouts.connect > Duration::ZERO);
    assert!(cfg.timeouts.read > Duration::ZERO);
    assert!(cfg.timeouts.write > Duration::ZERO);
    assert!(cfg.overall_timeout >= cfg.timeouts.read);
    assert!(cfg.overall_timeout >= cfg.timeouts.write);
    assert!(cfg.validate().is_ok());
}

#[test]
fn invalid_timeout_config_fails_closed() {
    let cfg = SdkConfig {
        transport: Transport::Tls,
        overall_timeout: Duration::from_millis(500),
        timeouts: Timeouts {
            connect: Duration::from_millis(100),
            read: Duration::from_secs(2),
            write: Duration::from_secs(2),
        },
        ..Default::default()
    };

    let err = cfg
        .validate()
        .expect_err("overall timeout below read/write timeout must fail");

    assert!(
        err.to_string().contains("overall_timeout"),
        "unexpected validation error: {err}"
    );
}

#[tokio::test]
async fn storage_get_rejects_zero_deadline() {
    let transport = TransportHandle::new(valid_sdk_config());
    let metrics = NoopSdkMetrics;
    let cap = dummy_capability();
    let addr = AddrB3::parse("b3:0000000000000000000000000000000000000000000000000000000000000000")
        .expect("valid b3 test address");

    let err =
        ron_app_sdk::planes::storage::storage_get(&transport, &metrics, cap, &addr, Duration::ZERO)
            .await
            .expect_err("zero deadline must fail before transport");

    match err {
        SdkError::SchemaViolation { path, detail } => {
            assert_eq!(path, "storage_get.deadline");
            assert!(detail.contains("> 0"));
        }
        other => panic!("expected SchemaViolation, got {other:?}"),
    }
}
