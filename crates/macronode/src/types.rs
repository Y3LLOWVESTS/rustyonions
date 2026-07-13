//! RO:WHAT — Shared runtime types for Macronode.
//! RO:WHY  — Keep main/http modules thin by centralizing state and build info.
//! RO:INVARIANTS —
//!   - AppState is cheap to clone (Arc-backed).
//!   - Handlers must not hold locks across .await.
//!   - BuildInfo is stable, small, and safe to expose publicly.
//!   - OperatorState is loopback/admin-control runtime state, not durable auth truth.

#![forbid(unsafe_code)]

use std::{
    fs::File,
    io::{self, Read},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    bench::BenchManager,
    bus::NodeBus,
    config::Config,
    readiness::ReadyProbes,
    services::{
        evidence_outbox::ServiceEvidenceOutbox,
        prune::{PruneCoordinator, PruneReport},
    },
};
use ron_policy::{B3Id, ModerationPolicy};
use svc_dht::{types::CrabNodeId, ProviderStore};
use svc_index::cache::IndexCache;
use svc_storage::{
    moderation_runtime::ModerationPolicyCounts, persistence_catalog::PersistenceCatalog,
    storage::DynStorage,
};

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub probes: Arc<ReadyProbes>,

    /// Runtime service status shared by workers and admin handlers.
    ///
    /// This contains operational state and a private immutable moderation
    /// snapshot used by persistence review. Exact moderated object IDs and
    /// policy paths are never exposed through the status API.
    pub runtime: Arc<RuntimeStatus>,
    /// Intra-node event bus used for KernelEvent traffic (config updates,
    /// health changes, crash notices, etc.).
    pub bus: NodeBus,
    pub started_at: Instant,

    /// Node-executed benchmark manager (bounded, safe loadgen).
    pub bench: Arc<BenchManager>,

    /// Runtime operator state for headless-first service-node controls.
    ///
    /// This stores only local process state. It is not wallet truth, ledger truth,
    /// or durable svc-admin RBAC truth.
    pub operator: Arc<OperatorState>,
}

/// Aggregate moderation runtime snapshot safe for operator status.
#[derive(Debug, Clone, Copy)]
pub struct ModerationRuntimeSnapshot {
    pub state: &'static str,
    pub source: &'static str,
    pub counts: ModerationPolicyCounts,
    pub signed_policy_verified: bool,
    pub signed_policy_epoch: Option<u64>,
    pub signed_policy_expires_at_unix_s: Option<u64>,
    pub rollback_guard_persisted: bool,
}

impl ModerationRuntimeSnapshot {
    #[must_use]
    pub fn configured(self) -> bool {
        self.state != "not_configured"
    }

    #[must_use]
    pub fn active(self) -> bool {
        self.state == "active"
    }

    #[must_use]
    pub fn load_failed(self) -> bool {
        self.state == "load_failed"
    }
}

impl Default for ModerationRuntimeSnapshot {
    fn default() -> Self {
        Self {
            state: "not_configured",
            source: "none",
            counts: ModerationPolicyCounts::default(),
            signed_policy_verified: false,
            signed_policy_epoch: None,
            signed_policy_expires_at_unix_s: None,
            rollback_guard_persisted: false,
        }
    }
}

/// Atomic runtime view of moderation status and its effective immutable
/// policy. The policy is absent while initialization has not completed or
/// whenever moderation/storage has entered a fail-closed posture.
#[derive(Debug, Default)]
struct ModerationRuntimeState {
    snapshot: ModerationRuntimeSnapshot,
    policy: Option<Arc<ModerationPolicy>>,
}

/// Process-local operational status shared with the admin plane.
#[derive(Debug, Default)]
pub struct RuntimeStatus {
    moderation: Mutex<ModerationRuntimeState>,
    prune: PruneCoordinator,

