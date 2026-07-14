//! RO:WHAT — Canonical Phase 18 Service Node identity and protocol-earned eligibility lifecycle DTOs.
//! RO:WHY — ECON/GOV: keep Sybil resistance state objective, strict, and reusable by registry, policy, quorum, and reward review.
//! RO:INTERACTS — service_node::quorum, svc-registry, ron-policy, svc-rewarder, ron-accounting, ron-audit.
//! RO:INVARIANTS — new descriptors start candidate; only eligible counts toward quorum; probation requires an external configured reward cap.
//! RO:CONFIG — cap values and promotion thresholds are supplied by reviewed policy/economics configuration, never hard-coded here.
//! RO:SECURITY — no founder/manual trust flag, wallet/ledger mutation, signature verification, or unilateral mint authority.
//! RO:TEST — tests/internal_roc_beta_phase18_service_node_eligibility.rs.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{EpochEligibilityStatusV1, EpochEligibilityV1};
use crate::id::ContentId;

/// Current version for Phase 18 Service Node eligibility DTOs.
pub const SERVICE_NODE_ELIGIBILITY_VERSION: u16 = 1;

/// Canonical schema for one Service Node identity and eligibility descriptor.
pub const SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA: &str = "ron.service_node.identity-descriptor.v1";

const MAX_REF_BYTES: usize = 256;

/// Structural validation failures for Phase 18 Service Node eligibility artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeEligibilityValidationError {
    /// A DTO used an unsupported schema.
    #[error("invalid Service Node eligibility schema: expected {expected}, got {actual}")]
    InvalidSchema {
        /// Required schema.
        expected: &'static str,
        /// Supplied schema.
        actual: String,
    },

    /// A DTO used an unsupported version.
    #[error("invalid Service Node eligibility version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// Required version.
        expected: u16,
        /// Supplied version.
        actual: u16,
    },

    /// A field failed bounded or semantic validation.
    #[error("invalid Service Node eligibility field {field}: {reason}")]
    InvalidField {
        /// Invalid field.
        field: &'static str,
        /// Stable failure explanation.
        reason: &'static str,
    },
}

/// Canonical protocol-earned lifecycle state for a Service Node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServiceNodeEligibilityStateV1 {
    /// Newly registered identity accumulating objective service history.
    Candidate,

    /// Limited-participation identity whose rewards require a configured cap.
    Probation,

    /// Identity permitted to count toward quorum under reviewed threshold rules.
    Eligible,

    /// Recoverable identity that does not currently count toward quorum.
    Degraded,

    /// Isolated identity that does not count toward quorum.
    Quarantined,

    /// Identity denied protocol participation.
    Blocked,
}

impl ServiceNodeEligibilityStateV1 {
    /// Return whether this lifecycle state may count toward an epoch quorum.
    #[must_use]
    pub const fn counts_toward_quorum(self) -> bool {
        matches!(self, Self::Eligible)
    }

    /// Return whether reward review must apply the configured probation cap.
    ///
    /// This method does not define the cap amount. Mutable cap values remain
    /// owned by reviewed economics/policy configuration.
    #[must_use]
    pub const fn requires_probation_reward_cap(self) -> bool {
        matches!(self, Self::Probation)
    }

    /// Project this lifecycle state into the existing epoch quorum status.
    #[must_use]
    pub const fn epoch_quorum_status(self) -> EpochEligibilityStatusV1 {
        if self.counts_toward_quorum() {
            EpochEligibilityStatusV1::Eligible
        } else {
            EpochEligibilityStatusV1::Ineligible
        }
    }
}

/// Registry identity and objective history roots for one Service Node.
///
/// This descriptor is strict shape/state data only. Registry and policy layers
/// must verify the referenced binding and history material before accepting a
/// state change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeIdentityDescriptorV1 {
    /// Stable wire schema.
    pub schema: String,

    /// DTO version.
    pub version: u16,

    /// Canonical Service Node identity.
    pub service_node_id: String,

    /// Registry entry that owns this descriptor.
    pub registry_entry_id: String,

    /// Reward-recipient binding committed for this node.
    pub reward_binding_id: String,

    /// Public verification key identity expected for quorum signatures.
    pub key_id: String,

    /// First epoch in which this identity was registered.
    pub registered_at_epoch: u64,

    /// Current protocol-earned lifecycle state.
    pub state: ServiceNodeEligibilityStateV1,

    /// First epoch in which the current state applies.
    pub state_effective_epoch: u64,

    /// Canonical root of objective useful-service history.
    pub service_history_root: ContentId,

    /// Canonical root of challenge and fraud-review history.
    pub challenge_history_root: ContentId,
}

