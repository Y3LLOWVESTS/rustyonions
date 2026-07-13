//! RO:WHAT — Strict availability and hot-cache service-evidence contracts.
//! RO:WHY — Phase 13 needs independently witnessed service evidence.
//! RO:INTERACTS — macronode review, ron-kms signatures, later accounting.
//! RO:INVARIANTS — witness required; self-traffic rejected; replay keys stable.
//! RO:SECURITY — no IP addresses, payout targets, wallet, or ledger authority.
//! RO:TEST — tests/service_node_availability_hot_cache_evidence.rs.

use crate::id::ContentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::SERVICE_EVIDENCE_VERSION;

pub const AVAILABILITY_PROOF_SCHEMA: &str = "ron.service_node.availability_proof.v1";

pub const HOT_CACHE_PROOF_SCHEMA: &str = "ron.service_node.hot_cache_proof.v1";

pub const SERVICE_CHALLENGE_ACK_SCHEMA: &str = "ron.service_node.challenge_ack.v1";

pub const AVAILABILITY_SIGNING_DOMAIN: &[u8] = b"ron.service_node.availability_proof.v1\0";

pub const HOT_CACHE_SIGNING_DOMAIN: &[u8] = b"ron.service_node.hot_cache_proof.v1\0";

const MAX_TOKEN_BYTES: usize = 512;
const ED25519_SIGNATURE_HEX_BYTES: usize = 128;

/// Evidence type bound by a challenge acknowledgment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeEvidenceKindV1 {
    Availability,
    HotCache,
    RangeRequest,
    Repair,
    PolicyRefusal,
    ModerationAction,
}

/// Cache tier that may honestly produce `HotCacheProofV1`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HotCacheTierV1 {
    /// Process-local memory-backed hot cache.
    Memory,
}

/// Shared validation failures for availability and hot-cache proofs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ChallengeEvidenceValidationError {
    #[error("invalid evidence schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid evidence version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("availability evidence requires an independent witness and acknowledgment")]
    ProviderOnlyClaim,

    #[error("self-traffic challenge evidence is not accepted")]
    SelfTraffic,

    #[error("service observation and witness acknowledgment must be distinct")]
    WitnessReferenceCollision,

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("response completion must not precede challenge issuance")]
    InvalidTimeOrder,

    #[error("returned sample length mismatch: requested={requested}, returned={returned}")]
    SampleLengthMismatch { requested: u64, returned: u64 },

    #[error("challenge evidence requires full content-address verification")]
    UnverifiedContent,

    #[error("hot-cache evidence cannot be created for a cache miss")]
    HotCacheMiss,

    #[error("hot-cache evidence cannot claim an origin fetch")]
    OriginFetchPerformed,

    #[error(
        "reported latency exceeds observed challenge window: latency_us={latency_us}, window_us={window_us}"
    )]
    LatencyExceedsWindow { latency_us: u64, window_us: u64 },

    #[error("challenge evidence authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

/// Validation failures for detached witness acknowledgments.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceChallengeAckValidationError {
    #[error("invalid challenge acknowledgment schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid challenge acknowledgment version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("challenge acknowledgment kind mismatch: expected {expected:?}, got {actual:?}")]
    KindMismatch {
        expected: ChallengeEvidenceKindV1,
        actual: ChallengeEvidenceKindV1,
    },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("challenge acknowledgment does not match proof field {field}")]
    BindingMismatch { field: &'static str },

    #[error(
        "challenge signature must contain 128 lowercase hexadecimal characters; actual={actual}"
    )]
    InvalidSignatureLength { actual: usize },

    #[error("challenge signature must contain lowercase hexadecimal characters only")]
    InvalidSignatureHex,

    #[error("challenge acknowledgment authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

/// Stable replay identity for one availability challenge.
///
/// `proof_id` is intentionally excluded so changing a producer-assigned
/// identifier cannot evade duplicate detection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AvailabilityProofReplayKeyV1 {
    pub service_node_id: String,
    pub witness_node_id: String,
    pub challenge_id: String,
    pub challenge_nonce: String,
    pub content_id: ContentId,
}

