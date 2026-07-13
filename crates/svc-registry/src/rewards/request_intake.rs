//! RO:WHAT — Registry-backed reward-recipient binding request intake.
//! RO:WHY — Phase 5C moves from macronode-local request display state into svc-registry-backed binding logic.
//! RO:INTERACTS — RewardBindingRegistry, ron-proto binding DTOs, future passport/auth signing.
//! RO:INVARIANTS — no wallet mutation; no ledger mutation; no confirmed ROC; no arbitrary evidence payout override.
//! RO:SECURITY — reward recipient must be registry-derived, not smuggled by service evidence.

use ron_proto::{
    ContentId, RewardBindingSignatureRefV1, ServiceNodeRewardBindingRotationV1,
    ServiceNodeRewardBindingV1, SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION,
    SERVICE_NODE_REWARD_BINDING_VERSION,
};
use thiserror::Error;

use super::{RewardBindingRegistry, RewardBindingRegistryError};

/// Recipient identity already resolved by the future passport/registry boundary.
///
/// This local Phase 5C type is deliberately plain: it models the result of a
/// trusted identity lookup without implementing passport, auth, wallet, or ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRewardRecipientForBinding {
    /// Canonical internal account id to receive future rewards.
    pub account_id: String,
    /// Operator-visible CrabLink/RON display address.
    pub display_address: String,
}

/// Registry-backed binding request.
///
/// This is not service evidence and it is not a payout request. It is the
/// validated input used to construct a strict ron-proto binding DTO and insert
/// it into the local reward-binding registry model.
#[derive(Debug, Clone)]
pub struct RegistryRewardBindingRequestV1 {
    /// Unique binding id.
    pub binding_id: String,
    /// Service node being bound.
    pub service_node_id: String,
    /// Operator account requesting/owning the binding.
    pub operator_account_id: String,
    /// Operator-visible address.
    pub operator_display_address: String,
    /// Optional passport id observed by the future passport boundary.
    pub operator_passport_id: Option<String>,
    /// Resolved reward recipient.
    pub reward_recipient: ResolvedRewardRecipientForBinding,
    /// Service-node public key reference.
    pub node_public_key: String,
    /// Operator signature reference.
    pub operator_signature: RewardBindingSignatureRefV1,
    /// Node signature reference.
    pub node_signature: RewardBindingSignatureRefV1,
    /// Request creation time in milliseconds.
    pub created_at_ms: u64,
    /// First epoch where this binding can be effective.
    pub effective_epoch: u64,
    /// Optional expiration time in milliseconds.
    pub expires_at_ms: Option<u64>,
    /// Nonce for replay rejection.
    pub rotation_nonce: String,
    /// Policy hash binding this request to policy text/material.
    pub policy_hash: ContentId,
    /// Forbidden: service evidence cannot carry payout destination.
    pub evidence_payout_override: Option<String>,
}

/// Registry-backed rotation request.
#[derive(Debug, Clone)]
pub struct RegistryRewardBindingRotationRequestV1 {
    /// Unique rotation id.
    pub rotation_id: String,
    /// Service node being rotated.
    pub service_node_id: String,
    /// Existing binding id that this rotation replaces.
    pub old_binding_id: String,
    /// New resolved reward recipient.
    pub new_reward_recipient: ResolvedRewardRecipientForBinding,
    /// Request time in milliseconds.
    pub requested_at_ms: u64,
    /// Epoch where the request was made.
    pub requested_epoch: u64,
    /// Future epoch where the rotation can take effect.
    pub effective_epoch: u64,
    /// Nonce for replay rejection.
    pub rotation_nonce: String,
    /// Policy hash binding this request to policy text/material.
    pub policy_hash: ContentId,
    /// New operator signature reference.
    pub operator_signature: RewardBindingSignatureRefV1,
    /// Node signature reference.
    pub node_signature: RewardBindingSignatureRefV1,
    /// Optional old operator signature reference.
    pub old_operator_signature: Option<RewardBindingSignatureRefV1>,
    /// Forbidden: service evidence cannot carry payout destination.
    pub evidence_payout_override: Option<String>,
}

/// Non-authoritative receipt returned by registry intake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryRewardBindingRequestReceipt {
    /// Intake status string for operator/admin surfaces.
    pub status: &'static str,
    /// Service node affected by the request.
    pub service_node_id: String,
    /// Binding or rotation id accepted by the registry model.
    pub request_id: String,
    /// Registry-derived recipient account id.
    pub reward_recipient_account_id: String,
    /// Registry-derived display address.
    pub reward_recipient_display_address: String,
    /// Wallet mutation remains false at this layer.
    pub wallet_mutation: bool,
    /// Ledger mutation remains false at this layer.
    pub ledger_mutation: bool,
    /// Confirmed ROC is not produced by registry intake.
    pub confirmed_roc: Option<u64>,
}

