//! RO:WHAT — Deterministic policy evaluation for protocol-earned Service Node eligibility.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 18 requires objective promotion, degradation, quarantine, and reward-review gates.
//! RO:INTERACTS — `ron-proto` Service Node descriptors, `svc-registry` history custody, and later quorum/reward enforcement.
//! RO:INVARIANTS — thresholds are caller-supplied; history roots must match; candidate cannot skip probation; false/pending challenges do not punish.
//! RO:CONFIG — no production defaults; reviewed policy configuration must provide every threshold and challenge limit.
//! RO:SECURITY — decisions do not mutate registry, quorum, rewards, wallet, ledger, balances, receipts, minting, or finality.
//! RO:TEST — `tests/internal_roc_beta_phase18_service_node_eligibility_policy.rs`.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use ron_proto::{
    ContentId, ServiceNodeEligibilityStateV1, ServiceNodeEligibilityValidationError,
    ServiceNodeIdentityDescriptorV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::economics::INTERNAL_ROC_BPS_DENOMINATOR;

/// Current deterministic Service Node eligibility-policy version.
pub const SERVICE_NODE_ELIGIBILITY_POLICY_VERSION: u16 = 1;

type EligibilityTransition = (
    ServiceNodeEligibilityStateV1,
    ServiceNodeEligibilityReasonCodeV1,
);

/// Objective service-history requirements for one lifecycle transition.
///
/// Counts apply to the period represented by the descriptor's current state
/// and committed history roots. The policy evaluator does not collect or
/// attest these counts; upstream accounting/audit material must do that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeHistoryThresholdsV1 {
    /// Minimum epochs spent in the current lifecycle state.
    pub minimum_state_age_epochs: u64,

    /// Minimum successful useful-service events observed in the current state.
    pub minimum_successful_service_events: u64,

    /// Minimum distinct requester identities represented by the service history.
    pub minimum_distinct_requesters: u64,

    /// Minimum independent provider peers represented by corroborating history.
    pub minimum_distinct_provider_peers: u64,
}

impl ServiceNodeHistoryThresholdsV1 {
    const fn validate(
        self,
        policy_field: &'static str,
    ) -> Result<(), ServiceNodeEligibilityPolicyError> {
        if self.minimum_state_age_epochs == 0 {
            return invalid_policy(
                policy_field,
                "minimum_state_age_epochs must be greater than zero",
            );
        }

        if self.minimum_successful_service_events == 0 {
            return invalid_policy(
                policy_field,
                "minimum_successful_service_events must be greater than zero",
            );
        }

        if self.minimum_distinct_requesters == 0 {
            return invalid_policy(
                policy_field,
                "minimum_distinct_requesters must be greater than zero",
            );
        }

        if self.minimum_distinct_provider_peers == 0 {
            return invalid_policy(
                policy_field,
                "minimum_distinct_provider_peers must be greater than zero",
            );
        }

        Ok(())
    }

    const fn is_met_by(
        self,
        state_age_epochs: u64,
        observation: &ServiceNodeEligibilityObservationV1,
        failure_rate_bps: u16,
        maximum_failure_rate_bps: u16,
    ) -> bool {
        state_age_epochs >= self.minimum_state_age_epochs
            && observation.successful_service_events_in_state
                >= self.minimum_successful_service_events
            && observation.distinct_requesters_in_state >= self.minimum_distinct_requesters
            && observation.distinct_provider_peers_in_state >= self.minimum_distinct_provider_peers
            && failure_rate_bps <= maximum_failure_rate_bps
    }
}

/// Reviewed objective policy for Service Node lifecycle evaluation.
///
/// No defaults are provided. Callers must load and validate an explicit policy
/// document so production thresholds cannot be silently scattered through
/// runtime crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeEligibilityPolicyV1 {
    /// Policy version.
    pub version: u16,

    /// Requirements to move from candidate to probation.
    pub candidate_to_probation: ServiceNodeHistoryThresholdsV1,

    /// Requirements to move from probation to eligible.
    pub probation_to_eligible: ServiceNodeHistoryThresholdsV1,

    /// Maximum failed-service rate accepted for promotion or eligibility.
    pub maximum_failure_rate_bps: u16,

    /// Upheld challenges required to enter degraded state.
    pub degrade_upheld_challenge_count: u64,

    /// Upheld challenges required to enter quarantine.
    pub quarantine_upheld_challenge_count: u64,

    /// Upheld challenges required to enter blocked state.
    pub block_upheld_challenge_count: u64,
}

