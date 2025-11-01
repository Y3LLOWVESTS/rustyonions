//! RO:WHAT   v1: /objects health stub (ready to swap to StorageClient).
//! RO:INVARS 200/JSON only; no internal details.

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
