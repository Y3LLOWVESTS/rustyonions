//! RO:WHAT  — v1 App plane router.
//! RO:WHY   — Reserve `/v1/app/*` and provide a minimal health probe plus an
//!            OPTIONAL generic static-site mount for early demos/integration.
//!
//! RO:INVARS
//!   - Do NOT embed demo content in code (no "default HTML").
//!   - Do NOT expose internal content IDs / hashes.
//!   - Keep this module about HTTP shape + thin glue only.
//!   - The static-site mount is OPTIONAL and config-driven (env), and is
//!     intentionally generic so it can be reused beyond the WOW demo.
//!
//! FUTURE:
//!   - Replace/extend with real `RonApp` mounts via `ron-app-sdk` AppContract.
//!   - Enforce capability/auth extraction before invoking app handlers.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::{
    path::{Component, Path as FsPath, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

/// Simple health payload for the app plane.
#[derive(Debug, Serialize)]
struct AppPlaneHealth {
    /// Always `true` for now; later we can surface per-app readiness.
    ok: bool,
    /// Human-readable note for debugging / integration tests.
    note: &'static str,
}

/// JSON response for a "reload" action (generation bump).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReloadResponse {
    ok: bool,
    generation: u64,
}

/// Shared state for app-plane optional mounts (kept minimal).
#[derive(Clone)]
struct AppPlaneState {
    static_dir: Option<PathBuf>,
}

impl AppPlaneState {
    fn from_env() -> Self {
        let static_dir = std::env::var("OMNIGATE_APP_STATIC_DIR")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        Self { static_dir }
    }
}

/// A simple generation counter that UIs can observe after "reload".
static SITE_RELOAD_GEN: AtomicU64 = AtomicU64::new(0);

/// Build the `/v1/app/*` routing tree.
///
/// Always exposes:
///   - `GET /v1/app/healthz`
///
/// Optionally (if `OMNIGATE_APP_STATIC_DIR` is set):
///   - `GET  /v1/app/site/*path`     (serves static files from disk)
///   - `POST /v1/app/site/reload`    (increments generation counter)
///
/// NOTE: This does not embed any demo content. If files don’t exist, we return 404.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let st = Arc::new(AppPlaneState::from_env());

    Router::new()
        .route("/healthz", get(get_health))
        // Static-site mount (optional)
        .route("/site/reload", post(site_reload))
        .route("/site", get(site_index))
        .route("/site/", get(site_index))
        .route("/site/*path", get(site_serve))
        .with_state(st)
}

/// `GET /v1/app/healthz`
///
/// Minimal "is the app plane wired?" probe.
async fn get_health() -> Json<AppPlaneHealth> {
    Json(AppPlaneHealth {
        ok: true,
        note: "app plane mounted",
    })
}

/// `POST /v1/app/site/reload`
///
/// Generic reload semantics for static-site hosting:
/// increments a generation counter. The site UI can call this, then re-fetch
/// `/v1/app/site` (or specific assets) and display the generation number.
///
/// IMPORTANT: Serving from disk already reflects changes immediately; the "reload"
/// endpoint exists so an operator action is *observable* (metrics tick + gen bump).
async fn site_reload(State(_st): State<Arc<AppPlaneState>>) -> impl IntoResponse {
    // Even if static_dir isn't set, we still treat it as not-found to avoid
    // advertising routes when unused.
    if !static_enabled() {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }

    let gen = SITE_RELOAD_GEN.fetch_add(1, Ordering::Relaxed) + 1;
    Json(ReloadResponse {
        ok: true,
        generation: gen,
    })
    .into_response()
}

/// `GET /v1/app/site`
///
/// Convenience: serve index.html.
async fn site_index(State(st): State<Arc<AppPlaneState>>) -> Response {
    if !static_enabled() {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }

    let Some(root) = &st.static_dir else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };

    let path = root.join("index.html");
    serve_file(&path)
}

/// `GET /v1/app/site/*path`
///
/// Serve a file from `OMNIGATE_APP_STATIC_DIR` safely.
/// - No traversal (`..`) or absolute paths.
/// - If `*path` is empty, serves index.html.
async fn site_serve(State(st): State<Arc<AppPlaneState>>, Path(path): Path<String>) -> Response {
    if !static_enabled() {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }

    let Some(root) = &st.static_dir else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };

    let rel = if path.trim().is_empty() {
        "index.html".to_string()
    } else {
        path
    };

    let Some(safe_rel) = sanitize_rel_path(&rel) else {
        return (StatusCode::BAD_REQUEST, "invalid path").into_response();
    };

    let full = root.join(safe_rel);
    serve_file(&full)
}

// --------------------------- helpers ---------------------------

fn static_enabled() -> bool {
    matches!(
        std::env::var("OMNIGATE_APP_STATIC_DIR").as_deref(),
        Ok(v) if !v.trim().is_empty()
    )
}

/// Ensure the path is a safe relative path:
/// - must not be absolute
/// - must not contain `..` or Windows prefixes
/// - must not contain backslashes
fn sanitize_rel_path(s: &str) -> Option<PathBuf> {
    if s.contains('\\') {
        return None;
    }
    let p = PathBuf::from(s);

    // Reject absolute paths
    if p.is_absolute() {
        return None;
    }

    // Reject traversal and weird components
    for c in p.components() {
        match c {
            Component::Normal(_) => {}
            Component::CurDir => {}
            // Disallow parent dir and prefixes/root
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => return None,
        }
    }

    Some(p)
}

fn serve_file(path: &FsPath) -> Response {
    // Do not leak filesystem paths in responses.
    let Ok(bytes) = std::fs::read(path) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };

    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;

    // Content-Type best-effort
    let ct = content_type_for_path(path);
    resp.headers_mut().insert(header::CONTENT_TYPE, ct);

    // Dev-friendly: discourage caching so edits are visible immediately.
    resp.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

    // Observable "reload generation" (so the UI can display it if desired).
    let gen = SITE_RELOAD_GEN.load(Ordering::Relaxed);
    if let Ok(v) = HeaderValue::from_str(&gen.to_string()) {
        resp.headers_mut().insert("x-ron-reload-gen", v);
    }

    resp
}

fn content_type_for_path(p: &FsPath) -> HeaderValue {
    match p.extension().and_then(|s| s.to_str()).unwrap_or_default() {
        "html" => HeaderValue::from_static("text/html; charset=utf-8"),
        "css" => HeaderValue::from_static("text/css; charset=utf-8"),
        "js" => HeaderValue::from_static("application/javascript; charset=utf-8"),
        "json" => HeaderValue::from_static("application/json; charset=utf-8"),
        "svg" => HeaderValue::from_static("image/svg+xml; charset=utf-8"),
        "png" => HeaderValue::from_static("image/png"),
        "jpg" | "jpeg" => HeaderValue::from_static("image/jpeg"),
        "txt" => HeaderValue::from_static("text/plain; charset=utf-8"),
        _ => HeaderValue::from_static("application/octet-stream"),
    }
}
