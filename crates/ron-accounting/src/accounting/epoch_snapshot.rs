//! RO:WHAT — Deterministic epoch snapshot artifact for classified node evidence.
//! RO:WHY — Phase 14 needs a stable accounting commitment before reward planning.
//! RO:INTERACTS — service/user classification batches and later svc-rewarder plans.
//! RO:INVARIANTS — explicit epoch window, canonical order, BLAKE3 artifact CID.
//! RO:SECURITY — no consensus, payout, balance, receipt, wallet, or ledger authority.
//! RO:TEST — tests/epoch_accounting_snapshot.rs.

#![forbid(unsafe_code)]

use super::node_evidence_wire::{
    EvidenceContentId, ServiceEvidenceAccountingKindV1, UserVerificationEvidenceKindV1,
};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{Error, Result},
    utils::{encode::to_canonical_bytes, hashing::b3_hex},
};

use super::{
    ServiceEvidenceAccountingClassV1, ServiceEvidenceClassificationBatchV1,
    UserVerificationAccountingClassV1, UserVerificationClassificationBatchV1,
    SERVICE_EVIDENCE_CLASSIFICATION_BATCH_SCHEMA, SERVICE_EVIDENCE_CLASSIFICATION_BATCH_VERSION,
    USER_VERIFICATION_CLASSIFICATION_BATCH_SCHEMA, USER_VERIFICATION_CLASSIFICATION_BATCH_VERSION,
};

pub const ACCOUNTING_EPOCH_SNAPSHOT_SCHEMA: &str = "ron.accounting.epoch-snapshot-artifact.v1";

pub const ACCOUNTING_EPOCH_SNAPSHOT_VERSION: u16 = 1;

/// Schema for the exact validated economics model bound to a snapshot.
pub const ACCOUNTING_ECONOMICS_CONFIG_BINDING_SCHEMA: &str =
    "ron.accounting.economics-config-binding.v1";

const MAX_EPOCH_ID_BYTES: usize = 160;
const MAX_CONFIG_SCHEMA_BYTES: usize = 160;

/// Explicit economics profile selected before snapshot construction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountingEconomicsProfileV1 {
    /// Reviewed non-development economics document.
    Canonical,

    /// Explicit development/private-beta economics document.
    Development,
}

/// Identity of the validated normalized economics model used for an epoch.
///
/// `ron-accounting` does not parse rates, merge canonical/development files,
/// select recipients, or authorize payouts. A later `ron-policy` boundary must
/// supply this identity after validating and canonically hashing one complete
/// profile document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccountingEconomicsConfigBindingV1 {
    pub schema: String,
    pub config_schema: String,
    pub config_version: u16,
    pub profile: AccountingEconomicsProfileV1,
    pub economics_config_hash: EvidenceContentId,
}

impl AccountingEconomicsConfigBindingV1 {
    /// Build a binding from one policy-validated normalized model.
    ///
    /// # Errors
    ///
    /// Returns `Error::SchemaViolation` when schema/version/hash identity is
    /// malformed.
    pub fn from_validated_model(
        config_schema: impl Into<String>,
        config_version: u16,
        profile: AccountingEconomicsProfileV1,
        economics_config_hash: &str,
    ) -> Result<Self> {
        let binding = Self {
            schema: ACCOUNTING_ECONOMICS_CONFIG_BINDING_SCHEMA.to_owned(),
            config_schema: config_schema.into(),
            config_version,
            profile,
            economics_config_hash: economics_config_hash.parse()?,
        };

        binding.validate()?;
        Ok(binding)
    }

    /// Validate strict schema/version/hash identity.
    ///
    /// # Errors
    ///
    /// Returns `Error::SchemaViolation` when the binding is malformed.
    pub fn validate(&self) -> Result<()> {
        if self.schema != ACCOUNTING_ECONOMICS_CONFIG_BINDING_SCHEMA {
            return Err(Error::schema(
                "invalid accounting economics config binding schema",
            ));
        }

        validate_config_schema(&self.config_schema)?;

        if self.config_version == 0 {
            return Err(Error::schema(
                "accounting economics config_version must be nonzero",
            ));
        }

        self.economics_config_hash.validate()
    }
}

/// Explicit half-open epoch window: `[starts_at_ms, ends_at_ms)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccountingEpochWindowV1 {
    pub epoch_id: String,
    pub starts_at_ms: u64,
    pub ends_at_ms: u64,

    /// Caller-supplied artifact production time.
    pub produced_at_ms: u64,
}