/// Registry request-intake errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RewardBindingRequestError {
    /// Service evidence tried to carry its own payout recipient.
    #[error("service evidence must not carry payout recipient override")]
    EvidencePayoutOverrideForbidden,

    /// The registry rejected the constructed binding/rotation DTO.
    #[error("registry rejected reward binding request: {0}")]
    Registry(#[from] RewardBindingRegistryError),
}

/// Local request-intake wrapper around RewardBindingRegistry.
#[derive(Debug, Clone)]
pub struct RegistryRewardBindingRequestIntake {
    registry: RewardBindingRegistry,
}

impl RegistryRewardBindingRequestIntake {
    /// Create request intake backed by a local reward-binding registry.
    #[must_use]
    pub fn new(registry: RewardBindingRegistry) -> Self {
        Self { registry }
    }

    /// Read the underlying registry model.
    #[must_use]
    pub fn registry(&self) -> &RewardBindingRegistry {
        &self.registry
    }

    /// Mutably read the underlying registry model for focused tests/future wiring.
    pub fn registry_mut(&mut self) -> &mut RewardBindingRegistry {
        &mut self.registry
    }

    /// Accept a registry-backed binding request.
    ///
    /// This constructs a strict ron-proto binding DTO and inserts it through the
    /// existing registry path. It does not execute rewards or mutate wallet/ledger.
    pub fn submit_binding_request(
        &mut self,
        request: RegistryRewardBindingRequestV1,
    ) -> Result<RegistryRewardBindingRequestReceipt, RewardBindingRequestError> {
        if request.evidence_payout_override.is_some() {
            return Err(RewardBindingRequestError::EvidencePayoutOverrideForbidden);
        }

        let binding = ServiceNodeRewardBindingV1 {
            version: SERVICE_NODE_REWARD_BINDING_VERSION,
            binding_id: request.binding_id.clone(),
            service_node_id: request.service_node_id.clone(),
            operator_account_id: request.operator_account_id,
            operator_display_address: request.operator_display_address,
            operator_passport_id: request.operator_passport_id,
            reward_recipient_account_id: request.reward_recipient.account_id.clone(),
            reward_recipient_display_address: request.reward_recipient.display_address.clone(),
            node_public_key: request.node_public_key,
            operator_signature: request.operator_signature,
            node_signature: request.node_signature,
            created_at_ms: request.created_at_ms,
            effective_epoch: request.effective_epoch,
            expires_at_ms: request.expires_at_ms,
            rotation_nonce: request.rotation_nonce,
            policy_hash: request.policy_hash,
        };

        self.registry.insert_binding(binding)?;

        Ok(RegistryRewardBindingRequestReceipt {
            status: "binding accepted by registry intake",
            service_node_id: request.service_node_id,
            request_id: request.binding_id,
            reward_recipient_account_id: request.reward_recipient.account_id,
            reward_recipient_display_address: request.reward_recipient.display_address,
            wallet_mutation: false,
            ledger_mutation: false,
            confirmed_roc: None,
        })
    }

    /// Accept a registry-backed future-epoch rotation request.
    ///
    /// This constructs a strict ron-proto rotation DTO and schedules it through the
    /// registry. It does not apply the rotation, execute rewards, or mutate wallet/ledger.
    pub fn submit_rotation_request(
        &mut self,
        request: RegistryRewardBindingRotationRequestV1,
    ) -> Result<RegistryRewardBindingRequestReceipt, RewardBindingRequestError> {
        if request.evidence_payout_override.is_some() {
            return Err(RewardBindingRequestError::EvidencePayoutOverrideForbidden);
        }

        let rotation = ServiceNodeRewardBindingRotationV1 {
            version: SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION,
            rotation_id: request.rotation_id.clone(),
            service_node_id: request.service_node_id.clone(),
            old_binding_id: request.old_binding_id,
            new_reward_recipient_account_id: request.new_reward_recipient.account_id.clone(),
            new_reward_recipient_display_address: request
                .new_reward_recipient
                .display_address
                .clone(),
            requested_at_ms: request.requested_at_ms,
            requested_epoch: request.requested_epoch,
            effective_epoch: request.effective_epoch,
            rotation_nonce: request.rotation_nonce,
            policy_hash: request.policy_hash,
            operator_signature: request.operator_signature,
            node_signature: request.node_signature,
            old_operator_signature: request.old_operator_signature,
        };

        self.registry.schedule_rotation(rotation)?;

        Ok(RegistryRewardBindingRequestReceipt {
            status: "rotation accepted by registry intake",
            service_node_id: request.service_node_id,
            request_id: request.rotation_id,
            reward_recipient_account_id: request.new_reward_recipient.account_id,
            reward_recipient_display_address: request.new_reward_recipient.display_address,
            wallet_mutation: false,
            ledger_mutation: false,
            confirmed_roc: None,
        })
    }
}
