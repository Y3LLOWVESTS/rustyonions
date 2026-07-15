//! RO:WHAT — Local service-node reward-recipient binding registry model.
//! RO:WHY — BUILD_PLAN_Z Phase 5 requires registry resolution from service_node_id to bound ROC recipient.
//! RO:INTERACTS — ron-proto reward binding DTOs, future svc-passport/ron-auth signing, future wallet/ledger payout.
//! RO:INVARIANTS — no wallet mutation, no ledger mutation, no payout execution, no arbitrary evidence payout override.
//! RO:SECURITY — service evidence must resolve through this registry-style binding, not carry its own payout address.

pub mod issuance_guard;
pub mod request_intake;

use std::collections::{BTreeMap, BTreeSet};

use ron_proto::{
    ContentId, NodeRewardRecipientStateV1, NodeRewardRecipientStatusV1,
    RewardRecipientResolutionStateV1, RewardRecipientResolutionV1,
    ServiceNodeRewardBindingRotationV1, ServiceNodeRewardBindingV1,
    ServiceNodeRewardBindingValidationError, NODE_REWARD_RECIPIENT_STATUS_VERSION,
    REWARD_RECIPIENT_RESOLUTION_VERSION,
};
use thiserror::Error;

pub use issuance_guard::{
    preview_reward_payout_authorization, RewardPayoutAuthorizationError,
    RewardPayoutAuthorizationPreview, SelfIssuanceMode,
};

pub use request_intake::{
    RegistryRewardBindingRequestIntake, RegistryRewardBindingRequestReceipt,
    RegistryRewardBindingRequestV1, RegistryRewardBindingRotationRequestV1,
    ResolvedRewardRecipientForBinding, RewardBindingRequestError,
};

/// Local reward-binding registry errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RewardBindingRegistryError {
    /// The DTO itself failed strict ron-proto validation.
    #[error("invalid reward binding DTO: {0}")]
    InvalidBinding(#[from] ServiceNodeRewardBindingValidationError),

    /// A binding id was already present.
    #[error("duplicate reward binding id: {binding_id}")]
    DuplicateBindingId {
        /// Duplicate binding id.
        binding_id: String,
    },

    /// A rotation nonce was already used.
    #[error("duplicate reward binding rotation nonce: {rotation_nonce}")]
    DuplicateRotationNonce {
        /// Duplicate rotation nonce.
        rotation_nonce: String,
    },

    /// A service node already has a pending recipient rotation.
    #[error(
        "service node already has a pending reward binding rotation:          {service_node_id}"
    )]
    PendingRotationAlreadyScheduled {
        /// Service node whose existing pending rotation remains authoritative.
        service_node_id: String,
    },

    /// A service node already has a current binding in this local registry.
    #[error("service node already has a reward binding: {service_node_id}")]
    ServiceNodeAlreadyBound {
        /// Service node id.
        service_node_id: String,
    },

    /// A rotation referenced a missing binding.
    #[error("reward binding not found: {binding_id}")]
    BindingNotFound {
        /// Binding id.
        binding_id: String,
    },

    /// A rotation did not match the service node on the current binding.
    #[error("reward binding service node mismatch")]
    ServiceNodeMismatch,
}

/// In-memory reward binding model used by focused Phase 5 tests.
///
/// This is not yet the durable registry store. It proves the acceptance and
/// resolution rules before wiring persistence, HTTP, passport/auth signatures,
/// quorum roots, wallet execution, or ledger receipts.
#[derive(Debug, Clone)]
pub struct RewardBindingRegistry {
    registry_root: ContentId,
    reward_binding_root: ContentId,
    bindings_by_id: BTreeMap<String, ServiceNodeRewardBindingV1>,
    current_binding_by_node: BTreeMap<String, String>,
    pending_rotation_by_node: BTreeMap<String, ServiceNodeRewardBindingRotationV1>,
    used_rotation_nonces: BTreeSet<String>,
}

impl RewardBindingRegistry {
    /// Create an empty local reward-binding registry model.
    #[must_use]
    pub fn new(registry_root: ContentId, reward_binding_root: ContentId) -> Self {
        Self {
            registry_root,
            reward_binding_root,
            bindings_by_id: BTreeMap::new(),
            current_binding_by_node: BTreeMap::new(),
            pending_rotation_by_node: BTreeMap::new(),
            used_rotation_nonces: BTreeSet::new(),
        }
    }

