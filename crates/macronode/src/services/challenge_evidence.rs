//! RO:WHAT — Signed review for availability and hot-cache evidence.
//! RO:WHY — Phase 13 requires independent witnesses and duplicate rejection.
//! RO:INTERACTS — ron-proto challenge evidence and ron-kms Ed25519.
//! RO:INVARIANTS — signature before replay; bounded process-local memory.
//! RO:SECURITY — resolver supplies trusted key; no payout or mutation authority.
//! RO:TEST — tests/service_node_challenge_evidence.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use parking_lot::Mutex;
use ron_kms::backends::ed25519;
use ron_proto::{
    AvailabilityProofReplayKeyV1, AvailabilityProofV1, ChallengeEvidenceValidationError,
    HotCacheProofReplayKeyV1, HotCacheProofV1, ServiceChallengeAckV1,
    ServiceChallengeAckValidationError,
};
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

/// Resolves a trusted Ed25519 witness key.
///
/// The proof and acknowledgment cannot self-supply the trusted key.
pub trait WitnessKeyResolver {
    fn resolve_ed25519_key(&self, witness_node_id: &str, witness_key_id: &str) -> Option<[u8; 32]>;
}

impl<F> WitnessKeyResolver for F
where
    F: Fn(&str, &str) -> Option<[u8; 32]>,
{
    fn resolve_ed25519_key(&self, witness_node_id: &str, witness_key_id: &str) -> Option<[u8; 32]> {
        self(witness_node_id, witness_key_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ChallengeEvidenceReviewerConfigError {
    #[error("challenge evidence replay capacity must be greater than zero")]
    ZeroReplayCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChallengeEvidenceReplayKeyV1 {
    Availability(AvailabilityProofReplayKeyV1),
    HotCache(HotCacheProofReplayKeyV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailabilityEvidenceCandidateV1 {
    pub proof: AvailabilityProofV1,
    pub replay_key: AvailabilityProofReplayKeyV1,
    pub witness_ack: ServiceChallengeAckV1,

    pub witness_signature_verified: bool,
    pub accounting_accepted: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotCacheEvidenceCandidateV1 {
    pub proof: HotCacheProofV1,
    pub replay_key: HotCacheProofReplayKeyV1,
    pub witness_ack: ServiceChallengeAckV1,

    pub witness_signature_verified: bool,
    pub durable_storage_proven: bool,
    pub accounting_accepted: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeEvidenceCandidateV1 {
    Availability(Box<AvailabilityEvidenceCandidateV1>),
    HotCache(Box<HotCacheEvidenceCandidateV1>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeEvidenceReview {
    Candidate(Box<ChallengeEvidenceCandidateV1>),

    Duplicate {
        replay_key: ChallengeEvidenceReplayKeyV1,
    },

    AvailabilityInvalid {
        error: ChallengeEvidenceValidationError,
    },

    HotCacheInvalid {
        error: ChallengeEvidenceValidationError,
    },

    AcknowledgmentInvalid {
        error: ServiceChallengeAckValidationError,
    },

    WitnessKeyUnavailable {
        witness_node_id: String,
        witness_key_id: String,
    },

    WitnessSignatureInvalid {
        witness_node_id: String,
        witness_key_id: String,
    },
}

/// Process-local bounded reviewer.
///
/// This does not claim durable or network-wide replay protection.
pub struct ChallengeEvidenceReviewer {
    replay: Mutex<ReplayWindow>,
}

impl ChallengeEvidenceReviewer {
    pub fn new(replay_capacity: usize) -> Result<Self, ChallengeEvidenceReviewerConfigError> {
        if replay_capacity == 0 {
            return Err(ChallengeEvidenceReviewerConfigError::ZeroReplayCapacity);
        }

        Ok(Self {
            replay: Mutex::new(ReplayWindow::new(replay_capacity)),
        })
    }

    pub fn review_availability_signed<R>(
        &self,
        proof: AvailabilityProofV1,
        acknowledgment: ServiceChallengeAckV1,
        resolver: &R,
    ) -> ChallengeEvidenceReview
    where
        R: WitnessKeyResolver,
    {
        if let Err(error) = proof.validate() {
            return ChallengeEvidenceReview::AvailabilityInvalid { error };
        }

        let signature = match acknowledgment.validate_for_availability(&proof) {
            Ok(signature) => signature,
            Err(error) => {
                return ChallengeEvidenceReview::AcknowledgmentInvalid { error };
            }
        };

        let witness_node_id = acknowledgment.witness_node_id.clone();
        let witness_key_id = acknowledgment.witness_key_id.clone();

        let Some(public_key) = resolver.resolve_ed25519_key(&witness_node_id, &witness_key_id)
        else {
            return ChallengeEvidenceReview::WitnessKeyUnavailable {
                witness_node_id,
                witness_key_id,
            };
        };

        let signing_bytes = proof.witness_ack_signing_bytes(&acknowledgment.witness_key_id);

        if !ed25519::verify(&public_key, &signing_bytes, &signature) {
            return ChallengeEvidenceReview::WitnessSignatureInvalid {
                witness_node_id,
                witness_key_id,
            };
        }

        let proof_replay_key = proof.replay_key();
        let replay_key = ChallengeEvidenceReplayKeyV1::Availability(proof_replay_key.clone());

        if !self.observe(replay_key.clone()) {
            return ChallengeEvidenceReview::Duplicate { replay_key };
        }

        ChallengeEvidenceReview::Candidate(Box::new(ChallengeEvidenceCandidateV1::Availability(
            Box::new(AvailabilityEvidenceCandidateV1 {
                proof,
                replay_key: proof_replay_key,
                witness_ack: acknowledgment,
                witness_signature_verified: true,
                accounting_accepted: false,
                reward_truth: false,
                payout_authority: false,
                wallet_mutation: false,
                ledger_mutation: false,
            }),
        )))
    }

    pub fn review_hot_cache_signed<R>(
        &self,
        proof: HotCacheProofV1,
        acknowledgment: ServiceChallengeAckV1,
        resolver: &R,
    ) -> ChallengeEvidenceReview
    where
        R: WitnessKeyResolver,
    {
        if let Err(error) = proof.validate() {
            return ChallengeEvidenceReview::HotCacheInvalid { error };
        }

        let signature = match acknowledgment.validate_for_hot_cache(&proof) {
            Ok(signature) => signature,
            Err(error) => {
                return ChallengeEvidenceReview::AcknowledgmentInvalid { error };
            }
        };

        let witness_node_id = acknowledgment.witness_node_id.clone();
        let witness_key_id = acknowledgment.witness_key_id.clone();

        let Some(public_key) = resolver.resolve_ed25519_key(&witness_node_id, &witness_key_id)
        else {
            return ChallengeEvidenceReview::WitnessKeyUnavailable {
                witness_node_id,
                witness_key_id,
            };
        };

        let signing_bytes = proof.witness_ack_signing_bytes(&acknowledgment.witness_key_id);

        if !ed25519::verify(&public_key, &signing_bytes, &signature) {
            return ChallengeEvidenceReview::WitnessSignatureInvalid {
                witness_node_id,
                witness_key_id,
            };
        }

        let proof_replay_key = proof.replay_key();
        let replay_key = ChallengeEvidenceReplayKeyV1::HotCache(proof_replay_key.clone());

        if !self.observe(replay_key.clone()) {
            return ChallengeEvidenceReview::Duplicate { replay_key };
        }

        ChallengeEvidenceReview::Candidate(Box::new(ChallengeEvidenceCandidateV1::HotCache(
            Box::new(HotCacheEvidenceCandidateV1 {
                proof,
                replay_key: proof_replay_key,
                witness_ack: acknowledgment,
                witness_signature_verified: true,
                durable_storage_proven: false,
                accounting_accepted: false,
                reward_truth: false,
                payout_authority: false,
                wallet_mutation: false,
                ledger_mutation: false,
            }),
        )))
    }

    pub fn observed_key_count(&self) -> usize {
        self.replay.lock().observed_count()
    }

    fn observe(&self, replay_key: ChallengeEvidenceReplayKeyV1) -> bool {
        self.replay.lock().observe(replay_key)
    }
}

struct ReplayWindow {
    capacity: usize,
    seen: HashSet<ChallengeEvidenceReplayKeyV1>,
    order: VecDeque<ChallengeEvidenceReplayKeyV1>,
}

impl ReplayWindow {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            seen: HashSet::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    fn observe(&mut self, replay_key: ChallengeEvidenceReplayKeyV1) -> bool {
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
