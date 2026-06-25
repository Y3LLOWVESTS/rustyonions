//! RO:WHAT — Strict Phase 3 passport-gated validator-set, capability, and attestation-authorization DTOs.
//! RO:WHY — ECON/GOV: turn Phase 2 committee attestations into passport/registry-gated membership without validator economics.
//! RO:INTERACTS — verifier attestations, ContentId, SignatureAlg, future svc-passport/svc-registry/ron-policy gates, ron-ledger read-only evaluator.
//! RO:INVARIANTS — DTO/validation only; no IO; no crypto execution; no settlement/finality; no staking/slashing; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — passport/capability fields are shape data only and grant no spend, unlock, bridge, staking, or settlement authority.
//! RO:TEST — tests/quickchain_phase3_validator_set.rs.

use serde::{Deserialize, Serialize};

use crate::{id::ContentId, quantum::SignatureAlg};

use super::{
    validate_bounded_nonempty, validate_chain_id, validate_epoch_id, validate_ref, validate_schema,
    validate_version, QuickChainResult, QuickChainValidationError, QuickChainVerifierAttestationV1,
    QuickChainVerifierReplayStatusV1, MAX_QUICKCHAIN_REF_BYTES,
};

/// Schema tag for one passport/registry-known validator identity.
pub const QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA: &str = "quickchain.validator-identity.v1";

/// Schema tag for one validator capability grant.
pub const QUICKCHAIN_VALIDATOR_CAPABILITY_SCHEMA: &str = "quickchain.validator-capability.v1";

/// Schema tag for one passport-gated validator set snapshot.
pub const QUICKCHAIN_VALIDATOR_SET_SCHEMA: &str = "quickchain.validator-set.v1";

/// Schema tag for one validator-attestation authorization request.
pub const QUICKCHAIN_VALIDATOR_AUTHORIZATION_REQUEST_SCHEMA: &str =
    "quickchain.validator-authorization-request.v1";

/// Schema tag for one validator-attestation authorization result.
pub const QUICKCHAIN_VALIDATOR_AUTHORIZATION_RESULT_SCHEMA: &str =
    "quickchain.validator-authorization-result.v1";

/// Phase 3 Round 1 validator-set algorithm token.
pub const QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1: &str =
    "passport_registry_validator_set_v1";

/// Capability scope that may authorize verification attestations only.
pub const QUICKCHAIN_VALIDATOR_CAPABILITY_SCOPE_VERIFY_V1: &str = "quickchain.validator.verify.v1";

/// Maximum validator identities carried in one Phase 3 validator-set artifact.
pub const MAX_QUICKCHAIN_PASSPORT_VALIDATORS: usize = 128;

/// Passport/registry lifecycle state for one validator identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorLifecycleStatusV1 {
    /// Identity exists but is not yet authorized to attest.
    Pending,
    /// Identity is currently eligible to attest if its capability is valid.
    Active,
    /// Identity was revoked by registry/policy/passport governance.
    Revoked,
    /// Identity/capability window has expired.
    Expired,
}

/// Authorization status for a Phase 3 validator-attestation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorAuthorizationStatusV1 {
    /// The attestation identity is authorized by the supplied passport-gated set.
    Authorized,
    /// The attestation identity is rejected by the supplied passport-gated set.
    Rejected,
}

