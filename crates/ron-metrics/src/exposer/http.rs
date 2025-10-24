//! RO:WHAT — Axum router + handlers for /metrics, /healthz, /readyz.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use crate::{
    metrics::Metrics,
    readiness::{make_ready_json, ReadyJson, ReadyPolicy},
};
use prometheus::{Encoder, TextEncoder}; // <-- Encoder trait needed
use std::time::{Instant, SystemTime};

#[derive(Clone)]
pub struct AppState {
    pub metrics: Metrics,
    pub ready_since: SystemTime,
}

pub fn make_router(metrics: Metrics) -> Router {
    let state = AppState {
        metrics,
        ready_since: SystemTime::now(),
    };

    Router::new()
        .route("/metrics", get(get_metrics))
        .route("/healthz", get(get_healthz))
        .route("/readyz", get(get_readyz))
        .with_state(state)
}

async fn get_metrics(State(st): State<AppState>) -> impl IntoResponse {
    let t0 = Instant::now();
    let mf = st.metrics.registry().gather();

    let mut buf = Vec::with_capacity(64 * 1024);
    let enc = TextEncoder::new();

    if let Err(e) = enc.encode(&mf, &mut buf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("encode failed: {e}"),
        )
            .into_response();
    }

    let secs = t0.elapsed().as_secs_f64();
    st.metrics.observe_exposition(secs, "/metrics");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, enc.format_type())
        .body(axum::body::Body::from(buf))
        .unwrap()
}

async fn get_healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn get_readyz(State(st): State<AppState>) -> impl IntoResponse {
    let health = st.metrics.health();
    let snap = health.snapshot();
    let all_ready = snap.values().all(|v| *v);
    let missing: Vec<String> = snap
        .into_iter()
        .filter_map(|(svc, ok)| if ok { None } else { Some(svc) })
        .collect();

    let policy: ReadyPolicy = st.metrics.ready_policy();
    let body: ReadyJson = make_ready_json(all_ready, missing, policy, st.ready_since);

    if all_ready {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        let mut resp = (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response();
        let retry_after = policy.retry_after_secs.to_string();
        resp.headers_mut()
            .insert(header::RETRY_AFTER, retry_after.parse().unwrap());
        resp
    }
}
