//! RO:WHAT — Service Node validation and signing of canonical QuickChain epoch-transition identity.
//! RO:WHY — FINAL_BETA Phase 19 requires real independent Service Nodes to participate in quorum
//!          without duplicating transition semantics or accepting caller-prepared transition hashes.
//! RO:INTERACTS — ron-proto canonical transition identity/signature message, ron-kms Ed25519 signing,
//!                and BLAKE3 transition identity hashing.
//! RO:INVARIANTS — identity validates first; chain/node/key eligibility must match; transition hash is
//!                 recomputed locally; one participant returns exactly one signature.
//! RO:SECURITY — no quorum aggregation, wallet mutation, ledger mutation, payout execution, receipt
//!               creation, finality authority, paid unlock, or CrabLink authority.
//! RO:TEST — focused unit tests below plus later live Phase 19 multi-node integration.

#![forbid(unsafe_code)]

use std::{error::Error as StdError, fmt, sync::Arc};

use ron_kms::{KeyId, Signer};
use ron_proto::{
    quantum::SignatureAlg, service_node_signature_message_bytes, ContentId,
    EpochEligibilityStatusV1, RocEpochTransitionIdentityV1, ServiceNodeSignatureV1,
    ROC_EPOCH_TRANSITION_HASH_DOMAIN, ROC_EPOCH_TRANSITION_VERSION,
};

/// Configuration binding one running Service Node to its quorum signing key.
///
/// This structure carries participation identity only. It owns no quorum,
/// checkpoint, wallet, ledger, payout, or finality state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceNodeQuorumParticipant {
    pub chain_id: String,
    pub service_node_id: String,
    pub logical_key_ref: String,
    pub concrete_key: KeyId,
}

impl ServiceNodeQuorumParticipant {
    #[must_use]
    pub fn new(
        chain_id: impl Into<String>,
        service_node_id: impl Into<String>,
        logical_key_ref: impl Into<String>,
        concrete_key: KeyId,
    ) -> Self {
        Self {
            chain_id: chain_id.into(),
            service_node_id: service_node_id.into(),
            logical_key_ref: logical_key_ref.into(),
            concrete_key,
        }
    }

    /// Validate a canonical transition identity and produce this Service Node's
    /// single Ed25519 quorum signature.
    ///
    /// The caller cannot provide a transition hash. The hash is derived here
    /// from the validated protocol-owned identity using the locked historical
    /// transition hash domain.
    pub fn validate_and_sign(
        &self,
        kms: &dyn Signer,
        identity: &RocEpochTransitionIdentityV1,
    ) -> Result<ServiceNodeSignatureV1, QuorumParticipationError> {
        identity
            .validate()
            .map_err(|error| QuorumParticipationError::InvalidIdentity(format!("{error:?}")))?;

        if identity.chain_id != self.chain_id {
            return Err(QuorumParticipationError::ChainMismatch {
                expected: self.chain_id.clone(),
                actual: identity.chain_id.clone(),
            });
        }

        let eligibility = identity
            .eligibilities
            .iter()
            .find(|candidate| candidate.service_node_id == self.service_node_id)
            .ok_or_else(|| QuorumParticipationError::ServiceNodeMissing {
                service_node_id: self.service_node_id.clone(),
            })?;

        if eligibility.status != EpochEligibilityStatusV1::Eligible {
            return Err(QuorumParticipationError::ServiceNodeIneligible {
                service_node_id: self.service_node_id.clone(),
            });
        }

        if eligibility.key_id != self.logical_key_ref {
            return Err(QuorumParticipationError::KeyReferenceMismatch {
                expected: eligibility.key_id.clone(),
                actual: self.logical_key_ref.clone(),
            });
        }

        let transition_hash = validated_transition_hash(identity)?;

        let mut signature = ServiceNodeSignatureV1 {
            version: ROC_EPOCH_TRANSITION_VERSION,
            chain_id: identity.chain_id.clone(),
            epoch_id: identity.epoch_id.clone(),
            service_node_id: self.service_node_id.clone(),
            key_id: self.logical_key_ref.clone(),
            algorithm: SignatureAlg::Ed25519,
            transition_hash,
            signature_wire: "00".repeat(64),
        };

        let message = service_node_signature_message_bytes(&signature)
            .map_err(|error| QuorumParticipationError::SignatureMessage(format!("{error:?}")))?;

        let raw_signature = kms
            .sign(&self.concrete_key, &message)
            .map_err(|error| QuorumParticipationError::Signing(format!("{error:?}")))?;

        signature.signature_wire = encode_lower_hex(&raw_signature);

        Ok(signature)
    }
}

