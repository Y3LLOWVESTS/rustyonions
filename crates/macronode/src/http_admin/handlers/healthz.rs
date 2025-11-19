//! RO:WHAT — `/healthz` liveness handler.
//! RO:WHY  — Simple "is the process alive" probe.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct Checks<'a> {
    event_loop: &'a str,
    clock: &'a str,
}

#[derive(Serialize)]
struct HealthBody<'a> {
    ok: bool,
    checks: Checks<'a>,
}

pub async fn handler() -> impl IntoResponse {
    let checks = Checks {
        event_loop: "ok",
        clock: "ok",
    };

    Json(HealthBody { ok: true, checks })
}
