// crates/macronode/src/http_admin/handlers/status.rs

//! RO:WHAT — `/api/v1/status` handler.
//! RO:WHY  — Give operators a basic runtime + readiness + service snapshot
//!           in one call.
//!
//! RO:INTERACTS —
//!   - Uses `AppState` for config + probes + start time.
//!   - Reuses the same readiness logic as `/readyz` via `ReadyProbes::snapshot()`.
//!
//! RO:INVARIANTS —
//!   - `ready` field matches the `required_ready()` gate used by `/readyz`.
//!   - `deps` mirrors the high-level `/readyz` dependency labels
//!     (config/network/gateway/storage).
//!   - `services` is a low-cardinality map of core services macronode supervises.
//!   - No blocking I/O; cheap and safe to call frequently.

use std::{collections::BTreeMap, time::Instant};

use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::{observability::metrics::update_macronode_metrics, types::AppState};

#[derive(Serialize)]
struct StatusDeps {
    config: &'static str,
    network: &'static str,
    gateway: &'static str,
    storage: &'static str,
}

#[derive(Serialize)]
struct StatusBody {
    /// Seconds since this macronode process started.
    uptime_seconds: u64,
    /// Profile name for this node (always "macronode" for this crate).
    profile: &'static str,
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

    // High-level dependency view; mirrors `/readyz` top-level deps.
    let deps = StatusDeps {
        config: if snap.cfg_loaded { "loaded" } else { "pending" },
        network: if snap.listeners_bound {
            "ok"
        } else {
            "pending"
        },
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

    Json(StatusBody {
        uptime_seconds: uptime,
        profile: "macronode",
        http_addr: cfg.http_addr.to_string(),
        metrics_addr: cfg.metrics_addr.to_string(),
        log_level: cfg.log_level.clone(),
        ready,
        deps,
        services,
    })
}
