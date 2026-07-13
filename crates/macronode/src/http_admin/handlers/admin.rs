//! RO:WHAT — Local service-node admin-control handlers for crabnode.
//! RO:WHY — BUILD_PLAN_Z Phase 4A needs real loopback controls before svc-admin UI bootstrap.
//! RO:INTERACTS — AppState::operator, crabnode admin commands, /api/v1/status.
//! RO:INVARIANTS — no fake user creation; setup tokens are runtime-local, short-lived, one-use.
//! RO:SECURITY — intended for loopback admin plane; token bytes come from /dev/urandom.
//! RO:TEST — integration: crates/macronode/tests/operator_admin.rs.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::types::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AdminUiResponse {
    status: &'static str,
    admin_ui_enabled: bool,
    admin_ui_bind: String,
    admin_ui_runtime_required: bool,
    operator_ui_profile: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SetupTokenResponse {
    status: &'static str,
    token: String,
    setup_url: String,
    expires_at_unix_s: u64,
    expires_in_seconds: u64,
    note: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ErrorResponse {
    status: &'static str,
    error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConsumeSetupTokenRequest {
    token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ConsumeSetupTokenResponse {
    status: &'static str,
    consumed: bool,
    note: &'static str,
}

pub async fn enable_web(State(state): State<AppState>) -> impl IntoResponse {
    state.operator.set_admin_ui_enabled(true);
    Json(ui_response(&state, "admin UI enabled"))
}

pub async fn disable_web(State(state): State<AppState>) -> impl IntoResponse {
    state.operator.set_admin_ui_enabled(false);
    Json(ui_response(&state, "admin UI disabled"))
}

pub async fn setup_token(State(state): State<AppState>) -> impl IntoResponse {
    match state.operator.issue_setup_token() {
        Ok(snapshot) => (
            StatusCode::OK,
            Json(SetupTokenResponse {
                status: "setup token issued",
                token: snapshot.token,
                setup_url: snapshot.setup_url,
                expires_at_unix_s: snapshot.expires_at_unix_s,
                expires_in_seconds: snapshot.expires_in_seconds,
                note: "Phase 4A token is runtime-local and one-use; durable admin user creation is the next svc-admin/RBAC slice.",
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                status: "setup token unavailable",
                error: err.to_string(),
            }),
        )
            .into_response(),
    }
}

pub async fn consume_setup_token(
    State(state): State<AppState>,
    Json(req): Json<ConsumeSetupTokenRequest>,
) -> impl IntoResponse {
    let consumed = state.operator.consume_setup_token(&req.token);

    let status = if consumed {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    };

    (
        status,
        Json(ConsumeSetupTokenResponse {
            status: if consumed {
                "setup token consumed"
            } else {
                "setup token rejected"
            },
            consumed,
            note: "Token consumption burns the runtime token. Durable admin creation remains a later svc-admin/RBAC step.",
        }),
    )
}

fn ui_response(state: &AppState, status: &'static str) -> AdminUiResponse {
    AdminUiResponse {
        status,
        admin_ui_enabled: state.operator.admin_ui_enabled(),
        admin_ui_bind: state.cfg.admin_ui_bind.to_string(),
        admin_ui_runtime_required: state.cfg.admin_ui_runtime_required,
        operator_ui_profile: state.cfg.operator_ui_profile.clone(),
    }
}
