//! RO:WHAT   v1: /ping + /index/healthz + /index/search + /sleep (bounded).
//! RO:WHY    Fast client confidence checks and a simple load helper for readiness smoke.
//! RO:INVARS JSON DTOs are stable and tiny; 200 on success. /sleep clamps ms ≤ 1000.

use axum::{
    extract::Query,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
pub struct PingResponse {
    pub ok: bool,
}

#[derive(Serialize)]
pub struct HealthReply {
    pub ok: bool,
}

#[derive(Deserialize, Serialize)]
pub struct SearchRequest {
    pub q: String,
}

#[derive(Serialize)]
pub struct SearchReply {
    pub ok: bool,
    pub echoed: String,
}

#[derive(Deserialize)]
pub struct SleepQ {
    /// Milliseconds to sleep; clamped to ≤ 1000. Default: 500.
    pub ms: Option<u64>,
}

#[derive(Serialize)]
pub struct SleepReply {
    pub ok: bool,
    pub slept_ms: u64,
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/ping", get(ping))
        .route("/index/healthz", get(healthz))
        .route("/index/search", post(search_echo))
        .route("/sleep", get(sleep_ms))
}

pub async fn ping() -> Json<PingResponse> {
    Json(PingResponse { ok: true })
}

pub async fn healthz() -> Json<HealthReply> {
    Json(HealthReply { ok: true })
}

pub async fn search_echo(Json(body): Json<SearchRequest>) -> Json<SearchReply> {
    Json(SearchReply {
        ok: true,
        echoed: body.q,
    })
}

/// Bounded sleep helper for readiness/inflight smoke tests.
pub async fn sleep_ms(Query(q): Query<SleepQ>) -> Json<SleepReply> {
    let ms = q.ms.unwrap_or(500).min(1_000);
    tokio::time::sleep(Duration::from_millis(ms)).await;
    Json(SleepReply {
        ok: true,
        slept_ms: ms,
    })
}
