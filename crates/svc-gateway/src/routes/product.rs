//! `WEB3_2` product route exposure.
//!
//! RO:WHAT — Public edge routes for `crab://`, typed `b3` pages, paid prepare, identity/profile, wallet hold/display, image, text asset, content-view, and site flows.
//! RO:WHY — P6/P7/P12; Concerns: DX/SEC/ECON. Browser clients need clean gateway paths over stable `omnigate` routes.
//! RO:INTERACTS — `omnigate` `/v1/crab`, `/v1/b3`, `/v1/paid`, `/v1/identity`, `/v1/wallet`, `/v1/assets`, `/v1/content`, `/v1/sites`.
//! RO:INVARIANTS — proxy-only; no manifest parsing; no pricing; no storage writes; no direct passport/wallet/ledger mutation.
//! RO:METRICS — route inherits gateway HTTP metrics/correlation layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`.
//! RO:SECURITY — forwards selected auth/idempotency/`x-ron-*` headers; filters hop-by-hop headers.
//! RO:TEST — `tests/product_routes_proxy.rs`, `tests/identity_routes_proxy.rs`, `tests/content_view_routes_proxy.rs`; `CrabLink` smoke scripts.

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
/// POST /identity/passport/profile/claim
/// GET  /identity/passport/profile/:username
/// GET  /wallet/:account/balance
/// POST /wallet/hold
/// GET  /crab/resolve?url=...
/// GET  /b3/:asset
/// POST /paid/o/prepare
/// POST /assets/image/prepare
/// POST /assets/image
/// POST /assets/post/prepare
/// POST /assets/post
/// POST /assets/comment/prepare
/// POST /assets/comment
/// POST /assets/article/prepare
/// POST /assets/article
/// POST /content/view/quote
/// POST /content/view/pay
/// POST /sites/prepare
/// POST /sites
/// GET  /sites/:name
/// POST /sites/:name/visit/quote
/// POST /sites/:name/visit/pay
/// ```
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/identity/me", get(identity_me))
        .route("/identity/passport/bootstrap", post(passport_bootstrap))
        .route(
            "/identity/passport/profile/claim",
            post(passport_profile_claim),
        )
        .route(
            "/identity/passport/profile/:username",
            get(passport_profile_get),
        )
        .route("/wallet/:account/balance", get(wallet_balance))
        .route("/wallet/hold", post(wallet_hold))
        .route("/crab/resolve", get(resolve_crab))
        .route("/b3/:asset", get(resolve_b3_asset))
        .route("/paid/o/prepare", post(paid_object_prepare))
        .route("/assets/image/prepare", post(image_prepare))
        .route("/assets/image", post(image_upload))
        .route("/assets/post/prepare", post(post_prepare))
        .route("/assets/post", post(post_publish))
        .route("/assets/comment/prepare", post(comment_prepare))
        .route("/assets/comment", post(comment_publish))
        .route("/assets/article/prepare", post(article_prepare))
        .route("/assets/article", post(article_publish))
        .route("/content/view/quote", post(content_view_quote))
        .route("/content/view/pay", post(content_view_pay))
        .route("/sites/prepare", post(site_prepare))
        .route("/sites", post(site_create))
        .route("/sites/:name", get(site_resolve))
        .route("/sites/:name/visit/quote", post(site_visit_quote))
        .route("/sites/:name/visit/pay", post(site_visit_pay))
}

/// Proxy `GET /identity/me` to `omnigate /v1/identity/me`.
///
/// Gateway does not own identity semantics. It only forwards the request.
pub async fn identity_me(State(state): State<AppState>, uri: Uri, headers: HeaderMap) -> Response {
    let upstream_path = with_query("/v1/identity/me", uri.query());

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
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

/// Proxy `POST /identity/passport/profile/claim` to
/// `omnigate /v1/identity/passport/profile/claim`.
///
/// Gateway does not reserve usernames or persist profile truth. It only exposes
/// the public browser/client path and forwards to Omnigate.
pub async fn passport_profile_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/identity/passport/profile/claim",
        headers,
        body,
    )
    .await
}

