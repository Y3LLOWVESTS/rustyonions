// crates/micronode/benches/http_kv.rs
//! Simple HTTP-level benchmarks for micronode.
//!
//! RO:WHAT — Benchmark the in-process Axum router for:
//!   - `/healthz` fast-path.
//!   - `/v1/kv/{bucket}/{key}` small PUT/GET/DELETE roundtrip.
//!   - `/v1/kv/{bucket}/{key}` hot reject via DecodeGuard (Content-Encoding).
//!
//! RO:WHY  — Give us a quick sanity check on HTTP latencies for the Micronode
//!           profile without binding a real TCP port.
//!
//! RO:HOW  — Build the Router via `build_router(Config::default())` and drive
//!           requests with a Tokio runtime + `Router::oneshot(req)` inside a
//!           Criterion `b.iter(...)` loop.

use axum::body::Body;
use axum::Router;
use criterion::{criterion_group, criterion_main, Criterion};
use http::{Method, Request, StatusCode};
use micronode::app::build_router;
use micronode::config::schema::Config;
use tower::ServiceExt as _; // for `oneshot`

fn build_app() -> Router {
    // For benches we can rely on Config::default(): it should give us
    // localhost bind + in-memory storage engine.
    let cfg = Config::default();
    let (router, _state) = build_router(cfg);
    router
}

fn bench_healthz(c: &mut Criterion) {
    let router = build_app();

    // Shared runtime for this benchmark.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for benches");

    c.bench_function("http_healthz_fast_path", |b| {
        b.iter(|| {
            // Clone the Router so `oneshot(self, req)` can take ownership.
            let router = router.clone();

            rt.block_on(async move {
                let req = Request::builder()
                    .method(Method::GET)
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("healthz request build failed");

                let resp = router.oneshot(req).await.expect("healthz handler failed");

                // Sanity: ensure we stayed on the happy path.
                assert_eq!(resp.status(), StatusCode::OK);
            });
        });
    });
}

fn bench_kv_small_roundtrip(c: &mut Criterion) {
    let router = build_app();

    // Shared runtime for this benchmark.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for benches");

    c.bench_function("http_kv_small_put_get_delete", |b| {
        b.iter(|| {
            // Clone the Router so `oneshot(self, req)` can take ownership.
            let router = router.clone();

            rt.block_on(async move {
                let bucket = "bench";
                let key = "k";

                // Payload for PUT — we must set an accurate Content-Length
                // because BodyCapLayer enforces it for POST/PUT/PATCH.
                let payload = "hello-micronode";
                let payload_len = payload.len().to_string();

                // PUT small value
                let put_req = Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/v1/kv/{}/{}", bucket, key))
                    .header("content-type", "application/octet-stream")
                    .header("content-length", payload_len)
                    .body(Body::from(payload.to_owned()))
                    .expect("PUT request build failed");

                let put_resp = router.clone().oneshot(put_req).await.expect("PUT handler failed");
                assert!(
                    put_resp.status().is_success(),
                    "expected PUT success, got {}",
                    put_resp.status()
                );

                // GET the value
                let get_req = Request::builder()
                    .method(Method::GET)
                    .uri(format!("/v1/kv/{}/{}", bucket, key))
                    .body(Body::empty())
                    .expect("GET request build failed");

                let get_resp = router.clone().oneshot(get_req).await.expect("GET handler failed");
                assert_eq!(get_resp.status(), StatusCode::OK);

                // DELETE it
                let del_req = Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/v1/kv/{}/{}", bucket, key))
                    .body(Body::empty())
                    .expect("DELETE request build failed");

                let del_resp = router.oneshot(del_req).await.expect("DELETE handler failed");
                assert!(
                    del_resp.status().is_success(),
                    "expected DELETE success, got {}",
                    del_resp.status()
                );
            });
        });
    });
}

fn bench_kv_decode_guard_hot_reject(c: &mut Criterion) {
    let router = build_app();

    // Shared runtime for this benchmark.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for benches");

    c.bench_function("http_kv_decode_guard_hot_reject", |b| {
        b.iter(|| {
            // Clone the Router so `oneshot(self, req)` can take ownership.
            let router = router.clone();

            rt.block_on(async move {
                let bucket = "bench";
                let key = "guard";

                // Tiny payload with accurate length so BodyCapLayer is satisfied
                // and we actually exercise DecodeGuard.
                let payload = "x";
                let payload_len = payload.len().to_string();

                let req = Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/v1/kv/{}/{}", bucket, key))
                    .header("content-type", "application/octet-stream")
                    .header("content-length", payload_len)
                    .header("content-encoding", "gzip")
                    .body(Body::from(payload.to_owned()))
                    .expect("decode-guard PUT request build failed");

                let resp = router.oneshot(req).await.expect("decode-guard handler failed");

                // Sanity: ensure we’re measuring the hot reject, not a 411 from BodyCap.
                assert_eq!(
                    resp.status(),
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "expected 415 from DecodeGuard, got {}",
                    resp.status()
                );
            });
        });
    });
}

criterion_group!(
    micronode_http,
    bench_healthz,
    bench_kv_small_roundtrip,
    bench_kv_decode_guard_hot_reject
);
criterion_main!(micronode_http);
