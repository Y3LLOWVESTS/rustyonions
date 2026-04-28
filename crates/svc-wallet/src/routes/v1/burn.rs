//! RO:WHAT — POST /v1/burn handler.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES. Burning is a supply exception and must be cap-gated and idempotent.
//! RO:INTERACTS — WalletState, auth caps, policy, nonce table, idempotency store, ron-ledger adapter.
//! RO:INVARIANTS — burn requires burn capability; strict nonce; rollback reservation on failed ledger commit.
//! RO:METRICS — increments wallet_ops_total{op="burn"}, replay, success, and reject counters.
//! RO:CONFIG — validates asset and amount ceilings.
//! RO:SECURITY — bearer token is verified and discarded; no secret logging.
//! RO:TEST — integration contract will cover burn replay and insufficient funds.

use axum::{extract::State, http::HeaderMap, Json};

use crate::{
    accounting::client::AccountingEvent,
    auth::caps::WalletScope,
    dto::{
        requests::{resolve_idempotency_key, BurnRequest},
        responses::{Receipt, WalletOp},
    },
    policy::enforce::{enforce_local_policy, PolicyAction, PolicyContext},
    routes::{
        bearer_from_headers, corr_id_from_headers, idempotency_header, now_millis, HttpError,
        WalletState,
    },
    util::blake3_receipt::request_fingerprint,
};

/// POST /v1/burn.
pub async fn burn(
    State(state): State<WalletState>,
    headers: HeaderMap,
    Json(request): Json<BurnRequest>,
) -> Result<Json<Receipt>, HttpError> {
    let _guard = state.metrics.begin_request();
    let corr_id = corr_id_from_headers(&headers);

    let token = bearer_from_headers(&headers).map_err(|err| state.reject(err, corr_id.clone()))?;
    let claims = state
        .cap_verifier
        .verify(&token)
        .map_err(|err| state.reject(err, corr_id.clone()))?;
    claims
        .require_scope(WalletScope::Burn)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    request
        .validate(&state.config)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let ctx = PolicyContext {
        action: PolicyAction::Burn,
        asset: &request.asset,
        from: Some(&request.from),
        to: None,
        amount: Some(request.amount_minor),
    };
    enforce_local_policy(&state.config, &claims, &ctx)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let idem = resolve_idempotency_key(
        idempotency_header(&headers).as_deref(),
        request.idempotency_key.as_deref(),
    )
    .map_err(|err| state.reject(err, corr_id.clone()))?;
    let fingerprint = request_fingerprint(WalletOp::Burn, &request)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let now = now_millis();
    if let Some(receipt) = state
        .idem
        .lookup(&idem, &fingerprint, now)
        .map_err(|err| state.reject(err, corr_id.clone()))?
    {
        state.metrics.inc_idempotency_replay();
        state.metrics.inc_success();
        return Ok(Json(receipt));
    }

    let reservation = state
        .nonces
        .reserve_strict(&request.from, request.nonce)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let receipt_result = state.ledger.burn(&state.config, &request, &idem);
    let receipt = match receipt_result {
        Ok(receipt) => {
            reservation.commit();
            receipt
        }
        Err(err) => {
            state.nonces.rollback(reservation);
            return Err(state.reject(err, corr_id));
        }
    };

    state.idem.insert(idem, fingerprint, receipt.clone(), now);
    state.remember_receipt(receipt.clone());
    state.accounting.record(AccountingEvent {
        op: WalletOp::Burn.as_str(),
        asset: receipt.asset.clone(),
        amount_minor: receipt.amount_minor.get(),
    });
    state.metrics.inc_op(WalletOp::Burn);
    state.metrics.inc_success();

    Ok(Json(receipt))
}
