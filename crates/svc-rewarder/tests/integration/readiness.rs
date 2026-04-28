use axum::body::Body;
use axum::http::{Request, StatusCode};
use svc_rewarder::http::routes::router;
use svc_rewarder::http::RewarderState;
use svc_rewarder::Config;
use tower::ServiceExt;

#[tokio::test]
async fn readyz_is_ok_after_state_initialization() {
    let state = RewarderState::new(Config::default()).unwrap();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_degrades_when_queue_gate_false() {
    let state = RewarderState::new(Config::default()).unwrap();
    state.health.set(|s| s.queue_ok = false);
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}
