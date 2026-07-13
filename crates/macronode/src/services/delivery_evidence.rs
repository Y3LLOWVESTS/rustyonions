//! RO:WHAT — Bounded runtime review for requester-corroborated delivery evidence.
//! RO:WHY — Phase 13 requires duplicate rejection, signatures, and integrity findings.
//! RO:INTERACTS — ron-proto evidence DTOs, ron-kms Ed25519, later accounting.
//! RO:INVARIANTS — validate first; signature before replay; bounded local window.
//! RO:SECURITY — key resolved externally; no requester IP; no economic authority.
//! RO:TEST — service_node_delivery_evidence_review.rs and
//!   service_node_delivery_evidence_signature.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use parking_lot::Mutex;
use ron_kms::backends::ed25519;
use ron_proto::{
    ContentId, DeliveryProofReplayKeyV1, DeliveryProofV1, DeliveryProofValidationError,
    DeliveryRequesterAckV1, DeliveryRequesterAckValidationError,
};
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

pub const CORRUPT_DELIVERY_FINDING_SCHEMA: &str = "macronode.delivery_integrity_finding.v1";

/// Resolves a trusted Ed25519 requester key.
///
/// Implementations may consult passport, registry, or another authenticated
/// identity surface. The submitted evidence cannot provide the trusted key.
pub trait RequesterKeyResolver {
    fn resolve_ed25519_key(
        &self,
        requester_node_id: &str,
        requester_key_id: &str,
    ) -> Option<[u8; 32]>;
}

impl<F> RequesterKeyResolver for F
where
    F: Fn(&str, &str) -> Option<[u8; 32]>,
{
    fn resolve_ed25519_key(
        &self,
        requester_node_id: &str,
        requester_key_id: &str,
    ) -> Option<[u8; 32]> {
        self(requester_node_id, requester_key_id)
    }
}

/// Invalid delivery-evidence reviewer configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DeliveryEvidenceReviewerConfigError {
    #[error("delivery evidence replay capacity must be greater than zero")]
    ZeroReplayCapacity,
}

