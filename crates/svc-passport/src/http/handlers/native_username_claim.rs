//! RO:WHAT — Strict HTTP adapter for one CN-4 DeviceKey-protected username/profile claim.
//! RO:WHY — The public username mutation must bind the exact caller intent bytes to a live device-bound capability and PassportRequestProofV1 before durable profile truth changes.
//! RO:INTERACTS — PassportRequestProofV1 carried in the forwarded x-ron proof header, raw username-intent JSON bytes, server_request_proof_runtime, and UsernameClaimStore.
//! RO:INVARIANTS — caller never supplies Passport ownership; the HTTP body contains public profile intent only; body hash covers the exact forwarded bytes; canonical query is empty; admitted Passport identity is derived only from durable capability/device authority.
//! RO:METRICS — none in this slice; later route composition may add bounded outcome counters without identity-bearing labels.
//! RO:CONFIG — runtime, capability, and request-replay roots are injected by CrabNode composition; proof-header encoding is URL-safe base64 without padding.
//! RO:SECURITY — no DeviceKey/root private key, PIN, RecoveryRoot, bearer secret, caller-selected Passport subject, capability issuance, wallet, ledger, or node authority; internal errors remain redacted.
//! RO:TEST — focused unit tests below lock proof-header encoding, exact-byte body hashing, empty-query hashing, and rejection of caller-supplied Passport authority.

#![forbid(unsafe_code)]

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    Extension, Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ron_proto::{B3DigestHex, PassportRequestProofV1};
use serde::{Deserialize, Serialize};

use crate::{
    native::{
        admit_username_claim_request_proof_v1, NativePassportServerCapabilityRuntimeConfigV1,
        NativePassportServerRequestProofRuntimeConfigV1,
        NativePassportServerRequestProofRuntimeError, NativePassportServerRuntimeMountConfigV1,
    },
    profile::{ProfileClaimError, PublicProfileResponse, UsernameClaimRequest, UsernameClaimStore},
};

pub const NATIVE_USERNAME_REQUEST_PROOF_HEADER: &str = "x-ron-passport-request-proof";

pub const NATIVE_USERNAME_CLAIM_PROBLEM_SCHEMA_V1: &str =
    "svc-passport.native-username-claim-problem.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeProtectedUsernameClaimIntentV1 {
    pub requested_username: String,

    #[serde(default)]
    pub display_name: Option<String>,

    #[serde(default)]
    pub bio: Option<String>,

    #[serde(default)]
    pub avatar_image: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NativeUsernameClaimProblemV1 {
    pub schema: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub retryable: bool,
}

pub(crate) struct NativeProtectedUsernameClaimHttpState {
    profile_store: Arc<UsernameClaimStore>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
    capability_config: NativePassportServerCapabilityRuntimeConfigV1,
    request_config: NativePassportServerRequestProofRuntimeConfigV1,
}

impl NativeProtectedUsernameClaimHttpState {
    pub(crate) fn new(
        profile_store: Arc<UsernameClaimStore>,
        runtime_config: NativePassportServerRuntimeMountConfigV1,
        capability_config: NativePassportServerCapabilityRuntimeConfigV1,
        request_config: NativePassportServerRequestProofRuntimeConfigV1,
    ) -> Self {
        Self {
            profile_store,
            runtime_config,
            capability_config,
            request_config,
        }
    }
}

