//! RO:WHAT — Strict service-node delivery-evidence DTOs.
//! RO:WHY — BUILD_PLAN_Z Phase 13 needs useful-service evidence without fake reward claims.
//! RO:INTERACTS — macronode, OAP delivery, future evidence review, accounting Phase 14.
//! RO:INVARIANTS — requester corroboration required; self-traffic rejected; replay key stable.
//! RO:SECURITY — no requester IP/route; no payout recipient; no wallet or ledger authority.
//! RO:TEST — tests/service_node_delivery_evidence.rs.

use crate::id::ContentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current service-evidence DTO version.
pub const SERVICE_EVIDENCE_VERSION: u16 = 1;

/// Canonical schema label for `DeliveryProofV1`.
pub const DELIVERY_PROOF_SCHEMA: &str = "ron.service_node.delivery_proof.v1";

const MAX_PROOF_ID_BYTES: usize = 256;
const MAX_ACTOR_ID_BYTES: usize = 256;
const MAX_REQUEST_ID_BYTES: usize = 256;
const MAX_REFERENCE_BYTES: usize = 512;

/// Validation failures for Phase 13 service evidence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DeliveryProofValidationError {
    /// The schema label was not the canonical delivery-proof schema.
    #[error("invalid DeliveryProofV1 schema: expected {expected}, got {actual}")]
    InvalidSchema {
        /// Required schema.
        expected: &'static str,
        /// Supplied schema.
        actual: String,
    },

    /// The DTO version was unsupported.
    #[error("invalid DeliveryProofV1 version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// Required version.
        expected: u16,
        /// Supplied version.
        actual: u16,
    },

    /// A required field was empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },

    /// A field exceeded its maximum length.
    #[error("{field} exceeds maximum bytes: max={max}, actual={actual}")]
    FieldTooLong {
        /// Field name.
        field: &'static str,
        /// Maximum allowed byte length.
        max: usize,
        /// Actual byte length.
        actual: usize,
    },

    /// An identifier contained route, IP, whitespace, or unsupported syntax.
    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken {
        /// Field name.
        field: &'static str,
    },

    /// A provider attempted to submit evidence without requester corroboration.
    #[error("delivery evidence requires requester identity and acknowledgment")]
    ProviderOnlyClaim,

    /// Provider and requester resolved to the same node identity.
    #[error("self-traffic delivery evidence is not accepted")]
    SelfTraffic,

    /// Provider and requester references were identical.
    #[error("provider observation and requester acknowledgment must be distinct")]
    WitnessReferenceCollision,

    /// A delivery proof claimed no delivered bytes.
    #[error("bytes_delivered must be greater than zero")]
    ZeroBytes,

    /// A timestamp field was zero.
    #[error("{field} must be greater than zero")]
    ZeroTimestamp {
        /// Field name.
        field: &'static str,
    },

    /// Completion preceded delivery start.
    #[error("completed_at_ms must not precede started_at_ms")]
    InvalidTimeOrder,

    /// Successful delivery evidence did not assert content verification.
    #[error("delivery proof requires full content-address verification")]
    UnverifiedContent,

    /// An evidence-only authority boundary was violated.
    #[error("delivery proof authority boundary violated: {field}")]
    AuthorityBoundary {
        /// Contradictory field.
        field: &'static str,
    },
}

/// Deterministic duplicate-detection key for delivery evidence.
///
/// `proof_id` is deliberately excluded. A producer cannot evade replay
/// detection merely by assigning a new proof identifier to the same service,
/// requester, request, and object tuple.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeliveryProofReplayKeyV1 {
    /// Service node claiming delivery.
    pub service_node_id: String,

    /// Independent requester node identity.
    pub requester_node_id: String,

    /// Requester-generated request identifier.
    pub request_id: String,

    /// Object requested and delivered.
    pub content_id: ContentId,
}