impl AccountingEpochWindowV1 {
    /// Validate the epoch identity and time ordering.
    pub fn validate(&self) -> Result<()> {
        validate_epoch_id(&self.epoch_id)?;

        if self.starts_at_ms == 0 {
            return Err(Error::schema(
                "accounting epoch starts_at_ms must be greater than zero",
            ));
        }

        if self.ends_at_ms <= self.starts_at_ms {
            return Err(Error::schema(
                "accounting epoch ends_at_ms must be greater than starts_at_ms",
            ));
        }

        if self.produced_at_ms < self.ends_at_ms {
            return Err(Error::schema(
                "accounting snapshot produced_at_ms must be at or after epoch end",
            ));
        }

        Ok(())
    }

    #[must_use]
    pub const fn contains(&self, observed_at_ms: u64) -> bool {
        self.starts_at_ms <= observed_at_ms && observed_at_ms < self.ends_at_ms
    }
}

/// One proof-eligible Service Node accounting row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceAccountingSnapshotRowV1 {
    pub sequence: u64,
    pub kind: ServiceEvidenceAccountingKindV1,
    pub proof_id: String,
    pub service_node_id: String,
    pub content_id: EvidenceContentId,
    pub observed_at_ms: u64,
}

/// One proof-eligible User Node verification accounting row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserVerificationAccountingSnapshotRowV1 {
    pub sequence: u64,
    pub verification_kind: UserVerificationEvidenceKindV1,

    pub evidence_id: String,
    pub user_node_id: String,
    pub subject_ref: String,

    pub input_digest: EvidenceContentId,
    pub observed_at_ms: u64,
}

/// Canonical accounting artifact for one epoch.
///
/// The artifact CID can later be referenced by quorum/epoch-transition
/// machinery, but this object does not itself claim consensus acceptance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccountingEpochSnapshotV1 {
    pub schema: String,
    pub version: u16,

    pub epoch_id: String,
    pub starts_at_ms: u64,
    pub ends_at_ms: u64,
    pub produced_at_ms: u64,

    /// Exact validated normalized economics model bound into canonical bytes.
    pub economics_config: AccountingEconomicsConfigBindingV1,

    pub service_rows: Vec<ServiceAccountingSnapshotRowV1>,

    pub user_verification_rows: Vec<UserVerificationAccountingSnapshotRowV1>,

    pub source_service_input_count: usize,
    pub source_user_verification_input_count: usize,

    pub eligible_service_count: usize,
    pub eligible_user_verification_count: usize,

    /// Retained for audit/policy review, not reward planning.
    pub policy_evidence_count: usize,

    /// Retained for challenge adjudication, not reward planning.
    pub challenge_evidence_count: usize,

    pub deterministic_order: bool,
    pub snapshot_artifact_created: bool,

    /// Remains false until later quorum/epoch machinery accepts a commitment.
    pub consensus_root_claimed: bool,

    pub reward_plan_created: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

impl AccountingEpochSnapshotV1 {
    /// Return a canonicalized and validated copy.
    pub fn canonicalized(&self) -> Result<Self> {
        let mut canonical = self.clone();

        canonical.service_rows.sort_by(|left, right| {
            (
                left.service_node_id.as_str(),
                left.kind,
                left.proof_id.as_str(),
                left.sequence,
            )
                .cmp(&(
                    right.service_node_id.as_str(),
                    right.kind,
                    right.proof_id.as_str(),
                    right.sequence,
                ))
        });

        canonical.user_verification_rows.sort_by(|left, right| {
            (
                left.user_node_id.as_str(),
                left.verification_kind,
                left.evidence_id.as_str(),
                left.sequence,
            )
                .cmp(&(
                    right.user_node_id.as_str(),
                    right.verification_kind,
                    right.evidence_id.as_str(),
                    right.sequence,
                ))
        });

        canonical.validate()?;
        Ok(canonical)
    }