/// Proxy `GET /identity/passport/profile/:username` to
/// `omnigate /v1/identity/passport/profile/:username`.
///
/// Gateway does not hydrate profiles or interpret username ownership.
pub async fn passport_profile_get(
    State(state): State<AppState>,
    Path(username): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/v1/identity/passport/profile/{username}");

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `GET /wallet/:account/balance` to `omnigate /v1/wallet/:account/balance`.
///
/// Gateway does not own wallet truth. It only forwards display/read requests.
pub async fn wallet_balance(
    State(state): State<AppState>,
    Path(account): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/v1/wallet/{account}/balance");

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `POST /wallet/hold` to `omnigate /v1/wallet/hold`.
///
/// Gateway does not mutate the wallet directly. Omnigate forwards to the wallet
/// façade, and `svc-wallet` remains the normal economic mutation front door.
pub async fn wallet_hold(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/wallet/hold", headers, body).await
}

/// Proxy `GET /crab/resolve?url=...` to `omnigate /v1/crab/resolve?url=...`.
pub async fn resolve_crab(State(state): State<AppState>, uri: Uri, headers: HeaderMap) -> Response {
    let upstream_path = with_query("/v1/crab/resolve", uri.query());

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `GET /b3/:asset` to `omnigate /v1/b3/:asset`.
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
/// Gateway does not price storage or create wallet holds.
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

/// Proxy `POST /assets/post/prepare` to `omnigate /v1/assets/post/prepare`.
///
/// Gateway does not validate text content, price writes, store bytes, write index
/// pointers, or claim post publication truth.
pub async fn post_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/post/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/post` to `omnigate /v1/assets/post`.
///
/// Gateway only exposes the public `CrabLink` route. Omnigate owns post content
/// validation, paid proof validation, storage, manifest, and index work.
pub async fn post_publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/assets/post", headers, body).await
}

/// Proxy `POST /assets/comment/prepare` to `omnigate /v1/assets/comment/prepare`.
///
/// Gateway does not validate site/thread/parent relationships. Omnigate owns the
/// comment primitive semantics.
pub async fn comment_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/comment/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/comment` to `omnigate /v1/assets/comment`.
///
/// Gateway does not publish comments or write thread/index state.
pub async fn comment_publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/assets/comment", headers, body).await
}

/// Proxy `POST /assets/article/prepare` to `omnigate /v1/assets/article/prepare`.
///
/// Gateway does not validate article body/title/site context. Omnigate owns the
/// article primitive semantics.
pub async fn article_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/article/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/article` to `omnigate /v1/assets/article`.
///
/// Gateway does not publish articles, store bytes, create manifests, or write
/// asset pointers.
pub async fn article_publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/assets/article", headers, body).await
}

/// Proxy `POST /content/view/quote` to `omnigate /v1/content/view/quote`.
///
/// Gateway does not resolve manifests, select payout recipients, price views,
/// call wallet, or mutate ledger. It only exposes the public `CrabLink` path.
pub async fn content_view_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/content/view/quote",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /content/view/pay` to `omnigate /v1/content/view/pay`.
///
/// Gateway does not mutate the wallet or ledger. Omnigate coordinates the
/// product route and sends the transfer through svc-wallet.
pub async fn content_view_pay(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/content/view/pay", headers, body).await
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
pub async fn site_resolve(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/v1/sites/{name}");

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `POST /sites/:name/visit/quote` to
/// `omnigate /v1/sites/:name/visit/quote`.
///
/// Gateway does not quote, price, or inspect site manifests. It only forwards
/// the browser-facing paid site visit route to Omnigate.
pub async fn site_visit_quote(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/sites/{name}/visit/quote");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `POST /sites/:name/visit/pay` to
/// `omnigate /v1/sites/:name/visit/pay`.
///
/// Gateway does not mutate the wallet or ledger. Omnigate routes the mutation
/// through `svc-wallet`, which remains the economic front door.
pub async fn site_visit_pay(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/sites/{name}/visit/pay");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
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

    let mut resp = Response::new(Body::from(body_bytes));
    *resp.status_mut() = status;

    let resp_headers = resp.headers_mut();
    for (name, value) in &upstream_headers {
        if should_copy_response_header(name) {
            resp_headers.insert(name.clone(), value.clone());
        }
    }

    resp
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
