//! RO:WHAT — Strict Phase 15 ROC epoch-transition and invalid-epoch challenge DTOs.
//! RO:WHY — ECON/GOV: bind reward plans to reviewed roots and a Service Node quorum before execution.
//! RO:INTERACTS — service_node::quorum, ron-accounting snapshots, svc-rewarder plans, registry bindings.
//! RO:INVARIANTS — exact root binding; canonical allocations; checked caps; duplicate-plan rows reject.
//! RO:METRICS — none; DTO and deterministic local validation only.
//! RO:CONFIG — reward cap and quorum context arrive as reviewed inputs; no local mutable rates.
//! RO:SECURITY — no payout recipient override, signature shortcut, wallet mutation, or ledger mutation.
//! RO:TEST — tests/internal_roc_beta_phase15_epoch_transition.rs.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    EpochEligibilityStatusV1, EpochEligibilityV1, EpochQuorumThresholdV1, ServiceNodeQuorumV1,
    ServiceNodeSignatureV1,
};
use crate::{id::ContentId, quantum::SignatureAlg};

/// Current Phase 15 epoch-transition DTO version.
pub const ROC_EPOCH_TRANSITION_VERSION: u16 = 1;

/// Schema for one ROC epoch transition.
pub const ROC_EPOCH_TRANSITION_SCHEMA: &str = "ron.service_node.epoch-transition.v1";

/// Schema for one reward-plan allocation carried into an epoch transition.
pub const EPOCH_REWARD_ALLOCATION_SCHEMA: &str = "ron.service_node.epoch-reward-allocation.v1";

/// Schema for independently reviewed transition expectations.
pub const ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA: &str =
    "ron.service_node.epoch-transition-expectation.v1";

/// Historical transition-identity domain retained byte-for-byte for compatibility.
///
/// The `phase22` token is a legacy protocol label from the original private-beta
/// reward-loop implementation. It is not the current FINAL_BETA build phase.
pub const ROC_EPOCH_TRANSITION_IDENTITY_DOMAIN: &str = "rustyonions.phase22.epoch-transition.v1";

/// Historical BLAKE3 domain retained for consumers that hash the validated identity.
///
/// `ron-proto` owns the bytes contract only and does not perform hashing or signing.
pub const ROC_EPOCH_TRANSITION_HASH_DOMAIN: &str = "phase22.epoch-transition.v1";

/// Schema for one invalid-epoch challenge.
pub const INVALID_EPOCH_CHALLENGE_SCHEMA: &str = "ron.service_node.invalid-epoch-challenge.v1";

/// Maximum allocation rows in one transition.
pub const MAX_ROC_EPOCH_ALLOCATIONS: usize = 4_096;

/// Return the exact canonical message bytes signed by one service node.
///
/// `signature_wire` is deliberately excluded from its own signing preimage.
/// This function defines bytes only; signature-algorithm acceptance and key
/// verification remain the responsibility of the consuming verifier.
///
/// # Errors
///
/// Returns a serialization error only if the fixed canonical message shape
/// cannot be encoded.
pub fn service_node_signature_message_bytes(
    signature: &ServiceNodeSignatureV1,
) -> Result<Vec<u8>, serde_json::Error> {
    #[derive(Serialize)]
    struct SignatureMessage<'a> {
        domain: &'static str,
        version: u16,
        chain_id: &'a str,
        epoch_id: &'a str,
        service_node_id: &'a str,
        key_id: &'a str,
        algorithm: SignatureAlg,
        transition_hash: &'a ContentId,
    }

    serde_json::to_vec(&SignatureMessage {
        domain: "rustyonions.service-node-epoch-signature.v1",
        version: signature.version,
        chain_id: &signature.chain_id,
        epoch_id: &signature.epoch_id,
        service_node_id: &signature.service_node_id,
        key_id: &signature.key_id,
        algorithm: signature.algorithm,
        transition_hash: &signature.transition_hash,
    })
}

const MAX_CHAIN_ID_BYTES: usize = 64;
const MAX_EPOCH_ID_BYTES: usize = 128;
const MAX_REF_BYTES: usize = 256;
const MAX_MINOR_UNIT_DIGITS: usize = 39;

