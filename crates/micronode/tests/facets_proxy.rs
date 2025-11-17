// crates/micronode/tests/facets_proxy.rs
//! RO:WHAT — Integration test for Micronode facet hosting.
//! RO:WHY  — Prove that we can mount a facet (demo) and reach it via HTTP
//!           using the same in-process router used by benches.
//! RO:INTERACTS — build_router(Config::default()), facets::mount().

use axum::body::Body;
use http::{Request, StatusCode};
use micronode::{build_router, config::schema::Config};
use tower::ServiceExt as _; // for Router::oneshot

#[tokio::test]
async fn demo_facet_ping_works() {
    let cfg = Config::default();
    let (router, _state) = build_router(cfg);

    let resp = router
        .oneshot(
            Request::builder().method("GET").uri("/facets/demo/ping").body(Body::empty()).unwrap(),
        )
        .await
        .expect("router error");

    assert_eq!(resp.status(), StatusCode::OK);
}