/// Requester-corroborated evidence of one verified object delivery.
///
/// This DTO is not a receipt, reward plan, payout authorization, balance
/// mutation, or ledger commit. Signature/reference verification and replay-set
/// mutation belong to later runtime review layers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryProofV1 {
    /// Canonical schema. Must equal `DELIVERY_PROOF_SCHEMA`.
    pub schema: String,

    /// DTO version. Must equal `SERVICE_EVIDENCE_VERSION`.
    pub version: u16,

    /// Producer-assigned proof identifier.
    pub proof_id: String,

    /// Service node that claims it delivered the object.
    ///
    /// This must be an opaque node identifier, not an IP address, socket,
    /// residential route, or payout recipient.
    pub service_node_id: String,

    /// Independent node that requested and acknowledged the delivery.
    ///
    /// This is an opaque node identifier and must differ from
    /// `service_node_id`.
    pub requester_node_id: String,

    /// Requester-generated request identifier.
    pub request_id: String,

    /// Canonical BLAKE3-addressed object.
    pub content_id: ContentId,

    /// Local provider-side observation or signature reference.
    pub provider_observation_ref: String,

    /// Independent requester acknowledgment or signature reference.
    pub requester_ack_ref: String,

    /// Number of verified object bytes delivered.
    pub bytes_delivered: u64,

    /// Delivery start timestamp in milliseconds since Unix epoch.
    pub started_at_ms: u64,

    /// Delivery completion timestamp in milliseconds since Unix epoch.
    pub completed_at_ms: u64,

    /// Must be true for successful delivery evidence.
    ///
    /// The runtime producing this proof must have performed complete
    /// content-address verification before setting this field.
    pub content_verified: bool,

    /// Must remain true: this artifact is evidence only.
    pub evidence_only: bool,

    /// Must remain false: this proof is not reward truth.
    pub reward_truth: bool,

    /// Must remain false: this proof cannot authorize payout.
    pub payout_authority: bool,

    /// Must remain false: this proof cannot mutate a wallet.
    pub wallet_mutation: bool,

    /// Must remain false: this proof cannot mutate the ledger.
    pub ledger_mutation: bool,
}

impl DeliveryProofV1 {
    /// Validate the strict delivery-evidence shape and authority boundaries.
    ///
    /// This validates DTO structure only. It does not verify signatures,
    /// consult a replay set, calculate rewards, or mutate external state.
    pub fn validate(&self) -> Result<(), DeliveryProofValidationError> {
        if self.schema != DELIVERY_PROOF_SCHEMA {
            return Err(DeliveryProofValidationError::InvalidSchema {
                expected: DELIVERY_PROOF_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_EVIDENCE_VERSION {
            return Err(DeliveryProofValidationError::InvalidVersion {
                expected: SERVICE_EVIDENCE_VERSION,
                actual: self.version,
            });
        }

        validate_token("proof_id", &self.proof_id, MAX_PROOF_ID_BYTES)?;

        validate_token("service_node_id", &self.service_node_id, MAX_ACTOR_ID_BYTES)?;

        if self.requester_node_id.trim().is_empty() || self.requester_ack_ref.trim().is_empty() {
            return Err(DeliveryProofValidationError::ProviderOnlyClaim);
        }

        validate_token(
            "requester_node_id",
            &self.requester_node_id,
            MAX_ACTOR_ID_BYTES,
        )?;

        validate_token("request_id", &self.request_id, MAX_REQUEST_ID_BYTES)?;

        validate_token(
            "provider_observation_ref",
            &self.provider_observation_ref,
            MAX_REFERENCE_BYTES,
        )?;

        validate_token(
            "requester_ack_ref",
            &self.requester_ack_ref,
            MAX_REFERENCE_BYTES,
        )?;

        if self.service_node_id == self.requester_node_id {
            return Err(DeliveryProofValidationError::SelfTraffic);
        }

        if self.provider_observation_ref == self.requester_ack_ref {
            return Err(DeliveryProofValidationError::WitnessReferenceCollision);
        }

        if self.bytes_delivered == 0 {
            return Err(DeliveryProofValidationError::ZeroBytes);
        }

        validate_timestamp("started_at_ms", self.started_at_ms)?;

        validate_timestamp("completed_at_ms", self.completed_at_ms)?;

        if self.completed_at_ms < self.started_at_ms {
            return Err(DeliveryProofValidationError::InvalidTimeOrder);
        }

        if !self.content_verified {
            return Err(DeliveryProofValidationError::UnverifiedContent);
        }

        if !self.evidence_only {
            return Err(DeliveryProofValidationError::AuthorityBoundary {
                field: "evidence_only",
            });
        }

        for (field, value) in [
            ("reward_truth", self.reward_truth),
            ("payout_authority", self.payout_authority),
            ("wallet_mutation", self.wallet_mutation),
            ("ledger_mutation", self.ledger_mutation),
        ] {
            if value {
                return Err(DeliveryProofValidationError::AuthorityBoundary { field });
            }
        }

        Ok(())
    }

    /// Return the stable key a runtime replay guard should use.
    pub fn replay_key(&self) -> DeliveryProofReplayKeyV1 {
        DeliveryProofReplayKeyV1 {
            service_node_id: self.service_node_id.clone(),
            requester_node_id: self.requester_node_id.clone(),
            request_id: self.request_id.clone(),
            content_id: self.content_id.clone(),
        }
    }
}

fn validate_token(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), DeliveryProofValidationError> {
    if value.trim().is_empty() {
        return Err(DeliveryProofValidationError::EmptyField { field });
    }

    let actual = value.len();

    if actual > max {
        return Err(DeliveryProofValidationError::FieldTooLong { field, max, actual });
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b':')
    }) {
        return Err(DeliveryProofValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_timestamp(field: &'static str, value: u64) -> Result<(), DeliveryProofValidationError> {
    if value == 0 {
        return Err(DeliveryProofValidationError::ZeroTimestamp { field });
    }

    Ok(())
}

/// Canonical schema label for a requester delivery acknowledgment.
pub const DELIVERY_REQUESTER_ACK_SCHEMA: &str = "ron.service_node.delivery_requester_ack.v1";

/// Domain separator for requester acknowledgment signatures.
pub const DELIVERY_REQUESTER_ACK_DOMAIN: &[u8] = b"ron.service_node.delivery_requester_ack.v1\0";

const ED25519_SIGNATURE_HEX_BYTES: usize = 128;

/// Validation failures for a requester delivery acknowledgment.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DeliveryRequesterAckValidationError {
    /// The acknowledgment schema was not canonical.
    #[error("invalid requester acknowledgment schema: expected {expected}, got {actual}")]
    InvalidSchema {
        /// Required schema.
        expected: &'static str,
        /// Supplied schema.
        actual: String,
    },

    /// The acknowledgment version was unsupported.
    #[error("invalid requester acknowledgment version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// Required version.
        expected: u16,
        /// Supplied version.
        actual: u16,
    },

    /// A required token was empty or malformed.
    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken {
        /// Invalid field.
        field: &'static str,
    },

    /// The acknowledgment did not bind to the supplied proof.
    #[error("requester acknowledgment does not match proof field {field}")]
    BindingMismatch {
        /// Mismatched field.
        field: &'static str,
    },

    /// The Ed25519 signature had the wrong encoded length.
    #[error(
        "requester signature must contain 128 lowercase hexadecimal characters; actual={actual}"
    )]
    InvalidSignatureLength {
        /// Actual encoded byte length.
        actual: usize,
    },