impl ServiceNodeIdentityDescriptorV1 {
    /// Construct and validate a newly registered candidate descriptor.
    ///
    /// Promotion is deliberately absent from this constructor. Later policy
    /// evaluation must derive any state change from objective history.
    pub fn new_candidate(
        service_node_id: String,
        registry_entry_id: String,
        reward_binding_id: String,
        key_id: String,
        registered_at_epoch: u64,
        service_history_root: ContentId,
        challenge_history_root: ContentId,
    ) -> Result<Self, ServiceNodeEligibilityValidationError> {
        let descriptor = Self {
            schema: SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA.to_string(),
            version: SERVICE_NODE_ELIGIBILITY_VERSION,
            service_node_id,
            registry_entry_id,
            reward_binding_id,
            key_id,
            registered_at_epoch,
            state: ServiceNodeEligibilityStateV1::Candidate,
            state_effective_epoch: registered_at_epoch,
            service_history_root,
            challenge_history_root,
        };

        descriptor.validate()?;
        Ok(descriptor)
    }

    /// Validate strict identity, binding, epoch, and lifecycle shape.
    pub fn validate(&self) -> Result<(), ServiceNodeEligibilityValidationError> {
        if self.schema != SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA {
            return Err(ServiceNodeEligibilityValidationError::InvalidSchema {
                expected: SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_NODE_ELIGIBILITY_VERSION {
            return Err(ServiceNodeEligibilityValidationError::InvalidVersion {
                expected: SERVICE_NODE_ELIGIBILITY_VERSION,
                actual: self.version,
            });
        }

        validate_service_node_id(&self.service_node_id)?;
        validate_token("registry_entry_id", &self.registry_entry_id)?;
        validate_token("reward_binding_id", &self.reward_binding_id)?;
        validate_token("key_id", &self.key_id)?;

        if self.registered_at_epoch == 0 {
            return invalid("registered_at_epoch", "must be greater than zero");
        }

        if self.state_effective_epoch < self.registered_at_epoch {
            return invalid(
                "state_effective_epoch",
                "must not precede registered_at_epoch",
            );
        }

        Ok(())
    }

    /// Project this descriptor into the existing Phase 15 epoch quorum member.
    ///
    /// The projection preserves the canonical identity, registry, reward
    /// binding, and key references. It does not verify signatures or grant
    /// wallet, ledger, payout, mint, or finality authority.
    pub fn to_epoch_eligibility(
        &self,
    ) -> Result<EpochEligibilityV1, ServiceNodeEligibilityValidationError> {
        self.validate()?;

        Ok(EpochEligibilityV1 {
            version: super::SERVICE_NODE_QUORUM_VERSION,
            service_node_id: self.service_node_id.clone(),
            registry_entry_id: self.registry_entry_id.clone(),
            reward_binding_id: self.reward_binding_id.clone(),
            key_id: self.key_id.clone(),
            status: self.state.epoch_quorum_status(),
        })
    }
}

fn validate_service_node_id(value: &str) -> Result<(), ServiceNodeEligibilityValidationError> {
    validate_token("service_node_id", value)?;

    let Some(suffix) = value.strip_prefix("service_node:") else {
        return invalid("service_node_id", "must use service_node:<id> form");
    };

    if suffix.is_empty() {
        return invalid("service_node_id", "must use service_node:<id> form");
    }

    Ok(())
}

fn validate_token(
    field: &'static str,
    value: &str,
) -> Result<(), ServiceNodeEligibilityValidationError> {
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

fn invalid<T>(
    field: &'static str,
    reason: &'static str,
) -> Result<T, ServiceNodeEligibilityValidationError> {
    Err(ServiceNodeEligibilityValidationError::InvalidField { field, reason })
}