impl ServiceNodeEligibilityPolicyV1 {
    /// Validate policy thresholds and challenge escalation order.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeEligibilityPolicyError::InvalidPolicy`] when the
    /// version, history thresholds, failure-rate bound, or challenge escalation
    /// order is unsafe or malformed.
    pub fn validate(self) -> Result<(), ServiceNodeEligibilityPolicyError> {
        if self.version != SERVICE_NODE_ELIGIBILITY_POLICY_VERSION {
            return invalid_policy("version", "unsupported eligibility-policy version");
        }

        self.candidate_to_probation
            .validate("candidate_to_probation")?;
        self.probation_to_eligible
            .validate("probation_to_eligible")?;

        if self.probation_to_eligible.minimum_successful_service_events
            < self
                .candidate_to_probation
                .minimum_successful_service_events
        {
            return invalid_policy(
                "probation_to_eligible",
                "successful-service threshold must not be lower than candidate threshold",
            );
        }

        if self.probation_to_eligible.minimum_distinct_requesters
            < self.candidate_to_probation.minimum_distinct_requesters
        {
            return invalid_policy(
                "probation_to_eligible",
                "requester-diversity threshold must not be lower than candidate threshold",
            );
        }

        if self.probation_to_eligible.minimum_distinct_provider_peers
            < self.candidate_to_probation.minimum_distinct_provider_peers
        {
            return invalid_policy(
                "probation_to_eligible",
                "provider-diversity threshold must not be lower than candidate threshold",
            );
        }

        if self.maximum_failure_rate_bps > INTERNAL_ROC_BPS_DENOMINATOR {
            return invalid_policy(
                "maximum_failure_rate_bps",
                "must not exceed the canonical basis-point denominator",
            );
        }

        if self.degrade_upheld_challenge_count == 0 {
            return invalid_policy(
                "degrade_upheld_challenge_count",
                "must be greater than zero",
            );
        }

        if self.degrade_upheld_challenge_count >= self.quarantine_upheld_challenge_count {
            return invalid_policy(
                "quarantine_upheld_challenge_count",
                "must be greater than the degradation threshold",
            );
        }

        if self.quarantine_upheld_challenge_count >= self.block_upheld_challenge_count {
            return invalid_policy(
                "block_upheld_challenge_count",
                "must be greater than the quarantine threshold",
            );
        }

        Ok(())
    }

    /// Evaluate one canonical descriptor against objective history.
    ///
    /// The returned decision is policy output only. A separate authenticated
    /// registry transition must validate and apply it. This function cannot
    /// update registry state or confer economic/quorum authority.
    ///
    /// Candidate promotion is deliberately staged:
    ///
    /// ```text
    /// candidate -> probation -> eligible
    /// ```
    ///
    /// A candidate can never skip directly to eligible, regardless of how
    /// much history it accumulated before its first evaluation.
    ///
    /// # Errors
    ///
    /// Returns an error when the policy is invalid, the descriptor is invalid,
    /// the observation targets another node or epoch, either committed history
    /// root differs, or an unknown future lifecycle state is encountered.
    pub fn evaluate(
        self,
        descriptor: &ServiceNodeIdentityDescriptorV1,
        observation: &ServiceNodeEligibilityObservationV1,
    ) -> Result<ServiceNodeEligibilityDecisionV1, ServiceNodeEligibilityPolicyError> {
        self.validate()?;
        observation.validate_against(descriptor)?;

        let failure_rate_bps = observation.failure_rate_bps();
        let state_age_epochs = observation
            .evaluation_epoch
            .saturating_sub(descriptor.state_effective_epoch);

        let (next_state, reason) =
            self.select_next_state(descriptor, observation, state_age_epochs, failure_rate_bps)?;

        let state_effective_epoch = if next_state == descriptor.state {
            descriptor.state_effective_epoch
        } else {
            observation.evaluation_epoch
        };

        Ok(ServiceNodeEligibilityDecisionV1 {
            version: SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
            service_node_id: descriptor.service_node_id.clone(),
            previous_state: descriptor.state,
            next_state,
            reason,
            evaluation_epoch: observation.evaluation_epoch,
            state_effective_epoch,
            observed_failure_rate_bps: failure_rate_bps,
            reward_review: reward_review_for_state(next_state),
        })
    }

