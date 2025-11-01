//! RO:WHAT   v1: /dht health stub (client passthrough soon).
//! RO:INVARS 200/JSON only.

use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthReply {
    pub ok: bool,
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/healthz", get(healthz))
}

pub async fn healthz() -> Json<HealthReply> {
    Json(HealthReply { ok: true })
}
