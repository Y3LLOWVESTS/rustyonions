//! RO:WHAT — Deterministic Phase 17 user-node review of ROC epoch transitions.
//! RO:WHY — Regular user nodes must independently detect invalid economic
//! transitions without gaining wallet, ledger, payout, or finality authority.
//! RO:INTERACTS — ron-proto epoch transition and expectation DTOs; later
//! challenge construction and cryptographic signature verification.
//! RO:INVARIANTS — read-only review, exact reviewed roots, exact allocation
//! replay, no duplicate payout, cap enforcement, and supply conservation.
//! RO:SECURITY — findings are evidence only; this module never mutates value.
//! RO:TEST — tests/internal_roc_beta_phase17_epoch_replay.rs.

use std::collections::BTreeSet;

use ron_kms::{Alg, KeyId, Verifier};
use ron_proto::{
    service_node_signature_message_bytes, ContentId, InvalidEpochChallengeKindV1,
    InvalidEpochChallengeV1, RocEpochTransitionExpectationV1, RocEpochTransitionV1, SignatureAlg,
    INVALID_EPOCH_CHALLENGE_SCHEMA, ROC_EPOCH_TRANSITION_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema for one independently produced user-node replay observation.
pub const USER_NODE_EPOCH_REPLAY_OBSERVATION_SCHEMA: &str =
    "micronode.user-node-epoch-replay-observation.v1";

/// Version for the Phase 17 replay observation and review surface.
pub const USER_NODE_EPOCH_REPLAY_VERSION: u16 = 1;

/// Resolves a logical service-node quorum key reference into a concrete KMS key.
///
/// The resolver is read-only and owns no signing, wallet, ledger, payout, or
/// challenge-acceptance authority.
pub trait EpochQuorumKeyResolver {
    /// Return the concrete key for one logical transition key reference.
    fn resolve_key(&self, key_ref: &str) -> Option<KeyId>;
}

/// Deterministic failures from independent user-node signature verification.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EpochSignatureVerificationError {
    /// The transition requested a signature algorithm not accepted by Phase 17.
    #[error("unsupported service-node signature algorithm")]
    UnsupportedAlgorithm,

    /// The reviewed registry/key view could not resolve a logical key.
    #[error("unknown service-node quorum key reference: {key_ref}")]
    UnknownKey {
        /// Unresolved logical key reference.
        key_ref: String,
    },

    /// The resolved concrete key is not an Ed25519 key.
    #[error("resolved service-node quorum key is not Ed25519")]
    KeyAlgorithmMismatch,

    /// Signature bytes were not canonical 64-byte lowercase hexadecimal.
    #[error("service-node quorum signature is not canonical lowercase hex")]
    InvalidSignatureEncoding,

    /// Canonical signature-message construction failed.
    #[error("service-node quorum signature message could not be encoded")]
    MessageEncoding,

    /// The KMS verifier dependency could not complete verification.
    #[error("service-node quorum signature verifier dependency failed")]
    VerificationDependency,

    /// Cryptographic signature verification returned false.
    #[error("service-node quorum signature verification failed for {service_node_id}")]
    InvalidSignature {
        /// Service node whose signature did not verify.
        service_node_id: String,
    },
}

/// Errors produced while converting a rejected local review into canonical
/// invalid-epoch challenge evidence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EpochChallengeBuildError {
    /// Accepted transitions cannot produce fraud challenges.
    #[error("accepted epoch review cannot produce an invalid-epoch challenge")]
    AcceptedTransition,

    /// The review belongs to a different transition.
    #[error("epoch review transition hash does not match the challenged transition")]
    ReviewTransitionMismatch,

    /// The rejected review did not contain a challenge-mappable finding.
    #[error("epoch review contains no canonical invalid-epoch challenge category")]
    MissingChallengeKind,

    /// Deterministic evidence serialization failed.
    #[error("failed to serialize invalid-epoch challenge evidence: {0}")]
    EvidenceSerialization(String),

    /// BLAKE3 evidence could not be represented as a canonical content ID.
    #[error("failed to construct invalid-epoch challenge evidence hash: {0}")]
    InvalidEvidenceHash(String),

    /// The constructed ron-proto challenge failed canonical validation.
    #[error("constructed invalid-epoch challenge is invalid: {0}")]
    InvalidChallenge(String),
}

