//! RO:WHAT — Facet surface composition and demo/meta handlers.
//! RO:WHY  — Centralize facet mounting (demo + manifest-driven) and meta truth.
//! RO:INTERACTS — config::schema::SecurityMode, layers::security::RequireAuthLayer,
//!                facets::{loader,manifest}, axum Router.
//! RO:INVARIANTS —
//!   - `mount()` is safe to call when facets are disabled or loader fails.
//!   - `mount_with_registry()` mounts manifest-driven routes under
//!       `/facets/{facet_id}{route.path}`
//!     and exposes truthful `GET /facets/meta` and `GET /facets/_meta`.
//!   - Manifest `route.path` values are validated to start with `/`.
//!   - In `SecurityMode::DevAllow`, manifest facets are *not* gated by auth.
//!   - In stricter modes (DenyAll/External), manifest facets are gated via
//!     `RequireAuthLayer`.
//! RO:SECURITY —
//!   - Static facets: dev-friendly in `DevAllow`, gated in stricter modes.
//!   - Echo/proxy facets: also dev-open in `DevAllow`, gated otherwise.
//! RO:TEST — Covered by integration tests in `tests/facets_loader.rs`,
//!           `tests/facets_proxy.rs`, and the global auth-gate tests.

use crate::config::schema::SecurityMode;
use crate::layers::security::RequireAuthLayer;
use axum::{
    body::Body,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

pub mod loader;
pub mod manifest;

use loader::FacetRegistry;
use manifest::{FacetKind, FacetManifest};

/// Mount demo facet + empty meta when loader is disabled or empty.
/// Generic over router state `S` (must satisfy Axum’s Router bounds).
///
/// Used by `app::build_router` when:
///   - facets are disabled, or
///   - loader fails / dir missing.
///
/// This keeps a *predictable* surface for operators:
///   - `GET /facets/demo/ping`
///   - `GET /facets/meta` and `GET /facets/_meta` (with `loaded: []`)
pub fn mount<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .route("/facets/demo/ping", get(demo_ping))
        .route("/facets/meta", get(meta_empty))
        .route("/facets/_meta", get(meta_empty))
}

/// Mount facets using a concrete registry.
///
/// Generic over router state `S`.
///
/// Behavior:
///   - Always mounts truthful meta endpoints:
///       * `GET /facets/meta`
///       * `GET /facets/_meta` (legacy/alt alias)
///   - Mounts each manifest’s routes under `/facets/{facet_id}{route.path}`.
///   - Gating:
///       * `SecurityMode::DevAllow`  => facets are *not* wrapped in
///         `RequireAuthLayer` (dev-friendly).
///       * Any other mode (DenyAll/External/…) => facets are wrapped in
///         `RequireAuthLayer`, mirroring KV semantics.
///
/// This is what the `facets_loader` and `facets_proxy` integration tests exercise.
pub fn mount_with_registry<S>(
    mut router: Router<S>,
    reg: FacetRegistry,
    mode: SecurityMode,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // If nothing is loaded, fall back to demo + empty meta for sanity.
    if reg.manifests.is_empty() {
        return mount(router);
    }

    // Truthful meta endpoint, listing all loaded facets.
    //
    // NOTE: We capture the summary at mount time — good enough for current
    // design where manifests are loaded once at boot.
    let meta = Meta { loaded: reg.manifests.iter().map(FacetSummary::from).collect() };

    // Expose both `/facets/meta` and `/facets/_meta` using the same payload.
    router = router
        .route(
            "/facets/meta",
            get({
                let meta = meta.clone();
                move || {
                    let meta = meta.clone();
                    async move { Json(meta) }
                }
            }),
        )
        .route(
            "/facets/_meta",
            get({
                let meta = meta.clone();
                move || {
                    let meta = meta.clone();
                    async move { Json(meta) }
                }
            }),
        );

    // Mount each manifest's routes under `/facets/{id}{route.path}`.
    for m in &reg.manifests {
        let sub = build_router_for_manifest::<S>(m);

        router = match mode {
            // DevAllow => facets are dev-friendly: no auth gate here.
            SecurityMode::DevAllow => router.merge(sub),

            // All stricter modes (DenyAll, External, etc.) => gate via RequireAuth.
            _ => router.merge(sub.layer(RequireAuthLayer::new(mode))),
        };
    }

    router
}

