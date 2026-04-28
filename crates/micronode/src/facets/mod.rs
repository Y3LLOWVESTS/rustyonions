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
    body::{to_bytes, Body},
    http::{HeaderValue, Request, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::sync::OnceLock;

pub mod loader;
pub mod manifest;

use loader::FacetRegistry;
use manifest::{FacetKind, FacetManifest, UpstreamSpec};

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
pub fn mount_with_registry<S>(
    mut router: Router<S>,
    reg: FacetRegistry,
    mode: SecurityMode,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if reg.manifests.is_empty() {
        return mount(router);
    }

    let meta = Meta { loaded: reg.manifests.iter().map(FacetSummary::from).collect() };

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

    for m in &reg.manifests {
        let sub = build_router_for_manifest::<S>(m);

        router = match mode {
            SecurityMode::DevAllow => router.merge(sub),
            _ => router.merge(sub.layer(RequireAuthLayer::new(mode.clone()))),
        };
    }

    router
}

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
                    "GET" => r = r.route(&full_path, get(echo_handler)),
                    "POST" => r = r.route(&full_path, post(echo_handler)),
                    _ => {}
                }
            }
        }

        FacetKind::Static => {
            for rs in &m.route {
                let full_path = format!("{}{}", base, rs.path);
                let file = rs.file.clone().expect("validated: file present for static facets");
                r = r.route(&full_path, get(move || static_handler(file.clone())));
            }
        }

        FacetKind::Proxy => {
            let upstream =
                m.upstream.as_ref().expect("validated: upstream present for proxy facets");

            // Build a stable upstream base URL like "http://127.0.0.1:5901"
            let upstream_base = format!(
                "{}://{}:{}",
                upstream.scheme.trim().to_ascii_lowercase(),
                upstream.host.trim(),
                upstream.port
            );

            let base_path = upstream.base_path.clone().unwrap_or_default();

            for rs in &m.route {
                // If rs.path ends with "/*tail", mount as a prefix route and proxy subpaths.
                if let Some(prefix) = rs.path.strip_suffix("/*tail") {
                    let mount_prefix = format!("{}{}", base, prefix);

                    // upstream_prefix is what we map mount_prefix to on the upstream.
                    // If upstream_path is provided, use it. Otherwise mirror the prefix.
                    let upstream_prefix_src = rs.upstream_path.as_deref().unwrap_or(prefix);
                    let upstream_prefix = join_paths(&base_path, upstream_prefix_src);

                    match rs.method.to_ascii_uppercase().as_str() {
                        "GET" => {
                            let upstream_base = upstream_base.clone();
                            let mount_prefix = mount_prefix.clone();
                            let upstream_prefix = upstream_prefix.clone();
                            r = r.route(
                                &format!("{}/*tail", mount_prefix),
                                get(move |req: Request<Body>| {
                                    let upstream_base = upstream_base.clone();
                                    let mount_prefix = mount_prefix.clone();
                                    let upstream_prefix = upstream_prefix.clone();
                                    async move {
                                        proxy_request(
                                            req,
                                            &upstream_base,
                                            &mount_prefix,
                                            &upstream_prefix,
                                        )
                                        .await
                                    }
                                }),
                            );
                        }
                        "POST" => {
                            let upstream_base = upstream_base.clone();
                            let mount_prefix = mount_prefix.clone();
                            let upstream_prefix = upstream_prefix.clone();
                            r = r.route(
                                &format!("{}/*tail", mount_prefix),
                                post(move |req: Request<Body>| {
                                    let upstream_base = upstream_base.clone();
                                    let mount_prefix = mount_prefix.clone();
                                    let upstream_prefix = upstream_prefix.clone();
                                    async move {
                                        proxy_request(
                                            req,
                                            &upstream_base,
                                            &mount_prefix,
                                            &upstream_prefix,
                                        )
                                        .await
                                    }
                                }),
                            );
                        }
                        _ => {}
                    }
                    continue;
                }

                // Exact route: proxy only the exact path.
                let full_path = format!("{}{}", base, rs.path);

                let upstream_path_src = rs.upstream_path.as_deref().unwrap_or(&rs.path);
                let upstream_path = join_paths(&base_path, upstream_path_src);

                match rs.method.to_ascii_uppercase().as_str() {
                    "GET" => {
                        let upstream_base = upstream_base.clone();
                        let mount_prefix = full_path.clone();
                        let upstream_prefix = upstream_path.clone();
                        r = r.route(
                            &full_path,
                            get(move |req: Request<Body>| {
                                let upstream_base = upstream_base.clone();
                                let mount_prefix = mount_prefix.clone();
                                let upstream_prefix = upstream_prefix.clone();
                                async move {
                                    proxy_request(
                                        req,
                                        &upstream_base,
                                        &mount_prefix,
                                        &upstream_prefix,
                                    )
                                    .await
                                }
                            }),
                        );
                    }
                    "POST" => {
                        let upstream_base = upstream_base.clone();
                        let mount_prefix = full_path.clone();
                        let upstream_prefix = upstream_path.clone();
                        r = r.route(
                            &full_path,
                            post(move |req: Request<Body>| {
                                let upstream_base = upstream_base.clone();
                                let mount_prefix = mount_prefix.clone();
                                let upstream_prefix = upstream_prefix.clone();
                                async move {
                                    proxy_request(
                                        req,
                                        &upstream_base,
                                        &mount_prefix,
                                        &upstream_prefix,
                                    )
                                    .await
                                }
                            }),
                        );
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

async fn echo_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "echo": "ok" }))
}

