//! RO:WHAT — Registry custody and replay-safe application of bad-node containment.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 19 requires policy-derived containment to
//! become canonical registry state while evidence and appeals remain visible.
//!
//! RO:INTERACTS — `ron-proto` violation/status DTOs, `ron-policy` violation
//! review, and the Phase 18 `ServiceNodeEligibilityRegistry` state model.
//!
//! RO:INVARIANTS — report IDs, evidence roots, and node epochs do not replay;
//! containment never downgrades; appeals do not restore eligibility; one
//! canonical descriptor remains lifecycle truth.
//!
//! RO:SECURITY — no wallet, ledger, reward execution, receipt, payout, mint,
//! burn, settlement, quorum-signing, or finality authority.
//!
//! RO:TEST —
//! `tests/internal_roc_beta_phase19_enforcement_registry.rs`.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use ron_policy::{
    ServiceNodeRewardReviewPostureV1, ServiceNodeViolationDecisionV1,
    ServiceNodeViolationPolicyError, ServiceNodeViolationPolicyV1,
};
use ron_proto::{
    ContentId, ServiceNodeAppealStateV1, ServiceNodeAppealStatusV1, ServiceNodeEligibilityStateV1,
    ServiceNodeEnforcementStatusV1, ServiceNodeEnforcementValidationError,
    ServiceNodeIdentityDescriptorV1, ServiceNodeViolationReportV1,
    SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA, SERVICE_NODE_ENFORCEMENT_VERSION,
};
use thiserror::Error;

use crate::eligibility::{ServiceNodeEligibilityRegistry, ServiceNodeEligibilityTransitionError};

/// Process-local custody for Phase 19 violation evidence and status.
///
/// This model is intentionally independent from durable storage. It proves
/// validation, replay handling, atomic descriptor application, and visible
/// appeals before persistence or network intake is introduced.
#[derive(Debug, Clone, Default)]
pub struct ServiceNodeEnforcementRegistry {
    reports_by_id: BTreeMap<String, ServiceNodeViolationReportV1>,
    report_id_by_evidence_root: BTreeMap<ContentId, String>,
    last_violation_epoch_by_node: BTreeMap<String, u64>,
    status_by_node: BTreeMap<String, ServiceNodeEnforcementStatusV1>,
    service_node_by_appeal_id: BTreeMap<String, String>,
}

