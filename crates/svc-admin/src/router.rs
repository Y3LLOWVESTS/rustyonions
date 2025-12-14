// crates/svc-admin/src/router.rs
//
// RO:WHAT — HTTP surface for svc-admin (health, metrics, API).
// RO:WHY  — Provide a small, well-defined admin/control-plane API for
//           operators and the SPA (nodes, metrics, identity, actions).
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
//   - Phase 1: governance/auth metrics for actions and /api/me.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};

use crate::{auth, dto, metrics::actions as action_metrics, state::AppState};

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
        // Storage / DB inventory (read-only; node support rolls out behind capability).
        .route("/api/nodes/:id/storage/summary", get(node_storage_summary))
        .route("/api/nodes/:id/storage/databases", get(node_storage_databases))
        .route(
            "/api/nodes/:id/storage/databases/:name",
            get(node_storage_database_detail),
        )
        // Control-plane actions (config + auth gated).
        .route("/api/nodes/:id/reload", post(node_reload))
        .route("/api/nodes/:id/shutdown", post(node_shutdown))
        // Dev-only debug hook: synthetic crash for a node's service/plane.
        .route("/api/nodes/:id/debug/crash", post(node_debug_crash))
        .with_state(state)
}

/// Liveness probe.
///
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

/// Resolve identity for UI scope.
///
/// Phase 1 behavior:
/// - If auth fails, increment auth failure metric and fall back to dev identity
///   to keep the UI usable.
fn resolve_identity_ui(state: &Arc<AppState>, headers: &HeaderMap) -> auth::Identity {
    let auth_cfg = &state.config.auth;
    match auth::resolve_identity_from_headers(auth_cfg, headers) {
        Ok(idn) => idn,
        Err(err) => {
            action_metrics::inc_auth_failure("ui");
            tracing::warn!(
                target: "svc_admin::auth",
                scope = "ui",
                mode = %auth_cfg.mode,
                error = ?err,
                "failed to resolve identity for /api/me; falling back to dev identity"
            );
            auth::Identity::dev_fallback()
        }
    }
}

