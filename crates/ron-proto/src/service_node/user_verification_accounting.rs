//! RO:WHAT — Canonical User Node verification-evidence handoff to accounting.
//! RO:WHY — Phase 14 must classify verified User Node work before reward planning.
//! RO:INTERACTS — micronode verification workers and ron-accounting classification.
//! RO:INVARIANTS — verified attestation, strict result/reason relation, evidence only.
//! RO:SECURITY — no IP, raw engagement, payout target, wallet, or ledger authority.
//! RO:TEST — tests/user_verification_accounting_input.rs.

#![forbid(unsafe_code)]

use crate::id::ContentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA: &str =
    "ron.user_node.verification_accounting_input.v1";

pub const USER_VERIFICATION_ACCOUNTING_INPUT_VERSION: u16 = 1;

const MAX_TOKEN_BYTES: usize = 512;

/// Canonical User Node verification work kinds.
///
/// These describe verification work only. They do not imply accounting
/// acceptance, reward eligibility, payout approval, or ledger mutation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UserVerificationEvidenceKindV1 {
    MicrotxReceiptVerification,
    LedgerReceiptReplaySample,
    EpochTransitionReplaySample,
    RewardPlanVerification,
    RewardPoolCapVerification,
    DeliveryReceiptChallengeReview,
    B3IntegrityChallenge,
    AvailabilityChallengeReview,
    ReplayRejectionProof,
    DuplicateEvidenceRejection,
    InvalidEpochChallenge,
}

/// Result produced by deterministic local verification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UserVerificationResultV1 {
    /// The reviewed material passed the selected verification.
    VerifiedValid,

    /// The reviewed material was deterministically found invalid.
    VerifiedInvalid,

    /// The review produced challenge evidence requiring later acceptance.
    ChallengeRaised,
}

/// Bounded machine-readable reason for invalid/challenged material.
///
/// Free-form text is intentionally absent to keep evidence deterministic,
/// privacy-safe, and unsuitable for smuggling network identifiers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UserVerificationFailureReasonV1 {
    SubjectInvalid,
    DigestMismatch,
    ReplayDetected,
    DuplicateDetected,
    PolicyMismatch,
    EconomicsConfigMismatch,
    RewardPoolCapExceeded,
    SupplyMismatch,
    SignatureInvalid,
    MissingEvidence,
    OtherInvalidMaterial,
}

