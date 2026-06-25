//! RO:WHAT — Strict Phase 3 Round 2 validator lifecycle hardening DTOs.
//! RO:WHY — ECON/GOV: model rotation, revocation, downtime, evidence, and governance updates without validator economics.
//! RO:INTERACTS — validator_set DTOs, verifier attestations, future registry/passport/policy gates, ron-ledger read-only evaluators.
//! RO:INVARIANTS — DTO/validation only; no IO; no crypto execution; no settlement/finality; no staking/slashing; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — lifecycle/evidence fields are shape data only and grant no spend, unlock, bridge, staking, slashing, or settlement authority.
//! RO:TEST — tests/quickchain_phase3_validator_lifecycle.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    validate_chain_id, validate_epoch_id, validate_ref, validate_schema, validate_version,
    QuickChainResult, QuickChainValidationError, QuickChainVerifierReplayStatusV1,
};

/// Schema tag for a validator key/capability rotation request artifact.
pub const QUICKCHAIN_VALIDATOR_ROTATION_SCHEMA: &str = "quickchain.validator-rotation.v1";

/// Schema tag for a validator revocation request artifact.
pub const QUICKCHAIN_VALIDATOR_REVOCATION_SCHEMA: &str = "quickchain.validator-revocation.v1";

/// Schema tag for validator downtime/degraded status evidence.
pub const QUICKCHAIN_VALIDATOR_DOWNTIME_REPORT_SCHEMA: &str =
    "quickchain.validator-downtime-report.v1";

/// Schema tag for split-brain / double-attestation evidence.
pub const QUICKCHAIN_VALIDATOR_EQUIVOCATION_EVIDENCE_SCHEMA: &str =
    "quickchain.validator-equivocation-evidence.v1";

/// Schema tag for replay challenge evidence.
pub const QUICKCHAIN_REPLAY_CHALLENGE_EVIDENCE_SCHEMA: &str =
    "quickchain.replay-challenge-evidence.v1";

/// Schema tag for governance-gated validator parameter updates.
pub const QUICKCHAIN_VALIDATOR_PARAMETER_UPDATE_SCHEMA: &str =
    "quickchain.validator-parameter-update.v1";

/// Schema tag for read-only lifecycle hardening evaluator results.
pub const QUICKCHAIN_VALIDATOR_LIFECYCLE_DECISION_SCHEMA: &str =
    "quickchain.validator-lifecycle-decision.v1";

/// Validator lifecycle operation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorLifecycleOperationV1 {
    RotateValidator,
    RevokeValidator,
    MarkDowntime,
    RecordEquivocationEvidence,
    RecordReplayChallengeEvidence,
    GovernanceParameterUpdate,
}

/// Deterministic read-only lifecycle evaluator status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorLifecycleDecisionStatusV1 {
    Accepted,
    Rejected,
}

/// Deterministic read-only lifecycle evaluator rejection code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorLifecycleRejectionCodeV1 {
    ValidatorSetHashMismatch,
    ChainEpochMismatch,
    UnknownValidator,
    ValidatorNotActive,
    RevokedValidator,
    ExpiredValidator,
    PassportSubjectMismatch,
    CapabilityMismatch,
    KeyMismatch,
    RotationKeyUnchanged,
    RotationCapabilityUnchanged,
    NoConflictingAttestations,
    GovernanceApprovalMissing,
    BondedEconomicsForbidden,
}

/// Validator revocation reason shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorRevocationReasonV1 {
    PassportRevoked,
    RegistryRevoked,
    CapabilityRevoked,
    KeyCompromised,
    PolicyViolation,
    GovernanceAction,
}

/// Validator downtime/degraded status shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorDowntimeStatusV1 {
    Degraded,
    Down,
    Recovered,
}

/// Replay challenge evidence category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainReplayChallengeKindV1 {
    InvalidReplayResult,
    MissingReplayArtifact,
    MalformedAttestation,
    UnauthorizedValidator,
    DoubleAttestation,
}

