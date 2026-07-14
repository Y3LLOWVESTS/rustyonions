//! RO:WHAT — Deterministic policy review for Service Node violation reports.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 19 requires malicious and unreliable nodes to
//! be contained and reward-denied using objective protocol evidence.
//!
//! RO:INTERACTS — `ron-proto` violation reports, Phase 18 eligibility
//! descriptors, later `svc-registry` containment custody, and reward review.
//!
//! RO:INVARIANTS — containment reuses degraded/quarantined/blocked; severity
//! never downgrades existing containment; every decision denies rewards and
//! quorum participation; appeals remain visible but non-authoritative.
//!
//! RO:CONFIG — callers supply reviewed repeat thresholds; no hidden defaults.
//!
//! RO:SECURITY — policy review cannot mutate registry, wallet, ledger, payout,
//! receipt, mint, burn, settlement, or finality state.
//!
//! RO:TEST —
//! `tests/internal_roc_beta_phase19_service_node_enforcement_policy.rs`.

#![forbid(unsafe_code)]

use ron_proto::{
    ContentId, ServiceNodeAppealStatusV1, ServiceNodeEligibilityStateV1,
    ServiceNodeEligibilityValidationError, ServiceNodeEnforcementValidationError,
    ServiceNodeIdentityDescriptorV1, ServiceNodeViolationKindV1, ServiceNodeViolationReportV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ServiceNodeRewardReviewPostureV1;

/// Current Phase 19 violation-policy version.
pub const SERVICE_NODE_VIOLATION_POLICY_VERSION: u16 = 1;

/// Reviewed thresholds for repeated bad-node observations.
///
/// Single severe violations still apply their baseline containment immediately.
/// These thresholds escalate repeated lower-severity behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeViolationPolicyV1 {
    /// Policy version.
    pub version: u16,

    /// Observation count that escalates a lower-severity signal to quarantine.
    pub repeated_violation_quarantine_count: u64,

    /// Observation count that escalates any signal to blocked.
    pub repeated_violation_block_count: u64,
}

impl ServiceNodeViolationPolicyV1 {
    /// Validate reviewed repeat thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeViolationPolicyError`] when the version is
    /// unsupported or thresholds would permit immediate or unordered escalation.
    pub const fn validate(self) -> Result<(), ServiceNodeViolationPolicyError> {
        if self.version != SERVICE_NODE_VIOLATION_POLICY_VERSION {
            return Err(ServiceNodeViolationPolicyError::InvalidPolicy {
                field: "version",
                reason: "unsupported violation-policy version",
            });
        }

        if self.repeated_violation_quarantine_count < 2 {
            return Err(ServiceNodeViolationPolicyError::InvalidPolicy {
                field: "repeated_violation_quarantine_count",
                reason: "must be at least two",
            });
        }

        if self.repeated_violation_block_count <= self.repeated_violation_quarantine_count {
            return Err(ServiceNodeViolationPolicyError::InvalidPolicy {
                field: "repeated_violation_block_count",
                reason: "must be greater than the quarantine threshold",
            });
        }

        Ok(())
    }

    /// Evaluate one validated violation report against canonical node state.
    ///
    /// Severe signals apply their baseline containment immediately. Repeated
    /// observations may escalate lower-severity signals. Existing quarantine or
    /// blocked state is never downgraded.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeViolationPolicyError`] for malformed policy,
    /// descriptor, or report data; identity mismatch; stale evidence; or an
    /// unsupported future violation kind.
    pub fn evaluate(
        self,
        descriptor: &ServiceNodeIdentityDescriptorV1,
        report: &ServiceNodeViolationReportV1,
    ) -> Result<ServiceNodeViolationDecisionV1, ServiceNodeViolationPolicyError> {
        self.validate()?;
        descriptor.validate()?;
        report.validate()?;

        if descriptor.service_node_id != report.service_node_id {
            return Err(ServiceNodeViolationPolicyError::ServiceNodeIdMismatch {
                expected: descriptor.service_node_id.clone(),
                actual: report.service_node_id.clone(),
            });
        }

        if report.detected_epoch < descriptor.registered_at_epoch
            || report.detected_epoch < descriptor.state_effective_epoch
        {
            return Err(ServiceNodeViolationPolicyError::StaleViolationReport {
                detected_epoch: report.detected_epoch,
                registered_at_epoch: descriptor.registered_at_epoch,
                state_effective_epoch: descriptor.state_effective_epoch,
            });
        }

        let baseline = baseline_containment_state(report.violation)?;

        let recommended = self.escalated_state(baseline, report.observed_count);

        let next_state = stronger_containment(descriptor.state, recommended);

        let reason = decision_reason(descriptor.state, baseline, recommended, next_state);

        let state_effective_epoch = if next_state == descriptor.state {
            descriptor.state_effective_epoch
        } else {
            report.detected_epoch
        };

        Ok(ServiceNodeViolationDecisionV1 {
            version: SERVICE_NODE_VIOLATION_POLICY_VERSION,
            report_id: report.report_id.clone(),
            service_node_id: report.service_node_id.clone(),
            violation: report.violation,
            previous_state: descriptor.state,
            next_state,
            reason,
            evidence_root: report.evidence_root.clone(),
            evaluation_epoch: report.detected_epoch,
            state_effective_epoch,
            appeal: ServiceNodeAppealStatusV1::not_appealed(),
            reward_review: ServiceNodeRewardReviewPostureV1::Denied,
        })
    }

