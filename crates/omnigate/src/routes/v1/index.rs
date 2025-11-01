//! RO:WHAT   v1: /ping + /index/healthz (passthrough-ready).
//! RO:WHY    Fast client confidence checks before real traffic.
//! RO:INVARS JSON DTOs are stable and tiny; 200 on success.

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

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

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/ping", get(ping))
        .route("/index/healthz", get(healthz))
        // placeholder to prove JSON plumbing; replace with real downstream:
        .route("/index/search", post(search_echo))
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
