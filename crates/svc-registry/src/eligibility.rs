//! RO:WHAT — Registry custody for canonical Service Node eligibility descriptors and history roots.
//! RO:WHY — BUILD_PLAN_Z Phase 18 requires objective identity/history records before policy may evaluate lifecycle transitions.
//! RO:INTERACTS — ron-proto eligibility DTOs and svc-registry reward-recipient bindings.
//! RO:INVARIANTS — new records start candidate; reward binding must resolve exactly; history epochs and roots advance monotonically.
//! RO:CONFIG — no thresholds or reward caps are defined here; reviewed policy/economics configuration owns mutable values.
//! RO:SECURITY — no manual trust, wallet/ledger mutation, reward execution, signature verification, minting, or unilateral state promotion.
//! RO:TEST — tests/internal_roc_beta_phase18_eligibility_registry.rs.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use ron_policy::{
    ServiceNodeEligibilityDecisionV1, ServiceNodeEligibilityObservationV1,
    ServiceNodeEligibilityPolicyError, ServiceNodeEligibilityPolicyV1,
};
use ron_proto::{
    ContentId, RewardRecipientResolutionStateV1, ServiceNodeEligibilityStateV1,
    ServiceNodeEligibilityValidationError, ServiceNodeIdentityDescriptorV1,
};
use thiserror::Error;

use crate::rewards::RewardBindingRegistry;

/// One optimistic, epoch-bound update to objective Service Node history roots.
///
/// The update carries expected roots so stale or concurrently superseded
/// observations cannot silently replace newer registry history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNodeHistoryRootsUpdate {
    /// Service Node whose objective history changed.
    pub service_node_id: String,

    /// Last service-history root the updater observed.
    pub expected_service_history_root: ContentId,

    /// Replacement service-history root.
    pub service_history_root: ContentId,

    /// Last challenge-history root the updater observed.
    pub expected_challenge_history_root: ContentId,

    /// Replacement challenge-history root.
    pub challenge_history_root: ContentId,

    /// Epoch in which these replacement roots become observable.
    pub effective_epoch: u64,
}

/// Registry acceptance and history-custody failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeEligibilityRegistryError {
    /// The shared ron-proto descriptor was malformed.
    #[error("invalid Service Node eligibility descriptor: {0}")]
    InvalidDescriptor(#[from] ServiceNodeEligibilityValidationError),

    /// New registry identities must enter through candidate posture.
    #[error("new Service Node eligibility descriptor must start as candidate, got {actual:?}")]
    InitialStateMustBeCandidate {
        /// State supplied by the descriptor.
        actual: ServiceNodeEligibilityStateV1,
    },

    /// The Service Node identity already exists.
    #[error("Service Node eligibility descriptor already exists: {service_node_id}")]
    DuplicateServiceNodeId {
        /// Duplicate Service Node identity.
        service_node_id: String,
    },

    /// The registry entry is already assigned to another Service Node.
    #[error("Service Node registry entry already exists: {registry_entry_id}")]
    DuplicateRegistryEntryId {
        /// Duplicate registry entry.
        registry_entry_id: String,
    },

    /// No descriptor exists for the requested Service Node.
    #[error("Service Node eligibility descriptor not found: {service_node_id}")]
    DescriptorNotFound {
        /// Missing Service Node identity.
        service_node_id: String,
    },

    /// The reward-recipient binding was not active and resolvable.
    #[error("Service Node reward binding is not resolved for {service_node_id}: state={state:?}")]
    RewardBindingUnresolved {
        /// Service Node identity.
        service_node_id: String,

        /// Binding-resolution posture.
        state: RewardRecipientResolutionStateV1,
    },

    /// The descriptor referenced a different binding from registry truth.
    #[error(
        "Service Node reward binding mismatch for {service_node_id}: expected {expected_binding_id}, got {actual_binding_id:?}"
    )]
    RewardBindingMismatch {
        /// Service Node identity.
        service_node_id: String,

        /// Binding committed by the descriptor.
        expected_binding_id: String,

        /// Binding resolved by the reward-binding registry.
        actual_binding_id: Option<String>,
    },

    /// History updates must move to a strictly later epoch.
    #[error(
        "Service Node history epoch must advance for {service_node_id}: current={current_epoch}, proposed={proposed_epoch}"
    )]
    HistoryEpochRegression {
        /// Service Node identity.
        service_node_id: String,

        /// Most recent accepted history epoch.
        current_epoch: u64,

        /// Proposed replacement epoch.
        proposed_epoch: u64,
    },

    /// The updater supplied a stale service-history root.
    #[error("stale Service Node service-history root: {service_node_id}")]
    ServiceHistoryRootMismatch {
        /// Service Node identity.
        service_node_id: String,
    },

    /// The updater supplied a stale challenge-history root.
    #[error("stale Service Node challenge-history root: {service_node_id}")]
    ChallengeHistoryRootMismatch {
        /// Service Node identity.
        service_node_id: String,
    },

    /// At least one objective history root must change.
    #[error("Service Node history update did not change either history root: {service_node_id}")]
    HistoryRootsUnchanged {
        /// Service Node identity.
        service_node_id: String,
    },
}

