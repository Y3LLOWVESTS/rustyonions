//! RO:WHAT — I-3 OAP frame-bound checks for the SDK boundary.
//! RO:WHY — Ensures SDK callers reject oversized mutation payloads before transport.
//! RO:INTERACTS — transport::OAP_MAX_FRAME_BYTES, storage plane payload validation.
//! RO:INVARIANTS — OAP max frame is 1 MiB; 64 KiB remains the normal streaming chunk size.
//! RO:SECURITY — Prevents unbounded client-side buffering and oversized frame construction.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::{hint::black_box, time::Duration};

use bytes::Bytes;
use ron_app_sdk::{
    transport::{TransportHandle, OAP_MAX_FRAME_BYTES},
    Capability, NoopSdkMetrics, SdkConfig, SdkError,
};

fn dummy_capability() -> Capability {
    Capability {
        subject: "test-subject".to_owned(),
        scope: "storage:write".to_owned(),
        issued_at: 0,
        expires_at: u64::MAX,
        caveats: Vec::new(),
    }
}

#[tokio::test]
async fn storage_put_rejects_one_byte_over_oap_frame_cap() {
    let cfg = SdkConfig::default();
    let transport = TransportHandle::new(cfg);
    let metrics = NoopSdkMetrics;
    let cap = dummy_capability();
    let oversized = Bytes::from(vec![0u8; OAP_MAX_FRAME_BYTES.saturating_add(1)]);

    let err = ron_app_sdk::planes::storage::storage_put(
        &transport,
        &metrics,
        cap,
        oversized,
        Duration::from_secs(1),
        None,
    )
    .await
    .expect_err("payload over OAP cap must fail before transport");

    match err {
        SdkError::OapViolation { reason } => assert_eq!(reason, "payload-too-large"),
        other => panic!("expected OapViolation, got {other:?}"),
    }
}

#[test]
fn canonical_oap_and_streaming_bounds_are_distinct() {
    let oap_max = black_box(OAP_MAX_FRAME_BYTES);
    let stream_chunk = black_box(64usize * 1024);

    assert_eq!(oap_max, 1_048_576);
    assert_eq!(stream_chunk, 65_536);
    assert!(
        stream_chunk < oap_max,
        "stream chunks must remain smaller than the OAP frame cap"
    );
    assert_eq!(oap_max / stream_chunk, 16);
}