    /// Validate counts, ordering, epoch membership, and authority posture.
    pub fn validate(&self) -> Result<()> {
        if self.schema != ACCOUNTING_EPOCH_SNAPSHOT_SCHEMA {
            return Err(Error::schema("invalid accounting epoch snapshot schema"));
        }

        if self.version != ACCOUNTING_EPOCH_SNAPSHOT_VERSION {
            return Err(Error::schema("invalid accounting epoch snapshot version"));
        }

        let window = AccountingEpochWindowV1 {
            epoch_id: self.epoch_id.clone(),
            starts_at_ms: self.starts_at_ms,
            ends_at_ms: self.ends_at_ms,
            produced_at_ms: self.produced_at_ms,
        };

        window.validate()?;
        self.economics_config.validate()?;

        if self.source_service_input_count + self.source_user_verification_input_count == 0 {
            return Err(Error::schema(
                "accounting epoch snapshot requires at least one classified input",
            ));
        }

        if self.eligible_service_count != self.service_rows.len() {
            return Err(Error::schema(
                "eligible service count does not match service rows",
            ));
        }

        if self.eligible_user_verification_count != self.user_verification_rows.len() {
            return Err(Error::schema(
                "eligible user verification count does not match user rows",
            ));
        }

        if self.source_service_input_count
            != self
                .eligible_service_count
                .checked_add(self.policy_evidence_count)
                .ok_or_else(|| Error::schema("service snapshot count overflow"))?
        {
            return Err(Error::schema(
                "service source count does not match eligible plus policy evidence",
            ));
        }

        if self.source_user_verification_input_count
            != self
                .eligible_user_verification_count
                .checked_add(self.challenge_evidence_count)
                .ok_or_else(|| Error::schema("user verification snapshot count overflow"))?
        {
            return Err(Error::schema(
                "user source count does not match eligible plus challenge evidence",
            ));
        }

        for row in &self.service_rows {
            if !window.contains(row.observed_at_ms) {
                return Err(Error::schema(format!(
                    "service evidence outside epoch window: {}",
                    row.proof_id
                )));
            }
        }

        for row in &self.user_verification_rows {
            if !window.contains(row.observed_at_ms) {
                return Err(Error::schema(format!(
                    "user verification evidence outside epoch window: {}",
                    row.evidence_id
                )));
            }
        }

        if self.service_rows.windows(2).any(|pair| {
            (
                pair[0].service_node_id.as_str(),
                pair[0].kind,
                pair[0].proof_id.as_str(),
                pair[0].sequence,
            ) >= (
                pair[1].service_node_id.as_str(),
                pair[1].kind,
                pair[1].proof_id.as_str(),
                pair[1].sequence,
            )
        }) {
            return Err(Error::schema(
                "service snapshot rows must be strictly canonical",
            ));
        }

        if self.user_verification_rows.windows(2).any(|pair| {
            (
                pair[0].user_node_id.as_str(),
                pair[0].verification_kind,
                pair[0].evidence_id.as_str(),
                pair[0].sequence,
            ) >= (
                pair[1].user_node_id.as_str(),
                pair[1].verification_kind,
                pair[1].evidence_id.as_str(),
                pair[1].sequence,
            )
        }) {
            return Err(Error::schema(
                "user verification snapshot rows must be strictly canonical",
            ));
        }

        if !self.deterministic_order {
            return Err(Error::schema(
                "accounting epoch snapshot must declare deterministic order",
            ));
        }

        if !self.snapshot_artifact_created {
            return Err(Error::schema(
                "accounting epoch snapshot must be an artifact",
            ));
        }

        for (field, value) in [
            ("consensus_root_claimed", self.consensus_root_claimed),
            ("reward_plan_created", self.reward_plan_created),
            ("payout_authority", self.payout_authority),
            ("wallet_mutation", self.wallet_mutation),
            ("ledger_mutation", self.ledger_mutation),
        ] {
            if value {
                return Err(Error::schema(format!(
                    "accounting epoch snapshot authority boundary violated: {field}"
                )));
            }
        }

        Ok(())
    }

    /// Deterministic compact bytes for this canonical artifact.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        to_canonical_bytes(&self.canonicalized()?)
    }

    /// BLAKE3 artifact CID over the canonical snapshot bytes.
    ///
    /// This is a deterministic commitment, not quorum or ledger truth.
    pub fn canonical_artifact_cid(&self) -> Result<String> {
        Ok(b3_hex(&self.canonical_bytes()?))
    }
}

