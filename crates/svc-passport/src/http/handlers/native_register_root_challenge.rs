//! RO:WHAT — Strict HTTP handler for the CN-4 Native Passport RegisterRoot challenge operation.
//! RO:WHY — Physical CrabLink enrollment needs one real server-signed, durable, purpose-bound challenge before root proof submission can be exposed.
//! RO:INTERACTS — `NativePassportServerRuntimeMountConfigV1`, durable challenge runtime, injected `KmsClient`, and canonical `ron-proto` Native Passport DTOs.
//! RO:INVARIANTS — route purpose is always RegisterRoot; trusted time/network/environment/audience/service KID come only from server composition; successful response was durably stored before return.
//! RO:METRICS — none yet; mounted service/gateway observability remains composition-owned.
//! RO:CONFIG — uses the already-reviewed CN-4 runtime mount config and body cap supplied by router composition.
//! RO:SECURITY — strict JSON; no caller-supplied trusted time, service KID, network, environment, device binding, secret, PIN, capability, username, wallet, or ledger authority.
//! RO:TEST — `crabnode_cn4_register_root_challenge_route.rs`.

#![forbid(unsafe_code)]

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{extract::rejection::JsonRejection, http::StatusCode, Extension, Json};
use ron_proto::{B3DigestHex, NativePassportScopeV1, PassportChallengeV1, PassportIdV1};
use serde::{Deserialize, Serialize};

use crate::{
    kms::client::KmsClient,
    native::{
        issue_register_root_challenge_durable, NativePassportServerRuntimeMountConfigV1,
        NativePassportServerRuntimeMountError,
    },
};

const PROBLEM_SCHEMA: &str = "svc-passport.native-register-root-challenge-problem.v1";

/// Dependencies retained by only the constrained RegisterRoot challenge route.
pub(crate) struct NativeRegisterRootChallengeHttpState {
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
}

impl NativeRegisterRootChallengeHttpState {
    pub(crate) fn new(
        kms: Arc<dyn KmsClient>,
        runtime_config: NativePassportServerRuntimeMountConfigV1,
    ) -> Self {
        Self {
            kms,
            runtime_config,
        }
    }
}

/// Caller-controlled fields for one RegisterRoot challenge.
///
/// Purpose is intentionally absent. This route is purpose-specific and always
/// constructs `PassportChallengePurposeV1::RegisterRoot` inside svc-passport.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRegisterRootChallengeRequestV1 {
    pub passport_id: PassportIdV1,
    pub requested_scopes: Vec<NativePassportScopeV1>,
    pub operation_body_hash: B3DigestHex,
}

#[derive(Debug, Serialize)]
pub struct NativeRegisterRootChallengeProblemV1 {
    pub schema: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub retryable: bool,
}

/// `POST /v1/passport/register/challenge`
pub(crate) async fn issue_register_root_challenge(
    Extension(state): Extension<Arc<NativeRegisterRootChallengeHttpState>>,
    body: Result<Json<NativeRegisterRootChallengeRequestV1>, JsonRejection>,
) -> Result<Json<PassportChallengeV1>, (StatusCode, Json<NativeRegisterRootChallengeProblemV1>)> {
    let Json(request) = body.map_err(|_| {
        problem(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "RegisterRoot challenge request is invalid",
            false,
        )
    })?;

    /*
     * The wall clock is process-owned trusted input. It is deliberately absent
     * from the HTTP DTO and therefore cannot be influenced by the webview or
     * any remote caller.
     */
    let now_ms = trusted_now_ms()?;

    let challenge = issue_register_root_challenge_durable(
        &state.runtime_config,
        Arc::clone(&state.kms),
        request.passport_id,
        request.requested_scopes,
        request.operation_body_hash,
        now_ms,
    )
    .await
    .map_err(problem_for_runtime_error)?;

    Ok(Json(challenge))
}

fn trusted_now_ms() -> Result<u64, (StatusCode, Json<NativeRegisterRootChallengeProblemV1>)> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "trusted service time is unavailable",
            true,
        )
    })?;

    let millis = u64::try_from(duration.as_millis()).map_err(|_| {
        problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "trusted service time is unavailable",
            true,
        )
    })?;

    if millis == 0 {
        return Err(problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "trusted service time is unavailable",
            true,
        ));
    }

    Ok(millis)
}

fn problem_for_runtime_error(
    error: NativePassportServerRuntimeMountError,
) -> (StatusCode, Json<NativeRegisterRootChallengeProblemV1>) {
    match error {
        NativePassportServerRuntimeMountError::InvalidRegisterRootChallengeRequest => problem(
            StatusCode::BAD_REQUEST,
            "invalid_register_root_challenge",
            "RegisterRoot challenge bindings are invalid",
            false,
        ),

        _ => {
            /*
             * KMS, durable-state, concurrency, capacity, and recovery failures
             * stay opaque to the caller. Internal error text may contain local
             * implementation detail and must not become an HTTP oracle.
             */
            problem(
                StatusCode::SERVICE_UNAVAILABLE,
                "challenge_service_unavailable",
                "Native Passport challenge service is unavailable",
                true,
            )
        }
    }
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
) -> (StatusCode, Json<NativeRegisterRootChallengeProblemV1>) {
    (
        status,
        Json(NativeRegisterRootChallengeProblemV1 {
            schema: PROBLEM_SCHEMA,
            code,
            message,
            retryable,
        }),
    )
}
