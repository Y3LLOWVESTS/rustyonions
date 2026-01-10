// crates/svc-admin/src/router.rs
//
// RO:WHAT — HTTP surface for svc-admin (health, metrics, API).
// RO:WHY  — Provide a small, well-defined admin/control-plane API for
//           operators and the SPA (nodes, metrics, identity, actions).
// RO:INVARIANTS —
//   - Read-only GET endpoints are always safe for untrusted callers.
//   - Control-plane actions are gated by config + auth.
//   - No blocking operations; all IO is async via axum/reqwest.
// RO:CAPABILITY ROLLOUT —
//   - Node optional endpoints: svc-admin returns 501 when node lacks them,
//     enabling SPA to fall back to deterministic mocks without breaking.
// RO:TWO-PLANE —
//   - UI/API plane (e.g. :5300): SPA-facing JSON + control actions.
//   - Metrics plane (e.g. :5310): /healthz /readyz /metrics only.

#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};

use crate::{
    auth,
    auth::local as local_auth,
    dto,
    error::Error as SvcError,
    metrics::actions as action_metrics,
    state::AppState,
};

/// UI/API router (used by the main listener, e.g. :5300).
///
/// RO:INVARIANTS
/// - Must NOT expose /metrics on this plane.
/// - Health/ready/metrics live on the metrics listener (e.g. :5310).
///
/// IMPORTANT (Axum 0.7):
/// - This function returns a *finished* Router (alias), not `Router<Arc<AppState>>`.
/// - Internally we first build a “missing-state” router that uses `State<Arc<AppState>>`
///   in handlers, then we call `.with_state(state)` to produce the final Router
///   that `axum::serve` can accept.
pub fn build_router(state: Arc<AppState>) -> Router {
    let local_gate = middleware::from_fn_with_state(state.clone(), local_api_gate);

    Router::new()
        // -----------------------------
        // Auth routes (local mode)
        // -----------------------------
        .route("/api/auth/login", post(auth_login))
        .route("/api/auth/logout", post(auth_logout))
        .route("/api/auth/me", get(auth_me))
        // Versioned aliases
        .route("/api/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/logout", post(auth_logout))
        .route("/api/v1/auth/me", get(auth_me))
        // -----------------------------
        // Current (non-versioned) API
        // -----------------------------
        .route("/api/ui-config", get(ui_config))
        .route("/api/me", get(me))
        .route("/api/nodes", get(nodes))
        .route("/api/nodes/:id/status", get(node_status))
        .route("/api/nodes/:id/metrics/facets", get(node_facets))
        // System summary (optional rollout)
        .route("/api/nodes/:id/system/summary", get(node_system_summary))
        // ✅ Network accounting (optional rollout)
        .route(
            "/api/nodes/:id/system/net/accounting",
            get(node_system_net_accounting),
        )
        // Storage / DB inventory (optional rollout)
        .route("/api/nodes/:id/storage/summary", get(node_storage_summary))
        .route(
            "/api/nodes/:id/storage/databases",
            get(node_storage_databases),
        )
        .route(
            "/api/nodes/:id/storage/databases/:name",
            get(node_storage_database_detail),
        )
        // Benchmarks (optional rollout; node-executed)
        .route("/api/nodes/:id/bench/run", post(node_bench_run))
        .route("/api/nodes/:id/bench/runs/:run_id", get(node_bench_status))
        .route(
            "/api/nodes/:id/bench/runs/:run_id/result",
            get(node_bench_result),
        )
        // Dev-only App Playground (gated by ui.dev.enable_app_playground).
        .route("/api/playground/examples", get(playground_examples))
        .route(
            "/api/playground/manifest/validate",
            post(playground_validate_manifest),
        )
        // Control-plane actions (config + auth gated).
        .route("/api/nodes/:id/reload", post(node_reload))
        .route("/api/nodes/:id/shutdown", post(node_shutdown))
        .route("/api/nodes/:id/debug/crash", post(node_debug_crash))
        // -----------------------------
        // Versioned aliases (/api/v1/*)
        // -----------------------------
        .route("/api/v1/ui-config", get(ui_config))
        .route("/api/v1/me", get(me))
        .route("/api/v1/nodes", get(nodes))
        .route("/api/v1/nodes/:id/status", get(node_status))
        .route("/api/v1/nodes/:id/metrics/facets", get(node_facets))
        .route("/api/v1/nodes/:id/system/summary", get(node_system_summary))
        // ✅ Network accounting (optional rollout)
        .route(
            "/api/v1/nodes/:id/system/net/accounting",
            get(node_system_net_accounting),
        )
        .route("/api/v1/nodes/:id/storage/summary", get(node_storage_summary))
        .route(
            "/api/v1/nodes/:id/storage/databases",
            get(node_storage_databases),
        )
        .route(
            "/api/v1/nodes/:id/storage/databases/:name",
            get(node_storage_database_detail),
        )
        .route("/api/v1/nodes/:id/bench/run", post(node_bench_run))
        .route(
            "/api/v1/nodes/:id/bench/runs/:run_id",
            get(node_bench_status),
        )
        .route(
            "/api/v1/nodes/:id/bench/runs/:run_id/result",
            get(node_bench_result),
        )
        .route("/api/v1/playground/examples", get(playground_examples))
        .route(
            "/api/v1/playground/manifest/validate",
            post(playground_validate_manifest),
        )
        .route("/api/v1/nodes/:id/reload", post(node_reload))
        .route("/api/v1/nodes/:id/shutdown", post(node_shutdown))
        .route("/api/v1/nodes/:id/debug/crash", post(node_debug_crash))
        // Local-mode gate: protect /api/* (except allowlisted endpoints).
        .layer(local_gate)
        // IMPORTANT: this finalizes the router into the `Router` alias type
        // that `axum::serve` expects.
        .with_state(state)
}

