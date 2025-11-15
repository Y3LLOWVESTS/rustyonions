//! RO:WHAT — Storage plane helpers for content-addressed blobs.
//! RO:WHY  — Give SDK callers a simple `get/put` surface while centralizing
//!           retries, deadlines, OAP frame limits, and metrics.
//! RO:INTERACTS — `TransportHandle::call_oap`, `SdkMetrics`, `RetryCfg`,
//!                `IdemCfg`, `AddrB3` (`ContentId` in `ron-proto`), `Capability`.
//! RO:INVARIANTS —
//!   - Never blocks past the provided `deadline` (best effort; clamps retries).
//!   - Does not send frames > `OAP_MAX_FRAME_BYTES` on PUT.
//!   - Surfaces all failures as `SdkError` (no panics).
//!   - Keeps metric labels low-cardinality (`"storage_get"`, `"storage_put"` only).

use std::time::{Duration, Instant};

use bytes::Bytes;
use tokio::time::sleep;

use crate::errors::SdkError;
use crate::idempotency::derive_idempotency_key;
use crate::metrics::SdkMetrics;
use crate::retry::backoff_schedule;
use crate::transport::{TransportHandle, OAP_MAX_FRAME_BYTES};
use crate::types::{AddrB3, Capability, IdemKey};

/// Stable metric/endpoint labels — keep these low-cardinality.
const EP_STORAGE_GET: &str = "storage_get";
const EP_STORAGE_PUT: &str = "storage_put";

/// RO:WHAT — Map `SdkError` into a coarse, low-cardinality failure reason for metrics.
/// RO:WHY  — Avoid exploding label cardinality while still giving operators a useful view.
fn failure_reason(err: &SdkError) -> &'static str {
    use SdkError::*;
    match err {
        DeadlineExceeded => "deadline",
        Transport(_) => "transport",
        Tls => "tls",
        TorUnavailable => "tor",
        OapViolation { .. } => "oap_violation",
        CapabilityExpired => "cap_expired",
        CapabilityDenied => "cap_denied",
        SchemaViolation { .. } => "schema",
        NotFound => "not_found",
        Conflict => "conflict",
        RateLimited { .. } => "rate_limited",
        Server(_) => "server",
        Unknown(_) => "unknown",
    }
}

/// Perform a content-addressed GET from the storage plane.
///
/// Rough facade for `GET /o/{addr}` in the Micronode/Macronode surface once wired.
pub async fn storage_get(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    _cap: Capability,
    addr: &AddrB3,
    deadline: Duration,
) -> Result<Bytes, SdkError> {
    let retry_cfg = &transport.config().retry;
    let endpoint_path = format!("/o/{}", addr.as_str());

    let start = Instant::now();
    let mut last_err: Option<SdkError> = None;

    for (attempt, delay) in backoff_schedule(retry_cfg).enumerate() {
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            let err = SdkError::DeadlineExceeded;
            metrics.observe_latency(EP_STORAGE_GET, false, elapsed.as_millis() as u64);
            metrics.inc_failure(EP_STORAGE_GET, failure_reason(&err));
            return Err(err);
        }

        let remaining = deadline.saturating_sub(elapsed);
        let call_started = Instant::now();

        let result = transport.call_oap(&endpoint_path, &[], remaining).await;

        match result {
            Ok(body) => {
                let latency = call_started.elapsed().as_millis() as u64;
                metrics.observe_latency(EP_STORAGE_GET, true, latency);
                return Ok(Bytes::from(body));
            }
            Err(err) => {
                let latency = call_started.elapsed().as_millis() as u64;
                metrics.observe_latency(EP_STORAGE_GET, false, latency);
                metrics.inc_failure(EP_STORAGE_GET, failure_reason(&err));

                if !err.is_retriable() {
                    return Err(err);
                }

                last_err = Some(err);

                // If there is no backoff, retry immediately (still within deadline).
                if delay.is_zero() {
                    metrics.inc_retry(EP_STORAGE_GET);
                    continue;
                }

                let elapsed_after = start.elapsed();
                if elapsed_after + delay >= deadline {
                    // No time left for another full attempt.
                    break;
                }

                metrics.inc_retry(EP_STORAGE_GET);
                sleep(delay).await;

                let _ = attempt; // keep `attempt` referenced to avoid lint noise.
            }
        }
    }

    Err(last_err.unwrap_or(SdkError::DeadlineExceeded))
}

