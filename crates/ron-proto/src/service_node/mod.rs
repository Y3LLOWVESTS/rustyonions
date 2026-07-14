//! RO:WHAT — Service-node binding, evidence, quorum, epoch-transition, and eligibility DTOs.
//! RO:WHY — BUILD_PLAN_Z requires objective service identity, bound rewards, quorum issuance, and protocol-earned eligibility.
//! RO:INTERACTS — svc-registry, ron-policy, ron-accounting, svc-rewarder, svc-wallet, ron-ledger, micronode, macronode.
//! RO:INVARIANTS — DTO-only; one canonical eligibility lifecycle; no signature verification, registry mutation, or wallet/ledger authority.
//! RO:SECURITY — evidence references service_node_id; recipients resolve through bindings; no founder/manual trust path.

mod accounting_input;
pub use accounting_input::*;

mod eligibility;
pub use eligibility::*;

mod enforcement;
pub use enforcement::*;

mod quorum;
pub use quorum::*;

mod epoch_transition;
pub use epoch_transition::*;

mod user_verification_accounting;
pub use user_verification_accounting::*;

mod availability_evidence;
pub use availability_evidence::*;

mod range_repair_evidence;
pub use range_repair_evidence::*;

mod policy_evidence;
pub use policy_evidence::*;

mod evidence;
pub use evidence::*;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::id::ContentId;

/// Current reward binding DTO version.
pub const SERVICE_NODE_REWARD_BINDING_VERSION: u16 = 1;

/// Current reward binding rotation DTO version.
pub const SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION: u16 = 1;

/// Current reward-recipient resolution DTO version.
pub const REWARD_RECIPIENT_RESOLUTION_VERSION: u16 = 1;

/// Current node reward-recipient status DTO version.
pub const NODE_REWARD_RECIPIENT_STATUS_VERSION: u16 = 1;

const MAX_REF_BYTES: usize = 512;
const MAX_DISPLAY_ADDRESS_BYTES: usize = 64;
const MAX_SIGNATURE_BYTES: usize = 1024;

/// Validation errors for service-node reward-recipient DTOs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeRewardBindingValidationError {
    /// DTO version was not the current version for that type.
    #[error("invalid {ty} version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// DTO type name.
        ty: &'static str,
        /// Expected version.
        expected: u16,
        /// Actual version.
        actual: u16,
    },

    /// A required field was empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },

    /// A field exceeded its maximum byte length.
    #[error("{field} exceeds max bytes")]
    FieldTooLong {
        /// Field name.
        field: &'static str,
        /// Maximum byte length.
        max: usize,
        /// Actual byte length.
        actual: usize,
    },

    /// An identifier-like field contained unsupported characters.
    #[error("{field} contains unsupported characters")]
    InvalidToken {
        /// Field name.
        field: &'static str,
    },

    /// A human-facing `@` address was malformed.
    #[error("{field} must be a canonical @ address")]
    InvalidDisplayAddress {
        /// Field name.
        field: &'static str,
    },

    /// An epoch field was zero.
    #[error("{field} must be greater than zero")]
    ZeroEpoch {
        /// Field name.
        field: &'static str,
    },

    /// A timestamp field was zero.
    #[error("{field} must be greater than zero")]
    ZeroTimestamp {
        /// Field name.
        field: &'static str,
    },

    /// Expiry or effective ordering was invalid.
    #[error("invalid time or epoch ordering: {field}")]
    InvalidOrdering {
        /// Field/relation name.
        field: &'static str,
    },

    /// A status DTO contained a field that contradicts its state.
    #[error("invalid status shape: {field}")]
    InvalidStatusShape {
        /// Field/relation name.
        field: &'static str,
    },
}

/// Signature reference carried by reward binding DTOs.
///
/// Verification belongs to `ron-auth`, `svc-passport`, and key-management layers.
/// `ron-proto` only represents the strict wire shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardBindingSignatureRefV1 {
    /// Signature algorithm tag.
    pub alg: String,
    /// Public key reference or key ID.
    pub public_key_ref: String,
    /// Signature bytes encoded by the signing layer.
    pub signature: String,
}

