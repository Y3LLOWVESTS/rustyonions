//! RO:WHAT — Canonical bad-node violation, containment-status, and appeal DTOs.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 19 requires shared protocol evidence for
//! containing malicious or unreliable Service Nodes without creating another
//! lifecycle state machine.
//!
//! RO:INTERACTS — Phase 18 eligibility descriptors, `svc-registry` custody,
//! `ron-policy` containment decisions, reward review, user-node challenges,
//! `macronode`, `micronode`, and later operator status projection.
//!
//! RO:INVARIANTS — reports are evidence only; containment reuses `Degraded`,
//! `Quarantined`, and `Blocked`; appeals do not silently restore eligibility;
//! no raw IP address, payout amount, wallet, ledger, mint, or finality fields.
//!
//! RO:SECURITY — validation grants no quarantine, block, reward, quorum,
//! balance, receipt, mint, burn, settlement, or finality authority.
//!
//! RO:TEST — `tests/internal_roc_beta_phase19_bad_node_enforcement.rs`.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::ServiceNodeEligibilityStateV1;
use crate::id::ContentId;

/// Current Phase 19 bad-node enforcement DTO version.
pub const SERVICE_NODE_ENFORCEMENT_VERSION: u16 = 1;

/// Canonical schema for one observed bad-node violation report.
pub const SERVICE_NODE_VIOLATION_REPORT_SCHEMA: &str = "ron.service_node.violation-report.v1";

/// Canonical schema for one registry-visible containment status.
pub const SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA: &str = "ron.service_node.enforcement-status.v1";

const MAX_REF_BYTES: usize = 256;

/// Structural validation failures for Phase 19 enforcement artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeEnforcementValidationError {
    /// A DTO used an unsupported schema.
    #[error("invalid Service Node enforcement schema: expected {expected}, got {actual}")]
    InvalidSchema {
        /// Required schema.
        expected: &'static str,

        /// Supplied schema.
        actual: String,
    },

    /// A DTO used an unsupported version.
    #[error("invalid Service Node enforcement version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// Required version.
        expected: u16,

        /// Supplied version.
        actual: u16,
    },

    /// A field failed bounded or semantic validation.
    #[error("invalid Service Node enforcement field {field}: {reason}")]
    InvalidField {
        /// Invalid field.
        field: &'static str,

        /// Stable failure reason.
        reason: &'static str,
    },
}

/// Canonical Phase 19 bad-node signal taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServiceNodeViolationKindV1 {
    /// Provider returned bytes that do not match the requested BLAKE3 identity.
    HashMismatch,

    /// Node served or advertised content forbidden by a denylist.
    DenylistViolation,

    /// Node ignored a valid owner tombstone.
    TombstoneViolation,

    /// Node submitted materially false delivery evidence.
    FakeDeliveryProof,

    /// Node produced abusive or misleading provider advertisements.
    ProviderSpam,

    /// Node repeatedly submitted replayed protocol material.
    ReplayAttempt,

    /// Node constructed self-traffic or collapsed-actor reward loops.
    SelfTrafficLoop,

    /// Node failed a required objective challenge.
    ChallengeFailure,

    /// Node proposed an invalid epoch transition.
    InvalidEpochProposal,

    /// Node supplied an invalid epoch-transition signature.
    InvalidEpochSignature,

    /// Node attempted unilateral ROC issuance or mint authority.
    UnilateralMintAttempt,

    /// Node published or exposed forbidden user network identity.
    PrivacyLeak,

    /// Node attempted to bypass or abuse reward-recipient binding.
    RewardRecipientBindingAbuse,
}

impl ServiceNodeViolationKindV1 {
    /// Whether this signal represents a user-network privacy violation.
    #[must_use]
    pub const fn is_privacy_violation(self) -> bool {
        matches!(self, Self::PrivacyLeak)
    }

    /// Whether this signal concerns epoch or issuance authority.
    #[must_use]
    pub const fn is_epoch_or_issuance_violation(self) -> bool {
        matches!(
            self,
            Self::InvalidEpochProposal | Self::InvalidEpochSignature | Self::UnilateralMintAttempt
        )
    }

    /// Whether this signal concerns reward evidence or recipient binding.
    #[must_use]
    pub const fn is_reward_path_violation(self) -> bool {
        matches!(
            self,
            Self::FakeDeliveryProof | Self::SelfTrafficLoop | Self::RewardRecipientBindingAbuse
        )
    }
}

/// Strict lifecycle for one operator-visible appeal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServiceNodeAppealStateV1 {
    /// No appeal has been submitted.
    NotAppealed,

    /// An appeal is awaiting objective review.
    Pending,

    /// Review accepted the appeal.
    Accepted,

    /// Review rejected the appeal.
    Rejected,
}

