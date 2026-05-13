// crates/svc-passport/src/http/router.rs
//! RO:WHAT — HTTP router assembly for svc-passport.
//! RO:WHY — Axum 0.7 serve accepts Router<()> directly; shared state is carried via typed Extension layers.
//! RO:INTERACTS — issue/verify/profile handlers, DevKms, IssuerState, UsernameClaimStore, metrics exporter.
//! RO:INVARIANTS — body caps on mutating/hot routes; no wallet/ledger mutation in profile routes.
//! RO:METRICS — /metrics exporter plus handler-level passport counters where already present.
//! RO:CONFIG — PASSPORT_MAX_MSG_BYTES, PASSPORT_VERIFY_CONCURRENCY, PASSPORT_VERIFY_BATCH_CONCURRENCY.
//! RO:SECURITY — profile routes expose public display claims only; verify routes preserve aud/alg checks.
//! RO:TEST — tests/handlers.rs, tests/profile_routes.rs, tests/limits.rs, tests/audience_alg.rs.

use crate::{
    config::Config,
    health::Health,
    kms::client::{DevKms, KmsClient},
    metrics,
    profile::UsernameClaimStore,
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

/// Build the svc-passport HTTP router.
///
/// The router remains unit-state. Internal shared state is injected with typed
/// `Extension(Arc<_>)` layers so Axum 0.7 service bootstrap stays simple.
pub fn build_router(cfg: Config, _health: Health) -> Router {
    let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());
    let issuer = Arc::new(IssuerState::new(cfg, kms));
    let profile_store = Arc::new(UsernameClaimStore::new());

    let max_body_bytes = env_usize("PASSPORT_MAX_MSG_BYTES", 1_048_576);
    let verify_conc = env_usize("PASSPORT_VERIFY_CONCURRENCY", 64);
    let verify_batch_conc = env_usize("PASSPORT_VERIFY_BATCH_CONCURRENCY", 16);

    async fn healthz() -> impl IntoResponse {
        Json(serde_json::json!({ "ok": true }))
    }

    Router::new()
        // Admin/ops plane basics.
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics::export))
        // v1 capability-token API.
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
        // NEXT_LEVEL Phase 3 local public profile API.
        .route("/v1/passport/profile/_debug", get(profile::profile_debug))
        .route(
            "/v1/passport/profile/claim",
            post(profile::claim_profile).route_layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .route("/v1/passport/profile/:username", get(profile::get_profile))
        // Admin/dev KMS plane.
        .route("/admin/rotate", post(issue::rotate))
        .route("/admin/attest", get(issue::attest))
        // Typed Extension layers. Handlers request only the state type they need.
        .layer(Extension(profile_store))
        .layer(Extension(issuer))
}

fn env_usize(name: &str, default_value: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default_value)
}
