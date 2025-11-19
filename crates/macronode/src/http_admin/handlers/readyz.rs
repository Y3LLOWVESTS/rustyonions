//! RO:WHAT — Axum adapter for `/readyz`.
//! RO:WHY  — Delegate to `readiness::handler` with shared probes.

use std::sync::Arc;

use axum::response::IntoResponse;

use crate::readiness::{self, ReadyProbes};

pub async fn handler(probes: Arc<ReadyProbes>) -> impl IntoResponse {
    readiness::handler(probes).await
}