    /// Process-local persistence-review metadata shared by the service-node
    /// admin plane.
    ///
    /// This catalog records eligibility workflow only. It does not prove that
    /// bytes were written to a durable backend.
    persistence: PersistenceCatalog,

    /// Bounded process-local stream of cryptographically reviewed service
    /// evidence. Records remain evidence-only and carry no accounting,
    /// reward, payout, wallet, or ledger authority.
    evidence: ServiceEvidenceOutbox,
}

impl RuntimeStatus {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Shared process-local persistence-review catalog.
    #[must_use]
    pub fn persistence_catalog(&self) -> &PersistenceCatalog {
        &self.persistence
    }

    /// Shared bounded service-evidence outbox.
    ///
    /// Only reviewed candidate types may be published. The outbox is
    /// process-local and is not accounting, reward, wallet, or ledger truth.
    #[must_use]
    pub fn service_evidence_outbox(&self) -> &ServiceEvidenceOutbox {
        &self.evidence
    }

    pub fn register_prune_storage(&self, storage: DynStorage) {
        self.prune.register_storage(storage);
    }

    pub fn clear_prune_storage(&self) {
        self.prune.clear_storage();
    }

    pub fn register_prune_provider_store(
        &self,
        providers: Arc<ProviderStore>,
        node_id: CrabNodeId,
    ) {
        self.prune.register_provider_store(providers, node_id);
    }

    pub fn clear_prune_provider_store(&self) {
        self.prune.clear_provider_store();
    }

    pub fn register_prune_index_cache(&self, cache: IndexCache) {
        self.prune.register_index_cache(cache);
    }

    pub fn clear_prune_index_cache(&self) {
        self.prune.clear_index_cache();
    }

    pub async fn prune_local_object(&self, object: &B3Id) -> PruneReport {
        self.prune.prune(object).await
    }

    #[must_use]
    pub fn moderation_snapshot(&self) -> ModerationRuntimeSnapshot {
        self.moderation
            .lock()
            .expect("moderation runtime status mutex poisoned")
            .snapshot
    }

    /// Return the exact immutable moderation policy currently enforced by
    /// embedded storage.
    ///
    /// The error snapshot describes why no policy is currently safe to use.
    /// Callers must fail closed rather than substitute a fabricated policy.
    pub fn persistence_moderation_policy(
        &self,
    ) -> Result<Arc<ModerationPolicy>, ModerationRuntimeSnapshot> {
        let state = self
            .moderation
            .lock()
            .expect("moderation runtime status mutex poisoned");

        state.policy.clone().ok_or(state.snapshot)
    }

    /// Activate the same explicit empty policy used by the default
    /// svc-storage router when no moderation file was configured.
    pub fn set_moderation_not_configured(&self) {
        let mut state = self
            .moderation
            .lock()
            .expect("moderation runtime status mutex poisoned");

        state.snapshot = ModerationRuntimeSnapshot::default();
        state.policy = Some(Arc::new(ModerationPolicy::default()));
    }

    pub fn set_moderation_active(
        &self,
        policy: Arc<ModerationPolicy>,
        counts: ModerationPolicyCounts,
    ) {
        let mut state = self
            .moderation
            .lock()
            .expect("moderation runtime status mutex poisoned");

        state.snapshot = ModerationRuntimeSnapshot {
            state: "active",
            source: "unsigned_local",
            counts,
            signed_policy_verified: false,
            signed_policy_epoch: None,
            signed_policy_expires_at_unix_s: None,
            rollback_guard_persisted: false,
        };
        state.policy = Some(policy);
    }

    pub fn set_signed_moderation_active(
        &self,
        source: &'static str,
        policy: Arc<ModerationPolicy>,
        counts: ModerationPolicyCounts,
        epoch: u64,
        expires_at_unix_s: u64,
    ) {
        let mut state = self
            .moderation
            .lock()
            .expect("moderation runtime status mutex poisoned");

        state.snapshot = ModerationRuntimeSnapshot {
            state: "active",
            source,
            counts,
            signed_policy_verified: true,
            signed_policy_epoch: Some(epoch),
            signed_policy_expires_at_unix_s: Some(expires_at_unix_s),
            rollback_guard_persisted: true,
        };
        state.policy = Some(policy);
    }

