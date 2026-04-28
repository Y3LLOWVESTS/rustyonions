//! RO:WHAT — GET /v1/balance handler.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/DX. Exposes read-only balances from ron-ledger via wallet policy.
//! RO:INTERACTS — WalletState, auth caps, policy, ledger adapter, BalanceQuery.
//! RO:INVARIANTS — read requires read capability; wallet does not own durable balance truth.
//! RO:METRICS — increments request/success/reject counters.
//! RO:CONFIG — validates asset/account against WalletConfig.
//! RO:SECURITY — bearer token is verified but never logged.
//! RO:TEST — integration contract will cover HTTP response shape.

use axum::{extract::Query, extract::State, http::HeaderMap, Json};

use crate::{
    auth::caps::WalletScope,
    dto::{requests::BalanceQuery, responses::BalanceResponse},
    policy::enforce::{enforce_local_policy, PolicyAction, PolicyContext},
    routes::{bearer_from_headers, corr_id_from_headers, HttpError, WalletState},
};

/// GET /v1/balance?account=...&asset=...
pub async fn balance(
    State(state): State<WalletState>,
    headers: HeaderMap,
    Query(query): Query<BalanceQuery>,
) -> Result<Json<BalanceResponse>, HttpError> {
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

    query
        .validate(&state.config)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let ctx = PolicyContext {
        action: PolicyAction::Read,
        asset: &query.asset,
        from: Some(&query.account),
        to: None,
        amount: None,
    };
    enforce_local_policy(&state.config, &claims, &ctx)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let response = state
        .ledger
        .balance(&state.config, &query.account)
        .map_err(|err| state.reject(err, corr_id))?;

    state.metrics.inc_success();
    Ok(Json(response))
}
