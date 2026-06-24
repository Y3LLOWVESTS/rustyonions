//! RO:WHAT — Strict QuickChain verifier replay-bundle and replay-result DTOs.
//! RO:WHY — ECON/GOV: Phase 2 starts with independently replayable artifacts before committee signing or quorum semantics.
//! RO:INTERACTS — root_material DTOs, tree roots, inclusion proofs, future read-only replicated replay.
//! RO:INVARIANTS — DTO/validation only; no IO; no crypto execution; no settlement/finality; no signing; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — verifier replay status is diagnostic only and grants no spend, receipt, bridge, or settlement authority.
//! RO:TEST — tests/quickchain_verifier_replay.rs.

use serde::{Deserialize, Serialize};

use crate::{id::ContentId, quantum::SignatureAlg};

use super::{
    validate_bounded_nonempty, validate_chain_id, validate_epoch_id, validate_ref, validate_schema,
    validate_version, QuickChainResult, QuickChainTreeInclusionProofV1,
    QuickChainTreeMaterialBatchV1, QuickChainTreeMaterialKindV1, QuickChainTreeRootV1,
    QuickChainValidationError, MAX_QUICKCHAIN_SIGNATURE_BYTES,
};

/// Schema tag for a Phase 2 verifier replay bundle.
pub const QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA: &str = "quickchain.verifier-replay-bundle.v1";

/// Schema tag for one read-only root replay check.
pub const QUICKCHAIN_VERIFIER_ROOT_CHECK_SCHEMA: &str = "quickchain.verifier-root-check.v1";

/// Schema tag for one read-only inclusion-proof replay check.
pub const QUICKCHAIN_VERIFIER_PROOF_CHECK_SCHEMA: &str = "quickchain.verifier-proof-check.v1";

/// Schema tag for a Phase 2 read-only verifier replay result.
pub const QUICKCHAIN_VERIFIER_REPLAY_RESULT_SCHEMA: &str = "quickchain.verifier-replay-result.v1";

/// Schema tag for one committee member identity in Phase 2 readiness DTOs.
pub const QUICKCHAIN_COMMITTEE_MEMBER_SCHEMA: &str = "quickchain.committee-member.v1";

/// Schema tag for one shape-checked verification attestation.
pub const QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA: &str = "quickchain.verifier-attestation.v1";

/// Schema tag for a bounded committee attestation set.
pub const QUICKCHAIN_COMMITTEE_ATTESTATION_SET_SCHEMA: &str =
    "quickchain.committee-attestation-set.v1";

/// Schema tag for a committee readiness evaluation result.
pub const QUICKCHAIN_COMMITTEE_READINESS_RESULT_SCHEMA: &str =
    "quickchain.committee-readiness-result.v1";

/// Schema tag for the payload a committee member claims to sign.
pub const QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA: &str =
    "quickchain.verifier-attestation-payload.v1";

/// Phase 2 Round 2 bounded committee agreement/readiness algorithm token.
pub const QUICKCHAIN_COMMITTEE_AGREEMENT_ALGORITHM_BOUNDED_V1: &str =
    "bounded_committee_replay_attestation_v1";

/// Maximum committee members carried in one Phase 2 readiness artifact.
pub const MAX_QUICKCHAIN_COMMITTEE_MEMBERS: usize = 128;

/// Maximum attestations carried in one Phase 2 readiness artifact.
pub const MAX_QUICKCHAIN_COMMITTEE_ATTESTATIONS: usize = 128;

/// Maximum committee agreement algorithm token length.
pub const MAX_QUICKCHAIN_COMMITTEE_ALGORITHM_BYTES: usize = 128;

/// Phase 2 Round 1 read-only verifier replay algorithm token.
pub const QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1: &str =
    "read_only_root_and_proof_replay_v1";

/// Number of tree kinds currently modeled by QuickChain tree material DTOs.
pub const MAX_QUICKCHAIN_VERIFIER_TREE_KINDS: usize = 5;

/// Maximum inclusion proofs carried by one replay bundle.
pub const MAX_QUICKCHAIN_VERIFIER_INCLUSION_PROOFS: usize = 16_384;

/// Maximum replay checks carried by one result.
pub const MAX_QUICKCHAIN_VERIFIER_REPLAY_CHECKS: usize = 16_384;

