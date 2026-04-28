//! RO:WHAT — POST /v1/hold, /v1/capture, and /v1/release handlers.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES. Paid operations must reserve, capture, and refund through wallet.
//! RO:INTERACTS — WalletState, auth caps, policy, nonce table, idempotency store, ron-ledger escrow adapter.
//! RO:INVARIANTS — idempotent replay wins before nonce; strict debit nonce; failed commit rolls nonce back; ledger stays truth.
//! RO:METRICS — increments wallet_ops_total{op="hold|capture|release"}, replay, success, and reject counters.
//! RO:CONFIG — validates asset and amount ceilings.
//! RO:SECURITY — bearer token is verified and discarded; no memo/account leakage in metrics.
//! RO:TEST — tests/http_escrow.rs covers route flow, replay, and over-capture rejection.

use axum::{extract::State, http::HeaderMap, Json};

use crate::{
    accounting::client::AccountingEvent,
    auth::caps::WalletScope,
    dto::{
        requests::{resolve_idempotency_key, TransferRequest},
        responses::{Receipt, WalletOp},
    },
    errors::WalletError,
    policy::enforce::{enforce_local_policy, PolicyAction, PolicyContext},
    routes::{
        bearer_from_headers, corr_id_from_headers, idempotency_header, now_millis, HttpError,
        WalletState,
    },
    util::blake3_receipt::request_fingerprint,
};

/// POST /v1/hold.
///
/// Request shape currently reuses `TransferRequest`:
///
/// ```json
/// {
///   "from": "acct_user",
///   "to": "escrow_hold_1",
///   "asset": "roc",
///   "amount_minor": "70",
///   "nonce": 1,
///   "memo": "storage hold"
/// }
/// ```
pub async fn hold(
    State(state): State<WalletState>,
    headers: HeaderMap,
    Json(request): Json<TransferRequest>,
) -> Result<Json<Receipt>, HttpError> {
    escrow_move(state, headers, request, WalletOp::Hold).await
}

/// POST /v1/capture.
///
/// Request shape currently reuses `TransferRequest`:
///
/// ```json
/// {
///   "from": "escrow_hold_1",
///   "to": "svc_storage",
///   "asset": "roc",
///   "amount_minor": "40",
///   "nonce": 1,
///   "memo": "storage capture"
/// }
/// ```
pub async fn capture(
    State(state): State<WalletState>,
    headers: HeaderMap,
    Json(request): Json<TransferRequest>,
) -> Result<Json<Receipt>, HttpError> {
    escrow_move(state, headers, request, WalletOp::Capture).await
}

/// POST /v1/release.
///
/// Request shape currently reuses `TransferRequest`:
///
/// ```json
/// {
///   "from": "escrow_hold_1",
///   "to": "acct_user",
///   "asset": "roc",
///   "amount_minor": "30",
///   "nonce": 2,
///   "memo": "storage release"
/// }
/// ```
pub async fn release(
    State(state): State<WalletState>,
    headers: HeaderMap,
    Json(request): Json<TransferRequest>,
) -> Result<Json<Receipt>, HttpError> {
    escrow_move(state, headers, request, WalletOp::Release).await
}

async fn escrow_move(
    state: WalletState,
    headers: HeaderMap,
    request: TransferRequest,
    op: WalletOp,
) -> Result<Json<Receipt>, HttpError> {
    let _guard = state.metrics.begin_request();
    let corr_id = corr_id_from_headers(&headers);

    let token = bearer_from_headers(&headers).map_err(|err| state.reject(err, corr_id.clone()))?;
    let claims = state
        .cap_verifier
        .verify(&token)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    // Until dedicated capability scopes are minted, escrow mutations require the
    // same write scope as transfer. This keeps the v1 dev path compatible with
    // existing StaticCapabilityVerifier claims while still denying ambient access.
    claims
        .require_scope(WalletScope::Transfer)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    request
        .validate(&state.config)
        .map_err(|err| state.reject(err, corr_id.clone()))?;

    let ctx = PolicyContext {
        action: PolicyAction::Transfer,
        asset: &request.asset,
        from: Some(&request.from),
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
    let fingerprint =
        request_fingerprint(op, &request).map_err(|err| state.reject(err, corr_id.clone()))?;

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

    let receipt_result = match op {
        WalletOp::Hold => state.ledger.hold(&state.config, &request, &idem),
        WalletOp::Capture => state.ledger.capture(&state.config, &request, &idem),
        WalletOp::Release => state.ledger.release(&state.config, &request, &idem),
        _ => Err(WalletError::bad_request(
            "unsupported escrow operation for escrow route",
        )),
    };

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
        op: op.as_str(),
        asset: receipt.asset.clone(),
        amount_minor: receipt.amount_minor.get(),
    });
    state.metrics.inc_op(op);
    state.metrics.inc_success();

    Ok(Json(receipt))
}