/// Deterministic rejection codes for passport-gated validator authorization.
///
/// These codes are diagnostic only. They do not slash, stake, settle, bridge,
/// mutate balances, unlock paid content, or create finality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainValidatorAuthorizationRejectionCodeV1 {
    /// The validator set did not require passport gating.
    PassportRequired,
    /// Bonded/high-stakes economics were present before Phase 4 authorization.
    BondedEconomicsForbidden,
    /// The attesting identity is not in the supplied validator set.
    UnauthorizedIdentity,
    /// The identity exists but is not active.
    ValidatorNotActive,
    /// The validator or capability has been revoked.
    RevokedValidator,
    /// The capability is not valid yet at the request evaluation time.
    CapabilityNotYetValid,
    /// The capability or identity has expired at the request evaluation time.
    ExpiredCapability,
    /// The validator-set hash in the request does not match the supplied set.
    ValidatorSetHashMismatch,
    /// The attestation/capability/set chain or epoch does not match.
    ChainEpochMismatch,
    /// The capability scope is not the verification-attestation scope.
    CapabilityScopeMismatch,
    /// The passport subject does not match the registry identity.
    PassportSubjectMismatch,
    /// The registry entry does not match the validator identity.
    RegistryEntryMismatch,
    /// The capability id does not match the validator identity.
    CapabilityMismatch,
    /// The key id or signature algorithm does not match.
    KeyMismatch,
    /// The attestation targets a different replay result hash.
    ReplayResultHashMismatch,
    /// The attestation targets a different replay status.
    ReplayStatusMismatch,
}

/// One passport/registry-known validator identity.
///
/// This is membership shape only. It is not a wallet, balance, stake, slash,
/// finality, bridge, or public-chain authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorIdentityV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_id: String,
    pub passport_subject: String,
    pub registry_entry_id: String,
    pub key_id: String,
    pub capability_id: String,
    pub signature_algorithm: SignatureAlg,
    pub lifecycle_status: QuickChainValidatorLifecycleStatusV1,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
}

impl QuickChainValidatorIdentityV1 {
    /// Validate identity DTO shape and explicit time bounds only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorIdentityV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
        )?;
        validate_version("QuickChainValidatorIdentityV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("passport_subject", &self.passport_subject)?;
        validate_ref("registry_entry_id", &self.registry_entry_id)?;
        validate_ref("key_id", &self.key_id)?;
        validate_ref("capability_id", &self.capability_id)?;
        validate_activation_window(self.not_before_ms, self.expires_at_ms)
    }
}

/// One capability grant for verification attestations.
///
/// This DTO is shape data only. `ron-proto` does not verify signatures and does
/// not decide whether a passport is real.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorCapabilityV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_id: String,
    pub passport_subject: String,
    pub registry_entry_id: String,
    pub key_id: String,
    pub capability_id: String,
    pub capability_scope: String,
    pub signature_algorithm: SignatureAlg,
    pub issued_at_ms: u64,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
    pub revoked_at_ms: Option<u64>,
}

impl QuickChainValidatorCapabilityV1 {
    /// Validate capability DTO shape and explicit time bounds only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorCapabilityV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_CAPABILITY_SCHEMA,
        )?;
        validate_version("QuickChainValidatorCapabilityV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("passport_subject", &self.passport_subject)?;
        validate_ref("registry_entry_id", &self.registry_entry_id)?;
        validate_ref("key_id", &self.key_id)?;
        validate_ref("capability_id", &self.capability_id)?;
        validate_capability_scope(&self.capability_scope)?;

        if self.issued_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "issued_at_ms",
                reason: "must be greater than zero",
            });
        }

        if self.not_before_ms < self.issued_at_ms {
            return Err(QuickChainValidationError::InvalidField {
                field: "not_before_ms",
                reason: "must be greater than or equal to issued_at_ms",
            });
        }

        validate_activation_window(self.not_before_ms, self.expires_at_ms)?;

        if let Some(revoked_at_ms) = self.revoked_at_ms {
            if revoked_at_ms == 0 {
                return Err(QuickChainValidationError::InvalidField {
                    field: "revoked_at_ms",
                    reason: "must be greater than zero when present",
                });
            }

            if revoked_at_ms < self.issued_at_ms {
                return Err(QuickChainValidationError::InvalidField {
                    field: "revoked_at_ms",
                    reason: "must be greater than or equal to issued_at_ms when present",
                });
            }
        }

        Ok(())
    }
}

