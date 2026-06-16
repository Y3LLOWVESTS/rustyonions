//! `WEB3_2` product route exposure.
//!
//! RO:WHAT — Public edge routes for `crab://`, typed `b3` pages, paid prepare, identity/profile, wallet hold/display, image, music, text asset, content-view, and site flows.
//! RO:WHY — P6/P7/P12; Concerns: DX/SEC/ECON. Browser clients need clean gateway paths over stable `omnigate` routes.
//! RO:INTERACTS — `omnigate` `/v1/crab`, `/v1/b3`, `/v1/paid`, `/v1/identity`, `/v1/wallet`, `/v1/assets`, `/v1/content`, `/v1/sites`.
//! RO:INVARIANTS — proxy-only; no manifest parsing; no pricing; no storage writes; no direct passport/wallet/ledger mutation.
//! RO:METRICS — route inherits gateway HTTP metrics/correlation layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`.
//! RO:SECURITY — forwards selected auth/idempotency/`x-ron-*` headers; filters hop-by-hop headers.
//! RO:TEST — `tests/product_routes_proxy.rs`, `tests/identity_routes_proxy.rs`, `tests/content_view_routes_proxy.rs`; `CrabLink` smoke scripts.

use crate::{errors, headers::proxy, state::AppState};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::Response,
    routing::{get, post},
    Router,
};

/// HTTP body cap for media upload proxy routes.
/// OAP frame caps remain separate and stay at 1 MiB.
const IMAGE_UPLOAD_BODY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const MEDIA_UPLOAD_BODY_LIMIT_BYTES: usize = IMAGE_UPLOAD_BODY_LIMIT_BYTES;

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
/// POST /assets/video/prepare
/// POST /assets/video
/// POST /assets/music/prepare
/// POST /assets/music
/// POST /assets/podcast/prepare
/// POST /assets/podcast
/// POST /assets/stream/prepare
/// POST /assets/stream
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
/// GET  /chat/resolve?url=...
/// POST /chat/prepare
/// POST /chat
/// GET  /chat/:room_id/messages
/// GET  /chat/:room_id/messages/latest
/// POST /chat/:room_id/messages/quote
/// POST /chat/:room_id/messages/send
/// POST /chat/:room_id/mod/delete
/// POST /chat/:room_id/mod/block
/// POST /chat/:room_id/mod/pin
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
        .route("/chat/resolve", get(chat_resolve))
        .route("/chat/prepare", post(chat_prepare))
        .route("/chat", post(chat_create))
        .route("/chat/:room_id/messages", get(chat_messages_list))
        .route("/chat/:room_id/messages/latest", get(chat_messages_latest))
        .route("/chat/:room_id/messages/quote", post(chat_message_quote))
        .route("/chat/:room_id/messages/send", post(chat_message_send))
        .route("/chat/:room_id/mod/delete", post(chat_mod_delete))
        .route("/chat/:room_id/mod/block", post(chat_mod_block))
        .route("/chat/:room_id/mod/pin", post(chat_mod_pin))
        .route("/assets/image/prepare", post(image_prepare))
        .route("/assets/image", post(image_upload))
        .route("/assets/video/prepare", post(video_prepare))
        .route("/assets/video", post(video_upload))
        .route("/assets/music/prepare", post(music_prepare))
        .route("/assets/music", post(music_upload))
        .route("/assets/podcast/prepare", post(podcast_prepare))
        .route("/assets/podcast", post(podcast_upload))
        .route("/assets/stream/prepare", post(stream_prepare))
        .route("/assets/stream", post(stream_publish))
        .route("/assets/post/prepare", post(post_prepare))
        .route("/assets/post", post(post_publish))
        .route("/assets/comment/prepare", post(comment_prepare))
        .route("/assets/comment", post(comment_publish))
        .route("/assets/article/prepare", post(article_prepare))
        .route("/assets/article", post(article_publish))
        .route("/content/view/quote", post(content_view_quote))
        .route("/content/view/pay", post(content_view_pay))
        .route("/streams/:stream_id/start", post(stream_start))
        .route("/streams/:stream_id/stop", post(stream_stop))
        .route("/streams/:stream_id/status", get(stream_status))
        .route("/streams/:stream_id/segments", post(stream_segment_put))
        .route(
            "/streams/:stream_id/segments/latest",
            post(stream_segment_latest),
        )
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

