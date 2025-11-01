//! RO:WHAT   v1/mailbox surface (health stub now; real ops later).
//! RO:INVARS Stable JSON shapes for health; S must be Send+Sync for layering.

use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthReply {
    pub ok: bool,
}

pub async fn healthz() -> Json<HealthReply> {
    Json(HealthReply { ok: true })
}

/// Minimal router for /v1/mailbox/*
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/healthz", get(healthz))
}