/// Maximum replay algorithm token length.
pub const MAX_QUICKCHAIN_VERIFIER_ALGORITHM_BYTES: usize = 128;

/// Maximum diagnostic detail string length.
pub const MAX_QUICKCHAIN_VERIFIER_DETAIL_BYTES: usize = 256;

/// One deterministic replay check status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainVerifierCheckStatusV1 {
    /// Recomputed artifact matched the expected artifact.
    Verified,
    /// Recomputed artifact did not match the expected artifact.
    Mismatch,
}

/// Whole-bundle deterministic replay status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainVerifierReplayStatusV1 {
    /// Every root/proof check matched.
    Verified,
    /// At least one root/proof check did not match.
    Mismatch,
}

/// Read-only Phase 2 replay bundle.
///
/// This DTO carries all artifacts needed for a verifier implementation to
/// recompute roots and verify inclusion proofs. It does not carry committee
/// signatures, quorum claims, finality, bridge state, staking state, or spend
/// authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainVerifierReplayBundleV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Read-only replay algorithm token.
    pub replay_algorithm: String,
    /// Material batches to replay.
    #[serde(default)]
    pub material_batches: Vec<QuickChainTreeMaterialBatchV1>,
    /// Expected roots for the material batches.
    #[serde(default)]
    pub expected_roots: Vec<QuickChainTreeRootV1>,
    /// Optional inclusion proofs to verify against expected roots.
    #[serde(default)]
    pub inclusion_proofs: Vec<QuickChainTreeInclusionProofV1>,
}

impl QuickChainVerifierReplayBundleV1 {
    /// Validate bundle DTO shape and internal cross-artifact consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainVerifierReplayBundleV1.schema",
            &self.schema,
            QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA,
        )?;
        validate_version("QuickChainVerifierReplayBundleV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_replay_algorithm(&self.replay_algorithm)?;

        if self.material_batches.is_empty() {
            return Err(QuickChainValidationError::InvalidField {
                field: "material_batches",
                reason: "replay bundle requires at least one material batch",
            });
        }

        if self.material_batches.len() > MAX_QUICKCHAIN_VERIFIER_TREE_KINDS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "material_batches",
                max: MAX_QUICKCHAIN_VERIFIER_TREE_KINDS,
                actual: self.material_batches.len(),
            });
        }

        if self.expected_roots.len() != self.material_batches.len() {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_roots",
                reason: "replay bundle requires exactly one expected root per material batch",
            });
        }

        if self.inclusion_proofs.len() > MAX_QUICKCHAIN_VERIFIER_INCLUSION_PROOFS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "inclusion_proofs",
                max: MAX_QUICKCHAIN_VERIFIER_INCLUSION_PROOFS,
                actual: self.inclusion_proofs.len(),
            });
        }

        let mut material_trees = Vec::<&'static str>::with_capacity(self.material_batches.len());

        for batch in &self.material_batches {
            batch.validate()?;
            ensure_chain_epoch(
                "material_batches",
                &batch.chain_id,
                &batch.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            let key = tree_key(batch.tree);
            if material_trees.contains(&key) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "material_batches.tree",
                    reason: "replay bundle cannot contain duplicate material trees",
                });
            }

            material_trees.push(key);
        }

        let mut root_trees = Vec::<&'static str>::with_capacity(self.expected_roots.len());

        for root in &self.expected_roots {
            root.validate()?;
            ensure_chain_epoch(
                "expected_roots",
                &root.chain_id,
                &root.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            let key = tree_key(root.tree);
            if root_trees.contains(&key) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "expected_roots.tree",
                    reason: "replay bundle cannot contain duplicate expected roots",
                });
            }

            if !material_trees.contains(&key) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "expected_roots.tree",
                    reason: "each expected root must correspond to a material batch",
                });
            }

            let Some(material) = material_batch_for_tree(&self.material_batches, root.tree) else {
                return Err(QuickChainValidationError::InvalidField {
                    field: "expected_roots.tree",
                    reason: "each expected root must correspond to a material batch",
                });
            };

            if root.source_items_count != material.items.len() as u64 {
                return Err(QuickChainValidationError::InvalidField {
                    field: "expected_roots.source_items_count",
                    reason: "expected root source count must match its material batch",
                });
            }

            root_trees.push(key);
        }

        for proof in &self.inclusion_proofs {
            proof.validate()?;
            ensure_chain_epoch(
                "inclusion_proofs",
                &proof.chain_id,
                &proof.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            let key = tree_key(proof.tree);
            if !material_trees.contains(&key) || !root_trees.contains(&key) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "inclusion_proofs.tree",
                    reason: "proof tree must correspond to material and expected root artifacts",
                });
            }

            let Some(material) = material_batch_for_tree(&self.material_batches, proof.tree) else {
                return Err(QuickChainValidationError::InvalidField {
                    field: "inclusion_proofs.tree",
                    reason: "proof tree must correspond to a material batch",
                });
            };

            if proof.source_items_count != material.items.len() as u64 {
                return Err(QuickChainValidationError::InvalidField {
                    field: "inclusion_proofs.source_items_count",
                    reason: "proof source count must match its material batch",
                });
            }

            let Some(root) = expected_root_for_tree(&self.expected_roots, proof.tree) else {
                return Err(QuickChainValidationError::InvalidField {
                    field: "inclusion_proofs.tree",
                    reason: "proof tree must correspond to an expected root",
                });
            };

            if proof.root_hash != root.root_hash {
                return Err(QuickChainValidationError::InvalidField {
                    field: "inclusion_proofs.root_hash",
                    reason: "proof root hash must match the expected root for its tree",
                });
            }
        }

        Ok(())
    }
}

