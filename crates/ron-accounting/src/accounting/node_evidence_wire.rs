//! RO:WHAT — Strict authority-free node-evidence ingress DTOs for accounting.
//! RO:WHY — Accounting must consume evidence without linking protocol/root authority.
//! RO:INTERACTS — serialized Service Node and User Node evidence handoffs.
//! RO:INVARIANTS — exact schemas, verified evidence, bounded identities, no authority.
//! RO:SECURITY — no IP, reward amount, payout, balance, wallet, or ledger authority.
//! RO:TEST — Phase 14 classification and epoch snapshot integration tests.

#![forbid(unsafe_code)]

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA: &str = "ron.service_node.accounting_input.v1";

pub const SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION: u16 = 1;

pub const USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA: &str =
    "ron.user_node.verification_accounting_input.v1";

pub const USER_VERIFICATION_ACCOUNTING_INPUT_VERSION: u16 = 1;

const MAX_TOKEN_BYTES: usize = 512;
const MAX_RELATED_ACTORS: usize = 4;

/// Strict lowercase BLAKE3 content or evidence digest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct EvidenceContentId(String);

impl EvidenceContentId {
    /// Validate the stored `b3:<64 lowercase hex>` value.
    pub fn validate(&self) -> Result<()> {
        validate_b3("content_id", &self.0)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for EvidenceContentId {
    type Err = Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        validate_b3("content_id", value)?;
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for EvidenceContentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Canonical Service Node evidence kind accepted at accounting ingress.
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

/// Strict Service Node evidence handoff accepted by accounting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceEvidenceAccountingInputV1 {
    pub schema: String,
    pub version: u16,

    pub sequence: u64,
    pub kind: ServiceEvidenceAccountingKindV1,

    pub proof_id: String,
    pub service_node_id: String,
    pub witness_node_id: String,
    pub related_actor_ids: Vec<String>,

    pub content_id: EvidenceContentId,
    pub observed_at_ms: u64,

    pub signature_verified: bool,
    pub evidence_only: bool,

    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

impl ServiceEvidenceAccountingInputV1 {
    /// Validate the strict Service Node accounting-ingress boundary.
    pub fn validate(&self) -> Result<()> {
        if self.schema != SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA {
            return Err(Error::schema("invalid service evidence accounting schema"));
        }

        if self.version != SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION {
            return Err(Error::schema("invalid service evidence accounting version"));
        }

        if self.sequence == 0 {
            return Err(Error::schema(
                "service evidence sequence must be greater than zero",
            ));
        }

        if self.observed_at_ms == 0 {
            return Err(Error::schema(
                "service evidence observed_at_ms must be greater than zero",
            ));
        }

        validate_token("proof_id", &self.proof_id)?;
        validate_token("service_node_id", &self.service_node_id)?;
        validate_token("witness_node_id", &self.witness_node_id)?;
        self.content_id.validate()?;

        if self.service_node_id == self.witness_node_id {
            return Err(Error::schema("service evidence rejects self-traffic"));
        }

        if self.related_actor_ids.len() > MAX_RELATED_ACTORS {
            return Err(Error::schema(format!(
                "related actor count exceeds maximum: max={}, actual={}",
                MAX_RELATED_ACTORS,
                self.related_actor_ids.len()
            )));
        }

        for actor in &self.related_actor_ids {
            validate_token("related_actor_ids[]", actor)?;

            if actor == &self.service_node_id || actor == &self.witness_node_id {
                return Err(Error::schema("related actor collides with a primary actor"));
            }
        }

        if self
            .related_actor_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::schema(
                "related actors must be strictly sorted and unique",
            ));
        }

        if !self.signature_verified {
            return Err(Error::schema(
                "service evidence requires a verified signature",
            ));
        }

        if !self.evidence_only {
            return Err(Error::schema("service evidence must remain evidence-only"));
        }

        validate_authority_flags(
            self.accounting_accepted,
            self.reward_eligible,
            self.reward_truth,
            self.payout_authority,
            self.wallet_mutation,
            self.ledger_mutation,
        )
    }
}

/// Canonical User Node verification work kind.
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

/// Deterministic User Node verification result.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UserVerificationResultV1 {
    VerifiedValid,
    VerifiedInvalid,
    ChallengeRaised,
}

