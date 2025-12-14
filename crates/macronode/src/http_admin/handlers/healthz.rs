// crates/macronode/src/http_admin/handlers/healthz.rs

//! RO:WHAT — `/healthz` liveness handler.
//! RO:WHY  — Simple "is the process alive" probe plus a facet metric so
//!           svc-admin always has at least one facet to display.
//!
//! RO:INTERACTS — crate::observability::metrics::observe_facet_ok
//! RO:INVARIANTS —
//!   - Never blocks; cheap enough to call frequently.
//!   - If this handler returns 200, we always emit an "ok" facet event
//!     for `admin.healthz`.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::observability::metrics::observe_facet_ok;

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
    // In this slice we assume that if we can build a response at all,
    // the event loop + monotonic clock are both healthy enough.
    let checks = Checks {
        event_loop: "ok",
        clock: "ok",
    };

    // Record that the admin health facet was exercised and succeeded.
    // This shows up as:
    //   ron_facet_requests_total{facet="admin.healthz",result="ok"} N
    // in macronode's /metrics, which svc-admin then aggregates into
    // short-horizon facet summaries.
    observe_facet_ok("admin.healthz");

    Json(HealthBody { ok: true, checks })
}