impl RewardBindingSignatureRefV1 {
    /// Validate the signature reference shape.
    pub fn validate(&self) -> Result<(), ServiceNodeRewardBindingValidationError> {
        validate_token("signature.alg", &self.alg, MAX_REF_BYTES)?;
        validate_token(
            "signature.public_key_ref",
            &self.public_key_ref,
            MAX_REF_BYTES,
        )?;
        validate_bounded_nonempty("signature.signature", &self.signature, MAX_SIGNATURE_BYTES)
    }
}

/// Signed binding from a service node to a durable reward recipient account.
///
/// This DTO does not pay rewards. It only describes the binding that registry,
/// policy, wallet, and ledger layers must later verify and enforce.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardBindingV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Stable binding identifier.
    pub binding_id: String,
    /// Service node that earned reward-eligible evidence.
    pub service_node_id: String,
    /// Canonical operator account identifier.
    pub operator_account_id: String,
    /// Human-facing operator display address, e.g. `@stevan`.
    pub operator_display_address: String,
    /// Optional passport identifier associated with the operator.
    #[serde(default)]
    pub operator_passport_id: Option<String>,
    /// Canonical account that may receive ROC after wallet/ledger-confirmed payout.
    pub reward_recipient_account_id: String,
    /// Human-facing recipient display address, e.g. `@stevan`.
    pub reward_recipient_display_address: String,
    /// Service-node public key reference.
    pub node_public_key: String,
    /// Operator/account signature reference.
    pub operator_signature: RewardBindingSignatureRefV1,
    /// Node signature reference.
    pub node_signature: RewardBindingSignatureRefV1,
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_at_ms: u64,
    /// First epoch where the binding may be used.
    pub effective_epoch: u64,
    /// Optional expiry timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub expires_at_ms: Option<u64>,
    /// Replay/rotation nonce. Registry rejects duplicates.
    pub rotation_nonce: String,
    /// Policy bundle hash governing this binding.
    pub policy_hash: ContentId,
}

impl ServiceNodeRewardBindingV1 {
    /// Validate binding shape and non-authority boundaries.
    pub fn validate(&self) -> Result<(), ServiceNodeRewardBindingValidationError> {
        validate_version(
            "ServiceNodeRewardBindingV1",
            self.version,
            SERVICE_NODE_REWARD_BINDING_VERSION,
        )?;
        validate_token("binding_id", &self.binding_id, MAX_REF_BYTES)?;
        validate_token("service_node_id", &self.service_node_id, MAX_REF_BYTES)?;
        validate_token(
            "operator_account_id",
            &self.operator_account_id,
            MAX_REF_BYTES,
        )?;
        validate_display_address("operator_display_address", &self.operator_display_address)?;

        if let Some(passport_id) = &self.operator_passport_id {
            validate_token("operator_passport_id", passport_id, MAX_REF_BYTES)?;
        }

        validate_token(
            "reward_recipient_account_id",
            &self.reward_recipient_account_id,
            MAX_REF_BYTES,
        )?;
        validate_display_address(
            "reward_recipient_display_address",
            &self.reward_recipient_display_address,
        )?;
        validate_bounded_nonempty(
            "node_public_key",
            &self.node_public_key,
            MAX_SIGNATURE_BYTES,
        )?;
        self.operator_signature.validate()?;
        self.node_signature.validate()?;
        validate_timestamp("created_at_ms", self.created_at_ms)?;
        validate_epoch("effective_epoch", self.effective_epoch)?;

        if let Some(expires_at_ms) = self.expires_at_ms {
            if expires_at_ms <= self.created_at_ms {
                return Err(ServiceNodeRewardBindingValidationError::InvalidOrdering {
                    field: "expires_at_ms",
                });
            }
        }

        validate_token("rotation_nonce", &self.rotation_nonce, MAX_REF_BYTES)?;

        Ok(())
    }
}