/// Registry-visible appeal posture for one containment record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeAppealStatusV1 {
    /// Current appeal lifecycle state.
    pub state: ServiceNodeAppealStateV1,

    /// Stable appeal identifier when an appeal exists.
    #[serde(default)]
    pub appeal_id: Option<String>,

    /// Epoch in which the appeal was submitted.
    #[serde(default)]
    pub submitted_epoch: Option<u64>,

    /// Epoch in which the appeal was resolved.
    #[serde(default)]
    pub resolved_epoch: Option<u64>,

    /// Objective review evidence for an accepted or rejected appeal.
    #[serde(default)]
    pub resolution_evidence_root: Option<ContentId>,
}

impl ServiceNodeAppealStatusV1 {
    /// Construct the canonical no-appeal posture.
    #[must_use]
    pub const fn not_appealed() -> Self {
        Self {
            state: ServiceNodeAppealStateV1::NotAppealed,
            appeal_id: None,
            submitted_epoch: None,
            resolved_epoch: None,
            resolution_evidence_root: None,
        }
    }

    /// Validate strict appeal-state field relationships.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeEnforcementValidationError`] when fields contradict
    /// the declared appeal state or epoch ordering.
    pub fn validate(&self) -> Result<(), ServiceNodeEnforcementValidationError> {
        match self.state {
            ServiceNodeAppealStateV1::NotAppealed => {
                if self.appeal_id.is_some()
                    || self.submitted_epoch.is_some()
                    || self.resolved_epoch.is_some()
                    || self.resolution_evidence_root.is_some()
                {
                    return invalid("appeal", "not_appealed must not carry appeal fields");
                }
            }
            ServiceNodeAppealStateV1::Pending => {
                validate_appeal_identity(self.appeal_id.as_deref(), self.submitted_epoch)?;

                if self.resolved_epoch.is_some() || self.resolution_evidence_root.is_some() {
                    return invalid("appeal", "pending appeal must not carry resolution fields");
                }
            }
            ServiceNodeAppealStateV1::Accepted | ServiceNodeAppealStateV1::Rejected => {
                let submitted_epoch =
                    validate_appeal_identity(self.appeal_id.as_deref(), self.submitted_epoch)?;

                let resolved_epoch = self.resolved_epoch.ok_or_else(|| {
                    invalid_error("appeal.resolved_epoch", "required for resolved appeal")
                })?;

                if resolved_epoch <= submitted_epoch {
                    return invalid(
                        "appeal.resolved_epoch",
                        "must be later than submitted_epoch",
                    );
                }

                if self.resolution_evidence_root.is_none() {
                    return invalid(
                        "appeal.resolution_evidence_root",
                        "required for resolved appeal",
                    );
                }
            }
        }

        Ok(())
    }

    /// Whether an appeal is currently awaiting review.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self.state, ServiceNodeAppealStateV1::Pending)
    }

    /// Appeal visibility grants no lifecycle or economic mutation authority.
    #[must_use]
    pub const fn authorizes_state_change(&self) -> bool {
        false
    }
}

/// Evidence-only report describing one observed Service Node violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeViolationReportV1 {
    /// Stable wire schema.
    pub schema: String,

    /// DTO version.
    pub version: u16,

    /// Stable report identifier.
    pub report_id: String,

    /// Service Node accused by the objective report.
    pub service_node_id: String,

    /// Node or verifier that produced the report.
    pub reporter_id: String,

    /// Canonical violation signal.
    pub violation: ServiceNodeViolationKindV1,

    /// Root of objective evidence supporting the report.
    pub evidence_root: ContentId,

    /// Epoch in which the violation was detected.
    pub detected_epoch: u64,

    /// Number of observations represented by this report.
    pub observed_count: u64,
}

impl ServiceNodeViolationReportV1 {
    /// Validate report identity, epoch, count, and evidence-only posture.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeEnforcementValidationError`] for malformed
    /// identifiers, unsupported schema/version, zero epoch, or zero count.
    pub fn validate(&self) -> Result<(), ServiceNodeEnforcementValidationError> {
        validate_schema(&self.schema, SERVICE_NODE_VIOLATION_REPORT_SCHEMA)?;
        validate_version(self.version)?;
        validate_token("report_id", &self.report_id)?;
        validate_service_node_id(&self.service_node_id)?;
        validate_token("reporter_id", &self.reporter_id)?;

        if self.detected_epoch == 0 {
            return invalid("detected_epoch", "must be greater than zero");
        }

        if self.observed_count == 0 {
            return invalid("observed_count", "must be greater than zero");
        }

        Ok(())
    }

    /// Violation reports do not directly mutate registry lifecycle state.
    #[must_use]
    pub const fn authorizes_containment_mutation(&self) -> bool {
        false
    }

    /// Violation reports do not directly deny or execute rewards.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }
}

