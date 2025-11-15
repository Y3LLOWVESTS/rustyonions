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
use crate::transport::{OAP_MAX_FRAME_BYTES, TransportHandle};
use crate::types::{AddrB3, Capability};

/// Optional idempotency key type alias (from `idempotency.rs`).
pub type IdemKey = crate::idempotency::IdempotencyKey;

/// Logical metric endpoints for this plane.
const STORAGE_GET_ENDPOINT: &str = "storage_get";
const STORAGE_PUT_ENDPOINT: &str = "storage_put";

/// Fetch a blob by content ID.
///
/// This is a *thin* wrapper over the transport:
/// - Validates the deadline is non-zero (best-effort).
/// - Delegates to `TransportHandle::call_oap`.
/// - Emits latency + failure metrics.
///
/// Once `call_oap` is wired to `ron-transport`, this will perform an
/// OAP/1 request equivalent to `GET /o/{addr_b3}` at the gateway.
pub async fn storage_get(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    addr: &AddrB3,
    deadline: Duration,
) -> Result<Bytes, SdkError> {
    // Minimal guard: we expect callers to supply a sane deadline.
    if deadline == Duration::from_millis(0) {
        return Err(SdkError::schema_violation(
            "storage_get.deadline",
            "deadline must be > 0",
        ));
    }

    let start = Instant::now();

    // NOTE: For OAP/1 we treat the path as a logical endpoint; the
    // transport is responsible for mapping this into concrete HTTP/TCP.
    let endpoint = format!("/o/{}", addr.as_str());

    // For read operations we send an empty payload.
    let payload: &[u8] = &[];

    // We do *not* log or otherwise inspect the capability here — that
    // should already have been validated at issue-time by svc-passport.
    let _ = cap; // placeholder until we thread caps into call_oap.

    let result = transport.call_oap(&endpoint, payload, deadline).await;
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
/// - Rejects payloads larger than `OAP_MAX_FRAME_BYTES` with
///   `SdkError::OapViolation`.
/// - Delegates to transport for the actual OAP/1 call (`POST /put`).
/// - Attempts to parse an `AddrB3` from the response body.
/// - Emits latency + failure metrics.
///
/// Idempotency:
/// - `idem_key` is *logical*; it is up to the transport / gateway to
///   coalesce retried requests that share the same key. The SDK only
///   ensures it is supplied.
pub async fn storage_put(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    blob: Bytes,
    deadline: Duration,
    idem_key: Option<IdemKey>,
) -> Result<AddrB3, SdkError> {
    if deadline == Duration::from_millis(0) {
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

    // For now we ignore the idempotency key at the transport layer; it
    // will be threaded into the OAP header set once we define the wire
    // format for idempotent requests.
    let _ = idem_key;
    let _ = cap;

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

/// Try to parse an `AddrB3` from the gateway response body.
///
/// For now we accept a *plain text* body with the canonical `b3:`
/// string. If we later standardise on a JSON envelope (e.g.
/// `{"cid":"b3:..."}`) we can extend this parser without changing the
/// public API of `storage_put`.
fn parse_addr_b3_from_body(body: &[u8]) -> Result<AddrB3, SdkError> {
    let s = std::str::from_utf8(body).map_err(|_| {
        SdkError::schema_violation("storage_put.body", "response was not valid UTF-8")
    })?;

    let trimmed = s.trim();

    AddrB3::parse(trimmed).map_err(|_| {
        SdkError::schema_violation("storage_put.body", "response did not contain a valid AddrB3")
    })
}

/// Map an error into a coarse, low-cardinality reason string for metrics.
///
/// This keeps the label space small while still distinguishing the
/// obvious buckets we care about.
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
        // Minimal, obviously-not-real capability suitable for exercising
        // client-side invariants in tests. We intentionally keep this
        // in-sync with `CapTokenHdr` from `ron-proto`.
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

        // Construct a payload one byte larger than the allowed frame cap.
        let oversized = Bytes::from(vec![0u8; OAP_MAX_FRAME_BYTES + 1]);
        let deadline = Duration::from_secs(1);

        let err = storage_put(&transport, &metrics, cap, oversized, deadline, None)
            .await
            .expect_err("expected OapViolation for oversized payload");

        match err {
            SdkError::OapViolation { reason } => {
                assert_eq!(reason, "payload-too-large");
            }
            other => panic!("expected OapViolation, got {:?}", other),
        }
    }

    #[test]
    fn parse_addr_b3_rejects_garbage() {
        let body = b"not-a-valid-b3-id";
        let err = parse_addr_b3_from_body(body).expect_err("should reject invalid CID");
        match err {
            SdkError::SchemaViolation { path, .. } => {
                assert_eq!(path, "storage_put.body");
            }
            other => panic!("expected SchemaViolation, got {:?}", other),
        }
    }
}
