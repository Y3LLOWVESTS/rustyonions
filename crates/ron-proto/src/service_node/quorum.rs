//! RO:WHAT — Strict Phase 15 Service Node eligibility, threshold, signature, and quorum DTOs.
//! RO:WHY — ECON/GOV: require objective multi-node participation before a ROC epoch transition can advance toward execution.
//! RO:INTERACTS — svc-registry eligibility, ron-kms verification, future RocEpochTransitionV1 review, wallet/ledger execution.
//! RO:INVARIANTS — strict-majority threshold; at least two signers; canonical ordering; duplicate/out-of-set signature rejection.
//! RO:SECURITY — signature bytes are data only; no private keys, crypto bypass, wallet mutation, ledger mutation, or recipient override.
//! RO:TEST — tests/internal_roc_beta_phase15_service_node_quorum.rs.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{id::ContentId, quantum::SignatureAlg};

/// Current version for Phase 15 Service Node quorum DTOs.
pub const SERVICE_NODE_QUORUM_VERSION: u16 = 1;

/// Canonical schema for a Phase 15 Service Node quorum artifact.
pub const SERVICE_NODE_QUORUM_SCHEMA: &str = "ron.service_node.quorum.v1";

/// Maximum bounded Service Node membership/signature set.
pub const MAX_SERVICE_NODE_QUORUM_MEMBERS: usize = 128;

const MAX_CHAIN_ID_BYTES: usize = 64;
const MAX_EPOCH_ID_BYTES: usize = 96;
const MAX_REF_BYTES: usize = 256;
const MAX_SIGNATURE_BYTES: usize = 2_048;
const BPS_DENOMINATOR: u16 = 10_000;

/// Structural validation failures for Phase 15 Service Node quorum artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceNodeQuorumValidationError {
    /// A DTO used an unsupported version.
    #[error("invalid {ty} version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// DTO type.
        ty: &'static str,
        /// Required version.
        expected: u16,
        /// Supplied version.
        actual: u16,
    },

    /// A field failed bounded or semantic validation.
    #[error("invalid Service Node quorum field {field}: {reason}")]
    InvalidField {
        /// Invalid field.
        field: &'static str,
        /// Stable failure explanation.
        reason: &'static str,
    },

    /// A bounded vector exceeded the protocol maximum.
    #[error("too many items for {field}: max={max}, actual={actual}")]
    TooManyItems {
        /// Bounded field.
        field: &'static str,
        /// Maximum permitted items.
        max: usize,
        /// Supplied item count.
        actual: usize,
    },

    /// A value that must be unique was repeated.
    #[error("duplicate Service Node quorum value: {field}")]
    Duplicate {
        /// Repeated field.
        field: &'static str,
    },

    /// Signature or quorum material did not bind to the expected artifact.
    #[error("Service Node quorum binding mismatch: {field}")]
    Mismatch {
        /// Mismatched field.
        field: &'static str,
    },

    /// The supplied unique signature set did not reach the threshold.
    #[error("insufficient Service Node quorum signatures: required={required}, actual={actual}")]
    InsufficientQuorum {
        /// Required signature count.
        required: usize,
        /// Supplied signature count.
        actual: usize,
    },
}

/// Eligibility posture for a Service Node in one specific ROC epoch.
///
/// Phase 18 lifecycle descriptors deterministically project into this value.
/// The quorum artifact continues to consume only an explicit epoch-bound result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EpochEligibilityStatusV1 {
    /// Node may participate in this epoch quorum.
    Eligible,

    /// Node may not participate in this epoch quorum.
    Ineligible,
}

/// Registry, reward-binding, and signing-key identity for one epoch member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpochEligibilityV1 {
    /// DTO version.
    pub version: u16,

    /// Canonical Service Node identity.
    pub service_node_id: String,

    /// Registry entry that admitted the node into this epoch set.
    pub registry_entry_id: String,

    /// Reward-recipient binding identity committed for this node.
    ///
    /// This is an identity/reference only. It is not a recipient account and
    /// does not grant payout authority.
    pub reward_binding_id: String,

    /// Public verification key identity expected for this node.
    pub key_id: String,

    /// Epoch-specific eligibility result.
    pub status: EpochEligibilityStatusV1,
}