/// Governance-gated validator parameter name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorGovernanceParameterV1 {
    MaxValidators,
    QuorumBps,
    ChallengeWindowMs,
    CapabilityMaxTtlMs,
    ValidatorSetAlgorithm,
}

/// Validator key/capability rotation artifact.
///
/// This artifact is shape/evidence data only. It does not rotate a live key by
/// itself and does not grant validation, wallet, finality, bridge, or settlement
/// authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorRotationV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub validator_id: String,
    pub passport_subject: String,
    pub old_key_id: String,
    pub new_key_id: String,
    pub old_capability_id: String,
    pub new_capability_id: String,
    pub effective_at_ms: u64,
    pub governance_approval_ref: String,
}

impl QuickChainValidatorRotationV1 {
    /// Validate rotation DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorRotationV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_ROTATION_SCHEMA,
        )?;
        validate_version("QuickChainValidatorRotationV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("passport_subject", &self.passport_subject)?;
        validate_ref("old_key_id", &self.old_key_id)?;
        validate_ref("new_key_id", &self.new_key_id)?;
        validate_ref("old_capability_id", &self.old_capability_id)?;
        validate_ref("new_capability_id", &self.new_capability_id)?;
        validate_ref("governance_approval_ref", &self.governance_approval_ref)?;
        validate_positive_ms("effective_at_ms", self.effective_at_ms)?;

        if self.old_key_id == self.new_key_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "new_key_id",
                reason: "rotation must change key id",
            });
        }

        if self.old_capability_id == self.new_capability_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "new_capability_id",
                reason: "rotation must change capability id",
            });
        }

        Ok(())
    }
}

/// Validator revocation artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorRevocationV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub validator_id: String,
    pub passport_subject: String,
    pub key_id: String,
    pub capability_id: String,
    pub reason: QuickChainValidatorRevocationReasonV1,
    pub revoked_at_ms: u64,
    pub governance_approval_ref: String,
}

impl QuickChainValidatorRevocationV1 {
    /// Validate revocation DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorRevocationV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_REVOCATION_SCHEMA,
        )?;
        validate_version("QuickChainValidatorRevocationV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("passport_subject", &self.passport_subject)?;
        validate_ref("key_id", &self.key_id)?;
        validate_ref("capability_id", &self.capability_id)?;
        validate_ref("governance_approval_ref", &self.governance_approval_ref)?;
        validate_positive_ms("revoked_at_ms", self.revoked_at_ms)
    }
}

/// Validator downtime/degraded status report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorDowntimeReportV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub validator_id: String,
    pub downtime_status: QuickChainValidatorDowntimeStatusV1,
    pub observed_at_ms: u64,
    pub evidence_ref: String,
}

impl QuickChainValidatorDowntimeReportV1 {
    /// Validate downtime DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorDowntimeReportV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_DOWNTIME_REPORT_SCHEMA,
        )?;
        validate_version("QuickChainValidatorDowntimeReportV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("evidence_ref", &self.evidence_ref)?;
        validate_positive_ms("observed_at_ms", self.observed_at_ms)
    }
}

/// Evidence that a validator signed conflicting replay attestations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorEquivocationEvidenceV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub validator_id: String,
    pub key_id: String,
    pub first_attestation_hash: ContentId,
    pub second_attestation_hash: ContentId,
    pub first_replay_result_hash: ContentId,
    pub second_replay_result_hash: ContentId,
    pub first_replay_status: QuickChainVerifierReplayStatusV1,
    pub second_replay_status: QuickChainVerifierReplayStatusV1,
    pub evidence_ref: String,
    pub observed_at_ms: u64,
}