/// Stable replay identity for one hot-cache challenge.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HotCacheProofReplayKeyV1 {
    pub service_node_id: String,
    pub witness_node_id: String,
    pub challenge_id: String,
    pub challenge_nonce: String,
    pub content_id: ContentId,
}

/// Independently witnessed evidence that a service node answered an exact
/// content-availability challenge.
///
/// This does not prove long-term durability, reward eligibility, payout
/// approval, wallet mutation, or ledger acceptance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityProofV1 {
    pub schema: String,
    pub version: u16,
    pub proof_id: String,

    pub service_node_id: String,
    pub witness_node_id: String,

    pub challenge_id: String,
    pub challenge_nonce: String,
    pub content_id: ContentId,

    pub service_observation_ref: String,
    pub witness_ack_ref: String,

    pub requested_sample_offset: u64,
    pub requested_sample_length: u64,
    pub bytes_returned: u64,

    pub challenge_issued_at_ms: u64,
    pub response_completed_at_ms: u64,

    pub content_verified: bool,

    pub evidence_only: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Independently witnessed evidence that a verified response came directly
/// from a process-local hot cache without an origin fetch.
///
/// This does not prove durable storage or establish a reward amount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HotCacheProofV1 {
    pub schema: String,
    pub version: u16,
    pub proof_id: String,

    pub service_node_id: String,
    pub witness_node_id: String,

    pub challenge_id: String,
    pub challenge_nonce: String,
    pub content_id: ContentId,

    pub service_observation_ref: String,
    pub witness_ack_ref: String,

    pub bytes_returned: u64,
    pub challenge_issued_at_ms: u64,
    pub response_completed_at_ms: u64,
    pub response_latency_micros: u64,

    pub cache_tier: HotCacheTierV1,
    pub cache_hit: bool,
    pub origin_fetch_performed: bool,
    pub content_verified: bool,

    pub evidence_only: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Detached witness acknowledgment.
///
/// The trusted public key is deliberately absent. Runtime verification must
/// resolve it from `witness_node_id` and `witness_key_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceChallengeAckV1 {
    pub schema: String,
    pub version: u16,
    pub evidence_kind: ChallengeEvidenceKindV1,

    pub ack_id: String,
    pub witness_node_id: String,
    pub witness_key_id: String,
    pub signature_hex: String,

    pub evidence_only: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

impl AvailabilityProofV1 {
    pub fn validate(&self) -> Result<(), ChallengeEvidenceValidationError> {
        validate_common_proof(&CommonProofFields {
            schema: &self.schema,
            expected_schema: AVAILABILITY_PROOF_SCHEMA,
            version: self.version,
            proof_id: &self.proof_id,
            service_node_id: &self.service_node_id,
            witness_node_id: &self.witness_node_id,
            challenge_id: &self.challenge_id,
            challenge_nonce: &self.challenge_nonce,
            service_observation_ref: &self.service_observation_ref,
            witness_ack_ref: &self.witness_ack_ref,
            challenge_issued_at_ms: self.challenge_issued_at_ms,
            response_completed_at_ms: self.response_completed_at_ms,
            content_verified: self.content_verified,
            evidence_only: self.evidence_only,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
        })?;

        if self.requested_sample_length == 0 {
            return Err(ChallengeEvidenceValidationError::ZeroValue {
                field: "requested_sample_length",
            });
        }

        if self.bytes_returned != self.requested_sample_length {
            return Err(ChallengeEvidenceValidationError::SampleLengthMismatch {
                requested: self.requested_sample_length,
                returned: self.bytes_returned,
            });
        }

        Ok(())
    }

    pub fn replay_key(&self) -> AvailabilityProofReplayKeyV1 {
        AvailabilityProofReplayKeyV1 {
            service_node_id: self.service_node_id.clone(),
            witness_node_id: self.witness_node_id.clone(),
            challenge_id: self.challenge_id.clone(),
            challenge_nonce: self.challenge_nonce.clone(),
            content_id: self.content_id.clone(),
        }
    }

