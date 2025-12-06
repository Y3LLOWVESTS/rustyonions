// crates/svc-admin/src/router.rs
//
// RO:WHAT — HTTP surface for svc-admin (health, metrics, API).
// RO:WHY  — Provide a small, well-defined admin/control-plane API for
//          operators and the SPA (nodes, metrics, identity, actions).
// RO:INTERACTS — state::AppState, dto::{ui,me,node,metrics}, auth,
//                metrics::prometheus_bridge, nodes::registry.
// RO:INVARIANTS —
//   - Read-only GET endpoints are always safe for untrusted callers.
//   - Control-plane actions (reload/shutdown) are gated by config + auth.
//   - No blocking operations; all IO is async via axum/reqwest.
//
// RO:METRICS/LOGS —
//   - Relies on Prometheus default registry via /metrics.
//   - Emits audit-ish logs on node actions.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};

use crate::{auth, dto, state::AppState};

/// Build the axum router for svc-admin.
///
/// This is used by server bootstrap for both the main UI/API and the
/// metrics/health listener (the latter only uses a subset of routes).
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
        // Control-plane actions (config + auth gated).
        .route("/api/nodes/:id/reload", post(node_reload))
        .route("/api/nodes/:id/shutdown", post(node_shutdown))
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
/// Uses the auth module to resolve Identity based on configured auth mode
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

/// Status view for a single node.
///
/// Returns 404 if the node id is not in the registry.
async fn node_status(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<dto::node::AdminStatusView>, StatusCode> {
    match state.nodes.get_status(&id).await {
        Some(view) => Ok(Json(view)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Facet metrics for a single node.
///
/// This pulls from the in-memory facet metrics store which is fed by the
/// sampler tasks.
async fn node_facets(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<dto::metrics::FacetMetricsSummary>> {
    let summaries = state.facet_metrics.summaries_for_node(&id);
    Json(summaries)
}

/// POST /api/nodes/{id}/reload
///
/// Gates:
///   - config.actions.enable_reload MUST be true
///   - caller MUST have role "admin" or "ops" (coarse-grained)
async fn node_reload(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<dto::node::NodeActionResponse>, StatusCode> {
    // Config gate: if reloads are disabled, pretend the endpoint does not exist.
    if !state.config.actions.enable_reload {
        return Err(StatusCode::NOT_FOUND);
    }

    // Auth gate.
    let auth_cfg = &state.config.auth;
    let identity =
        match auth::resolve_identity_from_headers(auth_cfg, &headers) {
            Ok(idn) => idn,
            Err(_err) => return Err(StatusCode::UNAUTHORIZED),
        };

    let allowed = identity
        .roles
        .iter()
        .any(|r| r == "admin" || r == "ops");
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    if !state.nodes.contains(&id) {
        return Err(StatusCode::NOT_FOUND);
    }

    match state.nodes.reload_node(&id).await {
        Ok(resp) => {
            tracing::info!(
                target: "svc_admin::audit",
                action = "reload",
                node_id = %resp.node_id,
                subject = %identity.subject,
                roles = ?identity.roles,
                "node reload requested"
            );
            Ok(Json(resp))
        }
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::audit",
                action = "reload",
                node_id = %id,
                subject = %identity.subject,
                roles = ?identity.roles,
                error = %err,
                "node reload failed"
            );
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

/// POST /api/nodes/{id}/shutdown
///
/// Gates:
///   - config.actions.enable_shutdown MUST be true
///   - caller MUST have role "admin" (stricter than reload)
async fn node_shutdown(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<dto::node::NodeActionResponse>, StatusCode> {
    // Config gate.
    if !state.config.actions.enable_shutdown {
        return Err(StatusCode::NOT_FOUND);
    }

    // Auth gate.
    let auth_cfg = &state.config.auth;
    let identity =
        match auth::resolve_identity_from_headers(auth_cfg, &headers) {
            Ok(idn) => idn,
            Err(_err) => return Err(StatusCode::UNAUTHORIZED),
        };

    let allowed = identity.roles.iter().any(|r| r == "admin");
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    if !state.nodes.contains(&id) {
        return Err(StatusCode::NOT_FOUND);
    }

    match state.nodes.shutdown_node(&id).await {
        Ok(resp) => {
            tracing::info!(
                target: "svc_admin::audit",
                action = "shutdown",
                node_id = %resp.node_id,
                subject = %identity.subject,
                roles = ?identity.roles,
                "node shutdown requested"
            );
            Ok(Json(resp))
        }
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::audit",
                action = "shutdown",
                node_id = %id,
                subject = %identity.subject,
                roles = ?identity.roles,
                error = %err,
                "node shutdown failed"
            );
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}
