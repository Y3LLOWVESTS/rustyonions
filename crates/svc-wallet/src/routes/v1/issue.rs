//! RO:WHAT — POST /v1/issue handler.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES. Minting is a supply exception and must be cap-gated and idempotent.
//! RO:INTERACTS — WalletState, auth caps, policy, idempotency store, ledger adapter, accounting seam.
//! RO:INVARIANTS — issue requires issue capability; idempotent replay returns byte-identical receipt; no floats.
//! RO:METRICS — increments wallet_ops_total{op="issue"}, replay, success, and reject counters.
//! RO:CONFIG — validates asset and amount ceilings.
//! RO:SECURITY — bearer token is not stored; receipt cache stores identifiers only.
//! RO:TEST — integration contract will cover idempotent issue replay.

use axum::{extract::State, http::HeaderMap, Json};

use crate::{
    accounting::client::AccountingEvent,
    auth::caps::WalletScope,
    dto::{
        requests::{resolve_idempotency_key, IssueRequest},
        responses::{Receipt, WalletOp},
    },
    policy::enforce::{enforce_local_policy, PolicyAction, PolicyContext},
    routes::{
        bearer_from_headers, corr_id_from_headers, idempotency_header, now_millis, HttpError,
        WalletState,
    },
    util::blake3_receipt::request_fingerprint,
};

/// POST /v1/issue.
pub async fn issue(
    State(state): State<WalletState>,
    headers: HeaderMap,
    Json(request): Json<IssueRequest>,
) -> Result<Json<Receipt>, HttpError> {
    let _guard = state.metrics.begin_request();
    let corr_id = corr_id_from_headers(&headers);

    let token = bearer_from_headers(&headers).map_err(|err| state.reject(err, corr_id.clone()))?;
    let claims = state
        .cap_verifier
        .verify(&token)
        .map_err(|err| state.reject(err, corr_id.clone()))?;
    claims
        .require_scope(WalletScope::Issue)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    request
        .validate(&state.config)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let ctx = PolicyContext {
        action: PolicyAction::Issue,
        asset: &request.asset,
        from: None,
        to: Some(&request.to),
        amount: Some(request.amount_minor),
    };
    enforce_local_policy(&state.config, &claims, &ctx)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let idem = resolve_idempotency_key(
        idempotency_header(&headers).as_deref(),
        request.idempotency_key.as_deref(),
    )
    .map_err(|err| state.reject(err, corr_id.clone()))?;
    let fingerprint = request_fingerprint(WalletOp::Issue, &request)
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

    let receipt = state
        .ledger
        .issue(&state.config, &request, &idem)
        .map_err(|err| state.reject(err, corr_id))?;

    state.idem.insert(idem, fingerprint, receipt.clone(), now);
    state.remember_receipt(receipt.clone());
    state.accounting.record(AccountingEvent {
        op: WalletOp::Issue.as_str(),
        asset: receipt.asset.clone(),
        amount_minor: receipt.amount_minor.get(),
    });
    state.metrics.inc_op(WalletOp::Issue);
    state.metrics.inc_success();

    Ok(Json(receipt))
}
