//! RO:WHAT — Canonical service-evidence handoff into ron-accounting.
//! RO:WHY — Phase 14 must classify reviewed evidence before reward planning.
//! RO:INTERACTS — macronode evidence outbox and ron-accounting classification.
//! RO:INVARIANTS — signed evidence only; deterministic actors; no economic authority.
//! RO:SECURITY — no IP fields, reward amounts, payout targets, wallet, or ledger authority.
//! RO:TEST — tests/service_node_accounting_input.rs.

#![forbid(unsafe_code)]

use crate::id::ContentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA: &str = "ron.service_node.accounting_input.v1";

pub const SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION: u16 = 1;

const MAX_TOKEN_BYTES: usize = 512;
const MAX_RELATED_ACTORS: usize = 4;

/// Canonical Phase 14 accounting kind for reviewed service evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEvidenceAccountingKindV1 {
    Delivery,
    Availability,
    RangeRequest,
    Repair,
    HotCache,
    PolicyRefusal,
    ModerationAction,
}

/// Strict handoff from a service-node evidence outbox into accounting.
///
/// This DTO carries already-reviewed evidence metadata. Accounting still
/// performs its own structural and authority-boundary validation before
/// classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceEvidenceAccountingInputV1 {
    pub schema: String,
    pub version: u16,

    /// Process-local evidence-outbox sequence.
    pub sequence: u64,
    pub kind: ServiceEvidenceAccountingKindV1,

    pub proof_id: String,
    pub service_node_id: String,
    pub witness_node_id: String,

    /// Repair source, moderation operator, or another bounded secondary actor.
    pub related_actor_ids: Vec<String>,

    pub content_id: ContentId,
    pub observed_at_ms: u64,

    pub signature_verified: bool,
    pub evidence_only: bool,

    /// Must remain false when evidence enters accounting.
    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ServiceEvidenceAccountingInputValidationError {
    #[error("invalid accounting-input schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid accounting-input version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("service node and witness identities must be distinct")]
    SelfTraffic,

    #[error("related actor count exceeds maximum: max={max}, actual={actual}")]
    TooManyRelatedActors { max: usize, actual: usize },

    #[error("related actors must be strictly sorted and unique")]
    RelatedActorsNotCanonical,

    #[error("related actor collides with primary actor: {field}")]
    ActorCollision { field: &'static str },

    #[error("service evidence accounting handoff requires a verified signature")]
    SignatureNotVerified,

    #[error("service evidence accounting handoff requires evidence_only=true")]
    NotEvidenceOnly,

    #[error("service evidence accounting authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

impl ServiceEvidenceAccountingInputV1 {
    pub fn validate(&self) -> Result<(), ServiceEvidenceAccountingInputValidationError> {
        if self.schema != SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA {
            return Err(
                ServiceEvidenceAccountingInputValidationError::InvalidSchema {
                    expected: SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA,
                    actual: self.schema.clone(),
                },
            );
        }

        if self.version != SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION {
            return Err(
                ServiceEvidenceAccountingInputValidationError::InvalidVersion {
                    expected: SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
                    actual: self.version,
                },
            );
        }

        if self.sequence == 0 {
            return Err(ServiceEvidenceAccountingInputValidationError::ZeroValue {
                field: "sequence",
            });
        }

        if self.observed_at_ms == 0 {
            return Err(ServiceEvidenceAccountingInputValidationError::ZeroValue {
                field: "observed_at_ms",
            });
        }

        validate_token("proof_id", &self.proof_id)?;
        validate_token("service_node_id", &self.service_node_id)?;
        validate_token("witness_node_id", &self.witness_node_id)?;

        if self.service_node_id == self.witness_node_id {
            return Err(ServiceEvidenceAccountingInputValidationError::SelfTraffic);
        }

        if self.related_actor_ids.len() > MAX_RELATED_ACTORS {
            return Err(
                ServiceEvidenceAccountingInputValidationError::TooManyRelatedActors {
                    max: MAX_RELATED_ACTORS,
                    actual: self.related_actor_ids.len(),
                },
            );
        }

        for actor in &self.related_actor_ids {
            validate_token("related_actor_ids[]", actor)?;

            if actor == &self.service_node_id {
                return Err(
                    ServiceEvidenceAccountingInputValidationError::ActorCollision {
                        field: "service_node_id",
                    },
                );
            }

            if actor == &self.witness_node_id {
                return Err(
                    ServiceEvidenceAccountingInputValidationError::ActorCollision {
                        field: "witness_node_id",
                    },
                );
            }
        }

        if self
            .related_actor_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(ServiceEvidenceAccountingInputValidationError::RelatedActorsNotCanonical);
        }

        if !self.signature_verified {
            return Err(ServiceEvidenceAccountingInputValidationError::SignatureNotVerified);
        }

        if !self.evidence_only {
            return Err(ServiceEvidenceAccountingInputValidationError::NotEvidenceOnly);
        }

        for (field, value) in [
            ("accounting_accepted", self.accounting_accepted),
            ("reward_eligible", self.reward_eligible),
            ("reward_truth", self.reward_truth),
            ("payout_authority", self.payout_authority),
            ("wallet_mutation", self.wallet_mutation),
            ("ledger_mutation", self.ledger_mutation),
        ] {
            if value {
                return Err(
                    ServiceEvidenceAccountingInputValidationError::AuthorityBoundary { field },
                );
            }
        }

        Ok(())
    }
}

fn validate_token(
    field: &'static str,
    value: &str,
) -> Result<(), ServiceEvidenceAccountingInputValidationError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.' | b'/')
        });

    if !valid {
        return Err(ServiceEvidenceAccountingInputValidationError::InvalidToken { field });
    }

    Ok(())
}