/// Metrics/health-only router (use this for the metrics listener, e.g. :5310).
///
/// RO:INVARIANTS
/// - Must NOT expose any UI or /api/* routes.
/// - Safe to bind more broadly than the main UI/API listener (still recommended to protect in prod).
///
/// As with `build_router`, this returns a finished `Router` alias.
pub fn build_metrics_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route(
            "/metrics",
            get(crate::metrics::prometheus_bridge::metrics_handler),
        )
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ready": true }))
}

async fn ui_config(State(state): State<Arc<AppState>>) -> Json<dto::ui::UiConfigDto> {
    Json(dto::ui::UiConfigDto::from_cfg(&state.config))
}

// -----------------------------------------------------------------------------
// Local-mode API gate (middleware)
// -----------------------------------------------------------------------------

fn is_allowlisted_local_api_path(path: &str) -> bool {
    matches!(
        path,
        "/api/ui-config"
            | "/api/auth/login"
            | "/api/auth/logout"
            | "/api/auth/me"
            | "/api/v1/ui-config"
            | "/api/v1/auth/login"
            | "/api/v1/auth/logout"
            | "/api/v1/auth/me"
    )
}

/// If auth.mode == "local", require a valid session cookie for /api/* routes,
/// except a small allowlist needed for boot/login.
///
/// Also injects an Identity into request extensions for downstream handlers.
async fn local_api_gate(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    // Only gate in local mode.
    if state.config.auth.mode != "local" {
        return next.run(req).await;
    }

    let path = req.uri().path();

    // Only gate API routes; allow SPA/static routes to flow.
    if !path.starts_with("/api/") {
        return next.run(req).await;
    }

    // Allow a minimal unauth surface.
    if is_allowlisted_local_api_path(path) {
        return next.run(req).await;
    }

    let Some(local) = state.local_auth.as_ref() else {
        // Misconfiguration: local mode declared but backend not initialized.
        tracing::error!(
            target: "svc_admin::auth",
            mode = "local",
            "local auth requested but local_auth state is missing"
        );
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    match local.authenticate_headers(req.headers()) {
        Ok((Some(ctx), maybe_set_cookie)) => {
            // Inject a coarse Identity so existing handlers can stay role-based.
            let idn = auth::Identity {
                subject: ctx.username.clone(),
                display_name: ctx.username.clone(),
                roles: ctx.roles.clone(),
            };
            req.extensions_mut().insert(idn);

            let mut resp = next.run(req).await;

            // Optionally refresh the cookie to keep sessions warm.
            if let Some(sc) = maybe_set_cookie {
                if let Ok(v) = sc.parse() {
                    resp.headers_mut().append(header::SET_COOKIE, v);
                }
            }

            resp
        }
        Ok((None, _)) => StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => e.into_response(),
    }
}

// -----------------------------------------------------------------------------
// Auth endpoints (mounted in main router; active only in local mode)
// -----------------------------------------------------------------------------

async fn auth_login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<local_auth::LoginRequest>,
) -> Result<(StatusCode, HeaderMap, Json<local_auth::MeResponse>), StatusCode> {
    if state.config.auth.mode != "local" {
        return Err(StatusCode::NOT_FOUND);
    }
    let Some(local) = state.local_auth.clone() else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    local_auth::login(State(local), Json(req))
        .await
        .map_err(|e| e.into_response().status())
}

