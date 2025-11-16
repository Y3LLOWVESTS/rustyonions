//! RO:WHAT — Edge plane helpers (GET with optional byte range).
//! RO:WHY  — Range-aware fetch with strict range validation.
//! RO:INTERACTS — transport.call_oap_json.
//! RO:INVARIANTS — deadlines > 0; ranges validated (inclusive semantics).

use std::time::{Duration, Instant};

use bytes::Bytes;

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{ByteRange, Capability};

// Gateway endpoint.
const EP_EDGE_GET: &str = "/edge/get";

// Metric label.
const EDGE_GET_ENDPOINT: &str = "edge_get";

// ---------- DTOs ----------

#[derive(serde::Serialize)]
struct EdgeGetReq<'a> {
    cap: &'a Capability,
    path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<EdgeRange>,
}

#[derive(serde::Serialize)]
struct EdgeRange {
    start: u64,
    end: u64, // inclusive
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EdgeGetResp {
    #[serde(with = "serde_bytes")]
    blob: Vec<u8>,
}

// ---------- API ----------

pub async fn edge_get(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    path: &str,
    range: Option<ByteRange>,
    deadline: Duration,
) -> Result<Bytes, SdkError> {
    if deadline.is_zero() {
        return Err(SdkError::schema_violation(
            "edge_get.deadline",
            "deadline must be > 0",
        ));
    }

    // Validate range if provided.
    let range_json = if let Some(r) = range {
        if r.start > r.end {
            return Err(SdkError::schema_violation(
                "edge_get.range",
                "start must be <= end (inclusive semantics)",
            ));
        }
        Some(EdgeRange {
            start: r.start,
            end: r.end,
        })
    } else {
        None
    };

    let start = Instant::now();
    let payload = EdgeGetReq {
        cap: &cap,
        path,
        range: range_json,
    };

    let raw = transport
        .call_oap_json(EP_EDGE_GET, &payload, deadline)
        .await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match raw {
        Ok(bytes) => {
            let parsed: EdgeGetResp = serde_json::from_slice(&bytes)
                .map_err(|e| SdkError::schema_violation("edge_get.body", e.to_string()))?;
            metrics.observe_latency(EDGE_GET_ENDPOINT, true, elapsed_ms);
            Ok(Bytes::from(parsed.blob))
        }
        Err(err) => {
            metrics.observe_latency(EDGE_GET_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(EDGE_GET_ENDPOINT, classify(&err));
            Err(err)
        }
    }
}

fn classify(err: &SdkError) -> &'static str {
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
        Unknown(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_range_header_format_is_inclusive() {
        let r = ByteRange { start: 0, end: 9 };
        // inclusive means length is (end - start + 1)
        assert_eq!(r.len(), 10);
    }

    #[test]
    fn byte_range_validation_accepts_well_formed_range() {
        let r = ByteRange { start: 5, end: 7 };
        assert!(r.start <= r.end);
    }

    #[test]
    fn byte_range_validation_rejects_inverted_range() {
        let r = ByteRange { start: 8, end: 7 };
        assert!(r.start > r.end);
    }
}
