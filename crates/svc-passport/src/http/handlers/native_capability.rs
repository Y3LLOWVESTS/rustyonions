//! RO:WHAT — Strict HTTP adapter for the fixed CN-4 Native Passport IssueCapability challenge/proof pair.
//! RO:WHY — Expose the proven durable capability runtime through one bounded svc-passport seam before Omnigate/gateway and CrabLink integration.
//! RO:INTERACTS — capability service seam, injected `KmsClient`, trusted service clock, Passport/Device/challenge/signature/capability DTOs, and constrained router composition.
//! RO:INVARIANTS — challenge purpose is always IssueCapability; caller controls only Passport, Device, requested scopes, or concrete proof evidence; trusted context, KID, policy version, TTL, operation hash, capability ID, and acceptance time stay server-owned.
//! RO:METRICS — none yet; later service composition may add bounded result counters without identity-bearing labels.
//! RO:CONFIG — base Native Passport runtime plus dedicated capability snapshot/redo roots and service-owned TTL.
//! RO:SECURITY — strict JSON plus 16 KiB route cap; no DeviceKey/root secret/PIN/RecoveryRoot, caller-selected authority fields, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — `crabnode_cn4_capability_route.rs` plus the focused capability-runtime tests.

#![forbid(unsafe_code)]

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{extract::rejection::JsonRejection, http::StatusCode, Extension, Json};

use ron_proto::{
    DeviceIdV1, Ed25519SignatureV1, NativePassportDeviceBoundCapabilityV1, NativePassportScopeV1,
    PassportChallengeV1, PassportIdV1,
};

use serde::{Deserialize, Serialize};

use crate::{
    kms::client::KmsClient,
    native::{
        issue_capability_challenge_service, submit_capability_proof_service,
        NativePassportCapabilityServiceDispositionV1, NativePassportCapabilityServiceError,
        NativePassportServerCapabilityRuntimeConfigV1, NativePassportServerRuntimeMountConfigV1,
    },
};

pub const NATIVE_CAPABILITY_PROBLEM_SCHEMA_V1: &str = "svc-passport.native-capability-problem.v1";

pub const NATIVE_CAPABILITY_RESULT_SCHEMA_V1: &str = "svc-passport.native-capability-result.v1";

pub(crate) struct NativeCapabilityHttpState {
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
    capability_config: NativePassportServerCapabilityRuntimeConfigV1,
}

impl NativeCapabilityHttpState {
    pub(crate) fn new(
        kms: Arc<dyn KmsClient>,
        runtime_config: NativePassportServerRuntimeMountConfigV1,
        capability_config: NativePassportServerCapabilityRuntimeConfigV1,
    ) -> Self {
        Self {
            kms,
            runtime_config,
            capability_config,
        }
    }
}

/// Caller-controlled bindings for one fixed IssueCapability challenge.
///
/// Purpose, TTL, policy version, service KID, operation hash, trusted context,
/// and time are intentionally absent.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCapabilityChallengeRequestV1 {
    pub passport_id: PassportIdV1,
    pub device_id: DeviceIdV1,
    pub requested_scopes: Vec<NativePassportScopeV1>,
}

/// Caller-controlled proof evidence for one previously issued capability
/// challenge.
///
/// The device public key is absent because svc-passport must load it from the
/// durable root-signed DeviceAuthorization.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCapabilityProofRequestV1 {
    pub challenge: PassportChallengeV1,
    pub proof_created_at_ms: u64,
    pub proof_signature: Ed25519SignatureV1,
}

#[derive(Debug, Serialize)]
pub struct NativeCapabilityProofResultV1 {
    pub schema: &'static str,
    pub status: &'static str,
    pub capability: NativePassportDeviceBoundCapabilityV1,
    pub durable_generation: u64,
}

#[derive(Debug, Serialize)]
pub struct NativeCapabilityProblemV1 {
    pub schema: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub retryable: bool,
}

pub(crate) async fn issue_capability_challenge(
    Extension(state): Extension<Arc<NativeCapabilityHttpState>>,
    body: Result<Json<NativeCapabilityChallengeRequestV1>, JsonRejection>,
) -> Result<Json<PassportChallengeV1>, (StatusCode, Json<NativeCapabilityProblemV1>)> {
    let Json(request) = body.map_err(problem_for_json_rejection)?;

    let now_ms = trusted_now_ms()?;

    let challenge = issue_capability_challenge_service(
        &state.runtime_config,
        &state.capability_config,
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

pub(crate) async fn submit_capability_proof(
    Extension(state): Extension<Arc<NativeCapabilityHttpState>>,
    body: Result<Json<NativeCapabilityProofRequestV1>, JsonRejection>,
) -> Result<Json<NativeCapabilityProofResultV1>, (StatusCode, Json<NativeCapabilityProblemV1>)> {
    let Json(request) = body.map_err(problem_for_json_rejection)?;

    let accepted_at_ms = trusted_now_ms()?;

    let outcome = submit_capability_proof_service(
        &state.runtime_config,
        &state.capability_config,
        Arc::clone(&state.kms),
        request.challenge,
        request.proof_created_at_ms,
        request.proof_signature,
        accepted_at_ms,
    )
    .await
    .map_err(problem_for_service_error)?;

    let status = match outcome.disposition {
        NativePassportCapabilityServiceDispositionV1::Issued => "issued",

        NativePassportCapabilityServiceDispositionV1::AlreadyIssued => "already_issued",
    };

    Ok(Json(NativeCapabilityProofResultV1 {
        schema: NATIVE_CAPABILITY_RESULT_SCHEMA_V1,
        status,
        capability: outcome.capability,
        durable_generation: outcome.durable_generation,
    }))
}

fn trusted_now_ms() -> Result<u64, (StatusCode, Json<NativeCapabilityProblemV1>)> {
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
) -> (StatusCode, Json<NativeCapabilityProblemV1>) {
    if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Capability request exceeds the maximum body size",
            false,
        );
    }

    problem(
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "Capability request is invalid",
        false,
    )
}

fn problem_for_service_error(
    error: NativePassportCapabilityServiceError,
) -> (StatusCode, Json<NativeCapabilityProblemV1>) {
    match error {
        NativePassportCapabilityServiceError::InvalidRequest => problem(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Capability request is invalid",
            false,
        ),

        NativePassportCapabilityServiceError::NotFound => problem(
            StatusCode::NOT_FOUND,
            "device_not_registered",
            "Registered Passport device was not found",
            false,
        ),

        NativePassportCapabilityServiceError::Forbidden => problem(
            StatusCode::FORBIDDEN,
            "capability_issuance_rejected",
            "Capability issuance is not authorized",
            false,
        ),

        NativePassportCapabilityServiceError::ChallengeNotConsumable => problem(
            StatusCode::CONFLICT,
            "challenge_not_consumable",
            "Capability challenge is no longer consumable",
            false,
        ),

        NativePassportCapabilityServiceError::Conflict => problem(
            StatusCode::CONFLICT,
            "capability_conflict",
            "Capability issuance conflicts with durable state",
            false,
        ),

        NativePassportCapabilityServiceError::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "capability_service_unavailable",
            "Native Passport capability service is unavailable",
            true,
        ),
    }
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
) -> (StatusCode, Json<NativeCapabilityProblemV1>) {
    (
        status,
        Json(NativeCapabilityProblemV1 {
            schema: NATIVE_CAPABILITY_PROBLEM_SCHEMA_V1,
            code,
            message,
            retryable,
        }),
    )
}
