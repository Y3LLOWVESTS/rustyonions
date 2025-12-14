// crates/macronode/src/http_admin/handlers/status.rs

//! RO:WHAT — `/api/v1/status` handler.
//! RO:WHY  — Give operators a basic runtime + readiness + service snapshot
//!           in one call.
//!
//! RO:INTERACTS —
//!   - Uses `AppState` for config + probes + start time.
//!   - Uses `BuildInfo` for service/version metadata.
//!   - Reuses the same readiness logic as `/readyz` via `ReadyProbes::snapshot()`.
//!   - Exposes the RON-STATUS-V1 subset (`profile`/`version`/`planes`) that
//!     svc-admin and other dashboards consume across node profiles.
//!
//! RO:INVARIANTS —
//!   - `ready` field matches the `required_ready()` gate used by `/readyz`.
//!   - `deps` mirrors the high-level `/readyz` dependency labels
//!     (config/network/gateway/storage).
//!   - `services` is a low-cardinality map of core services macronode supervises.
//!   - `planes` is derived from `services` and restart counters using a stable
//!     mapping to `{name, health, ready, restart_count}`.
//!   - No blocking I/O; cheap and safe to call frequently.

use std::{collections::BTreeMap, time::Instant};

use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::{
    observability::metrics::{observe_facet_ok, update_macronode_metrics},
    types::{AppState, BuildInfo},
};

#[derive(Serialize)]
struct StatusDeps {
    config: &'static str,
    network: &'static str,
    gateway: &'static str,
    storage: &'static str,
}

/// Per-plane status used for the RON-STATUS-V1 contract.
///
/// This mirrors the shape consumed by `svc-admin` (`PlaneStatus` inside
/// `AdminStatusView`).
#[derive(Serialize)]
struct PlaneStatusBody {
    /// Plane name (overlay/gateway/storage/index/mailbox/dht/…).
    name: &'static str,
    /// Coarse health indicator for the plane.
    ///
    /// Values:
    ///   - "healthy"
    ///   - "degraded"
    ///   - "down"
    health: &'static str,
    /// Whether this plane is considered "ready".
    ///
    /// For v1 we treat "ok" services as ready, all others as not-ready.
    ready: bool,
    /// Best-effort restart count for the plane, backed by supervisor
    /// crash counters exposed via `ReadySnapshot`.
    restart_count: u64,
}

#[derive(Serialize)]
struct StatusBody {
    /// Seconds since this macronode process started.
    uptime_seconds: u64,
    /// Profile name for this node (always "macronode" for this crate).
    profile: &'static str,
    /// Service version (semantic version or build identifier).
    ///
    /// This is the `version` field in the RON-STATUS-V1 subset and must
    /// stay stable for dashboards that diff or group by version.
    version: String,
    /// Admin HTTP bind address (where `/healthz`/`/readyz`/`/metrics` live).
    http_addr: String,
    /// Metrics bind address (currently shares the admin listener, but kept
    /// separate for future split).
    metrics_addr: String,
    /// Effective log level for this process.
    log_level: String,
    /// Whether the node considers itself "ready" according to the same
    /// gates used by `/readyz`.
    ready: bool,
    /// Per-dependency status, mirroring `/readyz`.
    deps: StatusDeps,
    /// Per-service summary.
    ///
    /// Keys:
    ///   - "svc-gateway"
    ///   - "svc-storage"
    ///   - "svc-index"
    ///   - "svc-mailbox"
    ///   - "svc-overlay"
    ///   - "svc-dht"
    ///
    /// Values are simple strings for now:
    ///   - "ok"      — service is bound and reported healthy/coarse-ok.
    ///   - "pending" — service has not yet met its readiness condition.
    services: BTreeMap<String, String>,
    /// Plane-level status used by cross-node dashboards (RON-STATUS-V1).
    ///
    /// This array is intentionally small and stable; clients like `svc-admin`
    /// rely on it to render plane tiles and aggregate health.
    planes: Vec<PlaneStatusBody>,
}

/// Map a low-level service label into a coarse health string.
///
/// Input values are the service map's `"ok"` / `"pending"` / other flags.
/// The output is one of the stable RON-STATUS-V1 health values.
fn status_label_to_health(status: &str) -> &'static str {
    match status {
        "ok" => "healthy",
        "pending" => "degraded",
        _ => "down",
    }
}