/// Process-local Phase 18 registry model for eligibility descriptors.
///
/// This model proves deterministic registration, history-root custody, and
/// replay-protected application of policy-derived lifecycle transitions. It
/// does not yet wire durable storage, signed registry publication, fresh-node
/// weighting, rewards, wallet, or ledger.
#[derive(Debug, Clone, Default)]
pub struct ServiceNodeEligibilityRegistry {
    descriptors_by_node: BTreeMap<String, ServiceNodeIdentityDescriptorV1>,
    node_by_registry_entry: BTreeMap<String, String>,
    history_epoch_by_node: BTreeMap<String, u64>,
    last_policy_evaluation_epoch_by_node: BTreeMap<String, u64>,
}

impl ServiceNodeEligibilityRegistry {
    /// Create an empty Service Node eligibility registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            descriptors_by_node: BTreeMap::new(),
            node_by_registry_entry: BTreeMap::new(),
            history_epoch_by_node: BTreeMap::new(),
            last_policy_evaluation_epoch_by_node: BTreeMap::new(),
        }
    }

    /// Register one newly participating Service Node.
    ///
    /// Registration succeeds only when:
    ///
    /// - the canonical descriptor validates;
    /// - its initial state is `Candidate`;
    /// - its Service Node and registry-entry identities are unique;
    /// - its reward binding resolves as active for the registration epoch;
    /// - the resolved binding ID exactly matches the descriptor commitment.
    pub fn register_candidate(
        &mut self,
        descriptor: ServiceNodeIdentityDescriptorV1,
        reward_bindings: &RewardBindingRegistry,
        observed_at_ms: u64,
    ) -> Result<(), ServiceNodeEligibilityRegistryError> {
        descriptor.validate()?;

        if descriptor.state != ServiceNodeEligibilityStateV1::Candidate {
            return Err(
                ServiceNodeEligibilityRegistryError::InitialStateMustBeCandidate {
                    actual: descriptor.state,
                },
            );
        }

        if self
            .descriptors_by_node
            .contains_key(&descriptor.service_node_id)
        {
            return Err(
                ServiceNodeEligibilityRegistryError::DuplicateServiceNodeId {
                    service_node_id: descriptor.service_node_id,
                },
            );
        }

        if self
            .node_by_registry_entry
            .contains_key(&descriptor.registry_entry_id)
        {
            return Err(
                ServiceNodeEligibilityRegistryError::DuplicateRegistryEntryId {
                    registry_entry_id: descriptor.registry_entry_id,
                },
            );
        }

        let resolution = reward_bindings.resolve(
            &descriptor.service_node_id,
            descriptor.registered_at_epoch,
            observed_at_ms,
        );

        if resolution.state != RewardRecipientResolutionStateV1::Resolved {
            return Err(
                ServiceNodeEligibilityRegistryError::RewardBindingUnresolved {
                    service_node_id: descriptor.service_node_id,
                    state: resolution.state,
                },
            );
        }

        if resolution.binding_id.as_deref() != Some(descriptor.reward_binding_id.as_str()) {
            return Err(ServiceNodeEligibilityRegistryError::RewardBindingMismatch {
                service_node_id: descriptor.service_node_id,
                expected_binding_id: descriptor.reward_binding_id,
                actual_binding_id: resolution.binding_id,
            });
        }

        let service_node_id = descriptor.service_node_id.clone();
        let registry_entry_id = descriptor.registry_entry_id.clone();
        let registered_at_epoch = descriptor.registered_at_epoch;

        self.node_by_registry_entry
            .insert(registry_entry_id, service_node_id.clone());
        self.history_epoch_by_node
            .insert(service_node_id.clone(), registered_at_epoch);
        self.descriptors_by_node.insert(service_node_id, descriptor);

        Ok(())
    }

    /// Return one canonical descriptor.
    #[must_use]
    pub fn descriptor(&self, service_node_id: &str) -> Option<&ServiceNodeIdentityDescriptorV1> {
        self.descriptors_by_node.get(service_node_id)
    }

    /// Return the last accepted history-root epoch.
    #[must_use]
    pub fn history_effective_epoch(&self, service_node_id: &str) -> Option<u64> {
        self.history_epoch_by_node.get(service_node_id).copied()
    }

    /// Iterate over canonical descriptors in Service Node identity order.
    ///
    /// The backing `BTreeMap` makes this order deterministic. Consumers must
    /// still apply lifecycle and policy checks before treating a descriptor as
    /// quorum eligible.
    pub fn descriptors(&self) -> impl Iterator<Item = &ServiceNodeIdentityDescriptorV1> {
        self.descriptors_by_node.values()
    }

    /// Number of registered Service Node identities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.descriptors_by_node.len()
    }

    /// Whether no Service Node identities are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.descriptors_by_node.is_empty()
    }

    /// Replace objective history roots using optimistic root matching.
    ///
    /// This operation deliberately cannot change lifecycle state, key identity,
    /// registry entry, reward binding, registration epoch, wallet state, ledger
    /// state, rewards, quorum truth, or finality.
    pub fn update_history_roots(
        &mut self,
        update: ServiceNodeHistoryRootsUpdate,
    ) -> Result<ServiceNodeIdentityDescriptorV1, ServiceNodeEligibilityRegistryError> {
        let current_epoch = self
            .history_epoch_by_node
            .get(&update.service_node_id)
            .copied()
            .ok_or_else(|| ServiceNodeEligibilityRegistryError::DescriptorNotFound {
                service_node_id: update.service_node_id.clone(),
            })?;

        if update.effective_epoch <= current_epoch {
            return Err(
                ServiceNodeEligibilityRegistryError::HistoryEpochRegression {
                    service_node_id: update.service_node_id,
                    current_epoch,
                    proposed_epoch: update.effective_epoch,
                },
            );
        }

        let descriptor = self
            .descriptors_by_node
            .get_mut(&update.service_node_id)
            .ok_or_else(|| ServiceNodeEligibilityRegistryError::DescriptorNotFound {
                service_node_id: update.service_node_id.clone(),
            })?;

        if descriptor.service_history_root != update.expected_service_history_root {
            return Err(
                ServiceNodeEligibilityRegistryError::ServiceHistoryRootMismatch {
                    service_node_id: update.service_node_id,
                },
            );
        }

        if descriptor.challenge_history_root != update.expected_challenge_history_root {
            return Err(
                ServiceNodeEligibilityRegistryError::ChallengeHistoryRootMismatch {
                    service_node_id: update.service_node_id,
                },
            );
        }

        if descriptor.service_history_root == update.service_history_root
            && descriptor.challenge_history_root == update.challenge_history_root
        {
            return Err(ServiceNodeEligibilityRegistryError::HistoryRootsUnchanged {
                service_node_id: update.service_node_id,
            });
        }

        descriptor.service_history_root = update.service_history_root;
        descriptor.challenge_history_root = update.challenge_history_root;

        self.history_epoch_by_node
            .insert(update.service_node_id, update.effective_epoch);

        Ok(descriptor.clone())
    }
}