/// Local replay facts independently observed by a user node.
///
/// These values are review inputs only. They do not authorize wallet or ledger
/// mutation and do not themselves create an invalid-epoch challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpochReplayObservationV1 {
    /// Exact observation schema.
    pub schema: String,
    /// Exact observation version.
    pub version: u16,
    /// Independently reproduced accounting snapshot root.
    pub accounting_snapshot_hash: ContentId,
    /// Independently reproduced reward plan root.
    pub reward_plan_hash: ContentId,
    /// Independently reviewed policy hash.
    pub policy_hash: ContentId,
    /// Independently reviewed economics configuration hash.
    pub economics_config_hash: ContentId,
    /// Independently reviewed service-node registry root.
    pub registry_root: ContentId,
    /// Independently reviewed reward-recipient binding root.
    pub reward_binding_root: ContentId,
    /// Allocation identities observed in accepted ledger replay.
    pub applied_allocation_ids: Vec<String>,
    /// Reward total reconstructed from accepted payout evidence.
    pub replayed_reward_total_minor_units: String,
    /// Total ROC supply before applying the reviewed epoch.
    pub supply_before_minor_units: String,
    /// Total ROC supply after applying the reviewed epoch.
    pub supply_after_minor_units: String,
}

/// Stable category for one user-node epoch-audit finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpochAuditFindingKindV1 {
    /// Observation schema, version, identifier, or numeric evidence is invalid.
    InvalidObservation,
    /// The transition does not satisfy the canonical ron-proto expectation.
    InvalidTransition,
    /// At least one quorum signature failed independent cryptographic review.
    InvalidServiceNodeSignature,
    /// Accounting snapshot root differs from independently reviewed truth.
    AccountingSnapshotHashMismatch,
    /// Reward plan root differs from independently reviewed truth.
    RewardPlanHashMismatch,
    /// Policy hash differs from independently reviewed truth.
    PolicyHashMismatch,
    /// Economics configuration hash differs from reviewed configuration.
    EconomicsConfigHashMismatch,
    /// Registry root differs from independently reviewed registry state.
    RegistryRootMismatch,
    /// Reward-recipient binding root differs from reviewed binding state.
    RewardBindingRootMismatch,
    /// The replay contains a duplicate payout allocation.
    DuplicatePayout,
    /// The replayed allocation set differs from the transition allocation set.
    ReplayAllocationSetMismatch,
    /// Replayed rewards exceed the reviewed epoch cap.
    RewardCapExceeded,
    /// Replayed reward total differs from the transition-declared total.
    ReplayTotalMismatch,
    /// Supply after replay is not supply-before plus accepted epoch rewards.
    SupplyConservationMismatch,
}

/// One deterministic finding produced by local user-node review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpochAuditFindingV1 {
    /// Stable machine-readable finding category.
    pub kind: EpochAuditFindingKindV1,
    /// Small deterministic explanation without secrets or peer addresses.
    pub detail: String,
}

/// Final local review status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserNodeEpochReviewStatusV1 {
    /// All currently implemented deterministic review gates passed.
    Accepted,
    /// One or more findings require canonical challenge construction.
    ChallengeRequired,
}

/// Read-only result of independently reviewing one epoch transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserNodeEpochReviewV1 {
    status: UserNodeEpochReviewStatusV1,
    transition_hash: ContentId,
    replayed_reward_total_minor_units: String,
    findings: Vec<EpochAuditFindingV1>,
    wallet_mutation: bool,
    ledger_mutation: bool,
    challenge_submitted: bool,
}

impl UserNodeEpochReviewV1 {
    /// Return the review status.
    pub const fn status(&self) -> UserNodeEpochReviewStatusV1 {
        self.status
    }

    /// True only when every implemented review gate passed.
    pub const fn is_accepted(&self) -> bool {
        matches!(self.status, UserNodeEpochReviewStatusV1::Accepted)
    }

    /// Borrow all deterministic findings.
    pub fn findings(&self) -> &[EpochAuditFindingV1] {
        &self.findings
    }

    /// Return the reviewed transition hash.
    pub const fn transition_hash(&self) -> &ContentId {
        &self.transition_hash
    }

    /// Return the replayed reward total exactly as reviewed.
    pub fn replayed_reward_total_minor_units(&self) -> &str {
        &self.replayed_reward_total_minor_units
    }

