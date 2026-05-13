//! RO:WHAT — HTTP handlers for NEXT_LEVEL public profile username claims.
//! RO:WHY — Exposes the green `svc_passport::profile` claim core without wallet/ledger/storage mutation.
//! RO:INTERACTS — profile::{UsernameClaimStore, UsernameClaimRequest}, router Extension state, future Omnigate proxy.
//! RO:INVARIANTS — no private keys; no spend authority; no wallet mutation; no public main↔alt linkage.
//! RO:METRICS — profile route metrics will be added at the router/observability layer later.
//! RO:CONFIG — none.
//! RO:SECURITY — fail closed on invalid username; duplicate claims return deterministic conflict.
//! RO:TEST — tests/profile_routes.rs.

use axum::{extract::Path, http::StatusCode, Extension, Json};
use serde::Serialize;
use serde_json::json;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::profile::{
    ProfileClaimError, PublicProfileResponse, UsernameClaimRequest, UsernameClaimStore,
};

/// Stable problem body for profile route failures.
#[derive(Debug, Serialize)]
pub struct ProfileProblem<'a> {
    /// Stable machine-readable code.
    pub code: &'a str,
    /// Human-readable safe message.
    pub message: &'a str,
    /// Whether retrying the same request may succeed.
    pub retryable: bool,
}

/// POST /v1/passport/profile/claim
///
/// Claims a main-passport username in the current in-memory store. This is a
/// Phase 3 local core route, not the final durable production persistence path.
pub async fn claim_profile(
    Extension(store): Extension<Arc<UsernameClaimStore>>,
    Json(request): Json<UsernameClaimRequest>,
) -> Result<(StatusCode, Json<PublicProfileResponse>), (StatusCode, Json<ProfileProblem<'static>>)>
{
    let now_ms = now_ms();

    let record = store
        .claim_main_username(request, now_ms)
        .map_err(problem_for_claim_error)?;

    Ok((
        StatusCode::CREATED,
        Json(PublicProfileResponse::from(&record)),
    ))
}

/// GET /v1/passport/profile/:username
///
/// Returns a read-only public profile if this process has a confirmed claim.
pub async fn get_profile(
    Extension(store): Extension<Arc<UsernameClaimStore>>,
    Path(username): Path<String>,
) -> Result<Json<PublicProfileResponse>, (StatusCode, Json<ProfileProblem<'static>>)> {
    let Some(profile) = store
        .public_profile(&username)
        .map_err(problem_for_claim_error)?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ProfileProblem {
                code: "profile_not_found",
                message: "public profile was not found",
                retryable: false,
            }),
        ));
    };

    Ok(Json(profile))
}

/// GET /v1/passport/profile/_debug
///
/// Tiny debug endpoint for local green gates. It intentionally exposes no
/// private state and no list of claimed usernames.
pub async fn profile_debug() -> Json<serde_json::Value> {
    Json(json!({
        "schema": "svc-passport.profile-debug.v1",
        "profile_routes": [
            "POST /v1/passport/profile/claim",
            "GET /v1/passport/profile/:username"
        ],
        "wallet_mutation": false,
        "ledger_mutation": false,
        "private_keys": false,
        "alt_linkage": false
    }))
}

fn problem_for_claim_error(err: ProfileClaimError) -> (StatusCode, Json<ProfileProblem<'static>>) {
    let status = status_for_claim_error(&err);
    let code = err.code();
    let message = message_for_claim_error(&err);
    let retryable = matches!(err, ProfileClaimError::StorePoisoned);

    (
        status,
        Json(ProfileProblem {
            code,
            message,
            retryable,
        }),
    )
}

fn status_for_claim_error(err: &ProfileClaimError) -> StatusCode {
    match err {
        ProfileClaimError::UsernameUnavailable { .. }
        | ProfileClaimError::PassportAlreadyHasUsername { .. } => StatusCode::CONFLICT,

        ProfileClaimError::StorePoisoned | ProfileClaimError::StoreCorrupt { .. } => {
            StatusCode::INTERNAL_SERVER_ERROR
        }

        ProfileClaimError::EmptyField { .. }
        | ProfileClaimError::FieldTooLong { .. }
        | ProfileClaimError::UsernameTooShort { .. }
        | ProfileClaimError::UsernameTooLong { .. }
        | ProfileClaimError::InvalidUsernameStart
        | ProfileClaimError::InvalidUsernameCharacter
        | ProfileClaimError::ConsecutiveDots
        | ProfileClaimError::InvalidUsernameTrailingPunctuation
        | ProfileClaimError::ReservedUsername { .. }
        | ProfileClaimError::InvalidCrabUrl { .. }
        | ProfileClaimError::InvalidTimestamp { .. } => StatusCode::BAD_REQUEST,
    }
}

fn message_for_claim_error(err: &ProfileClaimError) -> &'static str {
    match err {
        ProfileClaimError::EmptyField { .. } => "a required field was empty",
        ProfileClaimError::FieldTooLong { .. } => "a field exceeded its maximum length",
        ProfileClaimError::UsernameTooShort { .. } => "username is too short",
        ProfileClaimError::UsernameTooLong { .. } => "username is too long",
        ProfileClaimError::InvalidUsernameStart => "username must start with a letter or digit",
        ProfileClaimError::InvalidUsernameCharacter => "username contains an invalid character",
        ProfileClaimError::ConsecutiveDots => "username contains consecutive dots",
        ProfileClaimError::InvalidUsernameTrailingPunctuation => {
            "username has invalid trailing punctuation"
        }
        ProfileClaimError::ReservedUsername { .. } => "username is reserved",
        ProfileClaimError::UsernameUnavailable { .. } => "username is unavailable",
        ProfileClaimError::PassportAlreadyHasUsername { .. } => "passport already has a username",
        ProfileClaimError::InvalidCrabUrl { .. } => "public URL must use crab://",
        ProfileClaimError::InvalidTimestamp { .. } => "invalid timestamp",
        ProfileClaimError::StorePoisoned => "profile claim store is unavailable",
        ProfileClaimError::StoreCorrupt { .. } => "profile claim store is corrupt",
    }
}

fn now_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    u64::try_from(millis).unwrap_or(u64::MAX)
}