pub(crate) async fn claim_protected_profile(
    Extension(state): Extension<Arc<NativeProtectedUsernameClaimHttpState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<
    (StatusCode, Json<PublicProfileResponse>),
    (StatusCode, Json<NativeUsernameClaimProblemV1>),
> {
    /*
     * Parse the strict public intent first. There is intentionally no
     * `passport_subject` field: ownership is derived after request-proof
     * admission from durable svc-passport authority.
     */
    let intent: NativeProtectedUsernameClaimIntentV1 =
        serde_json::from_slice(&body).map_err(|_| {
            problem(
                StatusCode::BAD_REQUEST,
                "invalid_username_claim",
                "Username claim request is invalid",
                false,
            )
        })?;

    let proof = decode_request_proof_header(&headers)?;

    let expected_body_hash = digest_b3("body_hash", body.as_ref())?;

    /*
     * This public mutation has no query parameters. Hashing the exact empty
     * byte string gives one deterministic canonical query binding.
     */
    let expected_query_hash = digest_b3("canonical_query_hash", b"")?;

    let accepted_at_ms = trusted_now_ms()?;

    let admission = admit_username_claim_request_proof_v1(
        &state.runtime_config,
        &state.capability_config,
        &state.request_config,
        &proof,
        &expected_query_hash,
        &expected_body_hash,
        accepted_at_ms,
    )
    .map_err(problem_for_request_proof_error)?;

    let request = UsernameClaimRequest {
        passport_subject: admission.passport_id.as_str().to_owned(),
        requested_username: intent.requested_username,
        display_name: intent.display_name,
        bio: intent.bio,
        avatar_image: intent.avatar_image,
    };

    let record = state
        .profile_store
        .claim_main_username(request, accepted_at_ms)
        .map_err(problem_for_profile_error)?;

    Ok((
        StatusCode::CREATED,
        Json(PublicProfileResponse::from(&record)),
    ))
}

fn decode_request_proof_header(
    headers: &HeaderMap,
) -> Result<PassportRequestProofV1, (StatusCode, Json<NativeUsernameClaimProblemV1>)> {
    let encoded = headers
        .get(NATIVE_USERNAME_REQUEST_PROOF_HEADER)
        .ok_or_else(|| {
            problem(
                StatusCode::UNAUTHORIZED,
                "request_proof_required",
                "Native Passport request proof is required",
                false,
            )
        })?
        .to_str()
        .map_err(|_| {
            problem(
                StatusCode::UNAUTHORIZED,
                "request_proof_invalid",
                "Native Passport request proof is invalid",
                false,
            )
        })?;

    let bytes = URL_SAFE_NO_PAD.decode(encoded.as_bytes()).map_err(|_| {
        problem(
            StatusCode::UNAUTHORIZED,
            "request_proof_invalid",
            "Native Passport request proof is invalid",
            false,
        )
    })?;

    let proof: PassportRequestProofV1 = serde_json::from_slice(&bytes).map_err(|_| {
        problem(
            StatusCode::UNAUTHORIZED,
            "request_proof_invalid",
            "Native Passport request proof is invalid",
            false,
        )
    })?;

    proof.validate().map_err(|_| {
        problem(
            StatusCode::UNAUTHORIZED,
            "request_proof_invalid",
            "Native Passport request proof is invalid",
            false,
        )
    })?;

    Ok(proof)
}

fn digest_b3(
    field: &'static str,
    bytes: &[u8],
) -> Result<B3DigestHex, (StatusCode, Json<NativeUsernameClaimProblemV1>)> {
    B3DigestHex::parse(field, blake3::hash(bytes).to_hex().to_string()).map_err(|_| {
        problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "request_binding_unavailable",
            "Native Passport request binding is unavailable",
            true,
        )
    })
}

fn trusted_now_ms() -> Result<u64, (StatusCode, Json<NativeUsernameClaimProblemV1>)> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "Trusted service time is unavailable",
            true,
        )
    })?;

    let now_ms = u64::try_from(duration.as_millis()).map_err(|_| {
        problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "Trusted service time is unavailable",
            true,
        )
    })?;

    if now_ms == 0 {
        return Err(problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "trusted_time_unavailable",
            "Trusted service time is unavailable",
            true,
        ));
    }

    Ok(now_ms)
}

fn problem_for_request_proof_error(
    error: NativePassportServerRequestProofRuntimeError,
) -> (StatusCode, Json<NativeUsernameClaimProblemV1>) {
    match error {
        NativePassportServerRequestProofRuntimeError::InvalidRequest => problem(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Protected username request is invalid",
            false,
        ),

        NativePassportServerRequestProofRuntimeError::ReplayRejected => problem(
            StatusCode::CONFLICT,
            "request_replay",
            "Protected username request is no longer consumable",
            false,
        ),

        NativePassportServerRequestProofRuntimeError::AuthorityChanged => problem(
            StatusCode::CONFLICT,
            "authority_changed",
            "Native Passport authority changed during request verification",
            false,
        ),

        NativePassportServerRequestProofRuntimeError::CapabilityNotFound
        | NativePassportServerRequestProofRuntimeError::CapabilityRejected
        | NativePassportServerRequestProofRuntimeError::DeviceAuthorityRejected
        | NativePassportServerRequestProofRuntimeError::ProofRejected => problem(
            StatusCode::FORBIDDEN,
            "username_claim_not_authorized",
            "Username claim is not authorized",
            false,
        ),

        NativePassportServerRequestProofRuntimeError::InvalidConfig
        | NativePassportServerRequestProofRuntimeError::StateFull
        | NativePassportServerRequestProofRuntimeError::StorageUnavailable
        | NativePassportServerRequestProofRuntimeError::StorageCorrupt => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "username_claim_service_unavailable",
            "Username claim service is unavailable",
            true,
        ),
    }
}