/// Deterministic validation errors for Phase 15 epoch transitions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RocEpochTransitionValidationError {
    #[error("invalid schema for {field}: expected {expected}, got {actual}")]
    InvalidSchema {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },

    #[error("invalid version for {field}: expected {expected}, got {actual}")]
    InvalidVersion {
        field: &'static str,
        expected: u16,
        actual: u16,
    },

    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },

    #[error("{field} exceeds maximum bytes: max={max}, actual={actual}")]
    FieldTooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },

    #[error("{field} contains unsupported characters")]
    InvalidToken { field: &'static str },

    #[error("too many items for {field}: max={max}, actual={actual}")]
    TooManyItems {
        field: &'static str,
        max: usize,
        actual: usize,
    },

    #[error("invalid minor-unit amount for {field}: {reason}")]
    InvalidMoney {
        field: &'static str,
        reason: &'static str,
    },

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("{field} must be unique and canonically sorted")]
    NonCanonical { field: &'static str },

    #[error("duplicate transition material rejected: {field}")]
    Duplicate { field: &'static str },

    #[error("transition binding mismatch: {field}")]
    Mismatch { field: &'static str },

    #[error("ineligible transition identity: {field}")]
    Ineligible { field: &'static str },

    #[error("invalid Service Node quorum: {reason}")]
    InvalidQuorum { reason: String },

    #[error("reward cap overflow: cap={cap}, total={total}")]
    CapOverflow { cap: u128, total: u128 },

    #[error("allocation sum mismatch: declared={declared}, calculated={calculated}")]
    AllocationSumMismatch { declared: u128, calculated: u128 },

    #[error("checked arithmetic overflow: {field}")]
    ArithmeticOverflow { field: &'static str },
}

/// One reward-plan allocation proposed for an epoch transition.
///
/// Recipient accounts are intentionally absent. A later wallet/ledger phase
/// resolves `service_node_id` through accepted registry and reward-binding roots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpochRewardAllocationV1 {
    pub schema: String,
    pub version: u16,

    /// Transition-local allocation identity.
    pub allocation_id: String,

    /// Identity of the exact allocation in the accepted reward plan.
    pub reward_plan_allocation_id: String,

    pub service_node_id: String,
    pub source_pool: String,

    /// Canonical positive decimal integer.
    pub amount_minor_units: String,
}

impl EpochRewardAllocationV1 {
    /// Validate allocation shape without resolving a payout recipient.
    pub fn validate(&self) -> Result<(), RocEpochTransitionValidationError> {
        validate_schema(
            "EpochRewardAllocationV1.schema",
            &self.schema,
            EPOCH_REWARD_ALLOCATION_SCHEMA,
        )?;
        validate_version("EpochRewardAllocationV1.version", self.version)?;

        validate_token("allocation_id", &self.allocation_id, MAX_REF_BYTES)?;
        validate_token(
            "reward_plan_allocation_id",
            &self.reward_plan_allocation_id,
            MAX_REF_BYTES,
        )?;
        validate_service_node_id(&self.service_node_id)?;
        validate_token("source_pool", &self.source_pool, MAX_REF_BYTES)?;

        if parse_minor_units("amount_minor_units", &self.amount_minor_units)? == 0 {
            return Err(RocEpochTransitionValidationError::ZeroValue {
                field: "amount_minor_units",
            });
        }

        Ok(())
    }
}

/// Independently supplied roots, cap, threshold, and eligibility context.
///
/// This prevents a transition from selecting its own policy, economics,
/// registry, reward-binding, eligibility, threshold, or cap values and then
/// validating itself against those same untrusted values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RocEpochTransitionExpectationV1 {
    pub schema: String,
    pub version: u16,

    pub chain_id: String,
    pub epoch_id: String,

    pub accounting_snapshot_hash: ContentId,
    pub reward_plan_hash: ContentId,
    pub policy_hash: ContentId,
    pub economics_config_hash: ContentId,
    pub registry_root: ContentId,
    pub reward_binding_root: ContentId,
    pub evidence_root: ContentId,

    pub reward_cap_minor_units: String,

    pub threshold: EpochQuorumThresholdV1,
    pub eligibilities: Vec<EpochEligibilityV1>,
}

impl RocEpochTransitionExpectationV1 {
    /// Validate independently reviewed transition context.
    pub fn validate(&self) -> Result<(), RocEpochTransitionValidationError> {
        validate_schema(
            "RocEpochTransitionExpectationV1.schema",
            &self.schema,
            ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA,
        )?;
        validate_version("RocEpochTransitionExpectationV1.version", self.version)?;

        validate_token("chain_id", &self.chain_id, MAX_CHAIN_ID_BYTES)?;
        validate_token("epoch_id", &self.epoch_id, MAX_EPOCH_ID_BYTES)?;

        if parse_minor_units("reward_cap_minor_units", &self.reward_cap_minor_units)? == 0 {
            return Err(RocEpochTransitionValidationError::ZeroValue {
                field: "reward_cap_minor_units",
            });
        }

        self.threshold.validate().map_err(|error| {
            RocEpochTransitionValidationError::InvalidQuorum {
                reason: error.to_string(),
            }
        })?;

        if usize::from(self.threshold.eligible_service_nodes) != self.eligibilities.len() {
            return Err(RocEpochTransitionValidationError::Mismatch {
                field: "threshold.eligible_service_nodes",
            });
        }

        let mut service_nodes = BTreeSet::new();
        let mut registry_entries = BTreeSet::new();
        let mut reward_bindings = BTreeSet::new();
        let mut key_ids = BTreeSet::new();

        for eligibility in &self.eligibilities {
            eligibility.validate().map_err(|error| {
                RocEpochTransitionValidationError::InvalidQuorum {
                    reason: error.to_string(),
                }
            })?;

            if eligibility.status != EpochEligibilityStatusV1::Eligible {
                return Err(RocEpochTransitionValidationError::Ineligible {
                    field: "eligibilities.status",
                });
            }

            if !service_nodes.insert(eligibility.service_node_id.as_str()) {
                return Err(RocEpochTransitionValidationError::Duplicate {
                    field: "eligibilities.service_node_id",
                });
            }

            if !registry_entries.insert(eligibility.registry_entry_id.as_str()) {
                return Err(RocEpochTransitionValidationError::Duplicate {
                    field: "eligibilities.registry_entry_id",
                });
            }

            if !reward_bindings.insert(eligibility.reward_binding_id.as_str()) {
                return Err(RocEpochTransitionValidationError::Duplicate {
                    field: "eligibilities.reward_binding_id",
                });
            }

            if !key_ids.insert(eligibility.key_id.as_str()) {
                return Err(RocEpochTransitionValidationError::Duplicate {
                    field: "eligibilities.key_id",
                });
            }
        }

        ensure_eligibilities_are_canonical(&self.eligibilities)
    }
}

/// Canonical immutable identity that Service Nodes review before signing.
///
/// This structure deliberately contains no transition hash, signatures,
/// production timestamp, recipient account, wallet instruction, or ledger
/// mutation. Its serialized field order preserves the historical private-beta
/// transition preimage exactly so all consumers can derive one identical digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RocEpochTransitionIdentityV1 {
    pub domain: String,
    pub chain_id: String,
    pub epoch_id: String,

    pub accounting_snapshot_hash: ContentId,
    pub reward_plan_hash: ContentId,
    pub policy_hash: ContentId,
    pub economics_config_hash: ContentId,
    pub registry_root: ContentId,
    pub reward_binding_root: ContentId,
    pub evidence_root: ContentId,

    pub reward_cap_minor_units: String,
    pub reward_total_minor_units: String,

    pub allocations: Vec<EpochRewardAllocationV1>,
    pub threshold: EpochQuorumThresholdV1,
    pub eligibilities: Vec<EpochEligibilityV1>,
}

impl RocEpochTransitionIdentityV1 {
    /// Construct canonical pre-sign identity from independently reviewed context
    /// plus the deterministic reward allocations proposed for the epoch.
    #[must_use]
    pub fn from_expectation_and_allocations(
        expected: &RocEpochTransitionExpectationV1,
        reward_total_minor_units: impl Into<String>,
        allocations: Vec<EpochRewardAllocationV1>,
    ) -> Self {
        Self {
            domain: ROC_EPOCH_TRANSITION_IDENTITY_DOMAIN.to_owned(),
            chain_id: expected.chain_id.clone(),
            epoch_id: expected.epoch_id.clone(),
            accounting_snapshot_hash: expected.accounting_snapshot_hash.clone(),
            reward_plan_hash: expected.reward_plan_hash.clone(),
            policy_hash: expected.policy_hash.clone(),
            economics_config_hash: expected.economics_config_hash.clone(),
            registry_root: expected.registry_root.clone(),
            reward_binding_root: expected.reward_binding_root.clone(),
            evidence_root: expected.evidence_root.clone(),
            reward_cap_minor_units: expected.reward_cap_minor_units.clone(),
            reward_total_minor_units: reward_total_minor_units.into(),
            allocations,
            threshold: expected.threshold.clone(),
            eligibilities: expected.eligibilities.clone(),
        }
    }

    /// Validate exactly the reviewed context and reward material that may enter
    /// the transition digest before any Service Node signature is produced.
    pub fn validate(&self) -> Result<(), RocEpochTransitionValidationError> {
        if self.domain != ROC_EPOCH_TRANSITION_IDENTITY_DOMAIN {
            return Err(RocEpochTransitionValidationError::Mismatch { field: "domain" });
        }

        let expected = RocEpochTransitionExpectationV1 {
            schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),
            version: ROC_EPOCH_TRANSITION_VERSION,
            chain_id: self.chain_id.clone(),
            epoch_id: self.epoch_id.clone(),
            accounting_snapshot_hash: self.accounting_snapshot_hash.clone(),
            reward_plan_hash: self.reward_plan_hash.clone(),
            policy_hash: self.policy_hash.clone(),
            economics_config_hash: self.economics_config_hash.clone(),
            registry_root: self.registry_root.clone(),
            reward_binding_root: self.reward_binding_root.clone(),
            evidence_root: self.evidence_root.clone(),
            reward_cap_minor_units: self.reward_cap_minor_units.clone(),
            threshold: self.threshold.clone(),
            eligibilities: self.eligibilities.clone(),
        };

        expected.validate()?;

        validate_transition_reward_material(
            &self.reward_cap_minor_units,
            &self.reward_total_minor_units,
            &self.allocations,
        )?;

        validate_allocation_eligibility(&self.allocations, &self.eligibilities)
    }
}