    const fn escalated_state(
        self,
        baseline: ServiceNodeEligibilityStateV1,
        observed_count: u64,
    ) -> ServiceNodeEligibilityStateV1 {
        if observed_count >= self.repeated_violation_block_count {
            return ServiceNodeEligibilityStateV1::Blocked;
        }

        if observed_count >= self.repeated_violation_quarantine_count
            && containment_rank(baseline)
                < containment_rank(ServiceNodeEligibilityStateV1::Quarantined)
        {
            return ServiceNodeEligibilityStateV1::Quarantined;
        }

        baseline
    }
}

/// Stable explanation for one violation-policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServiceNodeViolationDecisionReasonV1 {
    /// The signal's baseline policy classification selected containment.
    BaselineContainment,

    /// Repeated observations escalated a lower-severity signal to quarantine.
    RepeatedViolationQuarantined,

    /// Repeated observations escalated the node to blocked.
    RepeatedViolationBlocked,

    /// Existing stronger containment was retained.
    ExistingContainmentMaintained,
}

/// Non-authoritative result of one violation-policy review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeViolationDecisionV1 {
    /// Decision schema version.
    pub version: u16,

    /// Violation report reviewed by policy.
    pub report_id: String,

    /// Canonical Service Node identity.
    pub service_node_id: String,

    /// Objective signal reviewed.
    pub violation: ServiceNodeViolationKindV1,

    /// Registry state present during review.
    pub previous_state: ServiceNodeEligibilityStateV1,

    /// Containment state selected by policy.
    pub next_state: ServiceNodeEligibilityStateV1,

    /// Stable reason for the selected state.
    pub reason: ServiceNodeViolationDecisionReasonV1,

    /// Evidence commitment copied from the reviewed report.
    pub evidence_root: ContentId,

    /// Epoch in which the violation was reviewed.
    pub evaluation_epoch: u64,

    /// Epoch to use if a later registry transition is approved.
    pub state_effective_epoch: u64,

    /// Operator-visible appeal posture.
    pub appeal: ServiceNodeAppealStatusV1,

    /// Reward posture selected for every contained node.
    pub reward_review: ServiceNodeRewardReviewPostureV1,
}

impl ServiceNodeViolationDecisionV1 {
    /// Whether policy selected a lifecycle transition.
    #[must_use]
    pub fn is_transition(&self) -> bool {
        self.previous_state != self.next_state
    }

    /// Containment decisions never count toward quorum.
    #[must_use]
    pub const fn counts_toward_quorum(&self) -> bool {
        self.next_state.counts_toward_quorum()
    }

    /// Every Phase 19 containment decision denies reward planning.
    #[must_use]
    pub const fn rewards_denied(&self) -> bool {
        true
    }

    /// Policy evaluation does not mutate registry state.
    #[must_use]
    pub const fn authorizes_registry_mutation(&self) -> bool {
        false
    }

    /// Policy evaluation grants no economic mutation authority.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }

    /// Policy evaluation does not resolve appeals.
    #[must_use]
    pub const fn authorizes_appeal_resolution(&self) -> bool {
        false
    }
}

