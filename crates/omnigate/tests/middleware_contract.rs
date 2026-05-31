// RO:WHAT — Contract tests for edge guards: decompression policy and HTTP body caps.
// RO:WHY — Prevent regressions: unknown/stacked encodings => 415; over-budget compressed/body => 413.
// RO:INTERACTS — omnigate::middleware::decompress_guard, omnigate::middleware::body_caps.
// RO:INVARIANTS — OAP cap is not changed here; HTTP body cap is separately bounded.
// RO:TEST — cargo test -p omnigate --test middleware_contract.

use axum::{
    body::{self, Body},
    http::{Request, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tower::{service_fn, ServiceBuilder, ServiceExt};

const READ_LIMIT: usize = 256 * 1024;
const MIB: usize = 1024 * 1024;
const DEFAULT_HTTP_BODY_CAP_BYTES: usize = 64 * MIB;

#[tokio::test]
async fn decompress_guard_unknown_encoding_415() {
    let svc = ServiceBuilder::new()
        .layer(omnigate::middleware::decompress_guard::layer())
        .service(service_fn(|_req| async move {
            Ok::<_, std::convert::Infallible>(Json(json!({"ok": true})).into_response())
        }));

    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header(axum::http::header::CONTENT_ENCODING, "compress")
        .body(Body::from("tiny"))
        .unwrap();

    let resp = svc.oneshot(req).await.unwrap().into_response();
    assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let bytes = body::to_bytes(resp.into_body(), READ_LIMIT).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["reason"], "unsupported_media_type");
}

#[tokio::test]
async fn decompress_guard_stacked_encodings_415() {
    let svc = ServiceBuilder::new()
        .layer(omnigate::middleware::decompress_guard::layer())
        .service(service_fn(|_req| async move {
            Ok::<_, std::convert::Infallible>(Json(json!({"ok": true})).into_response())
        }));

    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header(axum::http::header::CONTENT_ENCODING, "gzip, br")
        .body(Body::from("tiny"))
        .unwrap();

    let resp = svc.oneshot(req).await.unwrap().into_response();
    assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let bytes = body::to_bytes(resp.into_body(), READ_LIMIT).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["reason"], "unsupported_media_type");
}

#[tokio::test]
async fn decompress_guard_compressed_over_budget_413() {
    let svc = ServiceBuilder::new()
        .layer(omnigate::middleware::decompress_guard::layer())
        .service(service_fn(|_req| async move {
            Ok::<_, std::convert::Infallible>(Json(json!({"ok": true})).into_response())
        }));

    // With EXPANSION_CAP=10 and the simple test-layer max-expanded budget of
    // 1 MiB, any compressed length > ~104_857 bytes triggers 413.
    let declared_len = 200_000u64;

    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header(axum::http::header::CONTENT_ENCODING, "gzip")
        .header(axum::http::header::CONTENT_LENGTH, declared_len.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = svc.oneshot(req).await.unwrap().into_response();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let bytes = body::to_bytes(resp.into_body(), READ_LIMIT).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["reason"], "payload_too_large");
}

#[tokio::test]
async fn body_caps_oversized_by_header_413() {
    let svc = ServiceBuilder::new()
        .layer(omnigate::middleware::body_caps::layer())
        .service(service_fn(|_req| async move {
            Ok::<_, std::convert::Infallible>(Json(json!({"ok": true})).into_response())
        }));

    // The current HTTP body cap is 64 MiB. We only declare the length here; the
    // test does not allocate a 64 MiB body.
    let oversized = DEFAULT_HTTP_BODY_CAP_BYTES + 1;

    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header(axum::http::header::CONTENT_LENGTH, oversized.to_string())
        .body(Body::empty())
        .unwrap();

    let resp = svc.oneshot(req).await.unwrap().into_response();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let bytes = body::to_bytes(resp.into_body(), READ_LIMIT).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["reason"], "payload_too_large");
}

#[tokio::test]
async fn body_caps_small_ok_200() {
    let svc = ServiceBuilder::new()
        .layer(omnigate::middleware::body_caps::layer())
        .service(service_fn(|_req| async move {
            Ok::<_, std::convert::Infallible>(Json(json!({"ok": true})).into_response())
        }));

    let body_txt = "hello world";
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header(
            axum::http::header::CONTENT_LENGTH,
            body_txt.len().to_string(),
        )
        .body(Body::from(body_txt.to_string()))
        .unwrap();

    let resp = svc.oneshot(req).await.unwrap().into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = body::to_bytes(resp.into_body(), READ_LIMIT).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["ok"], true);
}