/// Build a sub-router for a single manifest.
///
/// The router returned here is *not* gated by auth; callers decide whether to
/// wrap it with `RequireAuthLayer` or not based on `SecurityMode`.
///
/// Paths are mounted as:
///   `/facets/{facet_id}{route.path}`
/// e.g.:
///   facet.id   = "docs"
///   route.path = "/hello"
///   → `/facets/docs/hello`
fn build_router_for_manifest<S>(m: &FacetManifest) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let mut r = Router::new();
    let base = format!("/facets/{}", m.facet.id);

    match m.facet.kind {
        FacetKind::Echo => {
            for rs in &m.route {
                let full_path = format!("{}{}", base, rs.path);
                match rs.method.to_ascii_uppercase().as_str() {
                    "GET" => {
                        r = r.route(&full_path, get(echo_handler));
                    }
                    "POST" => {
                        r = r.route(&full_path, post(echo_handler));
                    }
                    _ => {
                        // Unknown methods are ignored; manifest validation should
                        // have caught bad methods earlier.
                    }
                }
            }
        }
        FacetKind::Static => {
            for rs in &m.route {
                let full_path = format!("{}{}", base, rs.path);
                let file = rs.file.clone().expect("validated: file present for static facets");
                // Capture the file path by clone into the handler closure.
                r = r.route(&full_path, get(move || static_handler(file.clone())));
            }
        }
        FacetKind::Proxy => {
            // Placeholder: 501 Not Implemented for all declared routes.
            //
            // Future: wire this to svc-index / svc-storage / other services.
            for rs in &m.route {
                let full_path = format!("{}{}", base, rs.path);
                match rs.method.to_ascii_uppercase().as_str() {
                    "GET" => {
                        r = r.route(&full_path, get(proxy_not_implemented));
                    }
                    "POST" => {
                        r = r.route(&full_path, post(proxy_not_implemented));
                    }
                    _ => {}
                }
            }
        }
    }

    r
}

// -------- Handlers --------

async fn demo_ping() -> &'static str {
    "pong"
}

async fn meta_empty() -> Json<Meta> {
    Json(Meta { loaded: vec![] })
}

// Keep echo simple to avoid needing Axum's `query` feature.
// We just return a static acknowledgement JSON.
async fn echo_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "echo": "ok" }))
}

async fn static_handler(file: String) -> (StatusCode, Body) {
    match tokio::fs::read(file).await {
        Ok(bytes) => (StatusCode::OK, Body::from(bytes)),
        Err(_) => (StatusCode::NOT_FOUND, Body::from("not found")),
    }
}

// Temporary placeholder for proxy facets.
async fn proxy_not_implemented() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

// -------- Meta types --------

#[derive(Debug, Clone, Serialize)]
struct Meta {
    loaded: Vec<FacetSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct FacetSummary {
    id: String,
    kind: String,
    routes: Vec<RouteSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct RouteSummary {
    method: String,
    path: String,
}

impl From<&FacetManifest> for FacetSummary {
    fn from(m: &FacetManifest) -> Self {
        Self {
            id: m.facet.id.clone(),
            kind: match m.facet.kind {
                FacetKind::Static => "static".into(),
                FacetKind::Echo => "echo".into(),
                FacetKind::Proxy => "proxy".into(),
            },
            routes: m
                .route
                .iter()
                .map(|r| RouteSummary {
                    method: r.method.to_ascii_uppercase(),
                    // For meta, it’s more operator-friendly to show the full
                    // mounted path (`/facets/{id}{route.path}`) rather than
                    // the raw manifest path.
                    path: format!("/facets/{}{}", m.facet.id, r.path),
                })
                .collect(),
        }
    }
}