/// One recomputed root comparison from a read-only verifier run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainVerifierRootCheckV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Tree kind checked.
    pub tree: QuickChainTreeMaterialKindV1,
    /// Expected root hash from the replay bundle.
    pub expected_root_hash: ContentId,
    /// Root hash recomputed by the verifier.
    pub recomputed_root_hash: ContentId,
    /// Deterministic comparison status.
    pub status: QuickChainVerifierCheckStatusV1,
    /// Optional bounded diagnostic detail.
    #[serde(default)]
    pub detail: Option<String>,
}

impl QuickChainVerifierRootCheckV1 {
    /// Validate root-check DTO shape and status/hash consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainVerifierRootCheckV1.schema",
            &self.schema,
            QUICKCHAIN_VERIFIER_ROOT_CHECK_SCHEMA,
        )?;
        validate_version("QuickChainVerifierRootCheckV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_optional_detail("QuickChainVerifierRootCheckV1.detail", &self.detail)?;

        match self.status {
            QuickChainVerifierCheckStatusV1::Verified => {
                if self.expected_root_hash != self.recomputed_root_hash {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason:
                            "verified root checks must carry equal expected and recomputed roots",
                    });
                }
            }
            QuickChainVerifierCheckStatusV1::Mismatch => {
                if self.expected_root_hash == self.recomputed_root_hash {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason: "mismatch root checks must carry different expected and recomputed roots",
                    });
                }
            }
        }

        Ok(())
    }
}

/// One inclusion-proof verification comparison from a read-only verifier run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainVerifierProofCheckV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Tree kind checked.
    pub tree: QuickChainTreeMaterialKindV1,
    /// Leaf sort key checked.
    pub leaf_sort_key_hex: String,
    /// Root hash targeted by this proof.
    pub root_hash: ContentId,
    /// Deterministic proof-check status.
    pub status: QuickChainVerifierCheckStatusV1,
    /// Optional bounded diagnostic detail.
    #[serde(default)]
    pub detail: Option<String>,
}

impl QuickChainVerifierProofCheckV1 {
    /// Validate proof-check DTO shape.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainVerifierProofCheckV1.schema",
            &self.schema,
            QUICKCHAIN_VERIFIER_PROOF_CHECK_SCHEMA,
        )?;
        validate_version("QuickChainVerifierProofCheckV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_lower_hex_sort_key(&self.leaf_sort_key_hex)?;
        validate_optional_detail("QuickChainVerifierProofCheckV1.detail", &self.detail)
    }
}

/// Result of one read-only replay-bundle verification run.
///
/// This result is a reproducibility report only. It is not finality, not quorum,
/// not a signature set, and not settlement authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainVerifierReplayResultV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Read-only replay algorithm token.
    pub replay_algorithm: String,
    /// Whole-bundle status.
    pub status: QuickChainVerifierReplayStatusV1,
    /// Count copied from the checked bundle.
    pub material_batches_count: u64,
    /// Count copied from the checked bundle.
    pub expected_roots_count: u64,
    /// Count copied from the checked bundle.
    pub inclusion_proofs_count: u64,
    /// Root replay checks.
    #[serde(default)]
    pub root_checks: Vec<QuickChainVerifierRootCheckV1>,
    /// Inclusion-proof replay checks.
    #[serde(default)]
    pub proof_checks: Vec<QuickChainVerifierProofCheckV1>,
}