    pub fn set_moderation_load_failed(&self, source: &'static str) {
        let mut state = self
            .moderation
            .lock()
            .expect("moderation runtime status mutex poisoned");

        state.snapshot = ModerationRuntimeSnapshot {
            state: "load_failed",
            source,
            counts: ModerationPolicyCounts::default(),
            signed_policy_verified: false,
            signed_policy_epoch: None,
            signed_policy_expires_at_unix_s: None,
            rollback_guard_persisted: false,
        };
        state.policy = None;
    }

    pub fn set_moderation_expired(&self) {
        let mut state = self
            .moderation
            .lock()
            .expect("moderation runtime status mutex poisoned");

        if state.snapshot.signed_policy_verified {
            state.snapshot.state = "expired";
            state.policy = None;
        }
    }

    pub fn set_moderation_inactive(&self) {
        let mut state = self
            .moderation
            .lock()
            .expect("moderation runtime status mutex poisoned");

        if state.snapshot.state == "active" {
            state.snapshot.state = "inactive";
        }

        state.policy = None;
    }
}

#[derive(Clone)]
pub struct BuildInfo {
    pub service: &'static str,
    pub version: &'static str,
    pub git_sha: &'static str,
    pub build_ts: &'static str,
    pub rustc: &'static str,
    pub msrv: &'static str,
}

impl BuildInfo {
    pub fn current() -> Self {
        Self {
            service: "macronode",
            version: env!("CARGO_PKG_VERSION"),
            git_sha: option_env!("RON_GIT_SHA").unwrap_or("unknown"),
            build_ts: option_env!("RON_BUILD_TS").unwrap_or("unknown"),
            rustc: option_env!("RON_RUSTC").unwrap_or("unknown"),
            msrv: "1.80.0",
        }
    }
}

/// Runtime-only first-run setup token snapshot.
///
/// Phase 4A deliberately stops at generating and consuming a local setup token.
/// Creating durable svc-admin users/passwords remains the next svc-admin/RBAC
/// slice, so this type does not claim admin creation or login finality.
#[derive(Debug, Clone)]
pub struct SetupTokenSnapshot {
    pub token: String,
    pub setup_url: String,
    pub expires_at_unix_s: u64,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone)]
struct SetupTokenRecord {
    token: String,
    expires_at_unix_s: u64,
}

/// Runtime-local reward recipient snapshot.
///
/// This is operator-display/request state only. It is not registry finality,
/// wallet truth, ledger truth, or confirmed ROC balance truth.
#[derive(Debug, Clone)]
pub struct RewardRecipientSnapshot {
    pub state: &'static str,
    pub reward_recipient_display_address: Option<String>,
    pub pending_rotation_display_address: Option<String>,
    pub updated_at_unix_s: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct RewardRecipientRecord {
    reward_recipient_display_address: Option<String>,
    pending_rotation_display_address: Option<String>,
    updated_at_unix_s: Option<u64>,
}

/// Local operator controls shared by macronode admin handlers.
///
/// The state is intentionally tiny:
/// - admin UI enable/disable is runtime-local.
/// - setup tokens are runtime-local, short-lived, and one-use.
/// - reward recipient binding here is request/display state only.
/// - no user/password/ledger/wallet mutation happens here.
#[derive(Debug)]
pub struct OperatorState {
    admin_ui_enabled: AtomicBool,
    setup_base_url: String,
    setup_token_ttl: Duration,
    setup_token: Mutex<Option<SetupTokenRecord>>,
    reward_recipient: Mutex<RewardRecipientRecord>,
}

impl OperatorState {
    pub fn new(admin_ui_enabled: bool, setup_base_url: String, setup_token_ttl: Duration) -> Self {
        Self {
            admin_ui_enabled: AtomicBool::new(admin_ui_enabled),
            setup_base_url,
            setup_token_ttl,
            setup_token: Mutex::new(None),
            reward_recipient: Mutex::new(RewardRecipientRecord::default()),
        }
    }

