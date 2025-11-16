//! RO:WHAT — Storage plane helpers for content-addressed blobs.
//! RO:WHY  — Give SDK users a boring `get/put` API on top of OAP/1 and
//!           capabilities, with size caps and error taxonomy handled here.
//! RO:INTERACTS — Delegates to `TransportHandle::call_oap` and uses
//!                `SdkMetrics` for latency/failure tracking.
//! RO:INVARIANTS —
//!   - All calls require a `Capability`; no anonymous storage access.
//!   - Requests larger than `OAP_MAX_FRAME_BYTES` are rejected client-side.
//!   - Errors are surfaced as `SdkError` (no panics).
//! RO:METRICS — Uses `SdkMetrics` with low-cardinality endpoints
//!              (`"storage_get"`, `"storage_put"`).
//! RO:CONFIG — Size cap is enforced via `OAP_MAX_FRAME_BYTES`; deadlines
//!             are per-call and must be supplied by the caller.
//! RO:SECURITY — Capability header must already encode macaroon-style
//!               restrictions; we do not log capability contents.
//! RO:TEST HOOKS — Unit tests here; integration/interop tests live under
//!                  `tests/i_*` once we wire real transport.

use std::time::{Duration, Instant};

use bytes::Bytes;

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::{TransportHandle, OAP_MAX_FRAME_BYTES};
use crate::types::{AddrB3, Capability};

/// Optional idempotency key type alias (from `idempotency.rs`).
pub type IdemKey = crate::idempotency::IdempotencyKey;

/// Logical metric endpoints for this plane.
const STORAGE_GET_ENDPOINT: &str = "storage_get";
const STORAGE_PUT_ENDPOINT: &str = "storage_put";

/// Fetch a blob by content ID.
///
/// Thin wrapper:
/// - Validates the deadline is non-zero.
/// - Delegates to `TransportHandle::call_oap`.
/// - Emits latency + failure metrics.
///
/// Transport currently treats `endpoint` as a logical path and maps
/// it to concrete HTTP. We use `/o/<b3>` for reads (raw bytes body).
pub async fn storage_get(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    addr: &AddrB3,
    deadline: Duration,
) -> Result<Bytes, SdkError> {
    if deadline.is_zero() {
        return Err(SdkError::schema_violation(
            "storage_get.deadline",
            "deadline must be > 0",
        ));
    }

    let start = Instant::now();
    let endpoint = format!("/o/{}", addr.as_str());

    // Capability threading is a TODO for the transport layer. For now
    // it is validated at the gateway; we avoid logging cap contents.
    let _ = cap;

    let result = transport.call_oap(&endpoint, &[], deadline).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(body) => {
            metrics.observe_latency(STORAGE_GET_ENDPOINT, true, elapsed_ms);
            Ok(Bytes::from(body))
        }
        Err(err) => {
            metrics.observe_latency(STORAGE_GET_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(STORAGE_GET_ENDPOINT, classify_error(&err));
            Err(err)
        }
    }
}

/// Store a blob and return its content ID (`AddrB3`).
///
/// Behavior:
/// - Rejects payloads larger than `OAP_MAX_FRAME_BYTES`.
/// - Delegates to transport (`POST /put`) returning plain-text `b3:...`.
/// - Parses/validates `AddrB3`.
/// - Emits latency + failure metrics.
///
/// Idempotency:
/// - `idem_key` is forwarded once the wire header is finalized. For now
///   it’s accepted but not yet serialized on the wire.
pub async fn storage_put(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    blob: Bytes,
    deadline: Duration,
    idem_key: Option<IdemKey>,
) -> Result<AddrB3, SdkError> {
    if deadline.is_zero() {
        return Err(SdkError::schema_violation(
            "storage_put.deadline",
            "deadline must be > 0",
        ));
    }

    // Enforce the OAP frame cap at the SDK boundary.
    if blob.len() > OAP_MAX_FRAME_BYTES {
        return Err(SdkError::OapViolation {
            reason: "payload-too-large",
        });
    }

    let start = Instant::now();
    let endpoint = "/put";

    // TODO: pass capability + idempotency key in OAP headers once the
    // client transport supports it. Keep them out of logs.
    let _ = cap;
    let _ = idem_key;

    let result = transport
        .call_oap(endpoint, blob.as_ref(), deadline)
        .await
        .and_then(|body| parse_addr_b3_from_body(&body));

    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(addr) => {
            metrics.observe_latency(STORAGE_PUT_ENDPOINT, true, elapsed_ms);
            Ok(addr)
        }
        Err(err) => {
            metrics.observe_latency(STORAGE_PUT_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(STORAGE_PUT_ENDPOINT, classify_error(&err));
            Err(err)
        }
    }
}