/// Process-local binding between one reviewed Service Node participant
/// identity and an injected signing backend.
///
/// This wrapper deliberately owns no quorum aggregate, checkpoint,
/// wallet, ledger, payout, receipt, or finality state. The signing backend
/// is injected so macronode does not silently manufacture a dev-only key
/// and call it durable Service Node identity.
#[derive(Clone)]
pub struct QuorumParticipationRuntime {
    participant: ServiceNodeQuorumParticipant,
    signer: Arc<dyn Signer>,
}

impl QuorumParticipationRuntime {
    #[must_use]
    pub fn new(participant: ServiceNodeQuorumParticipant, signer: Arc<dyn Signer>) -> Self {
        Self {
            participant,
            signer,
        }
    }

    #[must_use]
    pub fn chain_id(&self) -> &str {
        &self.participant.chain_id
    }

    #[must_use]
    pub fn service_node_id(&self) -> &str {
        &self.participant.service_node_id
    }

    #[must_use]
    pub fn logical_key_ref(&self) -> &str {
        &self.participant.logical_key_ref
    }

    /// Produce this process participant's single signature after all
    /// canonical identity, chain, eligibility, and key-binding checks.
    pub fn sign_transition(
        &self,
        identity: &RocEpochTransitionIdentityV1,
    ) -> Result<ServiceNodeSignatureV1, QuorumParticipationError> {
        self.participant
            .validate_and_sign(self.signer.as_ref(), identity)
    }
}

impl fmt::Debug for QuorumParticipationRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuorumParticipationRuntime")
            .field("chain_id", &self.participant.chain_id)
            .field("service_node_id", &self.participant.service_node_id)
            .field("logical_key_ref", &self.participant.logical_key_ref)
            .field("signer_backend", &"<redacted>")
            .finish()
    }
}

/// Process-runtime registration and signing failures.
///
/// Registration is intentionally one-way in this Phase 19 slice.
/// Replacing a live participant/key requires a future explicit rotation
/// protocol rather than an accidental second registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuorumParticipationRuntimeError {
    AlreadyConfigured,
    NotConfigured,
    Participation(QuorumParticipationError),
}

impl fmt::Display for QuorumParticipationRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConfigured => {
                formatter.write_str("Service Node quorum participant is already configured")
            }

            Self::NotConfigured => {
                formatter.write_str("Service Node quorum participant is not configured")
            }

            Self::Participation(error) => write!(
                formatter,
                "Service Node quorum participation rejected: {error}"
            ),
        }
    }
}

impl StdError for QuorumParticipationRuntimeError {}

impl From<QuorumParticipationError> for QuorumParticipationRuntimeError {
    fn from(error: QuorumParticipationError) -> Self {
        Self::Participation(error)
    }
}

/// Reproduce the locked reward-loop transition identity hash.
///
/// Compatibility contract:
///
/// `BLAKE3(ROC_EPOCH_TRANSITION_HASH_DOMAIN || 0x00 || serde_json(identity))`
fn validated_transition_hash(
    identity: &RocEpochTransitionIdentityV1,
) -> Result<ContentId, QuorumParticipationError> {
    identity
        .validate()
        .map_err(|error| QuorumParticipationError::InvalidIdentity(format!("{error:?}")))?;

    let bytes = serde_json::to_vec(identity)
        .map_err(|error| QuorumParticipationError::Serialization(format!("{error:?}")))?;

    let mut hasher = blake3::Hasher::new();

    hasher.update(ROC_EPOCH_TRANSITION_HASH_DOMAIN.as_bytes());
    hasher.update(&[0]);
    hasher.update(&bytes);

    ContentId::parse(&format!("b3:{}", hasher.finalize().to_hex()))
        .map_err(|error| QuorumParticipationError::HashEncoding(format!("{error:?}")))
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(HEX[usize::from(byte >> 4)] as char);
        encoded.push(HEX[usize::from(byte & 0x0f)] as char);
    }

    encoded
}