    pub fn admin_ui_enabled(&self) -> bool {
        self.admin_ui_enabled.load(Ordering::Acquire)
    }

    pub fn set_admin_ui_enabled(&self, enabled: bool) {
        self.admin_ui_enabled.store(enabled, Ordering::Release);
    }

    pub fn issue_setup_token(&self) -> io::Result<SetupTokenSnapshot> {
        let token = random_token_hex(32)?;
        let now = now_unix_s();
        let ttl = self.setup_token_ttl.as_secs();
        let expires_at = now.saturating_add(ttl);

        let record = SetupTokenRecord {
            token: token.clone(),
            expires_at_unix_s: expires_at,
        };

        {
            let mut slot = self
                .setup_token
                .lock()
                .expect("operator setup token mutex poisoned");
            *slot = Some(record);
        }

        let setup_url = format!(
            "{}/setup?token={}",
            self.setup_base_url.trim_end_matches('/'),
            token
        );

        Ok(SetupTokenSnapshot {
            token,
            setup_url,
            expires_at_unix_s: expires_at,
            expires_in_seconds: ttl,
        })
    }

    pub fn consume_setup_token(&self, token: &str) -> bool {
        let token = token.trim();
        if token.is_empty() {
            return false;
        }

        let mut slot = self
            .setup_token
            .lock()
            .expect("operator setup token mutex poisoned");

        let Some(record) = slot.as_ref() else {
            return false;
        };

        let now = now_unix_s();
        if record.expires_at_unix_s <= now {
            *slot = None;
            return false;
        }

        if record.token != token {
            return false;
        }

        *slot = None;
        true
    }

    pub fn setup_token_active(&self) -> bool {
        let mut slot = self
            .setup_token
            .lock()
            .expect("operator setup token mutex poisoned");

        let Some(record) = slot.as_ref() else {
            return false;
        };

        if record.expires_at_unix_s <= now_unix_s() {
            *slot = None;
            return false;
        }

        true
    }

    pub fn reward_recipient_snapshot(&self) -> RewardRecipientSnapshot {
        let record = self
            .reward_recipient
            .lock()
            .expect("operator reward recipient mutex poisoned")
            .clone();

        reward_snapshot_from_record(record)
    }

    pub fn bind_reward_recipient(&self, display_address: String) -> RewardRecipientSnapshot {
        let mut record = self
            .reward_recipient
            .lock()
            .expect("operator reward recipient mutex poisoned");

        record.reward_recipient_display_address = Some(display_address);
        record.pending_rotation_display_address = None;
        record.updated_at_unix_s = Some(now_unix_s());

        reward_snapshot_from_record(record.clone())
    }

