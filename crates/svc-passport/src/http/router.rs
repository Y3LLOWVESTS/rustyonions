//! RO:WHAT — HTTP router assembly for full svc-passport, constrained CrabNode Native Passport identity, and fixed CN-4 device-bound capability issuance.
//! RO:WHY — Keep Passport/device/capability authority in svc-passport while exposing only explicitly reviewed fixed-purpose routes with injected durable dependencies.
//! RO:INTERACTS — issue/verify/profile handlers, `KmsClient`, `IssuerState`, `UsernameClaimStore`, RegisterRoot/device-session/capability runtimes, metrics exporter, and macronode CN-3/CN-4.
//! RO:INVARIANTS — existing builders retain their prior surfaces; capability-enabled composition adds only fixed IssueCapability challenge/proof routes after recovery; ProveSession never becomes a caller-selectable generic proof API.
//! RO:METRICS — full router retains `/metrics`; constrained CrabNode identity/capability composition adds no process-global metrics.
//! RO:CONFIG — `PASSPORT_MAX_MSG_BYTES`, trusted Native Passport roots/context, and explicit dedicated capability state/redo roots plus bounded service-owned TTL.
//! RO:SECURITY — KMS stays service-owned; no caller-selected purpose/TTL/policy/context, secret export, username mutation, KMS admin, wallet mutation, or ledger mutation.
//! RO:TEST — existing CN-4 router tests plus `crabnode_cn4_capability_route`.

use crate::{
    config::Config, health::Health, kms::client::KmsClient, metrics, profile::UsernameClaimStore,
    state::issuer::IssuerState,
};

use axum::{
    extract::DefaultBodyLimit,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};

use std::sync::Arc;

use tower::limit::ConcurrencyLimitLayer;

use crate::http::handlers::{issue, profile, verify};

#[cfg(feature = "native-passport")]
use crate::http::handlers::native_capability::{
    issue_capability_challenge, submit_capability_proof, NativeCapabilityHttpState,
};

#[cfg(feature = "native-passport")]
use crate::http::handlers::native_username_claim::{
    claim_protected_profile, NativeProtectedUsernameClaimHttpState,
};

#[cfg(feature = "native-passport")]
use crate::http::handlers::native_register_root_challenge::{
    issue_register_root_challenge, NativeRegisterRootChallengeHttpState,
};

#[cfg(feature = "native-passport")]
use crate::http::handlers::native_register_root_proof::{
    submit_register_root_proof, NativeRegisterRootProofHttpState,
};

#[cfg(feature = "native-passport")]
use crate::http::handlers::native_register_root_trust_anchor::{
    read_register_root_trust_anchor, NativeRegisterRootTrustAnchorHttpState,
};

#[cfg(feature = "native-passport")]
use crate::native::{
    preflight_native_passport_capability_runtime, preflight_native_passport_request_proof_runtime,
    preflight_native_passport_server_runtime_mount, NativePassportServerCapabilityRuntimeConfigV1,
    NativePassportServerRequestProofRuntimeConfigV1, NativePassportServerRuntimeMountConfigV1,
    NativePassportServerRuntimeMountError,
};

/// Build the normal full svc-passport HTTP router.
///
/// Standalone svc-passport keeps its existing behavior. The default service
/// binary may use the feature-gated development KMS; production callers can
/// continue using `build_router_with_kms` with an injected implementation.
pub fn build_router(cfg: Config, health: Health) -> Router {
    let kms = default_dev_kms();

    build_router_with_kms(cfg, health, kms)
}

#[cfg(feature = "dev-kms")]
fn default_dev_kms() -> Arc<dyn KmsClient> {
    Arc::new(crate::kms::client::DevKms::new())
}

#[cfg(not(feature = "dev-kms"))]
fn default_dev_kms() -> Arc<dyn KmsClient> {
    panic!("svc-passport default router requires an injected service KMS when dev-kms is disabled")
}

async fn healthz() -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true
    }))
}

