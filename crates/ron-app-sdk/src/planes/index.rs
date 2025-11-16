// REPLACE ENTIRE FILE with this version
//! RO:WHAT — Index plane helpers (logical key → content address).
//! RO:WHY  — Provide a small, typed interface; retries via transport.

use std::time::{Duration, Instant};

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{AddrB3, Capability, IndexKey};

const ENDPOINT: &str = "/index/resolve";
const METRIC_ENDPOINT: &str = "index_resolve";

#[derive(serde::Serialize)]
struct ResolveReq<'a> {
    cap: &'a Capability,
    key: &'a IndexKey,
}

pub async fn index_resolve(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    key: &IndexKey,
    deadline: Duration,
) -> Result<AddrB3, SdkError> {
    if deadline.as_millis() == 0 {
        return Err(SdkError::schema_violation(
            "index_resolve.deadline",
            "deadline must be > 0",
        ));
    }

    let started = Instant::now();
    let req = ResolveReq { cap: &cap, key };

    let raw = transport.call_oap_json(ENDPOINT, &req, deadline).await;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    match raw {
        Ok(bytes) => {
            // Accept plain-text CID for now (e.g., "b3:...").
            let s = std::str::from_utf8(&bytes)
                .map_err(|e| SdkError::schema_violation("index_resolve.body", e.to_string()))?
                .trim()
                .to_string();

            let cid = AddrB3::parse(&s).map_err(|_| {
                SdkError::schema_violation(
                    "index_resolve.body",
                    "response did not contain a valid AddrB3",
                )
            })?;

            metrics.observe_latency(METRIC_ENDPOINT, true, elapsed_ms);
            Ok(cid)
        }
        Err(err) => {
            metrics.observe_latency(METRIC_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(METRIC_ENDPOINT, classify(&err));
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