    pub fn witness_ack_signing_bytes(&self, witness_key_id: &str) -> Vec<u8> {
        let content_id = self.content_id.to_string();
        let mut out = Vec::with_capacity(512);

        out.extend_from_slice(AVAILABILITY_SIGNING_DOMAIN);

        append_signing_string(&mut out, &self.schema);
        out.extend_from_slice(&self.version.to_be_bytes());
        append_signing_string(&mut out, &self.proof_id);
        append_signing_string(&mut out, &self.service_node_id);
        append_signing_string(&mut out, &self.witness_node_id);
        append_signing_string(&mut out, &self.challenge_id);
        append_signing_string(&mut out, &self.challenge_nonce);
        append_signing_string(&mut out, &content_id);
        append_signing_string(&mut out, &self.service_observation_ref);
        append_signing_string(&mut out, &self.witness_ack_ref);
        append_signing_string(&mut out, witness_key_id);

        out.extend_from_slice(&self.requested_sample_offset.to_be_bytes());
        out.extend_from_slice(&self.requested_sample_length.to_be_bytes());
        out.extend_from_slice(&self.bytes_returned.to_be_bytes());
        out.extend_from_slice(&self.challenge_issued_at_ms.to_be_bytes());
        out.extend_from_slice(&self.response_completed_at_ms.to_be_bytes());

        append_authority_bytes(
            &mut out,
            self.content_verified,
            self.evidence_only,
            self.reward_truth,
            self.payout_authority,
            self.wallet_mutation,
            self.ledger_mutation,
        );

        out
    }
}

impl HotCacheProofV1 {
    pub fn validate(&self) -> Result<(), ChallengeEvidenceValidationError> {
        validate_common_proof(&CommonProofFields {
            schema: &self.schema,
            expected_schema: HOT_CACHE_PROOF_SCHEMA,
            version: self.version,
            proof_id: &self.proof_id,
            service_node_id: &self.service_node_id,
            witness_node_id: &self.witness_node_id,
            challenge_id: &self.challenge_id,
            challenge_nonce: &self.challenge_nonce,
            service_observation_ref: &self.service_observation_ref,
            witness_ack_ref: &self.witness_ack_ref,
            challenge_issued_at_ms: self.challenge_issued_at_ms,
            response_completed_at_ms: self.response_completed_at_ms,
            content_verified: self.content_verified,
            evidence_only: self.evidence_only,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
        })?;

        if self.bytes_returned == 0 {
            return Err(ChallengeEvidenceValidationError::ZeroValue {
                field: "bytes_returned",
            });
        }

        if self.response_latency_micros == 0 {
            return Err(ChallengeEvidenceValidationError::ZeroValue {
                field: "response_latency_micros",
            });
        }

        if !self.cache_hit {
            return Err(ChallengeEvidenceValidationError::HotCacheMiss);
        }

        if self.origin_fetch_performed {
            return Err(ChallengeEvidenceValidationError::OriginFetchPerformed);
        }

        let window_us = self
            .response_completed_at_ms
            .saturating_sub(self.challenge_issued_at_ms)
            .saturating_mul(1_000);

        if self.response_latency_micros > window_us {
            return Err(ChallengeEvidenceValidationError::LatencyExceedsWindow {
                latency_us: self.response_latency_micros,
                window_us,
            });
        }

        Ok(())
    }

    pub fn replay_key(&self) -> HotCacheProofReplayKeyV1 {
        HotCacheProofReplayKeyV1 {
            service_node_id: self.service_node_id.clone(),
            witness_node_id: self.witness_node_id.clone(),
            challenge_id: self.challenge_id.clone(),
            challenge_nonce: self.challenge_nonce.clone(),
            content_id: self.content_id.clone(),
        }
    }