/// Canonical public profile route owner.
///
/// Both the full service router and CrabNode's profile-only composition reuse
/// this exact route table. CN-4 will replace the backing store without moving
/// username/profile authority out of svc-passport.
fn profile_routes(max_body_bytes: usize) -> Router {
    profile_routes_with_store(max_body_bytes, Arc::new(UsernameClaimStore::new()))
}

fn profile_routes_with_store(
    max_body_bytes: usize,
    profile_store: Arc<UsernameClaimStore>,
) -> Router {
    Router::new()
        .route("/v1/passport/profile/_debug", get(profile::profile_debug))
        .route(
            "/v1/passport/profile/claim",
            post(profile::claim_profile).route_layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .route(
            "/v1/passport/profile/by-subject/:passport_subject",
            get(profile::get_profile_by_passport_subject),
        )
        .route("/v1/passport/profile/:username", get(profile::get_profile))
        .layer(Extension(profile_store))
}

/// Build the public profile read surface without a username mutation route.
///
/// Protected CrabNode composition uses this table so the historical
/// caller-supplied Passport-subject claim cannot coexist with DeviceKey
/// request-proof authority.
fn profile_read_routes_with_store(profile_store: Arc<UsernameClaimStore>) -> Router {
    Router::new()
        .route("/v1/passport/profile/_debug", get(profile::profile_debug))
        .route(
            "/v1/passport/profile/by-subject/:passport_subject",
            get(profile::get_profile_by_passport_subject),
        )
        .route("/v1/passport/profile/:username", get(profile::get_profile))
        .layer(Extension(profile_store))
}

/// Build the CrabNode public profile/identity-only router.
///
/// This surface deliberately contains no capability issuance, verification,
/// public-key export, KMS rotation, KMS attestation, or KMS construction.
pub fn build_profile_router() -> Router {
    let max_body_bytes = env_usize("PASSPORT_MAX_MSG_BYTES", 1_048_576);

    Router::new()
        .route("/healthz", get(healthz))
        .merge(profile_routes(max_body_bytes))
}

/// Build the KMS-free CrabNode profile surface with an injected svc-passport claim store.
///
/// This changes only persistence backing. It does not add issue/verify,
/// key-export, KMS-admin, wallet, or ledger authority.
pub fn build_profile_router_with_store(profile_store: Arc<UsernameClaimStore>) -> Router {
    let max_body_bytes = env_usize("PASSPORT_MAX_MSG_BYTES", 1_048_576);

    Router::new()
        .route("/healthz", get(healthz))
        .merge(profile_routes_with_store(max_body_bytes, profile_store))
}

/// Build CrabNode's constrained profile router only after the real Native
/// Passport durable runtime has passed startup/recovery preflight.
///
/// This adds the reviewed RegisterRoot trust-anchor/challenge/proof,
/// root-authorized device admission, and fixed-purpose `ProveSession`
/// challenge/proof surfaces. The compact `/v1/passport/challenge` and
/// `/v1/passport/prove` paths are not caller-selectable generic proof APIs:
/// svc-passport fixes their purpose to `ProveSession`. Capability issue/verify,
/// `/v1/keys`, metrics, and KMS-admin routes remain absent.
#[cfg(feature = "native-passport")]
pub async fn build_native_profile_router_with_store_and_kms(
    profile_store: Arc<UsernameClaimStore>,
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
) -> Result<Router, NativePassportServerRuntimeMountError> {
    let app = build_profile_router_with_store(profile_store);

    mount_native_profile_runtime_routes(app, kms, runtime_config).await
}

#[cfg(feature = "native-passport")]
async fn mount_native_profile_runtime_routes(
    app: Router,
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
) -> Result<Router, NativePassportServerRuntimeMountError> {
    /*
     * Recovery and KMS identity remain startup gates. The caller chooses
     * whether the profile base contains mutation; this helper owns only the
     * canonical Native Passport RegisterRoot/device-session route set.
     */
    preflight_native_passport_server_runtime_mount(&runtime_config, Arc::clone(&kms)).await?;

    let max_body_bytes = env_usize("PASSPORT_MAX_MSG_BYTES", 1_048_576);

    let register_root_trust_anchor_state = Arc::new(
        NativeRegisterRootTrustAnchorHttpState::new(Arc::clone(&kms), &runtime_config).await?,
    );

    let register_root_challenge_state = Arc::new(NativeRegisterRootChallengeHttpState::new(
        Arc::clone(&kms),
        runtime_config.clone(),
    ));

    let device_authorize_state = Arc::new(
        crate::http::handlers::native_device_authorize::NativeDeviceAuthorizeHttpState::new(
            runtime_config.clone(),
        ),
    );

    let device_session_state = Arc::new(
        crate::http::handlers::native_device_session::NativeDeviceSessionHttpState::new(
            Arc::clone(&kms),
            runtime_config.clone(),
        ),
    );

    let register_root_proof_state =
        Arc::new(NativeRegisterRootProofHttpState::new(kms, runtime_config));

    Ok(app
        .route(
            "/v1/passport/register/trust-anchor",
            get(read_register_root_trust_anchor),
        )
        .route(
            "/v1/passport/register/challenge",
            post(issue_register_root_challenge).route_layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .route(
            "/v1/passport/device/authorize",
            post(crate::http::handlers::native_device_authorize::submit_device_authorization)
                .route_layer(DefaultBodyLimit::max(16_384)),
        )
        .route(
            "/v1/passport/challenge",
            post(crate::http::handlers::native_device_session::issue_device_session_challenge)
                .route_layer(DefaultBodyLimit::max(16_384)),
        )
        .route(
            "/v1/passport/prove",
            post(crate::http::handlers::native_device_session::submit_device_session_proof)
                .route_layer(DefaultBodyLimit::max(16_384)),
        )
        .route(
            "/v1/passport/register/proof",
            post(submit_register_root_proof).route_layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .layer(Extension(register_root_trust_anchor_state))
        .layer(Extension(register_root_challenge_state))
        .layer(Extension(device_authorize_state))
        .layer(Extension(device_session_state))
        .layer(Extension(register_root_proof_state)))
}

/// Build CrabNode's constrained Native Passport router with the fixed
/// device-bound IssueCapability challenge/proof surface.
///
/// The existing native builder deliberately remains capability-free. This
/// builder first proves the existing Native Passport runtime is recoverable,
/// then proves capability redo recovery is complete, and only then mounts
/// the two fixed-purpose capability routes.
#[cfg(feature = "native-passport")]
pub async fn build_native_profile_router_with_store_kms_and_capability(
    profile_store: Arc<UsernameClaimStore>,
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
    capability_config: NativePassportServerCapabilityRuntimeConfigV1,
) -> Result<Router, NativePassportServerRuntimeMountError> {
    let app = build_native_profile_router_with_store_and_kms(
        profile_store,
        Arc::clone(&kms),
        runtime_config.clone(),
    )
    .await?;

    preflight_native_passport_capability_runtime(
        &runtime_config,
        &capability_config,
        Arc::clone(&kms),
    )
    .await
    .map_err(|_| {
        NativePassportServerRuntimeMountError::DurableRuntimeRecovery(
            "capability runtime recovery failed".to_owned(),
        )
    })?;

    let capability_state = Arc::new(NativeCapabilityHttpState::new(
        kms,
        runtime_config,
        capability_config,
    ));

    Ok(app
        .route(
            "/v1/passport/capability/challenge",
            post(issue_capability_challenge).route_layer(DefaultBodyLimit::max(16_384)),
        )
        .route(
            "/v1/passport/capability/prove",
            post(submit_capability_proof).route_layer(DefaultBodyLimit::max(16_384)),
        )
        .layer(Extension(capability_state)))
}

/// Build the public CrabNode Native Passport router with protected username
/// mutation plus fixed device-bound capability issuance.
///
/// Unlike the historical capability builder, this composition starts from
/// profile reads only. The sole username mutation route derives Passport
/// ownership from the admitted capability + current DeviceAuthorization and
/// requires an exact DeviceKey-signed PassportRequestProofV1.
#[cfg(feature = "native-passport")]
pub async fn build_native_profile_router_with_store_kms_capability_and_request_proof(
    profile_store: Arc<UsernameClaimStore>,
    kms: Arc<dyn KmsClient>,
    runtime_config: NativePassportServerRuntimeMountConfigV1,
    capability_config: NativePassportServerCapabilityRuntimeConfigV1,
    request_config: NativePassportServerRequestProofRuntimeConfigV1,
) -> Result<Router, NativePassportServerRuntimeMountError> {
    let app = Router::new()
        .route("/healthz", get(healthz))
        .merge(profile_read_routes_with_store(Arc::clone(&profile_store)));

    let app =
        mount_native_profile_runtime_routes(app, Arc::clone(&kms), runtime_config.clone()).await?;

    preflight_native_passport_capability_runtime(
        &runtime_config,
        &capability_config,
        Arc::clone(&kms),
    )
    .await
    .map_err(|_| {
        NativePassportServerRuntimeMountError::DurableRuntimeRecovery(
            "capability runtime recovery failed".to_owned(),
        )
    })?;

    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            NativePassportServerRuntimeMountError::DurableRuntimeRecovery(
                "request-proof trusted time unavailable".to_owned(),
            )
        })?;

    let now_ms = u64::try_from(duration.as_millis()).map_err(|_| {
        NativePassportServerRuntimeMountError::DurableRuntimeRecovery(
            "request-proof trusted time unavailable".to_owned(),
        )
    })?;

    preflight_native_passport_request_proof_runtime(
        &runtime_config,
        &capability_config,
        &request_config,
        now_ms,
    )
    .map_err(|_| {
        NativePassportServerRuntimeMountError::DurableRuntimeRecovery(
            "request-proof replay recovery failed".to_owned(),
        )
    })?;

    let protected_username_state = Arc::new(NativeProtectedUsernameClaimHttpState::new(
        profile_store,
        runtime_config.clone(),
        capability_config.clone(),
        request_config,
    ));

    let capability_state = Arc::new(NativeCapabilityHttpState::new(
        kms,
        runtime_config,
        capability_config,
    ));

    Ok(app
        .route(
            "/v1/passport/profile/claim",
            post(claim_protected_profile).route_layer(DefaultBodyLimit::max(16_384)),
        )
        .route(
            "/v1/passport/capability/challenge",
            post(issue_capability_challenge).route_layer(DefaultBodyLimit::max(16_384)),
        )
        .route(
            "/v1/passport/capability/prove",
            post(submit_capability_proof).route_layer(DefaultBodyLimit::max(16_384)),
        )
        .layer(Extension(protected_username_state))
        .layer(Extension(capability_state)))
}

/// Build the full svc-passport HTTP router with an explicitly injected KMS.
///
/// The KMS-bearing service surface remains separate from CrabNode's
/// profile-only composition.
pub fn build_router_with_kms(cfg: Config, _health: Health, kms: Arc<dyn KmsClient>) -> Router {
    let issuer = Arc::new(IssuerState::new(cfg, kms));

    let max_body_bytes = env_usize("PASSPORT_MAX_MSG_BYTES", 1_048_576);

    let verify_conc = env_usize("PASSPORT_VERIFY_CONCURRENCY", 64);

    let verify_batch_conc = env_usize("PASSPORT_VERIFY_BATCH_CONCURRENCY", 16);

    Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics::export))
        .route(
            "/v1/passport/issue",
            post(issue::issue).route_layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .route(
            "/v1/passport/verify",
            post(verify::verify)
                .route_layer(DefaultBodyLimit::max(max_body_bytes))
                .route_layer(ConcurrencyLimitLayer::new(verify_conc)),
        )
        .route(
            "/v1/passport/verify_batch",
            post(verify::verify_batch)
                .route_layer(DefaultBodyLimit::max(max_body_bytes))
                .route_layer(ConcurrencyLimitLayer::new(verify_batch_conc)),
        )
        .route("/v1/keys", get(issue::keys))
        .merge(profile_routes(max_body_bytes))
        .route("/admin/rotate", post(issue::rotate))
        .route("/admin/attest", get(issue::attest))
        .layer(Extension(issuer))
}

fn env_usize(name: &str, default_value: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default_value)
}
