//! RO:WHAT — Transport handle + retry wrapper for ron-app-sdk.
//! RO:WHY  — Central place to enforce OAP/1 limits (1 MiB max frame)
//!           and apply per-call deadlines/retries, independent of
//!           concrete transport flavor (TLS/Tor).
//! RO:INTERACTS — Uses crate::config::{SdkConfig, Transport},
//!                crate::errors::{SdkError, RetryClass},
//!                crate::retry::backoff_schedule, and reqwest for HTTPS;
//!                called by planes::{storage, edge, mailbox, index}.
//! RO:INVARIANTS — client-only (no listeners); no lock across `.await`;
//!                 OAP_MAX_FRAME_BYTES enforced before network I/O;
//!                 outer deadlines respected including backoff sleeps.
//! RO:METRICS — planes record metrics around these calls; this module
//!              itself is metric-agnostic.
//! RO:CONFIG — reads SdkConfig.transport/gateway_addr/overall_timeout/
//!             timeouts/retry.
//! RO:SECURITY — TLS verification delegated to reqwest+rustls;
//!               capability headers/payloads are supplied by planes;
//!               no secrets logged here.
//! RO:TEST — local unit tests for size/deadline behavior; integration
//!           invariants in `tests/i_3_oap_bounds.rs` and
//!           `tests/i_5_retries_deadlines.rs` once fully wired.

use std::{cmp, time::Duration};

use tokio::time::{sleep, Instant};

use crate::config::{SdkConfig, Transport as TransportKind};
use crate::errors::{RetryClass, SdkError};
use crate::retry::backoff_schedule;

use super::mapping::{map_http_status, map_reqwest_error};

/// Hard OAP/1 frame size cap (1 MiB).
///
/// Callers must ensure no single OAP DATA frame ever exceeds this
/// size. The transport adapter enforces this before any network I/O
/// is attempted, so oversized payloads fail fast client-side.
pub const OAP_MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Opaque handle for SDK transport.
///
/// For now this wraps `SdkConfig` and builds a `reqwest::Client`
/// per-call. Once we wire in a richer OAP client or connection
/// pooling, this type is where that will live.
#[derive(Debug, Clone)]
pub struct TransportHandle {
    cfg: SdkConfig,
}

impl TransportHandle {
    /// Construct a new handle from configuration.
    ///
    /// This is intentionally infallible so that `RonAppSdk::new` can
    /// stay simple; any construction errors for the underlying HTTP
    /// client are surfaced at call time as `SdkError`.
    pub fn new(cfg: SdkConfig) -> Self {
        Self { cfg }
    }

    /// Access the underlying configuration.
    pub fn config(&self) -> &SdkConfig {
        &self.cfg
    }

    /// Convenience: serialize `value` as JSON and send via OAP POST.
    ///
    /// This helper centralizes JSON encoding so planes can remain thin.
    pub async fn call_oap_json<T: serde::Serialize>(
        &self,
        endpoint: &str,
        value: &T,
        deadline: Duration,
    ) -> Result<Vec<u8>, SdkError> {
        let body = serde_json::to_vec(value)
            .map_err(|e| SdkError::Unknown(format!("json encode failed: {e}")))?;
        self.call_oap(endpoint, &body, deadline).await
    }

    /// Perform a single low-level OAP request over the configured transport.
    ///
    /// This method:
    /// - Enforces `OAP_MAX_FRAME_BYTES` at the SDK boundary.
    /// - Honors `SdkConfig::transport` (currently: TLS only; Tor is
    ///   fail-fast with `SdkError::TorUnavailable`).
    /// - Clamps the per-call `deadline` by `SdkConfig::overall_timeout`.
    /// - Maps HTTP status codes + reqwest errors into `SdkError`.
    ///
    /// It does **not** perform retries; see `call_oap_with_retry` for
    /// the higher-level wrapper that uses `RetryCfg` + `RetryClass`.
    pub async fn call_oap(
        &self,
        endpoint: &str,
        payload: &[u8],
        deadline: Duration,
    ) -> Result<Vec<u8>, SdkError> {
        // Enforce OAP/1 max frame cap at SDK boundary.
        if payload.len() > OAP_MAX_FRAME_BYTES {
            return Err(SdkError::OapViolation {
                reason: "oap payload exceeds OAP_MAX_FRAME_BYTES (1 MiB)",
            });
        }

        // Tor transport is not wired yet; fail-fast with a stable error.
        if matches!(self.cfg.transport, TransportKind::Tor) {
            return Err(SdkError::TorUnavailable);
        }

        // Clamp the per-call deadline by the config-level overall timeout.
        let cfg_deadline = self.cfg.overall_timeout;
        let effective_deadline = cmp::min(deadline, cfg_deadline);
        if effective_deadline.is_zero() {
            return Err(SdkError::DeadlineExceeded);
        }

        // Build the full URL: <gateway_addr>/<endpoint>.
        let base = self.cfg.gateway_addr.trim_end_matches('/');
        let path = endpoint.trim_start_matches('/');
        let url = format!("{base}/{path}");

        // Minimal HTTP client; we keep it per-call for now to keep
        // `TransportHandle::new` infallible. We can pool this later.
        let client = reqwest::Client::builder()
            .connect_timeout(self.cfg.timeouts.connect)
            .timeout(effective_deadline)
            .build()
            .map_err(map_reqwest_error)?;

        let resp = client
            .post(&url)
            .body(payload.to_vec())
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = resp.status();

        if !status.is_success() {
            // Extract Retry-After if present (for 429).
            let retry_after = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                resp.headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs)
            } else {
                None
            };

