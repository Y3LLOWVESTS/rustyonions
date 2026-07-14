//! RO:WHAT — Process-local custody of canonical Service Node lifecycle status.
//! RO:WHY — The optional operator console needs quorum, containment, appeal,
//!          and effective-epoch truth without creating another state machine.
//! RO:INTERACTS — Phase 18 identity descriptors and Phase 19 enforcement DTOs
//!                from ron-proto.
//! RO:INVARIANTS —
//!   - Canonical ron-proto DTOs remain the only lifecycle source.
//!   - Contained descriptors require matching enforcement status.
//!   - Non-contained descriptors reject enforcement status.
//!   - Node identity, lifecycle state, and effective epoch must match.
//!   - This store grants no registry, appeal-resolution, reward, wallet,
//!     ledger, receipt, mint, burn, or finality authority.
//!
//! RO:TEST — Unit tests below preserve quorum and pending-appeal truth.

#![forbid(unsafe_code)]

use std::{error::Error, fmt, sync::Mutex};

use ron_proto::{
    ServiceNodeEligibilityStateV1, ServiceNodeEnforcementStatusV1, ServiceNodeIdentityDescriptorV1,
};

/// One validated canonical lifecycle snapshot safe for operator projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNodeLifecycleSnapshot {
    /// Canonical Phase 18 identity and lifecycle descriptor.
    pub descriptor: ServiceNodeIdentityDescriptorV1,

    /// Canonical Phase 19 containment and appeal status, when contained.
    pub enforcement: Option<ServiceNodeEnforcementStatusV1>,
}

/// Rejection while replacing the process-local lifecycle snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ServiceNodeLifecycleStatusError {
    /// The canonical descriptor failed its own validation.
    InvalidDescriptor(String),

    /// The canonical enforcement status failed its own validation.
    InvalidEnforcement(String),

    /// Degraded, quarantined, or blocked state lacked enforcement status.
    ContainedStateMissingEnforcement,

    /// Candidate, probation, or eligible state carried enforcement status.
    UnexpectedEnforcementStatus,

    /// Descriptor and enforcement status identify different nodes.
    ServiceNodeMismatch {
        descriptor_node: String,
        enforcement_node: String,
    },

    /// Descriptor and enforcement status disagree about lifecycle state.
    StateMismatch {
        descriptor_state: ServiceNodeEligibilityStateV1,
        enforcement_state: ServiceNodeEligibilityStateV1,
    },

    /// Descriptor and enforcement status disagree about effective epoch.
    EffectiveEpochMismatch {
        descriptor_epoch: u64,
        enforcement_epoch: u64,
    },
}

impl fmt::Display for ServiceNodeLifecycleStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(reason) => {
                write!(formatter, "invalid Service Node descriptor: {reason}")
            }
            Self::InvalidEnforcement(reason) => {
                write!(
                    formatter,
                    "invalid Service Node enforcement status: {reason}"
                )
            }
            Self::ContainedStateMissingEnforcement => write!(
                formatter,
                "contained lifecycle state requires canonical enforcement status"
            ),
            Self::UnexpectedEnforcementStatus => write!(
                formatter,
                "non-contained lifecycle state must not carry enforcement status"
            ),
            Self::ServiceNodeMismatch {
                descriptor_node,
                enforcement_node,
            } => write!(
                formatter,
                "Service Node lifecycle identity mismatch: descriptor={descriptor_node}, enforcement={enforcement_node}"
            ),
            Self::StateMismatch {
                descriptor_state,
                enforcement_state,
            } => write!(
                formatter,
                "Service Node lifecycle state mismatch: descriptor={descriptor_state:?}, enforcement={enforcement_state:?}"
            ),
            Self::EffectiveEpochMismatch {
                descriptor_epoch,
                enforcement_epoch,
            } => write!(
                formatter,
                "Service Node lifecycle effective-epoch mismatch: descriptor={descriptor_epoch}, enforcement={enforcement_epoch}"
            ),
        }
    }
}

impl Error for ServiceNodeLifecycleStatusError {}

/// Validated process-local lifecycle snapshot.
///
/// The mutex guards one small cloneable status object. It never guards
/// registry storage, network calls, policy evaluation, or economic mutation.
#[derive(Debug, Default)]
pub struct ServiceNodeLifecycleStatusStore {
    current: Mutex<Option<ServiceNodeLifecycleSnapshot>>,
}

impl ServiceNodeLifecycleStatusStore {
    /// Replace the current snapshot after validating canonical relationships.
    #[allow(dead_code)]
    // Called only by a future canonical registry-status feeder. Keeping this
    // dormant is preferable to manufacturing lifecycle state at startup.
    pub fn replace(
        &self,
        descriptor: ServiceNodeIdentityDescriptorV1,
        enforcement: Option<ServiceNodeEnforcementStatusV1>,
    ) -> Result<(), ServiceNodeLifecycleStatusError> {
        descriptor.validate().map_err(|error| {
            ServiceNodeLifecycleStatusError::InvalidDescriptor(error.to_string())
        })?;

        if let Some(status) = enforcement.as_ref() {
            status.validate().map_err(|error| {
                ServiceNodeLifecycleStatusError::InvalidEnforcement(error.to_string())
            })?;

            if status.service_node_id != descriptor.service_node_id {
                return Err(ServiceNodeLifecycleStatusError::ServiceNodeMismatch {
                    descriptor_node: descriptor.service_node_id.clone(),
                    enforcement_node: status.service_node_id.clone(),
                });
            }

            if status.state != descriptor.state {
                return Err(ServiceNodeLifecycleStatusError::StateMismatch {
                    descriptor_state: descriptor.state,
                    enforcement_state: status.state,
                });
            }

            if status.effective_epoch != descriptor.state_effective_epoch {
                return Err(ServiceNodeLifecycleStatusError::EffectiveEpochMismatch {
                    descriptor_epoch: descriptor.state_effective_epoch,
                    enforcement_epoch: status.effective_epoch,
                });
            }
        }

        let contained = matches!(
            descriptor.state,
            ServiceNodeEligibilityStateV1::Degraded
                | ServiceNodeEligibilityStateV1::Quarantined
                | ServiceNodeEligibilityStateV1::Blocked
        );

        match (contained, enforcement.is_some()) {
            (true, false) => {
                return Err(ServiceNodeLifecycleStatusError::ContainedStateMissingEnforcement);
            }
            (false, true) => {
                return Err(ServiceNodeLifecycleStatusError::UnexpectedEnforcementStatus);
            }
            _ => {}
        }

        *self
            .current
            .lock()
            .expect("Service Node lifecycle status mutex poisoned") =
            Some(ServiceNodeLifecycleSnapshot {
                descriptor,
                enforcement,
            });

        Ok(())
    }

