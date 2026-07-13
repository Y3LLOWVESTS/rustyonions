//! RO:WHAT — Strict range-request and repair service-evidence contracts.
//! RO:WHY — Phase 13 needs witnessed partial delivery and repair evidence.
//! RO:INTERACTS — challenge acknowledgments, macronode review, later accounting.
//! RO:INVARIANTS — exact ranges; distinct repair actors; stable replay keys.
//! RO:SECURITY — no IP fields, provider-only claims, payout, wallet, or ledger authority.
//! RO:TEST — tests/service_node_range_repair_evidence.rs.

#![forbid(unsafe_code)]

use crate::id::ContentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    ChallengeEvidenceKindV1, ServiceChallengeAckV1, ServiceChallengeAckValidationError,
    SERVICE_EVIDENCE_VERSION,
};

pub const RANGE_REQUEST_PROOF_SCHEMA: &str = "ron.service_node.range_request_proof.v1";

pub const REPAIR_PROOF_SCHEMA: &str = "ron.service_node.repair_proof.v1";

pub const RANGE_REQUEST_SIGNING_DOMAIN: &[u8] = b"ron.service_node.range_request_proof.v1\0";

pub const REPAIR_SIGNING_DOMAIN: &[u8] = b"ron.service_node.repair_proof.v1\0";

const MAX_TOKEN_BYTES: usize = 512;
const B3_TEXT_BYTES: usize = 67;

/// Validation failures for range-request evidence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RangeRequestProofValidationError {
    #[error("invalid range-request schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid range-request version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("range-request evidence requires an independent witness")]
    ProviderOnlyClaim,

    #[error("self-traffic range requests are not accepted")]
    SelfTraffic,

    #[error("service observation and witness acknowledgment must be distinct")]
    WitnessReferenceCollision,

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("range request overflows the u64 object offset space")]
    RangeOverflow,

    #[error("range length mismatch: requested={requested}, returned={returned}")]
    RangeLengthMismatch { requested: u64, returned: u64 },

    #[error("{field} must be a canonical lowercase b3 digest")]
    InvalidRangeDigest { field: &'static str },

    #[error("observed range digest does not match the expected digest")]
    RangeDigestMismatch,

    #[error("range response completion must not precede request start")]
    InvalidTimeOrder,

    #[error("range-request authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

/// Validation failures for repair evidence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RepairProofValidationError {
    #[error("invalid repair schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid repair version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("repair evidence requires an independent witness")]
    ProviderOnlyClaim,

    #[error("repair actors must be distinct: {left} collides with {right}")]
    ActorCollision {
        left: &'static str,
        right: &'static str,
    },

    #[error("repair observation references must be pairwise distinct")]
    ObservationReferenceCollision,

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("repair completion must not precede repair start")]
    InvalidTimeOrder,

    #[error("repair source content must pass content-address verification")]
    UnverifiedSource,

    #[error("repaired target content must pass content-address verification")]
    UnverifiedTarget,

    #[error("repair evidence cannot claim success before the local object is restored")]
    LocalObjectNotRestored,

    #[error("repair authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

/// Stable range-request replay identity.
///
/// `proof_id` is intentionally excluded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeRequestProofReplayKeyV1 {
    pub service_node_id: String,
    pub witness_node_id: String,
    pub request_id: String,
    pub request_nonce: String,
    pub content_id: ContentId,
    pub range_start: u64,
    pub range_length: u64,
}

/// Stable repair replay identity.
///
/// `proof_id` is intentionally excluded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepairProofReplayKeyV1 {
    pub repairing_service_node_id: String,
    pub source_service_node_id: String,
    pub witness_node_id: String,
    pub repair_id: String,
    pub repair_nonce: String,
    pub content_id: ContentId,
}

/// Independently witnessed exact byte-range response.
///
/// The range has its own expected and observed BLAKE3 digest. This evidence
/// does not claim that the entire object was delivered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RangeRequestProofV1 {
    pub schema: String,
    pub version: u16,
    pub proof_id: String,

    pub service_node_id: String,
    pub witness_node_id: String,

    pub request_id: String,
    pub request_nonce: String,
    pub content_id: ContentId,

    pub service_observation_ref: String,
    pub witness_ack_ref: String,

    pub range_start: u64,
    pub range_length: u64,
    pub bytes_returned: u64,

    pub expected_range_b3: String,
    pub observed_range_b3: String,

    pub request_started_at_ms: u64,
    pub response_completed_at_ms: u64,

    pub evidence_only: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Independently witnessed local repair evidence.
