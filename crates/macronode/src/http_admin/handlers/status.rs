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

/// Truthful OAP foundation posture.
///
/// Phase 8 exposes the canonical protocol limits but does not claim that the
/// Phase 9 OBJ_GET serving runtime or full-digest verification path is active.
#[derive(Serialize)]
struct OapFoundationStatus {
    protocol: &'static str,
    version: u16,
    runtime_state: &'static str,
    max_frame_bytes: u32,
    stream_chunk_bytes: usize,
    object_fetch_active: bool,
    full_digest_verification_active: bool,
}

/// Truthful service-provider posture.
///
/// The embedded DHT router and local provider store are active when the DHT
/// listener is bound. Network advertisement and discoverable provider
/// publication remain inactive and must not be inferred from local readiness.
#[derive(Serialize)]
struct ProviderStatus {
    state: &'static str,
    dht_worker_status: &'static str,
    advertisement_active: bool,
    provider_records_published: u64,
    public_node_uri_format: &'static str,
    residential_ip_publication: bool,
}

/// Truthful service-node policy posture.
///
/// Serve-time policy gating belongs to Phase 9. Operator moderation, blocklists,
/// pruning, quarantine, and tombstones belong to Phase 10.
#[derive(Serialize)]
struct PolicyStatus {
    state: &'static str,
    serve_policy_enforced: bool,
    oap_serve_policy_enforced: bool,
    operator_moderation_active: bool,
    global_moderation_active: bool,
    moderation_configured: bool,
    moderation_state: &'static str,
    moderation_source: &'static str,
    moderation_load_failed: bool,
    signed_policy_verified: bool,
    signed_policy_epoch: Option<u64>,
    signed_policy_expires_at_unix_s: Option<u64>,
    rollback_guard_persisted: bool,
    moderation_activation: &'static str,
    moderation_hot_reload: bool,
    moderation_entries: ModerationEntryCountsStatus,
    unvetted_persistence_posture: &'static str,
    serve_gate_phase: &'static str,
    moderation_phase: &'static str,
}

#[derive(Serialize)]
struct ModerationEntryCountsStatus {
    total: u64,
    global_deny: u64,
    local_block: u64,
    local_allow: u64,
    owner_tombstone: u64,
    quarantine: u64,
}

/// Reward-recipient posture projected into the main node status.
///
/// This remains runtime-local request/display state. It is not registry,
/// wallet, ledger, payout, or confirmed-ROC truth.
#[derive(Serialize)]
struct RewardBindingStatus {
    state: &'static str,
    reward_recipient_display_address: Option<String>,
    pending_rotation_display_address: Option<String>,
    updated_at_unix_s: Option<u64>,
    registry_finality: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
    confirmed_roc_minor_units: Option<u64>,
}