    pub fn request_reward_recipient_rotation(
        &self,
        display_address: String,
    ) -> RewardRecipientSnapshot {
        let mut record = self
            .reward_recipient
            .lock()
            .expect("operator reward recipient mutex poisoned");

        record.pending_rotation_display_address = Some(display_address);
        record.updated_at_unix_s = Some(now_unix_s());

        reward_snapshot_from_record(record.clone())
    }
}

fn reward_snapshot_from_record(record: RewardRecipientRecord) -> RewardRecipientSnapshot {
    let state = if record.pending_rotation_display_address.is_some() {
        "pending_rotation"
    } else if record.reward_recipient_display_address.is_some() {
        "bound"
    } else {
        "unbound"
    };

    RewardRecipientSnapshot {
        state,
        reward_recipient_display_address: record.reward_recipient_display_address,
        pending_rotation_display_address: record.pending_rotation_display_address,
        updated_at_unix_s: record.updated_at_unix_s,
    }
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

fn random_token_hex(len_bytes: usize) -> io::Result<String> {
    let mut bytes = vec![0_u8; len_bytes];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ron_policy::ModerationReasonCode;

    const OBJECT: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn runtime_moderation_policy_snapshot_tracks_effective_policy() {
        let runtime = RuntimeStatus::new();
        let object = OBJECT
            .parse::<B3Id>()
            .expect("test object must be canonical");

        let initializing = runtime
            .persistence_moderation_policy()
            .expect_err("uninitialized runtime must fail closed");

        assert_eq!(initializing.state, "not_configured");

        runtime.set_moderation_not_configured();

        let empty = runtime
            .persistence_moderation_policy()
            .expect("explicit no-config posture uses the storage router's empty policy");

        assert_eq!(empty.evaluate(&object).reason, ModerationReasonCode::NoRule);

        let mut configured = ModerationPolicy::default();
        assert!(configured.insert_global_deny(object.clone()));

        let counts = ModerationPolicyCounts::from_policy(&configured);
        let configured = Arc::new(configured);

        runtime.set_moderation_active(configured.clone(), counts);

        let active = runtime
            .persistence_moderation_policy()
            .expect("active policy must be available");

        assert!(Arc::ptr_eq(&active, &configured));
        assert_eq!(
            active.evaluate(&object).reason,
            ModerationReasonCode::GlobalDeny
        );

        runtime.set_moderation_inactive();

        let inactive = runtime
            .persistence_moderation_policy()
            .expect_err("inactive storage must not expose policy for approval");

        assert_eq!(inactive.state, "inactive");

        runtime.set_signed_moderation_active("signed_global", configured, counts, 7, u64::MAX);

        assert!(
            runtime.persistence_moderation_policy().is_ok(),
            "verified unexpired signed policy must be available"
        );

        runtime.set_moderation_expired();

        let expired = runtime
            .persistence_moderation_policy()
            .expect_err("expired signed policy must fail closed");

        assert_eq!(expired.state, "expired");
    }
}

#[cfg(test)]
mod persistence_review_tests {
    use std::sync::Arc;

    use ron_policy::{B3Id, ModerationPolicy, PersistencePolicy, PersistenceReviewLevel};
    use ron_proto::asset::AssetKind;
    use svc_storage::persistence::PersistenceState;

    use super::{ModerationPolicyCounts, RuntimeStatus};

    const GLOBAL_DENY_OBJECT: &str =
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TOMBSTONED_OBJECT: &str =
        "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const BLOCKED_OBJECT: &str =
        "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const QUARANTINED_OBJECT: &str =
        "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const UNAVAILABLE_OBJECT: &str =
        "b3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn eligible_policy() -> PersistencePolicy {
        let mut policy = PersistencePolicy::default();

        assert!(
            policy.allow_asset_kind(AssetKind::Image),
            "image category should be newly allowed"
        );

        policy.set_operator_pinning_allowed(true);
        policy
    }

    fn assert_refusal_projection(
        object: B3Id,
        moderation: ModerationPolicy,
        expected_state: PersistenceState,
    ) {
        let runtime = RuntimeStatus::new();
        let counts = ModerationPolicyCounts::from_policy(&moderation);

        runtime.set_moderation_active(Arc::new(moderation), counts);

        assert!(runtime
            .persistence_catalog()
            .register(object.clone(), AssetKind::Image));

        let active = runtime
            .persistence_moderation_policy()
            .expect("configured moderation must be available");

        let policy = eligible_policy();

        let mutation = runtime
            .persistence_catalog()
            .approve(
                &object,
                &policy,
                active.as_ref(),
                PersistenceReviewLevel::ModerationApproved,
            )
            .expect("moderation refusal should project into a persistence state");

        assert_eq!(mutation.candidate().state(), expected_state);
        assert!(!mutation.candidate().is_durable_storage_eligible());

        // A later pin attempt must not override the refusal.
        let _ = runtime.persistence_catalog().pin(
            &object,
            &policy,
            active.as_ref(),
            PersistenceReviewLevel::ModerationApproved,
        );

        let final_candidate = runtime
            .persistence_catalog()
            .status(&object)
            .expect("candidate must remain observable");

        assert_eq!(final_candidate.state(), expected_state);
        assert!(!final_candidate.is_durable_storage_eligible());
    }

    fn assert_policy_unavailable(
        runtime: &RuntimeStatus,
        object: &B3Id,
        expected_runtime_state: &str,
    ) {
        let unavailable = runtime
            .persistence_moderation_policy()
            .expect_err("unsafe moderation posture must fail closed");

        assert_eq!(unavailable.state, expected_runtime_state);

        let candidate = runtime
            .persistence_catalog()
            .status(object)
            .expect("candidate must remain registered");

        assert_eq!(candidate.state(), PersistenceState::EphemeralUnvetted);
        assert!(!candidate.is_durable_storage_eligible());
    }

    #[test]
    fn runtime_persistence_review_projects_every_canonical_refusal() {
        let globally_denied = GLOBAL_DENY_OBJECT
            .parse::<B3Id>()
            .expect("global-deny object must be canonical");

        let mut global_policy = ModerationPolicy::default();
        assert!(global_policy.insert_global_deny(globally_denied.clone()));

        assert_refusal_projection(
            globally_denied,
            global_policy,
            PersistenceState::GlobalDenied,
        );

        let tombstoned = TOMBSTONED_OBJECT
            .parse::<B3Id>()
            .expect("tombstoned object must be canonical");

        let mut tombstone_policy = ModerationPolicy::default();
        assert!(tombstone_policy.insert_owner_tombstone(tombstoned.clone()));

        assert_refusal_projection(
            tombstoned,
            tombstone_policy,
            PersistenceState::OwnerTombstoned,
        );

        let locally_blocked = BLOCKED_OBJECT
            .parse::<B3Id>()
            .expect("blocked object must be canonical");

        let mut block_policy = ModerationPolicy::default();
        assert!(block_policy.insert_local_block(locally_blocked.clone()));

        assert_refusal_projection(
            locally_blocked,
            block_policy,
            PersistenceState::OperatorBlocked,
        );

        let quarantined = QUARANTINED_OBJECT
            .parse::<B3Id>()
            .expect("quarantined object must be canonical");

        let mut quarantine_policy = ModerationPolicy::default();
        assert!(quarantine_policy.insert_quarantine(quarantined.clone()));

        assert_refusal_projection(
            quarantined,
            quarantine_policy,
            PersistenceState::Quarantined,
        );
    }

    #[test]
    fn runtime_persistence_review_fails_closed_without_effective_moderation() {
        let runtime = RuntimeStatus::new();
        let object = UNAVAILABLE_OBJECT
            .parse::<B3Id>()
            .expect("test object must be canonical");

        assert!(runtime
            .persistence_catalog()
            .register(object.clone(), AssetKind::Image));

        assert_policy_unavailable(&runtime, &object, "not_configured");

        runtime.set_moderation_load_failed("unsigned_local");

        assert_policy_unavailable(&runtime, &object, "load_failed");

        runtime.set_signed_moderation_active(
            "signed_global",
            Arc::new(ModerationPolicy::default()),
            ModerationPolicyCounts::default(),
            1,
            u64::MAX,
        );
        runtime.set_moderation_expired();

        assert_policy_unavailable(&runtime, &object, "expired");

        runtime.set_moderation_active(
            Arc::new(ModerationPolicy::default()),
            ModerationPolicyCounts::default(),
        );
        runtime.set_moderation_inactive();

        assert_policy_unavailable(&runtime, &object, "inactive");
    }
}