    pub fn witness_ack_signing_bytes(&self, witness_key_id: &str) -> Vec<u8> {
        let content_id = self.content_id.to_string();
        let mut out = Vec::with_capacity(512);

        out.extend_from_slice(HOT_CACHE_SIGNING_DOMAIN);

        append_signing_string(&mut out, &self.schema);
        out.extend_from_slice(&self.version.to_be_bytes());
        append_signing_string(&mut out, &self.proof_id);
        append_signing_string(&mut out, &self.service_node_id);
        append_signing_string(&mut out, &self.witness_node_id);
        append_signing_string(&mut out, &self.challenge_id);
        append_signing_string(&mut out, &self.challenge_nonce);
        append_signing_string(&mut out, &content_id);
        append_signing_string(&mut out, &self.service_observation_ref);
        append_signing_string(&mut out, &self.witness_ack_ref);
        append_signing_string(&mut out, witness_key_id);

        out.extend_from_slice(&self.bytes_returned.to_be_bytes());
        out.extend_from_slice(&self.challenge_issued_at_ms.to_be_bytes());
        out.extend_from_slice(&self.response_completed_at_ms.to_be_bytes());
        out.extend_from_slice(&self.response_latency_micros.to_be_bytes());

        out.push(match self.cache_tier {
            HotCacheTierV1::Memory => 1,
        });

        out.push(u8::from(self.cache_hit));
        out.push(u8::from(self.origin_fetch_performed));

        append_authority_bytes(
            &mut out,
            self.content_verified,
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
    pub fn validate_for_availability(
        &self,
        proof: &AvailabilityProofV1,
    ) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
        self.validate_for_binding(
            ChallengeEvidenceKindV1::Availability,
            &proof.witness_ack_ref,
            &proof.witness_node_id,
        )
    }

    pub fn validate_for_hot_cache(
        &self,
        proof: &HotCacheProofV1,
    ) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
        self.validate_for_binding(
            ChallengeEvidenceKindV1::HotCache,
            &proof.witness_ack_ref,
            &proof.witness_node_id,
        )
    }