    const fn select_next_state(
        self,
        descriptor: &ServiceNodeIdentityDescriptorV1,
        observation: &ServiceNodeEligibilityObservationV1,
        state_age_epochs: u64,
        failure_rate_bps: u16,
    ) -> Result<EligibilityTransition, ServiceNodeEligibilityPolicyError> {
        if let Some(transition) =
            self.challenge_transition(descriptor.state, observation.upheld_challenges_in_state)
        {
            return Ok(transition);
        }

        self.history_transition(
            descriptor.state,
            observation,
            state_age_epochs,
            failure_rate_bps,
        )
    }

    const fn challenge_transition(
        self,
        current_state: ServiceNodeEligibilityStateV1,
        upheld_challenges: u64,
    ) -> Option<EligibilityTransition> {
        if matches!(current_state, ServiceNodeEligibilityStateV1::Blocked) {
            return Some((
                ServiceNodeEligibilityStateV1::Blocked,
                ServiceNodeEligibilityReasonCodeV1::BlockedMaintained,
            ));
        }

        if upheld_challenges >= self.block_upheld_challenge_count {
            return Some((
                ServiceNodeEligibilityStateV1::Blocked,
                ServiceNodeEligibilityReasonCodeV1::ChallengeHistoryBlocked,
            ));
        }

        if matches!(current_state, ServiceNodeEligibilityStateV1::Quarantined) {
            return Some((
                ServiceNodeEligibilityStateV1::Quarantined,
                ServiceNodeEligibilityReasonCodeV1::QuarantineMaintained,
            ));
        }

        if upheld_challenges >= self.quarantine_upheld_challenge_count {
            return Some((
                ServiceNodeEligibilityStateV1::Quarantined,
                ServiceNodeEligibilityReasonCodeV1::ChallengeHistoryQuarantined,
            ));
        }

        if upheld_challenges >= self.degrade_upheld_challenge_count {
            return Some((
                ServiceNodeEligibilityStateV1::Degraded,
                ServiceNodeEligibilityReasonCodeV1::ChallengeHistoryDegraded,
            ));
        }

        None
    }

    const fn history_transition(
        self,
        current_state: ServiceNodeEligibilityStateV1,
        observation: &ServiceNodeEligibilityObservationV1,
        state_age_epochs: u64,
        failure_rate_bps: u16,
    ) -> Result<EligibilityTransition, ServiceNodeEligibilityPolicyError> {
        match current_state {
            ServiceNodeEligibilityStateV1::Candidate => {
                Ok(self.candidate_transition(observation, state_age_epochs, failure_rate_bps))
            }
            ServiceNodeEligibilityStateV1::Probation => {
                Ok(self.probation_transition(observation, state_age_epochs, failure_rate_bps))
            }
            ServiceNodeEligibilityStateV1::Eligible => {
                Ok(self.eligible_transition(failure_rate_bps))
            }
            ServiceNodeEligibilityStateV1::Degraded => {
                Ok(self.degraded_transition(observation, state_age_epochs, failure_rate_bps))
            }
            ServiceNodeEligibilityStateV1::Quarantined => Ok((
                ServiceNodeEligibilityStateV1::Quarantined,
                ServiceNodeEligibilityReasonCodeV1::QuarantineMaintained,
            )),
            ServiceNodeEligibilityStateV1::Blocked => Ok((
                ServiceNodeEligibilityStateV1::Blocked,
                ServiceNodeEligibilityReasonCodeV1::BlockedMaintained,
            )),
            _ => Err(ServiceNodeEligibilityPolicyError::UnsupportedLifecycleState),
        }
    }

    const fn candidate_transition(
        self,
        observation: &ServiceNodeEligibilityObservationV1,
        state_age_epochs: u64,
        failure_rate_bps: u16,
    ) -> EligibilityTransition {
        if self.candidate_to_probation.is_met_by(
            state_age_epochs,
            observation,
            failure_rate_bps,
            self.maximum_failure_rate_bps,
        ) {
            (
                ServiceNodeEligibilityStateV1::Probation,
                ServiceNodeEligibilityReasonCodeV1::CandidateEnteredProbation,
            )
        } else {
            (
                ServiceNodeEligibilityStateV1::Candidate,
                ServiceNodeEligibilityReasonCodeV1::CandidateHistoryPending,
            )
        }
    }

