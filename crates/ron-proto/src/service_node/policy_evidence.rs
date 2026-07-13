//! RO:WHAT — Strict policy-refusal and moderation-action evidence contracts.
//! RO:WHY — Phase 13 must record policy enforcement without creating payout truth.
//! RO:INTERACTS — moderation policy, macronode review, signed witness acknowledgments.
//! RO:INVARIANTS — denied bytes remain unserved; moderation actions remain local evidence.
//! RO:SECURITY — no requester IP, reward eligibility, payout, wallet, or ledger authority.
//! RO:TEST — tests/service_node_policy_evidence.rs.

#![forbid(unsafe_code)]

use crate::id::ContentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    ChallengeEvidenceKindV1, ServiceChallengeAckV1, ServiceChallengeAckValidationError,
    SERVICE_EVIDENCE_VERSION,
};

pub const POLICY_REFUSAL_PROOF_SCHEMA: &str = "ron.service_node.policy_refusal_proof.v1";

pub const MODERATION_ACTION_PROOF_SCHEMA: &str = "ron.service_node.moderation_action_proof.v1";

pub const POLICY_REFUSAL_SIGNING_DOMAIN: &[u8] = b"ron.service_node.policy_refusal_proof.v1\0";

pub const MODERATION_ACTION_SIGNING_DOMAIN: &[u8] =
    b"ron.service_node.moderation_action_proof.v1\0";

const MAX_TOKEN_BYTES: usize = 512;

/// Canonical policy reasons that prohibit object serving.
///
/// Local allow and no-rule states are deliberately absent because they
/// cannot honestly justify a refusal proof.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRefusalReasonV1 {
    GlobalDeny,
    OwnerTombstone,
    LocalBlock,
    Quarantine,
}

/// Local operator moderation action.
///
/// Prune is deliberately absent because prune completion has separate
/// storage/provider/index outcomes and must not be implied by this DTO.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModerationActionKindV1 {
    LocalBlock,
    LocalUnblock,
    LocalAllow,
    LocalRemoveAllow,
    Quarantine,
    ReleaseQuarantine,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PolicyRefusalProofValidationError {
    #[error("invalid policy-refusal schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid policy-refusal version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("policy-refusal evidence requires an independent requester acknowledgment")]
    ProviderOnlyClaim,

    #[error("self-traffic policy-refusal evidence is not accepted")]
    SelfTraffic,

    #[error("policy-refusal observation and requester acknowledgment must be distinct")]
    ObservationReferenceCollision,

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("refusal timestamp must not precede request timestamp")]
    InvalidTimeOrder,

    #[error("policy-refusal evidence requires policy_enforced=true")]
    PolicyNotEnforced,

    #[error("policy-refusal evidence cannot claim that content was served")]
    ContentServed,

    #[error("policy-refusal evidence cannot report served bytes: {bytes_served}")]
    BytesServed { bytes_served: u64 },

    #[error("policy-refusal evidence cannot claim direct reward eligibility")]
    RewardEligible,

    #[error("policy-refusal authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ModerationActionProofValidationError {
    #[error("invalid moderation-action schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid moderation-action version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("moderation-action evidence requires an independent witness")]
    ProviderOnlyClaim,

    #[error("moderation actors must be distinct: {left} collides with {right}")]
    ActorCollision {
        left: &'static str,
        right: &'static str,
    },

    #[error("moderation policy snapshots and witness references must be distinct")]
    ReferenceCollision,

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("moderation-action evidence requires a real changed=true action")]
    NoChange,

    #[error("moderation-action evidence cannot claim runtime hot reload")]
    RuntimeHotReloadClaim,

    #[error("moderation-action evidence cannot claim storage deletion")]
    StorageDeleteClaim,

    #[error("moderation-action evidence cannot claim provider withdrawal")]
    ProviderWithdrawalClaim,

    #[error("moderation-action evidence cannot claim network propagation")]
    NetworkPropagationClaim,

    #[error("moderation-action evidence cannot claim direct reward eligibility")]
    RewardEligible,

    #[error("moderation-action authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