/// Passport-gated validator set snapshot.
///
/// The hash fields are references supplied by callers. This DTO does not compute
/// validator-set hashes or policy/registry hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorSetV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub policy_hash: ContentId,
    pub registry_snapshot_hash: ContentId,
    pub passport_required: bool,
    pub bond_required: bool,
    pub validator_set_algorithm: String,
    pub members: Vec<QuickChainValidatorIdentityV1>,
}

impl QuickChainValidatorSetV1 {
    /// Validate validator-set DTO shape and deterministic uniqueness.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorSetV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_SET_SCHEMA,
        )?;
        validate_version("QuickChainValidatorSetV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_validator_set_algorithm(&self.validator_set_algorithm)?;

        if !self.passport_required {
            return Err(QuickChainValidationError::InvalidField {
                field: "passport_required",
                reason: "Phase 3 Round 1 validator sets must require passports",
            });
        }

        if self.bond_required {
            return Err(QuickChainValidationError::InvalidField {
                field: "bond_required",
                reason: "bonded validator economics are not authorized in Phase 3 Round 1",
            });
        }

        if self.members.is_empty() {
            return Err(QuickChainValidationError::InvalidField {
                field: "members",
                reason: "validator set must contain at least one identity",
            });
        }

        if self.members.len() > MAX_QUICKCHAIN_PASSPORT_VALIDATORS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "members",
                max: MAX_QUICKCHAIN_PASSPORT_VALIDATORS,
                actual: self.members.len(),
            });
        }

        let mut validator_ids = Vec::<&str>::with_capacity(self.members.len());
        let mut passport_subjects = Vec::<&str>::with_capacity(self.members.len());
        let mut capability_ids = Vec::<&str>::with_capacity(self.members.len());

        for member in &self.members {
            member.validate()?;
            ensure_chain_epoch(
                "members",
                &member.chain_id,
                &member.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            if validator_ids.contains(&member.validator_id.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "members.validator_id",
                    reason: "validator ids must be unique",
                });
            }

            if passport_subjects.contains(&member.passport_subject.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "members.passport_subject",
                    reason: "passport subjects must be unique in one validator set",
                });
            }

            if capability_ids.contains(&member.capability_id.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "members.capability_id",
                    reason: "capability ids must be unique in one validator set",
                });
            }

            validator_ids.push(member.validator_id.as_str());
            passport_subjects.push(member.passport_subject.as_str());
            capability_ids.push(member.capability_id.as_str());
        }

        Ok(())
    }
}

/// Request to authorize one verification attestation against a passport-gated set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorAuthorizationRequestV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub replay_result_hash: ContentId,
    pub replay_status: QuickChainVerifierReplayStatusV1,
    pub evaluation_time_ms: u64,
    pub validator_set: QuickChainValidatorSetV1,
    pub attestation: QuickChainVerifierAttestationV1,
    pub capability: QuickChainValidatorCapabilityV1,
}

impl QuickChainValidatorAuthorizationRequestV1 {
    /// Validate request shape and cross-artifact identity bindings.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorAuthorizationRequestV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_AUTHORIZATION_REQUEST_SCHEMA,
        )?;
        validate_version(
            "QuickChainValidatorAuthorizationRequestV1.version",
            self.version,
        )?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;

        if self.evaluation_time_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "evaluation_time_ms",
                reason: "must be greater than zero",
            });
        }

        self.validator_set.validate()?;
        self.attestation.validate()?;
        self.capability.validate()?;

        if self.validator_set.validator_set_hash != self.validator_set_hash {
            return Err(QuickChainValidationError::InvalidField {
                field: "validator_set_hash",
                reason: "request validator_set_hash must match supplied validator set",
            });
        }

        ensure_chain_epoch(
            "validator_set",
            &self.validator_set.chain_id,
            &self.validator_set.epoch_id,
            &self.chain_id,
            &self.epoch_id,
        )?;
        ensure_chain_epoch(
            "attestation",
            &self.attestation.chain_id,
            &self.attestation.epoch_id,
            &self.chain_id,
            &self.epoch_id,
        )?;
        ensure_chain_epoch(
            "capability",
            &self.capability.chain_id,
            &self.capability.epoch_id,
            &self.chain_id,
            &self.epoch_id,
        )?;

        if self.attestation.replay_result_hash != self.replay_result_hash {
            return Err(QuickChainValidationError::InvalidField {
                field: "attestation.replay_result_hash",
                reason: "attestation replay result hash must match request target",
            });
        }

        if self.attestation.replay_status != self.replay_status {
            return Err(QuickChainValidationError::InvalidField {
                field: "attestation.replay_status",
                reason: "attestation replay status must match request target",
            });
        }

        if self.attestation.committee_member_id != self.capability.validator_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "attestation.committee_member_id",
                reason: "Phase 3 attestation member id must match capability validator id",
            });
        }

        Ok(())
    }
}

