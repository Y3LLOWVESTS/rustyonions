// crates/macronode/src/http_admin/handlers/debug_crash.rs
//! RO:WHAT — `/api/v1/debug/crash` handler.
//! RO:WHY  — Dev-only hook to simulate a service crash event so we can
//!           exercise restart counters and dashboards without chaos tooling.
//!
//! RO:INVARIANTS —
//!   - Must be treated as a *dangerous* endpoint; guarded by the same
//!     admin auth middleware as `/api/v1/shutdown` and `/api/v1/reload`.
//!   - Emits a `NodeEvent::ServiceCrashed` onto the intra-node bus.
//!   - For v1 it also directly bumps restart counters in `ReadyProbes` so
//!     dashboards can observe the effect without real restarts.
//!   - Emits facet metrics for `admin.debug_crash` so svc-admin can show
//!     per-node facet RPS/error rate for this dev-only tool.

use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    bus::NodeEvent,
    observability::metrics::{observe_facet_error, observe_facet_ok},
    types::AppState,
};

/// Query parameters for the debug crash endpoint.
///
/// Example:
///   POST /api/v1/debug/crash?service=svc-storage
#[derive(Debug, Deserialize)]
pub struct DebugCrashQuery {
    /// Logical service name to "crash".
    ///
    /// Expected values (v1):
    ///   - "svc-gateway"
    ///   - "svc-storage"
    ///   - "svc-index"
    ///   - "svc-mailbox"
    ///   - "svc-overlay"
    ///   - "svc-dht"
    ///
    /// If omitted, we default to `"svc-storage"`.
    pub service: Option<String>,
}

#[derive(Serialize)]
struct DebugCrashResp {
    status: &'static str,
    service: String,
    note: &'static str,
}

/// POST `/api/v1/debug/crash`
///
/// Dev-only endpoint to emit a synthetic `ServiceCrashed` event on the
/// intra-node bus and bump restart counters. This does *not* yet affect
/// real worker tasks — it is purely a synthetic event generator for now.
///
/// Example:
///   curl -X POST "http://127.0.0.1:8080/api/v1/debug/crash?service=svc-storage"
pub async fn handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<DebugCrashQuery>,
) -> impl IntoResponse {
    // Default to a reasonable, low-blast-radius service.
    let service = params
        .service
        .unwrap_or_else(|| "svc-storage".to_string());

    // Constrain to the known service set for now so we don't drift from
    // the canonical registry.
    let allowed = [
        "svc-gateway",
        "svc-storage",
        "svc-index",
        "svc-mailbox",
        "svc-overlay",
        "svc-dht",
    ];

    if !allowed.contains(&service.as_str()) {
        warn!(%service, "macronode debug_crash: unknown service requested");

        // Record a facet error for invalid usage so svc-admin can surface
        // this as a non-zero error rate on the debug facet.
        observe_facet_error("admin.debug_crash");

        let body = DebugCrashResp {
            status: "invalid service",
            service,
            note: "expected one of svc-gateway,svc-storage,svc-index,svc-mailbox,svc-overlay,svc-dht",
        };
        return Json(body);
    }

    info!(
        %service,
        "macronode debug_crash: emitting synthetic ServiceCrashed event"
    );

    // 1) Emit a synthetic ServiceCrashed event on the intra-node bus.
    //    This is future-proofing for when the supervisor subscribes to
    //    node events and drives real restart logic.
    let mut facet_ok = true;
    if let Err(send_err) = state
        .bus
        .publish(NodeEvent::ServiceCrashed {
            service: service.clone(),
        })
    {
        // This is expected for now (no subscribers yet), so we log at WARN
        // but do not treat it as a fatal error for the API.
        warn!(
            %service,
            ?send_err,
            "macronode debug_crash: failed to publish ServiceCrashed event on bus"
        );
        facet_ok = false;
    }

    // 2) Directly bump the restart counter for this service so that
    //    `/api/v1/status` and dashboards see the effect immediately.
    state.probes.inc_restart_for(&service);

    // 3) Emit facet metrics for this dev-only admin facet.
    //
    // Semantics:
    //   - If we could publish the synthetic event, treat as "ok".
    //   - If bus publish failed (miswired / no bus), treat as "error".
    if facet_ok {
        observe_facet_ok("admin.debug_crash");
    } else {
        observe_facet_error("admin.debug_crash");
    }

    let body = DebugCrashResp {
        status: "debug crash event emitted",
        service,
        note: "restart counter bumped; no real worker was killed (synthetic event)",
    };

    Json(body)
}