/// Stable replay identity for one refused request.
///
/// `proof_id` is intentionally excluded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyRefusalProofReplayKeyV1 {
    pub service_node_id: String,
    pub requester_node_id: String,
    pub request_id: String,
    pub request_nonce: String,
    pub content_id: ContentId,
}

/// Stable replay identity for one local moderation action.
///
/// `proof_id` is intentionally excluded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModerationActionProofReplayKeyV1 {
    pub service_node_id: String,
    pub operator_subject_id: String,
    pub witness_node_id: String,
    pub action_id: String,
    pub action_nonce: String,
    pub content_id: ContentId,
}

/// Evidence that a service node refused a request because the effective
/// moderation policy prohibited serving the exact content object.
///
/// This records enforcement behavior only. It is not a direct reward claim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyRefusalProofV1 {
    pub schema: String,
    pub version: u16,
    pub proof_id: String,

    pub service_node_id: String,
    pub requester_node_id: String,

    pub request_id: String,
    pub request_nonce: String,
    pub content_id: ContentId,

    pub policy_snapshot_ref: String,
    pub service_observation_ref: String,
    pub requester_ack_ref: String,

    pub refusal_reason: PolicyRefusalReasonV1,

    pub requested_at_ms: u64,
    pub refused_at_ms: u64,

    pub policy_enforced: bool,
    pub content_served: bool,
    pub bytes_served: u64,

    pub evidence_only: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Evidence that a local operator moderation action changed policy metadata.
///
/// This does not prove runtime activation, storage deletion, provider
/// withdrawal, network propagation, accounting acceptance, or reward value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModerationActionProofV1 {
    pub schema: String,
    pub version: u16,
    pub proof_id: String,

    pub service_node_id: String,
    pub operator_subject_id: String,
    pub witness_node_id: String,

    pub action_id: String,
    pub action_nonce: String,
    pub content_id: ContentId,

    pub policy_snapshot_before_ref: String,
    pub policy_snapshot_after_ref: String,
    pub action_observation_ref: String,
    pub witness_ack_ref: String,

    pub action: ModerationActionKindV1,
    pub action_recorded_at_ms: u64,

    pub changed: bool,
    pub runtime_hot_reload: bool,
    pub storage_delete: bool,
    pub provider_withdrawal: bool,
    pub network_propagation: bool,

    pub evidence_only: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

