//! GET `/o/{addr}` with Range support placeholder.
//!
//! RO:WHAT — Temporary range-read stub for gateway object route.
//! RO:WHY — Keeps range route shape visible until real range proxy/read path is wired.
//! RO:INTERACTS — Axum route table only.
//! RO:INVARIANTS — stub only; no storage writes; no wallet/ledger mutation.
//! RO:METRICS — route-level HTTP metrics when mounted.
//! RO:CONFIG — none.
//! RO:SECURITY — public stub response only.
//! RO:TEST — covered indirectly by gateway route smoke once mounted.

use axum::{extract::Path, response::IntoResponse};

/// Return a temporary range-read stub response.
#[must_use]
pub fn get_range(Path(addr): Path<String>) -> impl IntoResponse {
    (
        http::StatusCode::NOT_IMPLEMENTED,
        format!("range read stub for {addr}"),
    )
}