/// Registry-visible containment record using the Phase 18 lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeEnforcementStatusV1 {
    /// Stable wire schema.
    pub schema: String,

    /// DTO version.
    pub version: u16,

    /// Stable status identifier.
    pub status_id: String,

    /// Canonical Service Node identity.
    pub service_node_id: String,

    /// Existing Phase 18 containment lifecycle state.
    pub state: ServiceNodeEligibilityStateV1,

    /// Violation responsible for the current containment posture.
    pub reason: ServiceNodeViolationKindV1,

    /// Evidence root supporting the current status.
    pub evidence_root: ContentId,

    /// First epoch in which this status applies.
    pub effective_epoch: u64,

    /// Operator-visible appeal posture.
    pub appeal: ServiceNodeAppealStatusV1,
}

impl ServiceNodeEnforcementStatusV1 {
    /// Validate identity, containment state, epoch, and appeal posture.
    ///
    /// Only `Degraded`, `Quarantined`, and `Blocked` are valid here. Candidate,
    /// probation, and eligible descriptors remain represented by the canonical
    /// Phase 18 identity descriptor rather than a second status machine.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeEnforcementValidationError`] for malformed shape,
    /// a non-containment lifecycle state, or invalid appeal fields.
    pub fn validate(&self) -> Result<(), ServiceNodeEnforcementValidationError> {
        validate_schema(&self.schema, SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA)?;
        validate_version(self.version)?;
        validate_token("status_id", &self.status_id)?;
        validate_service_node_id(&self.service_node_id)?;

        if !matches!(
            self.state,
            ServiceNodeEligibilityStateV1::Degraded
                | ServiceNodeEligibilityStateV1::Quarantined
                | ServiceNodeEligibilityStateV1::Blocked
        ) {
            return invalid(
                "state",
                "must reuse degraded, quarantined, or blocked lifecycle state",
            );
        }

        if self.effective_epoch == 0 {
            return invalid("effective_epoch", "must be greater than zero");
        }

        self.appeal.validate()
    }

    /// Contained nodes do not count toward quorum.
    #[must_use]
    pub const fn counts_toward_quorum(&self) -> bool {
        self.state.counts_toward_quorum()
    }

    /// Phase 19 containment denies reward planning.
    #[must_use]
    pub const fn permits_reward_planning(&self) -> bool {
        false
    }

    /// Status projection grants no registry or economic mutation authority.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }
}

fn validate_schema(
    actual: &str,
    expected: &'static str,
) -> Result<(), ServiceNodeEnforcementValidationError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ServiceNodeEnforcementValidationError::InvalidSchema {
            expected,
            actual: actual.to_string(),
        })
    }
}

const fn validate_version(actual: u16) -> Result<(), ServiceNodeEnforcementValidationError> {
    if actual == SERVICE_NODE_ENFORCEMENT_VERSION {
        Ok(())
    } else {
        Err(ServiceNodeEnforcementValidationError::InvalidVersion {
            expected: SERVICE_NODE_ENFORCEMENT_VERSION,
            actual,
        })
    }
}

fn validate_service_node_id(value: &str) -> Result<(), ServiceNodeEnforcementValidationError> {
    validate_token("service_node_id", value)?;

    if value
        .strip_prefix("service_node:")
        .map_or(true, str::is_empty)
    {
        return invalid("service_node_id", "must use service_node:<id> form");
    }

    Ok(())
}

fn validate_token(
    field: &'static str,
    value: &str,
) -> Result<(), ServiceNodeEnforcementValidationError> {
    if value.trim().is_empty() {
        return invalid(field, "must not be empty");
    }

    if value.len() > MAX_REF_BYTES {
        return invalid(field, "exceeds maximum byte length");
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
    }) {
        return invalid(field, "contains unsupported characters");
    }

    Ok(())
}

fn validate_appeal_identity(
    appeal_id: Option<&str>,
    submitted_epoch: Option<u64>,
) -> Result<u64, ServiceNodeEnforcementValidationError> {
    let appeal_id = appeal_id
        .ok_or_else(|| invalid_error("appeal.appeal_id", "required when appeal exists"))?;

    validate_token("appeal.appeal_id", appeal_id)?;

    let submitted_epoch = submitted_epoch
        .ok_or_else(|| invalid_error("appeal.submitted_epoch", "required when appeal exists"))?;

    if submitted_epoch == 0 {
        return invalid("appeal.submitted_epoch", "must be greater than zero");
    }

    Ok(submitted_epoch)
}

const fn invalid_error(
    field: &'static str,
    reason: &'static str,
) -> ServiceNodeEnforcementValidationError {
    ServiceNodeEnforcementValidationError::InvalidField { field, reason }
}

const fn invalid<T>(
    field: &'static str,
    reason: &'static str,
) -> Result<T, ServiceNodeEnforcementValidationError> {
    Err(invalid_error(field, reason))
}