/// Deterministic Phase 19 violation-policy failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeViolationPolicyError {
    /// Reviewed policy values were malformed or unsafe.
    #[error("invalid Service Node violation policy field {field}: {reason}")]
    InvalidPolicy {
        /// Invalid field.
        field: &'static str,

        /// Stable validation reason.
        reason: &'static str,
    },

    /// Canonical node descriptor failed validation.
    #[error(transparent)]
    InvalidDescriptor(#[from] ServiceNodeEligibilityValidationError),

    /// Canonical violation report failed validation.
    #[error(transparent)]
    InvalidReport(#[from] ServiceNodeEnforcementValidationError),

    /// Report and descriptor named different nodes.
    #[error("Service Node violation identity mismatch: expected {expected}, got {actual}")]
    ServiceNodeIdMismatch {
        /// Descriptor node identity.
        expected: String,

        /// Report node identity.
        actual: String,
    },

    /// Evidence predates registration or current-state activation.
    #[error(
        "stale Service Node violation report epoch {detected_epoch}: registration={registered_at_epoch}, state_effective={state_effective_epoch}"
    )]
    StaleViolationReport {
        /// Reported detection epoch.
        detected_epoch: u64,

        /// Node registration epoch.
        registered_at_epoch: u64,

        /// Current state activation epoch.
        state_effective_epoch: u64,
    },

    /// A future violation kind lacks reviewed containment semantics.
    #[error("unsupported Service Node violation kind")]
    UnsupportedViolationKind,
}

const fn baseline_containment_state(
    violation: ServiceNodeViolationKindV1,
) -> Result<ServiceNodeEligibilityStateV1, ServiceNodeViolationPolicyError> {
    match violation {
        ServiceNodeViolationKindV1::ProviderSpam | ServiceNodeViolationKindV1::ChallengeFailure => {
            Ok(ServiceNodeEligibilityStateV1::Degraded)
        }

        ServiceNodeViolationKindV1::HashMismatch
        | ServiceNodeViolationKindV1::FakeDeliveryProof
        | ServiceNodeViolationKindV1::ReplayAttempt
        | ServiceNodeViolationKindV1::SelfTrafficLoop
        | ServiceNodeViolationKindV1::InvalidEpochProposal
        | ServiceNodeViolationKindV1::InvalidEpochSignature
        | ServiceNodeViolationKindV1::PrivacyLeak
        | ServiceNodeViolationKindV1::RewardRecipientBindingAbuse => {
            Ok(ServiceNodeEligibilityStateV1::Quarantined)
        }

        ServiceNodeViolationKindV1::DenylistViolation
        | ServiceNodeViolationKindV1::TombstoneViolation
        | ServiceNodeViolationKindV1::UnilateralMintAttempt => {
            Ok(ServiceNodeEligibilityStateV1::Blocked)
        }

        _ => Err(ServiceNodeViolationPolicyError::UnsupportedViolationKind),
    }
}

const fn containment_rank(state: ServiceNodeEligibilityStateV1) -> u8 {
    match state {
        ServiceNodeEligibilityStateV1::Degraded => 1,
        ServiceNodeEligibilityStateV1::Quarantined => 2,
        ServiceNodeEligibilityStateV1::Blocked => 3,
        _ => 0,
    }
}

const fn stronger_containment(
    current: ServiceNodeEligibilityStateV1,
    recommended: ServiceNodeEligibilityStateV1,
) -> ServiceNodeEligibilityStateV1 {
    if containment_rank(current) >= containment_rank(recommended) && containment_rank(current) > 0 {
        current
    } else {
        recommended
    }
}

const fn decision_reason(
    current: ServiceNodeEligibilityStateV1,
    baseline: ServiceNodeEligibilityStateV1,
    recommended: ServiceNodeEligibilityStateV1,
    selected: ServiceNodeEligibilityStateV1,
) -> ServiceNodeViolationDecisionReasonV1 {
    if containment_rank(current) > containment_rank(recommended)
        || (containment_rank(current) > 0
            && containment_rank(current) == containment_rank(recommended))
    {
        return ServiceNodeViolationDecisionReasonV1::ExistingContainmentMaintained;
    }

    if matches!(selected, ServiceNodeEligibilityStateV1::Blocked)
        && !matches!(baseline, ServiceNodeEligibilityStateV1::Blocked)
    {
        return ServiceNodeViolationDecisionReasonV1::RepeatedViolationBlocked;
    }

    if matches!(selected, ServiceNodeEligibilityStateV1::Quarantined)
        && matches!(baseline, ServiceNodeEligibilityStateV1::Degraded)
    {
        return ServiceNodeViolationDecisionReasonV1::RepeatedViolationQuarantined;
    }

    ServiceNodeViolationDecisionReasonV1::BaselineContainment
}