/// Root-bound lifecycle transition applied by `svc-registry`.
///
/// The embedded decision remains a declarative `ron-policy` artifact. The
/// returned descriptor is the canonical process-local registry record after
/// this registry independently checked and applied that decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNodeEligibilityTransitionV1 {
    /// Deterministic decision produced from current registry state.
    pub decision: ServiceNodeEligibilityDecisionV1,

    /// Canonical descriptor after transition application.
    pub descriptor: ServiceNodeIdentityDescriptorV1,

    /// Whether the lifecycle state changed.
    pub state_changed: bool,
}

impl ServiceNodeEligibilityTransitionV1 {
    /// Whether the resulting state may project into quorum eligibility.
    #[must_use]
    pub const fn counts_toward_quorum(&self) -> bool {
        self.descriptor.state.counts_toward_quorum()
    }

    /// Whether the resulting state requires the configured probation reward cap.
    #[must_use]
    pub const fn requires_probation_reward_cap(&self) -> bool {
        self.descriptor.state.requires_probation_reward_cap()
    }

    /// Registry transition application grants no economic mutation authority.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }
}

/// Failure while evaluating or applying a lifecycle policy review.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeEligibilityTransitionError {
    /// No canonical descriptor exists for the observed Service Node.
    #[error("Service Node eligibility descriptor not found: {service_node_id}")]
    DescriptorNotFound {
        /// Missing Service Node identity.
        service_node_id: String,
    },

    /// Registry history custody was missing for an existing descriptor.
    #[error("Service Node eligibility history epoch not found: {service_node_id}")]
    HistoryEpochNotFound {
        /// Service Node whose history epoch was missing.
        service_node_id: String,
    },

    /// Observation preceded the registry's latest committed history update.
    #[error(
        "Service Node policy evaluation epoch {evaluation_epoch} predates committed history epoch {history_epoch}"
    )]
    EvaluationPredatesHistory {
        /// Latest committed history epoch.
        history_epoch: u64,

        /// Proposed evaluation epoch.
        evaluation_epoch: u64,
    },

    /// The same or an older policy evaluation was already consumed.
    #[error(
        "Service Node policy evaluation replay rejected: evaluation_epoch={evaluation_epoch}, last_applied_epoch={last_applied_epoch}"
    )]
    PolicyEvaluationReplay {
        /// Proposed evaluation epoch.
        evaluation_epoch: u64,

        /// Latest consumed evaluation epoch.
        last_applied_epoch: u64,
    },

    /// A reviewed containment transition expected a different current state.
    #[error(
        "Service Node reviewed containment state mismatch: expected {expected:?}, got {actual:?}"
    )]
    ReviewedContainmentStateMismatch {
        /// State observed by the policy review.
        expected: ServiceNodeEligibilityStateV1,

        /// State currently held by registry custody.
        actual: ServiceNodeEligibilityStateV1,
    },

    /// A reviewed containment transition selected a non-containment state.
    #[error("Service Node reviewed containment selected invalid state: {actual:?}")]
    InvalidReviewedContainmentState {
        /// Invalid selected state.
        actual: ServiceNodeEligibilityStateV1,
    },

    /// A policy result contained an inconsistent state-effective epoch.
    #[error("Service Node policy decision has an invalid state-effective epoch")]
    InvalidDecisionStateEpoch,

    /// The resulting canonical descriptor was invalid.
    #[error(transparent)]
    InvalidDescriptor(#[from] ServiceNodeEligibilityValidationError),

    /// Deterministic policy evaluation rejected its inputs.
    #[error(transparent)]
    Policy(#[from] ServiceNodeEligibilityPolicyError),
}

impl ServiceNodeEligibilityRegistry {
    /// Evaluate and atomically apply one root-bound lifecycle review.
    ///
    /// The registry:
    ///
    /// - reads its own canonical descriptor;
    /// - checks the observation against committed history timing;
    /// - rejects repeated or out-of-order evaluation epochs;
    /// - obtains the lifecycle decision directly from `ron-policy`;
    /// - validates the resulting descriptor before replacing registry state.
    ///
    /// No-op reviews are consumed as well. Repeating a candidate review at the
    /// same epoch therefore rejects as replay rather than masquerading as new
    /// objective work.
    ///
    /// This operation does not execute rewards, change balances, write ledger
    /// receipts, create quorum signatures, or grant finality.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceNodeEligibilityTransitionError`] when registry state is
    /// missing, observation timing is stale, an evaluation is replayed, policy
    /// evaluation fails, or the resulting descriptor is invalid.
    pub fn evaluate_and_apply_policy(
        &mut self,
        policy: ServiceNodeEligibilityPolicyV1,
        observation: &ServiceNodeEligibilityObservationV1,
    ) -> Result<ServiceNodeEligibilityTransitionV1, ServiceNodeEligibilityTransitionError> {
        let descriptor = self
            .descriptors_by_node
            .get(&observation.service_node_id)
            .cloned()
            .ok_or_else(
                || ServiceNodeEligibilityTransitionError::DescriptorNotFound {
                    service_node_id: observation.service_node_id.clone(),
                },
            )?;

        let history_epoch = self
            .history_epoch_by_node
            .get(&observation.service_node_id)
            .copied()
            .ok_or_else(
                || ServiceNodeEligibilityTransitionError::HistoryEpochNotFound {
                    service_node_id: observation.service_node_id.clone(),
                },
            )?;

        if observation.evaluation_epoch < history_epoch {
            return Err(
                ServiceNodeEligibilityTransitionError::EvaluationPredatesHistory {
                    history_epoch,
                    evaluation_epoch: observation.evaluation_epoch,
                },
            );
        }

        if let Some(last_applied_epoch) = self
            .last_policy_evaluation_epoch_by_node
            .get(&observation.service_node_id)
            .copied()
        {
            if observation.evaluation_epoch <= last_applied_epoch {
                return Err(
                    ServiceNodeEligibilityTransitionError::PolicyEvaluationReplay {
                        evaluation_epoch: observation.evaluation_epoch,
                        last_applied_epoch,
                    },
                );
            }
        }

        let decision = policy.evaluate(&descriptor, observation)?;
        let state_changed = decision.previous_state != decision.next_state;

        if state_changed {
            if decision.state_effective_epoch != decision.evaluation_epoch {
                return Err(ServiceNodeEligibilityTransitionError::InvalidDecisionStateEpoch);
            }
        } else if decision.state_effective_epoch != descriptor.state_effective_epoch {
            return Err(ServiceNodeEligibilityTransitionError::InvalidDecisionStateEpoch);
        }

        let mut updated = descriptor;
        updated.state = decision.next_state;
        updated.state_effective_epoch = decision.state_effective_epoch;
        updated.validate()?;

        self.descriptors_by_node
            .insert(observation.service_node_id.clone(), updated.clone());

        self.last_policy_evaluation_epoch_by_node.insert(
            observation.service_node_id.clone(),
            observation.evaluation_epoch,
        );

        Ok(ServiceNodeEligibilityTransitionV1 {
            decision,
            descriptor: updated,
            state_changed,
        })
    }

    /// Apply a containment state already derived by reviewed policy.
    ///
    /// This crate-private bridge prevents sibling registry modules from
    /// replacing complete descriptors or inventing a second lifecycle model.
    /// Only `Degraded`, `Quarantined`, or `Blocked` may be applied.
    pub(crate) fn apply_reviewed_containment(
        &mut self,
        service_node_id: &str,
        expected_state: ServiceNodeEligibilityStateV1,
        next_state: ServiceNodeEligibilityStateV1,
        state_effective_epoch: u64,
    ) -> Result<ServiceNodeIdentityDescriptorV1, ServiceNodeEligibilityTransitionError> {
        if !matches!(
            next_state,
            ServiceNodeEligibilityStateV1::Degraded
                | ServiceNodeEligibilityStateV1::Quarantined
                | ServiceNodeEligibilityStateV1::Blocked
        ) {
            return Err(
                ServiceNodeEligibilityTransitionError::InvalidReviewedContainmentState {
                    actual: next_state,
                },
            );
        }

        let current = self
            .descriptors_by_node
            .get(service_node_id)
            .cloned()
            .ok_or_else(
                || ServiceNodeEligibilityTransitionError::DescriptorNotFound {
                    service_node_id: service_node_id.to_string(),
                },
            )?;

        if current.state != expected_state {
            return Err(
                ServiceNodeEligibilityTransitionError::ReviewedContainmentStateMismatch {
                    expected: expected_state,
                    actual: current.state,
                },
            );
        }

        if next_state == current.state {
            if state_effective_epoch != current.state_effective_epoch {
                return Err(ServiceNodeEligibilityTransitionError::InvalidDecisionStateEpoch);
            }
        } else if state_effective_epoch < current.state_effective_epoch {
            return Err(ServiceNodeEligibilityTransitionError::InvalidDecisionStateEpoch);
        }

        let mut updated = current;
        updated.state = next_state;
        updated.state_effective_epoch = state_effective_epoch;
        updated.validate()?;

        self.descriptors_by_node
            .insert(service_node_id.to_string(), updated.clone());

        Ok(updated)
    }

    /// Latest lifecycle-policy evaluation epoch consumed for one Service Node.
    #[must_use]
    pub fn last_policy_evaluation_epoch(&self, service_node_id: &str) -> Option<u64> {
        self.last_policy_evaluation_epoch_by_node
            .get(service_node_id)
            .copied()
    }
}
