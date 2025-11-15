//! RO:WHAT — Thin transport adapter for ron-app-sdk.
//! RO:WHY  — Central place to enforce OAP/1 limits (1 MiB max frame)
//!           and apply per-call deadlines, independent of which
//!           concrete transport (TLS, Tor) is used beneath.
//! RO:INTERACTS — Currently uses `reqwest` + rustls to talk to the
//!                configured gateway; later may wrap `ron-transport`
//!                or a richer OAP client.
//! RO:INVARIANTS —
//!   - SDK is client-only; no server/listener code here.
//!   - All outbound calls go through this module once wired.
//!   - Enforces OAP max_frame = 1 MiB at the SDK boundary.
//! RO:SECURITY — No secrets are logged here; capability handling and
//!               DTO hygiene live in the plane modules.

use std::{cmp, time::Duration};

use crate::config::{SdkConfig, Transport as TransportKind};
use crate::errors::SdkError;

/// Hard OAP/1 frame size cap (1 MiB).
///
/// Callers must ensure no single OAP DATA frame ever exceeds this
/// size. The transport adapter enforces this before any network
/// I/O is attempted.
pub const OAP_MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Opaque handle for SDK transport.
///
/// For now this is just a wrapper around `SdkConfig`; once we hook it
/// to a richer transport abstraction this may hold concrete clients
/// for TLS and Tor transports.
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

    /// Access the underlying config.
    pub fn config(&self) -> &SdkConfig {
        &self.cfg
    }

    /// Perform a low-level OAP request over the configured transport.
    ///
    /// This is the entrypoint that plane modules (storage/edge/mailbox/
    /// index) will call once transport is fully wired. For now it
    /// performs a minimal HTTP POST to the configured gateway address
    /// using `reqwest` + rustls, enforcing:
    ///
    /// - the 1 MiB OAP frame cap, and
    /// - an effective deadline derived from the SDK config.
    pub async fn call_oap(
        &self,
        endpoint: &str,
        payload: &[u8],
        deadline: Duration,
    ) -> Result<Vec<u8>, SdkError> {
        // Enforce the hard 1 MiB frame cap at the SDK boundary.
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

        // Build the full URL: <gateway_addr>/<endpoint>.
        let base = self.cfg.gateway_addr.trim_end_matches('/');
        let path = endpoint.trim_start_matches('/');
        let url = format!("{base}/{path}");

        // Minimal HTTP client; we keep it per-call for now to keep
        // `TransportHandle::new` infallible. We can pool this later.
        let client = reqwest::Client::builder()
            .connect_timeout(self.cfg.timeouts.connect)
            // Use the smaller of per-call deadline and config timeout.
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
}

/// Map an HTTP status code into the stable `SdkError` taxonomy.
fn map_http_status(
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
) -> SdkError {
    use reqwest::StatusCode;

    match status {
        StatusCode::NOT_FOUND => SdkError::NotFound,
        StatusCode::CONFLICT => SdkError::Conflict,
        StatusCode::UNAUTHORIZED => SdkError::CapabilityExpired,
        StatusCode::FORBIDDEN => SdkError::CapabilityDenied,
        StatusCode::TOO_MANY_REQUESTS => SdkError::rate_limited(retry_after),
        s if s.is_server_error() => SdkError::Server(s.as_u16()),
        _ => SdkError::Unknown(format!("unexpected HTTP status {}", status)),
    }
}

/// Map `reqwest::Error` into `SdkError`.
fn map_reqwest_error(err: reqwest::Error) -> SdkError {
    use std::io::ErrorKind;

    if err.is_timeout() {
        return SdkError::DeadlineExceeded;
    }

    // For now treat all other reqwest errors as generic transport
    // failures. If we ever need finer-grained mapping we can inspect
    // the error categories (connect vs request vs body) here.
    if err.is_connect() || err.is_request() || err.is_body() {
        return SdkError::Transport(ErrorKind::Other);
    }

    SdkError::Unknown(err.to_string())
}
