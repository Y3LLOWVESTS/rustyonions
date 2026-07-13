//! RO:WHAT — Signed range-request and repair evidence review.
//! RO:WHY — Phase 13 requires exact partial-delivery and repair evidence.
//! RO:INTERACTS — shared witness resolver, ron-proto, ron-kms.
//! RO:INVARIANTS — signature before replay; bounded process-local window.
//! RO:SECURITY — no full-object, network-repair, provider, or economic claims.
//! RO:TEST — tests/service_node_range_repair_evidence.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use parking_lot::Mutex;
use ron_kms::backends::ed25519;
use ron_proto::{
    RangeRequestProofReplayKeyV1, RangeRequestProofV1, RangeRequestProofValidationError,
    RepairProofReplayKeyV1, RepairProofV1, RepairProofValidationError, ServiceChallengeAckV1,
    ServiceChallengeAckValidationError,
};
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

use super::challenge_evidence::WitnessKeyResolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RangeRepairReviewerConfigError {
    #[error("range/repair replay capacity must be greater than zero")]
    ZeroReplayCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RangeRepairReplayKeyV1 {
    RangeRequest(RangeRequestProofReplayKeyV1),
    Repair(RepairProofReplayKeyV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeRequestEvidenceCandidateV1 {
    pub proof: RangeRequestProofV1,
    pub replay_key: RangeRequestProofReplayKeyV1,
    pub witness_ack: ServiceChallengeAckV1,

    pub witness_signature_verified: bool,
    pub full_object_delivery_proven: bool,
    pub accounting_accepted: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairEvidenceCandidateV1 {
    pub proof: RepairProofV1,
    pub replay_key: RepairProofReplayKeyV1,
    pub witness_ack: ServiceChallengeAckV1,

    pub witness_signature_verified: bool,
    pub network_repair_proven: bool,
    pub provider_publication_proven: bool,
    pub accounting_accepted: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeRepairEvidenceCandidateV1 {
    RangeRequest(Box<RangeRequestEvidenceCandidateV1>),
    Repair(Box<RepairEvidenceCandidateV1>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeRepairEvidenceReview {
    Candidate(Box<RangeRepairEvidenceCandidateV1>),

    Duplicate {
        replay_key: Box<RangeRepairReplayKeyV1>,
    },

    RangeRequestInvalid {
        error: RangeRequestProofValidationError,
    },

    RepairInvalid {
        error: RepairProofValidationError,
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

pub struct RangeRepairEvidenceReviewer {
    replay: Mutex<ReplayWindow>,
}

impl RangeRepairEvidenceReviewer {
    pub fn new(replay_capacity: usize) -> Result<Self, RangeRepairReviewerConfigError> {
        if replay_capacity == 0 {
            return Err(RangeRepairReviewerConfigError::ZeroReplayCapacity);
        }

        Ok(Self {
            replay: Mutex::new(ReplayWindow::new(replay_capacity)),
        })
    }

    pub fn review_range_request_signed<R>(
        &self,
        proof: RangeRequestProofV1,
        acknowledgment: ServiceChallengeAckV1,
        resolver: &R,
    ) -> RangeRepairEvidenceReview
    where
        R: WitnessKeyResolver,
    {
        if let Err(error) = proof.validate() {
            return RangeRepairEvidenceReview::RangeRequestInvalid { error };
        }

        let signature = match acknowledgment.validate_for_range_request(&proof) {
            Ok(signature) => signature,
            Err(error) => {
                return RangeRepairEvidenceReview::AcknowledgmentInvalid { error };
            }
        };

        if let Err(review) = verify_signature(
            &proof.witness_ack_signing_bytes(&acknowledgment.witness_key_id),
            signature,
            &acknowledgment,
            resolver,
        ) {
            return review;
        }

        let proof_replay_key = proof.replay_key();
        let replay_key = RangeRepairReplayKeyV1::RangeRequest(proof_replay_key.clone());

        if !self.observe(replay_key.clone()) {
            return RangeRepairEvidenceReview::Duplicate {
                replay_key: Box::new(replay_key),
            };
        }

        RangeRepairEvidenceReview::Candidate(Box::new(
            RangeRepairEvidenceCandidateV1::RangeRequest(Box::new(
                RangeRequestEvidenceCandidateV1 {
                    proof,
                    replay_key: proof_replay_key,
                    witness_ack: acknowledgment,
                    witness_signature_verified: true,
                    full_object_delivery_proven: false,
                    accounting_accepted: false,
                    reward_truth: false,
                    payout_authority: false,
                    wallet_mutation: false,
                    ledger_mutation: false,
                },
            )),
        ))
    }

    pub fn review_repair_signed<R>(
        &self,
        proof: RepairProofV1,
        acknowledgment: ServiceChallengeAckV1,
        resolver: &R,
    ) -> RangeRepairEvidenceReview
    where
        R: WitnessKeyResolver,
    {
        if let Err(error) = proof.validate() {
            return RangeRepairEvidenceReview::RepairInvalid { error };
        }

        let signature = match acknowledgment.validate_for_repair(&proof) {
            Ok(signature) => signature,
            Err(error) => {
                return RangeRepairEvidenceReview::AcknowledgmentInvalid { error };
            }
        };

        if let Err(review) = verify_signature(
            &proof.witness_ack_signing_bytes(&acknowledgment.witness_key_id),
            signature,
            &acknowledgment,
            resolver,
        ) {
            return review;
        }

        let proof_replay_key = proof.replay_key();
        let replay_key = RangeRepairReplayKeyV1::Repair(proof_replay_key.clone());

        if !self.observe(replay_key.clone()) {
            return RangeRepairEvidenceReview::Duplicate {
                replay_key: Box::new(replay_key),
            };
        }

        RangeRepairEvidenceReview::Candidate(Box::new(RangeRepairEvidenceCandidateV1::Repair(
            Box::new(RepairEvidenceCandidateV1 {
                proof,
                replay_key: proof_replay_key,
                witness_ack: acknowledgment,
                witness_signature_verified: true,
                network_repair_proven: false,
                provider_publication_proven: false,
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

    fn observe(&self, replay_key: RangeRepairReplayKeyV1) -> bool {
        self.replay.lock().observe(replay_key)
    }
}

fn verify_signature<R>(
    signing_bytes: &[u8],
    signature: [u8; 64],
    acknowledgment: &ServiceChallengeAckV1,
    resolver: &R,
) -> Result<(), RangeRepairEvidenceReview>
where
    R: WitnessKeyResolver,
{
    let witness_node_id = acknowledgment.witness_node_id.clone();
    let witness_key_id = acknowledgment.witness_key_id.clone();

    let Some(public_key) = resolver.resolve_ed25519_key(&witness_node_id, &witness_key_id) else {
        return Err(RangeRepairEvidenceReview::WitnessKeyUnavailable {
            witness_node_id,
            witness_key_id,
        });
    };

    if !ed25519::verify(&public_key, signing_bytes, &signature) {
        return Err(RangeRepairEvidenceReview::WitnessSignatureInvalid {
            witness_node_id,
            witness_key_id,
        });
    }

    Ok(())
}

struct ReplayWindow {
    capacity: usize,
    seen: HashSet<RangeRepairReplayKeyV1>,
    order: VecDeque<RangeRepairReplayKeyV1>,
}

impl ReplayWindow {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            seen: HashSet::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    fn observe(&mut self, replay_key: RangeRepairReplayKeyV1) -> bool {
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
