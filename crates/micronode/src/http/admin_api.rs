// crates/micronode/src/http/admin_api.rs
//! svc-admin compatible admin API for micronode.
//!
//! This crate is still "best-effort" on system stats. The key is:
//! - keep shapes stable for svc-admin
//! - keep readiness truthful via ReadyProbes
//! - keep storage truthful: micronode KV is in-memory ("amnesia") by default

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use parking_lot::Mutex;
use serde::Serialize;

// sysinfo v0.30+ removed the old *Ext traits; methods live on the concrete types.
use sysinfo::{Disks, Networks, System};

use crate::{
    config::schema::StorageEngine, observability::metrics as obs_metrics, state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/system/summary", get(system_summary))
        .route("/storage/summary", get(storage_summary))
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// Backward-compatible profile hint consumed by existing svc-admin code.
    pub profile: String,
    /// Product role in the two-node CrabLink model.
    pub node_role: String,
    /// Runtime profile backing this role.
    pub node_profile: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub capabilities: Vec<String>,
    pub amnesia_mode: bool,
    pub privacy_mode: bool,
    pub public_inbound_enabled: bool,
    pub verification_enabled: bool,
    pub content_serving_enabled: bool,
    pub economic_replay_enabled: bool,
    pub service_quorum_enabled: bool,
    pub wallet_execution_participant: bool,
    pub ledger_replay_enabled: bool,
    pub user_ip_publication: String,
    pub peer_ip_display: String,
    pub admin_bind_loopback_only: bool,
    pub admin_bind_publication: bool,
    pub transport_routes_public: bool,
    pub raw_socket_publication: bool,
    pub passive_runtime: PassiveRuntimeStatus,
    pub planes: Vec<PlaneStatus>,
}

#[derive(Debug, Serialize)]
pub struct PlaneStatus {
    pub name: String,
    /// IMPORTANT: svc-admin UI expects: "healthy" | "degraded" | "down"
    /// (Anything else renders as "Unknown".)
    pub health: String,
    pub ready: bool,
    pub restart_count: u64,
}

#[derive(Debug, Serialize)]
pub struct PassiveWorkerStatus {
    pub enabled: bool,
    /// Worker lifecycle state such as "active", "disabled", or "stubbed".
    /// None of these states claims reward or ledger finality.
    pub status: String,
    pub pending_items: u64,
    pub mutates_wallet: bool,
    pub mutates_ledger: bool,
}