impl ServiceNodeEnforcementRegistry {
    /// Create empty process-local enforcement custody.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            reports_by_id: BTreeMap::new(),
            report_id_by_evidence_root: BTreeMap::new(),
            last_violation_epoch_by_node: BTreeMap::new(),
            status_by_node: BTreeMap::new(),
            service_node_by_appeal_id: BTreeMap::new(),
        }
    }

    /// Return one accepted violation report.
    #[must_use]
    pub fn report(&self, report_id: &str) -> Option<&ServiceNodeViolationReportV1> {
        self.reports_by_id.get(report_id)
    }

    /// Return current visible containment and appeal status for a node.
    #[must_use]
    pub fn status(&self, service_node_id: &str) -> Option<&ServiceNodeEnforcementStatusV1> {
        self.status_by_node.get(service_node_id)
    }

    /// Number of accepted, non-replayed violation reports.
    #[must_use]
    pub fn report_count(&self) -> usize {
        self.reports_by_id.len()
    }

    /// Evaluate and atomically apply one violation report.
    ///
    /// Validation and every possible fallible review occur before registry or
    /// enforcement custody is changed. A successful result consumes the report
    /// ID, evidence root, and per-node violation epoch exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeEnforcementRegistryError`] for malformed evidence,
    /// replay, missing identity, stale epoch, inconsistent policy output, or a
    /// failed canonical descriptor transition.
    pub fn evaluate_and_apply_violation(
        &mut self,
        eligibility: &mut ServiceNodeEligibilityRegistry,
        policy: ServiceNodeViolationPolicyV1,
        report: ServiceNodeViolationReportV1,
    ) -> Result<ServiceNodeEnforcementTransitionV1, ServiceNodeEnforcementRegistryError> {
        report.validate()?;

        if self.reports_by_id.contains_key(&report.report_id) {
            return Err(ServiceNodeEnforcementRegistryError::ReportReplay {
                report_id: report.report_id,
            });
        }

        if let Some(original_report_id) = self.report_id_by_evidence_root.get(&report.evidence_root)
        {
            return Err(ServiceNodeEnforcementRegistryError::EvidenceReplay {
                evidence_root: report.evidence_root,
                original_report_id: original_report_id.clone(),
            });
        }

        if let Some(last_applied_epoch) = self
            .last_violation_epoch_by_node
            .get(&report.service_node_id)
            .copied()
        {
            if report.detected_epoch <= last_applied_epoch {
                return Err(ServiceNodeEnforcementRegistryError::ViolationEpochReplay {
                    service_node_id: report.service_node_id,
                    detected_epoch: report.detected_epoch,
                    last_applied_epoch,
                });
            }
        }

        let descriptor = eligibility
            .descriptor(&report.service_node_id)
            .cloned()
            .ok_or_else(|| ServiceNodeEnforcementRegistryError::DescriptorNotFound {
                service_node_id: report.service_node_id.clone(),
            })?;

        let decision = policy.evaluate(&descriptor, &report)?;

        validate_policy_decision(&descriptor, &report, &decision)?;

        let state_changed = decision.previous_state != decision.next_state;

        let existing_status = self.status_by_node.get(&report.service_node_id).cloned();

        let (next_status, status_changed) = if state_changed || existing_status.is_none() {
            let appeal = existing_status
                .as_ref()
                .map_or_else(ServiceNodeAppealStatusV1::not_appealed, |status| {
                    status.appeal.clone()
                });

            let status = ServiceNodeEnforcementStatusV1 {
                schema: SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA.to_string(),
                version: SERVICE_NODE_ENFORCEMENT_VERSION,
                status_id: format!("enforcement:{}", report.report_id),
                service_node_id: report.service_node_id.clone(),
                state: decision.next_state,
                reason: report.violation,
                evidence_root: report.evidence_root.clone(),
                effective_epoch: decision.state_effective_epoch,
                appeal,
            };

            status.validate()?;
            (status, true)
        } else {
            let Some(status) = existing_status else {
                return Err(ServiceNodeEnforcementRegistryError::InvalidPolicyDecision);
            };

            (status, false)
        };

        let updated_descriptor = eligibility.apply_reviewed_containment(
            &report.service_node_id,
            decision.previous_state,
            decision.next_state,
            decision.state_effective_epoch,
        )?;

        self.reports_by_id
            .insert(report.report_id.clone(), report.clone());

        self.report_id_by_evidence_root
            .insert(report.evidence_root.clone(), report.report_id.clone());

        self.last_violation_epoch_by_node
            .insert(report.service_node_id.clone(), report.detected_epoch);

        self.status_by_node
            .insert(report.service_node_id.clone(), next_status.clone());

        Ok(ServiceNodeEnforcementTransitionV1 {
            report,
            decision,
            descriptor: updated_descriptor,
            status: next_status,
            state_changed,
            status_changed,
        })
    }

    /// Submit one visible pending appeal for current containment.
    ///
    /// Submitting an appeal changes only the visible appeal posture. It cannot
    /// alter the canonical descriptor or restore quorum/reward eligibility.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeEnforcementRegistryError`] when no containment
    /// exists, an appeal ID replays, another appeal already exists, the epoch
    /// predates containment, or the resulting status is invalid.
    pub fn submit_appeal(
        &mut self,
        service_node_id: &str,
        appeal_id: String,
        submitted_epoch: u64,
    ) -> Result<ServiceNodeEnforcementStatusV1, ServiceNodeEnforcementRegistryError> {
        if let Some(original_service_node_id) = self.service_node_by_appeal_id.get(&appeal_id) {
            return Err(ServiceNodeEnforcementRegistryError::AppealIdReplay {
                appeal_id,
                original_service_node_id: original_service_node_id.clone(),
            });
        }

        let current = self
            .status_by_node
            .get(service_node_id)
            .cloned()
            .ok_or_else(
                || ServiceNodeEnforcementRegistryError::EnforcementStatusNotFound {
                    service_node_id: service_node_id.to_string(),
                },
            )?;

        if current.appeal.state != ServiceNodeAppealStateV1::NotAppealed {
            return Err(ServiceNodeEnforcementRegistryError::AppealAlreadyExists {
                service_node_id: service_node_id.to_string(),
                state: current.appeal.state,
            });
        }

        if submitted_epoch < current.effective_epoch {
            return Err(
                ServiceNodeEnforcementRegistryError::AppealPredatesContainment {
                    submitted_epoch,
                    containment_epoch: current.effective_epoch,
                },
            );
        }

        let mut updated = current;
        updated.appeal = ServiceNodeAppealStatusV1 {
            state: ServiceNodeAppealStateV1::Pending,
            appeal_id: Some(appeal_id.clone()),
            submitted_epoch: Some(submitted_epoch),
            resolved_epoch: None,
            resolution_evidence_root: None,
        };

        updated.validate()?;

        self.service_node_by_appeal_id
            .insert(appeal_id, service_node_id.to_string());

        self.status_by_node
            .insert(service_node_id.to_string(), updated.clone());

        Ok(updated)
    }
}

/// Result of one registry-applied violation review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNodeEnforcementTransitionV1 {
    /// Accepted objective report.
    pub report: ServiceNodeViolationReportV1,

    /// Deterministic policy decision.
    pub decision: ServiceNodeViolationDecisionV1,

    /// Canonical descriptor after application.
    pub descriptor: ServiceNodeIdentityDescriptorV1,

    /// Current registry-visible containment and appeal status.
    pub status: ServiceNodeEnforcementStatusV1,

    /// Whether the canonical lifecycle state changed.
    pub state_changed: bool,

    /// Whether the visible enforcement status changed.
    pub status_changed: bool,
}

