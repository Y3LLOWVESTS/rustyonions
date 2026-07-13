//! RO:WHAT — Local reward-recipient operator endpoints for crabnode.
//! RO:WHY — Phase 5B needs real macronode endpoints behind `crabnode rewards *`.
//! RO:INTERACTS — AppState::operator, crabnode rewards CLI, future svc-registry/passport flow.
//! RO:INVARIANTS — runtime-local request/display state only; no wallet/ledger mutation.
//! RO:SECURITY — rejects malformed @ addresses; sensitive POSTs are guarded by admin middleware.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::types::{AppState, RewardRecipientSnapshot};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindRewardRecipientRequest {
    reward_recipient_display_address: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateRewardRecipientRequest {
    new_reward_recipient_display_address: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RewardRecipientStatusResponse {
    version: u8,
    service_node_id: &'static str,
    state: &'static str,
    reward_recipient_display_address: Option<String>,
    pending_rotation_display_address: Option<String>,
    updated_at_unix_s: Option<u64>,
    wallet_mutation: bool,
    ledger_mutation: bool,
    confirmed_roc: Option<u64>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RewardRecipientMutationResponse {
    status: &'static str,
    state: &'static str,
    reward_recipient_display_address: Option<String>,
    pending_rotation_display_address: Option<String>,
    wallet_mutation: bool,
    ledger_mutation: bool,
    confirmed_roc: Option<u64>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RewardRecipientErrorResponse {
    status: &'static str,
    error: String,
    wallet_mutation: bool,
    ledger_mutation: bool,
}

pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot = state.operator.reward_recipient_snapshot();

    Json(RewardRecipientStatusResponse {
        version: 1,
        service_node_id: "local_service_node",
        state: snapshot.state,
        reward_recipient_display_address: snapshot.reward_recipient_display_address,
        pending_rotation_display_address: snapshot.pending_rotation_display_address,
        updated_at_unix_s: snapshot.updated_at_unix_s,
        wallet_mutation: false,
        ledger_mutation: false,
        confirmed_roc: None,
        note: "Runtime-local operator request state only; registry, wallet, ledger, and confirmed ROC remain separate.",
    })
}

pub async fn bind(
    State(state): State<AppState>,
    Json(req): Json<BindRewardRecipientRequest>,
) -> impl IntoResponse {
    let address = req.reward_recipient_display_address.trim();

    if let Err(error) = validate_crablink_address(address) {
        return (
            StatusCode::BAD_REQUEST,
            Json(RewardRecipientErrorResponse {
                status: "reward recipient rejected",
                error,
                wallet_mutation: false,
                ledger_mutation: false,
            }),
        )
            .into_response();
    }

    let snapshot = state.operator.bind_reward_recipient(address.to_string());

    let _ = req.note;

    (
        StatusCode::ACCEPTED,
        Json(response_from_snapshot(
            "binding request recorded",
            snapshot,
            "Request recorded locally for operator flow; no wallet/ledger mutation and no confirmed ROC.",
        )),
    )
        .into_response()
}

pub async fn rotate(
    State(state): State<AppState>,
    Json(req): Json<RotateRewardRecipientRequest>,
) -> impl IntoResponse {
    let address = req.new_reward_recipient_display_address.trim();

    if let Err(error) = validate_crablink_address(address) {
        return (
            StatusCode::BAD_REQUEST,
            Json(RewardRecipientErrorResponse {
                status: "reward recipient rotation rejected",
                error,
                wallet_mutation: false,
                ledger_mutation: false,
            }),
        )
            .into_response();
    }

    let current = state.operator.reward_recipient_snapshot();
    if current.reward_recipient_display_address.is_none() {
        return (
            StatusCode::CONFLICT,
            Json(RewardRecipientErrorResponse {
                status: "reward recipient rotation rejected",
                error: "cannot rotate before a reward recipient is bound".to_string(),
                wallet_mutation: false,
                ledger_mutation: false,
            }),
        )
            .into_response();
    }

    let snapshot = state
        .operator
        .request_reward_recipient_rotation(address.to_string());

    let _ = req.note;

    (
        StatusCode::ACCEPTED,
        Json(response_from_snapshot(
            "rotation request recorded",
            snapshot,
            "Rotation request recorded locally; effective-epoch registry acceptance is future work.",
        )),
    )
        .into_response()
}

fn response_from_snapshot(
    status: &'static str,
    snapshot: RewardRecipientSnapshot,
    note: &'static str,
) -> RewardRecipientMutationResponse {
    RewardRecipientMutationResponse {
        status,
        state: snapshot.state,
        reward_recipient_display_address: snapshot.reward_recipient_display_address,
        pending_rotation_display_address: snapshot.pending_rotation_display_address,
        wallet_mutation: false,
        ledger_mutation: false,
        confirmed_roc: None,
        note,
    }
}

fn validate_crablink_address(address: &str) -> Result<(), String> {
    let Some(username) = address.strip_prefix('@') else {
        return Err("reward recipient must be a CrabLink/RON @ address like @operator".to_string());
    };

    let bytes = username.as_bytes();
    if !(3..=32).contains(&bytes.len()) {
        return Err("reward recipient username must be 3..=32 characters after @".to_string());
    }

    if !bytes[0].is_ascii_alphanumeric() {
        return Err(
            "reward recipient username must start with an ASCII letter or digit".to_string(),
        );
    }

    if matches!(bytes[bytes.len() - 1], b'.' | b'-' | b'_') {
        return Err("reward recipient username must not end with '.', '-', or '_'".to_string());
    }

    let mut previous_dot = false;
    for byte in bytes {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(*byte, b'_' | b'-' | b'.');

        if !valid {
            return Err(
                "reward recipient username must use lowercase ASCII letters, digits, '.', '-', or '_'"
                    .to_string(),
            );
        }

        if previous_dot && *byte == b'.' {
            return Err("reward recipient username must not contain consecutive dots".to_string());
        }

        previous_dot = *byte == b'.';
    }

    Ok(())
}