fn problem_for_profile_error(
    error: ProfileClaimError,
) -> (StatusCode, Json<NativeUsernameClaimProblemV1>) {
    let status = match &error {
        ProfileClaimError::UsernameUnavailable { .. }
        | ProfileClaimError::PassportAlreadyHasUsername { .. } => StatusCode::CONFLICT,

        ProfileClaimError::StorePoisoned
        | ProfileClaimError::StoreUnavailable { .. }
        | ProfileClaimError::StoreCorrupt { .. } => StatusCode::SERVICE_UNAVAILABLE,

        _ => StatusCode::BAD_REQUEST,
    };

    let retryable = matches!(
        error,
        ProfileClaimError::StorePoisoned | ProfileClaimError::StoreUnavailable { .. }
    );

    problem(
        status,
        error.code(),
        "Username claim was rejected",
        retryable,
    )
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
) -> (StatusCode, Json<NativeUsernameClaimProblemV1>) {
    (
        status,
        Json(NativeUsernameClaimProblemV1 {
            schema: NATIVE_USERNAME_CLAIM_PROBLEM_SCHEMA_V1,
            code,
            message,
            retryable,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_proto::{
        CapabilityIdV1, DeviceIdV1, Ed25519SignatureV1, PASSPORT_REQUEST_PROOF_V1_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn digest(field: &'static str, value: &str) -> B3DigestHex {
        B3DigestHex::parse(field, value).expect("digest")
    }

    fn proof() -> PassportRequestProofV1 {
        PassportRequestProofV1 {
            version: PASSPORT_REQUEST_PROOF_V1_VERSION,
            capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}",))
                .expect("capability ID"),
            request_method: "POST".to_owned(),
            canonical_path: "/identity/passport/profile/claim".to_owned(),
            canonical_query_hash: digest("canonical_query_hash", HEX_C),
            body_hash: digest("body_hash", HEX_D),
            timestamp_ms: 1_000_000,
            request_nonce: digest("request_nonce", HEX_E),
            device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_B}",))
                .expect("Device ID"),
            device_signature: Ed25519SignatureV1::from_bytes([0x42; 64]),
        }
    }

    #[test]
    fn proof_header_round_trips_strict_v1_dto() {
        let proof = proof();

        let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&proof).expect("proof JSON"));

        let mut headers = HeaderMap::new();

        headers.insert(
            NATIVE_USERNAME_REQUEST_PROOF_HEADER,
            encoded.parse().expect("header"),
        );

        let decoded = decode_request_proof_header(&headers).expect("decode proof header");

        assert_eq!(decoded, proof);
    }

    #[test]
    fn body_hash_binds_exact_forwarded_bytes() {
        let first = br#"{"requested_username":"testmac"}"#;

        let second = br#"{ "requested_username":"testmac" }"#;

        let first_hash = digest_b3("body_hash", first).expect("first hash");

        let second_hash = digest_b3("body_hash", second).expect("second hash");

        assert_ne!(first_hash, second_hash);

        assert_eq!(
            first_hash.as_str(),
            blake3::hash(first).to_hex().to_string(),
        );
    }

    #[test]
    fn canonical_query_for_username_claim_is_empty_bytes() {
        let expected = digest_b3("canonical_query_hash", b"").expect("query hash");

        assert_eq!(expected.as_str(), blake3::hash(b"").to_hex().to_string(),);
    }

    #[test]
    fn caller_cannot_supply_passport_subject_authority() {
        let injected = br#"{
            "passport_subject":"passport:forged",
            "requested_username":"testmac"
        }"#;

        assert!(serde_json::from_slice::<NativeProtectedUsernameClaimIntentV1>(injected).is_err(),);

        let valid = br#"{
            "requested_username":"testmac",
            "display_name":null,
            "bio":null,
            "avatar_image":null
        }"#;

        let parsed = serde_json::from_slice::<NativeProtectedUsernameClaimIntentV1>(valid)
            .expect("valid intent");

        assert_eq!(parsed.requested_username, "testmac",);
    }

    #[test]
    fn source_has_no_secret_or_value_authority() {
        let source = include_str!("native_username_claim.rs");

        let implementation = source
            .split_once("\n#[cfg(test)]")
            .expect("implementation")
            .0;

        for forbidden in [
            "device_private_key",
            "root_private_key",
            "recovery_phrase",
            "raw_pin",
            "wallet.spend(",
            "ledger.write(",
            "reward.issue(",
            "node.control(",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "protected username HTTP adapter gained forbidden pattern {forbidden}",
            );
        }
    }
}