impl QuickChainVerifierReplayResultV1 {
    /// Validate replay-result DTO shape and status/check consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainVerifierReplayResultV1.schema",
            &self.schema,
            QUICKCHAIN_VERIFIER_REPLAY_RESULT_SCHEMA,
        )?;
        validate_version("QuickChainVerifierReplayResultV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_replay_algorithm(&self.replay_algorithm)?;

        if self.material_batches_count == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "material_batches_count",
                reason: "replay result must report at least one material batch",
            });
        }

        if self.root_checks.len() > MAX_QUICKCHAIN_VERIFIER_REPLAY_CHECKS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "root_checks",
                max: MAX_QUICKCHAIN_VERIFIER_REPLAY_CHECKS,
                actual: self.root_checks.len(),
            });
        }

        if self.proof_checks.len() > MAX_QUICKCHAIN_VERIFIER_REPLAY_CHECKS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "proof_checks",
                max: MAX_QUICKCHAIN_VERIFIER_REPLAY_CHECKS,
                actual: self.proof_checks.len(),
            });
        }

        if self.root_checks.len() as u64 != self.expected_roots_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "root_checks",
                reason: "root check count must equal expected root count",
            });
        }

        if self.proof_checks.len() as u64 != self.inclusion_proofs_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "proof_checks",
                reason: "proof check count must equal inclusion proof count",
            });
        }

        if self.expected_roots_count != self.material_batches_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_roots_count",
                reason: "expected root count must equal material batch count",
            });
        }

        let mut any_mismatch = false;

        for check in &self.root_checks {
            check.validate()?;
            ensure_chain_epoch(
                "root_checks",
                &check.chain_id,
                &check.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            if check.status == QuickChainVerifierCheckStatusV1::Mismatch {
                any_mismatch = true;
            }
        }

        for check in &self.proof_checks {
            check.validate()?;
            ensure_chain_epoch(
                "proof_checks",
                &check.chain_id,
                &check.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            if check.status == QuickChainVerifierCheckStatusV1::Mismatch {
                any_mismatch = true;
            }
        }

        match self.status {
            QuickChainVerifierReplayStatusV1::Verified => {
                if any_mismatch {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason: "verified replay results cannot contain mismatched checks",
                    });
                }
            }
            QuickChainVerifierReplayStatusV1::Mismatch => {
                if !any_mismatch {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason: "mismatch replay results require at least one mismatched check",
                    });
                }
            }
        }

        Ok(())
    }
}

/// Committee readiness state for Phase 2 Round 2.
///
/// This is not finality. It is only a bounded statement about whether enough
/// shape-checked committee attestations are present for a replay result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainCommitteeReadinessStatusV1 {
    /// Enough bounded attestations are present for the configured readiness count.
    Ready,
    /// The attestation set is well-formed but does not yet meet readiness count.
    NotReady,
    /// The artifacts expose a deterministic disagreement condition.
    Disagreement,
}

/// Deterministic disagreement/readiness reason tags.
///
/// These tags are diagnostic only. They do not slash, settle, bridge, stake,
/// prune, or mutate ROC balances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainCommitteeDisagreementCodeV1 {
    /// Fewer attestations than required are present.
    InsufficientAttestations,
    /// The same committee member appears more than once.
    DuplicateCommitteeMember,
    /// One member produced more than one attestation in the same set.
    DuplicateMemberAttestation,
    /// An attestation was supplied by a member outside the declared committee.
    AttestationOutsideCommittee,
    /// An attestation targets a different replay-result hash.
    ReplayResultHashMismatch,
    /// An attestation targets a different replay status.
    ReplayStatusMismatch,
    /// An attestation/member targets a different chain or epoch.
    ChainEpochMismatch,
}