impl PolicyRefusalProofV1 {
    pub fn validate(&self) -> Result<(), PolicyRefusalProofValidationError> {
        if self.schema != POLICY_REFUSAL_PROOF_SCHEMA {
            return Err(PolicyRefusalProofValidationError::InvalidSchema {
                expected: POLICY_REFUSAL_PROOF_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_EVIDENCE_VERSION {
            return Err(PolicyRefusalProofValidationError::InvalidVersion {
                expected: SERVICE_EVIDENCE_VERSION,
                actual: self.version,
            });
        }

        validate_refusal_token("proof_id", &self.proof_id)?;
        validate_refusal_token("service_node_id", &self.service_node_id)?;

        if self.requester_node_id.trim().is_empty() || self.requester_ack_ref.trim().is_empty() {
            return Err(PolicyRefusalProofValidationError::ProviderOnlyClaim);
        }

        validate_refusal_token("requester_node_id", &self.requester_node_id)?;
        validate_refusal_token("request_id", &self.request_id)?;
        validate_refusal_token("request_nonce", &self.request_nonce)?;
        validate_refusal_token("policy_snapshot_ref", &self.policy_snapshot_ref)?;
        validate_refusal_token("service_observation_ref", &self.service_observation_ref)?;
        validate_refusal_token("requester_ack_ref", &self.requester_ack_ref)?;

        if self.service_node_id == self.requester_node_id {
            return Err(PolicyRefusalProofValidationError::SelfTraffic);
        }

        if self.service_observation_ref == self.requester_ack_ref {
            return Err(PolicyRefusalProofValidationError::ObservationReferenceCollision);
        }

        if self.requested_at_ms == 0 {
            return Err(PolicyRefusalProofValidationError::ZeroValue {
                field: "requested_at_ms",
            });
        }

        if self.refused_at_ms == 0 {
            return Err(PolicyRefusalProofValidationError::ZeroValue {
                field: "refused_at_ms",
            });
        }

        if self.refused_at_ms < self.requested_at_ms {
            return Err(PolicyRefusalProofValidationError::InvalidTimeOrder);
        }

        if !self.policy_enforced {
            return Err(PolicyRefusalProofValidationError::PolicyNotEnforced);
        }

        if self.content_served {
            return Err(PolicyRefusalProofValidationError::ContentServed);
        }

        if self.bytes_served != 0 {
            return Err(PolicyRefusalProofValidationError::BytesServed {
                bytes_served: self.bytes_served,
            });
        }

        if self.reward_eligible {
            return Err(PolicyRefusalProofValidationError::RewardEligible);
        }

        validate_refusal_authority(self)
    }

    pub fn replay_key(&self) -> PolicyRefusalProofReplayKeyV1 {
        PolicyRefusalProofReplayKeyV1 {
            service_node_id: self.service_node_id.clone(),
            requester_node_id: self.requester_node_id.clone(),
            request_id: self.request_id.clone(),
            request_nonce: self.request_nonce.clone(),
            content_id: self.content_id.clone(),
        }
    }

    pub fn requester_ack_signing_bytes(&self, requester_key_id: &str) -> Vec<u8> {
        let content_id = self.content_id.to_string();
        let mut out = Vec::with_capacity(768);

        out.extend_from_slice(POLICY_REFUSAL_SIGNING_DOMAIN);

        append_signing_string(&mut out, &self.schema);
        out.extend_from_slice(&self.version.to_be_bytes());
        append_signing_string(&mut out, &self.proof_id);
        append_signing_string(&mut out, &self.service_node_id);
        append_signing_string(&mut out, &self.requester_node_id);
        append_signing_string(&mut out, &self.request_id);
        append_signing_string(&mut out, &self.request_nonce);
        append_signing_string(&mut out, &content_id);
        append_signing_string(&mut out, &self.policy_snapshot_ref);
        append_signing_string(&mut out, &self.service_observation_ref);
        append_signing_string(&mut out, &self.requester_ack_ref);
        append_signing_string(&mut out, requester_key_id);

        out.push(match self.refusal_reason {
            PolicyRefusalReasonV1::GlobalDeny => 1,
            PolicyRefusalReasonV1::OwnerTombstone => 2,
            PolicyRefusalReasonV1::LocalBlock => 3,
            PolicyRefusalReasonV1::Quarantine => 4,
        });

        out.extend_from_slice(&self.requested_at_ms.to_be_bytes());
        out.extend_from_slice(&self.refused_at_ms.to_be_bytes());
        out.extend_from_slice(&self.bytes_served.to_be_bytes());

        append_bool_bytes(
            &mut out,
            &[
                self.policy_enforced,
                self.content_served,
                self.evidence_only,
                self.reward_eligible,
                self.reward_truth,
                self.payout_authority,
                self.wallet_mutation,
                self.ledger_mutation,
            ],
        );

        out
    }
}

impl ModerationActionProofV1 {
    pub fn validate(&self) -> Result<(), ModerationActionProofValidationError> {
        if self.schema != MODERATION_ACTION_PROOF_SCHEMA {
            return Err(ModerationActionProofValidationError::InvalidSchema {
                expected: MODERATION_ACTION_PROOF_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_EVIDENCE_VERSION {
            return Err(ModerationActionProofValidationError::InvalidVersion {
                expected: SERVICE_EVIDENCE_VERSION,
                actual: self.version,
            });
        }

        validate_action_token("proof_id", &self.proof_id)?;
        validate_action_token("service_node_id", &self.service_node_id)?;
        validate_action_token("operator_subject_id", &self.operator_subject_id)?;

        if self.witness_node_id.trim().is_empty() || self.witness_ack_ref.trim().is_empty() {
            return Err(ModerationActionProofValidationError::ProviderOnlyClaim);
        }

        validate_action_token("witness_node_id", &self.witness_node_id)?;
        validate_action_token("action_id", &self.action_id)?;
        validate_action_token("action_nonce", &self.action_nonce)?;
        validate_action_token(
            "policy_snapshot_before_ref",
            &self.policy_snapshot_before_ref,
        )?;
        validate_action_token("policy_snapshot_after_ref", &self.policy_snapshot_after_ref)?;
        validate_action_token("action_observation_ref", &self.action_observation_ref)?;
        validate_action_token("witness_ack_ref", &self.witness_ack_ref)?;

        validate_distinct_actor(
            "service_node_id",
            &self.service_node_id,
            "operator_subject_id",
            &self.operator_subject_id,
        )?;

        validate_distinct_actor(
            "service_node_id",
            &self.service_node_id,
            "witness_node_id",
            &self.witness_node_id,
        )?;

        validate_distinct_actor(
            "operator_subject_id",
            &self.operator_subject_id,
            "witness_node_id",
            &self.witness_node_id,
        )?;

        if self.policy_snapshot_before_ref == self.policy_snapshot_after_ref
            || self.policy_snapshot_before_ref == self.witness_ack_ref
            || self.policy_snapshot_after_ref == self.witness_ack_ref
            || self.action_observation_ref == self.witness_ack_ref
        {
            return Err(ModerationActionProofValidationError::ReferenceCollision);
        }

        if self.action_recorded_at_ms == 0 {
            return Err(ModerationActionProofValidationError::ZeroValue {
                field: "action_recorded_at_ms",
            });
        }

        if !self.changed {
            return Err(ModerationActionProofValidationError::NoChange);
        }

        if self.runtime_hot_reload {
            return Err(ModerationActionProofValidationError::RuntimeHotReloadClaim);
        }

        if self.storage_delete {
            return Err(ModerationActionProofValidationError::StorageDeleteClaim);
        }

        if self.provider_withdrawal {
            return Err(ModerationActionProofValidationError::ProviderWithdrawalClaim);
        }

        if self.network_propagation {
            return Err(ModerationActionProofValidationError::NetworkPropagationClaim);
        }

        if self.reward_eligible {
            return Err(ModerationActionProofValidationError::RewardEligible);
        }

        validate_action_authority(self)
    }

    pub fn replay_key(&self) -> ModerationActionProofReplayKeyV1 {
        ModerationActionProofReplayKeyV1 {
            service_node_id: self.service_node_id.clone(),
            operator_subject_id: self.operator_subject_id.clone(),
            witness_node_id: self.witness_node_id.clone(),
            action_id: self.action_id.clone(),
            action_nonce: self.action_nonce.clone(),
            content_id: self.content_id.clone(),
        }
    }

    pub fn witness_ack_signing_bytes(&self, witness_key_id: &str) -> Vec<u8> {
        let content_id = self.content_id.to_string();
        let mut out = Vec::with_capacity(896);

        out.extend_from_slice(MODERATION_ACTION_SIGNING_DOMAIN);

        append_signing_string(&mut out, &self.schema);
        out.extend_from_slice(&self.version.to_be_bytes());
        append_signing_string(&mut out, &self.proof_id);
        append_signing_string(&mut out, &self.service_node_id);
        append_signing_string(&mut out, &self.operator_subject_id);
        append_signing_string(&mut out, &self.witness_node_id);
        append_signing_string(&mut out, &self.action_id);
        append_signing_string(&mut out, &self.action_nonce);
        append_signing_string(&mut out, &content_id);
        append_signing_string(&mut out, &self.policy_snapshot_before_ref);
        append_signing_string(&mut out, &self.policy_snapshot_after_ref);
        append_signing_string(&mut out, &self.action_observation_ref);
        append_signing_string(&mut out, &self.witness_ack_ref);
        append_signing_string(&mut out, witness_key_id);

        out.push(match self.action {
            ModerationActionKindV1::LocalBlock => 1,
            ModerationActionKindV1::LocalUnblock => 2,
            ModerationActionKindV1::LocalAllow => 3,
            ModerationActionKindV1::LocalRemoveAllow => 4,
            ModerationActionKindV1::Quarantine => 5,
            ModerationActionKindV1::ReleaseQuarantine => 6,
        });

        out.extend_from_slice(&self.action_recorded_at_ms.to_be_bytes());

        append_bool_bytes(
            &mut out,
            &[
                self.changed,
                self.runtime_hot_reload,
                self.storage_delete,
                self.provider_withdrawal,
                self.network_propagation,
                self.evidence_only,
                self.reward_eligible,
                self.reward_truth,
                self.payout_authority,
                self.wallet_mutation,
                self.ledger_mutation,
            ],
        );

        out
    }
}

impl ServiceChallengeAckV1 {
    pub fn validate_for_policy_refusal(
        &self,
        proof: &PolicyRefusalProofV1,
    ) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
        self.validate_for_binding(
            ChallengeEvidenceKindV1::PolicyRefusal,
            &proof.requester_ack_ref,
            &proof.requester_node_id,
        )
    }

    pub fn validate_for_moderation_action(
        &self,
        proof: &ModerationActionProofV1,
    ) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
        self.validate_for_binding(
            ChallengeEvidenceKindV1::ModerationAction,
            &proof.witness_ack_ref,
            &proof.witness_node_id,
        )
    }
}

fn validate_refusal_authority(
    proof: &PolicyRefusalProofV1,
) -> Result<(), PolicyRefusalProofValidationError> {
    if !proof.evidence_only {
        return Err(PolicyRefusalProofValidationError::AuthorityBoundary {
            field: "evidence_only",
        });
    }

    for (field, value) in [
        ("reward_truth", proof.reward_truth),
        ("payout_authority", proof.payout_authority),
        ("wallet_mutation", proof.wallet_mutation),
        ("ledger_mutation", proof.ledger_mutation),
    ] {
        if value {
            return Err(PolicyRefusalProofValidationError::AuthorityBoundary { field });
        }
    }

    Ok(())
}

fn validate_action_authority(
    proof: &ModerationActionProofV1,
) -> Result<(), ModerationActionProofValidationError> {
    if !proof.evidence_only {
        return Err(ModerationActionProofValidationError::AuthorityBoundary {
            field: "evidence_only",
        });
    }

    for (field, value) in [
        ("reward_truth", proof.reward_truth),
        ("payout_authority", proof.payout_authority),
        ("wallet_mutation", proof.wallet_mutation),
        ("ledger_mutation", proof.ledger_mutation),
    ] {
        if value {
            return Err(ModerationActionProofValidationError::AuthorityBoundary { field });
        }
    }

    Ok(())
}

fn validate_distinct_actor(
    left_name: &'static str,
    left_value: &str,
    right_name: &'static str,
    right_value: &str,
) -> Result<(), ModerationActionProofValidationError> {
    if left_value == right_value {
        return Err(ModerationActionProofValidationError::ActorCollision {
            left: left_name,
            right: right_name,
        });
    }

    Ok(())
}

fn validate_refusal_token(
    field: &'static str,
    value: &str,
) -> Result<(), PolicyRefusalProofValidationError> {
    if !valid_token(value) {
        return Err(PolicyRefusalProofValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_action_token(
    field: &'static str,
    value: &str,
) -> Result<(), ModerationActionProofValidationError> {
    if !valid_token(value) {
        return Err(ModerationActionProofValidationError::InvalidToken { field });
    }

    Ok(())
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b':')
        })
}

fn append_signing_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn append_bool_bytes(out: &mut Vec<u8>, values: &[bool]) {
    for value in values {
        out.push(u8::from(*value));
    }
}