    /// The signature contained unsupported hexadecimal syntax.
    #[error("requester signature must contain lowercase hexadecimal characters only")]
    InvalidSignatureHex,

    /// An evidence-only authority boundary was violated.
    #[error("requester acknowledgment authority boundary violated: {field}")]
    AuthorityBoundary {
        /// Contradictory field.
        field: &'static str,
    },
}

/// Detached requester acknowledgment for one `DeliveryProofV1`.
///
/// The public key is deliberately absent. Runtime verification must resolve
/// the trusted requester key from `requester_node_id` and `requester_key_id`.
/// A provider therefore cannot self-supply an arbitrary verification key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryRequesterAckV1 {
    /// Canonical schema.
    pub schema: String,

    /// DTO version.
    pub version: u16,

    /// Must exactly match the proof's `requester_ack_ref`.
    pub ack_id: String,

    /// Must exactly match the proof's requester identity.
    pub requester_node_id: String,

    /// Identity-system key reference used by the runtime key resolver.
    pub requester_key_id: String,

    /// Ed25519 signature encoded as exactly 128 lowercase hexadecimal
    /// characters.
    pub signature_hex: String,

    /// Must remain true: this artifact is evidence only.
    pub evidence_only: bool,

    /// Must remain false: this acknowledgment is not reward truth.
    pub reward_truth: bool,

    /// Must remain false: this acknowledgment cannot authorize payout.
    pub payout_authority: bool,

    /// Must remain false: this acknowledgment cannot mutate a wallet.
    pub wallet_mutation: bool,

    /// Must remain false: this acknowledgment cannot mutate the ledger.
    pub ledger_mutation: bool,
}

