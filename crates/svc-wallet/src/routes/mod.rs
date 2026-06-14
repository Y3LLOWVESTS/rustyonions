//! RO:WHAT — Axum router, shared HTTP state, and route error adapters for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES/DX. This is the HTTP boundary around the wallet core.
//! RO:INTERACTS — routes::{health,metrics,v1}, auth, ledger, idem, seq, readiness, metrics.
//! RO:INVARIANTS — capability required on v1 paths; idempotent writes; no durable truth outside ron-ledger.
//! RO:METRICS — increments wallet requests, rejects, successes, op counters, and idempotency replays.
//! RO:CONFIG — WalletConfig is shared read-only via Arc.
//! RO:SECURITY — bearer tokens are consumed but never stored/logged; error envelopes are redacted.
//! RO:TEST — dev_state_builds_router; receipt_book_roundtrip.

pub mod health;
pub mod metrics;
pub mod v1;

use std::{collections::HashMap, sync::Arc};

use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json, Router,
};
use parking_lot::RwLock;

use crate::{
    accounting::client::NoopAccountingClient,
    auth::caps::{CapabilityClaims, CapabilityVerifier, StaticCapabilityVerifier, WalletScope},
    config::WalletConfig,
    dto::{errors::ErrorResponse, responses::Receipt},
    errors::{WalletError, WalletErrorCode, WalletResult},
    idem::store::IdempotencyStore,
    ledger::client::LocalLedgerClient,
    metrics::WalletMetrics,
    readiness::ReadinessGate,
    seq::nonce::NonceTable,
    util::headers::{X_CORR_ID, X_REQUEST_ID},
};

/// Shared Axum state for the wallet HTTP service.
#[derive(Clone)]
pub struct WalletState {
    /// Runtime config.
    pub config: Arc<WalletConfig>,
    /// Readiness gate.
    pub readiness: ReadinessGate,
    /// Metrics handle.
    pub metrics: WalletMetrics,
    /// In-process ledger adapter for current Phase 2 dev path.
    pub ledger: Arc<LocalLedgerClient<ron_ledger::engine::MemoryStorage>>,
    /// In-memory idempotency store.
    pub idem: Arc<IdempotencyStore>,
    /// In-memory nonce table.
    pub nonces: Arc<NonceTable>,
    /// Capability verifier seam.
    pub cap_verifier: Arc<dyn CapabilityVerifier>,
    /// Accounting seam.
    pub accounting: NoopAccountingClient,
    receipts: Arc<RwLock<HashMap<String, Receipt>>>,
}

impl WalletState {
    /// Build a local amnesia-safe dev state using the in-memory ledger adapter.
    pub fn dev() -> WalletResult<Self> {
        let config = WalletConfig::default();
        config.validate()?;

        let readiness = ReadinessGate::new();
        readiness.mark_ready();

        let claims = CapabilityClaims {
            subject: "svc-wallet-dev".to_string(),
            scopes: vec![
                WalletScope::Read,
                WalletScope::Issue,
                WalletScope::Transfer,
                WalletScope::Burn,
            ],
            accounts: Vec::new(),
            assets: vec![config.asset.clone()],
        };

        Ok(Self {
            idem: Arc::new(IdempotencyStore::new(config.idempotency_ttl())),
            ledger: Arc::new(LocalLedgerClient::in_memory()?),
            config: Arc::new(config),
            readiness,
            metrics: WalletMetrics::default(),
            nonces: Arc::new(NonceTable::default()),
            cap_verifier: Arc::new(StaticCapabilityVerifier::new(claims)),
            accounting: NoopAccountingClient,
            receipts: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Record a short receipt for GET /v1/tx/{txid}.
    pub fn remember_receipt(&self, receipt: Receipt) {
        self.receipts.write().insert(receipt.txid.clone(), receipt);
    }

    /// Get a short receipt from the local receipt book.
    pub fn receipt(&self, txid: &str) -> Option<Receipt> {
        self.receipts.read().get(txid).cloned()
    }

    /// Convert an internal wallet error to an HTTP error and record metrics.
    pub fn reject(&self, error: WalletError, corr_id: Option<String>) -> HttpError {
        self.metrics.inc_reject(error.code.as_str());
        HttpError { error, corr_id }
    }
}

/// Build the svc-wallet HTTP router.
pub fn router(state: WalletState) -> Router {
    Router::new()
        .route("/healthz", axum::routing::get(health::healthz))
        .route("/readyz", axum::routing::get(health::readyz))
        .route("/metrics", axum::routing::get(metrics::metrics))
        .nest("/v1", v1::router())
        .with_state(state)
}

/// Route-layer HTTP error.
#[derive(Debug, Clone)]
pub struct HttpError {
    /// Stable wallet error.
    pub error: WalletError,
    /// Optional correlation id.
    pub corr_id: Option<String>,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.error.http_status())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = ErrorResponse::from_error(&self.error, self.corr_id);
        (status, Json(body)).into_response()
    }
}

/// Extract an optional correlation id.
pub fn corr_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(X_CORR_ID)
        .or_else(|| headers.get(X_REQUEST_ID))
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(ToOwned::to_owned)
}

/// Extract bearer token without logging it.
pub fn bearer_from_headers(headers: &HeaderMap) -> WalletResult<String> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| WalletError::new(WalletErrorCode::Unauthorized, "missing Authorization"))?
        .to_str()
        .map_err(|_| WalletError::new(WalletErrorCode::Unauthorized, "invalid Authorization"))?;

    let token = raw
        .strip_prefix("Bearer ")
        .ok_or_else(|| WalletError::new(WalletErrorCode::Unauthorized, "expected Bearer token"))?;

    if token.trim().is_empty() {
        return Err(WalletError::new(
            WalletErrorCode::Unauthorized,
            "empty bearer token",
        ));
    }

    Ok(token.to_string())
}

/// Extract a bounded idempotency key header, preferring the standard header.
pub fn idempotency_header(headers: &HeaderMap) -> Option<String> {
    use crate::util::headers::{IDEMPOTENCY_KEY, X_IDEMPOTENCY_KEY};

    headers
        .get(IDEMPOTENCY_KEY)
        .or_else(|| headers.get(X_IDEMPOTENCY_KEY))
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 64)
        .map(ToOwned::to_owned)
}

/// Current unix time in milliseconds.
pub fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    u64::try_from(millis).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_state_builds_router() {
        let state = WalletState::dev().unwrap();
        let _router = router(state);
    }

    #[test]
    fn receipt_book_roundtrip() {
        use crate::dto::{
            requests::AmountMinor,
            responses::{Receipt, ReceiptSettlementStatus, WalletOp},
        };

        let state = WalletState::dev().unwrap();
        let receipt = Receipt {
            txid: "tx_test".into(),
            op: WalletOp::Issue,
            from: None,
            to: Some("acct".into()),
            asset: "roc".into(),
            amount_minor: AmountMinor(1),
            nonce: None,
            idem: "idem".into(),
            ts: 1,
            ledger_seq_start: Some(1),
            ledger_seq_end: Some(1),
            ledger_root: "00".repeat(32),
            settlement_status: ReceiptSettlementStatus::Accepted,
            receipt_hash: "b3:test".into(),
        };
        state.remember_receipt(receipt.clone());
        assert_eq!(state.receipt("tx_test"), Some(receipt));
    }
}