    /// Phase 17 review never mutates a wallet.
    pub const fn wallet_mutation(&self) -> bool {
        self.wallet_mutation
    }

    /// Phase 17 review never mutates a ledger.
    pub const fn ledger_mutation(&self) -> bool {
        self.ledger_mutation
    }

    /// This first slice reports challenge necessity but does not submit one.
    pub const fn challenge_submitted(&self) -> bool {
        self.challenge_submitted
    }
}

/// Independently review one service-node epoch transition.
///
/// This function deliberately returns findings rather than mutating economic
/// state. Cryptographic signature-byte verification and construction of the
/// canonical `InvalidEpochChallengeV1` are subsequent Phase 17 slices.
pub fn review_epoch_transition(
    transition: &RocEpochTransitionV1,
    expected: &RocEpochTransitionExpectationV1,
    observation: &EpochReplayObservationV1,
) -> UserNodeEpochReviewV1 {
    let mut findings = Vec::new();

    validate_observation(observation, &mut findings);

    if let Err(error) = transition.validate_against(expected) {
        push_finding(
            &mut findings,
            EpochAuditFindingKindV1::InvalidTransition,
            format!("transition failed canonical reviewed-input validation: {error}"),
        );
    }

    compare_reviewed_hash(
        &mut findings,
        EpochAuditFindingKindV1::AccountingSnapshotHashMismatch,
        &observation.accounting_snapshot_hash,
        &expected.accounting_snapshot_hash,
        "accounting snapshot hash does not match independently reviewed truth",
    );
    compare_reviewed_hash(
        &mut findings,
        EpochAuditFindingKindV1::RewardPlanHashMismatch,
        &observation.reward_plan_hash,
        &expected.reward_plan_hash,
        "reward plan hash does not match independently reviewed truth",
    );
    compare_reviewed_hash(
        &mut findings,
        EpochAuditFindingKindV1::PolicyHashMismatch,
        &observation.policy_hash,
        &expected.policy_hash,
        "policy hash does not match independently reviewed truth",
    );
    compare_reviewed_hash(
        &mut findings,
        EpochAuditFindingKindV1::EconomicsConfigHashMismatch,
        &observation.economics_config_hash,
        &expected.economics_config_hash,
        "economics configuration hash does not match reviewed configuration",
    );
    compare_reviewed_hash(
        &mut findings,
        EpochAuditFindingKindV1::RegistryRootMismatch,
        &observation.registry_root,
        &expected.registry_root,
        "registry root does not match independently reviewed registry state",
    );
    compare_reviewed_hash(
        &mut findings,
        EpochAuditFindingKindV1::RewardBindingRootMismatch,
        &observation.reward_binding_root,
        &expected.reward_binding_root,
        "reward binding root does not match independently reviewed binding state",
    );

    review_allocation_replay(transition, observation, &mut findings);
    review_amounts_and_supply(transition, observation, &mut findings);

    let status = if findings.is_empty() {
        UserNodeEpochReviewStatusV1::Accepted
    } else {
        UserNodeEpochReviewStatusV1::ChallengeRequired
    };

    UserNodeEpochReviewV1 {
        status,
        transition_hash: transition.transition_hash.clone(),
        replayed_reward_total_minor_units: observation.replayed_reward_total_minor_units.clone(),
        findings,
        wallet_mutation: false,
        ledger_mutation: false,
        challenge_submitted: false,
    }
}

/// Independently review an epoch transition including real quorum signatures.
///
/// This is the Phase 17 acceptance path. It performs the structural/root/replay
/// review and then verifies every quorum signature against the supplied
/// read-only key view.
///
/// No signing, wallet mutation, ledger mutation, finality, or challenge
/// submission occurs here.
pub fn review_epoch_transition_with_signatures<V, R>(
    transition: &RocEpochTransitionV1,
    expected: &RocEpochTransitionExpectationV1,
    observation: &EpochReplayObservationV1,
    verifier: &V,
    key_resolver: &R,
) -> UserNodeEpochReviewV1
where
    V: Verifier,
    R: EpochQuorumKeyResolver,
{
    let mut review = review_epoch_transition(transition, expected, observation);

    if let Err(error) = verify_epoch_quorum_signatures(transition, verifier, key_resolver) {
        push_finding(
            &mut review.findings,
            EpochAuditFindingKindV1::InvalidServiceNodeSignature,
            error.to_string(),
        );
        review.status = UserNodeEpochReviewStatusV1::ChallengeRequired;
    }

    review
}