impl DeliveryRequesterAckV1 {
    /// Validate the acknowledgment and its exact binding to a proof.
    ///
    /// On success, returns the decoded 64-byte Ed25519 signature. Public-key
    /// lookup and cryptographic verification remain runtime responsibilities.
    pub fn validate_for(
        &self,
        proof: &DeliveryProofV1,
    ) -> Result<[u8; 64], DeliveryRequesterAckValidationError> {
        if self.schema != DELIVERY_REQUESTER_ACK_SCHEMA {
            return Err(DeliveryRequesterAckValidationError::InvalidSchema {
                expected: DELIVERY_REQUESTER_ACK_SCHEMA,
                actual: self.schema.clone(),
            });
        }

        if self.version != SERVICE_EVIDENCE_VERSION {
            return Err(DeliveryRequesterAckValidationError::InvalidVersion {
                expected: SERVICE_EVIDENCE_VERSION,
                actual: self.version,
            });
        }

        validate_ack_token("ack_id", &self.ack_id)?;
        validate_ack_token("requester_node_id", &self.requester_node_id)?;
        validate_ack_token("requester_key_id", &self.requester_key_id)?;

        if self.ack_id != proof.requester_ack_ref {
            return Err(DeliveryRequesterAckValidationError::BindingMismatch { field: "ack_id" });
        }

        if self.requester_node_id != proof.requester_node_id {
            return Err(DeliveryRequesterAckValidationError::BindingMismatch {
                field: "requester_node_id",
            });
        }

        if !self.evidence_only {
            return Err(DeliveryRequesterAckValidationError::AuthorityBoundary {
                field: "evidence_only",
            });
        }

        for (field, value) in [
            ("reward_truth", self.reward_truth),
            ("payout_authority", self.payout_authority),
            ("wallet_mutation", self.wallet_mutation),
            ("ledger_mutation", self.ledger_mutation),
        ] {
            if value {
                return Err(DeliveryRequesterAckValidationError::AuthorityBoundary { field });
            }
        }

        decode_signature_hex(&self.signature_hex)
    }
}

impl DeliveryProofV1 {
    /// Construct deterministic, domain-separated requester-acknowledgment
    /// signing bytes.
    ///
    /// Every material proof field and the resolved key reference are included.
    /// Changing the object, actors, request, byte count, timestamps, integrity
    /// result, authority posture, or key ID invalidates the signature.
    pub fn requester_ack_signing_bytes(&self, requester_key_id: &str) -> Vec<u8> {
        let content_id = self.content_id.to_string();
        let mut out = Vec::with_capacity(512);

        out.extend_from_slice(DELIVERY_REQUESTER_ACK_DOMAIN);

        append_signing_string(&mut out, &self.schema);
        out.extend_from_slice(&self.version.to_be_bytes());
        append_signing_string(&mut out, &self.proof_id);
        append_signing_string(&mut out, &self.service_node_id);
        append_signing_string(&mut out, &self.requester_node_id);
        append_signing_string(&mut out, &self.request_id);
        append_signing_string(&mut out, &content_id);
        append_signing_string(&mut out, &self.provider_observation_ref);
        append_signing_string(&mut out, &self.requester_ack_ref);
        append_signing_string(&mut out, requester_key_id);

        out.extend_from_slice(&self.bytes_delivered.to_be_bytes());
        out.extend_from_slice(&self.started_at_ms.to_be_bytes());
        out.extend_from_slice(&self.completed_at_ms.to_be_bytes());

        for value in [
            self.content_verified,
            self.evidence_only,
            self.reward_truth,
            self.payout_authority,
            self.wallet_mutation,
            self.ledger_mutation,
        ] {
            out.push(u8::from(value));
        }

        out
    }
}

fn validate_ack_token(
    field: &'static str,
    value: &str,
) -> Result<(), DeliveryRequesterAckValidationError> {
    if value.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b':')
        })
    {
        return Err(DeliveryRequesterAckValidationError::InvalidToken { field });
    }

    Ok(())
}

fn append_signing_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn decode_signature_hex(value: &str) -> Result<[u8; 64], DeliveryRequesterAckValidationError> {
    if value.len() != ED25519_SIGNATURE_HEX_BYTES {
        return Err(
            DeliveryRequesterAckValidationError::InvalidSignatureLength {
                actual: value.len(),
            },
        );
    }

    let mut signature = [0u8; 64];

    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = lower_hex_value(pair[0])
            .ok_or(DeliveryRequesterAckValidationError::InvalidSignatureHex)?;

        let low = lower_hex_value(pair[1])
            .ok_or(DeliveryRequesterAckValidationError::InvalidSignatureHex)?;

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