/// Validate the reward material shared by the pre-sign identity and the final
/// quorum-bound transition. This remains deterministic DTO validation only.
fn validate_transition_reward_material(
    reward_cap_minor_units: &str,
    reward_total_minor_units: &str,
    allocations: &[EpochRewardAllocationV1],
) -> Result<(), RocEpochTransitionValidationError> {
    if allocations.is_empty() {
        return Err(RocEpochTransitionValidationError::EmptyField {
            field: "allocations",
        });
    }

    if allocations.len() > MAX_ROC_EPOCH_ALLOCATIONS {
        return Err(RocEpochTransitionValidationError::TooManyItems {
            field: "allocations",
            max: MAX_ROC_EPOCH_ALLOCATIONS,
            actual: allocations.len(),
        });
    }

    let reward_cap = parse_minor_units("reward_cap_minor_units", reward_cap_minor_units)?;

    let reward_total = parse_minor_units("reward_total_minor_units", reward_total_minor_units)?;

    if reward_cap == 0 {
        return Err(RocEpochTransitionValidationError::ZeroValue {
            field: "reward_cap_minor_units",
        });
    }

    if reward_total == 0 {
        return Err(RocEpochTransitionValidationError::ZeroValue {
            field: "reward_total_minor_units",
        });
    }

    if reward_total > reward_cap {
        return Err(RocEpochTransitionValidationError::CapOverflow {
            cap: reward_cap,
            total: reward_total,
        });
    }

    let mut allocation_ids = BTreeSet::new();
    let mut reward_plan_allocation_ids = BTreeSet::new();
    let mut calculated_total = 0_u128;

    for allocation in allocations {
        allocation.validate()?;

        if !allocation_ids.insert(allocation.allocation_id.as_str()) {
            return Err(RocEpochTransitionValidationError::Duplicate {
                field: "allocations.allocation_id",
            });
        }

        if !reward_plan_allocation_ids.insert(allocation.reward_plan_allocation_id.as_str()) {
            return Err(RocEpochTransitionValidationError::Duplicate {
                field: "allocations.reward_plan_allocation_id",
            });
        }

        let amount = parse_minor_units("amount_minor_units", &allocation.amount_minor_units)?;

        calculated_total = calculated_total.checked_add(amount).ok_or(
            RocEpochTransitionValidationError::ArithmeticOverflow {
                field: "allocation_sum",
            },
        )?;
    }

    ensure_allocations_are_canonical(allocations)?;

    if calculated_total != reward_total {
        return Err(RocEpochTransitionValidationError::AllocationSumMismatch {
            declared: reward_total,
            calculated: calculated_total,
        });
    }

    Ok(())
}