impl ServiceNodeEnforcementTransitionV1 {
    /// Whether the resulting descriptor counts toward quorum.
    #[must_use]
    pub const fn counts_toward_quorum(&self) -> bool {
        self.descriptor.state.counts_toward_quorum()
    }

    /// Whether reward planning is denied.
    #[must_use]
    pub const fn rewards_denied(&self) -> bool {
        true
    }

    /// Registry containment grants no direct economic mutation.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }

    /// Registry containment does not resolve appeals.
    #[must_use]
    pub const fn authorizes_appeal_resolution(&self) -> bool {
        false
    }
}

/// Failure while accepting violation evidence or appeal submission.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeEnforcementRegistryError {
    /// A protocol enforcement artifact failed validation.
    #[error(transparent)]
    InvalidEnforcementArtifact(#[from] ServiceNodeEnforcementValidationError),

    /// No canonical Service Node descriptor exists.
    #[error("Service Node eligibility descriptor not found: {service_node_id}")]
    DescriptorNotFound {
        /// Missing Service Node identity.
        service_node_id: String,
    },

    /// A report ID was already consumed.
    #[error("Service Node violation report replay: {report_id}")]
    ReportReplay {
        /// Replayed report identity.
        report_id: String,
    },

    /// An evidence root was already consumed under another report ID.
    #[error(
        "Service Node violation evidence replay: evidence={evidence_root}, original_report={original_report_id}"
    )]
    EvidenceReplay {
        /// Replayed objective evidence root.
        evidence_root: ContentId,

        /// First report that consumed the evidence.
        original_report_id: String,
    },

    /// A node report did not advance its accepted violation epoch.
    #[error(
        "Service Node violation epoch replay for {service_node_id}: proposed={detected_epoch}, last={last_applied_epoch}"
    )]
    ViolationEpochReplay {
        /// Service Node identity.
        service_node_id: String,

        /// Proposed report epoch.
        detected_epoch: u64,

        /// Last consumed violation epoch.
        last_applied_epoch: u64,
    },

    /// Policy output contradicted its descriptor or report inputs.
    #[error("Service Node violation policy produced an inconsistent decision")]
    InvalidPolicyDecision,

    /// Deterministic policy evaluation rejected its inputs.
    #[error(transparent)]
    Policy(#[from] ServiceNodeViolationPolicyError),

    /// Canonical descriptor containment application failed.
    #[error(transparent)]
    EligibilityTransition(#[from] ServiceNodeEligibilityTransitionError),

    /// No visible containment exists for an appeal.
    #[error("Service Node enforcement status not found: {service_node_id}")]
    EnforcementStatusNotFound {
        /// Missing Service Node identity.
        service_node_id: String,
    },

    /// An appeal ID was already consumed.
    #[error("Service Node appeal replay: {appeal_id}, original_node={original_service_node_id}")]
    AppealIdReplay {
        /// Replayed appeal identity.
        appeal_id: String,

        /// Node that first consumed the appeal ID.
        original_service_node_id: String,
    },

    /// Current containment already has an appeal posture.
    #[error("Service Node appeal already exists for {service_node_id}: {state:?}")]
    AppealAlreadyExists {
        /// Service Node identity.
        service_node_id: String,

        /// Existing appeal state.
        state: ServiceNodeAppealStateV1,
    },

    /// Appeal submission predates the containment it challenges.
    #[error(
        "Service Node appeal epoch {submitted_epoch} predates containment epoch {containment_epoch}"
    )]
    AppealPredatesContainment {
        /// Proposed appeal epoch.
        submitted_epoch: u64,

        /// Current containment epoch.
        containment_epoch: u64,
    },
}

fn validate_policy_decision(
    descriptor: &ServiceNodeIdentityDescriptorV1,
    report: &ServiceNodeViolationReportV1,
    decision: &ServiceNodeViolationDecisionV1,
) -> Result<(), ServiceNodeEnforcementRegistryError> {
    let containment_state = matches!(
        decision.next_state,
        ServiceNodeEligibilityStateV1::Degraded
            | ServiceNodeEligibilityStateV1::Quarantined
            | ServiceNodeEligibilityStateV1::Blocked
    );

    if decision.report_id != report.report_id
        || decision.service_node_id != report.service_node_id
        || decision.violation != report.violation
        || decision.evidence_root != report.evidence_root
        || decision.previous_state != descriptor.state
        || decision.evaluation_epoch != report.detected_epoch
        || decision.reward_review != ServiceNodeRewardReviewPostureV1::Denied
        || decision.counts_toward_quorum()
        || !decision.rewards_denied()
        || !containment_state
    {
        return Err(ServiceNodeEnforcementRegistryError::InvalidPolicyDecision);
    }

    Ok(())
}