#[derive(Debug, Serialize)]
pub struct PassiveRuntimeStatus {
    pub enabled: bool,
    pub lifecycle_state: String,
    pub resource_mode: String,
    pub max_cpu_percent: u8,
    pub max_background_kbps: u32,
    pub pending_evidence_limit: u32,
    pub privacy_mode: bool,
    pub public_inbound_enabled: bool,
    pub peer_ip_display: String,
    pub verification_queue: PassiveWorkerStatus,
    pub economic_replay_worker: PassiveWorkerStatus,
    pub confirmed_roc_minor_units: Option<String>,
    pub confirmed_roc_source: String,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSummaryResponse {
    pub updated_at: String,
    pub cpu_percent: f64,
    pub ram_total_bytes: u64,
    pub ram_used_bytes: u64,
    pub net_rx_bps: u64,
    pub net_tx_bps: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummaryResponse {
    pub updated_at: String,
    pub node_id: String,
    pub mount: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub db_count: u64,

    // Extra truth for micronode (svc-admin can ignore unknown fields safely):
    pub engine: String,  // "mem" (amnesia)
    pub ephemeral: bool, // true
}

fn now_unix_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs()
}

fn now_best_effort_timestamp_string() -> String {
    // No chrono/time dependency here — keep it dependency-free and stable.
    // svc-admin currently just displays this string.
    // Format: "<unix-seconds>"
    now_unix_secs().to_string()
}

fn avg_cpu_percent(sys: &System) -> f64 {
    let cpus = sys.cpus();
    if cpus.is_empty() {
        return 0.0;
    }
    let sum: f64 = cpus.iter().map(|c| c.cpu_usage() as f64).sum();
    sum / (cpus.len() as f64)
}

// sysinfo memory units can vary by platform/version historically.
// On your side you already fixed the "8TiB instead of 8GiB" issue.
// We keep a conservative conversion here: treat values as KiB and convert once.
// If you ever see 1024x inflation again, remove the `* 1024`.
fn mem_to_bytes_kib_assumption(v: u64) -> u64 {
    v.saturating_mul(1024)
}

#[derive(Debug)]
struct SysRateState {
    sys: System,
    last_sample: Instant,
    last_rx_total: u64,
    last_tx_total: u64,
    initialized: bool,
}

impl SysRateState {
    fn new() -> Self {
        let mut sys = System::new();
        // Prime the caches so cpu_usage() has a baseline.
        sys.refresh_cpu();
        sys.refresh_memory();

        Self {
            sys,
            last_sample: Instant::now(),
            last_rx_total: 0,
            last_tx_total: 0,
            initialized: false,
        }
    }
}

static SYS_STATE: Mutex<Option<SysRateState>> = Mutex::new(None);

fn with_sys_state<R>(f: impl FnOnce(&mut SysRateState) -> R) -> R {
    let mut guard = SYS_STATE.lock();
    if guard.is_none() {
        *guard = Some(SysRateState::new());
    }
    f(guard.as_mut().unwrap())
}

pub fn build_status_response(st: &AppState) -> StatusResponse {
    let ready = st.probes.snapshot().required_ready();
    let health_ok = st.health.all_ready();

    // Planes are “conceptual but truthful”: they reflect readiness + health.
    let plane_health = |ok: bool| {
        if ok {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    };

    let planes = vec![
        PlaneStatus {
            name: "admin".to_string(),
            health: plane_health(health_ok),
            ready,
            restart_count: 0,
        },
        PlaneStatus {
            name: "kv".to_string(),
            health: plane_health(ready),
            ready,
            restart_count: 0,
        },
        PlaneStatus {
            name: "facets".to_string(),
            health: plane_health(ready),
            ready,
            restart_count: 0,
        },
    ];

    let amnesia_mode = matches!(st.cfg.storage.engine, StorageEngine::Mem);
    let public_inbound_enabled = !st.cfg.server.bind.ip().is_loopback();
    let admin_bind_loopback_only = st.cfg.server.bind.ip().is_loopback();
    let verification_queue_enabled = st.cfg.user_node.verification_queue_enabled;
    let economic_replay_worker_enabled = st.cfg.user_node.economic_replay_worker_enabled;
    let verification_paused = st.verification_paused();

    StatusResponse {
        profile: "micronode".to_string(),
        node_role: "user_node".to_string(),
        node_profile: "micronode".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        // Use our own process start time (truthful + stable across sysinfo versions/platforms).
        uptime_seconds: st.uptime_seconds(),
        capabilities: vec![
            "admin_api_v1".to_string(),
            "kv_v1".to_string(),
            "storage_mem_amnesia".to_string(),
            "user_node_status_v1".to_string(),
            "passive_user_node_runtime_v1".to_string(),
            "object_verification_queue_v1".to_string(),
            "economic_replay_stub_v1".to_string(),
            "private_by_default".to_string(),
        ],
        amnesia_mode,
        privacy_mode: true,
        public_inbound_enabled,
        // Verification is a real bounded evidence queue. Economic replay remains
        // parked. Neither surface claims reward, wallet, ledger, or finality truth.
        verification_enabled: verification_queue_enabled,
        content_serving_enabled: false,
        economic_replay_enabled: economic_replay_worker_enabled,
        service_quorum_enabled: false,
        wallet_execution_participant: false,
        ledger_replay_enabled: false,
        user_ip_publication: "forbidden".to_string(),
        peer_ip_display: "forbidden".to_string(),
        admin_bind_loopback_only,
        admin_bind_publication: false,
        transport_routes_public: false,
        raw_socket_publication: false,
        passive_runtime: PassiveRuntimeStatus {
            enabled: st.cfg.user_node.passive_runtime_enabled,
            lifecycle_state: if verification_paused {
                "paused".to_string()
            } else {
                "active".to_string()
            },
            resource_mode: st.cfg.user_node.resource_mode.as_str().to_string(),
            max_cpu_percent: st.cfg.user_node.max_cpu_percent,
            max_background_kbps: st.cfg.user_node.max_background_kbps,
            pending_evidence_limit: st.cfg.user_node.pending_evidence_limit,
            privacy_mode: true,
            public_inbound_enabled,
            peer_ip_display: "forbidden".to_string(),
            verification_queue: PassiveWorkerStatus {
                enabled: verification_queue_enabled,
                status: if !verification_queue_enabled {
                    "disabled".to_string()
                } else if verification_paused {
                    "paused".to_string()
                } else {
                    "active".to_string()
                },
                pending_items: u64::try_from(st.verification.pending_count()).unwrap_or(u64::MAX),
                mutates_wallet: false,
                mutates_ledger: false,
            },
            economic_replay_worker: PassiveWorkerStatus {
                enabled: economic_replay_worker_enabled,
                status: if !economic_replay_worker_enabled {
                    "disabled".to_string()
                } else if verification_paused {
                    "paused".to_string()
                } else {
                    "stubbed".to_string()
                },
                pending_items: 0,
                mutates_wallet: false,
                mutates_ledger: false,
            },
            confirmed_roc_minor_units: None,
            confirmed_roc_source: "wallet_ledger_receipt_only".to_string(),
            wallet_mutation: false,
            ledger_mutation: false,
        },
        planes,
    }
}

pub async fn status(State(st): State<AppState>) -> impl IntoResponse {
    // facet freshness metric (svc-admin expects these)
    obs_metrics::observe_facet_ok("admin.status");

    // Keep gauges updated so svc-admin can treat micronode like other nodes.
    let ready = st.probes.snapshot().required_ready();
    obs_metrics::update_micronode_metrics(st.started_at.elapsed(), ready);

    Json(build_status_response(&st))
}

pub async fn system_summary(State(st): State<AppState>) -> impl IntoResponse {
    obs_metrics::observe_facet_ok("admin.system_summary");

    let (cpu_percent, ram_total_bytes, ram_used_bytes, rx_bps, tx_bps) = with_sys_state(|st_sys| {
        st_sys.sys.refresh_cpu();
        st_sys.sys.refresh_memory();

        let cpu_percent = avg_cpu_percent(&st_sys.sys);

        let ram_total_bytes = mem_to_bytes_kib_assumption(st_sys.sys.total_memory());
        let ram_used_bytes = mem_to_bytes_kib_assumption(st_sys.sys.used_memory());

        // Network totals (best-effort): sum all interfaces.
        let nets = Networks::new_with_refreshed_list();
        let mut rx_total: u64 = 0;
        let mut tx_total: u64 = 0;

        for (_name, data) in nets.iter() {
            rx_total = rx_total.saturating_add(data.received());
            tx_total = tx_total.saturating_add(data.transmitted());
        }

        let now = Instant::now();
        let dt = now.duration_since(st_sys.last_sample);
        let dt_secs = dt.as_secs_f64().max(0.001);

        let (rx_bps, tx_bps) = if st_sys.initialized {
            let rx_delta = rx_total.saturating_sub(st_sys.last_rx_total) as f64;
            let tx_delta = tx_total.saturating_sub(st_sys.last_tx_total) as f64;
            ((rx_delta / dt_secs) as u64, (tx_delta / dt_secs) as u64)
        } else {
            (0, 0)
        };

        st_sys.last_sample = now;
        st_sys.last_rx_total = rx_total;
        st_sys.last_tx_total = tx_total;
        st_sys.initialized = true;

        (cpu_percent, ram_total_bytes, ram_used_bytes, rx_bps, tx_bps)
    });

    // Update readiness/uptime gauges too (cheap + helpful for dashboard)
    let ready = st.probes.snapshot().required_ready();
    obs_metrics::update_micronode_metrics(st.started_at.elapsed(), ready);

    Json(SystemSummaryResponse {
        updated_at: now_best_effort_timestamp_string(),
        cpu_percent,
        ram_total_bytes,
        ram_used_bytes,
        net_rx_bps: rx_bps,
        net_tx_bps: tx_bps,
    })
}

pub async fn storage_summary(State(st): State<AppState>) -> impl IntoResponse {
    obs_metrics::observe_facet_ok("admin.storage_summary");

    // Host disk totals (best-effort):
    let disks = Disks::new_with_refreshed_list();

    // Pick the disk mounted at "/" if present, else first disk.
    let mut chosen = None;
    for d in disks.iter() {
        if d.mount_point().to_string_lossy() == "/" {
            chosen = Some(d);
            break;
        }
    }
    let d = chosen.or_else(|| disks.iter().next());

    let (mount, fs_type, total, avail) = if let Some(d) = d {
        (
            d.mount_point().to_string_lossy().to_string(),
            d.file_system().to_string_lossy().to_string(),
            d.total_space(),
            d.available_space(),
        )
    } else {
        ("/".to_string(), "unknown".to_string(), 0, 0)
    };

    let used = total.saturating_sub(avail);

    // Update gauges too.
    let ready = st.probes.snapshot().required_ready();
    obs_metrics::update_micronode_metrics(st.started_at.elapsed(), ready);

    Json(StorageSummaryResponse {
        updated_at: now_best_effort_timestamp_string(),
        node_id: "micronode".to_string(),
        mount,
        fs_type,
        total_bytes: total,
        used_bytes: used,
        free_bytes: avail,
        db_count: 1,
        engine: "mem".to_string(),
        ephemeral: true,
    })
}
