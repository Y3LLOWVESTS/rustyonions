//! RO:WHAT — FINAL_BETA Phase 19 finalized QuickChain checkpoint evidence DTO.
//! RO:WHY — Encodes threshold-backed checkpoint evidence after deterministic candidate reproduction and cryptographic committee review.
//! RO:INTERACTS — committee checkpoint payload, validator set, checkpoint-validator signatures, canonical QuickChain validation.
//! RO:INVARIANTS — candidate remains unchanged; validator set must match candidate; ceil(2N/3) active-member threshold; unique ordered verified signatures; finalized state only.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — protocol evidence only; does not itself perform cryptographic verification, fork choice, wallet/ledger mutation, settlement, bridge, staking, or client authority.
//! RO:TEST — tests/final_beta_phase19_finalized_checkpoint_contract.rs.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    validate_chain_id, validate_epoch_id, validate_schema, validate_version,
    QuickChainCheckpointValidatorSignatureV1, QuickChainCommitteeCheckpointPayloadV1,
    QuickChainResult, QuickChainValidationError, QuickChainValidatorLifecycleStatusV1,
    QuickChainValidatorSetV1,
};
use crate::ContentId;

/// Schema for threshold-backed finalized checkpoint evidence.
pub const QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA: &str = "quickchain.finalized-checkpoint.v1";

/// Finality state carried by the finalized-checkpoint evidence artifact.
///
/// The first Phase 19 contract intentionally contains only the state that this
/// artifact is allowed to claim. Non-finalized candidates use the separate
/// unsigned candidate/committee-review surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuickChainCheckpointFinalityStateV1 {
    /// Required validator threshold has been independently verified and the
    /// finalized-checkpoint artifact has been constructed from that review.
    Finalized,
}

/// Threshold-backed finalized checkpoint evidence.
///
/// This DTO is deliberately distinct from the checkpoint hash payload:
/// validator signatures do not participate in the deterministic checkpoint
/// candidate hash.
///
/// Structural validation proves internal consistency only. Construction of a
/// trustworthy instance still requires the cryptographic committee-review
/// layer to verify every included signature before this artifact is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainFinalizedCheckpointV1 {
    /// Exact finalized-checkpoint schema.
    pub schema: String,

    /// QuickChain DTO version.
    pub version: u16,

    /// Exact deterministic unsigned committee checkpoint payload.
    pub candidate: QuickChainCommitteeCheckpointPayloadV1,

    /// BLAKE3 content ID of the exact canonical candidate payload.
    pub checkpoint_hash: ContentId,

    /// Full reviewed validator set whose hash is committed by the candidate.
    pub validator_set: QuickChainValidatorSetV1,

    /// Deterministic ceil(2N/3) active-validator threshold.
    pub required_signatures: u32,

    /// Number of unique signatures already cryptographically verified by the
    /// committee-review layer.
    pub verified_unique_signatures: u32,

    /// Unique verified signatures sorted strictly by validator ID.
    pub signatures: Vec<QuickChainCheckpointValidatorSignatureV1>,

    /// Finalization state of this threshold-backed artifact.
    pub finalization_state: QuickChainCheckpointFinalityStateV1,
}

