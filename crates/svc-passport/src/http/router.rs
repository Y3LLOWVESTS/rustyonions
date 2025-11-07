//! RO:WHAT — Axum router assembly for public v1 + admin ops.
//! RO:INVARIANTS — POST caps; JSON only; stable paths.

use crate::{
    health::Health, http::handlers as h, kms::client::KmsClient, state::issuer::IssuerState, Config,
};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn build_router(cfg: Config, health: Health) -> Router {
    // KMS client (trait object) — dev-kms by default; can swap to ron-kms later.
    let kms: Arc<dyn KmsClient> = Arc::new(crate::kms::client::DevKms::new());

    let issuer = Arc::new(IssuerState::new(cfg.clone(), kms.clone()));
    // Signal readiness once initial keys are present
    health.ready.set_config_loaded(true);

    Router::new()
        .route("/v1/passport/issue", post(h::issue::issue))
        .route("/v1/passport/verify", post(h::verify::verify_one))
        .route("/v1/passport/verify_batch", post(h::verify::verify_batch))
        .route("/v1/keys", get(h::issue::keys))
        .route("/admin/rotate", post(h::issue::rotate))
        .route("/admin/attest", get(h::issue::attest))
        .with_state((cfg, issuer, health))
}