/// Build a deterministic epoch snapshot from one or both classification paths.
///
/// Policy-refusal/moderation and invalid/challenge findings are counted but
/// excluded from reward-planning rows.
///
/// # Errors
///
/// Rejects invalid classification batches, out-of-window decisions, fabricated
/// authority flags, or an epoch with no classified input.
pub fn build_accounting_epoch_snapshot(
    window: AccountingEpochWindowV1,
    economics_config: &AccountingEconomicsConfigBindingV1,
    service_batch: Option<&ServiceEvidenceClassificationBatchV1>,
    user_batch: Option<&UserVerificationClassificationBatchV1>,
) -> Result<AccountingEpochSnapshotV1> {
    window.validate()?;
    economics_config.validate()?;

    if service_batch.is_none() && user_batch.is_none() {
        return Err(Error::schema(
            "accounting epoch snapshot requires a classification batch",
        ));
    }

    let mut service_rows = Vec::new();
    let mut user_verification_rows = Vec::new();

    let mut source_service_input_count = 0;
    let mut source_user_verification_input_count = 0;

    let mut policy_evidence_count = 0;
    let mut challenge_evidence_count = 0;

    if let Some(batch) = service_batch {
        validate_service_batch(batch)?;

        source_service_input_count = batch.input_count;
        policy_evidence_count = batch.policy_evidence_count;

        for decision in &batch.decisions {
            if !window.contains(decision.observed_at_ms) {
                return Err(Error::schema(format!(
                    "service evidence outside epoch window: {}",
                    decision.proof_id
                )));
            }

            if decision.class == ServiceEvidenceAccountingClassV1::ProofEligibleService {
                service_rows.push(ServiceAccountingSnapshotRowV1 {
                    sequence: decision.sequence,
                    kind: decision.kind,
                    proof_id: decision.proof_id.clone(),
                    service_node_id: decision.service_node_id.clone(),
                    content_id: decision.content_id.clone(),
                    observed_at_ms: decision.observed_at_ms,
                });
            }
        }
    }

    if let Some(batch) = user_batch {
        validate_user_batch(batch)?;

        source_user_verification_input_count = batch.input_count;

        challenge_evidence_count = batch.challenge_evidence_count;

        for decision in &batch.decisions {
            if !window.contains(decision.observed_at_ms) {
                return Err(Error::schema(format!(
                    "user verification evidence outside epoch window: {}",
                    decision.evidence_id
                )));
            }

            if decision.class == UserVerificationAccountingClassV1::ProofEligibleVerification {
                user_verification_rows.push(UserVerificationAccountingSnapshotRowV1 {
                    sequence: decision.sequence,
                    verification_kind: decision.verification_kind,
                    evidence_id: decision.evidence_id.clone(),
                    user_node_id: decision.user_node_id.clone(),
                    subject_ref: decision.subject_ref.clone(),
                    input_digest: decision.input_digest.clone(),
                    observed_at_ms: decision.observed_at_ms,
                });
            }
        }
    }

    AccountingEpochSnapshotV1 {
        schema: ACCOUNTING_EPOCH_SNAPSHOT_SCHEMA.to_owned(),
        version: ACCOUNTING_EPOCH_SNAPSHOT_VERSION,
        epoch_id: window.epoch_id,
        starts_at_ms: window.starts_at_ms,
        ends_at_ms: window.ends_at_ms,
        produced_at_ms: window.produced_at_ms,
        economics_config: economics_config.clone(),
        eligible_service_count: service_rows.len(),
        eligible_user_verification_count: user_verification_rows.len(),
        service_rows,
        user_verification_rows,
        source_service_input_count,
        source_user_verification_input_count,
        policy_evidence_count,
        challenge_evidence_count,
        deterministic_order: true,
        snapshot_artifact_created: true,
        consensus_root_claimed: false,
        reward_plan_created: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
    .canonicalized()
}

/// Canonical bytes for an accounting epoch snapshot artifact.
pub fn canonical_accounting_epoch_snapshot_bytes(
    snapshot: &AccountingEpochSnapshotV1,
) -> Result<Vec<u8>> {
    snapshot.canonical_bytes()
}

/// Canonical BLAKE3 CID for an accounting epoch snapshot artifact.
pub fn canonical_accounting_epoch_snapshot_artifact_cid(
    snapshot: &AccountingEpochSnapshotV1,
) -> Result<String> {
    snapshot.canonical_artifact_cid()
}

fn validate_service_batch(batch: &ServiceEvidenceClassificationBatchV1) -> Result<()> {
    if batch.schema != SERVICE_EVIDENCE_CLASSIFICATION_BATCH_SCHEMA
        || batch.version != SERVICE_EVIDENCE_CLASSIFICATION_BATCH_VERSION
    {
        return Err(Error::schema(
            "invalid service evidence classification batch",
        ));
    }

    if !batch.deterministic_order {
        return Err(Error::schema(
            "service classification batch is not deterministic",
        ));
    }

    if batch.input_count != batch.decisions.len()
        || batch.proof_eligible_count + batch.policy_evidence_count != batch.input_count
    {
        return Err(Error::schema("service classification batch count mismatch"));
    }

    if batch.accounting_snapshot_created
        || batch.reward_plan_created
        || batch.payout_authority
        || batch.wallet_mutation
        || batch.ledger_mutation
    {
        return Err(Error::schema(
            "service classification batch carries forbidden authority",
        ));
    }

    for decision in &batch.decisions {
        if decision.observed_at_ms == 0
            || !decision.verified_evidence
            || !decision.requires_policy_review
            || decision.accounting_snapshot_member
            || decision.reward_plan_created
            || decision.direct_protocol_roc_allocation
            || decision.reward_truth
            || decision.payout_authority
            || decision.wallet_mutation
            || decision.ledger_mutation
        {
            return Err(Error::schema(format!(
                "invalid service classification decision: {}",
                decision.proof_id
            )));
        }

        match decision.class {
            ServiceEvidenceAccountingClassV1::ProofEligibleService => {
                if !decision.reward_planning_candidate
                    || !decision.requires_economics_config
                    || !decision.requires_cap
                {
                    return Err(Error::schema(
                        "proof-eligible service decision lost planning gates",
                    ));
                }
            }

            ServiceEvidenceAccountingClassV1::PolicyEvidenceOnly => {
                if decision.reward_planning_candidate
                    || decision.requires_economics_config
                    || decision.requires_cap
                {
                    return Err(Error::schema(
                        "policy evidence cannot become reward-planning material",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_user_batch(batch: &UserVerificationClassificationBatchV1) -> Result<()> {
    if batch.schema != USER_VERIFICATION_CLASSIFICATION_BATCH_SCHEMA
        || batch.version != USER_VERIFICATION_CLASSIFICATION_BATCH_VERSION
    {
        return Err(Error::schema(
            "invalid user verification classification batch",
        ));
    }

    if !batch.deterministic_order {
        return Err(Error::schema(
            "user verification classification batch is not deterministic",
        ));
    }

    if batch.input_count != batch.decisions.len()
        || batch.proof_eligible_count + batch.challenge_evidence_count != batch.input_count
    {
        return Err(Error::schema(
            "user verification classification batch count mismatch",
        ));
    }

    if batch.accounting_snapshot_created
        || batch.reward_plan_created
        || batch.payout_authority
        || batch.wallet_mutation
        || batch.ledger_mutation
    {
        return Err(Error::schema(
            "user classification batch carries forbidden authority",
        ));
    }

    for decision in &batch.decisions {
        if decision.observed_at_ms == 0
            || !decision.verified_evidence
            || !decision.requires_policy_review
            || decision.accounting_snapshot_member
            || decision.reward_plan_created
            || decision.direct_protocol_roc_allocation
            || decision.reward_truth
            || decision.payout_authority
            || decision.wallet_mutation
            || decision.ledger_mutation
        {
            return Err(Error::schema(format!(
                "invalid user verification classification decision: {}",
                decision.evidence_id
            )));
        }

        match decision.class {
            UserVerificationAccountingClassV1::ProofEligibleVerification => {
                if !decision.reward_planning_candidate
                    || !decision.requires_economics_config
                    || !decision.requires_cap
                    || decision.requires_challenge_acceptance
                {
                    return Err(Error::schema(
                        "proof-eligible user decision lost planning gates",
                    ));
                }
            }

            UserVerificationAccountingClassV1::ChallengeEvidenceOnly => {
                if decision.reward_planning_candidate
                    || decision.requires_economics_config
                    || decision.requires_cap
                    || !decision.requires_challenge_acceptance
                {
                    return Err(Error::schema(
                        "challenge evidence cannot become reward-planning material",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_config_schema(value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_CONFIG_SCHEMA_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.' | b'/')
        });

    if !valid {
        return Err(Error::schema(
            "config_schema must be a bounded lowercase identifier",
        ));
    }

    Ok(())
}

fn validate_epoch_id(value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_EPOCH_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.' | b'/')
        });

    if !valid {
        return Err(Error::schema(
            "epoch_id must be a bounded lowercase identifier",
        ));
    }

    Ok(())
}