async fn auth_logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<(StatusCode, HeaderMap), StatusCode> {
    if state.config.auth.mode != "local" {
        return Err(StatusCode::NOT_FOUND);
    }
    let Some(local) = state.local_auth.clone() else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    local_auth::logout(State(local), headers)
        .await
        .map_err(|e| e.into_response().status())
}

async fn auth_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<local_auth::MeResponse>), StatusCode> {
    if state.config.auth.mode != "local" {
        return Err(StatusCode::NOT_FOUND);
    }
    let Some(local) = state.local_auth.clone() else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    local_auth::me(State(local), headers)
        .await
        .map_err(|e| e.into_response().status())
}

// -----------------------------------------------------------------------------
// Identity helpers
// -----------------------------------------------------------------------------

fn resolve_identity_ui(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    ext: Option<&auth::Identity>,
) -> Result<auth::Identity, StatusCode> {
    let auth_cfg = &state.config.auth;

    if auth_cfg.mode == "local" {
        // In local mode, identity must come from the local_api_gate extension.
        return ext.cloned().ok_or(StatusCode::UNAUTHORIZED);
    }

    match auth::resolve_identity_from_headers(auth_cfg, headers) {
        Ok(idn) => Ok(idn),
        Err(err) => {
            action_metrics::inc_auth_failure("ui");
            tracing::warn!(
                target: "svc_admin::auth",
                scope = "ui",
                mode = %auth_cfg.mode,
                error = ?err,
                "failed to resolve identity for /api/me; falling back to dev identity"
            );
            Ok(auth::Identity::dev_fallback())
        }
    }
}

fn resolve_identity_node_or_unauth(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    ext: Option<&auth::Identity>,
    action: &'static str,
    node_id: &str,
) -> Result<auth::Identity, StatusCode> {
    let auth_cfg = &state.config.auth;

    if auth_cfg.mode == "local" {
        return ext.cloned().ok_or(StatusCode::UNAUTHORIZED);
    }

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

fn map_registry_err(err: &SvcError) -> StatusCode {
    match err {
        SvcError::Config(_) => StatusCode::BAD_REQUEST,
        SvcError::Serde(_) => StatusCode::BAD_GATEWAY,
        SvcError::Http(_) => StatusCode::BAD_GATEWAY,
        SvcError::Upstream(_) => StatusCode::BAD_GATEWAY,
        SvcError::UpstreamStatus { .. } => StatusCode::BAD_GATEWAY,
        SvcError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        SvcError::Auth(_) => StatusCode::UNAUTHORIZED,
        SvcError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn map_bench_err(err: &SvcError) -> StatusCode {
    match err {
        SvcError::UpstreamStatus { status: 404, .. } => StatusCode::NOT_FOUND,
        SvcError::UpstreamStatus { status: 400, .. } => StatusCode::BAD_REQUEST,
        _ => map_registry_err(err),
    }
}

async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(ext_idn): Extension<Option<auth::Identity>>,
) -> Result<Json<dto::me::MeResponse>, StatusCode> {
    let auth_cfg = &state.config.auth;
    let identity = resolve_identity_ui(&state, &headers, ext_idn.as_ref())?;
    Ok(Json(dto::me::MeResponse::from_identity(identity, auth_cfg)))
}

async fn nodes(State(state): State<Arc<AppState>>) -> Json<Vec<dto::node::NodeSummary>> {
    Json(state.nodes.list_summaries())
}

async fn node_system_summary(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<dto::system::SystemSummaryDto>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    match state.nodes.try_system_summary(&id).await {
        Ok(Some(dto)) => Ok(Json(dto)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::system",
                node_id = %id,
                error = %err,
                "failed to fetch system summary from node admin plane"
            );
            Err(map_registry_err(&err))
        }
    }
}

async fn node_system_net_accounting(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    match state.nodes.try_system_net_accounting(&id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::system",
                node_id = %id,
                error = %err,
                "failed to fetch system net accounting from node admin plane"
            );
            Err(map_registry_err(&err))
        }
    }
}