/// Verify every quorum signature through the existing `ron-kms` verifier.
///
/// # Errors
///
/// Returns the first deterministic key, encoding, algorithm, dependency, or
/// signature failure. This function is read-only.
pub fn verify_epoch_quorum_signatures<V, R>(
    transition: &RocEpochTransitionV1,
    verifier: &V,
    key_resolver: &R,
) -> Result<(), EpochSignatureVerificationError>
where
    V: Verifier,
    R: EpochQuorumKeyResolver,
{
    for signature in &transition.quorum.signatures {
        if signature.algorithm != SignatureAlg::Ed25519 {
            return Err(EpochSignatureVerificationError::UnsupportedAlgorithm);
        }

        let key_id = key_resolver.resolve_key(&signature.key_id).ok_or_else(|| {
            EpochSignatureVerificationError::UnknownKey { key_ref: signature.key_id.clone() }
        })?;

        if key_id.alg != Alg::Ed25519 {
            return Err(EpochSignatureVerificationError::KeyAlgorithmMismatch);
        }

        if signature.signature_wire.len() != 128
            || !signature
                .signature_wire
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(EpochSignatureVerificationError::InvalidSignatureEncoding);
        }

        let signature_bytes = hex::decode(&signature.signature_wire)
            .map_err(|_| EpochSignatureVerificationError::InvalidSignatureEncoding)?;

        let message = service_node_signature_message_bytes(signature)
            .map_err(|_| EpochSignatureVerificationError::MessageEncoding)?;

        let verified = verifier
            .verify(&key_id, &message, &signature_bytes)
            .map_err(|_| EpochSignatureVerificationError::VerificationDependency)?;

        if !verified {
            return Err(EpochSignatureVerificationError::InvalidSignature {
                service_node_id: signature.service_node_id.clone(),
            });
        }
    }

    Ok(())
}

fn validate_observation(
    observation: &EpochReplayObservationV1,
    findings: &mut Vec<EpochAuditFindingV1>,
) {
    if observation.schema != USER_NODE_EPOCH_REPLAY_OBSERVATION_SCHEMA {
        push_finding(
            findings,
            EpochAuditFindingKindV1::InvalidObservation,
            "invalid user-node epoch replay observation schema",
        );
    }

    if observation.version != USER_NODE_EPOCH_REPLAY_VERSION {
        push_finding(
            findings,
            EpochAuditFindingKindV1::InvalidObservation,
            "invalid user-node epoch replay observation version",
        );
    }

    for allocation_id in &observation.applied_allocation_ids {
        if !valid_replay_identifier(allocation_id) {
            push_finding(
                findings,
                EpochAuditFindingKindV1::InvalidObservation,
                "replayed allocation id is empty, oversized, or contains unsupported characters",
            );
            break;
        }
    }
}

fn review_allocation_replay(
    transition: &RocEpochTransitionV1,
    observation: &EpochReplayObservationV1,
    findings: &mut Vec<EpochAuditFindingV1>,
) {
    let expected_ids: BTreeSet<&str> =
        transition.allocations.iter().map(|allocation| allocation.allocation_id.as_str()).collect();

    let observed_ids: BTreeSet<&str> =
        observation.applied_allocation_ids.iter().map(String::as_str).collect();

    if observed_ids.len() != observation.applied_allocation_ids.len() {
        push_finding(
            findings,
            EpochAuditFindingKindV1::DuplicatePayout,
            "ledger replay contains a duplicate epoch allocation",
        );
    }

    if expected_ids != observed_ids
        || transition.allocations.len() != observation.applied_allocation_ids.len()
    {
        push_finding(
            findings,
            EpochAuditFindingKindV1::ReplayAllocationSetMismatch,
            "ledger replay allocation set does not exactly match the transition",
        );
    }
}

