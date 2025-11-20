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
//!   - `deps` mirrors the `/readyz` dependency labels (config/network/gateway/storage).
//!   - `services` is a low-cardinality map of core services macronode supervises.
//!   - No blocking I/O; cheap and safe to call frequently.

use std::{collections::BTreeMap, time::Instant};

use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::types::AppState;

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
    /// separate at the config level for future slices).
    metrics_addr: String,
    /// Effective log level for this process.
    log_level: String,
    /// Whether the node considers itself "ready" according to the same
    /// gates used by `/readyz`.
    ready: bool,
    /// Per-dependency status, mirroring `/readyz`.
    deps: StatusDeps,
    /// Per-service summary (stubbed for now for non-gateway services).
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
    ///   - "ok"      — service is bound and reported healthy.
    ///   - "pending" — service has not yet met its readiness condition.
    ///   - "stub"    — service is a placeholder worker without real health.
    services: BTreeMap<String, String>,
}

pub async fn handler(state: axum::extract::State<AppState>) -> impl IntoResponse {
    let AppState {
        cfg,
        probes,
        started_at,
        ..
    } = state.0;

    let uptime = Instant::now()
        .saturating_duration_since(started_at)
        .as_secs();

    let snap = probes.snapshot();
    let ready = snap.required_ready();

    let deps = StatusDeps {
        config: if snap.cfg_loaded { "loaded" } else { "pending" },
        network: if snap.listeners_bound {
            "ok"
        } else {
            "pending"
        },
        gateway: if snap.gateway_bound { "ok" } else { "pending" },
        storage: if snap.deps_ok { "ok" } else { "pending" },
    };

    // For now, only `svc-gateway` has a real bound listener that we can
    // reflect directly. The rest are stub workers, but we still expose
    // them so operators see the intended composition.
    let mut services = BTreeMap::new();

    services.insert(
        "svc-gateway".to_string(),
        if snap.gateway_bound { "ok" } else { "pending" }.to_string(),
    );
    services.insert("svc-storage".to_string(), "stub".to_string());
    services.insert("svc-index".to_string(), "stub".to_string());
    services.insert("svc-mailbox".to_string(), "stub".to_string());
    services.insert("svc-overlay".to_string(), "stub".to_string());
    services.insert("svc-dht".to_string(), "stub".to_string());

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
