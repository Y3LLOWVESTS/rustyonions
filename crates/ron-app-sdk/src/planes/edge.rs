// REPLACE ENTIRE FILE with this version
//! RO:WHAT — Edge plane helpers.
//! RO:WHY  — Ergonomic facade over svc-edge GETs with inclusive ranges.

use std::time::Duration;

use bytes::Bytes;

use crate::{
    errors::SdkError,
    metrics::SdkMetrics,
    transport::TransportHandle,
    types::{ByteRange, Capability},
};

const EP_GET: &str = "/edge/get";
const METRIC_ENDPOINT: &str = "edge_get";

#[derive(serde::Serialize)]
struct EdgeGetReq<'a> {
    cap: &'a Capability,
    path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<RangeDto>,
}

#[derive(serde::Serialize)]
struct RangeDto {
    start: u64,
    end: u64, // inclusive
}

pub async fn edge_get(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    path: &str,
    range: Option<ByteRange>,
    deadline: Duration,
) -> Result<Bytes, SdkError> {
    if let Some(ref r) = range {
        validate_range(r)?;
        let _ = r.len(); // keep inclusive semantics front-and-center
    }

    if deadline.as_millis() == 0 {
        return Err(SdkError::schema_violation(
            "edge_get.deadline",
            "deadline must be > 0",
        ));
    }

    let dto = EdgeGetReq {
        cap: &cap,
        path,
        range: range.map(|r| RangeDto {
            start: r.start,
            end: r.end,
        }),
    };

    let t0 = std::time::Instant::now();
    let raw = transport.call_oap_json(EP_GET, &dto, deadline).await;
    let ms = t0.elapsed().as_millis() as u64;

    match raw {
        Ok(bytes) => {
            metrics.observe_latency(METRIC_ENDPOINT, true, ms);
            Ok(Bytes::from(bytes))
        }
        Err(err) => {
            metrics.observe_latency(METRIC_ENDPOINT, false, ms);
            metrics.inc_failure(METRIC_ENDPOINT, classify(&err));
            Err(err)
        }
    }
}

fn validate_range(range: &ByteRange) -> Result<(), SdkError> {
    if range.end < range.start {
        return Err(SdkError::OapViolation { reason: "range" });
    }
    Ok(())
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
    fn byte_range_validation_accepts_well_formed_range() {
        let r = ByteRange { start: 0, end: 9 };
        assert!(super::validate_range(&r).is_ok());
    }

    #[test]
    fn byte_range_validation_rejects_inverted_range() {
        let r = ByteRange { start: 10, end: 5 };
        let err = super::validate_range(&r).unwrap_err();
        match err {
            SdkError::OapViolation { reason } => assert_eq!(reason, "range"),
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn byte_range_header_format_is_inclusive() {
        let r = ByteRange { start: 0, end: 65_535 };
        assert_eq!(r.len(), 65_536);
    }
}