impl EpochEligibilityV1 {
    /// Validate bounded identity shape.
    pub fn validate(&self) -> Result<(), ServiceNodeQuorumValidationError> {
        validate_version("EpochEligibilityV1", self.version)?;
        validate_service_node_id(&self.service_node_id)?;
        validate_token("registry_entry_id", &self.registry_entry_id, MAX_REF_BYTES)?;
        validate_token("reward_binding_id", &self.reward_binding_id, MAX_REF_BYTES)?;
        validate_token("key_id", &self.key_id, MAX_REF_BYTES)
    }
}

/// Deterministic quorum count derived from an eligible membership set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpochQuorumThresholdV1 {
    /// DTO version.
    pub version: u16,

    /// Total eligible Service Nodes committed into this epoch.
    pub eligible_service_nodes: u16,

    /// Required percentage in basis points.
    ///
    /// Phase 15 requires a strict majority: greater than 5,000 and no greater
    /// than 10,000.
    pub quorum_bps: u16,

    /// Absolute floor even when the basis-point calculation is lower.
    ///
    /// This must always be at least two.
    pub minimum_signatures: u16,

    /// Deterministically calculated signature requirement.
    pub required_signatures: u16,
}

impl EpochQuorumThresholdV1 {
    /// Validate the threshold and recompute its required signature count.
    pub fn validate(&self) -> Result<(), ServiceNodeQuorumValidationError> {
        validate_version("EpochQuorumThresholdV1", self.version)?;

        if self.eligible_service_nodes < 2 {
            return invalid(
                "eligible_service_nodes",
                "must contain at least two Service Nodes",
            );
        }

        if usize::from(self.eligible_service_nodes) > MAX_SERVICE_NODE_QUORUM_MEMBERS {
            return Err(ServiceNodeQuorumValidationError::TooManyItems {
                field: "eligible_service_nodes",
                max: MAX_SERVICE_NODE_QUORUM_MEMBERS,
                actual: usize::from(self.eligible_service_nodes),
            });
        }

        if self.quorum_bps <= BPS_DENOMINATOR / 2 || self.quorum_bps > BPS_DENOMINATOR {
            return invalid(
                "quorum_bps",
                "must be a strict majority at or below 10000 bps",
            );
        }

        if self.minimum_signatures < 2 || self.minimum_signatures > self.eligible_service_nodes {
            return invalid(
                "minimum_signatures",
                "must be at least two and no greater than eligible_service_nodes",
            );
        }

        let weighted = u32::from(self.eligible_service_nodes) * u32::from(self.quorum_bps);

        let expected = weighted
            .div_ceil(u32::from(BPS_DENOMINATOR))
            .max(u32::from(self.minimum_signatures));

        if u32::from(self.required_signatures) != expected
            || self.required_signatures > self.eligible_service_nodes
        {
            return invalid(
                "required_signatures",
                "must equal the deterministic quorum calculation",
            );
        }

        Ok(())
    }
}

/// One Service Node signature over a canonical epoch-transition hash.
///
/// This DTO validates shape and identity binding only. `ron-proto` does not
/// perform cryptographic verification or claim that the signature is valid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeSignatureV1 {
    /// DTO version.
    pub version: u16,

    /// Chain/domain identifier.
    pub chain_id: String,

    /// Epoch identifier.
    pub epoch_id: String,

    /// Signing Service Node.
    pub service_node_id: String,

    /// Public verification key identity.
    pub key_id: String,

    /// Declared signature algorithm.
    pub algorithm: SignatureAlg,

    /// Canonical transition hash that was signed.
    pub transition_hash: ContentId,

    /// Bounded wire-form signature material.
    pub signature_wire: String,
}

