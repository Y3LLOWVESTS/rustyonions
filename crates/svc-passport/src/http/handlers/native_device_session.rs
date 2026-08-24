//! RO:WHAT — Fixed-purpose CN-4 HTTP adapter for Native Passport device-session challenge and possession proof.
//! RO:WHY — An already root-authorized device must prove control of its registered DeviceKey before capability issuance or username/profile mutation.
//! RO:INTERACTS — `server_device_session_runtime`, injected `KmsClient`, `PassportChallengeV1`, canonical Passport/Device/scope/signature DTOs, and the constrained CrabNode svc-passport router.
//! RO:INVARIANTS — `/v1/passport/challenge` always means `ProveSession`; caller cannot select purpose/trusted context/time/service KID; `/v1/passport/prove` accepts only a complete service challenge plus device signature; successful proof consumes once and grants no capability itself.
//! RO:METRICS — none yet; later composition may add bounded outcome counters without identity labels.
//! RO:CONFIG — durable roots/trusted context/KMS come only from reviewed CrabNode composition; both request bodies are capped by the router.
//! RO:SECURITY — no RecoveryRoot, PIN, device private key, vault bytes, capability material, username mutation, wallet mutation, ledger mutation, or raw internal error text crosses HTTP.
//! RO:TEST — focused CN-4 route acceptance follows this compile slice.

#![forbid(unsafe_code)]

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{extract::rejection::JsonRejection, http::StatusCode, Extension, Json};
use ron_proto::{
    DeviceIdV1, Ed25519SignatureV1, NativePassportScopeV1, PassportChallengeV1, PassportIdV1,
};
use serde::{Deserialize, Serialize};

use crate::{
    kms::client::KmsClient,
    native::{
        issue_device_session_challenge_service, submit_device_session_proof_service,
        NativePassportDeviceSessionServiceError, NativePassportServerRuntimeMountConfigV1,
    },
};

const PROBLEM_SCHEMA: &str = "svc-passport.native-device-session-problem.v1";

const PROOF_RESULT_SCHEMA: &str = "svc-passport.native-device-session-proof-result.v1";

pub(crate) struct NativeDeviceSessionHttpState {
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
}

impl NativeDeviceSessionHttpState {
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

/// Caller-controlled bindings for one fixed `ProveSession` challenge.
///
/// Purpose, network, environment, audience, service identity, nonce, and time
/// are intentionally absent and remain svc-passport authority.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDeviceSessionChallengeRequestV1 {
    pub passport_id: PassportIdV1,
    pub device_id: DeviceIdV1,
    pub requested_scopes: Vec<NativePassportScopeV1>,
}

/// Caller-controlled evidence for one fixed device-session proof.
///
/// Device public key is deliberately absent. Verification always reads the
/// authorized public key from durable svc-passport registry state.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDeviceSessionProofRequestV1 {
    pub challenge: PassportChallengeV1,
    pub proof_created_at_ms: u64,
    pub proof_signature: Ed25519SignatureV1,
}

#[derive(Debug, Serialize)]
pub struct NativeDeviceSessionProofResultV1 {
    pub schema: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct NativeDeviceSessionProblemV1 {
    pub schema: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub retryable: bool,
}

/// `POST /v1/passport/challenge`.
///
/// Despite the historical compact path, this endpoint is not generic:
/// svc-passport constructs only `PassportChallengePurposeV1::ProveSession`.
pub(crate) async fn issue_device_session_challenge(
    Extension(state): Extension<Arc<NativeDeviceSessionHttpState>>,
    body: Result<Json<NativeDeviceSessionChallengeRequestV1>, JsonRejection>,
) -> Result<Json<PassportChallengeV1>, (StatusCode, Json<NativeDeviceSessionProblemV1>)> {
    let Json(request) = body.map_err(problem_for_json_rejection)?;

    let now_ms = trusted_now_ms()?;

    let challenge = issue_device_session_challenge_service(
        &state.runtime_config,
        Arc::clone(&state.kms),
        request.passport_id,
        request.device_id,
        request.requested_scopes,
        now_ms,
    )
    .await
    .map_err(problem_for_service_error)?;

    Ok(Json(challenge))
}

/// `POST /v1/passport/prove`.
///
/// The runtime accepts only a valid, live `ProveSession` challenge and the
/// registered device's canonical Ed25519 possession signature.
pub(crate) async fn submit_device_session_proof(
    Extension(state): Extension<Arc<NativeDeviceSessionHttpState>>,
    body: Result<Json<NativeDeviceSessionProofRequestV1>, JsonRejection>,
) -> Result<Json<NativeDeviceSessionProofResultV1>, (StatusCode, Json<NativeDeviceSessionProblemV1>)>
{
    let Json(request) = body.map_err(problem_for_json_rejection)?;

    let accepted_at_ms = trusted_now_ms()?;

    submit_device_session_proof_service(
        &state.runtime_config,
        Arc::clone(&state.kms),
        request.challenge,
        request.proof_created_at_ms,
        request.proof_signature,
        accepted_at_ms,
    )
    .await
    .map_err(problem_for_service_error)?;

    Ok(Json(NativeDeviceSessionProofResultV1 {
        schema: PROOF_RESULT_SCHEMA,
        status: "proven",
    }))
}

fn trusted_now_ms() -> Result<u64, (StatusCode, Json<NativeDeviceSessionProblemV1>)> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "Trusted service time is unavailable",
            true,
        )
    })?;

    let millis = u64::try_from(duration.as_millis()).map_err(|_| {
        problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "Trusted service time is unavailable",
            true,
        )
    })?;

    if millis == 0 {
        return Err(problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "Trusted service time is unavailable",
            true,
        ));
    }

    Ok(millis)
}

fn problem_for_json_rejection(
    rejection: JsonRejection,
) -> (StatusCode, Json<NativeDeviceSessionProblemV1>) {
    if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Device session request exceeds the maximum body size",
            false,
        );
    }

    problem(
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "Device session request is invalid",
        false,
    )
}

fn problem_for_service_error(
    error: NativePassportDeviceSessionServiceError,
) -> (StatusCode, Json<NativeDeviceSessionProblemV1>) {
    match error {
        NativePassportDeviceSessionServiceError::InvalidRequest => problem(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Device session request is invalid",
            false,
        ),

        NativePassportDeviceSessionServiceError::NotFound => problem(
            StatusCode::NOT_FOUND,
            "device_not_registered",
            "Registered device was not found",
            false,
        ),

        NativePassportDeviceSessionServiceError::Forbidden => problem(
            StatusCode::FORBIDDEN,
            "device_session_rejected",
            "Device session proof is not authorized",
            false,
        ),

        NativePassportDeviceSessionServiceError::ChallengeNotConsumable => problem(
            StatusCode::CONFLICT,
            "challenge_not_consumable",
            "Device session challenge is no longer consumable",
            false,
        ),

        NativePassportDeviceSessionServiceError::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "device_session_service_unavailable",
            "Native Passport device session service is unavailable",
            true,
        ),
    }
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
) -> (StatusCode, Json<NativeDeviceSessionProblemV1>) {
    (
        status,
        Json(NativeDeviceSessionProblemV1 {
            schema: PROBLEM_SCHEMA,
            code,
            message,
            retryable,
        }),
    )
}
