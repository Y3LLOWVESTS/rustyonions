//! RO:WHAT — Example Axum API instrumented with ron-metrics middleware (latency + status-class).
//! RO:WHY  — Zero-touch HTTP visibility: /metrics + automatic histograms/counters.
//! Run: RON_METRICS_METRICS_ADDR=127.0.0.1:0 cargo run -p ron-metrics --example axum_api

use axum::{routing::get, Router};
use std::{env, net::SocketAddr, time::Duration};
use tokio::time::sleep;

use ron_metrics::{
    axum_latency,           // latency histogram middleware (request_latency_seconds)
    axum_status,            // status-class counter middleware (request_status_total)
    build_info::build_version,
    exposer::http::make_router as make_metrics_router,
    BaseLabels, HealthState, Metrics,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Base labels
    let base = BaseLabels {
        service: env::var("RON_SERVICE").unwrap_or_else(|_| "demo-api".into()),
        instance: env::var("RON_INSTANCE").unwrap_or_else(|_| "local-1".into()),
        build_version: build_version(),
        amnesia: env::var("RON_AMNESIA").unwrap_or_else(|_| "off".into()),
    };

    // Health state
    let health = HealthState::new();
    health.set("config_loaded".into(), true);
    health.set("db".into(), true);

    // Metrics
    let metrics = Metrics::new(base, health)?;

    // App routes (business endpoints)
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/sleep", get(|| async {
            // Simulate work
            sleep(Duration::from_millis(12)).await;
            "ok"
        }));

    // Expose /metrics, /healthz, /readyz on same server
    let app = app.merge(make_metrics_router(metrics.clone()));

    // Attach middlewares (order is fine either way; both apply to all routes)
    let app = axum_latency::attach(app, metrics.clone());
    let app = axum_status::attach(app, metrics.clone());

    // Bind & serve
    let bind: SocketAddr = env::var("RON_METRICS_METRICS_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:0".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    println!("api     :  http://{}/ping", addr);
    println!("sleep   :  http://{}/sleep", addr);
    println!("metrics :  http://{}/metrics", addr);
    println!("healthz :  http://{}/healthz", addr);
    println!("readyz  :  http://{}/readyz", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