    /// Insert a validated binding as the current binding for its service node.
    ///
    /// The registry rejects duplicate binding ids, duplicate rotation nonces,
    /// and second current bindings for the same service node.
    pub fn insert_binding(
        &mut self,
        binding: ServiceNodeRewardBindingV1,
    ) -> Result<(), RewardBindingRegistryError> {
        binding.validate()?;

        if self.bindings_by_id.contains_key(&binding.binding_id) {
            return Err(RewardBindingRegistryError::DuplicateBindingId {
                binding_id: binding.binding_id,
            });
        }

        if self
            .current_binding_by_node
            .contains_key(&binding.service_node_id)
        {
            return Err(RewardBindingRegistryError::ServiceNodeAlreadyBound {
                service_node_id: binding.service_node_id,
            });
        }

        if !self
            .used_rotation_nonces
            .insert(binding.rotation_nonce.clone())
        {
            return Err(RewardBindingRegistryError::DuplicateRotationNonce {
                rotation_nonce: binding.rotation_nonce,
            });
        }

        self.current_binding_by_node
            .insert(binding.service_node_id.clone(), binding.binding_id.clone());
        self.bindings_by_id
            .insert(binding.binding_id.clone(), binding);

        Ok(())
    }

    /// Schedule a future reward-recipient rotation for a currently bound node.
    ///
    /// This does not apply the rotation or mutate wallet/ledger state. It only
    /// proves the local registry can hold a validated, nonce-unique pending
    /// rotation that status surfaces can report truthfully.
    pub fn schedule_rotation(
        &mut self,
        rotation: ServiceNodeRewardBindingRotationV1,
    ) -> Result<(), RewardBindingRegistryError> {
        rotation.validate()?;

        let Some(current_binding) = self.bindings_by_id.get(&rotation.old_binding_id) else {
            return Err(RewardBindingRegistryError::BindingNotFound {
                binding_id: rotation.old_binding_id,
            });
        };

        if current_binding.service_node_id != rotation.service_node_id {
            return Err(RewardBindingRegistryError::ServiceNodeMismatch);
        }

        if self.used_rotation_nonces.contains(&rotation.rotation_nonce) {
            return Err(RewardBindingRegistryError::DuplicateRotationNonce {
                rotation_nonce: rotation.rotation_nonce,
            });
        }

        if self
            .pending_rotation_by_node
            .contains_key(&rotation.service_node_id)
        {
            return Err(
                RewardBindingRegistryError::PendingRotationAlreadyScheduled {
                    service_node_id: rotation.service_node_id,
                },
            );
        }

        let nonce_inserted = self
            .used_rotation_nonces
            .insert(rotation.rotation_nonce.clone());

        debug_assert!(
            nonce_inserted,
            "rotation nonce was checked before insertion",
        );

        self.pending_rotation_by_node
            .insert(rotation.service_node_id.clone(), rotation);

        Ok(())
    }

    /// Apply a pending reward-recipient rotation once its effective epoch has arrived.
    ///
    /// This creates a new current binding from the signed rotation and keeps the old
    /// binding in history. It still does not execute rewards, mutate wallet state,
    /// mutate ledger state, or authorize payout by itself.
    pub fn apply_due_rotation(
        &mut self,
        service_node_id: &str,
        current_epoch: u64,
        applied_at_ms: u64,
    ) -> Result<bool, RewardBindingRegistryError> {
        let Some(rotation) = self.pending_rotation_by_node.get(service_node_id).cloned() else {
            return Ok(false);
        };

        if rotation.effective_epoch > current_epoch {
            return Ok(false);
        }

        let Some(current_binding_id) = self.current_binding_by_node.get(service_node_id).cloned()
        else {
            return Err(RewardBindingRegistryError::BindingNotFound {
                binding_id: rotation.old_binding_id,
            });
        };

        if current_binding_id != rotation.old_binding_id {
            return Err(RewardBindingRegistryError::ServiceNodeMismatch);
        }

        let Some(current_binding) = self.bindings_by_id.get(&current_binding_id).cloned() else {
            return Err(RewardBindingRegistryError::BindingNotFound {
                binding_id: current_binding_id,
            });
        };

        let new_binding_id = format!(
            "{}:rotated:{}",
            current_binding.binding_id, rotation.rotation_id
        );

        if self.bindings_by_id.contains_key(&new_binding_id) {
            return Err(RewardBindingRegistryError::DuplicateBindingId {
                binding_id: new_binding_id,
            });
        }

        let next_binding = ServiceNodeRewardBindingV1 {
            version: current_binding.version,
            binding_id: new_binding_id,
            service_node_id: rotation.service_node_id.clone(),
            operator_account_id: rotation.new_reward_recipient_account_id.clone(),
            operator_display_address: rotation.new_reward_recipient_display_address.clone(),
            operator_passport_id: current_binding.operator_passport_id.clone(),
            reward_recipient_account_id: rotation.new_reward_recipient_account_id.clone(),
            reward_recipient_display_address: rotation.new_reward_recipient_display_address.clone(),
            node_public_key: current_binding.node_public_key.clone(),
            operator_signature: rotation.operator_signature.clone(),
            node_signature: rotation.node_signature.clone(),
            created_at_ms: applied_at_ms,
            effective_epoch: rotation.effective_epoch,
            expires_at_ms: current_binding.expires_at_ms,
            rotation_nonce: rotation.rotation_nonce.clone(),
            policy_hash: rotation.policy_hash.clone(),
        };

        next_binding.validate()?;

        self.current_binding_by_node
            .insert(service_node_id.to_string(), next_binding.binding_id.clone());
        self.bindings_by_id
            .insert(next_binding.binding_id.clone(), next_binding);
        self.pending_rotation_by_node.remove(service_node_id);

        Ok(true)
    }