/// Strict post-verification handoff from a User Node into accounting.
///
/// The signature or local attestation has already been checked before this
/// DTO is accepted. Accounting revalidates all structural and non-authority
/// fields before classifying the evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserVerificationAccountingInputV1 {
    pub schema: String,
    pub version: u16,

    /// Process-local verification spool sequence.
    pub sequence: u64,

    pub evidence_id: String,
    pub user_node_id: String,

    /// Privacy-safe receipt, epoch, content, operation, or plan reference.
    pub subject_ref: String,

    pub verification_kind: UserVerificationEvidenceKindV1,
    pub observed_at_ms: u64,

    /// Canonical digest of the exact material that was reviewed.
    pub input_digest: ContentId,

    pub result: UserVerificationResultV1,

    /// Required for invalid/challenge results and absent for valid results.
    pub failure_reason: Option<UserVerificationFailureReasonV1>,

    pub nonce: String,
    pub idempotency_key: String,
    pub capability_id: String,

    /// Optional opaque privacy-route identity, never an IP or socket.
    pub privacy_route_id: Option<String>,

    /// Reference to the checked local or cryptographic attestation.
    pub attestation_ref: String,
    pub local_attestation_verified: bool,

    pub evidence_only: bool,

    /// These must remain false at the accounting ingress boundary.
    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum UserVerificationAccountingInputValidationError {
    #[error("invalid user-verification accounting schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },

    #[error("invalid user-verification accounting version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

    #[error("{field} must be greater than zero")]
    ZeroValue { field: &'static str },

    #[error("{field} must be a privacy-safe lowercase identifier token")]
    InvalidToken { field: &'static str },

    #[error("verified-valid evidence must not contain a failure reason")]
    UnexpectedFailureReason,

    #[error("invalid or challenged evidence requires a failure reason")]
    MissingFailureReason,

    #[error("user-verification accounting handoff requires a verified attestation")]
    AttestationNotVerified,

    #[error("user-verification accounting handoff requires evidence_only=true")]
    NotEvidenceOnly,

    #[error("user-verification accounting authority boundary violated: {field}")]
    AuthorityBoundary { field: &'static str },
}

impl UserVerificationAccountingInputV1 {
    pub fn validate(&self) -> Result<(), UserVerificationAccountingInputValidationError> {
        if self.schema != USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA {
            return Err(
                UserVerificationAccountingInputValidationError::InvalidSchema {
                    expected: USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA,
                    actual: self.schema.clone(),
                },
            );
        }

        if self.version != USER_VERIFICATION_ACCOUNTING_INPUT_VERSION {
            return Err(
                UserVerificationAccountingInputValidationError::InvalidVersion {
                    expected: USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
                    actual: self.version,
                },
            );
        }

        if self.sequence == 0 {
            return Err(UserVerificationAccountingInputValidationError::ZeroValue {
                field: "sequence",
            });
        }

        if self.observed_at_ms == 0 {
            return Err(UserVerificationAccountingInputValidationError::ZeroValue {
                field: "observed_at_ms",
            });
        }

        validate_token("evidence_id", &self.evidence_id)?;
        validate_token("user_node_id", &self.user_node_id)?;
        validate_token("subject_ref", &self.subject_ref)?;
        validate_token("nonce", &self.nonce)?;
        validate_token("idempotency_key", &self.idempotency_key)?;
        validate_token("capability_id", &self.capability_id)?;
        validate_token("attestation_ref", &self.attestation_ref)?;

        if let Some(privacy_route_id) = &self.privacy_route_id {
            validate_privacy_route_id(privacy_route_id)?;
        }

        match (self.result, self.failure_reason) {
            (UserVerificationResultV1::VerifiedValid, None) => {}

            (UserVerificationResultV1::VerifiedValid, Some(_)) => {
                return Err(
                    UserVerificationAccountingInputValidationError::UnexpectedFailureReason,
                );
            }

            (
                UserVerificationResultV1::VerifiedInvalid
                | UserVerificationResultV1::ChallengeRaised,
                Some(_),
            ) => {}

            (
                UserVerificationResultV1::VerifiedInvalid
                | UserVerificationResultV1::ChallengeRaised,
                None,
            ) => {
                return Err(UserVerificationAccountingInputValidationError::MissingFailureReason);
            }
        }

        if !self.local_attestation_verified {
            return Err(UserVerificationAccountingInputValidationError::AttestationNotVerified);
        }

        if !self.evidence_only {
            return Err(UserVerificationAccountingInputValidationError::NotEvidenceOnly);
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
                    UserVerificationAccountingInputValidationError::AuthorityBoundary { field },
                );
            }
        }

        Ok(())
    }
}

fn validate_privacy_route_id(
    value: &str,
) -> Result<(), UserVerificationAccountingInputValidationError> {
    let colon_count = value.bytes().filter(|byte| *byte == b':').count();

    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && colon_count <= 1
        && !value.starts_with(':')
        && !value.ends_with(':')
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b':')
        });

    if !valid {
        return Err(
            UserVerificationAccountingInputValidationError::InvalidToken {
                field: "privacy_route_id",
            },
        );
    }

    Ok(())
}

fn validate_token(
    field: &'static str,
    value: &str,
) -> Result<(), UserVerificationAccountingInputValidationError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.' | b'/')
        });

    if !valid {
        return Err(UserVerificationAccountingInputValidationError::InvalidToken { field });
    }

    Ok(())
}
