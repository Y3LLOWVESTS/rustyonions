//! RO:WHAT — Loopback User Node object-verification HTTP boundary.
//! RO:WHY — Allows CrabLink to hand already-fetched bytes to the real micronode verifier.
//! RO:INTERACTS — AppState verification queue and canonical ron-proto evidence.
//! RO:INVARIANTS — bounded request/read; replay rejected; pending evidence only.
//! RO:SECURITY — local bind is enforced by micronode config; no IP fields or economic authority.
//! RO:TEST — tests/object_verification.rs.

#![forbid(unsafe_code)]

use crate::{
    state::AppState,
    verification::{
        ObjectVerificationQueueError, ObjectVerificationRequest,
        MAX_PENDING_VERIFICATION_READ_ITEMS,
    },
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingVerificationQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerificationErrorResponse {
    schema: &'static str,
    error: &'static str,
    message: String,

    duplicate_rejected: bool,
    queue_mutation: bool,

    accounting_accepted: bool,
    reward_eligible: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,

    confirmed_roc_minor_units: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerificationControlResponse {
    schema: &'static str,
    status: &'static str,
    changed: bool,

    lifecycle_state: &'static str,
    verification_queue_status: &'static str,
    pending_items: usize,

    read_only_status_available: bool,
    new_verification_writes_enabled: bool,
    queue_mutation: bool,

    accounting_accepted: bool,
    reward_eligible: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,

    confirmed_roc_minor_units: Option<String>,
}

pub async fn pause(State(state): State<AppState>) -> Response {
    let changed = state.set_verification_paused(true);

    Json(control_response(&state, "verification paused", changed)).into_response()
}

pub async fn resume(State(state): State<AppState>) -> Response {
    let changed = state.set_verification_paused(false);

    Json(control_response(&state, "verification resumed", changed)).into_response()
}

pub async fn verify_object(
    State(state): State<AppState>,
    Json(request): Json<ObjectVerificationRequest>,
) -> Response {
    if !state.cfg.user_node.verification_queue_enabled {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "verification_disabled",
            "User Node verification queue is disabled",
            false,
        );
    }

    if state.verification_paused() {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "verification_paused",
            "User Node verification is paused; new evidence writes fail closed",
            false,
        );
    }

    match state.verification.review_and_enqueue(request) {
        Ok(result) => (StatusCode::ACCEPTED, Json(result)).into_response(),

        Err(ObjectVerificationQueueError::DuplicateIdempotencyKey) => error_response(
            StatusCode::CONFLICT,
            "duplicate_idempotency_key",
            "verification replay was rejected",
            true,
        ),

        Err(ObjectVerificationQueueError::QueueFull) => error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "pending_queue_full",
            "pending verification queue is full",
            false,
        ),

        Err(ObjectVerificationQueueError::InvalidRequest(message)) => {
            error_response(StatusCode::BAD_REQUEST, "invalid_verification_request", &message, false)
        }

        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "verification_internal_error",
            &error.to_string(),
            false,
        ),
    }
}

pub async fn pending(
    State(state): State<AppState>,
    Query(query): Query<PendingVerificationQuery>,
) -> Response {
    if !state.cfg.user_node.verification_queue_enabled {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "verification_disabled",
            "User Node verification queue is disabled",
            false,
        );
    }

    let limit = query.limit.unwrap_or(32);

    if !(1..=MAX_PENDING_VERIFICATION_READ_ITEMS).contains(&limit) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_pending_limit",
            "pending limit is outside the bounded range",
            false,
        );
    }

    Json(state.verification.pending(limit)).into_response()
}

fn control_response(
    state: &AppState,
    status: &'static str,
    changed: bool,
) -> VerificationControlResponse {
    let paused = state.verification_paused();
    let queue_enabled = state.cfg.user_node.verification_queue_enabled;

    VerificationControlResponse {
        schema: "micronode.verification_control.v1",
        status,
        changed,

        lifecycle_state: if paused { "paused" } else { "active" },

        verification_queue_status: if !queue_enabled {
            "disabled"
        } else if paused {
            "paused"
        } else {
            "active"
        },

        pending_items: state.verification.pending_count(),

        read_only_status_available: true,
        new_verification_writes_enabled: queue_enabled && !paused,
        queue_mutation: false,

        accounting_accepted: false,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,

        confirmed_roc_minor_units: None,
    }
}

fn error_response(
    status: StatusCode,
    error: &'static str,
    message: &str,
    duplicate_rejected: bool,
) -> Response {
    (
        status,
        Json(VerificationErrorResponse {
            schema: "micronode.object_verification_error.v1",

            error,
            message: message.to_string(),

            duplicate_rejected,
            queue_mutation: false,

            accounting_accepted: false,
            reward_eligible: false,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,

            confirmed_roc_minor_units: None,
        }),
    )
        .into_response()
}