/// Ensure every proposed allocation belongs to a reviewed eligible Service Node.
fn validate_allocation_eligibility(
    allocations: &[EpochRewardAllocationV1],
    eligibilities: &[EpochEligibilityV1],
) -> Result<(), RocEpochTransitionValidationError> {
    let eligible_nodes = eligibilities
        .iter()
        .map(|eligibility| eligibility.service_node_id.as_str())
        .collect::<BTreeSet<_>>();

    for allocation in allocations {
        if !eligible_nodes.contains(allocation.service_node_id.as_str()) {
            return Err(RocEpochTransitionValidationError::Ineligible {
                field: "allocations.service_node_id",
            });
        }
    }

    Ok(())
}

/// Deterministic Phase 15 ROC epoch-transition model.
///
/// The transition is quorum-reviewed planning material. It does not resolve
/// recipient accounts, mutate wallet or ledger state, or create receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RocEpochTransitionV1 {
    pub schema: String,
    pub version: u16,

    pub chain_id: String,
    pub epoch_id: String,

    /// Digest signed by the quorum after canonical transition construction.
    pub transition_hash: ContentId,

    pub accounting_snapshot_hash: ContentId,
    pub reward_plan_hash: ContentId,
    pub policy_hash: ContentId,
    pub economics_config_hash: ContentId,
    pub registry_root: ContentId,
    pub reward_binding_root: ContentId,
    pub evidence_root: ContentId,

    pub reward_cap_minor_units: String,
    pub reward_total_minor_units: String,

    pub allocations: Vec<EpochRewardAllocationV1>,
    pub quorum: ServiceNodeQuorumV1,

    pub produced_at_ms: u64,
}

