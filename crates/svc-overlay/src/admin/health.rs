//! /healthz — liveness only (process loop tick).

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    alive: bool,
}

pub async fn healthz() -> impl IntoResponse {
    Json(Health { alive: true })
}