/// Signed pending rotation from one recipient binding to another.
///
/// Rotation is delayed by epoch so old unpaid epochs cannot be silently redirected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardBindingRotationV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Stable rotation identifier.
    pub rotation_id: String,
    /// Service node whose recipient is being rotated.
    pub service_node_id: String,
    /// Binding being superseded.
    pub old_binding_id: String,
    /// New canonical recipient account.
    pub new_reward_recipient_account_id: String,
    /// New human-facing recipient display address.
    pub new_reward_recipient_display_address: String,
    /// Timestamp when the rotation was requested.
    pub requested_at_ms: u64,
    /// Epoch when the rotation was requested.
    pub requested_epoch: u64,
    /// Future epoch where the rotation becomes active.
    pub effective_epoch: u64,
    /// Replay/rotation nonce. Registry rejects duplicates.
    pub rotation_nonce: String,
    /// Policy bundle hash governing this rotation.
    pub policy_hash: ContentId,
    /// New recipient/operator signature.
    pub operator_signature: RewardBindingSignatureRefV1,
    /// Node signature.
    pub node_signature: RewardBindingSignatureRefV1,
    /// Optional old recipient signature when available.
    #[serde(default)]
    pub old_operator_signature: Option<RewardBindingSignatureRefV1>,
}

impl ServiceNodeRewardBindingRotationV1 {
    /// Validate rotation shape.
    pub fn validate(&self) -> Result<(), ServiceNodeRewardBindingValidationError> {
        validate_version(
            "ServiceNodeRewardBindingRotationV1",
            self.version,
            SERVICE_NODE_REWARD_BINDING_ROTATION_VERSION,
        )?;
        validate_token("rotation_id", &self.rotation_id, MAX_REF_BYTES)?;
        validate_token("service_node_id", &self.service_node_id, MAX_REF_BYTES)?;
        validate_token("old_binding_id", &self.old_binding_id, MAX_REF_BYTES)?;
        validate_token(
            "new_reward_recipient_account_id",
            &self.new_reward_recipient_account_id,
            MAX_REF_BYTES,
        )?;
        validate_display_address(
            "new_reward_recipient_display_address",
            &self.new_reward_recipient_display_address,
        )?;
        validate_timestamp("requested_at_ms", self.requested_at_ms)?;
        validate_epoch("requested_epoch", self.requested_epoch)?;
        validate_epoch("effective_epoch", self.effective_epoch)?;

        if self.effective_epoch <= self.requested_epoch {
            return Err(ServiceNodeRewardBindingValidationError::InvalidOrdering {
                field: "effective_epoch",
            });
        }

        validate_token("rotation_nonce", &self.rotation_nonce, MAX_REF_BYTES)?;
        self.operator_signature.validate()?;
        self.node_signature.validate()?;

        if let Some(old_signature) = &self.old_operator_signature {
            old_signature.validate()?;
        }

        Ok(())
    }
}

/// Reward recipient resolution state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RewardRecipientResolutionStateV1 {
    /// Registry resolved the service node to a bound canonical account.
    Resolved,
    /// No usable binding exists.
    Unbound,
    /// Binding exists but is expired.
    Expired,
    /// Binding/account/node is disabled.
    Disabled,
}

/// Registry resolution from `service_node_id` to canonical account.
///
/// This DTO still does not pay rewards. It gives wallet/ledger layers the
/// canonical recipient identity they must verify before payout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardRecipientResolutionV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Service node being resolved.
    pub service_node_id: String,
    /// Epoch where the resolution was computed.
    pub resolved_at_epoch: u64,
    /// Registry root used for resolution.
    pub registry_root: ContentId,
    /// Reward-binding root used for resolution.
    pub reward_binding_root: ContentId,
    /// Resolution state.
    pub state: RewardRecipientResolutionStateV1,
    /// Binding id when resolved or known.
    #[serde(default)]
    pub binding_id: Option<String>,
    /// Canonical recipient account when resolved.
    #[serde(default)]
    pub reward_recipient_account_id: Option<String>,
    /// Human-facing recipient address when resolved.
    #[serde(default)]
    pub reward_recipient_display_address: Option<String>,
}