impl QuickChainValidatorEquivocationEvidenceV1 {
    /// Validate equivocation evidence shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorEquivocationEvidenceV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_EQUIVOCATION_EVIDENCE_SCHEMA,
        )?;
        validate_version(
            "QuickChainValidatorEquivocationEvidenceV1.version",
            self.version,
        )?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("key_id", &self.key_id)?;
        validate_ref("evidence_ref", &self.evidence_ref)?;
        validate_positive_ms("observed_at_ms", self.observed_at_ms)?;

        if self.first_attestation_hash == self.second_attestation_hash {
            return Err(QuickChainValidationError::InvalidField {
                field: "second_attestation_hash",
                reason: "equivocation evidence requires two distinct attestations",
            });
        }

        if self.first_replay_result_hash == self.second_replay_result_hash
            && self.first_replay_status == self.second_replay_status
        {
            return Err(QuickChainValidationError::InvalidField {
                field: "second_replay_result_hash",
                reason: "equivocation evidence requires conflicting replay targets",
            });
        }

        Ok(())
    }
}

/// Evidence submitted to challenge a replay/attestation artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainReplayChallengeEvidenceV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub challenge_kind: QuickChainReplayChallengeKindV1,
    pub replay_bundle_hash: ContentId,
    pub disputed_replay_result_hash: ContentId,
    pub challenger_ref: String,
    pub evidence_ref: String,
    pub submitted_at_ms: u64,
}

impl QuickChainReplayChallengeEvidenceV1 {
    /// Validate replay challenge evidence shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainReplayChallengeEvidenceV1.schema",
            &self.schema,
            QUICKCHAIN_REPLAY_CHALLENGE_EVIDENCE_SCHEMA,
        )?;
        validate_version("QuickChainReplayChallengeEvidenceV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("challenger_ref", &self.challenger_ref)?;
        validate_ref("evidence_ref", &self.evidence_ref)?;
        validate_positive_ms("submitted_at_ms", self.submitted_at_ms)
    }
}

/// Governance-gated validator parameter update artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorParameterUpdateV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub parameter: QuickChainValidatorGovernanceParameterV1,
    pub previous_value_ref: String,
    pub new_value_ref: String,
    pub effective_epoch_id: String,
    pub governance_approval_ref: String,
}

impl QuickChainValidatorParameterUpdateV1 {
    /// Validate governance parameter update shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorParameterUpdateV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_PARAMETER_UPDATE_SCHEMA,
        )?;
        validate_version("QuickChainValidatorParameterUpdateV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_epoch_id(&self.effective_epoch_id)?;
        validate_ref("previous_value_ref", &self.previous_value_ref)?;
        validate_ref("new_value_ref", &self.new_value_ref)?;
        validate_ref("governance_approval_ref", &self.governance_approval_ref)?;

        if self.previous_value_ref == self.new_value_ref {
            return Err(QuickChainValidationError::InvalidField {
                field: "new_value_ref",
                reason: "parameter update must change the parameter value reference",
            });
        }

        Ok(())
    }
}

/// Read-only lifecycle hardening evaluator result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorLifecycleDecisionV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub validator_id: String,
    pub operation: QuickChainValidatorLifecycleOperationV1,
    pub status: QuickChainValidatorLifecycleDecisionStatusV1,
    pub rejection_code: Option<QuickChainValidatorLifecycleRejectionCodeV1>,
}

impl QuickChainValidatorLifecycleDecisionV1 {
    /// Validate decision shape and status/rejection consistency only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorLifecycleDecisionV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_LIFECYCLE_DECISION_SCHEMA,
        )?;
        validate_version(
            "QuickChainValidatorLifecycleDecisionV1.version",
            self.version,
        )?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;

        match self.status {
            QuickChainValidatorLifecycleDecisionStatusV1::Accepted => {
                if self.rejection_code.is_some() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason: "accepted lifecycle decision cannot carry a rejection code",
                    });
                }
            }
            QuickChainValidatorLifecycleDecisionStatusV1::Rejected => {
                if self.rejection_code.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason:
                            "rejected lifecycle decision requires a deterministic rejection code",
                    });
                }
            }
        }

        Ok(())
    }
}

fn validate_chain_epoch(chain_id: &str, epoch_id: &str) -> QuickChainResult<()> {
    validate_chain_id(chain_id)?;
    validate_epoch_id(epoch_id)
}

fn validate_positive_ms(field: &'static str, value: u64) -> QuickChainResult<()> {
    if value == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "must be greater than zero",
        });
    }

    Ok(())
}