/// One Phase 2 Round 2 committee member identity.
///
/// This is a replay-readiness identity only. It is not a passport-gated
/// validator record, not a bonded validator, not staking, and not economic
/// authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainCommitteeMemberV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Stable committee-member identifier for this readiness artifact.
    pub member_id: String,
    /// Public key reference used by the attestation shape.
    pub key_id: String,
}

impl QuickChainCommitteeMemberV1 {
    /// Validate committee-member DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainCommitteeMemberV1.schema",
            &self.schema,
            QUICKCHAIN_COMMITTEE_MEMBER_SCHEMA,
        )?;
        validate_version("QuickChainCommitteeMemberV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("member_id", &self.member_id)?;
        validate_ref("key_id", &self.key_id)
    }
}

/// One shape-checked Phase 2 Round 2 verification attestation.
///
/// `ron-proto` validates the wire shape only. It does not verify cryptographic
/// signatures and does not grant finality, settlement, bridge, staking, slashing,
/// spend, paid-unlock, or pruning authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainVerifierAttestationV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Committee member claiming this attestation.
    pub committee_member_id: String,
    /// Public key reference for this attestation.
    pub key_id: String,
    /// Signature algorithm tag. This is data only; no crypto is executed here.
    pub signature_algorithm: SignatureAlg,
    /// Schema tag for the signed payload shape.
    pub signed_payload_schema: String,
    /// Hash of the replay result this member claims to have checked.
    pub replay_result_hash: ContentId,
    /// Replay status this member claims to have observed.
    pub replay_status: QuickChainVerifierReplayStatusV1,
    /// Bounded signature wire material.
    pub signature_wire: String,
}

impl QuickChainVerifierAttestationV1 {
    /// Validate attestation DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainVerifierAttestationV1.schema",
            &self.schema,
            QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA,
        )?;
        validate_version("QuickChainVerifierAttestationV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("committee_member_id", &self.committee_member_id)?;
        validate_ref("key_id", &self.key_id)?;
        validate_schema(
            "signed_payload_schema",
            &self.signed_payload_schema,
            QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA,
        )?;
        validate_bounded_nonempty(
            "signature_wire",
            &self.signature_wire,
            MAX_QUICKCHAIN_SIGNATURE_BYTES,
        )
    }
}

/// Bounded Phase 2 Round 2 committee attestation set.
///
/// This object can prove only that a bounded set of committee members produced
/// shape-valid attestations over the same replay result hash/status. It does not
/// create finality, fork choice, bridge authority, pruning permission, staking,
/// slashing, wallet mutation, ledger mutation, or paid-unlock authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainCommitteeAttestationSetV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Committee readiness algorithm token.
    pub committee_algorithm: String,
    /// Replay result hash all attestations must target.
    pub replay_result_hash: ContentId,
    /// Replay status all attestations must target.
    pub replay_status: QuickChainVerifierReplayStatusV1,
    /// Count-based readiness threshold.
    pub required_attestations: u16,
    /// Declared committee members.
    pub committee_members: Vec<QuickChainCommitteeMemberV1>,
    /// Submitted attestations.
    pub attestations: Vec<QuickChainVerifierAttestationV1>,
}

