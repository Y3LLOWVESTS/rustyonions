//! RO:WHAT — Signed policy-refusal and moderation-action review.
//! RO:WHY — Phase 13 needs verifiable policy evidence without reward authority.
//! RO:INTERACTS — ron-proto policy evidence, ron-kms, shared witness resolver.
//! RO:INVARIANTS — signature before replay; bounded process-local window.
//! RO:SECURITY — no runtime activation, deletion, provider, payout, or ledger claims.
//! RO:TEST — tests/service_node_policy_evidence.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use parking_lot::Mutex;
use ron_kms::backends::ed25519;
use ron_proto::{
    ModerationActionProofReplayKeyV1, ModerationActionProofV1,
    ModerationActionProofValidationError, PolicyRefusalProofReplayKeyV1, PolicyRefusalProofV1,
    PolicyRefusalProofValidationError, ServiceChallengeAckV1, ServiceChallengeAckValidationError,
};
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

use super::challenge_evidence::WitnessKeyResolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PolicyEvidenceReviewerConfigError {
    #[error("policy evidence replay capacity must be greater than zero")]
    ZeroReplayCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolicyEvidenceReplayKeyV1 {
    PolicyRefusal(PolicyRefusalProofReplayKeyV1),
    ModerationAction(ModerationActionProofReplayKeyV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRefusalEvidenceCandidateV1 {
    pub proof: PolicyRefusalProofV1,
    pub replay_key: PolicyRefusalProofReplayKeyV1,
    pub requester_ack: ServiceChallengeAckV1,

    pub requester_signature_verified: bool,
    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModerationActionEvidenceCandidateV1 {
    pub proof: ModerationActionProofV1,
    pub replay_key: ModerationActionProofReplayKeyV1,
    pub witness_ack: ServiceChallengeAckV1,

    pub witness_signature_verified: bool,
    pub runtime_activation_proven: bool,
    pub storage_delete_proven: bool,
    pub provider_withdrawal_proven: bool,
    pub network_propagation_proven: bool,
    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyEvidenceCandidateV1 {
    PolicyRefusal(Box<PolicyRefusalEvidenceCandidateV1>),
    ModerationAction(Box<ModerationActionEvidenceCandidateV1>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyEvidenceReview {
    Candidate(Box<PolicyEvidenceCandidateV1>),

    Duplicate {
        replay_key: Box<PolicyEvidenceReplayKeyV1>,
    },

    PolicyRefusalInvalid {
        error: PolicyRefusalProofValidationError,
    },

    ModerationActionInvalid {
        error: ModerationActionProofValidationError,
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

pub struct PolicyEvidenceReviewer {
    replay: Mutex<ReplayWindow>,
}

impl PolicyEvidenceReviewer {
    pub fn new(replay_capacity: usize) -> Result<Self, PolicyEvidenceReviewerConfigError> {
        if replay_capacity == 0 {
            return Err(PolicyEvidenceReviewerConfigError::ZeroReplayCapacity);
        }

        Ok(Self {
            replay: Mutex::new(ReplayWindow::new(replay_capacity)),
        })
    }

    pub fn review_policy_refusal_signed<R>(
        &self,
        proof: PolicyRefusalProofV1,
        acknowledgment: ServiceChallengeAckV1,
        resolver: &R,
    ) -> PolicyEvidenceReview
    where
        R: WitnessKeyResolver,
    {
        if let Err(error) = proof.validate() {
            return PolicyEvidenceReview::PolicyRefusalInvalid { error };
        }

        let signature = match acknowledgment.validate_for_policy_refusal(&proof) {
            Ok(signature) => signature,
            Err(error) => {
                return PolicyEvidenceReview::AcknowledgmentInvalid { error };
            }
        };

        if let Err(review) = verify_signature(
            &proof.requester_ack_signing_bytes(&acknowledgment.witness_key_id),
            signature,
            &acknowledgment,
            resolver,
        ) {
            return review;
        }

        let proof_replay_key = proof.replay_key();
        let replay_key = PolicyEvidenceReplayKeyV1::PolicyRefusal(proof_replay_key.clone());

        if !self.observe(replay_key.clone()) {
            return PolicyEvidenceReview::Duplicate {
                replay_key: Box::new(replay_key),
            };
        }

        PolicyEvidenceReview::Candidate(Box::new(PolicyEvidenceCandidateV1::PolicyRefusal(
            Box::new(PolicyRefusalEvidenceCandidateV1 {
                proof,
                replay_key: proof_replay_key,
                requester_ack: acknowledgment,
                requester_signature_verified: true,
                accounting_accepted: false,
                reward_eligible: false,
                reward_truth: false,
                payout_authority: false,
                wallet_mutation: false,
                ledger_mutation: false,
            }),
        )))
    }

    pub fn review_moderation_action_signed<R>(
        &self,
        proof: ModerationActionProofV1,
        acknowledgment: ServiceChallengeAckV1,
        resolver: &R,
    ) -> PolicyEvidenceReview
    where
        R: WitnessKeyResolver,
    {
        if let Err(error) = proof.validate() {
            return PolicyEvidenceReview::ModerationActionInvalid { error };
        }

        let signature = match acknowledgment.validate_for_moderation_action(&proof) {
            Ok(signature) => signature,
            Err(error) => {
                return PolicyEvidenceReview::AcknowledgmentInvalid { error };
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
        let replay_key = PolicyEvidenceReplayKeyV1::ModerationAction(proof_replay_key.clone());

        if !self.observe(replay_key.clone()) {
            return PolicyEvidenceReview::Duplicate {
                replay_key: Box::new(replay_key),
            };
        }

        PolicyEvidenceReview::Candidate(Box::new(PolicyEvidenceCandidateV1::ModerationAction(
            Box::new(ModerationActionEvidenceCandidateV1 {
                proof,
                replay_key: proof_replay_key,
                witness_ack: acknowledgment,
                witness_signature_verified: true,
                runtime_activation_proven: false,
                storage_delete_proven: false,
                provider_withdrawal_proven: false,
                network_propagation_proven: false,
                accounting_accepted: false,
                reward_eligible: false,
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

    fn observe(&self, replay_key: PolicyEvidenceReplayKeyV1) -> bool {
        self.replay.lock().observe(replay_key)
    }
}

fn verify_signature<R>(
    signing_bytes: &[u8],
    signature: [u8; 64],
    acknowledgment: &ServiceChallengeAckV1,
    resolver: &R,
) -> Result<(), PolicyEvidenceReview>
where
    R: WitnessKeyResolver,
{
    let witness_node_id = acknowledgment.witness_node_id.clone();
    let witness_key_id = acknowledgment.witness_key_id.clone();

    let Some(public_key) = resolver.resolve_ed25519_key(&witness_node_id, &witness_key_id) else {
        return Err(PolicyEvidenceReview::WitnessKeyUnavailable {
            witness_node_id,
            witness_key_id,
        });
    };

    if !ed25519::verify(&public_key, signing_bytes, &signature) {
        return Err(PolicyEvidenceReview::WitnessSignatureInvalid {
            witness_node_id,
            witness_key_id,
        });
    }

    Ok(())
}

struct ReplayWindow {
    capacity: usize,
    seen: HashSet<PolicyEvidenceReplayKeyV1>,
    order: VecDeque<PolicyEvidenceReplayKeyV1>,
}

impl ReplayWindow {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            seen: HashSet::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    fn observe(&mut self, replay_key: PolicyEvidenceReplayKeyV1) -> bool {
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