fn review_amounts_and_supply(
    transition: &RocEpochTransitionV1,
    observation: &EpochReplayObservationV1,
    findings: &mut Vec<EpochAuditFindingV1>,
) {
    let reward_cap =
        parse_minor_units("reward_cap_minor_units", &transition.reward_cap_minor_units, findings);
    let declared_total = parse_minor_units(
        "reward_total_minor_units",
        &transition.reward_total_minor_units,
        findings,
    );
    let replayed_total = parse_minor_units(
        "replayed_reward_total_minor_units",
        &observation.replayed_reward_total_minor_units,
        findings,
    );
    let supply_before = parse_minor_units(
        "supply_before_minor_units",
        &observation.supply_before_minor_units,
        findings,
    );
    let supply_after = parse_minor_units(
        "supply_after_minor_units",
        &observation.supply_after_minor_units,
        findings,
    );

    if let (Some(replayed_total), Some(reward_cap)) = (replayed_total, reward_cap) {
        if replayed_total > reward_cap {
            push_finding(
                findings,
                EpochAuditFindingKindV1::RewardCapExceeded,
                "replayed reward total exceeds the reviewed epoch cap",
            );
        }
    }

    if let (Some(replayed_total), Some(declared_total)) = (replayed_total, declared_total) {
        if replayed_total != declared_total {
            push_finding(
                findings,
                EpochAuditFindingKindV1::ReplayTotalMismatch,
                "replayed reward total differs from the transition-declared total",
            );
        }
    }

    if let (Some(supply_before), Some(replayed_total), Some(supply_after)) =
        (supply_before, replayed_total, supply_after)
    {
        match supply_before.checked_add(replayed_total) {
            Some(expected_supply_after) if expected_supply_after == supply_after => {}
            _ => push_finding(
                findings,
                EpochAuditFindingKindV1::SupplyConservationMismatch,
                "supply after replay is not supply before plus accepted epoch rewards",
            ),
        }
    }
}

fn parse_minor_units(
    field: &'static str,
    value: &str,
    findings: &mut Vec<EpochAuditFindingV1>,
) -> Option<u128> {
    let canonical = !value.is_empty()
        && value.chars().all(|character| character.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'));

    if !canonical {
        push_finding(
            findings,
            EpochAuditFindingKindV1::InvalidObservation,
            format!("{field} must be canonical unsigned integer minor units"),
        );
        return None;
    }

    match value.parse::<u128>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            push_finding(
                findings,
                EpochAuditFindingKindV1::InvalidObservation,
                format!("{field} exceeds the supported u128 minor-unit range"),
            );
            None
        }
    }
}

fn compare_reviewed_hash(
    findings: &mut Vec<EpochAuditFindingV1>,
    kind: EpochAuditFindingKindV1,
    observed: &ContentId,
    expected: &ContentId,
    detail: &'static str,
) {
    if observed != expected {
        push_finding(findings, kind, detail);
    }
}

fn push_finding(
    findings: &mut Vec<EpochAuditFindingV1>,
    kind: EpochAuditFindingKindV1,
    detail: impl Into<String>,
) {
    findings.push(EpochAuditFindingV1 { kind, detail: detail.into() });
}

fn valid_replay_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_' | '.' | '/')
        })
}

#[derive(Serialize)]
struct InvalidEpochChallengeEvidencePreimage<'a> {
    domain: &'static str,
    chain_id: &'a str,
    epoch_id: &'a str,
    transition_hash: &'a ContentId,
    challenger_id: &'a str,
    challenge_kind: InvalidEpochChallengeKindV1,
    replayed_reward_total_minor_units: &'a str,
    findings: &'a [EpochAuditFindingV1],
}