/// Fail-closed reasons why a Service Node refuses quorum participation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuorumParticipationError {
    InvalidIdentity(String),

    ChainMismatch { expected: String, actual: String },

    ServiceNodeMissing { service_node_id: String },

    ServiceNodeIneligible { service_node_id: String },

    KeyReferenceMismatch { expected: String, actual: String },

    Serialization(String),

    HashEncoding(String),

    SignatureMessage(String),

    Signing(String),
}

impl fmt::Display for QuorumParticipationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity(error) => {
                write!(formatter, "invalid transition identity: {error}")
            }

            Self::ChainMismatch { expected, actual } => write!(
                formatter,
                "transition chain mismatch: expected {expected}, received {actual}"
            ),

            Self::ServiceNodeMissing { service_node_id } => write!(
                formatter,
                "service node {service_node_id} is absent from reviewed epoch eligibility"
            ),

            Self::ServiceNodeIneligible { service_node_id } => write!(
                formatter,
                "service node {service_node_id} is not eligible for this epoch"
            ),

            Self::KeyReferenceMismatch { expected, actual } => write!(
                formatter,
                "quorum key reference mismatch: expected {expected}, received {actual}"
            ),

            Self::Serialization(error) => {
                write!(
                    formatter,
                    "transition identity serialization failed: {error}"
                )
            }

            Self::HashEncoding(error) => {
                write!(formatter, "transition hash encoding failed: {error}")
            }

            Self::SignatureMessage(error) => {
                write!(formatter, "signature message encoding failed: {error}")
            }

            Self::Signing(error) => {
                write!(formatter, "service node signing failed: {error}")
            }
        }
    }
}