impl RocEpochTransitionV1 {
    /// Validate cap conservation, allocation uniqueness, canonical ordering,
    /// eligibility, and quorum binding.
    pub fn validate(&self) -> Result<(), RocEpochTransitionValidationError> {
        validate_schema(
            "RocEpochTransitionV1.schema",
            &self.schema,
            ROC_EPOCH_TRANSITION_SCHEMA,
        )?;
        validate_version("RocEpochTransitionV1.version", self.version)?;

        validate_token("chain_id", &self.chain_id, MAX_CHAIN_ID_BYTES)?;
        validate_token("epoch_id", &self.epoch_id, MAX_EPOCH_ID_BYTES)?;

        if self.produced_at_ms == 0 {
            return Err(RocEpochTransitionValidationError::ZeroValue {
                field: "produced_at_ms",
            });
        }

        validate_transition_reward_material(
            &self.reward_cap_minor_units,
            &self.reward_total_minor_units,
            &self.allocations,
        )?;

        self.quorum.validate().map_err(|error| {
            RocEpochTransitionValidationError::InvalidQuorum {
                reason: error.to_string(),
            }
        })?;

        if self.quorum.chain_id != self.chain_id {
            return Err(RocEpochTransitionValidationError::Mismatch {
                field: "quorum.chain_id",
            });
        }

        if self.quorum.epoch_id != self.epoch_id {
            return Err(RocEpochTransitionValidationError::Mismatch {
                field: "quorum.epoch_id",
            });
        }

        if self.quorum.transition_hash != self.transition_hash {
            return Err(RocEpochTransitionValidationError::Mismatch {
                field: "quorum.transition_hash",
            });
        }

        validate_allocation_eligibility(&self.allocations, &self.quorum.eligibilities)?;

        Ok(())
    }