impl QuickChainFinalizedCheckpointV1 {
    /// Validate finalized checkpoint evidence structure.
    ///
    /// This validates candidate/set/signature bindings and the deterministic
    /// active-member threshold. It intentionally does not perform public-key
    /// cryptographic verification itself.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainFinalizedCheckpointV1.schema",
            &self.schema,
            QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA,
        )?;

        validate_version("QuickChainFinalizedCheckpointV1.version", self.version)?;

        self.candidate.validate()?;
        self.validator_set.validate()?;

        validate_chain_id(&self.candidate.chain_id)?;
        validate_epoch_id(&self.candidate.epoch_id)?;

        if self.candidate.height == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "candidate.height",
                reason: "finalized checkpoint height must be greater than zero",
            });
        }

        if self.validator_set.chain_id != self.candidate.chain_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "validator_set.chain_id",
                reason: "must match finalized checkpoint candidate chain",
            });
        }

        if self.validator_set.epoch_id != self.candidate.epoch_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "validator_set.epoch_id",
                reason: "must match finalized checkpoint candidate epoch",
            });
        }

        if self.validator_set.validator_set_hash != self.candidate.validator_set_hash {
            return Err(QuickChainValidationError::InvalidField {
                field: "validator_set.validator_set_hash",
                reason: "must match candidate validator_set_hash commitment",
            });
        }

        let active_members_usize = self
            .validator_set
            .members
            .iter()
            .filter(|member| {
                member.lifecycle_status == QuickChainValidatorLifecycleStatusV1::Active
            })
            .count();

        let active_members = u32::try_from(active_members_usize).map_err(|_| {
            QuickChainValidationError::InvalidField {
                field: "validator_set.members",
                reason: "active validator count exceeds finalized checkpoint wire range",
            }
        })?;

        if active_members < 2 {
            return Err(QuickChainValidationError::InvalidField {
                field: "validator_set.members",
                reason: "finality requires at least two active validators",
            });
        }

        let doubled =
            active_members
                .checked_mul(2)
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "required_signatures",
                    reason: "finality threshold arithmetic overflow",
                })?;

        let numerator = doubled
            .checked_add(2)
            .ok_or(QuickChainValidationError::InvalidField {
                field: "required_signatures",
                reason: "finality threshold arithmetic overflow",
            })?;

        let expected_required = numerator / 3;

        if self.required_signatures != expected_required {
            return Err(QuickChainValidationError::InvalidField {
                field: "required_signatures",
                reason: "must equal ceil(2N/3) of active validator members",
            });
        }

        if self.verified_unique_signatures < self.required_signatures {
            return Err(QuickChainValidationError::InvalidField {
                field: "verified_unique_signatures",
                reason: "must satisfy finalized checkpoint threshold",
            });
        }

        if self.verified_unique_signatures > active_members {
            return Err(QuickChainValidationError::InvalidField {
                field: "verified_unique_signatures",
                reason: "cannot exceed active validator count",
            });
        }

        let signature_count = u32::try_from(self.signatures.len()).map_err(|_| {
            QuickChainValidationError::InvalidField {
                field: "signatures",
                reason: "signature count exceeds finalized checkpoint wire range",
            }
        })?;

        if signature_count != self.verified_unique_signatures {
            return Err(QuickChainValidationError::InvalidField {
                field: "signatures",
                reason: "signature count must equal verified_unique_signatures",
            });
        }

        let mut seen_validator_ids = HashSet::<&str>::with_capacity(self.signatures.len());
        let mut seen_key_ids = HashSet::<&str>::with_capacity(self.signatures.len());
        let mut previous_validator_id: Option<&str> = None;

        for signature in &self.signatures {
            signature.validate()?;

            if signature.chain_id != self.candidate.chain_id {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.chain_id",
                    reason: "must match finalized checkpoint candidate chain",
                });
            }

            if signature.height != self.candidate.height {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.height",
                    reason: "must match finalized checkpoint candidate height",
                });
            }

            if signature.epoch_id != self.candidate.epoch_id {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.epoch_id",
                    reason: "must match finalized checkpoint candidate epoch",
                });
            }

            if signature.checkpoint_hash != self.checkpoint_hash {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.checkpoint_hash",
                    reason: "must match finalized checkpoint hash",
                });
            }

            let member = self
                .validator_set
                .members
                .iter()
                .find(|member| member.validator_id == signature.validator_id)
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "signatures.validator_id",
                    reason: "signature validator must exist in reviewed validator set",
                })?;

            if member.lifecycle_status != QuickChainValidatorLifecycleStatusV1::Active {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.validator_id",
                    reason: "signature validator must be active",
                });
            }

            if member.key_id != signature.key_id {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.key_id",
                    reason: "signature key must match reviewed validator member",
                });
            }

            if member.signature_algorithm != signature.algorithm {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.algorithm",
                    reason: "signature algorithm must match reviewed validator member",
                });
            }

            if !seen_validator_ids.insert(signature.validator_id.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.validator_id",
                    reason: "duplicate validator signatures are forbidden",
                });
            }

            if !seen_key_ids.insert(signature.key_id.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "signatures.key_id",
                    reason: "duplicate signing keys are forbidden",
                });
            }

            if let Some(previous) = previous_validator_id {
                if previous >= signature.validator_id.as_str() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "signatures",
                        reason: "signatures must be strictly sorted by validator_id",
                    });
                }
            }

            previous_validator_id = Some(signature.validator_id.as_str());
        }

        Ok(())
    }
}