///
/// This proves a bounded source-to-target restoration attempt whose source
/// and restored target both passed content-address verification. It does not
/// prove network-wide repair, provider publication, or payout acceptance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepairProofV1 {
    pub schema: String,
    pub version: u16,
    pub proof_id: String,

    pub repairing_service_node_id: String,
    pub source_service_node_id: String,
    pub witness_node_id: String,

    pub repair_id: String,
    pub repair_nonce: String,
    pub content_id: ContentId,

    pub source_observation_ref: String,
    pub target_observation_ref: String,
    pub witness_ack_ref: String,

    pub bytes_repaired: u64,

    pub repair_started_at_ms: u64,
    pub repair_completed_at_ms: u64,

    pub source_content_verified: bool,
    pub target_content_verified: bool,
    pub local_object_restored: bool,

    pub evidence_only: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

impl RangeRequestProofV1 {
    pub fn validate(&self) -> Result<(), RangeRequestProofValidationError> {
        if self.schema != RANGE_REQUEST_PROOF_SCHEMA {
            return Err(RangeRequestProofValidationError::InvalidSchema {
                expected: RANGE_REQUEST_PROOF_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_EVIDENCE_VERSION {
            return Err(RangeRequestProofValidationError::InvalidVersion {
                expected: SERVICE_EVIDENCE_VERSION,
                actual: self.version,
            });
        }

        validate_range_token("proof_id", &self.proof_id)?;
        validate_range_token("service_node_id", &self.service_node_id)?;

        if self.witness_node_id.trim().is_empty() || self.witness_ack_ref.trim().is_empty() {
            return Err(RangeRequestProofValidationError::ProviderOnlyClaim);
        }

        validate_range_token("witness_node_id", &self.witness_node_id)?;
        validate_range_token("request_id", &self.request_id)?;
        validate_range_token("request_nonce", &self.request_nonce)?;
        validate_range_token("service_observation_ref", &self.service_observation_ref)?;
        validate_range_token("witness_ack_ref", &self.witness_ack_ref)?;

        if self.service_node_id == self.witness_node_id {
            return Err(RangeRequestProofValidationError::SelfTraffic);
        }

        if self.service_observation_ref == self.witness_ack_ref {
            return Err(RangeRequestProofValidationError::WitnessReferenceCollision);
        }

        if self.range_length == 0 {
            return Err(RangeRequestProofValidationError::ZeroValue {
                field: "range_length",
            });
        }

        if self.range_start.checked_add(self.range_length).is_none() {
            return Err(RangeRequestProofValidationError::RangeOverflow);
        }

        if self.bytes_returned != self.range_length {
            return Err(RangeRequestProofValidationError::RangeLengthMismatch {
                requested: self.range_length,
                returned: self.bytes_returned,
            });
        }

        validate_b3("expected_range_b3", &self.expected_range_b3)?;
        validate_b3("observed_range_b3", &self.observed_range_b3)?;

        if self.expected_range_b3 != self.observed_range_b3 {
            return Err(RangeRequestProofValidationError::RangeDigestMismatch);
        }

        if self.request_started_at_ms == 0 {
            return Err(RangeRequestProofValidationError::ZeroValue {
                field: "request_started_at_ms",
            });
        }

        if self.response_completed_at_ms == 0 {
            return Err(RangeRequestProofValidationError::ZeroValue {
                field: "response_completed_at_ms",
            });
        }

        if self.response_completed_at_ms < self.request_started_at_ms {
            return Err(RangeRequestProofValidationError::InvalidTimeOrder);
        }

        validate_range_authority(self)
    }

    pub fn replay_key(&self) -> RangeRequestProofReplayKeyV1 {
        RangeRequestProofReplayKeyV1 {
            service_node_id: self.service_node_id.clone(),
            witness_node_id: self.witness_node_id.clone(),
            request_id: self.request_id.clone(),
            request_nonce: self.request_nonce.clone(),
            content_id: self.content_id.clone(),
            range_start: self.range_start,
            range_length: self.range_length,
        }
    }

    pub fn witness_ack_signing_bytes(&self, witness_key_id: &str) -> Vec<u8> {
        let content_id = self.content_id.to_string();
        let mut out = Vec::with_capacity(640);

        out.extend_from_slice(RANGE_REQUEST_SIGNING_DOMAIN);

        append_signing_string(&mut out, &self.schema);
        out.extend_from_slice(&self.version.to_be_bytes());
        append_signing_string(&mut out, &self.proof_id);
        append_signing_string(&mut out, &self.service_node_id);
        append_signing_string(&mut out, &self.witness_node_id);
        append_signing_string(&mut out, &self.request_id);
        append_signing_string(&mut out, &self.request_nonce);
        append_signing_string(&mut out, &content_id);
        append_signing_string(&mut out, &self.service_observation_ref);
        append_signing_string(&mut out, &self.witness_ack_ref);
        append_signing_string(&mut out, witness_key_id);

        out.extend_from_slice(&self.range_start.to_be_bytes());
        out.extend_from_slice(&self.range_length.to_be_bytes());
        out.extend_from_slice(&self.bytes_returned.to_be_bytes());

        append_signing_string(&mut out, &self.expected_range_b3);
        append_signing_string(&mut out, &self.observed_range_b3);

        out.extend_from_slice(&self.request_started_at_ms.to_be_bytes());
        out.extend_from_slice(&self.response_completed_at_ms.to_be_bytes());

        append_authority_bytes(
            &mut out,
            self.evidence_only,
            self.reward_truth,
            self.payout_authority,
            self.wallet_mutation,
            self.ledger_mutation,
        );

        out
    }
}

impl RepairProofV1 {
    pub fn validate(&self) -> Result<(), RepairProofValidationError> {
        if self.schema != REPAIR_PROOF_SCHEMA {
            return Err(RepairProofValidationError::InvalidSchema {
                expected: REPAIR_PROOF_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_EVIDENCE_VERSION {
            return Err(RepairProofValidationError::InvalidVersion {
                expected: SERVICE_EVIDENCE_VERSION,
                actual: self.version,
            });
        }

        validate_repair_token("proof_id", &self.proof_id)?;
        validate_repair_token("repairing_service_node_id", &self.repairing_service_node_id)?;
        validate_repair_token("source_service_node_id", &self.source_service_node_id)?;

        if self.witness_node_id.trim().is_empty() || self.witness_ack_ref.trim().is_empty() {
            return Err(RepairProofValidationError::ProviderOnlyClaim);
        }

        validate_repair_token("witness_node_id", &self.witness_node_id)?;
        validate_repair_token("repair_id", &self.repair_id)?;
        validate_repair_token("repair_nonce", &self.repair_nonce)?;
        validate_repair_token("source_observation_ref", &self.source_observation_ref)?;
        validate_repair_token("target_observation_ref", &self.target_observation_ref)?;
        validate_repair_token("witness_ack_ref", &self.witness_ack_ref)?;

        validate_distinct_actor(
            "repairing_service_node_id",
            &self.repairing_service_node_id,
            "source_service_node_id",
            &self.source_service_node_id,
        )?;

        validate_distinct_actor(
            "repairing_service_node_id",
            &self.repairing_service_node_id,
            "witness_node_id",
            &self.witness_node_id,
        )?;

        validate_distinct_actor(
            "source_service_node_id",
            &self.source_service_node_id,
            "witness_node_id",
            &self.witness_node_id,
        )?;

        if self.source_observation_ref == self.target_observation_ref
            || self.source_observation_ref == self.witness_ack_ref
            || self.target_observation_ref == self.witness_ack_ref
        {
            return Err(RepairProofValidationError::ObservationReferenceCollision);
        }

        if self.bytes_repaired == 0 {
            return Err(RepairProofValidationError::ZeroValue {
                field: "bytes_repaired",
            });
        }

        if self.repair_started_at_ms == 0 {
            return Err(RepairProofValidationError::ZeroValue {
                field: "repair_started_at_ms",
            });
        }

        if self.repair_completed_at_ms == 0 {
            return Err(RepairProofValidationError::ZeroValue {
                field: "repair_completed_at_ms",
            });
        }

        if self.repair_completed_at_ms < self.repair_started_at_ms {
            return Err(RepairProofValidationError::InvalidTimeOrder);
        }

        if !self.source_content_verified {
            return Err(RepairProofValidationError::UnverifiedSource);
        }

        if !self.target_content_verified {
            return Err(RepairProofValidationError::UnverifiedTarget);
        }

        if !self.local_object_restored {
            return Err(RepairProofValidationError::LocalObjectNotRestored);
        }

        validate_repair_authority(self)
    }

    pub fn replay_key(&self) -> RepairProofReplayKeyV1 {
        RepairProofReplayKeyV1 {
            repairing_service_node_id: self.repairing_service_node_id.clone(),
            source_service_node_id: self.source_service_node_id.clone(),
            witness_node_id: self.witness_node_id.clone(),
            repair_id: self.repair_id.clone(),
            repair_nonce: self.repair_nonce.clone(),
            content_id: self.content_id.clone(),
        }
    }

    pub fn witness_ack_signing_bytes(&self, witness_key_id: &str) -> Vec<u8> {
        let content_id = self.content_id.to_string();
        let mut out = Vec::with_capacity(768);

        out.extend_from_slice(REPAIR_SIGNING_DOMAIN);

        append_signing_string(&mut out, &self.schema);
        out.extend_from_slice(&self.version.to_be_bytes());
        append_signing_string(&mut out, &self.proof_id);
        append_signing_string(&mut out, &self.repairing_service_node_id);
        append_signing_string(&mut out, &self.source_service_node_id);
        append_signing_string(&mut out, &self.witness_node_id);
        append_signing_string(&mut out, &self.repair_id);
        append_signing_string(&mut out, &self.repair_nonce);
        append_signing_string(&mut out, &content_id);
        append_signing_string(&mut out, &self.source_observation_ref);
        append_signing_string(&mut out, &self.target_observation_ref);
        append_signing_string(&mut out, &self.witness_ack_ref);
        append_signing_string(&mut out, witness_key_id);

        out.extend_from_slice(&self.bytes_repaired.to_be_bytes());
        out.extend_from_slice(&self.repair_started_at_ms.to_be_bytes());
        out.extend_from_slice(&self.repair_completed_at_ms.to_be_bytes());

        for value in [
            self.source_content_verified,
            self.target_content_verified,
            self.local_object_restored,
        ] {
            out.push(u8::from(value));
        }

        append_authority_bytes(
            &mut out,
            self.evidence_only,
            self.reward_truth,
            self.payout_authority,
            self.wallet_mutation,
            self.ledger_mutation,
        );

        out
    }
}

impl ServiceChallengeAckV1 {
    pub fn validate_for_range_request(
        &self,
        proof: &RangeRequestProofV1,
    ) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
        self.validate_for_binding(
            ChallengeEvidenceKindV1::RangeRequest,
            &proof.witness_ack_ref,
            &proof.witness_node_id,
        )
    }

    pub fn validate_for_repair(
        &self,
        proof: &RepairProofV1,
    ) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
        self.validate_for_binding(
            ChallengeEvidenceKindV1::Repair,
            &proof.witness_ack_ref,
            &proof.witness_node_id,
        )
    }
}

fn validate_range_authority(
    proof: &RangeRequestProofV1,
) -> Result<(), RangeRequestProofValidationError> {
    if !proof.evidence_only {
        return Err(RangeRequestProofValidationError::AuthorityBoundary {
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
            return Err(RangeRequestProofValidationError::AuthorityBoundary { field });
        }
    }

    Ok(())
}

fn validate_repair_authority(proof: &RepairProofV1) -> Result<(), RepairProofValidationError> {
    if !proof.evidence_only {
        return Err(RepairProofValidationError::AuthorityBoundary {
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
            return Err(RepairProofValidationError::AuthorityBoundary { field });
        }
    }

    Ok(())
}

fn validate_distinct_actor(
    left_name: &'static str,
    left_value: &str,
    right_name: &'static str,
    right_value: &str,
) -> Result<(), RepairProofValidationError> {
    if left_value == right_value {
        return Err(RepairProofValidationError::ActorCollision {
            left: left_name,
            right: right_name,
        });
    }

    Ok(())
}

fn validate_range_token(
    field: &'static str,
    value: &str,
) -> Result<(), RangeRequestProofValidationError> {
    if !valid_token(value) {
        return Err(RangeRequestProofValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_repair_token(
    field: &'static str,
    value: &str,
) -> Result<(), RepairProofValidationError> {
    if !valid_token(value) {
        return Err(RepairProofValidationError::InvalidToken { field });
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

fn validate_b3(field: &'static str, value: &str) -> Result<(), RangeRequestProofValidationError> {
    let valid = value.len() == B3_TEXT_BYTES
        && value.starts_with("b3:")
        && value.as_bytes()[3..].iter().all(u8::is_ascii_hexdigit)
        && value.as_bytes()[3..]
            .iter()
            .all(|byte| !byte.is_ascii_alphabetic() || byte.is_ascii_lowercase());

    if !valid {
        return Err(RangeRequestProofValidationError::InvalidRangeDigest { field });
    }

    Ok(())
}

fn append_signing_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn append_authority_bytes(
    out: &mut Vec<u8>,
    evidence_only: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
) {
    for value in [
        evidence_only,
        reward_truth,
        payout_authority,
        wallet_mutation,
        ledger_mutation,
    ] {
        out.push(u8::from(value));
    }
}
