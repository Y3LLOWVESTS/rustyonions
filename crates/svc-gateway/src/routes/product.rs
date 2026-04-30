//! `WEB3_2` product route exposure.
//!
//! RO:WHAT — Public edge routes for `crab://`, typed `b3` asset pages, paid prepare, identity, wallet display, image, and site flows.
//! RO:WHY — Browser extension and HTTP clients need clean gateway paths over stable `omnigate` routes.
//! RO:INTERACTS — `omnigate` `/v1/crab/*`, `/v1/b3/*`, `/v1/paid/*`, `/v1/identity/*`, `/v1/wallet/*`, `/v1/assets/*`, `/v1/sites/*`.
//! RO:INVARIANTS — proxy-only; no manifest parsing; no pricing; no storage writes; no wallet/ledger mutation.
//! RO:METRICS — route inherits gateway HTTP metrics/correlation layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`.
//! RO:SECURITY — forwards selected auth/idempotency/`x-ron-*` headers; filters hop-by-hop headers.
//! RO:TEST — `tests/product_routes_proxy.rs`, `tests/identity_routes_proxy.rs`.

use crate::{errors, state::AppState};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, HeaderName, Method, Uri},
    response::Response,
    routing::{get, post},
    Router,
};

/// Router for product-facing `WEB3_2` edge routes.
///
/// Routes exposed:
///
/// ```text
/// GET  /identity/me
/// POST /identity/passport/bootstrap
/// GET  /wallet/:account/balance
/// GET  /crab/resolve?url=...
/// GET  /b3/:asset
/// POST /paid/o/prepare
/// POST /assets/image/prepare
/// POST /assets/image
/// POST /sites/prepare
/// POST /sites
/// GET  /sites/:name
/// ```
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/identity/me", get(identity_me))
        .route("/identity/passport/bootstrap", post(passport_bootstrap))
        .route("/wallet/:account/balance", get(wallet_balance))
        .route("/crab/resolve", get(resolve_crab))
        .route("/b3/:asset", get(resolve_b3_asset))
        .route("/paid/o/prepare", post(paid_object_prepare))
        .route("/assets/image/prepare", post(image_prepare))
        .route("/assets/image", post(image_upload))
        .route("/sites/prepare", post(site_prepare))
        .route("/sites", post(site_create))
        .route("/sites/:name", get(site_resolve))
}

/// Proxy `GET /identity/me` to `omnigate /v1/identity/me`.
///
/// Gateway does not own identity semantics. It only forwards the request.
pub async fn identity_me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    proxy_to_omnigate(
        &state,
        Method::GET,
        "/v1/identity/me",
        headers,
        Bytes::new(),
    )
    .await
}

/// Proxy `POST /identity/passport/bootstrap` to `omnigate /v1/identity/passport/bootstrap`.
///
/// Gateway does not create passports, keys, wallets, or starter grants.
pub async fn passport_bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/identity/passport/bootstrap",
        headers,
        body,
    )
    .await
}

/// Proxy `GET /wallet/:account/balance` to `omnigate /v1/wallet/:account/balance`.
///
/// Gateway does not read wallet state directly and does not mutate ledger.
pub async fn wallet_balance(
    State(state): State<AppState>,
    Path(account): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/v1/wallet/{account}/balance");
    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `GET /crab/resolve?url=...` to `omnigate /v1/crab/resolve?url=...`.
pub async fn resolve_crab(State(state): State<AppState>, uri: Uri, headers: HeaderMap) -> Response {
    let upstream_path = with_query("/v1/crab/resolve", uri.query());
    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `GET /b3/:asset` to `omnigate /v1/b3/:asset`.
///
/// `:asset` is expected to be `<64hex>.<kind>`, but gateway does not validate
/// product semantics; `omnigate` owns typed asset-page parsing.
pub async fn resolve_b3_asset(
    State(state): State<AppState>,
    Path(asset): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/v1/b3/{asset}");
    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `POST /paid/o/prepare` to `omnigate /v1/paid/o/prepare`.
///
/// This is a preflight/prepare route only. Gateway does not create holds.
pub async fn paid_object_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/paid/o/prepare", headers, body).await
}

/// Proxy `POST /assets/image/prepare` to `omnigate /v1/assets/image/prepare`.
pub async fn image_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/image/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/image` to `omnigate /v1/assets/image`.
pub async fn image_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/assets/image", headers, body).await
}

/// Proxy `POST /sites/prepare` to `omnigate /v1/sites/prepare`.
pub async fn site_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/sites/prepare", headers, body).await
}

/// Proxy `POST /sites` to `omnigate /v1/sites`.
pub async fn site_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/sites", headers, body).await
}

/// Proxy `GET /sites/:name` to `omnigate /v1/sites/:name`.
///
/// Gateway intentionally does not normalize, validate, or hydrate site names.
/// `omnigate` owns product semantics and fail-closed validation.
pub async fn site_resolve(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/v1/sites/{name}");
    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

async fn proxy_to_omnigate(
    state: &AppState,
    method: Method,
    upstream_path: &str,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let base = state.cfg.upstreams.omnigate_base_url.trim_end_matches('/');
    let upstream_url = format!("{base}{upstream_path}");

    let Ok(reqwest_method) = reqwest::Method::from_bytes(method.as_str().as_bytes()) else {
        return errors::upstream_unavailable("bad_method");
    };

    let mut req_builder = state.omnigate_client.request(reqwest_method, &upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let Ok(upstream_res) = req_builder.body(body).send().await else {
        return errors::upstream_unavailable("omnigate_connect");
    };

    let status = upstream_res.status();
    let upstream_headers = upstream_res.headers().clone();

    let Ok(body_bytes) = upstream_res.bytes().await else {
        return errors::upstream_unavailable("omnigate_read");
    };

    let mut response = Response::new(Body::from(body_bytes));
    *response.status_mut() = status;

    let resp_headers = response.headers_mut();
    for (name, value) in &upstream_headers {
        if should_copy_response_header(name) {
            resp_headers.insert(name.clone(), value.clone());
        }
    }

    response
}

fn with_query(path: &str, query: Option<&str>) -> String {
    match query {
        Some(query) if !query.is_empty() => format!("{path}?{query}"),
        _ => path.to_owned(),
    }
}

fn should_forward_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_or_host(name) || name == header::CONTENT_LENGTH {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || name.as_str().starts_with("x-ron-")
        || name.as_str() == "x-correlation-id"
        || name.as_str() == "x-request-id"
        || name.as_str() == "idempotency-key"
}

fn should_copy_response_header(name: &HeaderName) -> bool {
    name != header::TRANSFER_ENCODING
        && name != header::CONTENT_LENGTH
        && name != header::CONNECTION
}

fn is_hop_by_hop_or_host(name: &HeaderName) -> bool {
    name == header::HOST
        || name == header::CONNECTION
        || name == header::PROXY_AUTHORIZATION
        || name == header::TE
        || name == header::TRAILER
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
}