    const fn probation_transition(
        self,
        observation: &ServiceNodeEligibilityObservationV1,
        state_age_epochs: u64,
        failure_rate_bps: u16,
    ) -> EligibilityTransition {
        if failure_rate_bps > self.maximum_failure_rate_bps {
            return (
                ServiceNodeEligibilityStateV1::Degraded,
                ServiceNodeEligibilityReasonCodeV1::FailureRateDegraded,
            );
        }

        if self.probation_to_eligible.is_met_by(
            state_age_epochs,
            observation,
            failure_rate_bps,
            self.maximum_failure_rate_bps,
        ) {
            (
                ServiceNodeEligibilityStateV1::Eligible,
                ServiceNodeEligibilityReasonCodeV1::ProbationPromoted,
            )
        } else {
            (
                ServiceNodeEligibilityStateV1::Probation,
                ServiceNodeEligibilityReasonCodeV1::ProbationHistoryPending,
            )
        }
    }

    const fn eligible_transition(self, failure_rate_bps: u16) -> EligibilityTransition {
        if failure_rate_bps > self.maximum_failure_rate_bps {
            (
                ServiceNodeEligibilityStateV1::Degraded,
                ServiceNodeEligibilityReasonCodeV1::FailureRateDegraded,
            )
        } else {
            (
                ServiceNodeEligibilityStateV1::Eligible,
                ServiceNodeEligibilityReasonCodeV1::EligibleMaintained,
            )
        }
    }

    const fn degraded_transition(
        self,
        observation: &ServiceNodeEligibilityObservationV1,
        state_age_epochs: u64,
        failure_rate_bps: u16,
    ) -> EligibilityTransition {
        if observation.upheld_challenges_in_state == 0
            && self.probation_to_eligible.is_met_by(
                state_age_epochs,
                observation,
                failure_rate_bps,
                self.maximum_failure_rate_bps,
            )
        {
            (
                ServiceNodeEligibilityStateV1::Probation,
                ServiceNodeEligibilityReasonCodeV1::DegradedReturnedToProbation,
            )
        } else {
            (
                ServiceNodeEligibilityStateV1::Degraded,
                ServiceNodeEligibilityReasonCodeV1::DegradedRecoveryPending,
            )
        }
    }
}

/// Evidence-derived summary used for one lifecycle evaluation.
///
/// All counts describe the period beginning at the descriptor's current
/// `state_effective_epoch`. The roots bind this summary to the canonical
/// registry record. This DTO does not claim that the history is authenticated;
/// callers must obtain it from the reviewed accounting/audit path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeEligibilityObservationV1 {
    /// Service Node being evaluated.
    pub service_node_id: String,

    /// Epoch in which policy evaluation occurs.
    pub evaluation_epoch: u64,

    /// Service-history root observed by the evaluator.
    pub service_history_root: ContentId,

    /// Challenge-history root observed by the evaluator.
    pub challenge_history_root: ContentId,

    /// Successful useful-service events since the current state began.
    pub successful_service_events_in_state: u64,

    /// Failed useful-service events since the current state began.
    pub failed_service_events_in_state: u64,

    /// Distinct requester identities represented in current-state history.
    pub distinct_requesters_in_state: u64,

    /// Independent provider peers represented in corroborating current-state history.
    pub distinct_provider_peers_in_state: u64,

    /// Challenges objectively upheld since the current state began.
    pub upheld_challenges_in_state: u64,

    /// Challenges objectively rejected since the current state began.
    pub rejected_challenges_in_state: u64,

    /// Challenges still awaiting an objective outcome.
    pub pending_challenges_in_state: u64,
}