/// Machine-readable invalid/challenge reason.
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

/// Strict User Node verification handoff accepted by accounting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserVerificationAccountingInputV1 {
    pub schema: String,
    pub version: u16,

    pub sequence: u64,

    pub evidence_id: String,
    pub user_node_id: String,
    pub subject_ref: String,

    pub verification_kind: UserVerificationEvidenceKindV1,

    pub observed_at_ms: u64,
    pub input_digest: EvidenceContentId,

    pub result: UserVerificationResultV1,
    pub failure_reason: Option<UserVerificationFailureReasonV1>,

    pub nonce: String,
    pub idempotency_key: String,
    pub capability_id: String,

    pub privacy_route_id: Option<String>,

    pub attestation_ref: String,
    pub local_attestation_verified: bool,

    pub evidence_only: bool,

    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

impl UserVerificationAccountingInputV1 {
    /// Validate the strict User Node accounting-ingress boundary.
    pub fn validate(&self) -> Result<()> {
        if self.schema != USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA {
            return Err(Error::schema("invalid user verification accounting schema"));
        }

        if self.version != USER_VERIFICATION_ACCOUNTING_INPUT_VERSION {
            return Err(Error::schema(
                "invalid user verification accounting version",
            ));
        }

        if self.sequence == 0 {
            return Err(Error::schema(
                "user verification sequence must be greater than zero",
            ));
        }

        if self.observed_at_ms == 0 {
            return Err(Error::schema(
                "user verification observed_at_ms must be greater than zero",
            ));
        }

        validate_token("evidence_id", &self.evidence_id)?;
        validate_token("user_node_id", &self.user_node_id)?;
        validate_token("subject_ref", &self.subject_ref)?;
        validate_token("nonce", &self.nonce)?;
        validate_token("idempotency_key", &self.idempotency_key)?;
        validate_token("capability_id", &self.capability_id)?;
        validate_token("attestation_ref", &self.attestation_ref)?;
        self.input_digest.validate()?;

        if let Some(privacy_route_id) = &self.privacy_route_id {
            validate_privacy_route_id(privacy_route_id)?;
        }

        match (self.result, self.failure_reason) {
            (UserVerificationResultV1::VerifiedValid, None) => {}

            (UserVerificationResultV1::VerifiedValid, Some(_)) => {
                return Err(Error::schema(
                    "verified-valid evidence must not contain a failure reason",
                ));
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
                return Err(Error::schema(
                    "invalid or challenged evidence requires a failure reason",
                ));
            }
        }

        if !self.local_attestation_verified {
            return Err(Error::schema(
                "user verification requires a verified attestation",
            ));
        }

        if !self.evidence_only {
            return Err(Error::schema("user verification must remain evidence-only"));
        }

        validate_authority_flags(
            self.accounting_accepted,
            self.reward_eligible,
            self.reward_truth,
            self.payout_authority,
            self.wallet_mutation,
            self.ledger_mutation,
        )
    }
}

fn validate_token(field: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.' | b'/')
        });

    if !valid {
        return Err(Error::schema(format!(
            "{field} must be a bounded lowercase identifier token"
        )));
    }

    Ok(())
}

fn validate_privacy_route_id(value: &str) -> Result<()> {
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
        return Err(Error::schema(
            "privacy_route_id must be an opaque route identifier",
        ));
    }

    Ok(())
}

fn validate_b3(field: &str, value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(Error::schema(format!(
            "{field} must be b3:<64 lowercase hex>"
        )));
    };

    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::schema(format!(
            "{field} must be b3:<64 lowercase hex>"
        )));
    }

    Ok(())
}

fn validate_authority_flags(
    accounting_accepted: bool,
    reward_eligible: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
) -> Result<()> {
    for (field, value) in [
        ("accounting_accepted", accounting_accepted),
        ("reward_eligible", reward_eligible),
        ("reward_truth", reward_truth),
        ("payout_authority", payout_authority),
        ("wallet_mutation", wallet_mutation),
        ("ledger_mutation", ledger_mutation),
    ] {
        if value {
            return Err(Error::schema(format!(
                "accounting evidence authority boundary violated: {field}"
            )));
        }
    }

    Ok(())
}
