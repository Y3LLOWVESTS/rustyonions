//! RO:WHAT — HTTP adapter for one fixed root-authorized Native Passport device-registration mutation.
//! RO:WHY — CN-4 needs a bounded svc-passport admission seam for a previously root-signed DeviceAuthorizationV1 before possession/capability work begins.
//! RO:INTERACTS — DeviceAuthorizationV1, server_device_registration_runtime, trusted service clock, and CrabNode's constrained native router.
//! RO:INVARIANTS — caller supplies only the signed authorization; registered_at_ms and durable generation are server-owned; successful HTTP return follows strict root verification and immutable-generation persistence; exact repeat is idempotent.
//! RO:METRICS — none yet; composition owns future bounded admission metrics.
//! RO:CONFIG — registry path/network/environment come from reviewed Native Passport runtime configuration; route body is capped by the router.
//! RO:SECURITY — no device private key, possession proof, root secret, PIN, KMS signing, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — crabnode_cn4_device_authorize_route.rs plus server_device_registration_runtime focused tests.

#![forbid(unsafe_code)]

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{extract::rejection::JsonRejection, http::StatusCode, Extension, Json};

use ron_proto::DeviceAuthorizationV1;
use serde::{Deserialize, Serialize};

use crate::native::{
    register_device_authorization_durable, NativePassportServerDeviceRegistrationDispositionV1,
    NativePassportServerDeviceRegistrationError, NativePassportServerRuntimeMountConfigV1,
};

pub const NATIVE_DEVICE_AUTHORIZATION_RESULT_SCHEMA_V1: &str =
    "svc-passport.native-device-authorization-result.v1";

pub const NATIVE_DEVICE_AUTHORIZATION_PROBLEM_SCHEMA_V1: &str =
    "svc-passport.native-device-authorization-problem.v1";

pub(crate) struct NativeDeviceAuthorizeHttpState {
    runtime_config: NativePassportServerRuntimeMountConfigV1,
}

impl NativeDeviceAuthorizeHttpState {
    pub(crate) fn new(runtime_config: NativePassportServerRuntimeMountConfigV1) -> Self {
        Self { runtime_config }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDeviceAuthorizeRequestV1 {
    pub authorization: DeviceAuthorizationV1,
}

#[derive(Debug, Serialize)]
pub struct NativeDeviceAuthorizeResultV1 {
    pub schema: &'static str,
    pub status: &'static str,
    pub durable_generation: u64,
}

#[derive(Debug, Serialize)]
pub struct NativeDeviceAuthorizeProblemV1 {
    pub schema: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub retryable: bool,
}

pub(crate) async fn submit_device_authorization(
    Extension(state): Extension<Arc<NativeDeviceAuthorizeHttpState>>,

    body: Result<Json<NativeDeviceAuthorizeRequestV1>, JsonRejection>,
) -> Result<Json<NativeDeviceAuthorizeResultV1>, (StatusCode, Json<NativeDeviceAuthorizeProblemV1>)>
{
    let Json(request) = body.map_err(problem_for_json_rejection)?;

    let registered_at_ms = trusted_now_ms()?;

    let outcome = register_device_authorization_durable(
        &state.runtime_config,
        request.authorization,
        registered_at_ms,
    )
    .map_err(problem_for_registration_error)?;

    let status = match outcome.disposition {
        NativePassportServerDeviceRegistrationDispositionV1::Registered => "registered",

        NativePassportServerDeviceRegistrationDispositionV1::AlreadyRegistered => {
            "already_registered"
        }
    };

    Ok(Json(NativeDeviceAuthorizeResultV1 {
        schema: NATIVE_DEVICE_AUTHORIZATION_RESULT_SCHEMA_V1,

        status,

        durable_generation: outcome.durable_generation,
    }))
}

fn problem_for_json_rejection(
    rejection: JsonRejection,
) -> (StatusCode, Json<NativeDeviceAuthorizeProblemV1>) {
    if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Device authorization request exceeds the maximum body size",
            false,
        );
    }

    problem(
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "Device authorization request is invalid",
        false,
    )
}

fn trusted_now_ms() -> Result<u64, (StatusCode, Json<NativeDeviceAuthorizeProblemV1>)> {
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

fn problem_for_registration_error(
    error: NativePassportServerDeviceRegistrationError,
) -> (StatusCode, Json<NativeDeviceAuthorizeProblemV1>) {
    match error {
        NativePassportServerDeviceRegistrationError::UnknownPassport => problem(
            StatusCode::CONFLICT,
            "passport_root_not_registered",
            "Passport root is not registered",
            false,
        ),

        NativePassportServerDeviceRegistrationError::InvalidRegistrationTime
        | NativePassportServerDeviceRegistrationError::AuthorizationRejected => problem(
            StatusCode::FORBIDDEN,
            "device_authorization_rejected",
            "Device authorization is not accepted",
            false,
        ),

        NativePassportServerDeviceRegistrationError::DeviceRegistrationConflict => problem(
            StatusCode::CONFLICT,
            "device_authorization_conflict",
            "Device authorization conflicts with durable state",
            false,
        ),

        NativePassportServerDeviceRegistrationError::TrustedContextInvalid
        | NativePassportServerDeviceRegistrationError::GenerationOverflow
        | NativePassportServerDeviceRegistrationError::StorageUnavailable(_)
        | NativePassportServerDeviceRegistrationError::StorageCorrupt(_) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "device_authorization_service_unavailable",
            "Native Passport device authorization service is unavailable",
            true,
        ),
    }
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
) -> (StatusCode, Json<NativeDeviceAuthorizeProblemV1>) {
    (
        status,
        Json(NativeDeviceAuthorizeProblemV1 {
            schema: NATIVE_DEVICE_AUTHORIZATION_PROBLEM_SCHEMA_V1,

            code,
            message,
            retryable,
        }),
    )
}