    pub(crate) fn validate_for_binding(
        &self,
        expected_kind: ChallengeEvidenceKindV1,
        expected_ack_id: &str,
        expected_witness_node_id: &str,
    ) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
        if self.schema != SERVICE_CHALLENGE_ACK_SCHEMA {
            return Err(ServiceChallengeAckValidationError::InvalidSchema {
                expected: SERVICE_CHALLENGE_ACK_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_EVIDENCE_VERSION {
            return Err(ServiceChallengeAckValidationError::InvalidVersion {
                expected: SERVICE_EVIDENCE_VERSION,
                actual: self.version,
            });
        }

        if self.evidence_kind != expected_kind {
            return Err(ServiceChallengeAckValidationError::KindMismatch {
                expected: expected_kind,
                actual: self.evidence_kind,
            });
        }

        validate_ack_token("ack_id", &self.ack_id)?;
        validate_ack_token("witness_node_id", &self.witness_node_id)?;
        validate_ack_token("witness_key_id", &self.witness_key_id)?;

        if self.ack_id != expected_ack_id {
            return Err(ServiceChallengeAckValidationError::BindingMismatch { field: "ack_id" });
        }

        if self.witness_node_id != expected_witness_node_id {
            return Err(ServiceChallengeAckValidationError::BindingMismatch {
                field: "witness_node_id",
            });
        }

        validate_ack_authority(self)?;

        decode_signature_hex(&self.signature_hex)
    }
}

struct CommonProofFields<'a> {
    schema: &'a str,
    expected_schema: &'static str,
    version: u16,
    proof_id: &'a str,
    service_node_id: &'a str,
    witness_node_id: &'a str,
    challenge_id: &'a str,
    challenge_nonce: &'a str,
    service_observation_ref: &'a str,
    witness_ack_ref: &'a str,
    challenge_issued_at_ms: u64,
    response_completed_at_ms: u64,
    content_verified: bool,
    evidence_only: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
}

fn validate_common_proof(
    fields: &CommonProofFields<'_>,
) -> Result<(), ChallengeEvidenceValidationError> {
    if fields.schema != fields.expected_schema {
        return Err(ChallengeEvidenceValidationError::InvalidSchema {
            expected: fields.expected_schema,
            actual: fields.schema.to_owned(),
        });
    }

    if fields.version != SERVICE_EVIDENCE_VERSION {
        return Err(ChallengeEvidenceValidationError::InvalidVersion {
            expected: SERVICE_EVIDENCE_VERSION,
            actual: fields.version,
        });
    }

    validate_proof_token("proof_id", fields.proof_id)?;
    validate_proof_token("service_node_id", fields.service_node_id)?;

    if fields.witness_node_id.trim().is_empty() || fields.witness_ack_ref.trim().is_empty() {
        return Err(ChallengeEvidenceValidationError::ProviderOnlyClaim);
    }

    validate_proof_token("witness_node_id", fields.witness_node_id)?;
    validate_proof_token("challenge_id", fields.challenge_id)?;
    validate_proof_token("challenge_nonce", fields.challenge_nonce)?;
    validate_proof_token("service_observation_ref", fields.service_observation_ref)?;
    validate_proof_token("witness_ack_ref", fields.witness_ack_ref)?;

    if fields.service_node_id == fields.witness_node_id {
        return Err(ChallengeEvidenceValidationError::SelfTraffic);
    }

    if fields.service_observation_ref == fields.witness_ack_ref {
        return Err(ChallengeEvidenceValidationError::WitnessReferenceCollision);
    }

    if fields.challenge_issued_at_ms == 0 {
        return Err(ChallengeEvidenceValidationError::ZeroValue {
            field: "challenge_issued_at_ms",
        });
    }

    if fields.response_completed_at_ms == 0 {
        return Err(ChallengeEvidenceValidationError::ZeroValue {
            field: "response_completed_at_ms",
        });
    }

    if fields.response_completed_at_ms < fields.challenge_issued_at_ms {
        return Err(ChallengeEvidenceValidationError::InvalidTimeOrder);
    }

    if !fields.content_verified {
        return Err(ChallengeEvidenceValidationError::UnverifiedContent);
    }

    validate_proof_authority(fields)
}

fn validate_proof_authority(
    fields: &CommonProofFields<'_>,
) -> Result<(), ChallengeEvidenceValidationError> {
    if !fields.evidence_only {
        return Err(ChallengeEvidenceValidationError::AuthorityBoundary {
            field: "evidence_only",
        });
    }

    for (field, value) in [
        ("reward_truth", fields.reward_truth),
        ("payout_authority", fields.payout_authority),
        ("wallet_mutation", fields.wallet_mutation),
        ("ledger_mutation", fields.ledger_mutation),
    ] {
        if value {
            return Err(ChallengeEvidenceValidationError::AuthorityBoundary { field });
        }
    }

    Ok(())
}

fn validate_ack_authority(
    ack: &ServiceChallengeAckV1,
) -> Result<(), ServiceChallengeAckValidationError> {
    if !ack.evidence_only {
        return Err(ServiceChallengeAckValidationError::AuthorityBoundary {
            field: "evidence_only",
        });
    }

    for (field, value) in [
        ("reward_truth", ack.reward_truth),
        ("payout_authority", ack.payout_authority),
        ("wallet_mutation", ack.wallet_mutation),
        ("ledger_mutation", ack.ledger_mutation),
    ] {
        if value {
            return Err(ServiceChallengeAckValidationError::AuthorityBoundary { field });
        }
    }

    Ok(())
}

fn validate_proof_token(
    field: &'static str,
    value: &str,
) -> Result<(), ChallengeEvidenceValidationError> {
    if !valid_token(value) {
        return Err(ChallengeEvidenceValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_ack_token(
    field: &'static str,
    value: &str,
) -> Result<(), ServiceChallengeAckValidationError> {
    if !valid_token(value) {
        return Err(ServiceChallengeAckValidationError::InvalidToken { field });
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

fn append_authority_bytes(
    out: &mut Vec<u8>,
    content_verified: bool,
    evidence_only: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
) {
    for value in [
        content_verified,
        evidence_only,
        reward_truth,
        payout_authority,
        wallet_mutation,
        ledger_mutation,
    ] {
        out.push(u8::from(value));
    }
}

fn decode_signature_hex(value: &str) -> Result<[u8; 64], ServiceChallengeAckValidationError> {
    if value.len() != ED25519_SIGNATURE_HEX_BYTES {
        return Err(ServiceChallengeAckValidationError::InvalidSignatureLength {
            actual: value.len(),
        });
    }

    let mut signature = [0u8; 64];

    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = lower_hex_value(pair[0])
            .ok_or(ServiceChallengeAckValidationError::InvalidSignatureHex)?;

        let low = lower_hex_value(pair[1])
            .ok_or(ServiceChallengeAckValidationError::InvalidSignatureHex)?;

        signature[index] = (high << 4) | low;
    }

    Ok(signature)
}

fn lower_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(10 + byte - b'a'),
        _ => None,
    }
}
