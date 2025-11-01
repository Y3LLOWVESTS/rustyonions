//! Bench the omnigate middleware stack end-to-end (in-process, no sockets).
//! Build an axum Router, apply `omnigate::middleware::apply`, then
//! issue GET /ping requests and measure total round-trip time.
//!
//! Run: `cargo bench -p omnigate --bench middleware_ping`

use std::time::Duration;

use axum::{
    body::{self, Body},
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use tower::ServiceExt; // for .oneshot()

fn build_router() -> Router {
    let api = Router::new().route("/ping", get(|| async { "pong" }));
    omnigate::middleware::apply(api)
}

/// Issue a single in-process GET /ping and return (status, body_len).
async fn hit_ping(router: &Router) -> (StatusCode, usize) {
    let req = Request::builder()
        .method("GET")
        .uri("/ping")
        .body(Body::empty())
        .expect("request");

    let resp = router.clone().oneshot(req).await.expect("response");
    let status = resp.status();

    // axum 0.7: to_bytes requires an explicit limit
    let bytes = body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("to_bytes");

    (status, bytes.len())
}

fn bench_middleware_ping(c: &mut Criterion) {
    // Build router once; cloned per-req by ServiceExt::oneshot
    let router = build_router();

    // Current-thread Tokio runtime (cheap to drive short async ops)
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio rt");

    let mut group = c.benchmark_group("omnigate/middleware_ping");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(40);
    group.throughput(Throughput::Elements(1));

    group.bench_function("GET /ping (in-process)", |b| {
        b.iter_batched(
            || router.clone(),
            |svc| {
                rt.block_on(async move {
                    let (status, len) = hit_ping(&svc).await;
                    assert_eq!(status, StatusCode::OK);
                    black_box(len);
                });
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_middleware_ping);
criterion_main!(benches);
