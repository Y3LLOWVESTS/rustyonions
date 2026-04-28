// crates/macronode/src/http_admin/handlers/system_net_accounting.rs

//! RO:WHAT — `/api/v1/system/net/accounting` handler (bytes+req rollups + series).
//! RO:WHY  — svc-admin needs truthful windows + chart-ready series.
//! RO:INVARIANTS — camelCase JSON; bounded response sizes; no lock across .await

#![forbid(unsafe_code)]

use axum::{response::IntoResponse, Json};

use crate::observability::{metrics::observe_facet_ok, net_accounting};

pub async fn handler() -> impl IntoResponse {
    observe_facet_ok("admin.system.net_accounting");
    Json(net_accounting::snapshot())
}
