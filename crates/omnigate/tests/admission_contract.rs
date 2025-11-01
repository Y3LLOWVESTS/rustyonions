// RO:WHAT
// Contract checks for simple "admission-style" guards wired via `from_fn_with_state`.
// These are *test-only* helpers to make sure our layering semantics and extractor
// signatures are correct under Axum 0.7.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tower::ServiceExt;

// ----- Tiny quota guard -------------------------------------------------------

#[derive(Debug, Default)]
struct Tiny {
    // "hard" is a max-requests-left style counter.
    hard: AtomicUsize,
    // "soft" unused in these tiny tests but parked here for parity with previous shape.
    #[allow(dead_code)]
    soft: AtomicUsize,
}

async fn tiny_guard(State(state): State<Arc<Tiny>>, req: Request<Body>, next: Next) -> Response {
    // If no budget left, 429 immediately (cheap shed).
    if state.hard.load(Ordering::Relaxed) == 0 {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    // Decrement and pass through.
    state.hard.fetch_sub(1, Ordering::Relaxed);
    next.run(req).await
}

// ----- "Once" guard (allow exactly one) --------------------------------------

#[derive(Debug)]
struct Once(AtomicUsize);

async fn once_guard(State(state): State<Arc<Once>>, req: Request<Body>, next: Next) -> Response {
    // First request allowed (counter set to 1), subsequent ones 429.
    if state.0.fetch_sub(1, Ordering::Relaxed) == 0 {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    next.run(req).await
}

// ----- Tests -----------------------------------------------------------------

#[tokio::test]
async fn tiny_quota_allows_then_429s() {
    // Start with budget=2.
    let tiny = Arc::new(Tiny {
        hard: AtomicUsize::new(2),
        soft: AtomicUsize::new(0),
    });

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn_with_state(tiny.clone(), tiny_guard));

    // First two requests ok…
    let res = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // …third should be shed.
    let res = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn quota_when_exhausted_429() {
    // Allow exactly one request.
    let once = Arc::new(Once(AtomicUsize::new(1)));

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn_with_state(once.clone(), once_guard));

    // First request OK…
    let res = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // …second is 429.
    let res = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}