async fn static_handler(file: String) -> (StatusCode, Body) {
    match tokio::fs::read(file).await {
        Ok(bytes) => (StatusCode::OK, Body::from(bytes)),
        Err(_) => (StatusCode::NOT_FOUND, Body::from("not found")),
    }
}

// -------- Proxy implementation --------

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("reqwest client")
    })
}

/// Proxy `req` to `upstream_base`, rewriting path under `mount_prefix` to `upstream_prefix`.
///
/// - `mount_prefix` is the mounted path on micronode (e.g. "/facets/hello") or
///   an exact route (e.g. "/facets/hello/api").
/// - `upstream_prefix` is the upstream path prefix (e.g. "" or "/hello/api").
///
/// For wildcard routes, the tail after `mount_prefix` is appended to `upstream_prefix`.
async fn proxy_request(
    req: Request<Body>,
    upstream_base: &str,
    mount_prefix: &str,
    upstream_prefix: &str,
) -> Response<Body> {
    let base = match reqwest::Url::parse(upstream_base) {
        Ok(u) => u,
        Err(_) => return simple_status(StatusCode::BAD_GATEWAY),
    };

    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let body = req.into_body();

    let path = uri.path();
    let tail = if path.starts_with(mount_prefix) { &path[mount_prefix.len()..] } else { "" };

    let upstream_path = if tail.is_empty() {
        upstream_prefix.to_string()
    } else {
        join_paths(upstream_prefix, tail)
    };

    let mut target = base.clone();
    target.set_path(&normalize_path(&upstream_path));

    if let Some(q) = uri.query() {
        target.set_query(Some(q));
    }

    // Demo-grade: buffer body. Hard cap to prevent memory bombs.
    let bytes = match to_bytes(body, 8 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return simple_status(StatusCode::BAD_REQUEST),
    };

    let mut out = http_client().request(method, target);

    for (k, v) in headers.iter() {
        if is_hop_by_hop(k.as_str()) {
            continue;
        }
        if is_forbidden_outbound(k.as_str()) {
            continue;
        }
        out = out.header(k.as_str(), v.as_bytes());
    }

    let resp = match out.body(bytes.to_vec()).send().await {
        Ok(r) => r,
        Err(_) => return simple_status(StatusCode::BAD_GATEWAY),
    };

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    let mut builder = Response::builder().status(status);

    {
        let headers_mut = builder.headers_mut().expect("headers");
        for (k, v) in resp.headers().iter() {
            if is_hop_by_hop(k.as_str()) {
                continue;
            }
            if k.as_str().eq_ignore_ascii_case("content-length") {
                continue;
            }
            if let Ok(val) = HeaderValue::from_bytes(v.as_bytes()) {
                headers_mut.insert(k.clone(), val);
            }
        }
    }

    let body_bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return simple_status(StatusCode::BAD_GATEWAY),
    };

    builder.body(Body::from(body_bytes)).unwrap_or_else(|_| simple_status(StatusCode::BAD_GATEWAY))
}

fn simple_status(code: StatusCode) -> Response<Body> {
    Response::builder()
        .status(code)
        .body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn is_forbidden_outbound(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(), "host" | "content-length")
}

fn join_paths(a: &str, b: &str) -> String {
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    let a_end = a.ends_with('/');
    let b_start = b.starts_with('/');
    match (a_end, b_start) {
        (true, true) => format!("{}{}", a.trim_end_matches('/'), b),
        (false, false) => format!("{}/{}", a, b),
        _ => format!("{}{}", a, b),
    }
}

fn normalize_path(p: &str) -> String {
    if p.is_empty() {
        "/".to_string()
    } else if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{}", p)
    }
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

    #[serde(skip_serializing_if = "Option::is_none")]
    upstream: Option<UpstreamMeta>,
}

#[derive(Debug, Clone, Serialize)]
struct UpstreamMeta {
    scheme: String,
    host: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RouteSummary {
    method: String,
    path: String,
}

impl From<&FacetManifest> for FacetSummary {
    fn from(m: &FacetManifest) -> Self {
        let kind_str = match m.facet.kind {
            FacetKind::Static => "static",
            FacetKind::Echo => "echo",
            FacetKind::Proxy => "proxy",
        }
        .to_string();

        let upstream = match m.facet.kind {
            FacetKind::Proxy => m.upstream.as_ref().map(|u| UpstreamMeta {
                scheme: u.scheme.clone(),
                host: u.host.clone(),
                port: u.port,
                base_path: u.base_path.clone(),
            }),
            _ => None,
        };

        Self {
            id: m.facet.id.clone(),
            kind: kind_str,
            routes: m
                .route
                .iter()
                .map(|r| RouteSummary {
                    method: r.method.to_ascii_uppercase(),
                    path: format!("/facets/{}{}", m.facet.id, r.path),
                })
                .collect(),
            upstream,
        }
    }
}
