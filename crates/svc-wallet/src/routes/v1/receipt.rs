//! RO:WHAT — GET /v1/tx/{txid} handler.
//! RO:WHY  — Pillar 12; Concerns: ECON/DX/GOV. Receipts are the client-visible proof surface.
//! RO:INTERACTS — WalletState local receipt book; future ron-ledger receipt lookup.
//! RO:INVARIANTS — read capability required; local receipt book is a cache, not durable truth.
//! RO:METRICS — increments request/success/reject counters.
//! RO:CONFIG — none.
//! RO:SECURITY — bearer token is verified and discarded.
//! RO:TEST — integration contract will cover known/unknown txid.

use axum::{extract::Path, extract::State, http::HeaderMap, Json};

use crate::{
    auth::caps::WalletScope,
    dto::responses::Receipt,
    errors::{WalletError, WalletErrorCode},
    routes::{bearer_from_headers, corr_id_from_headers, HttpError, WalletState},
};

/// GET /v1/tx/{txid}.
pub async fn receipt(
    State(state): State<WalletState>,
    headers: HeaderMap,
    Path(txid): Path<String>,
) -> Result<Json<Receipt>, HttpError> {
    let _guard = state.metrics.begin_request();
    let corr_id = corr_id_from_headers(&headers);

    let token = bearer_from_headers(&headers).map_err(|err| state.reject(err, corr_id.clone()))?;
    let claims = state
        .cap_verifier
        .verify(&token)
        .map_err(|err| state.reject(err, corr_id.clone()))?;
    claims
        .require_scope(WalletScope::Read)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    if txid.is_empty() || txid.len() > 80 || !txid.chars().all(|c| c.is_ascii_graphic()) {
        return Err(state.reject(WalletError::bad_request("invalid txid"), corr_id));
    }

    let Some(receipt) = state.receipt(&txid) else {
        return Err(state.reject(
            WalletError::new(WalletErrorCode::NotFound, "receipt not found"),
            corr_id,
        ));
    };

    state.metrics.inc_success();
    Ok(Json(receipt))
}