/// Build the plane list from the per-service map and restart counters.
///
/// We keep the mapping explicit and low-cardinality so that dashboards can
/// rely on a stable set of plane names. Restart counts come from the
/// readiness snapshot but we pass them in as plain u64s so this module
/// doesn’t depend on the `ReadySnapshot` type.
fn build_planes(
    services: &BTreeMap<String, String>,
    node_ready: bool,
    gateway_restart_count: u64,
    storage_restart_count: u64,
    index_restart_count: u64,
    mailbox_restart_count: u64,
    overlay_restart_count: u64,
    dht_restart_count: u64,
) -> Vec<PlaneStatusBody> {
    // Helper to read a status string from the services map with a sane default.
    fn svc_status<'a>(services: &'a BTreeMap<String, String>, key: &str) -> &'a str {
        services
            .get(key)
            .map(String::as_str)
            // Treat missing entries as "pending" so that planes show up as degraded,
            // not silently omitted.
            .unwrap_or("pending")
    }

    let mut planes = Vec::with_capacity(6);

    // Gateway plane (HTTP ingress / API surface).
    let gw_status = svc_status(services, "svc-gateway");
    planes.push(PlaneStatusBody {
        name: "gateway",
        health: status_label_to_health(gw_status),
        ready: node_ready && gw_status == "ok",
        restart_count: gateway_restart_count,
    });

    // Storage plane (kv/blob/index backing services).
    let storage_status = svc_status(services, "svc-storage");
    planes.push(PlaneStatusBody {
        name: "storage",
        health: status_label_to_health(storage_status),
        ready: node_ready && storage_status == "ok",
        restart_count: storage_restart_count,
    });

    // Index plane.
    let index_status = svc_status(services, "svc-index");
    planes.push(PlaneStatusBody {
        name: "index",
        health: status_label_to_health(index_status),
        ready: node_ready && index_status == "ok",
        restart_count: index_restart_count,
    });

    // Mailbox plane.
    let mailbox_status = svc_status(services, "svc-mailbox");
    planes.push(PlaneStatusBody {
        name: "mailbox",
        health: status_label_to_health(mailbox_status),
        ready: node_ready && mailbox_status == "ok",
        restart_count: mailbox_restart_count,
    });

    // Overlay plane.
    let overlay_status = svc_status(services, "svc-overlay");
    planes.push(PlaneStatusBody {
        name: "overlay",
        health: status_label_to_health(overlay_status),
        ready: node_ready && overlay_status == "ok",
        restart_count: overlay_restart_count,
    });

    // DHT plane.
    let dht_status = svc_status(services, "svc-dht");
    planes.push(PlaneStatusBody {
        name: "dht",
        health: status_label_to_health(dht_status),
        ready: node_ready && dht_status == "ok",
        restart_count: dht_restart_count,
    });

    planes
}

pub async fn handler(state: axum::extract::State<AppState>) -> impl IntoResponse {
    let AppState {
        cfg,
        probes,
        started_at,
        ..
    } = state.0;

    // Uptime since process start.
    let uptime = Instant::now()
        .saturating_duration_since(started_at)
        .as_secs();

    // Snapshot of readiness bits (cheap, lock-free).
    let snap = probes.snapshot();
    let ready = snap.required_ready();

    // Keep metrics in sync with what we present via status.
    update_macronode_metrics(uptime, ready);

    // Record a successful facet hit for `/api/v1/status` so svc-admin can
    // aggregate it into per-node facet metrics.
    observe_facet_ok("admin.status");

    // High-level dependency view; mirrors `/readyz` top-level deps.
    let deps = StatusDeps {
        config: if snap.cfg_loaded { "loaded" } else { "pending" },
        network: if snap.listeners_bound { "ok" } else { "pending" },
        gateway: if snap.gateway_bound { "ok" } else { "pending" },
        // Today deps_ok flips true once gateway + storage + index workers are spawned.
        storage: if snap.deps_ok { "ok" } else { "pending" },
    };

    // Per-service view using the richer ReadySnapshot bits.
    let mut services = BTreeMap::new();

    // Gateway: real listener + readiness bit.
    services.insert(
        "svc-gateway".to_string(),
        if snap.gateway_bound { "ok" } else { "pending" }.to_string(),
    );

    // Storage: coarse deps_ok is still the right gate in this slice.
    services.insert(
        "svc-storage".to_string(),
        if snap.deps_ok { "ok" } else { "pending" }.to_string(),
    );

    // Index: now tracked via its own readiness bit (index_bound).
    services.insert(
        "svc-index".to_string(),
        if snap.index_bound { "ok" } else { "pending" }.to_string(),
    );

    // Mailbox/overlay/dht: each flip a per-service bit as their worker starts.
    services.insert(
        "svc-mailbox".to_string(),
        if snap.mailbox_bound { "ok" } else { "pending" }.to_string(),
    );

    services.insert(
        "svc-overlay".to_string(),
        if snap.overlay_bound { "ok" } else { "pending" }.to_string(),
    );

    services.insert(
        "svc-dht".to_string(),
        if snap.dht_bound { "ok" } else { "pending" }.to_string(),
    );

    // Build the plane list in the shape expected by svc-admin (RON-STATUS-V1),
    // now with real restart counters from the snapshot.
    let planes = build_planes(
        &services,
        ready,
        snap.gateway_restart_count,
        snap.storage_restart_count,
        snap.index_restart_count,
        snap.mailbox_restart_count,
        snap.overlay_restart_count,
        snap.dht_restart_count,
    );

    // Version string matches `/version` handler (BuildInfo::current()).
    let version = BuildInfo::current().version.to_string();

    Json(StatusBody {
        uptime_seconds: uptime,
        profile: "macronode",
        version,
        http_addr: cfg.http_addr.to_string(),
        metrics_addr: cfg.metrics_addr.to_string(),
        log_level: cfg.log_level.clone(),
        ready,
        deps,
        services,
        planes,
    })
}