    /// Resolve a service node to its current reward recipient.
    ///
    /// Resolution returns display/account identity only when the service node is
    /// currently bound and active for the requested epoch/time. It never accepts
    /// a payout address from service evidence.
    #[must_use]
    pub fn resolve(
        &self,
        service_node_id: &str,
        resolved_at_epoch: u64,
        observed_at_ms: u64,
    ) -> RewardRecipientResolutionV1 {
        let Some(binding_id) = self.current_binding_by_node.get(service_node_id) else {
            return self.unresolved_resolution(
                service_node_id,
                resolved_at_epoch,
                RewardRecipientResolutionStateV1::Unbound,
                None,
            );
        };

        let Some(binding) = self.bindings_by_id.get(binding_id) else {
            return self.unresolved_resolution(
                service_node_id,
                resolved_at_epoch,
                RewardRecipientResolutionStateV1::Unbound,
                None,
            );
        };

        if binding.effective_epoch > resolved_at_epoch {
            return self.unresolved_resolution(
                service_node_id,
                resolved_at_epoch,
                RewardRecipientResolutionStateV1::Unbound,
                Some(binding.binding_id.clone()),
            );
        }

        if binding
            .expires_at_ms
            .is_some_and(|expires_at_ms| expires_at_ms <= observed_at_ms)
        {
            return self.unresolved_resolution(
                service_node_id,
                resolved_at_epoch,
                RewardRecipientResolutionStateV1::Expired,
                Some(binding.binding_id.clone()),
            );
        }

        RewardRecipientResolutionV1 {
            version: REWARD_RECIPIENT_RESOLUTION_VERSION,
            service_node_id: service_node_id.to_string(),
            resolved_at_epoch,
            registry_root: self.registry_root.clone(),
            reward_binding_root: self.reward_binding_root.clone(),
            state: RewardRecipientResolutionStateV1::Resolved,
            binding_id: Some(binding.binding_id.clone()),
            reward_recipient_account_id: Some(binding.reward_recipient_account_id.clone()),
            reward_recipient_display_address: Some(
                binding.reward_recipient_display_address.clone(),
            ),
        }
    }

    /// Return a truthful reward-recipient status view for CLI/UI surfaces.
    ///
    /// This is display/control-plane status only. Confirmed ROC still belongs to
    /// wallet/ledger receipt surfaces.
    #[must_use]
    pub fn status(
        &self,
        service_node_id: &str,
        observed_registry_epoch: u64,
        observed_at_ms: u64,
    ) -> NodeRewardRecipientStatusV1 {
        let Some(binding_id) = self.current_binding_by_node.get(service_node_id) else {
            return NodeRewardRecipientStatusV1 {
                version: NODE_REWARD_RECIPIENT_STATUS_VERSION,
                service_node_id: service_node_id.to_string(),
                state: NodeRewardRecipientStateV1::Unbound,
                current_binding: None,
                pending_rotation: None,
                observed_registry_epoch,
                notes: Vec::new(),
            };
        };

        let binding = self.bindings_by_id.get(binding_id).cloned();
        let pending_rotation = self.pending_rotation_by_node.get(service_node_id).cloned();

        let state = if binding
            .as_ref()
            .and_then(|binding| binding.expires_at_ms)
            .is_some_and(|expires_at_ms| expires_at_ms <= observed_at_ms)
        {
            NodeRewardRecipientStateV1::Expired
        } else if pending_rotation.is_some() {
            NodeRewardRecipientStateV1::PendingRotation
        } else {
            NodeRewardRecipientStateV1::Bound
        };

        NodeRewardRecipientStatusV1 {
            version: NODE_REWARD_RECIPIENT_STATUS_VERSION,
            service_node_id: service_node_id.to_string(),
            state,
            current_binding: binding,
            pending_rotation,
            observed_registry_epoch,
            notes: vec!["display_only_not_ledger_truth".to_string()],
        }
    }

    fn unresolved_resolution(
        &self,
        service_node_id: &str,
        resolved_at_epoch: u64,
        state: RewardRecipientResolutionStateV1,
        binding_id: Option<String>,
    ) -> RewardRecipientResolutionV1 {
        RewardRecipientResolutionV1 {
            version: REWARD_RECIPIENT_RESOLUTION_VERSION,
            service_node_id: service_node_id.to_string(),
            resolved_at_epoch,
            registry_root: self.registry_root.clone(),
            reward_binding_root: self.reward_binding_root.clone(),
            state,
            binding_id,
            reward_recipient_account_id: None,
            reward_recipient_display_address: None,
        }
    }
}
