//! GET `/o/{addr}`.
//!
//! RO:WHAT — Temporary object read stub for gateway object route.
//! RO:WHY — Keeps route shape available until real object proxy/read path is wired.
//! RO:INTERACTS — `headers::etag::etag_from_b3`.
//! RO:INVARIANTS — stub only; no storage writes; no wallet/ledger mutation.
//! RO:METRICS — route-level HTTP metrics when mounted.
//! RO:CONFIG — none.
//! RO:SECURITY — public stub response only.
//! RO:TEST — covered indirectly by gateway route smoke once mounted.

use crate::headers::etag::etag_from_b3;
use axum::{extract::Path, response::IntoResponse};

/// Return a temporary object stub response.
#[must_use]
pub fn get_object(Path(addr): Path<String>) -> impl IntoResponse {
    (
        [(http::header::ETAG, etag_from_b3(&addr))],
        axum::body::Body::from(format!("object stub for {addr}")),
    )
}