async fn node_status(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<dto::node::AdminStatusView>, StatusCode> {
    match state.nodes.get_status(&id).await {
        Some(view) => Ok(Json(view)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn node_facets(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<dto::metrics::FacetMetricsSummary>> {
    Json(state.facet_metrics.summaries_for_node(&id))
}

async fn node_storage_summary(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<dto::storage::StorageSummaryDto>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    match state.nodes.try_storage_summary(&id).await {
        Ok(Some(dto)) => Ok(Json(dto)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::storage",
                node_id = %id,
                error = %err,
                "failed to fetch storage summary from node admin plane"
            );
            Err(map_registry_err(&err))
        }
    }
}

async fn node_storage_databases(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<dto::storage::DatabaseEntryDto>>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    match state.nodes.try_storage_databases(&id).await {
        Ok(Some(list)) => Ok(Json(list)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::storage",
                node_id = %id,
                error = %err,
                "failed to fetch storage databases list from node admin plane"
            );
            Err(map_registry_err(&err))
        }
    }
}

async fn node_storage_database_detail(
    Path((id, name)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<dto::storage::DatabaseDetailDto>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    match state.nodes.try_storage_database_detail(&id, &name).await {
        Ok(Some(dto)) => Ok(Json(dto)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::storage",
                node_id = %id,
                db = %name,
                error = %err,
                "failed to fetch storage database detail from node admin plane"
            );
            Err(map_registry_err(&err))
        }
    }
}

fn require_ops_or_admin(identity: &auth::Identity) -> bool {
    identity.roles.iter().any(|r| r == "admin" || r == "ops")
}

async fn node_bench_run(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(ext_idn): Extension<Option<auth::Identity>>,
    Json(req): Json<dto::bench::BenchRunReqDto>,
) -> Result<Json<dto::bench::BenchRunRespDto>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    let identity =
        resolve_identity_node_or_unauth(&state, &headers, ext_idn.as_ref(), "bench_run", &id)?;
    if !require_ops_or_admin(&identity) {
        action_metrics::inc_rejection("forbidden");
        return Err(StatusCode::FORBIDDEN);
    }

    match state.nodes.try_bench_run(&id, &req).await {
        Ok(Some(resp)) => Ok(Json(resp)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::bench",
                node_id = %id,
                error = %err,
                "bench run proxy failed"
            );
            Err(map_bench_err(&err))
        }
    }
}

async fn node_bench_status(
    Path((id, run_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(ext_idn): Extension<Option<auth::Identity>>,
) -> Result<Json<dto::bench::BenchRunStatusDto>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    let identity = resolve_identity_node_or_unauth(
        &state,
        &headers,
        ext_idn.as_ref(),
        "bench_status",
        &id,
    )?;
    if !require_ops_or_admin(&identity) {
        action_metrics::inc_rejection("forbidden");
        return Err(StatusCode::FORBIDDEN);
    }

    match state.nodes.try_bench_status(&id, &run_id).await {
        Ok(Some(dto)) => Ok(Json(dto)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => Err(map_bench_err(&err)),
    }
}

async fn node_bench_result(
    Path((id, run_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(ext_idn): Extension<Option<auth::Identity>>,
) -> Result<Json<dto::bench::BenchRunResultDto>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    let identity = resolve_identity_node_or_unauth(
        &state,
        &headers,
        ext_idn.as_ref(),
        "bench_result",
        &id,
    )?;
    if !require_ops_or_admin(&identity) {
        action_metrics::inc_rejection("forbidden");
        return Err(StatusCode::FORBIDDEN);
    }

    match state.nodes.try_bench_result(&id, &run_id).await {
        Ok(Some(dto)) => Ok(Json(dto)),
        Ok(None) => Err(StatusCode::NOT_IMPLEMENTED),
        Err(err) => Err(map_bench_err(&err)),
    }
}

// -----------------------------------------------------------------------------
// Dev-only App Playground
// -----------------------------------------------------------------------------

fn playground_enabled(state: &Arc<AppState>) -> bool {
    state.config.ui.dev.enable_app_playground
}

fn ensure_playground_enabled(state: &Arc<AppState>) -> Result<(), StatusCode> {
    if playground_enabled(state) {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaygroundExampleDto {
    id: String,
    title: String,
    description: String,
    manifest_toml: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaygroundValidateManifestReq {
    manifest_toml: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaygroundValidateManifestResp {
    ok: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    parsed: Option<serde_json::Value>,
}

async fn playground_examples(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PlaygroundExampleDto>>, StatusCode> {
    ensure_playground_enabled(&state)?;

    let examples = vec![
        PlaygroundExampleDto {
            id: "hello-world".to_string(),
            title: "Hello World Facet".to_string(),
            description: "Minimal manifest with a single GET route.".to_string(),
            manifest_toml: r#"[package]
name = "hello-world"
version = "0.1.0"

[facet]
kind = "http"
description = "Hello world facet"

[routes]
"/hello" = { method = "GET", response = "Hello from RON-CORE" }
"#
            .to_string(),
        },
        PlaygroundExampleDto {
            id: "echo-json".to_string(),
            title: "Echo JSON Facet".to_string(),
            description: "Shows POST + JSON (validation only in playground MVP).".to_string(),
            manifest_toml: r#"[package]
name = "echo-json"
version = "0.1.0"

[facet]
kind = "http"
description = "Echo JSON payloads"

[routes]
"/echo" = { method = "POST", content_type = "application/json" }
"#
            .to_string(),
        },
    ];

    Ok(Json(examples))
}

async fn playground_validate_manifest(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PlaygroundValidateManifestReq>,
) -> Result<Json<PlaygroundValidateManifestResp>, StatusCode> {
    ensure_playground_enabled(&state)?;

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let parsed_toml: Option<toml::Value> = match toml::from_str::<toml::Value>(&body.manifest_toml)
    {
        Ok(v) => Some(v),
        Err(e) => {
            errors.push(format!("TOML parse error: {e}"));
            None
        }
    };

    let parsed_json = if let Some(v) = &parsed_toml {
        match serde_json::to_value(v) {
            Ok(j) => Some(j),
            Err(e) => {
                errors.push(format!("Could not convert parsed TOML to JSON: {e}"));
                None
            }
        }
    } else {
        None
    };

    if let Some(v) = parsed_toml {
        let pkg = v.get("package").and_then(|x| x.as_table());

        let pkg_name_ok = pkg
            .and_then(|t| t.get("name"))
            .and_then(|x| x.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        let pkg_ver_ok = pkg
            .and_then(|t| t.get("version"))
            .and_then(|x| x.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !pkg_name_ok {
            errors.push("Missing required [package].name".to_string());
        }
        if !pkg_ver_ok {
            errors.push("Missing required [package].version".to_string());
        }

        let has_routes = v.get("routes").and_then(|x| x.as_table()).is_some();
        if !has_routes {
            warnings.push("No [routes] table found (expected for HTTP facets).".to_string());
        }

        let has_facet = v.get("facet").and_then(|x| x.as_table()).is_some();
        if !has_facet {
            warnings.push("No [facet] table found (metadata recommended).".to_string());
        }
    }

    Ok(Json(PlaygroundValidateManifestResp {
        ok: errors.is_empty(),
        errors,
        warnings,
        parsed: parsed_json,
    }))
}

#[derive(Debug, serde::Deserialize)]
struct DebugCrashRequest {
    service: Option<String>,
}

async fn node_debug_crash(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(ext_idn): Extension<Option<auth::Identity>>,
    Json(body): Json<DebugCrashRequest>,
) -> Result<Json<dto::node::NodeActionResponse>, StatusCode> {
    ensure_node_exists(&state, &id)?;

    let identity =
        resolve_identity_node_or_unauth(&state, &headers, ext_idn.as_ref(), "debug_crash", &id)?;
    let allowed = identity.roles.iter().any(|r| r == "admin");
    if !allowed {
        action_metrics::inc_rejection("forbidden");
        return Err(StatusCode::FORBIDDEN);
    }

    match state.nodes.debug_crash_node(&id, body.service).await {
        Ok(resp) => {
            tracing::info!(
                target: "svc_admin::audit",
                action = "debug_crash",
                node_id = %resp.node_id,
                subject = %identity.subject,
                roles = ?identity.roles,
                "debug crash forwarded to node admin plane"
            );
            Ok(Json(resp))
        }
        Err(err) => {
            tracing::warn!(
                target: "svc_admin::audit",
                action = "debug_crash",
                node_id = %id,
                subject = %identity.subject,
                roles = ?identity.roles,
                error = %err,
                "debug crash proxy failed"
            );
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

async fn node_reload(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(ext_idn): Extension<Option<auth::Identity>>,
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

    let identity =
        resolve_identity_node_or_unauth(&state, &headers, ext_idn.as_ref(), "reload", &id)?;

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

async fn node_shutdown(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(ext_idn): Extension<Option<auth::Identity>>,
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

    let identity =
        resolve_identity_node_or_unauth(&state, &headers, ext_idn.as_ref(), "shutdown", &id)?;

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