/// Try to parse an `AddrB3` from the gateway response body (plain text).
///
/// Accepts canonical `b3:<64 hex>`; returns `SchemaViolation` if the
/// body is not valid UTF-8 or not a valid address string.
fn parse_addr_b3_from_body(body: &[u8]) -> Result<AddrB3, SdkError> {
    let s = std::str::from_utf8(body).map_err(|_| {
        SdkError::schema_violation("storage_put.body", "response was not valid UTF-8")
    })?;
    let trimmed = s.trim();
    AddrB3::parse(trimmed).map_err(|_| {
        SdkError::schema_violation(
            "storage_put.body",
            "response did not contain a valid AddrB3",
        )
    })
}

/// Map an error into a coarse, low-cardinality reason string for metrics.
fn classify_error(err: &SdkError) -> &'static str {
    use crate::errors::RetryClass;
    use SdkError::*;

    match err {
        DeadlineExceeded => "deadline",
        Transport(_) => "transport",
        TorUnavailable => "tor",
        Tls => "tls",
        OapViolation { .. } => "oap",
        CapabilityExpired | CapabilityDenied => "capability",
        SchemaViolation { .. } => "schema",
        NotFound => "not_found",
        Conflict => "conflict",
        RateLimited { .. } => "rate_limited",
        Server(_) => "server",
        Unknown(_) => match err.retry_class() {
            RetryClass::Retriable => "unknown_retriable",
            RetryClass::NoRetry => "unknown_permanent",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use bytes::Bytes;

    use crate::config::SdkConfig;
    use crate::metrics::NoopSdkMetrics;
    use crate::transport::TransportHandle;

    fn dummy_capability() -> Capability {
        Capability {
            subject: "test-subject".to_string(),
            scope: "test-scope".to_string(),
            issued_at: 0,
            expires_at: u64::MAX,
            caveats: Vec::new(),
        }
    }

    #[tokio::test]
    async fn storage_put_rejects_payload_larger_than_oap_cap() {
        let cfg = SdkConfig::default();
        let transport = TransportHandle::new(cfg);
        let metrics = NoopSdkMetrics;
        let cap = dummy_capability();

        // One byte over cap → client-side failure.
        let oversized = Bytes::from(vec![0u8; OAP_MAX_FRAME_BYTES + 1]);
        let deadline = Duration::from_secs(1);

        let err = storage_put(&transport, &metrics, cap, oversized, deadline, None)
            .await
            .expect_err("expected OapViolation for oversized payload");

        match err {
            SdkError::OapViolation { reason } => assert_eq!(reason, "payload-too-large"),
            other => panic!("expected OapViolation, got {:?}", other),
        }
    }

    #[test]
    fn parse_addr_b3_rejects_garbage() {
        let body = b"not-a-valid-b3-id";
        let err = parse_addr_b3_from_body(body).expect_err("should reject invalid CID");
        match err {
            SdkError::SchemaViolation { path, .. } => assert_eq!(path, "storage_put.body"),
            other => panic!("expected SchemaViolation, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn storage_get_rejects_zero_deadline() {
        let cfg = SdkConfig::default();
        let transport = TransportHandle::new(cfg);
        let metrics = NoopSdkMetrics;
        let cap = dummy_capability();
        let addr =
            AddrB3::parse("b3:0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();

        let err = storage_get(&transport, &metrics, cap, &addr, Duration::ZERO)
            .await
            .expect_err("deadline=0 must fail");

        match err {
            SdkError::SchemaViolation { path, .. } => assert_eq!(path, "storage_get.deadline"),
            other => panic!("expected SchemaViolation, got {:?}", other),
        }
    }
}
