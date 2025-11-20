//! RO:WHAT — `/api/v1/shutdown` handler.
//! RO:WHY  — Allow operators to trigger a controlled process exit via HTTP.
//!           MVP: respond 202, then exit the process after a short delay so
//!           callers see a clean response before shutdown completes.

use std::time::Duration;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use tokio::time::sleep;
use tracing::info;

#[derive(Serialize)]
struct ShutdownBody<'a> {
    status: &'a str,
    delay_ms: u64,
}

/// POST `/api/v1/shutdown`
///
/// Semantics (MVP):
/// - Immediately returns `202 Accepted` with a small JSON payload.
/// - In the background, waits for a short delay and then calls `std::process::exit(0)`.
/// - This is a coarse, process-wide shutdown; we will replace this with
///   proper supervisor-driven graceful shutdown in a later pass.
pub async fn handler() -> impl IntoResponse {
    let delay_ms: u64 = 500;

    // Fire-and-forget task that will terminate the process shortly after
    // the HTTP response has been sent.
    tokio::spawn(async move {
        info!(
            "macronode admin: /api/v1/shutdown requested; exiting in {} ms",
            delay_ms
        );
        sleep(Duration::from_millis(delay_ms)).await;

        // NOTE: This is intentionally blunt for the first pass. A later
        // revision will coordinate shutdown through the supervisor so
        // services can drain gracefully.
        std::process::exit(0);
    });

    (
        StatusCode::ACCEPTED,
        Json(ShutdownBody {
            status: "shutdown scheduled",
            delay_ms,
        }),
    )
}
