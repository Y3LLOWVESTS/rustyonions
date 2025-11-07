// crates/svc-passport/src/http/router.rs
//! RO:WHAT   HTTP router assembly (unit-state Router<()>).
//! RO:WHY    Axum 0.7 serve() accepts Router<()> directly; state via Extension(Arc<_>).
//! RO:PLUS   /metrics + per-route body caps + concurrency guards for verify hotpaths.

use crate::{
    config::Config,
    health::Health,
    kms::client::{DevKms, KmsClient},
    metrics, // /metrics exporter
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

// Handlers
use crate::http::handlers::{issue, verify};

pub fn build_router(cfg: Config, _health: Health) -> Router {
    // KMS client (dev) and IssuerState as shared Arc
    let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());
    let issuer = Arc::new(IssuerState::new(cfg.clone(), kms));

    // Tunables (env-first; conservative defaults)
    let max_body_bytes: usize = std::env::var("PASSPORT_MAX_MSG_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_048_576); // 1 MiB

    let verify_conc: usize = std::env::var("PASSPORT_VERIFY_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);

    let verify_batch_conc: usize = std::env::var("PASSPORT_VERIFY_BATCH_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);

    // Minimal /healthz so we can always probe quickly
    async fn healthz() -> impl IntoResponse {
        Json(serde_json::json!({ "ok": true }))
    }

    Router::new()
        // Admin/ops plane basics
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics::export))
        // v1 API
        .route(
            "/v1/passport/issue",
            post(issue::issue).route_layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .route(
            "/v1/passport/verify",
            post(verify::verify_one)
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
        .route("/admin/rotate", post(issue::rotate))
        .route("/admin/attest", get(issue::attest))
        // Carry IssuerState via Extension so the Router stays unit-state
        .layer(Extension(issuer))
}