impl StdError for QuorumParticipationError {}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_kms::{memory_keystore, Keystore, Verifier};
    use ron_proto::{
        EpochEligibilityV1, EpochQuorumThresholdV1, EpochRewardAllocationV1,
        RocEpochTransitionExpectationV1, EPOCH_REWARD_ALLOCATION_SCHEMA,
        ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA,
    };

    const CHAIN_ID: &str = "rustyonions-dev";
    const EPOCH_ID: &str = "epoch:19";
    const ALPHA_NODE: &str = "service_node:alpha";
    const ALPHA_KEY_REF: &str = "key:phase19:alpha";

    fn cid(character: char) -> ContentId {
        format!("b3:{}", character.to_string().repeat(64))
            .parse()
            .expect("fixture content id must parse")
    }

    fn eligibility(
        service_node_id: &str,
        key_id: &str,
        status: EpochEligibilityStatusV1,
    ) -> EpochEligibilityV1 {
        EpochEligibilityV1 {
            version: ROC_EPOCH_TRANSITION_VERSION,
            service_node_id: service_node_id.to_owned(),
            registry_entry_id: format!("registry:{service_node_id}"),
            reward_binding_id: format!("binding:{service_node_id}"),
            key_id: key_id.to_owned(),
            status,
        }
    }

    fn threshold() -> EpochQuorumThresholdV1 {
        EpochQuorumThresholdV1 {
            version: ROC_EPOCH_TRANSITION_VERSION,
            eligible_service_nodes: 3,
            quorum_bps: 6_666,
            minimum_signatures: 2,
            required_signatures: 2,
        }
    }

    fn expectation() -> RocEpochTransitionExpectationV1 {
        RocEpochTransitionExpectationV1 {
            schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),
            version: ROC_EPOCH_TRANSITION_VERSION,
            chain_id: CHAIN_ID.to_owned(),
            epoch_id: EPOCH_ID.to_owned(),
            accounting_snapshot_hash: cid('a'),
            reward_plan_hash: cid('b'),
            policy_hash: cid('c'),
            economics_config_hash: cid('d'),
            registry_root: cid('e'),
            reward_binding_root: cid('f'),
            evidence_root: cid('1'),
            reward_cap_minor_units: "1000".to_owned(),
            threshold: threshold(),
            eligibilities: vec![
                eligibility(
                    ALPHA_NODE,
                    ALPHA_KEY_REF,
                    EpochEligibilityStatusV1::Eligible,
                ),
                eligibility(
                    "service_node:beta",
                    "key:phase19:beta",
                    EpochEligibilityStatusV1::Eligible,
                ),
                eligibility(
                    "service_node:gamma",
                    "key:phase19:gamma",
                    EpochEligibilityStatusV1::Eligible,
                ),
            ],
        }
    }

    fn allocation(
        allocation_id: &str,
        reward_plan_allocation_id: &str,
        service_node_id: &str,
        amount_minor_units: &str,
    ) -> EpochRewardAllocationV1 {
        EpochRewardAllocationV1 {
            schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),
            version: ROC_EPOCH_TRANSITION_VERSION,
            allocation_id: allocation_id.to_owned(),
            reward_plan_allocation_id: reward_plan_allocation_id.to_owned(),
            service_node_id: service_node_id.to_owned(),
            source_pool: "node_delivery".to_owned(),
            amount_minor_units: amount_minor_units.to_owned(),
        }
    }

    fn identity() -> RocEpochTransitionIdentityV1 {
        RocEpochTransitionIdentityV1::from_expectation_and_allocations(
            &expectation(),
            "1000",
            vec![
                allocation(
                    "allocation:alpha",
                    "reward_plan_allocation:alpha",
                    ALPHA_NODE,
                    "400",
                ),
                allocation(
                    "allocation:beta",
                    "reward_plan_allocation:beta",
                    "service_node:beta",
                    "600",
                ),
            ],
        )
    }

    fn decode_hex(input: &str) -> Vec<u8> {
        assert_eq!(input.len() % 2, 0);

        input
            .as_bytes()
            .chunks_exact(2)
            .map(|chunk| {
                let high = hex_nibble(chunk[0]);
                let low = hex_nibble(chunk[1]);

                (high << 4) | low
            })
            .collect()
    }

    fn hex_nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("unexpected non-lowercase-hex byte"),
        }
    }

    #[test]
    fn eligible_service_node_validates_hashes_and_signs_one_transition() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha")
            .expect("fixture key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key.clone());

        let transition_identity = identity();

        let signature = participant
            .validate_and_sign(&kms, &transition_identity)
            .expect("eligible Service Node must sign");

        assert_eq!(signature.chain_id, CHAIN_ID);
        assert_eq!(signature.epoch_id, EPOCH_ID);
        assert_eq!(signature.service_node_id, ALPHA_NODE);
        assert_eq!(signature.key_id, ALPHA_KEY_REF);
        assert_eq!(signature.algorithm, SignatureAlg::Ed25519);
        assert_eq!(signature.signature_wire.len(), 128);

        let message = service_node_signature_message_bytes(&signature)
            .expect("signature message must encode");

        let raw_signature = decode_hex(&signature.signature_wire);

        kms.verify(&key, &message, &raw_signature)
            .expect("returned Service Node signature must verify");
    }

    #[test]
    fn transition_hash_matches_locked_reward_loop_domain_zero_json_contract() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha-hash")
            .expect("fixture key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key);

        let transition_identity = identity();

        let signature = participant
            .validate_and_sign(&kms, &transition_identity)
            .expect("eligible Service Node must sign");

        let json = serde_json::to_vec(&transition_identity).expect("identity must serialize");

        let mut hasher = blake3::Hasher::new();

        hasher.update(ROC_EPOCH_TRANSITION_HASH_DOMAIN.as_bytes());
        hasher.update(&[0]);
        hasher.update(&json);

        let expected = ContentId::parse(&format!("b3:{}", hasher.finalize().to_hex()))
            .expect("expected hash must parse");

        assert_eq!(signature.transition_hash, expected);
    }

    #[test]
    fn participant_rejects_cross_chain_identity_before_signing() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha-chain")
            .expect("fixture key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new("other-chain", ALPHA_NODE, ALPHA_KEY_REF, key);

        let error = participant
            .validate_and_sign(&kms, &identity())
            .expect_err("cross-chain identity must reject");

        assert!(matches!(
            error,
            QuorumParticipationError::ChainMismatch { .. }
        ));
    }

    #[test]
    fn participant_rejects_node_absent_from_reviewed_eligibility() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-delta")
            .expect("fixture key must be created");

        let participant = ServiceNodeQuorumParticipant::new(
            CHAIN_ID,
            "service_node:delta",
            "key:phase19:delta",
            key,
        );

        let error = participant
            .validate_and_sign(&kms, &identity())
            .expect_err("unreviewed Service Node must reject");

        assert!(matches!(
            error,
            QuorumParticipationError::ServiceNodeMissing { .. }
        ));
    }

    #[test]
    fn participant_rejects_ineligible_service_node() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha-ineligible")
            .expect("fixture key must be created");

        let mut transition_identity = identity();

        transition_identity.eligibilities[0].status = EpochEligibilityStatusV1::Ineligible;

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key);

        let error = participant
            .validate_and_sign(&kms, &transition_identity)
            .expect_err("ineligible Service Node must reject");

        assert!(matches!(
            error,
            QuorumParticipationError::ServiceNodeIneligible { .. }
                | QuorumParticipationError::InvalidIdentity(_)
        ));
    }

    #[test]
    fn participant_rejects_logical_key_reference_mismatch() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha-key-mismatch")
            .expect("fixture key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, "key:phase19:wrong", key);

        let error = participant
            .validate_and_sign(&kms, &identity())
            .expect_err("wrong logical quorum key must reject");

        assert!(matches!(
            error,
            QuorumParticipationError::KeyReferenceMismatch { .. }
        ));
    }

    #[test]
    fn invalid_transition_identity_rejects_before_signature_creation() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha-invalid")
            .expect("fixture key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key);

        let mut transition_identity = identity();

        transition_identity.domain = "rustyonions.invalid-transition.v1".to_owned();

        let error = participant
            .validate_and_sign(&kms, &transition_identity)
            .expect_err("invalid canonical identity must reject");

        assert!(matches!(
            error,
            QuorumParticipationError::InvalidIdentity(_)
        ));
    }

    #[test]
    fn signature_cannot_be_reused_for_a_different_valid_identity() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha-tamper")
            .expect("fixture key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key.clone());

        let original_identity = identity();

        let original_signature = participant
            .validate_and_sign(&kms, &original_identity)
            .expect("original identity must sign");

        let mut changed_identity = original_identity;

        changed_identity.evidence_root = cid('2');

        changed_identity
            .validate()
            .expect("changed identity remains structurally valid");

        let changed_hash =
            validated_transition_hash(&changed_identity).expect("changed identity must hash");

        assert_ne!(original_signature.transition_hash, changed_hash);

        let mut poisoned_signature = original_signature.clone();

        poisoned_signature.transition_hash = changed_hash;

        let poisoned_message = service_node_signature_message_bytes(&poisoned_signature)
            .expect("poisoned signature message must encode");

        let original_raw = decode_hex(&original_signature.signature_wire);

        let original_message = service_node_signature_message_bytes(&original_signature)
            .expect("original signature message must encode");

        assert_ne!(
            original_message, poisoned_message,
            "changing the transition hash must change the canonical signature message"
        );

        let verifies_poisoned_message = kms
            .verify(&key, &poisoned_message, &original_raw)
            .expect("well-formed signature verification must complete");

        assert_eq!(
            verifies_poisoned_message, false,
            "signature for the original transition hash must not verify against the changed hash"
        );
    }

    #[test]
    fn participation_result_contains_no_wallet_ledger_or_finality_authority() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("service-node", "phase19-alpha-authority")
            .expect("fixture key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key);

        let signature = participant
            .validate_and_sign(&kms, &identity())
            .expect("eligible Service Node must sign");

        let value = serde_json::to_value(signature).expect("signature must serialize");

        let object = value
            .as_object()
            .expect("signature must serialize as an object");

        for forbidden in [
            "wallet_mutation",
            "ledger_mutation",
            "payout_authority",
            "receipt_created",
            "balance_truth",
            "finality",
            "quorum_finalized",
            "paid_unlock",
        ] {
            assert!(
                !object.contains_key(forbidden),
                "Service Node participation must not add authority field {forbidden}"
            );
        }
    }

    #[test]
    fn runtime_status_starts_without_quorum_participant_and_fails_closed() {
        let runtime = crate::types::RuntimeStatus::new();

        assert!(!runtime.quorum_participation_active());

        let error = runtime
            .sign_quorum_transition(&identity())
            .expect_err("unconfigured runtime must not manufacture a quorum signature");

        assert_eq!(error, QuorumParticipationRuntimeError::NotConfigured);
    }

    #[test]
    fn runtime_status_registers_one_participant_and_signs() {
        let runtime = crate::types::RuntimeStatus::new();

        let kms = Arc::new(memory_keystore());

        let key = kms
            .create_ed25519("service-node", "phase19-runtime-alpha")
            .expect("fixture runtime key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key.clone());

        let signer: Arc<dyn Signer> = kms.clone();

        let registration = Arc::new(QuorumParticipationRuntime::new(participant, signer));

        runtime
            .register_quorum_participant(registration)
            .expect("first runtime participant must register");

        assert!(runtime.quorum_participation_active());

        let signature = runtime
            .sign_quorum_transition(&identity())
            .expect("registered runtime participant must sign");

        assert_eq!(signature.service_node_id, ALPHA_NODE);

        assert_eq!(signature.key_id, ALPHA_KEY_REF);

        let message = service_node_signature_message_bytes(&signature)
            .expect("runtime signature message must encode");

        let raw_signature = decode_hex(&signature.signature_wire);

        let verifies = kms
            .verify(&key, &message, &raw_signature)
            .expect("runtime signature verification must complete");

        assert!(verifies);
    }

    #[test]
    fn runtime_status_rejects_second_participant_registration() {
        let runtime = crate::types::RuntimeStatus::new();

        let first_kms = Arc::new(memory_keystore());

        let first_key = first_kms
            .create_ed25519("service-node", "phase19-runtime-first")
            .expect("first fixture key must be created");

        let first_participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, first_key);

        let first_signer: Arc<dyn Signer> = first_kms;

        runtime
            .register_quorum_participant(Arc::new(QuorumParticipationRuntime::new(
                first_participant,
                first_signer,
            )))
            .expect("first participant must register");

        let second_kms = Arc::new(memory_keystore());

        let second_key = second_kms
            .create_ed25519("service-node", "phase19-runtime-second")
            .expect("second fixture key must be created");

        let second_participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, second_key);

        let second_signer: Arc<dyn Signer> = second_kms;

        let error = runtime
            .register_quorum_participant(Arc::new(QuorumParticipationRuntime::new(
                second_participant,
                second_signer,
            )))
            .expect_err("second registration must not silently replace the active key");

        assert_eq!(error, QuorumParticipationRuntimeError::AlreadyConfigured);
    }

    #[test]
    fn runtime_debug_output_does_not_expose_signer_backend_material() {
        let kms = Arc::new(memory_keystore());

        let key = kms
            .create_ed25519("service-node", "phase19-runtime-debug")
            .expect("fixture runtime key must be created");

        let participant =
            ServiceNodeQuorumParticipant::new(CHAIN_ID, ALPHA_NODE, ALPHA_KEY_REF, key);

        let signer: Arc<dyn Signer> = kms;

        let registration = QuorumParticipationRuntime::new(participant, signer);

        let rendered = format!("{registration:?}");

        assert!(rendered.contains(CHAIN_ID));
        assert!(rendered.contains(ALPHA_NODE));
        assert!(rendered.contains(ALPHA_KEY_REF));
        assert!(rendered.contains("<redacted>"));

        assert!(!rendered.contains("private"));
        assert!(!rendered.contains("secret"));
        assert!(!rendered.contains("seed"));
    }
}