/// A valid delivery proof awaiting or carrying cryptographic review.
///
/// This remains evidence only. Signature verification is not accounting
/// acceptance and cannot assign ROC value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryEvidenceCandidateV1 {
    pub proof: DeliveryProofV1,
    pub replay_key: DeliveryProofReplayKeyV1,
    pub requester_ack: Option<DeliveryRequesterAckV1>,

    pub requester_signature_verified: bool,
    pub accounting_accepted: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Local non-economic finding for failed content-address verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorruptDeliveryFindingV1 {
    pub schema: &'static str,
    pub proof_id: String,
    pub service_node_id: String,
    pub requester_node_id: String,
    pub request_id: String,
    pub expected_content_id: ContentId,

    pub finding: &'static str,
    pub provider_integrity_failure: bool,
    pub requester_signature_verified: bool,
    pub requester_key_id: Option<String>,

    pub reward_eligible: bool,
    pub automatic_economic_penalty: bool,
    pub evidence_only: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Deterministic result of one local delivery-evidence review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryEvidenceReview {
    Candidate(Box<DeliveryEvidenceCandidateV1>),

    Duplicate {
        replay_key: DeliveryProofReplayKeyV1,
    },

    CorruptDelivery(CorruptDeliveryFindingV1),

    Invalid {
        error: DeliveryProofValidationError,
    },

    AcknowledgmentInvalid {
        error: DeliveryRequesterAckValidationError,
    },

    RequesterKeyUnavailable {
        requester_node_id: String,
        requester_key_id: String,
    },

    RequesterSignatureInvalid {
        requester_node_id: String,
        requester_key_id: String,
    },
}

/// Process-local bounded reviewer.
pub struct DeliveryEvidenceReviewer {
    replay: Mutex<ReplayWindow>,
}

impl DeliveryEvidenceReviewer {
    pub fn new(replay_capacity: usize) -> Result<Self, DeliveryEvidenceReviewerConfigError> {
        if replay_capacity == 0 {
            return Err(DeliveryEvidenceReviewerConfigError::ZeroReplayCapacity);
        }

        Ok(Self {
            replay: Mutex::new(ReplayWindow::new(replay_capacity)),
        })
    }

    /// Structural preflight without cryptographic acceptance.
    ///
    /// This path keeps `requester_signature_verified=false`. A network ingress
    /// or accounting handoff must use `review_signed`.
    pub fn review(&self, proof: DeliveryProofV1) -> DeliveryEvidenceReview {
        match classify_proof(&proof) {
            Ok(ProofClassification::VerifiedDelivery) => self.review_valid(proof, None, false),
            Ok(ProofClassification::CorruptDelivery) => self.review_corrupt(proof, None, false),
            Err(error) => DeliveryEvidenceReview::Invalid { error },
        }
    }

    /// Verify a requester acknowledgment before replay insertion.
    ///
    /// The resolver supplies the trusted requester key. Unknown keys, malformed
    /// acknowledgments, and invalid signatures do not reserve replay state.
    pub fn review_signed<R>(
        &self,
        proof: DeliveryProofV1,
        acknowledgment: DeliveryRequesterAckV1,
        resolver: &R,
    ) -> DeliveryEvidenceReview
    where
        R: RequesterKeyResolver,
    {
        let classification = match classify_proof(&proof) {
            Ok(classification) => classification,
            Err(error) => {
                return DeliveryEvidenceReview::Invalid { error };
            }
        };

        let signature = match acknowledgment.validate_for(&proof) {
            Ok(signature) => signature,
            Err(error) => {
                return DeliveryEvidenceReview::AcknowledgmentInvalid { error };
            }
        };

        let requester_node_id = acknowledgment.requester_node_id.clone();
        let requester_key_id = acknowledgment.requester_key_id.clone();

        let Some(public_key) = resolver.resolve_ed25519_key(&requester_node_id, &requester_key_id)
        else {
            return DeliveryEvidenceReview::RequesterKeyUnavailable {
                requester_node_id,
                requester_key_id,
            };
        };

        let signing_bytes = proof.requester_ack_signing_bytes(&acknowledgment.requester_key_id);

        if !ed25519::verify(&public_key, &signing_bytes, &signature) {
            return DeliveryEvidenceReview::RequesterSignatureInvalid {
                requester_node_id,
                requester_key_id,
            };
        }

        match classification {
            ProofClassification::VerifiedDelivery => {
                self.review_valid(proof, Some(acknowledgment), true)
            }
            ProofClassification::CorruptDelivery => {
                self.review_corrupt(proof, Some(acknowledgment), true)
            }
        }
    }

    pub fn observed_key_count(&self) -> usize {
        self.replay.lock().observed_count()
    }

    fn review_valid(
        &self,
        proof: DeliveryProofV1,
        requester_ack: Option<DeliveryRequesterAckV1>,
        requester_signature_verified: bool,
    ) -> DeliveryEvidenceReview {
        let replay_key = proof.replay_key();

        if !self.observe_replay_key(replay_key.clone()) {
            return DeliveryEvidenceReview::Duplicate { replay_key };
        }

        DeliveryEvidenceReview::Candidate(Box::new(DeliveryEvidenceCandidateV1 {
            proof,
            replay_key,
            requester_ack,
            requester_signature_verified,
            accounting_accepted: false,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,
        }))
    }

    fn review_corrupt(
        &self,
        proof: DeliveryProofV1,
        requester_ack: Option<DeliveryRequesterAckV1>,
        requester_signature_verified: bool,
    ) -> DeliveryEvidenceReview {
        let replay_key = proof.replay_key();

        if !self.observe_replay_key(replay_key.clone()) {
            return DeliveryEvidenceReview::Duplicate { replay_key };
        }

        let requester_key_id = requester_ack
            .as_ref()
            .map(|ack| ack.requester_key_id.clone());

        DeliveryEvidenceReview::CorruptDelivery(CorruptDeliveryFindingV1 {
            schema: CORRUPT_DELIVERY_FINDING_SCHEMA,
            proof_id: proof.proof_id,
            service_node_id: proof.service_node_id,
            requester_node_id: proof.requester_node_id,
            request_id: proof.request_id,
            expected_content_id: proof.content_id,
            finding: "content_address_verification_failed",
            provider_integrity_failure: true,
            requester_signature_verified,
            requester_key_id,
            reward_eligible: false,
            automatic_economic_penalty: false,
            evidence_only: true,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,
        })
    }

    fn observe_replay_key(&self, replay_key: DeliveryProofReplayKeyV1) -> bool {
        self.replay.lock().observe(replay_key)
    }
}

#[derive(Clone, Copy)]
enum ProofClassification {
    VerifiedDelivery,
    CorruptDelivery,
}

fn classify_proof(
    proof: &DeliveryProofV1,
) -> Result<ProofClassification, DeliveryProofValidationError> {
    match proof.validate() {
        Ok(()) => Ok(ProofClassification::VerifiedDelivery),

        Err(DeliveryProofValidationError::UnverifiedContent) => {
            let mut otherwise_valid = proof.clone();
            otherwise_valid.content_verified = true;
            otherwise_valid.validate()?;

            Ok(ProofClassification::CorruptDelivery)
        }

        Err(error) => Err(error),
    }
}

struct ReplayWindow {
    capacity: usize,
    seen: HashSet<DeliveryProofReplayKeyV1>,
    order: VecDeque<DeliveryProofReplayKeyV1>,
}

impl ReplayWindow {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            seen: HashSet::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    fn observe(&mut self, replay_key: DeliveryProofReplayKeyV1) -> bool {
        if self.seen.contains(&replay_key) {
            return false;
        }

        if self.order.len() == self.capacity {
            if let Some(expired) = self.order.pop_front() {
                self.seen.remove(&expired);
            }
        }

        self.seen.insert(replay_key.clone());
        self.order.push_back(replay_key);
        true
    }

    fn observed_count(&self) -> usize {
        self.seen.len()
    }
}