impl RewardRecipientResolutionV1 {
    /// Validate resolution shape.
    pub fn validate(&self) -> Result<(), ServiceNodeRewardBindingValidationError> {
        validate_version(
            "RewardRecipientResolutionV1",
            self.version,
            REWARD_RECIPIENT_RESOLUTION_VERSION,
        )?;
        validate_token("service_node_id", &self.service_node_id, MAX_REF_BYTES)?;
        validate_epoch("resolved_at_epoch", self.resolved_at_epoch)?;

        if let Some(binding_id) = &self.binding_id {
            validate_token("binding_id", binding_id, MAX_REF_BYTES)?;
        }

        match self.state {
            RewardRecipientResolutionStateV1::Resolved => {
                let Some(account_id) = &self.reward_recipient_account_id else {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "reward_recipient_account_id",
                        },
                    );
                };
                let Some(display_address) = &self.reward_recipient_display_address else {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "reward_recipient_display_address",
                        },
                    );
                };
                let Some(binding_id) = &self.binding_id else {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "binding_id",
                        },
                    );
                };

                validate_token("binding_id", binding_id, MAX_REF_BYTES)?;
                validate_token("reward_recipient_account_id", account_id, MAX_REF_BYTES)?;
                validate_display_address("reward_recipient_display_address", display_address)?;
            }
            RewardRecipientResolutionStateV1::Unbound
            | RewardRecipientResolutionStateV1::Expired
            | RewardRecipientResolutionStateV1::Disabled => {
                if self.reward_recipient_account_id.is_some()
                    || self.reward_recipient_display_address.is_some()
                {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "unresolved_recipient_fields",
                        },
                    );
                }
            }
        }

        Ok(())
    }
}

/// Node reward-recipient status state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NodeRewardRecipientStateV1 {
    /// No reward recipient binding exists.
    Unbound,
    /// A current binding exists.
    Bound,
    /// A current binding exists and a future rotation is pending.
    PendingRotation,
    /// The current binding is expired.
    Expired,
    /// The node/account/binding is disabled by policy or registry state.
    Disabled,
}

/// Status view consumed by CLI/UI surfaces.
///
/// This is display/control-plane status only; confirmed ROC still comes from
/// wallet/ledger receipts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NodeRewardRecipientStatusV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Service node id.
    pub service_node_id: String,
    /// Status state.
    pub state: NodeRewardRecipientStateV1,
    /// Current binding, when available.
    #[serde(default)]
    pub current_binding: Option<ServiceNodeRewardBindingV1>,
    /// Pending rotation, when available.
    #[serde(default)]
    pub pending_rotation: Option<ServiceNodeRewardBindingRotationV1>,
    /// Last registry epoch observed by the reporter.
    pub observed_registry_epoch: u64,
    /// Human-readable non-authoritative notes.
    #[serde(default)]
    pub notes: Vec<String>,
}