    /// Remove process-local status without changing canonical registry state.
    #[allow(dead_code)]
    // Clears only a future canonical projection cache; it grants no state
    // transition or appeal-resolution authority.
    pub fn clear(&self) {
        *self
            .current
            .lock()
            .expect("Service Node lifecycle status mutex poisoned") = None;
    }

    /// Clone the current validated snapshot for read-only projection.
    #[must_use]
    pub fn snapshot(&self) -> Option<ServiceNodeLifecycleSnapshot> {
        self.current
            .lock()
            .expect("Service Node lifecycle status mutex poisoned")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use ron_proto::{
        ContentId, ServiceNodeAppealStateV1, ServiceNodeAppealStatusV1,
        ServiceNodeEligibilityStateV1, ServiceNodeEnforcementStatusV1,
        ServiceNodeIdentityDescriptorV1, ServiceNodeViolationKindV1,
        SERVICE_NODE_ELIGIBILITY_VERSION, SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA,
        SERVICE_NODE_ENFORCEMENT_VERSION, SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA,
    };

    use super::ServiceNodeLifecycleStatusStore;

    fn cid(label: &str) -> ContentId {
        let digest = match label {
            "service-history" => "1111111111111111111111111111111111111111111111111111111111111111",
            "challenge-history" => {
                "2222222222222222222222222222222222222222222222222222222222222222"
            }
            "containment-evidence" => {
                "3333333333333333333333333333333333333333333333333333333333333333"
            }
            other => panic!("unknown lifecycle fixture label: {other}"),
        };

        format!("b3:{digest}")
            .parse()
            .expect("fixture content ID should parse")
    }

    fn descriptor(
        state: ServiceNodeEligibilityStateV1,
        effective_epoch: u64,
    ) -> ServiceNodeIdentityDescriptorV1 {
        ServiceNodeIdentityDescriptorV1 {
            schema: SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA.to_string(),
            version: SERVICE_NODE_ELIGIBILITY_VERSION,
            service_node_id: "service_node:operator_01".to_string(),
            registry_entry_id: "registry:operator_01".to_string(),
            reward_binding_id: "binding:operator_01".to_string(),
            key_id: "key:operator_01".to_string(),
            registered_at_epoch: 10,
            state,
            state_effective_epoch: effective_epoch,
            service_history_root: cid("service-history"),
            challenge_history_root: cid("challenge-history"),
        }
    }

    fn pending_quarantine(effective_epoch: u64) -> ServiceNodeEnforcementStatusV1 {
        ServiceNodeEnforcementStatusV1 {
            schema: SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA.to_string(),
            version: SERVICE_NODE_ENFORCEMENT_VERSION,
            status_id: "enforcement:operator_01".to_string(),
            service_node_id: "service_node:operator_01".to_string(),
            state: ServiceNodeEligibilityStateV1::Quarantined,
            reason: ServiceNodeViolationKindV1::HashMismatch,
            evidence_root: cid("containment-evidence"),
            effective_epoch,
            appeal: ServiceNodeAppealStatusV1 {
                state: ServiceNodeAppealStateV1::Pending,
                appeal_id: Some("appeal:operator_01".to_string()),
                submitted_epoch: Some(effective_epoch + 1),
                resolved_epoch: None,
                resolution_evidence_root: None,
            },
        }
    }

    #[test]
    fn canonical_service_node_lifecycle_store_preserves_quorum_and_pending_appeal() {
        let store = ServiceNodeLifecycleStatusStore::default();

        store
            .replace(
                descriptor(ServiceNodeEligibilityStateV1::Eligible, 15),
                None,
            )
            .expect("eligible lifecycle snapshot should validate");

        let eligible = store.snapshot().expect("eligible snapshot");
        assert!(eligible.descriptor.state.counts_toward_quorum());
        assert!(eligible.enforcement.is_none());

        store
            .replace(
                descriptor(ServiceNodeEligibilityStateV1::Quarantined, 20),
                Some(pending_quarantine(20)),
            )
            .expect("quarantine lifecycle snapshot should validate");

        let quarantined = store.snapshot().expect("quarantine snapshot");
        assert!(!quarantined.descriptor.state.counts_toward_quorum());

        let enforcement = quarantined
            .enforcement
            .expect("quarantine requires enforcement status");

        assert!(!enforcement.counts_toward_quorum());
        assert!(!enforcement.permits_reward_planning());
        assert!(enforcement.appeal.is_pending());
        assert!(!enforcement.appeal.authorizes_state_change());
        assert!(!enforcement.authorizes_economic_mutation());
    }
}
