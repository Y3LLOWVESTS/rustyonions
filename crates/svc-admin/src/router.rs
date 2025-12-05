// crates/svc-admin/src/router.rs
//
// WHAT: Axum router for svc-admin.
// WHY:  Defines the HTTP surface exposed by the admin console.
//
// Endpoints:
//   - GET /healthz                        → liveness
//   - GET /readyz                         → readiness (coarse in v1)
//   - GET /metrics                        → Prometheus metrics for svc-admin
//   - GET /api/ui-config                  → UiConfigDto
//   - GET /api/me                         → MeResponse (current operator)
//   - GET /api/nodes                      → NodeSummary[]
//   - GET /api/nodes/{id}/status          → AdminStatusView
//   - GET /api/nodes/{id}/metrics/facets  → FacetMetricsSummary[]
//

use crate::auth;
use crate::dto;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

/// Build the HTTP router for svc-admin.
///
/// This wires:
/// - Liveness:   GET /healthz
/// - Readiness:  GET /readyz
/// - Metrics:    GET /metrics
/// - UI config:  GET /api/ui-config
/// - Identity:   GET /api/me
/// - Nodes:      GET /api/nodes
/// - Node view:  GET /api/nodes/{id}/status
/// - Facets:     GET /api/nodes/{id}/metrics/facets
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        // Prometheus metrics for svc-admin itself (default registry).
        .route(
            "/metrics",
            get(crate::metrics::prometheus_bridge::metrics_handler),
        )
        .route("/api/ui-config", get(ui_config))
        .route("/api/me", get(me))
        .route("/api/nodes", get(nodes))
        .route("/api/nodes/:id/status", get(node_status))
        .route("/api/nodes/:id/metrics/facets", get(node_facets))
        .with_state(state)
}

/// Simple liveness probe.
/// Invariant: if this is not 200/"ok", the process is very unhealthy.
async fn healthz() -> &'static str {
    "ok"
}

/// Readiness probe.
///
/// For now this always returns `{ "ready": true }` but is wired to AppState so
/// we can gate on real readiness later (node registry, samplers, etc.).
async fn readyz(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ready": true }))
}

/// UI config consumed by the SPA for theme/lang/read-only state.
async fn ui_config(State(state): State<Arc<AppState>>) -> Json<dto::ui::UiConfigDto> {
    Json(dto::ui::UiConfigDto::from_cfg(&state.config))
}

/// Identity endpoint for the current user.
///
/// Uses the auth module to resolve an Identity from the configured auth mode
/// and inbound headers, then maps that into MeResponse.
///
/// Modes:
///   - "none":    synthetic dev identity
///   - "ingress": X-User / X-Groups headers (soft behavior)
///   - "passport":currently unimplemented; falls back to dev identity here
async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Json<dto::me::MeResponse> {
    let auth_cfg = &state.config.auth;

    let identity = auth::resolve_identity_from_headers(auth_cfg, &headers)
        .unwrap_or_else(|_err| auth::Identity::dev_fallback());

    Json(dto::me::MeResponse::from_identity(identity, auth_cfg))
}

/// List all configured nodes as NodeSummary DTOs.
async fn nodes(State(state): State<Arc<AppState>>) -> Json<Vec<dto::node::NodeSummary>> {
    let summaries = state.nodes.list_summaries();
    Json(summaries)
}

/// Get a single node's admin view.
///
/// - 200 OK with AdminStatusView when the node is known;
/// - 404 NOT_FOUND when the node id is not in the registry.
async fn node_status(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<dto::node::AdminStatusView>, StatusCode> {
    match state.nodes.get_status(&id).await {
        Some(view) => Ok(Json(view)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get facet-level metrics summaries for a node.
///
/// Always 200 with `[]` when we have no samples yet; the facet sampler
/// layer is responsible for populating the underlying store.
async fn node_facets(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<dto::metrics::FacetMetricsSummary>> {
    let summaries = state.facet_metrics.summaries_for_node(&id);
    Json(summaries)
}
