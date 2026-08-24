//! RO:WHAT — Strict HTTP handler for the CN-4 fixed Native Passport RegisterRoot proof submission.
//! RO:WHY — Physical CrabLink enrollment must submit concrete root-control evidence through the existing crash-recoverable svc-passport transaction rather than a generic proof endpoint.
//! RO:INTERACTS — `NativePassportServerRuntimeMountConfigV1`, injected `KmsClient`, `PassportChallengeV1`, canonical root-proof verification, and the durable RegisterRoot coordinator.
//! RO:INVARIANTS — the complete service-signed challenge is required; root epoch, proof/challenge domains, trusted context, challenge hash, scopes, operation binding, and acceptance time are never caller authority; successful HTTP return follows durable challenge consumption and root registration.
//! RO:METRICS — none yet; service/gateway observability remains composition-owned.
//! RO:CONFIG — uses the reviewed CN-4 runtime mount configuration and router body bound.
//! RO:SECURITY — strict JSON; no generic proof purpose, root secret, recovery factor, PIN, service KID choice, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — `crabnode_cn4_register_root_proof_route.rs`.

#![forbid(unsafe_code)]

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{extract::rejection::JsonRejection, http::StatusCode, Extension, Json};
use ron_proto::{Ed25519PublicKeyHex, PassportChallengeV1};
use serde::{Deserialize, Serialize};

use crate::{
    kms::client::KmsClient,
    native::{
        submit_register_root_proof_durable, NativePassportRegisterRootSubmitDispositionV1,
        NativePassportServerRuntimeMountConfigV1, NativePassportServerRuntimeMountError,
    },
};

const RESULT_SCHEMA: &str = "svc-passport.native-register-root-proof-result.v1";

const PROBLEM_SCHEMA: &str = "svc-passport.native-register-root-proof-problem.v1";

/// Dependencies retained only by the fixed RegisterRoot proof route.
pub(crate) struct NativeRegisterRootProofHttpState {
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
}

impl NativeRegisterRootProofHttpState {
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

/// Caller-controlled evidence for one fixed RegisterRoot proof.
///
/// Every challenge-derived or service-policy field is intentionally absent.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRegisterRootProofRequestV1 {
    pub challenge: PassportChallengeV1,
    pub root_public_key: Ed25519PublicKeyHex,
    pub proof_created_at_ms: u64,
    pub proof_signed_payload_hex: String,
}

#[derive(Debug, Serialize)]
pub struct NativeRegisterRootProofResultV1 {
    pub schema: &'static str,
    pub status: &'static str,
    pub durable_generation: u64,
}

#[derive(Debug, Serialize)]
pub struct NativeRegisterRootProofProblemV1 {
    pub schema: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub retryable: bool,
}

/// `POST /v1/passport/register/proof`.
pub(crate) async fn submit_register_root_proof(
    Extension(state): Extension<Arc<NativeRegisterRootProofHttpState>>,
    body: Result<Json<NativeRegisterRootProofRequestV1>, JsonRejection>,
) -> Result<
    Json<NativeRegisterRootProofResultV1>,
    (StatusCode, Json<NativeRegisterRootProofProblemV1>),
> {
    let Json(request) = body.map_err(|_| {
        problem(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "RegisterRoot proof request is invalid",
            false,
        )
    })?;

    let accepted_at_ms = trusted_now_ms()?;

    let outcome = submit_register_root_proof_durable(
        &state.runtime_config,
        Arc::clone(&state.kms),
        request.challenge,
        request.root_public_key,
        request.proof_created_at_ms,
        request.proof_signed_payload_hex,
        accepted_at_ms,
    )
    .await
    .map_err(problem_for_runtime_error)?;

    let status = match outcome.disposition {
        NativePassportRegisterRootSubmitDispositionV1::Registered => "registered",
        NativePassportRegisterRootSubmitDispositionV1::AlreadyRegistered => "already_registered",
    };

    Ok(Json(NativeRegisterRootProofResultV1 {
        schema: RESULT_SCHEMA,
        status,
        durable_generation: outcome.durable_generation,
    }))
}

fn trusted_now_ms() -> Result<u64, (StatusCode, Json<NativeRegisterRootProofProblemV1>)> {
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
) -> (StatusCode, Json<NativeRegisterRootProofProblemV1>) {
    match error {
        NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest
        | NativePassportServerRuntimeMountError::RegisterRootProofRejected => problem(
            StatusCode::BAD_REQUEST,
            "register_root_proof_rejected",
            "RegisterRoot proof is invalid",
            false,
        ),

        NativePassportServerRuntimeMountError::RegisterRootProofReplayRejected => problem(
            StatusCode::CONFLICT,
            "register_root_proof_replay",
            "RegisterRoot challenge is no longer consumable",
            false,
        ),

        NativePassportServerRuntimeMountError::RegisterRootConflict => problem(
            StatusCode::CONFLICT,
            "register_root_conflict",
            "Passport root registration conflicts with durable state",
            false,
        ),

        _ => {
            /*
             * Store, KMS, recovery, CAS, and coordinator details remain
             * internal and never become an HTTP oracle.
             */
            problem(
                StatusCode::SERVICE_UNAVAILABLE,
                "register_root_service_unavailable",
                "Native Passport registration service is unavailable",
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
) -> (StatusCode, Json<NativeRegisterRootProofProblemV1>) {
    (
        status,
        Json(NativeRegisterRootProofProblemV1 {
            schema: PROBLEM_SCHEMA,
            code,
            message,
            retryable,
        }),
    )
}