impl QuickChainCommitteeAttestationSetV1 {
    /// Validate committee attestation set shape and deterministic consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainCommitteeAttestationSetV1.schema",
            &self.schema,
            QUICKCHAIN_COMMITTEE_ATTESTATION_SET_SCHEMA,
        )?;
        validate_version("QuickChainCommitteeAttestationSetV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_committee_algorithm(&self.committee_algorithm)?;

        if self.required_attestations == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "required_attestations",
                reason: "committee readiness requires at least one attestation",
            });
        }

        if self.committee_members.is_empty() {
            return Err(QuickChainValidationError::InvalidField {
                field: "committee_members",
                reason: "committee readiness requires at least one member",
            });
        }

        if self.committee_members.len() > MAX_QUICKCHAIN_COMMITTEE_MEMBERS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "committee_members",
                max: MAX_QUICKCHAIN_COMMITTEE_MEMBERS,
                actual: self.committee_members.len(),
            });
        }

        if self.attestations.len() > MAX_QUICKCHAIN_COMMITTEE_ATTESTATIONS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "attestations",
                max: MAX_QUICKCHAIN_COMMITTEE_ATTESTATIONS,
                actual: self.attestations.len(),
            });
        }

        if usize::from(self.required_attestations) > self.committee_members.len() {
            return Err(QuickChainValidationError::InvalidField {
                field: "required_attestations",
                reason: "required attestations cannot exceed committee size",
            });
        }

        let mut member_ids = Vec::<&str>::with_capacity(self.committee_members.len());

        for member in &self.committee_members {
            member.validate()?;
            ensure_chain_epoch(
                "committee_members",
                &member.chain_id,
                &member.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            if member_ids.contains(&member.member_id.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "committee_members.member_id",
                    reason: "committee member ids must be unique",
                });
            }

            member_ids.push(member.member_id.as_str());
        }

        let mut attesting_members = Vec::<&str>::with_capacity(self.attestations.len());

        for attestation in &self.attestations {
            attestation.validate()?;
            ensure_chain_epoch(
                "attestations",
                &attestation.chain_id,
                &attestation.epoch_id,
                &self.chain_id,
                &self.epoch_id,
            )?;

            if !member_ids.contains(&attestation.committee_member_id.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "attestations.committee_member_id",
                    reason: "attestation must come from a declared committee member",
                });
            }

            if attesting_members.contains(&attestation.committee_member_id.as_str()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "attestations.committee_member_id",
                    reason: "one committee member cannot attest more than once in one set",
                });
            }

            if attestation.replay_result_hash != self.replay_result_hash {
                return Err(QuickChainValidationError::InvalidField {
                    field: "attestations.replay_result_hash",
                    reason: "attestation replay result hash must match the committee set target",
                });
            }

            if attestation.replay_status != self.replay_status {
                return Err(QuickChainValidationError::InvalidField {
                    field: "attestations.replay_status",
                    reason: "attestation replay status must match the committee set target",
                });
            }

            attesting_members.push(attestation.committee_member_id.as_str());
        }

        Ok(())
    }
}

/// Deterministic Phase 2 Round 2 committee readiness result.
///
/// This result is diagnostic and bounded. It is not finality, not fork choice,
/// not settlement, not a bridge, not staking, not slashing, and not pruning
/// permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainCommitteeReadinessResultV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Chain identity.
    pub chain_id: String,
    /// Epoch identity.
    pub epoch_id: String,
    /// Committee readiness algorithm token.
    pub committee_algorithm: String,
    /// Replay result hash evaluated.
    pub replay_result_hash: ContentId,
    /// Replay status evaluated.
    pub replay_status: QuickChainVerifierReplayStatusV1,
    /// Count required for readiness.
    pub required_attestations: u16,
    /// Declared committee size.
    pub committee_members_count: u16,
    /// Accepted attestation count.
    pub accepted_attestations_count: u16,
    /// Readiness status.
    pub status: QuickChainCommitteeReadinessStatusV1,
    /// Deterministic reason when not ready or disagreement.
    #[serde(default)]
    pub disagreement_code: Option<QuickChainCommitteeDisagreementCodeV1>,
}

impl QuickChainCommitteeReadinessResultV1 {
    /// Validate readiness result shape and status/count consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainCommitteeReadinessResultV1.schema",
            &self.schema,
            QUICKCHAIN_COMMITTEE_READINESS_RESULT_SCHEMA,
        )?;
        validate_version("QuickChainCommitteeReadinessResultV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_committee_algorithm(&self.committee_algorithm)?;

        if self.required_attestations == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "required_attestations",
                reason: "committee readiness requires at least one attestation",
            });
        }

        if self.committee_members_count == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "committee_members_count",
                reason: "committee readiness requires at least one member",
            });
        }

        if self.required_attestations > self.committee_members_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "required_attestations",
                reason: "required attestations cannot exceed committee size",
            });
        }

        if self.accepted_attestations_count > self.committee_members_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "accepted_attestations_count",
                reason: "accepted attestations cannot exceed committee size",
            });
        }

        match self.status {
            QuickChainCommitteeReadinessStatusV1::Ready => {
                if self.accepted_attestations_count < self.required_attestations {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason: "ready status requires enough accepted attestations",
                    });
                }

                if self.disagreement_code.is_some() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "disagreement_code",
                        reason: "ready status cannot carry a disagreement code",
                    });
                }
            }
            QuickChainCommitteeReadinessStatusV1::NotReady => {
                if self.accepted_attestations_count >= self.required_attestations {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason: "not_ready status requires fewer than required attestations",
                    });
                }

                if self.disagreement_code
                    != Some(QuickChainCommitteeDisagreementCodeV1::InsufficientAttestations)
                {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "disagreement_code",
                        reason: "not_ready status requires insufficient_attestations",
                    });
                }
            }
            QuickChainCommitteeReadinessStatusV1::Disagreement => {
                if self.disagreement_code.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "disagreement_code",
                        reason: "disagreement status requires a deterministic code",
                    });
                }
            }
        }

        Ok(())
    }
}

