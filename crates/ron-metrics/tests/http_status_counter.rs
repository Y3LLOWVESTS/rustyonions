//! Ensures request_status_total increments and the latency histogram encodes.

use axum::{routing::get, Router};
use ron_metrics::build_info::build_version;
use ron_metrics::{
    axum_latency, axum_status, exposer::http::make_router, BaseLabels, HealthState, Metrics,
};
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn status_counter_and_latency_move() {
    let base = BaseLabels {
        service: "test-svc".into(),
        instance: "test-1".into(),
        build_version: build_version(),
        amnesia: "off".into(),
    };
    let health = HealthState::new();
    health.set("config_loaded".into(), true);
    let metrics = Metrics::new(base, health).expect("metrics");

    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .merge(make_router(metrics.clone()));
    let app = axum_latency::attach(app, metrics.clone());
    let app = axum_status::attach(app, metrics.clone());

    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let jh = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    // exercise endpoints
    let _ = reqwest::get(format!("http://{addr}/ping"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    // pull metrics text
    let body = reqwest::get(format!("http://{addr}/metrics"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // assert status counter moved (2xx)
    assert!(
        body.contains("request_status_total{status_class=\"2xx\"}"),
        "missing 2xx counter"
    );
    // assert latency histogram exported (count present)
    assert!(
        body.contains("request_latency_seconds_count"),
        "missing latency count"
    );

    drop(jh);
}