impl NodeRewardRecipientStatusV1 {
    /// Validate node reward-recipient status shape.
    pub fn validate(&self) -> Result<(), ServiceNodeRewardBindingValidationError> {
        validate_version(
            "NodeRewardRecipientStatusV1",
            self.version,
            NODE_REWARD_RECIPIENT_STATUS_VERSION,
        )?;
        validate_token("service_node_id", &self.service_node_id, MAX_REF_BYTES)?;
        validate_epoch("observed_registry_epoch", self.observed_registry_epoch)?;

        match self.state {
            NodeRewardRecipientStateV1::Unbound => {
                if self.current_binding.is_some() || self.pending_rotation.is_some() {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "unbound_status",
                        },
                    );
                }
            }
            NodeRewardRecipientStateV1::Bound => {
                let Some(binding) = &self.current_binding else {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "current_binding",
                        },
                    );
                };
                binding.validate()?;

                if self.pending_rotation.is_some() {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "pending_rotation",
                        },
                    );
                }
            }
            NodeRewardRecipientStateV1::PendingRotation => {
                let Some(binding) = &self.current_binding else {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "current_binding",
                        },
                    );
                };
                let Some(rotation) = &self.pending_rotation else {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "pending_rotation",
                        },
                    );
                };
                binding.validate()?;
                rotation.validate()?;

                if binding.service_node_id != self.service_node_id
                    || rotation.service_node_id != self.service_node_id
                {
                    return Err(
                        ServiceNodeRewardBindingValidationError::InvalidStatusShape {
                            field: "service_node_id",
                        },
                    );
                }
            }
            NodeRewardRecipientStateV1::Expired | NodeRewardRecipientStateV1::Disabled => {
                if let Some(binding) = &self.current_binding {
                    binding.validate()?;
                }
                if let Some(rotation) = &self.pending_rotation {
                    rotation.validate()?;
                }
            }
        }

        for note in &self.notes {
            validate_bounded_nonempty("notes[]", note, MAX_REF_BYTES)?;
        }

        Ok(())
    }
}

fn validate_version(
    ty: &'static str,
    actual: u16,
    expected: u16,
) -> Result<(), ServiceNodeRewardBindingValidationError> {
    if actual == expected {
        return Ok(());
    }

    Err(ServiceNodeRewardBindingValidationError::InvalidVersion {
        ty,
        expected,
        actual,
    })
}

fn validate_epoch(
    field: &'static str,
    epoch: u64,
) -> Result<(), ServiceNodeRewardBindingValidationError> {
    if epoch == 0 {
        return Err(ServiceNodeRewardBindingValidationError::ZeroEpoch { field });
    }

    Ok(())
}

fn validate_timestamp(
    field: &'static str,
    timestamp_ms: u64,
) -> Result<(), ServiceNodeRewardBindingValidationError> {
    if timestamp_ms == 0 {
        return Err(ServiceNodeRewardBindingValidationError::ZeroTimestamp { field });
    }

    Ok(())
}

fn validate_bounded_nonempty(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ServiceNodeRewardBindingValidationError> {
    if value.trim().is_empty() {
        return Err(ServiceNodeRewardBindingValidationError::EmptyField { field });
    }

    if value.len() > max_bytes {
        return Err(ServiceNodeRewardBindingValidationError::FieldTooLong {
            field,
            max: max_bytes,
            actual: value.len(),
        });
    }

    Ok(())
}

fn validate_token(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ServiceNodeRewardBindingValidationError> {
    validate_bounded_nonempty(field, value, max_bytes)?;

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/')
    }) {
        return Err(ServiceNodeRewardBindingValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_display_address(
    field: &'static str,
    value: &str,
) -> Result<(), ServiceNodeRewardBindingValidationError> {
    validate_bounded_nonempty(field, value, MAX_DISPLAY_ADDRESS_BYTES)?;

    let Some(username) = value.strip_prefix('@') else {
        return Err(ServiceNodeRewardBindingValidationError::InvalidDisplayAddress { field });
    };

    if !is_valid_username(username) {
        return Err(ServiceNodeRewardBindingValidationError::InvalidDisplayAddress { field });
    }

    Ok(())
}

fn is_valid_username(username: &str) -> bool {
    let bytes = username.as_bytes();

    if !(3..=32).contains(&bytes.len()) {
        return false;
    }

    if !bytes[0].is_ascii_alphanumeric() {
        return false;
    }

    if matches!(bytes[bytes.len() - 1], b'.' | b'-' | b'_') {
        return false;
    }

    let mut previous_dot = false;
    for byte in bytes {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(*byte, b'_' | b'-' | b'.');

        if !valid {
            return false;
        }

        if previous_dot && *byte == b'.' {
            return false;
        }

        previous_dot = *byte == b'.';
    }

    true
}