/// Proxy `GET /chat/resolve?url=...` to `omnigate /v1/chat/resolve`.
///
/// Gateway stays proxy-only. Omnigate owns the chat room semantics and the
/// route currently returns an in-memory dev proof, not durable chat truth.
pub async fn chat_resolve(State(state): State<AppState>, uri: Uri, headers: HeaderMap) -> Response {
    let upstream_path = with_query("/v1/chat/resolve", uri.query());

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `POST /chat/prepare` to `omnigate /v1/chat/prepare`.
///
/// This prepares the future chat room contract but does not mutate wallet or
/// ledger state at the gateway.
pub async fn chat_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/chat/prepare", headers, body).await
}

/// Proxy `POST /chat` to `omnigate /v1/chat`.
///
/// Gateway does not create b3 CIDs, write index pointers, or invent receipts.
pub async fn chat_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/chat", headers, body).await
}

/// Proxy `GET /chat/:room_id/messages` to `omnigate /v1/chat/:room_id/messages`.
pub async fn chat_messages_list(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let upstream_path = with_query(&format!("/v1/chat/{room_id}/messages"), uri.query());

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `GET /chat/:room_id/messages/latest` to
/// `omnigate /v1/chat/:room_id/messages/latest`.
pub async fn chat_messages_latest(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let upstream_path = with_query(&format!("/v1/chat/{room_id}/messages/latest"), uri.query());

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `POST /chat/:room_id/messages/quote` to
/// `omnigate /v1/chat/:room_id/messages/quote`.
///
/// Quote is not wallet mutation. The future paid send must still use the
/// backend wallet path and return a backend receipt.
pub async fn chat_message_quote(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/chat/{room_id}/messages/quote");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `POST /chat/:room_id/messages/send` to
/// `omnigate /v1/chat/:room_id/messages/send`.
///
/// Gateway does not perform wallet mutation or create receipts. Omnigate must
/// fail paid sends closed until svc-wallet integration is explicitly wired.
pub async fn chat_message_send(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/chat/{room_id}/messages/send");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `POST /chat/:room_id/mod/delete` to `omnigate /v1/chat/:room_id/mod/delete`.
pub async fn chat_mod_delete(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/chat/{room_id}/mod/delete");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `POST /chat/:room_id/mod/block` to `omnigate /v1/chat/:room_id/mod/block`.
pub async fn chat_mod_block(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/chat/{room_id}/mod/block");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `POST /chat/:room_id/mod/pin` to `omnigate /v1/chat/:room_id/mod/pin`.
pub async fn chat_mod_pin(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/chat/{room_id}/mod/pin");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
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
    body: Body,
) -> Response {
    let Ok(body) = axum::body::to_bytes(body, IMAGE_UPLOAD_BODY_LIMIT_BYTES).await else {
        return errors::Problem {
            code: "image_upload_body_too_large",
            message: "image upload body exceeded the configured image upload cap",
            retryable: false,
            retry_after_ms: None,
            reason: Some("image_upload_body_too_large"),
        }
        .into_response_with(StatusCode::PAYLOAD_TOO_LARGE);
    };

    proxy_to_omnigate(&state, Method::POST, "/v1/assets/image", headers, body).await
}

/// Proxy `POST /assets/video/prepare` to `omnigate /v1/assets/video/prepare`.
///
/// Gateway does not validate video media semantics, price writes, store bytes,
/// write index pointers, transcode, or claim streaming support.
pub async fn video_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/video/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/video` to `omnigate /v1/assets/video`.
///
/// Gateway only exposes the public `CrabLink` route. Omnigate coordinates the
/// bounded video-lite paid storage and manifest/index write.
pub async fn video_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let Ok(body) = axum::body::to_bytes(body, MEDIA_UPLOAD_BODY_LIMIT_BYTES).await else {
        return errors::Problem {
            code: "video_upload_body_too_large",
            message: "video upload body exceeded the configured media upload cap",
            retryable: false,
            retry_after_ms: None,
            reason: Some("video_upload_body_too_large"),
        }
        .into_response_with(StatusCode::PAYLOAD_TOO_LARGE);
    };

    proxy_to_omnigate(&state, Method::POST, "/v1/assets/video", headers, body).await
}

/// Proxy `POST /assets/music/prepare` to `omnigate /v1/assets/music/prepare`.
///
/// Gateway does not validate music rights, price writes, store bytes, write index
/// pointers, transcode audio, upload cover art, or claim ownership proof.
pub async fn music_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/music/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/music` to `omnigate /v1/assets/music`.
///
/// Gateway only exposes the public `CrabLink` route. Omnigate coordinates the
/// bounded music-lite paid storage and manifest/index write.
pub async fn music_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let Ok(body) = axum::body::to_bytes(body, MEDIA_UPLOAD_BODY_LIMIT_BYTES).await else {
        return errors::Problem {
            code: "music_upload_body_too_large",
            message: "music upload body exceeded the configured media upload cap",
            retryable: false,
            retry_after_ms: None,
            reason: Some("music_upload_body_too_large"),
        }
        .into_response_with(StatusCode::PAYLOAD_TOO_LARGE);
    };

    proxy_to_omnigate(&state, Method::POST, "/v1/assets/music", headers, body).await
}

/// Proxy `POST /assets/podcast/prepare` to `omnigate /v1/assets/podcast/prepare`.
///
/// Gateway does not validate podcast rights, guest permissions, price writes,
/// store bytes, write index pointers, transcode audio, upload cover art, or
/// claim legal proof. It only exposes the public `CrabLink` route.
pub async fn podcast_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/podcast/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/podcast` to `omnigate /v1/assets/podcast`.
///
/// Gateway only exposes the public `CrabLink` route. Omnigate coordinates the
/// bounded podcast-lite paid storage and manifest/index write.
pub async fn podcast_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let Ok(body) = axum::body::to_bytes(body, MEDIA_UPLOAD_BODY_LIMIT_BYTES).await else {
        return errors::Problem {
            code: "podcast_upload_body_too_large",
            message: "podcast upload body exceeded the configured media upload cap",
            retryable: false,
            retry_after_ms: None,
            reason: Some("podcast_upload_body_too_large"),
        }
        .into_response_with(StatusCode::PAYLOAD_TOO_LARGE);
    };

    proxy_to_omnigate(&state, Method::POST, "/v1/assets/podcast", headers, body).await
}

/// Proxy `POST /assets/stream/prepare` to `omnigate /v1/assets/stream/prepare`.
///
/// Gateway does not create live sessions, claim viewer access, store segments,
/// mutate wallets, or mint stream truth. It only exposes the public `CrabLink` path.
pub async fn stream_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/stream/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/stream` to `omnigate /v1/assets/stream`.
///
/// Omnigate coordinates stream descriptor publication. Live ingest, viewer paid
/// windows, and stream segments remain separate future routes.
pub async fn stream_publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/assets/stream", headers, body).await
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

/// Proxy `POST /streams/:stream_id/start` to `omnigate /v1/streams/:stream_id/start`.
///
/// Gateway does not create wallet truth, stream keys, or segment truth. It only
/// exposes the public `CrabLink` path to Omnigate's bounded stream-lite route.
pub async fn stream_start(
    State(state): State<AppState>,
    Path(stream_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/streams/{stream_id}/start");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `POST /streams/:stream_id/stop` to `omnigate /v1/streams/:stream_id/stop`.
pub async fn stream_stop(
    State(state): State<AppState>,
    Path(stream_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/streams/{stream_id}/stop");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `GET /streams/:stream_id/status` to `omnigate /v1/streams/:stream_id/status`.
pub async fn stream_status(
    State(state): State<AppState>,
    Path(stream_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let upstream_path = format!("/v1/streams/{stream_id}/status");

    proxy_to_omnigate(&state, Method::GET, &upstream_path, headers, Bytes::new()).await
}

/// Proxy `POST /streams/:stream_id/segments` to `omnigate /v1/streams/:stream_id/segments`.
///
/// Stream-lite v1 accepts only bounded latest segments; it is not durable media storage.
pub async fn stream_segment_put(
    State(state): State<AppState>,
    Path(stream_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/streams/{stream_id}/segments");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
}

/// Proxy `POST /streams/:stream_id/segments/latest` to
/// `omnigate /v1/streams/:stream_id/segments/latest`.
///
/// Gateway does not validate paid access. Omnigate verifies the wallet receipt
/// against `svc-wallet` before returning viewer media.
pub async fn stream_segment_latest(
    State(state): State<AppState>,
    Path(stream_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let upstream_path = format!("/v1/streams/{stream_id}/segments/latest");

    proxy_to_omnigate(&state, Method::POST, &upstream_path, headers, body).await
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
        if proxy::should_forward_product_header(name) {
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
        if proxy::should_copy_response_header(name) {
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