impl ServiceNodeEligibilityObservationV1 {
    fn validate_against(
        &self,
        descriptor: &ServiceNodeIdentityDescriptorV1,
    ) -> Result<(), ServiceNodeEligibilityPolicyError> {
        descriptor.validate()?;

        if self.service_node_id != descriptor.service_node_id {
            return Err(ServiceNodeEligibilityPolicyError::ServiceNodeIdMismatch {
                expected: descriptor.service_node_id.clone(),
                actual: self.service_node_id.clone(),
            });
        }

        if self.evaluation_epoch < descriptor.registered_at_epoch
            || self.evaluation_epoch < descriptor.state_effective_epoch
        {
            return Err(ServiceNodeEligibilityPolicyError::InvalidEvaluationEpoch {
                registered_at_epoch: descriptor.registered_at_epoch,
                state_effective_epoch: descriptor.state_effective_epoch,
                evaluation_epoch: self.evaluation_epoch,
            });
        }

        if self.service_history_root != descriptor.service_history_root {
            return Err(ServiceNodeEligibilityPolicyError::ServiceHistoryRootMismatch);
        }

        if self.challenge_history_root != descriptor.challenge_history_root {
            return Err(ServiceNodeEligibilityPolicyError::ChallengeHistoryRootMismatch);
        }

        Ok(())
    }

    fn failure_rate_bps(&self) -> u16 {
        let successful = u128::from(self.successful_service_events_in_state);
        let failed = u128::from(self.failed_service_events_in_state);
        let total = successful + failed;

        if total == 0 {
            return 0;
        }

        let rate = failed * u128::from(INTERNAL_ROC_BPS_DENOMINATOR) / total;

        u16::try_from(rate).unwrap_or(INTERNAL_ROC_BPS_DENOMINATOR)
    }
}

/// Reward-review posture implied by a lifecycle decision.
///
/// This is not a payout approval. Numeric probation caps remain owned by
/// reviewed economics configuration and are applied by later reward review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServiceNodeRewardReviewPostureV1 {
    /// Candidate, degraded, quarantined, and blocked nodes receive no reward approval.
    Denied,

    /// Probation rewards require the configured external probation cap.
    ProbationCapRequired,

    /// Eligible nodes may proceed to ordinary capped reward review.
    StandardReview,
}

impl ServiceNodeRewardReviewPostureV1 {
    /// Return the canonical low-cardinality label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Denied => "denied",
            Self::ProbationCapRequired => "probation_cap_required",
            Self::StandardReview => "standard_review",
        }
    }
}

/// Stable reason selected by deterministic lifecycle evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServiceNodeEligibilityReasonCodeV1 {
    /// Candidate has not yet met objective history requirements.
    CandidateHistoryPending,

    /// Candidate met objective requirements and entered probation.
    CandidateEnteredProbation,

    /// Probation has not yet met objective eligibility requirements.
    ProbationHistoryPending,

    /// Probation completed the objective eligibility gate.
    ProbationPromoted,

    /// Eligible posture remains valid.
    EligibleMaintained,

    /// Failed-service rate exceeded the reviewed bound.
    FailureRateDegraded,

    /// Upheld challenge history crossed the degradation threshold.
    ChallengeHistoryDegraded,

    /// Upheld challenge history crossed the quarantine threshold.
    ChallengeHistoryQuarantined,

    /// Upheld challenge history crossed the block threshold.
    ChallengeHistoryBlocked,

    /// Degraded state has not yet satisfied the recovery gate.
    DegradedRecoveryPending,

    /// Degraded state recovered only to probation, not directly to eligible.
    DegradedReturnedToProbation,

    /// Quarantine remains until a later authenticated recovery/appeal path exists.
    QuarantineMaintained,

    /// Blocked state remains terminal for this evaluator.
    BlockedMaintained,
}

impl ServiceNodeEligibilityReasonCodeV1 {
    /// Return the canonical low-cardinality label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CandidateHistoryPending => "candidate_history_pending",
            Self::CandidateEnteredProbation => "candidate_entered_probation",
            Self::ProbationHistoryPending => "probation_history_pending",
            Self::ProbationPromoted => "probation_promoted",
            Self::EligibleMaintained => "eligible_maintained",
            Self::FailureRateDegraded => "failure_rate_degraded",
            Self::ChallengeHistoryDegraded => "challenge_history_degraded",
            Self::ChallengeHistoryQuarantined => "challenge_history_quarantined",
            Self::ChallengeHistoryBlocked => "challenge_history_blocked",
            Self::DegradedRecoveryPending => "degraded_recovery_pending",
            Self::DegradedReturnedToProbation => "degraded_returned_to_probation",
            Self::QuarantineMaintained => "quarantine_maintained",
            Self::BlockedMaintained => "blocked_maintained",
        }
    }
}