    /// Compare the transition against independently supplied reviewed inputs.
    pub fn validate_against(
        &self,
        expected: &RocEpochTransitionExpectationV1,
    ) -> Result<(), RocEpochTransitionValidationError> {
        self.validate()?;
        expected.validate()?;

        ensure_string_match("chain_id", &self.chain_id, &expected.chain_id)?;
        ensure_string_match("epoch_id", &self.epoch_id, &expected.epoch_id)?;

        ensure_hash_match(
            "accounting_snapshot_hash",
            &self.accounting_snapshot_hash,
            &expected.accounting_snapshot_hash,
        )?;
        ensure_hash_match(
            "reward_plan_hash",
            &self.reward_plan_hash,
            &expected.reward_plan_hash,
        )?;
        ensure_hash_match("policy_hash", &self.policy_hash, &expected.policy_hash)?;
        ensure_hash_match(
            "economics_config_hash",
            &self.economics_config_hash,
            &expected.economics_config_hash,
        )?;
        ensure_hash_match(
            "registry_root",
            &self.registry_root,
            &expected.registry_root,
        )?;
        ensure_hash_match(
            "reward_binding_root",
            &self.reward_binding_root,
            &expected.reward_binding_root,
        )?;
        ensure_hash_match(
            "evidence_root",
            &self.evidence_root,
            &expected.evidence_root,
        )?;

        ensure_string_match(
            "reward_cap_minor_units",
            &self.reward_cap_minor_units,
            &expected.reward_cap_minor_units,
        )?;

        if self.quorum.threshold != expected.threshold {
            return Err(RocEpochTransitionValidationError::Mismatch {
                field: "quorum.threshold",
            });
        }

        if self.quorum.eligibilities != expected.eligibilities {
            return Err(RocEpochTransitionValidationError::Mismatch {
                field: "quorum.eligibilities",
            });
        }

        Ok(())
    }
}

/// Deterministic invalid-epoch challenge category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InvalidEpochChallengeKindV1 {
    InvalidAccountingSnapshot,
    InvalidRewardPlan,
    InvalidPolicyHash,
    InvalidEconomicsConfigHash,
    InvalidRegistryRoot,
    InvalidRewardBindingRoot,
    InvalidEvidenceRoot,
    CapOverflow,
    DuplicateAllocation,
    InvalidEligibilitySet,
    InsufficientQuorum,
    InvalidServiceNodeSignature,
}

/// Challenge material describing an invalid epoch transition.
///
/// Validation here checks only deterministic shape. Challenge acceptance,
/// dispute resolution, quarantine, and economic effects belong to later phases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidEpochChallengeV1 {
    pub schema: String,
    pub version: u16,

    pub challenge_id: String,
    pub chain_id: String,
    pub epoch_id: String,

    pub transition_hash: ContentId,
    pub challenger_id: String,
    pub challenge_kind: InvalidEpochChallengeKindV1,
    pub evidence_hash: ContentId,

    pub submitted_at_ms: u64,
}

impl InvalidEpochChallengeV1 {
    /// Validate challenge shape without claiming challenge acceptance.
    pub fn validate(&self) -> Result<(), RocEpochTransitionValidationError> {
        validate_schema(
            "InvalidEpochChallengeV1.schema",
            &self.schema,
            INVALID_EPOCH_CHALLENGE_SCHEMA,
        )?;
        validate_version("InvalidEpochChallengeV1.version", self.version)?;

        validate_token("challenge_id", &self.challenge_id, MAX_REF_BYTES)?;
        validate_token("chain_id", &self.chain_id, MAX_CHAIN_ID_BYTES)?;
        validate_token("epoch_id", &self.epoch_id, MAX_EPOCH_ID_BYTES)?;
        validate_token("challenger_id", &self.challenger_id, MAX_REF_BYTES)?;

        if self.submitted_at_ms == 0 {
            return Err(RocEpochTransitionValidationError::ZeroValue {
                field: "submitted_at_ms",
            });
        }

        Ok(())
    }
}

fn validate_schema(
    field: &'static str,
    actual: &str,
    expected: &'static str,
) -> Result<(), RocEpochTransitionValidationError> {
    if actual == expected {
        return Ok(());
    }

    Err(RocEpochTransitionValidationError::InvalidSchema {
        field,
        expected,
        actual: actual.to_owned(),
    })
}