/// Truthful service-evidence runtime posture.
///
/// This reports only the bounded process-local outbox. It does not claim
/// durable storage, network-wide replay protection, accounting acceptance,
/// reward eligibility, payout approval, wallet mutation, or ledger mutation.
#[derive(Serialize)]
struct ServiceEvidenceStatus {
    state: &'static str,
    queued_records: usize,
    signature_required: bool,
    replay_scope: &'static str,
    durable: bool,
    accounting_accepted: bool,
    reward_eligible: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
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
    /// Product role in the two-node CrabLink model.
    node_role: &'static str,
    /// Runtime profile backing the product role.
    node_profile: &'static str,
    /// Capability labels for operator dashboards.
    capabilities: Vec<&'static str>,
    /// Whether the node reports amnesia-first operation.
    amnesia_mode: bool,
    /// Whether this is a privacy-first user node.
    privacy_mode: bool,
    /// Whether the configured admin/service bind is publicly reachable.
    public_inbound_enabled: bool,
    /// Whether the node can operate without any browser/admin UI.
    headless_mode: bool,
    /// Whether the optional local operator UI is enabled.
    admin_ui_enabled: bool,
    /// Bind address for the optional operator UI.
    admin_ui_bind: String,
    /// Operator UI profile label, e.g. service_node_local.
    operator_ui_profile: String,
    /// Whether the daemon requires the UI to operate. Must remain false.
    admin_ui_runtime_required: bool,
    /// Whether a runtime setup token is currently active.
    setup_token_active: bool,
    /// OAP protocol foundation and active-runtime truth.
    oap: OapFoundationStatus,
    /// Provider advertisement and DHT-worker truth.
    provider: ProviderStatus,
    /// Serve-policy and operator-moderation truth.
    policy: PolicyStatus,
    /// Runtime-local reward recipient binding truth.
    reward_binding: RewardBindingStatus,
    /// Bounded process-local reviewed service-evidence posture.
    service_evidence: ServiceEvidenceStatus,
    /// Whether passive user verification is active on this node.
    verification_enabled: bool,
    /// Whether this node is serving content/service surfaces.
    content_serving_enabled: bool,
    /// Whether read-only economic replay/audit work is active.
    economic_replay_enabled: bool,
    /// Whether service-node quorum participation is active.
    service_quorum_enabled: bool,
    /// Whether this node participates in wallet execution.
    wallet_execution_participant: bool,
    /// Whether ledger replay/audit work is active.
    ledger_replay_enabled: bool,
    /// User-IP publication posture.
    user_ip_publication: &'static str,
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

#[derive(Clone, Copy)]
struct PlaneRestartCounts {
    gateway: u64,
    storage: u64,
    index: u64,
    mailbox: u64,
    overlay: u64,
    dht: u64,
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
    restarts: PlaneRestartCounts,
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
        restart_count: restarts.gateway,
    });

    // Storage plane (kv/blob/index backing services).
    let storage_status = svc_status(services, "svc-storage");
    planes.push(PlaneStatusBody {
        name: "storage",
        health: status_label_to_health(storage_status),
        ready: node_ready && storage_status == "ok",
        restart_count: restarts.storage,
    });

    // Index plane.
    let index_status = svc_status(services, "svc-index");
    planes.push(PlaneStatusBody {
        name: "index",
        health: status_label_to_health(index_status),
        ready: node_ready && index_status == "ok",
        restart_count: restarts.index,
    });

    // Mailbox plane.
    let mailbox_status = svc_status(services, "svc-mailbox");
    planes.push(PlaneStatusBody {
        name: "mailbox",
        health: status_label_to_health(mailbox_status),
        ready: node_ready && mailbox_status == "ok",
        restart_count: restarts.mailbox,
    });

    // Overlay plane.
    let overlay_status = svc_status(services, "svc-overlay");
    planes.push(PlaneStatusBody {
        name: "overlay",
        health: status_label_to_health(overlay_status),
        ready: node_ready && overlay_status == "ok",
        restart_count: restarts.overlay,
    });

    // DHT plane.
    let dht_status = svc_status(services, "svc-dht");
    planes.push(PlaneStatusBody {
        name: "dht",
        health: status_label_to_health(dht_status),
        ready: node_ready && dht_status == "ok",
        restart_count: restarts.dht,
    });

    planes
}