/// Resolve identity for node/action scope.
///
/// Phase 1 behavior:
/// - If auth fails: increment auth failure + rejection("unauth") and return 401.
fn resolve_identity_node_or_unauth(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    action: &'static str,
    node_id: &str,
) -> Result<auth::Identity, StatusCode> {
    let auth_cfg = &state.config.auth;
    match auth::resolve_identity_from_headers(auth_cfg, headers) {
        Ok(idn) => Ok(idn),
        Err(err) => {
            action_metrics::inc_auth_failure("node");
            action_metrics::inc_rejection("unauth");
            tracing::warn!(
                target: "svc_admin::auth",
                action,
                node_id = %node_id,
                mode = %auth_cfg.mode,
                error = ?err,
                "action rejected: failed to resolve identity"
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn ensure_node_exists(state: &Arc<AppState>, id: &str) -> Result<(), StatusCode> {
    if state.nodes.contains(id) {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Identity endpoint for the current user.
async fn me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Json<dto::me::MeResponse> {
    let auth_cfg = &state.config.auth;
    let identity = resolve_identity_ui(&state, &headers);
    Json(dto::me::MeResponse::from_identity(identity, auth_cfg))
}

/// List all configured nodes as NodeSummary DTOs.
async fn nodes(State(state): State<Arc<AppState>>) -> Json<Vec<dto::node::NodeSummary>> {
    Json(state.nodes.list_summaries())
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
async fn node_facets(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<dto::metrics::FacetMetricsSummary>> {
    Json(state.facet_metrics.summaries_for_node(&id))
}

/// GET /api/nodes/{id}/storage/summary
///
/// Slice 3e: route scaffold only.
/// - 404 if node id not registered
/// - 501 until node/admin-plane + svc-admin wiring exists
async fn node_storage_summary(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    ensure_node_exists(&state, &id)?;
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/nodes/{id}/storage/databases
async fn node_storage_databases(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    ensure_node_exists(&state, &id)?;
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/nodes/{id}/storage/databases/{name}
async fn node_storage_database_detail(
    Path((id, _name)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    ensure_node_exists(&state, &id)?;
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Request body for debug crash proxy.
///
/// SPA sends `{ "service": "svc-storage" }` (field optional).
#[derive(Debug, serde::Deserialize)]
struct DebugCrashRequest {
    service: Option<String>,
}

/// POST /api/nodes/{id}/debug/crash
async fn node_debug_crash(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<DebugCrashRequest>,
) -> Result<Json<dto::node::NodeActionResponse>, StatusCode> {
    if !state.nodes.contains(&id) {
        tracing::info!(
            target: "svc_admin::audit",
            action = "debug_crash",
            node_id = %id,
            reason = "node_not_found",
            "debug crash rejected: unknown node id"
        );
        return Err(StatusCode::NOT_FOUND);
    }

    match state.nodes.debug_crash_node(&id, body.service).await {
        Ok(resp) => {
            tracing::info!(
                target: "svc_admin::audit",
                action = "debug_crash",
                node_id = %resp.node_id,
                "debug crash forwarded to node admin plane"
            );
            Ok(Json(resp))
        }
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::audit",
                action = "debug_crash",
                node_id = %id,
                error = %err,
                "debug crash proxy failed"
            );
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

/// POST /api/nodes/{id}/reload
async fn node_reload(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<dto::node::NodeActionResponse>, StatusCode> {
    if !state.config.actions.enable_reload {
        action_metrics::inc_rejection("disabled");
        tracing::warn!(
            target: "svc_admin::audit",
            action = "reload",
            node_id = %id,
            reason = "disabled",
            "node reload rejected: action disabled in config"
        );
        return Err(StatusCode::NOT_FOUND);
    }

    let identity = resolve_identity_node_or_unauth(&state, &headers, "reload", &id)?;

    let allowed = identity.roles.iter().any(|r| r == "admin" || r == "ops");
    if !allowed {
        action_metrics::inc_rejection("forbidden");
        tracing::warn!(
            target: "svc_admin::audit",
            action = "reload",
            node_id = %id,
            subject = %identity.subject,
            roles = ?identity.roles,
            "node reload rejected: forbidden (missing role)"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    if !state.nodes.contains(&id) {
        action_metrics::inc_rejection("node_not_found");
        tracing::info!(
            target: "svc_admin::audit",
            action = "reload",
            node_id = %id,
            subject = %identity.subject,
            roles = ?identity.roles,
            "node reload rejected: unknown node id"
        );
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
            action_metrics::inc_rejection("upstream_error");
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
async fn node_shutdown(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<dto::node::NodeActionResponse>, StatusCode> {
    if !state.config.actions.enable_shutdown {
        action_metrics::inc_rejection("disabled");
        tracing::warn!(
            target: "svc_admin::audit",
            action = "shutdown",
            node_id = %id,
            reason = "disabled",
            "node shutdown rejected: action disabled in config"
        );
        return Err(StatusCode::NOT_FOUND);
    }

    let identity = resolve_identity_node_or_unauth(&state, &headers, "shutdown", &id)?;

    let allowed = identity.roles.iter().any(|r| r == "admin");
    if !allowed {
        action_metrics::inc_rejection("forbidden");
        tracing::warn!(
            target: "svc_admin::audit",
            action = "shutdown",
            node_id = %id,
            subject = %identity.subject,
            roles = ?identity.roles,
            "node shutdown rejected: forbidden (missing role)"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    if !state.nodes.contains(&id) {
        action_metrics::inc_rejection("node_not_found");
        tracing::info!(
            target: "svc_admin::audit",
            action = "shutdown",
            node_id = %id,
            subject = %identity.subject,
            roles = ?identity.roles,
            "node shutdown rejected: unknown node id"
        );
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
            action_metrics::inc_rejection("upstream_error");
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