/// Perform a content-addressed PUT to the storage plane.
///
/// The SDK is responsible for respecting idempotency configuration and
/// retry posture; terminal verification (re-read and BLAKE3 check) is
/// optional and may be added later.
pub async fn storage_put(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    _cap: Capability,
    blob: Bytes,
    deadline: Duration,
    idem: Option<IdemKey>,
) -> Result<AddrB3, SdkError> {
    // Enforce the OAP send-frame cap at the plane boundary.
    if blob.len() > OAP_MAX_FRAME_BYTES {
        return Err(SdkError::OapViolation {
            reason: "payload-too-large",
        });
    }

    let cfg = transport.config();
    let retry_cfg = &cfg.retry;

    // Exercise idempotency derivation even if the caller didn’t provide a key,
    // so the idempotency helpers stay wired.
    if idem.is_none() {
        let _ = derive_idempotency_key(&cfg.idempotency, "POST", "/o", None);
    } else {
        let _ = idem.as_ref().map(|k| k.as_str());
    }

    let endpoint_path = "/o";
    let start = Instant::now();
    let mut last_err: Option<SdkError> = None;

    for (attempt, delay) in backoff_schedule(retry_cfg).enumerate() {
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            let err = SdkError::DeadlineExceeded;
            metrics.observe_latency(EP_STORAGE_PUT, false, elapsed.as_millis() as u64);
            metrics.inc_failure(EP_STORAGE_PUT, failure_reason(&err));
            return Err(err);
        }

        let remaining = deadline.saturating_sub(elapsed);
        let call_started = Instant::now();

        // For now the transport sees just raw bytes; the OAP/1 envelope will be
        // added once overlay/gateway wiring is in place.
        let result = transport
            .call_oap(endpoint_path, blob.as_ref(), remaining)
            .await;

        match result {
            Ok(body) => {
                let latency = call_started.elapsed().as_millis() as u64;
                metrics.observe_latency(EP_STORAGE_PUT, true, latency);

                // Try JSON `{ "cid": "<b3:...>" }` first (svc-storage/IDB contract),
                // then fall back to treating the whole body as a bare CID.
                let cid = parse_cid_from_body(&body)?;
                return Ok(cid);
            }
            Err(err) => {
                let latency = call_started.elapsed().as_millis() as u64;
                metrics.observe_latency(EP_STORAGE_PUT, false, latency);
                metrics.inc_failure(EP_STORAGE_PUT, failure_reason(&err));

                if !err.is_retriable() {
                    return Err(err);
                }

                last_err = Some(err);

                if delay.is_zero() {
                    metrics.inc_retry(EP_STORAGE_PUT);
                    continue;
                }

                let elapsed_after = start.elapsed();
                if elapsed_after + delay >= deadline {
                    break;
                }

                metrics.inc_retry(EP_STORAGE_PUT);
                sleep(delay).await;

                let _ = attempt;
            }
        }
    }

    Err(last_err.unwrap_or(SdkError::DeadlineExceeded))
}

/// RO:WHAT — Parse an `AddrB3` from a storage PUT response body.
/// RO:WHY  — Allow `svc-storage` style JSON (`{ "cid": "<b3:...>" }`) while
///           remaining tolerant of a plain-text CID body during early bring-up.
fn parse_cid_from_body(body: &[u8]) -> Result<AddrB3, SdkError> {
    if body.is_empty() {
        return Err(SdkError::schema_violation("cid", "empty response"));
    }

    let text = std::str::from_utf8(body)
        .map_err(|e| SdkError::schema_violation("cid", format!("non-utf8 response: {e}")))?;

    // Preferred form: JSON `{ "cid": "<b3:...>" }`.
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(cid) = json.get("cid").and_then(|v| v.as_str()) {
            return AddrB3::parse(cid)
                .map_err(|e| SdkError::schema_violation("cid", e.to_string()));
        }
    }

    // Fallback: treat the entire body as the CID string.
    AddrB3::parse(text.trim()).map_err(|e| SdkError::schema_violation("cid", e.to_string()))
}