pub async fn handler(state: axum::extract::State<AppState>) -> impl IntoResponse {
    let AppState {
        cfg,
        probes,
        runtime,
        started_at,
        operator,
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
        network: if snap.listeners_bound {
            "ok"
        } else {
            "pending"
        },
        gateway: if snap.gateway_bound { "ok" } else { "pending" },
        storage: if snap.storage_bound { "ok" } else { "pending" },
    };

    // Per-service view using the richer ReadySnapshot bits.
    let mut services = BTreeMap::new();

    // Gateway: real listener + readiness bit.
    services.insert(
        "svc-gateway".to_string(),
        if snap.gateway_bound { "ok" } else { "pending" }.to_string(),
    );

    // Storage: real embedded listener readiness.
    services.insert(
        "svc-storage".to_string(),
        if snap.storage_bound { "ok" } else { "pending" }.to_string(),
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
        PlaneRestartCounts {
            gateway: snap.gateway_restart_count,
            storage: snap.storage_restart_count,
            index: snap.index_restart_count,
            mailbox: snap.mailbox_restart_count,
            overlay: snap.overlay_restart_count,
            dht: snap.dht_restart_count,
        },
    );

    // Version string matches `/version` handler (BuildInfo::current()).
    let version = BuildInfo::current().version.to_string();

    // Take one reward-recipient snapshot so all projected fields represent the
    // same runtime-local state.
    let reward_binding = operator.reward_recipient_snapshot();
    let moderation = runtime.moderation_snapshot();
    let service_evidence_outbox = runtime.service_evidence_outbox();

    Json(StatusBody {
        uptime_seconds: uptime,
        profile: "macronode",
        node_role: "service_node",
        node_profile: "macronode",
        capabilities: vec![
            "admin_api_v1",
            "service_node_status_v1",
            "headless_operator_status_v1",
            "optional_admin_ui_v1",
            "content_service_shell",
            "service_supervision",
            "oap_foundation_status_v1",
            "oap_object_fetch_v1",
            "provider_status_v1",
            "policy_status_v1",
            "moderation_runtime_status_v1",
            "signed_moderation_policy_v1",
            "local_prune_v1",
            "reward_binding_status_v1",
            "service_evidence_outbox_v1",
        ],
        amnesia_mode: false,
        privacy_mode: false,
        public_inbound_enabled: !cfg.http_addr.ip().is_loopback(),
        headless_mode: cfg.headless_mode,
        admin_ui_enabled: operator.admin_ui_enabled(),
        admin_ui_bind: cfg.admin_ui_bind.to_string(),
        operator_ui_profile: cfg.operator_ui_profile.clone(),
        admin_ui_runtime_required: cfg.admin_ui_runtime_required,
        setup_token_active: operator.setup_token_active(),
        oap: OapFoundationStatus {
            protocol: "oap/1",
            version: oap::OAP_VERSION,
            runtime_state: "active_local_http_oap",
            max_frame_bytes: oap::MAX_FRAME_BYTES,
            stream_chunk_bytes: oap::STREAM_CHUNK_SIZE,
            object_fetch_active: true,
            full_digest_verification_active: true,
        },
        provider: ProviderStatus {
            state: if snap.dht_bound {
                "local_provider_store_active_not_advertising"
            } else {
                "pending"
            },
            dht_worker_status: if snap.dht_bound {
                "active_embedded_router"
            } else {
                "pending"
            },
            advertisement_active: false,
            provider_records_published: 0,
            public_node_uri_format: "crab://node/<node-id>",
            residential_ip_publication: false,
        },
        policy: PolicyStatus {
            state: "all_object_read_policy_active",
            // Legacy GET/HEAD and OAP OBJ_GET share the same immutable
            // moderation snapshot before storage access.
            serve_policy_enforced: true,
            oap_serve_policy_enforced: true,
            operator_moderation_active: moderation.active() && moderation.source != "signed_global",
            global_moderation_active: moderation.active() && moderation.signed_policy_verified,
            moderation_configured: moderation.configured(),
            moderation_state: moderation.state,
            moderation_source: moderation.source,
            moderation_load_failed: moderation.load_failed(),
            signed_policy_verified: moderation.signed_policy_verified,
            signed_policy_epoch: moderation.signed_policy_epoch,
            signed_policy_expires_at_unix_s: moderation.signed_policy_expires_at_unix_s,
            rollback_guard_persisted: moderation.rollback_guard_persisted,
            moderation_activation: "startup_snapshot",
            moderation_hot_reload: false,
            moderation_entries: ModerationEntryCountsStatus {
                total: moderation.counts.total(),
                global_deny: moderation.counts.global_deny,
                local_block: moderation.counts.local_block,
                local_allow: moderation.counts.local_allow,
                owner_tombstone: moderation.counts.owner_tombstone,
                quarantine: moderation.counts.quarantine,
            },
            unvetted_persistence_posture: "amnesia_first",
            serve_gate_phase: "phase_10_all_object_reads_active",
            moderation_phase: "phase_10",
        },
        reward_binding: RewardBindingStatus {
            state: reward_binding.state,
            reward_recipient_display_address: reward_binding.reward_recipient_display_address,
            pending_rotation_display_address: reward_binding.pending_rotation_display_address,
            updated_at_unix_s: reward_binding.updated_at_unix_s,
            registry_finality: false,
            wallet_mutation: false,
            ledger_mutation: false,
            confirmed_roc_minor_units: None,
        },
        service_evidence: ServiceEvidenceStatus {
            state: "bounded_process_local_outbox",
            queued_records: service_evidence_outbox.len(),
            signature_required: true,
            replay_scope: "bounded_process_local",
            durable: false,
            accounting_accepted: false,
            reward_eligible: false,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,
        },
        verification_enabled: false,
        content_serving_enabled: true,
        economic_replay_enabled: false,
        service_quorum_enabled: false,
        wallet_execution_participant: false,
        ledger_replay_enabled: false,
        user_ip_publication: "not_applicable_service_node",
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