fn validate_version(
    field: &'static str,
    actual: u16,
) -> Result<(), RocEpochTransitionValidationError> {
    if actual == ROC_EPOCH_TRANSITION_VERSION {
        return Ok(());
    }

    Err(RocEpochTransitionValidationError::InvalidVersion {
        field,
        expected: ROC_EPOCH_TRANSITION_VERSION,
        actual,
    })
}

fn validate_service_node_id(value: &str) -> Result<(), RocEpochTransitionValidationError> {
    validate_bounded_nonempty("service_node_id", value, MAX_REF_BYTES)?;

    let Some(suffix) = value.strip_prefix("service_node:") else {
        return Err(RocEpochTransitionValidationError::InvalidToken {
            field: "service_node_id",
        });
    };

    if suffix.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.')
        })
    {
        return Err(RocEpochTransitionValidationError::InvalidToken {
            field: "service_node_id",
        });
    }

    Ok(())
}

fn validate_token(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), RocEpochTransitionValidationError> {
    validate_bounded_nonempty(field, value, max)?;

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
    }) {
        return Err(RocEpochTransitionValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_bounded_nonempty(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), RocEpochTransitionValidationError> {
    if value.trim().is_empty() {
        return Err(RocEpochTransitionValidationError::EmptyField { field });
    }

    if value.len() > max {
        return Err(RocEpochTransitionValidationError::FieldTooLong {
            field,
            max,
            actual: value.len(),
        });
    }

    Ok(())
}

fn ensure_allocations_are_canonical(
    allocations: &[EpochRewardAllocationV1],
) -> Result<(), RocEpochTransitionValidationError> {
    if allocations.windows(2).any(|pair| {
        (
            pair[0].service_node_id.as_str(),
            pair[0].source_pool.as_str(),
            pair[0].reward_plan_allocation_id.as_str(),
            pair[0].allocation_id.as_str(),
        ) >= (
            pair[1].service_node_id.as_str(),
            pair[1].source_pool.as_str(),
            pair[1].reward_plan_allocation_id.as_str(),
            pair[1].allocation_id.as_str(),
        )
    }) {
        return Err(RocEpochTransitionValidationError::NonCanonical {
            field: "allocations",
        });
    }

    Ok(())
}

fn ensure_eligibilities_are_canonical(
    eligibilities: &[EpochEligibilityV1],
) -> Result<(), RocEpochTransitionValidationError> {
    if eligibilities
        .windows(2)
        .any(|pair| pair[0].service_node_id.as_str() >= pair[1].service_node_id.as_str())
    {
        return Err(RocEpochTransitionValidationError::NonCanonical {
            field: "eligibilities.service_node_id",
        });
    }

    Ok(())
}

fn parse_minor_units(
    field: &'static str,
    value: &str,
) -> Result<u128, RocEpochTransitionValidationError> {
    if value.is_empty() {
        return Err(RocEpochTransitionValidationError::InvalidMoney {
            field,
            reason: "must not be empty",
        });
    }

    if value.len() > MAX_MINOR_UNIT_DIGITS {
        return Err(RocEpochTransitionValidationError::InvalidMoney {
            field,
            reason: "must not exceed u128 decimal width",
        });
    }

    if value.len() > 1 && value.starts_with('0') {
        return Err(RocEpochTransitionValidationError::InvalidMoney {
            field,
            reason: "must not contain leading zeroes",
        });
    }

    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RocEpochTransitionValidationError::InvalidMoney {
            field,
            reason: "must contain decimal digits only",
        });
    }

    value
        .parse::<u128>()
        .map_err(|_| RocEpochTransitionValidationError::InvalidMoney {
            field,
            reason: "must fit in u128 minor units",
        })
}

fn ensure_hash_match(
    field: &'static str,
    actual: &ContentId,
    expected: &ContentId,
) -> Result<(), RocEpochTransitionValidationError> {
    if actual == expected {
        return Ok(());
    }

    Err(RocEpochTransitionValidationError::Mismatch { field })
}

fn ensure_string_match(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), RocEpochTransitionValidationError> {
    if actual == expected {
        return Ok(());
    }

    Err(RocEpochTransitionValidationError::Mismatch { field })
}