/// Build one canonical invalid-epoch challenge from a rejected local review.
///
/// The resulting DTO is an outbound evidence artifact only. Construction does
/// not submit, accept, quarantine, finalize, reward, slash, or mutate anything.
///
/// # Errors
///
/// Returns an error when the review was accepted, belongs to another
/// transition, has no canonical challenge category, cannot be hashed, or
/// produces a challenge rejected by `ron-proto`.
pub fn build_invalid_epoch_challenge(
    transition: &RocEpochTransitionV1,
    review: &UserNodeEpochReviewV1,
    challenger_id: &str,
    submitted_at_ms: u64,
) -> Result<InvalidEpochChallengeV1, EpochChallengeBuildError> {
    if review.is_accepted() {
        return Err(EpochChallengeBuildError::AcceptedTransition);
    }

    if review.transition_hash() != &transition.transition_hash {
        return Err(EpochChallengeBuildError::ReviewTransitionMismatch);
    }

    let challenge_kind = primary_challenge_kind(review.findings())
        .ok_or(EpochChallengeBuildError::MissingChallengeKind)?;

    let evidence_preimage = InvalidEpochChallengeEvidencePreimage {
        domain: "rustyonions.user-node-invalid-epoch-evidence.v1",
        chain_id: &transition.chain_id,
        epoch_id: &transition.epoch_id,
        transition_hash: &transition.transition_hash,
        challenger_id,
        challenge_kind,
        replayed_reward_total_minor_units: review.replayed_reward_total_minor_units(),
        findings: review.findings(),
    };

    let evidence_bytes = serde_json::to_vec(&evidence_preimage)
        .map_err(|error| EpochChallengeBuildError::EvidenceSerialization(error.to_string()))?;

    let evidence_digest = blake3::hash(&evidence_bytes).to_hex().to_string();

    let evidence_hash = format!("b3:{evidence_digest}")
        .parse::<ContentId>()
        .map_err(|error| EpochChallengeBuildError::InvalidEvidenceHash(error.to_string()))?;

    let challenge = InvalidEpochChallengeV1 {
        schema: INVALID_EPOCH_CHALLENGE_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        challenge_id: format!("challenge:{}:{}", transition.epoch_id, &evidence_digest[..16]),
        chain_id: transition.chain_id.clone(),
        epoch_id: transition.epoch_id.clone(),
        transition_hash: transition.transition_hash.clone(),
        challenger_id: challenger_id.to_owned(),
        challenge_kind,
        evidence_hash,
        submitted_at_ms,
    };

    challenge
        .validate()
        .map_err(|error| EpochChallengeBuildError::InvalidChallenge(error.to_string()))?;

    Ok(challenge)
}

/// Choose one canonical challenge category using stable severity precedence.
///
/// The complete finding set remains committed inside `evidence_hash`; the
/// single ron-proto category identifies the primary dispute route.
fn primary_challenge_kind(findings: &[EpochAuditFindingV1]) -> Option<InvalidEpochChallengeKindV1> {
    let precedence = [
        (
            EpochAuditFindingKindV1::AccountingSnapshotHashMismatch,
            InvalidEpochChallengeKindV1::InvalidAccountingSnapshot,
        ),
        (
            EpochAuditFindingKindV1::RewardPlanHashMismatch,
            InvalidEpochChallengeKindV1::InvalidRewardPlan,
        ),
        (
            EpochAuditFindingKindV1::PolicyHashMismatch,
            InvalidEpochChallengeKindV1::InvalidPolicyHash,
        ),
        (
            EpochAuditFindingKindV1::EconomicsConfigHashMismatch,
            InvalidEpochChallengeKindV1::InvalidEconomicsConfigHash,
        ),
        (
            EpochAuditFindingKindV1::RegistryRootMismatch,
            InvalidEpochChallengeKindV1::InvalidRegistryRoot,
        ),
        (
            EpochAuditFindingKindV1::RewardBindingRootMismatch,
            InvalidEpochChallengeKindV1::InvalidRewardBindingRoot,
        ),
        (EpochAuditFindingKindV1::RewardCapExceeded, InvalidEpochChallengeKindV1::CapOverflow),
        (
            EpochAuditFindingKindV1::DuplicatePayout,
            InvalidEpochChallengeKindV1::DuplicateAllocation,
        ),
        (
            EpochAuditFindingKindV1::InvalidServiceNodeSignature,
            InvalidEpochChallengeKindV1::InvalidServiceNodeSignature,
        ),
        (
            EpochAuditFindingKindV1::InvalidTransition,
            InvalidEpochChallengeKindV1::InvalidEligibilitySet,
        ),
    ];

    for (finding_kind, challenge_kind) in precedence {
        if findings.iter().any(|finding| finding.kind == finding_kind) {
            return Some(challenge_kind);
        }
    }

    if findings.iter().any(|finding| {
        matches!(
            finding.kind,
            EpochAuditFindingKindV1::InvalidObservation
                | EpochAuditFindingKindV1::ReplayAllocationSetMismatch
                | EpochAuditFindingKindV1::ReplayTotalMismatch
                | EpochAuditFindingKindV1::SupplyConservationMismatch
        )
    }) {
        return Some(InvalidEpochChallengeKindV1::InvalidEvidenceRoot);
    }

    None
}
