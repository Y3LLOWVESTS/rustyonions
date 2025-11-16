//! RO:WHAT — Index plane helpers (logical key → AddrB3 resolve).
//! RO:WHY  — Capability-first resolve with strict schema checks.
//! RO:INTERACTS — transport.call_oap_json.
//! RO:INVARIANTS — deadlines > 0; returned address must parse as AddrB3.

use std::time::{Duration, Instant};

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{AddrB3, Capability, IndexKey};

// Gateway endpoint.
const EP_INDEX_RESOLVE: &str = "/index/resolve";

// Metric label.
const INDEX_RESOLVE_ENDPOINT: &str = "index_resolve";

// ---------- DTOs ----------

#[derive(serde::Serialize)]
struct ResolveReq<'a> {
    cap: &'a Capability,
    key: &'a IndexKey,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveResp {
    addr_b3: String, // "b3:<hex64>"
}

// ---------- API ----------

pub async fn index_resolve(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    key: &IndexKey,
    deadline: Duration,
) -> Result<AddrB3, SdkError> {
    if deadline.is_zero() {
        // Keep metrics trait simple for beta: just latency + failure classification.
        metrics.observe_latency(INDEX_RESOLVE_ENDPOINT, false, 0);
        metrics.inc_failure(INDEX_RESOLVE_ENDPOINT, "index_resolve.deadline");
        return Err(SdkError::schema_violation(
            "index_resolve.deadline",
            "deadline must be > 0",
        ));
    }

    let started = Instant::now();
    let payload = ResolveReq { cap: &cap, key };

    // Serialize + transport.
    let raw = transport
        .call_oap_json(EP_INDEX_RESOLVE, &payload, deadline)
        .await;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    match raw {
        Ok(bytes) => {
            // Parse body.
            let parsed: ResolveResp = match serde_json::from_slice(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    metrics.observe_latency(INDEX_RESOLVE_ENDPOINT, false, elapsed_ms);
                    metrics.inc_failure(INDEX_RESOLVE_ENDPOINT, "index_resolve.body");
                    return Err(SdkError::schema_violation(
                        "index_resolve.body",
                        e.to_string(),
                    ));
                }
            };

            // Validate the returned address.
            let addr = AddrB3::parse(&parsed.addr_b3).map_err(|e| {
                metrics.observe_latency(INDEX_RESOLVE_ENDPOINT, false, elapsed_ms);
                metrics.inc_failure(INDEX_RESOLVE_ENDPOINT, "index_resolve.addr");
                SdkError::schema_violation("index_resolve.addr_b3", e.to_string())
            })?;

            metrics.observe_latency(INDEX_RESOLVE_ENDPOINT, true, elapsed_ms);
            Ok(addr)
        }
        Err(err) => {
            metrics.observe_latency(INDEX_RESOLVE_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(INDEX_RESOLVE_ENDPOINT, classify(&err));
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
    fn dummy_compile_only() {
        // Keep a tiny test so the module is exercised without needing
        // helpers from other crates.
        let _ = (INDEX_RESOLVE_ENDPOINT, EP_INDEX_RESOLVE);
    }
}
