//! RO:WHAT — API routes: read plane + SSE + commit write path.
//! RO:INVARIANTS — POST /registry/commit monotonic bump; idempotency-by-value left to client.

use std::sync::Arc;

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use http::StatusCode;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use super::middleware::{
    auth::AuthCfg,
    corr_id::CorrLayer,
    limits::{limits_layer, LimitCfg},
    metrics::MetricsLayer,
    timeouts::{timeouts_layer, TimeoutCfg},
};
use super::sse::sse_stream;
use crate::config::model::Config;
use crate::http::responses;
use crate::observability::metrics::RegistryMetrics;
use crate::storage::RegistryStore;

/// Shared application state for HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub metrics: RegistryMetrics,
    pub store: Arc<dyn RegistryStore>,

    // Config bits handlers need fast access to:
    pub sse_heartbeat_ms: u64,
}

pub fn registry_routes_with_cfg(
    metrics: RegistryMetrics,
    store: Arc<dyn RegistryStore>,
    cfg: &Config,
) -> Router {
    // Construct auth cfg and *read* a field so it’s not dead code until real auth wired.
    let auth = AuthCfg::default();
    let _auth_enabled = auth.enabled;

    let state = AppState {
        metrics: metrics.clone(),
        store,
        sse_heartbeat_ms: cfg.sse.heartbeat_ms,
    };

    // Build middleware stack in the intended order.
    let limit_cfg = LimitCfg {
        max_body_bytes: cfg.limits.max_request_bytes,
    };
    let timeout_cfg = TimeoutCfg {
        overall_ms: cfg.timeouts.request_ms,
    };

    let cors_layer = build_cors(&cfg.cors.allowed_origins);

    let stack = ServiceBuilder::new()
        .layer(MetricsLayer::new(metrics.clone()))
        .layer(limits_layer(&limit_cfg))
        .layer(timeouts_layer(&timeout_cfg))
        .layer(cors_layer)
        .layer(CorrLayer::new());

    Router::new()
        .route("/registry/head", get(get_head))
        .route("/registry/commit", post(post_commit))
        .route("/registry/stream", get(sse_stream))
        .fallback(|| async { (StatusCode::NOT_FOUND, "not found") })
        .with_state(state)
        .layer(stack)
}

fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    // If "*" present, be permissive (dev)
    if allowed_origins.iter().any(|o| o == "*") {
        return CorsLayer::permissive();
    }
    // Otherwise allow the exact origins provided; if parsing fails, fall back to permissive.
    let mut layer = CorsLayer::new().allow_methods(Any).allow_headers(Any);
    for origin in allowed_origins {
        if let Ok(hv) = http::HeaderValue::from_str(origin) {
            layer = layer.allow_origin(hv);
        }
    }
    layer
}

/// GET /registry/head
async fn get_head(State(st): State<AppState>) -> impl IntoResponse {
    let head = st.store.head().await;
    st.metrics.set_head_version(head.version);
    Json(head)
}

/// POST /registry/commit
#[derive(Debug, serde::Deserialize)]
struct CommitReq {
    /// b3:<base64> payload; validated minimally here.
    payload_b3: String,
}

async fn post_commit(State(st): State<AppState>, Json(req): Json<CommitReq>) -> impl IntoResponse {
    if !req.payload_b3.starts_with("b3:") {
        return responses::err(
            StatusCode::BAD_REQUEST,
            "invalid_payload",
            "payload_b3 must start with 'b3:'",
            "",
        )
        .into_response();
    }

    match st.store.commit(req.payload_b3).await {
        Ok(head) => {
            st.metrics.inc_commit_ok();
            st.metrics.set_head_version(head.version);
            (StatusCode::OK, Json(head)).into_response()
        }
        Err(e) => {
            st.metrics.inc_commit_err();
            responses::err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "commit_failed",
                &e.to_string(),
                "",
            )
            .into_response()
        }
    }
}