impl ServiceNodeSignatureV1 {
    /// Validate DTO shape only; cryptographic verification remains external.
    pub fn validate(&self) -> Result<(), ServiceNodeQuorumValidationError> {
        validate_version("ServiceNodeSignatureV1", self.version)?;
        validate_token("chain_id", &self.chain_id, MAX_CHAIN_ID_BYTES)?;
        validate_token("epoch_id", &self.epoch_id, MAX_EPOCH_ID_BYTES)?;
        validate_service_node_id(&self.service_node_id)?;
        validate_token("key_id", &self.key_id, MAX_REF_BYTES)?;
        validate_nonempty("signature_wire", &self.signature_wire, MAX_SIGNATURE_BYTES)
    }
}

/// Canonical structural quorum for one ROC epoch transition.
///
/// Acceptance here means only that the DTO is internally coherent and has the
/// required number of unique, eligible signer references. Later coordinator
/// work must cryptographically verify every signature before describing the
/// transition as quorum-approved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeQuorumV1 {
    /// Stable wire schema.
    pub schema: String,

    /// DTO version.
    pub version: u16,

    /// Chain/domain identifier.
    pub chain_id: String,

    /// Epoch identifier.
    pub epoch_id: String,

    /// Canonical hash of the proposed epoch transition.
    pub transition_hash: ContentId,

    /// Deterministic threshold parameters.
    pub threshold: EpochQuorumThresholdV1,

    /// Canonically sorted eligible membership set.
    pub eligibilities: Vec<EpochEligibilityV1>,

    /// Canonically sorted unique signer set.
    pub signatures: Vec<ServiceNodeSignatureV1>,
}