/// Non-authoritative lifecycle decision returned by `ron-policy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeEligibilityDecisionV1 {
    /// Decision version.
    pub version: u16,

    /// Service Node bound to this decision.
    pub service_node_id: String,

    /// State present in the evaluated registry descriptor.
    pub previous_state: ServiceNodeEligibilityStateV1,

    /// State selected by objective policy evaluation.
    pub next_state: ServiceNodeEligibilityStateV1,

    /// Stable reason for the selected state.
    pub reason: ServiceNodeEligibilityReasonCodeV1,

    /// Epoch in which evaluation occurred.
    pub evaluation_epoch: u64,

    /// State-effective epoch to use if a later registry transition is approved.
    pub state_effective_epoch: u64,

    /// Deterministically calculated failed-service rate.
    pub observed_failure_rate_bps: u16,

    /// Reward-review posture implied by the selected state.
    pub reward_review: ServiceNodeRewardReviewPostureV1,
}

impl ServiceNodeEligibilityDecisionV1 {
    /// Whether policy selected a lifecycle transition.
    #[must_use]
    pub fn is_transition(&self) -> bool {
        self.previous_state != self.next_state
    }

    /// Whether the selected state may project into quorum eligibility.
    #[must_use]
    pub const fn counts_toward_quorum(&self) -> bool {
        self.next_state.counts_toward_quorum()
    }

    /// Whether reward review must apply a configured probation cap.
    #[must_use]
    pub const fn requires_probation_reward_cap(&self) -> bool {
        self.next_state.requires_probation_reward_cap()
    }

    /// Policy evaluation never directly authorizes registry mutation.
    #[must_use]
    pub const fn authorizes_registry_mutation(&self) -> bool {
        false
    }

    /// Policy evaluation never authorizes wallet, ledger, payout, or mint mutation.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }
}

/// Deterministic policy-evaluation failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeEligibilityPolicyError {
    /// The reviewed policy document was malformed or unsafe.
    #[error("invalid Service Node eligibility policy field {field}: {reason}")]
    InvalidPolicy {
        /// Invalid policy field or section.
        field: &'static str,

        /// Stable validation explanation.
        reason: &'static str,
    },

    /// The shared canonical descriptor was malformed.
    #[error(transparent)]
    InvalidDescriptor(#[from] ServiceNodeEligibilityValidationError),

    /// Observation targeted another Service Node identity.
    #[error("Service Node observation identity mismatch: expected {expected}, got {actual}")]
    ServiceNodeIdMismatch {
        /// Descriptor identity.
        expected: String,

        /// Observation identity.
        actual: String,
    },

    /// Observation epoch preceded registration or current-state activation.
    #[error(
        "invalid Service Node evaluation epoch {evaluation_epoch}: registration={registered_at_epoch}, state_effective={state_effective_epoch}"
    )]
    InvalidEvaluationEpoch {
        /// Registration epoch.
        registered_at_epoch: u64,

        /// Current state-effective epoch.
        state_effective_epoch: u64,

        /// Proposed evaluation epoch.
        evaluation_epoch: u64,
    },

    /// Observation did not match the descriptor's service-history commitment.
    #[error("Service Node service-history root mismatch")]
    ServiceHistoryRootMismatch,

    /// Observation did not match the descriptor's challenge-history commitment.
    #[error("Service Node challenge-history root mismatch")]
    ChallengeHistoryRootMismatch,

    /// A future lifecycle variant is not yet supported by this evaluator.
    #[error("unsupported Service Node lifecycle state")]
    UnsupportedLifecycleState,
}

const fn reward_review_for_state(
    state: ServiceNodeEligibilityStateV1,
) -> ServiceNodeRewardReviewPostureV1 {
    match state {
        ServiceNodeEligibilityStateV1::Probation => {
            ServiceNodeRewardReviewPostureV1::ProbationCapRequired
        }
        ServiceNodeEligibilityStateV1::Eligible => ServiceNodeRewardReviewPostureV1::StandardReview,
        _ => ServiceNodeRewardReviewPostureV1::Denied,
    }
}

const fn invalid_policy<T>(
    field: &'static str,
    reason: &'static str,
) -> Result<T, ServiceNodeEligibilityPolicyError> {
    Err(ServiceNodeEligibilityPolicyError::InvalidPolicy { field, reason })
}