            return Err(map_http_status(status, retry_after));
        }

        let body = resp.bytes().await.map_err(map_reqwest_error)?;
        Ok(body.to_vec())
    }

    /// Perform an OAP request with retries and deadline awareness.
    ///
    /// Behavior:
    /// - Uses `RetryCfg` from `SdkConfig` to drive exponential backoff.
    /// - Uses `SdkError::retry_class()` to decide which failures are
    ///   safe to retry.
    /// - Enforces an **outer** deadline equal to
    ///   `min(deadline, overall_timeout)`, including sleep.
    /// - Respects `max_attempts` (including the initial attempt).
    ///
    /// This is the helper that plane modules should normally call.
    pub async fn call_oap_with_retry(
        &self,
        endpoint: &str,
        payload: &[u8],
        deadline: Duration,
    ) -> Result<Vec<u8>, SdkError> {
        let outer_deadline = cmp::min(deadline, self.cfg.overall_timeout);
        if outer_deadline.is_zero() {
            return Err(SdkError::DeadlineExceeded);
        }

        let start = Instant::now();
        let retry_cfg = &self.cfg.retry;

        // Attempt 0 is the initial attempt (no backoff before it).
        let mut attempt: u32 = 0;

        loop {
            let elapsed = start.elapsed();
            if elapsed >= outer_deadline {
                return Err(SdkError::DeadlineExceeded);
            }

            let remaining = outer_deadline.saturating_sub(elapsed);

            match self.call_oap(endpoint, payload, remaining).await {
                Ok(bytes) => return Ok(bytes),
                Err(err) => {
                    // If this isn't retriable, bail immediately.
                    if !matches!(err.retry_class(), RetryClass::Retriable) {
                        return Err(err);
                    }

                    // `max_attempts` is total attempts including the first.
                    let max_attempts = if retry_cfg.max_attempts == 0 {
                        1
                    } else {
                        retry_cfg.max_attempts
                    };

                    // We've already done `attempt + 1` attempts (initial + retries so far).
                    if attempt + 1 >= max_attempts {
                        return Err(err);
                    }

                    // Compute backoff for this retry attempt.
                    let delay = backoff_schedule(retry_cfg)
                        .nth(attempt as usize)
                        .unwrap_or(retry_cfg.cap);

                    let sleep_dur = cmp::min(delay, outer_deadline.saturating_sub(start.elapsed()));

                    if !sleep_dur.is_zero() {
                        sleep(sleep_dur).await;
                    }
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CacheCfg, IdemCfg, PqMode, RetryCfg, SdkConfig, Timeouts, TorCfg, TracingCfg,
        Transport as TransportKind,
    };

    fn dummy_config() -> SdkConfig {
        SdkConfig {
            transport: TransportKind::Tls,
            gateway_addr: "http://127.0.0.1:8080".to_string(),
            overall_timeout: Duration::from_millis(5000),
            timeouts: Timeouts::default(),
            retry: RetryCfg::default(),
            idempotency: IdemCfg::default(),
            cache: CacheCfg::default(),
            tracing: TracingCfg::default(),
            pq_mode: PqMode::Off,
            tor: TorCfg::default(),
        }
    }

    /// Oversized payloads must fail fast without network I/O.
    #[tokio::test]
    async fn rejects_payload_larger_than_oap_cap() {
        let cfg = dummy_config();
        let handle = TransportHandle::new(cfg);
        let payload = vec![0u8; OAP_MAX_FRAME_BYTES + 1];

        let res = handle
            .call_oap("healthz", &payload, Duration::from_secs(1))
            .await;

        assert!(matches!(res, Err(SdkError::OapViolation { .. })));
    }

    /// Zero deadlines must be rejected up front.
    #[tokio::test]
    async fn deadline_zero_fails_fast() {
        let cfg = dummy_config();
        let handle = TransportHandle::new(cfg);
        let payload: [u8; 0] = [];

        let res = handle.call_oap("healthz", &payload, Duration::ZERO).await;

        assert!(matches!(res, Err(SdkError::DeadlineExceeded)));
    }

    /// Outer deadline should stop retries rather than looping forever.
    #[tokio::test]
    async fn outer_deadline_limits_retries() {
        let mut cfg = dummy_config();
        cfg.overall_timeout = Duration::from_millis(10);
        cfg.retry.max_attempts = 10;
        let handle = TransportHandle::new(cfg);

        let payload: [u8; 0] = [];
        let res = handle
            .call_oap_with_retry("healthz", &payload, Duration::ZERO)
            .await;

        assert!(matches!(res, Err(SdkError::DeadlineExceeded)));
    }
}
