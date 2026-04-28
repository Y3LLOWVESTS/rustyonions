//! RO:WHAT — JSON error envelope for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: DX/SEC/GOV. Keeps deterministic error response shape aligned with OpenAPI.
//! RO:INTERACTS — crate::errors and routes.
//! RO:INVARIANTS — stable code strings; messages are redacted; corr_id is echoed when supplied.
//! RO:METRICS — route layer records code into wallet_rejects_total.
//! RO:CONFIG — none.
//! RO:SECURITY — never include Authorization, macaroon, or secret material in details.
//! RO:TEST — from_wallet_error_maps_status_and_retryable.

use serde::{Deserialize, Serialize};

use crate::errors::WalletError;

/// JSON error envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    /// Stable machine-readable code.
    pub code: String,
    /// HTTP status code.
    pub http: u16,
    /// Small redacted message.
    pub message: String,
    /// Whether retrying the same request identity may succeed.
    pub retryable: bool,
    /// Correlation id, if present.
    pub corr_id: Option<String>,
}

impl ErrorResponse {
    /// Convert a wallet error into an error response.
    pub fn from_error(error: &WalletError, corr_id: Option<String>) -> Self {
        Self {
            code: error.code.as_str().to_string(),
            http: error.http_status(),
            message: error.message.clone(),
            retryable: error.retryable(),
            corr_id,
        }
    }
}