/// Result of a passport-gated validator-attestation authorization check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorAuthorizationResultV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub validator_id: String,
    pub passport_subject: String,
    pub key_id: String,
    pub replay_result_hash: ContentId,
    pub replay_status: QuickChainVerifierReplayStatusV1,
    pub status: QuickChainValidatorAuthorizationStatusV1,
    pub rejection_code: Option<QuickChainValidatorAuthorizationRejectionCodeV1>,
}

impl QuickChainValidatorAuthorizationResultV1 {
    /// Validate authorization-result shape and status/rejection consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorAuthorizationResultV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_AUTHORIZATION_RESULT_SCHEMA,
        )?;
        validate_version(
            "QuickChainValidatorAuthorizationResultV1.version",
            self.version,
        )?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("passport_subject", &self.passport_subject)?;
        validate_ref("key_id", &self.key_id)?;

        match self.status {
            QuickChainValidatorAuthorizationStatusV1::Authorized => {
                if self.rejection_code.is_some() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason: "authorized result cannot carry a rejection code",
                    });
                }
            }
            QuickChainValidatorAuthorizationStatusV1::Rejected => {
                if self.rejection_code.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason: "rejected result requires a deterministic rejection code",
                    });
                }
            }
        }

        Ok(())
    }
}

fn validate_validator_set_algorithm(value: &str) -> QuickChainResult<()> {
    validate_bounded_nonempty("validator_set_algorithm", value, MAX_QUICKCHAIN_REF_BYTES)?;

    if value != QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "validator_set_algorithm",
            reason: "unsupported QuickChain validator-set algorithm",
        });
    }

    Ok(())
}

fn validate_capability_scope(value: &str) -> QuickChainResult<()> {
    validate_ref("capability_scope", value)?;

    if value != QUICKCHAIN_VALIDATOR_CAPABILITY_SCOPE_VERIFY_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "capability_scope",
            reason: "unsupported QuickChain validator capability scope",
        });
    }

    Ok(())
}

fn validate_activation_window(not_before_ms: u64, expires_at_ms: u64) -> QuickChainResult<()> {
    if not_before_ms == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field: "not_before_ms",
            reason: "must be greater than zero",
        });
    }

    if expires_at_ms == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field: "expires_at_ms",
            reason: "must be greater than zero",
        });
    }

    if expires_at_ms <= not_before_ms {
        return Err(QuickChainValidationError::InvalidField {
            field: "expires_at_ms",
            reason: "must be greater than not_before_ms",
        });
    }

    Ok(())
}

fn ensure_chain_epoch(
    field: &'static str,
    chain_id: &str,
    epoch_id: &str,
    expected_chain_id: &str,
    expected_epoch_id: &str,
) -> QuickChainResult<()> {
    if chain_id != expected_chain_id {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "chain_id must match request chain_id",
        });
    }

    if epoch_id != expected_epoch_id {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "epoch_id must match request epoch_id",
        });
    }

    Ok(())
}