impl ServiceNodeQuorumV1 {
    /// Validate deterministic membership and signature references.
    ///
    /// This method intentionally does not verify cryptographic signatures and
    /// does not create wallet, ledger, mint, payout, or finality authority.
    pub fn validate(&self) -> Result<(), ServiceNodeQuorumValidationError> {
        if self.schema != SERVICE_NODE_QUORUM_SCHEMA {
            return invalid(
                "schema",
                "must match the Phase 15 Service Node quorum schema",
            );
        }

        validate_version("ServiceNodeQuorumV1", self.version)?;
        validate_token("chain_id", &self.chain_id, MAX_CHAIN_ID_BYTES)?;
        validate_token("epoch_id", &self.epoch_id, MAX_EPOCH_ID_BYTES)?;
        self.threshold.validate()?;

        if self.eligibilities.is_empty() {
            return invalid("eligibilities", "must not be empty");
        }

        if self.eligibilities.len() > MAX_SERVICE_NODE_QUORUM_MEMBERS {
            return Err(ServiceNodeQuorumValidationError::TooManyItems {
                field: "eligibilities",
                max: MAX_SERVICE_NODE_QUORUM_MEMBERS,
                actual: self.eligibilities.len(),
            });
        }

        if usize::from(self.threshold.eligible_service_nodes) != self.eligibilities.len() {
            return invalid(
                "threshold.eligible_service_nodes",
                "must match eligibilities length",
            );
        }

        let mut eligible_keys = BTreeMap::new();
        let mut registry_entries = BTreeSet::new();
        let mut reward_bindings = BTreeSet::new();

        for eligibility in &self.eligibilities {
            eligibility.validate()?;

            if eligibility.status != EpochEligibilityStatusV1::Eligible {
                return invalid("eligibilities.status", "quorum membership must be eligible");
            }

            if eligible_keys
                .insert(
                    eligibility.service_node_id.as_str(),
                    eligibility.key_id.as_str(),
                )
                .is_some()
            {
                return Err(ServiceNodeQuorumValidationError::Duplicate {
                    field: "eligibilities.service_node_id",
                });
            }

            if !registry_entries.insert(eligibility.registry_entry_id.as_str()) {
                return Err(ServiceNodeQuorumValidationError::Duplicate {
                    field: "eligibilities.registry_entry_id",
                });
            }

            if !reward_bindings.insert(eligibility.reward_binding_id.as_str()) {
                return Err(ServiceNodeQuorumValidationError::Duplicate {
                    field: "eligibilities.reward_binding_id",
                });
            }
        }

        if self
            .eligibilities
            .windows(2)
            .any(|pair| pair[0].service_node_id >= pair[1].service_node_id)
        {
            return invalid(
                "eligibilities.service_node_id",
                "must be unique and sorted ascending",
            );
        }

        if self.signatures.len() > MAX_SERVICE_NODE_QUORUM_MEMBERS {
            return Err(ServiceNodeQuorumValidationError::TooManyItems {
                field: "signatures",
                max: MAX_SERVICE_NODE_QUORUM_MEMBERS,
                actual: self.signatures.len(),
            });
        }

        let required = usize::from(self.threshold.required_signatures);

        if self.signatures.len() < required {
            return Err(ServiceNodeQuorumValidationError::InsufficientQuorum {
                required,
                actual: self.signatures.len(),
            });
        }

        let mut signers = BTreeSet::new();

        for signature in &self.signatures {
            signature.validate()?;

            if !signers.insert(signature.service_node_id.as_str()) {
                return Err(ServiceNodeQuorumValidationError::Duplicate {
                    field: "signatures.service_node_id",
                });
            }

            if signature.chain_id != self.chain_id {
                return Err(ServiceNodeQuorumValidationError::Mismatch {
                    field: "signatures.chain_id",
                });
            }

            if signature.epoch_id != self.epoch_id {
                return Err(ServiceNodeQuorumValidationError::Mismatch {
                    field: "signatures.epoch_id",
                });
            }

            if signature.transition_hash != self.transition_hash {
                return Err(ServiceNodeQuorumValidationError::Mismatch {
                    field: "signatures.transition_hash",
                });
            }

            let Some(expected_key_id) = eligible_keys.get(signature.service_node_id.as_str())
            else {
                return invalid(
                    "signatures.service_node_id",
                    "signature must come from the eligibility set",
                );
            };

            if signature.key_id.as_str() != *expected_key_id {
                return Err(ServiceNodeQuorumValidationError::Mismatch {
                    field: "signatures.key_id",
                });
            }
        }

        if self
            .signatures
            .windows(2)
            .any(|pair| pair[0].service_node_id >= pair[1].service_node_id)
        {
            return invalid(
                "signatures.service_node_id",
                "must be unique and sorted ascending",
            );
        }

        Ok(())
    }
}

fn validate_version(ty: &'static str, actual: u16) -> Result<(), ServiceNodeQuorumValidationError> {
    if actual == SERVICE_NODE_QUORUM_VERSION {
        Ok(())
    } else {
        Err(ServiceNodeQuorumValidationError::InvalidVersion {
            ty,
            expected: SERVICE_NODE_QUORUM_VERSION,
            actual,
        })
    }
}

fn validate_service_node_id(value: &str) -> Result<(), ServiceNodeQuorumValidationError> {
    validate_token("service_node_id", value, MAX_REF_BYTES)?;

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
    max: usize,
) -> Result<(), ServiceNodeQuorumValidationError> {
    validate_nonempty(field, value, max)?;

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
    }) {
        return invalid(field, "contains unsupported characters");
    }

    Ok(())
}

fn validate_nonempty(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), ServiceNodeQuorumValidationError> {
    if value.trim().is_empty() {
        return invalid(field, "must not be empty");
    }

    if value.len() > max {
        return invalid(field, "exceeds maximum byte length");
    }

    Ok(())
}

fn invalid<T>(
    field: &'static str,
    reason: &'static str,
) -> Result<T, ServiceNodeQuorumValidationError> {
    Err(ServiceNodeQuorumValidationError::InvalidField { field, reason })
}