fn validate_committee_algorithm(value: &str) -> QuickChainResult<()> {
    validate_ascii_token(
        "committee_algorithm",
        value,
        MAX_QUICKCHAIN_COMMITTEE_ALGORITHM_BYTES,
    )?;

    if value != QUICKCHAIN_COMMITTEE_AGREEMENT_ALGORITHM_BOUNDED_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "committee_algorithm",
            reason: "unsupported QuickChain committee agreement algorithm",
        });
    }

    Ok(())
}

fn material_batch_for_tree(
    material_batches: &[QuickChainTreeMaterialBatchV1],
    tree: QuickChainTreeMaterialKindV1,
) -> Option<&QuickChainTreeMaterialBatchV1> {
    material_batches.iter().find(|batch| batch.tree == tree)
}

fn expected_root_for_tree(
    expected_roots: &[QuickChainTreeRootV1],
    tree: QuickChainTreeMaterialKindV1,
) -> Option<&QuickChainTreeRootV1> {
    expected_roots.iter().find(|root| root.tree == tree)
}

fn tree_key(tree: QuickChainTreeMaterialKindV1) -> &'static str {
    match tree {
        QuickChainTreeMaterialKindV1::State => "state",
        QuickChainTreeMaterialKindV1::Holds => "holds",
        QuickChainTreeMaterialKindV1::Receipts => "receipts",
        QuickChainTreeMaterialKindV1::Accounting => "accounting",
        QuickChainTreeMaterialKindV1::Rewards => "rewards",
    }
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
            reason: "chain_id must match replay bundle chain_id",
        });
    }

    if epoch_id != expected_epoch_id {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "epoch_id must match replay bundle epoch_id",
        });
    }

    Ok(())
}

fn validate_replay_algorithm(value: &str) -> QuickChainResult<()> {
    validate_ascii_token(
        "replay_algorithm",
        value,
        MAX_QUICKCHAIN_VERIFIER_ALGORITHM_BYTES,
    )?;

    if value != QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "replay_algorithm",
            reason: "unsupported QuickChain verifier replay algorithm",
        });
    }

    Ok(())
}

fn validate_lower_hex_sort_key(value: &str) -> QuickChainResult<()> {
    if value.is_empty() {
        return Err(QuickChainValidationError::EmptyField {
            field: "leaf_sort_key_hex",
        });
    }

    if value.len() % 2 != 0 {
        return Err(QuickChainValidationError::InvalidField {
            field: "leaf_sort_key_hex",
            reason: "hex sort key must have an even number of characters",
        });
    }

    if !value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(QuickChainValidationError::InvalidField {
            field: "leaf_sort_key_hex",
            reason: "hex sort key must be lowercase hexadecimal",
        });
    }

    Ok(())
}

fn validate_optional_detail(field: &'static str, value: &Option<String>) -> QuickChainResult<()> {
    let Some(detail) = value else {
        return Ok(());
    };

    if detail.trim().is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    let actual = detail.len();
    if actual > MAX_QUICKCHAIN_VERIFIER_DETAIL_BYTES {
        return Err(QuickChainValidationError::FieldTooLong {
            field,
            max: MAX_QUICKCHAIN_VERIFIER_DETAIL_BYTES,
            actual,
        });
    }

    if !detail
        .bytes()
        .all(|byte| byte == b' ' || byte.is_ascii_graphic())
    {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "detail must contain printable ASCII only",
        });
    }

    Ok(())
}

fn validate_ascii_token(field: &'static str, value: &str, max: usize) -> QuickChainResult<()> {
    if value.trim().is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    let actual = value.len();
    if actual > max {
        return Err(QuickChainValidationError::FieldTooLong { field, max, actual });
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
    }) {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason:
                "must contain only lowercase ASCII letters, digits, underscores, hyphens, or dots",
        });
    }

    Ok(())
}
