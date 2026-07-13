//! RO:WHAT — Bounded process-local outbox for reviewed service evidence.
//! RO:WHY — Phase 13 needs one runtime stream after proof review succeeds.
//! RO:INTERACTS — all seven Phase 13 evidence reviewers and RuntimeStatus.
//! RO:INVARIANTS — reviewed signatures only; replay identities remain unique.
//! RO:SECURITY — never accounting, reward, payout, wallet, or ledger truth.
//! RO:TEST — focused unit tests beside this behavior.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use parking_lot::Mutex;
use ron_proto::{
    AvailabilityProofReplayKeyV1, ContentId, DeliveryProofReplayKeyV1, HotCacheProofReplayKeyV1,
    ModerationActionProofReplayKeyV1, PolicyRefusalProofReplayKeyV1, RangeRequestProofReplayKeyV1,
    RepairProofReplayKeyV1, ServiceEvidenceAccountingInputV1,
    ServiceEvidenceAccountingInputValidationError, ServiceEvidenceAccountingKindV1,
    SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA, SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
};
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

use super::{
    challenge_evidence::{AvailabilityEvidenceCandidateV1, HotCacheEvidenceCandidateV1},
    delivery_evidence::DeliveryEvidenceCandidateV1,
    policy_evidence::{ModerationActionEvidenceCandidateV1, PolicyRefusalEvidenceCandidateV1},
    range_repair_evidence::{RangeRequestEvidenceCandidateV1, RepairEvidenceCandidateV1},
};

pub const DEFAULT_SERVICE_EVIDENCE_OUTBOX_CAPACITY: usize = 1_024;
pub const MAX_SERVICE_EVIDENCE_READ_ITEMS: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEvidenceKindV1 {
    Delivery,
    Availability,
    RangeRequest,
    Repair,
    HotCache,
    PolicyRefusal,
    ModerationAction,
}

/// Safe runtime projection of one reviewed proof.
///
/// The full proof remains in its typed reviewer surface. The outbox records
/// only bounded routing/audit metadata and explicit non-authority posture.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceEvidenceRecordV1 {
    pub sequence: u64,
    pub kind: ServiceEvidenceKindV1,

    pub proof_id: String,
    pub service_node_id: String,
    pub witness_node_id: String,

    /// Optional second actor such as a repair source or local operator.
    pub related_actor_ids: Vec<String>,

    pub content_id: ContentId,
    pub observed_at_ms: u64,

    pub signature_verified: bool,
    pub evidence_only: bool,

    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

impl ServiceEvidenceRecordV1 {
    /// Convert this reviewed runtime record into the canonical Phase 14
    /// accounting handoff.
    ///
    /// The conversion revalidates the shared DTO and cannot add reward,
    /// payout, wallet, or ledger authority.
    pub fn accounting_input(
        &self,
    ) -> Result<ServiceEvidenceAccountingInputV1, ServiceEvidenceAccountingInputValidationError>
    {
        let kind = match self.kind {
            ServiceEvidenceKindV1::Delivery => ServiceEvidenceAccountingKindV1::Delivery,
            ServiceEvidenceKindV1::Availability => ServiceEvidenceAccountingKindV1::Availability,
            ServiceEvidenceKindV1::RangeRequest => ServiceEvidenceAccountingKindV1::RangeRequest,
            ServiceEvidenceKindV1::Repair => ServiceEvidenceAccountingKindV1::Repair,
            ServiceEvidenceKindV1::HotCache => ServiceEvidenceAccountingKindV1::HotCache,
            ServiceEvidenceKindV1::PolicyRefusal => ServiceEvidenceAccountingKindV1::PolicyRefusal,
            ServiceEvidenceKindV1::ModerationAction => {
                ServiceEvidenceAccountingKindV1::ModerationAction
            }
        };

        let mut related_actor_ids = self.related_actor_ids.clone();
        related_actor_ids.sort();
        related_actor_ids.dedup();

        let input = ServiceEvidenceAccountingInputV1 {
            schema: SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA.to_owned(),
            version: SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
            sequence: self.sequence,
            kind,
            proof_id: self.proof_id.clone(),
            service_node_id: self.service_node_id.clone(),
            witness_node_id: self.witness_node_id.clone(),
            related_actor_ids,
            content_id: self.content_id.clone(),
            observed_at_ms: self.observed_at_ms,
            signature_verified: self.signature_verified,
            evidence_only: self.evidence_only,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: self.reward_eligible,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
        };

        input.validate()?;
        Ok(input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEvidencePublishOutcome {
    Published(Box<ServiceEvidenceRecordV1>),

    Duplicate {
        kind: ServiceEvidenceKindV1,
        proof_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ServiceEvidenceOutboxError {
    #[error("service evidence outbox capacity must be greater than zero")]
    ZeroCapacity,

    #[error("service evidence read limit must be within 1..={maximum}; actual={actual}")]
    InvalidReadLimit { actual: usize, maximum: usize },

    #[error("{kind:?} candidate proof was rejected before publication: {reason}")]
    InvalidProof {
        kind: ServiceEvidenceKindV1,
        reason: String,
    },

    #[error("{kind:?} candidate boundary was rejected: {field}")]
    InvalidCandidate {
        kind: ServiceEvidenceKindV1,
        field: &'static str,
    },

    #[error("service evidence outbox sequence exhausted")]
    SequenceExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ServiceEvidenceReplayKeyV1 {
    Delivery(DeliveryProofReplayKeyV1),
    Availability(AvailabilityProofReplayKeyV1),
    RangeRequest(RangeRequestProofReplayKeyV1),
    Repair(RepairProofReplayKeyV1),
    HotCache(HotCacheProofReplayKeyV1),
    PolicyRefusal(PolicyRefusalProofReplayKeyV1),
    ModerationAction(ModerationActionProofReplayKeyV1),
}

pub(crate) struct ServiceEvidenceDraftV1 {
    key: ServiceEvidenceReplayKeyV1,
    kind: ServiceEvidenceKindV1,

    proof_id: String,
    service_node_id: String,
    witness_node_id: String,
    related_actor_ids: Vec<String>,

    content_id: ContentId,
    observed_at_ms: u64,

    signature_verified: bool,
    accounting_accepted: bool,
    reward_eligible: bool,
    reward_truth: bool,
    payout_authority: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,

    unsupported_claims: Vec<(&'static str, bool)>,
}

pub(crate) trait VerifiedServiceEvidence {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError>;
}

impl VerifiedServiceEvidence for DeliveryEvidenceCandidateV1 {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
        let kind = ServiceEvidenceKindV1::Delivery;

        self.proof
            .validate()
            .map_err(|error| ServiceEvidenceOutboxError::InvalidProof {
                kind,
                reason: error.to_string(),
            })?;

        if self.replay_key != self.proof.replay_key() {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind,
                field: "replay_key",
            });
        }

        Ok(ServiceEvidenceDraftV1 {
            key: ServiceEvidenceReplayKeyV1::Delivery(self.replay_key.clone()),
            kind,
            proof_id: self.proof.proof_id.clone(),
            service_node_id: self.proof.service_node_id.clone(),
            witness_node_id: self.proof.requester_node_id.clone(),
            related_actor_ids: Vec::new(),
            content_id: self.proof.content_id.clone(),
            observed_at_ms: self.proof.completed_at_ms,
            signature_verified: self.requester_signature_verified,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: false,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
            unsupported_claims: Vec::new(),
        })
    }
}

impl VerifiedServiceEvidence for AvailabilityEvidenceCandidateV1 {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
        let kind = ServiceEvidenceKindV1::Availability;

        self.proof
            .validate()
            .map_err(|error| ServiceEvidenceOutboxError::InvalidProof {
                kind,
                reason: error.to_string(),
            })?;

        if self.replay_key != self.proof.replay_key() {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind,
                field: "replay_key",
            });
        }

        Ok(ServiceEvidenceDraftV1 {
            key: ServiceEvidenceReplayKeyV1::Availability(self.replay_key.clone()),
            kind,
            proof_id: self.proof.proof_id.clone(),
            service_node_id: self.proof.service_node_id.clone(),
            witness_node_id: self.proof.witness_node_id.clone(),
            related_actor_ids: Vec::new(),
            content_id: self.proof.content_id.clone(),
            observed_at_ms: self.proof.response_completed_at_ms,
            signature_verified: self.witness_signature_verified,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: false,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
            unsupported_claims: Vec::new(),
        })
    }
}

impl VerifiedServiceEvidence for HotCacheEvidenceCandidateV1 {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
        let kind = ServiceEvidenceKindV1::HotCache;

        self.proof
            .validate()
            .map_err(|error| ServiceEvidenceOutboxError::InvalidProof {
                kind,
                reason: error.to_string(),
            })?;

        if self.replay_key != self.proof.replay_key() {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind,
                field: "replay_key",
            });
        }

        Ok(ServiceEvidenceDraftV1 {
            key: ServiceEvidenceReplayKeyV1::HotCache(self.replay_key.clone()),
            kind,
            proof_id: self.proof.proof_id.clone(),
            service_node_id: self.proof.service_node_id.clone(),
            witness_node_id: self.proof.witness_node_id.clone(),
            related_actor_ids: Vec::new(),
            content_id: self.proof.content_id.clone(),
            observed_at_ms: self.proof.response_completed_at_ms,
            signature_verified: self.witness_signature_verified,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: false,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
            unsupported_claims: vec![("durable_storage_proven", self.durable_storage_proven)],
        })
    }
}

impl VerifiedServiceEvidence for RangeRequestEvidenceCandidateV1 {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
        let kind = ServiceEvidenceKindV1::RangeRequest;

        self.proof
            .validate()
            .map_err(|error| ServiceEvidenceOutboxError::InvalidProof {
                kind,
                reason: error.to_string(),
            })?;

        if self.replay_key != self.proof.replay_key() {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind,
                field: "replay_key",
            });
        }

        Ok(ServiceEvidenceDraftV1 {
            key: ServiceEvidenceReplayKeyV1::RangeRequest(self.replay_key.clone()),
            kind,
            proof_id: self.proof.proof_id.clone(),
            service_node_id: self.proof.service_node_id.clone(),
            witness_node_id: self.proof.witness_node_id.clone(),
            related_actor_ids: Vec::new(),
            content_id: self.proof.content_id.clone(),
            observed_at_ms: self.proof.response_completed_at_ms,
            signature_verified: self.witness_signature_verified,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: false,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
            unsupported_claims: vec![(
                "full_object_delivery_proven",
                self.full_object_delivery_proven,
            )],
        })
    }
}

impl VerifiedServiceEvidence for RepairEvidenceCandidateV1 {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
        let kind = ServiceEvidenceKindV1::Repair;

        self.proof
            .validate()
            .map_err(|error| ServiceEvidenceOutboxError::InvalidProof {
                kind,
                reason: error.to_string(),
            })?;

        if self.replay_key != self.proof.replay_key() {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind,
                field: "replay_key",
            });
        }

        Ok(ServiceEvidenceDraftV1 {
            key: ServiceEvidenceReplayKeyV1::Repair(self.replay_key.clone()),
            kind,
            proof_id: self.proof.proof_id.clone(),
            service_node_id: self.proof.repairing_service_node_id.clone(),
            witness_node_id: self.proof.witness_node_id.clone(),
            related_actor_ids: vec![self.proof.source_service_node_id.clone()],
            content_id: self.proof.content_id.clone(),
            observed_at_ms: self.proof.repair_completed_at_ms,
            signature_verified: self.witness_signature_verified,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: false,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
            unsupported_claims: vec![
                ("network_repair_proven", self.network_repair_proven),
                (
                    "provider_publication_proven",
                    self.provider_publication_proven,
                ),
            ],
        })
    }
}

impl VerifiedServiceEvidence for PolicyRefusalEvidenceCandidateV1 {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
        let kind = ServiceEvidenceKindV1::PolicyRefusal;

        self.proof
            .validate()
            .map_err(|error| ServiceEvidenceOutboxError::InvalidProof {
                kind,
                reason: error.to_string(),
            })?;

        if self.replay_key != self.proof.replay_key() {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind,
                field: "replay_key",
            });
        }

        Ok(ServiceEvidenceDraftV1 {
            key: ServiceEvidenceReplayKeyV1::PolicyRefusal(self.replay_key.clone()),
            kind,
            proof_id: self.proof.proof_id.clone(),
            service_node_id: self.proof.service_node_id.clone(),
            witness_node_id: self.proof.requester_node_id.clone(),
            related_actor_ids: Vec::new(),
            content_id: self.proof.content_id.clone(),
            observed_at_ms: self.proof.refused_at_ms,
            signature_verified: self.requester_signature_verified,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: self.reward_eligible,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
            unsupported_claims: Vec::new(),
        })
    }
}

impl VerifiedServiceEvidence for ModerationActionEvidenceCandidateV1 {
    fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
        let kind = ServiceEvidenceKindV1::ModerationAction;

        self.proof
            .validate()
            .map_err(|error| ServiceEvidenceOutboxError::InvalidProof {
                kind,
                reason: error.to_string(),
            })?;

        if self.replay_key != self.proof.replay_key() {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind,
                field: "replay_key",
            });
        }

        Ok(ServiceEvidenceDraftV1 {
            key: ServiceEvidenceReplayKeyV1::ModerationAction(self.replay_key.clone()),
            kind,
            proof_id: self.proof.proof_id.clone(),
            service_node_id: self.proof.service_node_id.clone(),
            witness_node_id: self.proof.witness_node_id.clone(),
            related_actor_ids: vec![self.proof.operator_subject_id.clone()],
            content_id: self.proof.content_id.clone(),
            observed_at_ms: self.proof.action_recorded_at_ms,
            signature_verified: self.witness_signature_verified,
            accounting_accepted: self.accounting_accepted,
            reward_eligible: self.reward_eligible,
            reward_truth: self.reward_truth,
            payout_authority: self.payout_authority,
            wallet_mutation: self.wallet_mutation,
            ledger_mutation: self.ledger_mutation,
            unsupported_claims: vec![
                ("runtime_activation_proven", self.runtime_activation_proven),
                ("storage_delete_proven", self.storage_delete_proven),
                (
                    "provider_withdrawal_proven",
                    self.provider_withdrawal_proven,
                ),
                (
                    "network_propagation_proven",
                    self.network_propagation_proven,
                ),
            ],
        })
    }
}

#[derive(Debug)]
struct OutboxState {
    next_sequence: u64,
    seen: HashSet<ServiceEvidenceReplayKeyV1>,
    records: VecDeque<(ServiceEvidenceReplayKeyV1, ServiceEvidenceRecordV1)>,
}

impl OutboxState {
    fn new(capacity: usize) -> Self {
        Self {
            next_sequence: 1,
            seen: HashSet::with_capacity(capacity),
            records: VecDeque::with_capacity(capacity),
        }
    }
}

#[derive(Debug)]
pub struct ServiceEvidenceOutbox {
    capacity: usize,
    state: Mutex<OutboxState>,
}

impl ServiceEvidenceOutbox {
    pub fn new(capacity: usize) -> Result<Self, ServiceEvidenceOutboxError> {
        if capacity == 0 {
            return Err(ServiceEvidenceOutboxError::ZeroCapacity);
        }

        Ok(Self {
            capacity,
            state: Mutex::new(OutboxState::new(capacity)),
        })
    }

    pub(crate) fn publish<T>(
        &self,
        candidate: &T,
    ) -> Result<ServiceEvidencePublishOutcome, ServiceEvidenceOutboxError>
    where
        T: VerifiedServiceEvidence,
    {
        let draft = candidate.evidence_draft()?;
        validate_draft(&draft)?;

        let mut state = self.state.lock();

        if state.seen.contains(&draft.key) {
            return Ok(ServiceEvidencePublishOutcome::Duplicate {
                kind: draft.kind,
                proof_id: draft.proof_id,
            });
        }

        let sequence = state.next_sequence;
        state.next_sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or(ServiceEvidenceOutboxError::SequenceExhausted)?;

        if state.records.len() == self.capacity {
            if let Some((expired_key, _)) = state.records.pop_front() {
                state.seen.remove(&expired_key);
            }
        }

        let record = ServiceEvidenceRecordV1 {
            sequence,
            kind: draft.kind,
            proof_id: draft.proof_id,
            service_node_id: draft.service_node_id,
            witness_node_id: draft.witness_node_id,
            related_actor_ids: draft.related_actor_ids,
            content_id: draft.content_id,
            observed_at_ms: draft.observed_at_ms,
            signature_verified: true,
            evidence_only: true,
            accounting_accepted: false,
            reward_eligible: false,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,
        };

        state.seen.insert(draft.key.clone());
        state.records.push_back((draft.key, record.clone()));

        Ok(ServiceEvidencePublishOutcome::Published(Box::new(record)))
    }

    pub fn recent(
        &self,
        limit: usize,
    ) -> Result<Vec<ServiceEvidenceRecordV1>, ServiceEvidenceOutboxError> {
        if limit == 0 || limit > MAX_SERVICE_EVIDENCE_READ_ITEMS {
            return Err(ServiceEvidenceOutboxError::InvalidReadLimit {
                actual: limit,
                maximum: MAX_SERVICE_EVIDENCE_READ_ITEMS,
            });
        }

        let state = self.state.lock();

        Ok(state
            .records
            .iter()
            .rev()
            .take(limit)
            .map(|(_, record)| record.clone())
            .collect())
    }

    pub fn len(&self) -> usize {
        self.state.lock().records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ServiceEvidenceOutbox {
    fn default() -> Self {
        Self::new(DEFAULT_SERVICE_EVIDENCE_OUTBOX_CAPACITY)
            .expect("default evidence outbox capacity is nonzero")
    }
}

fn validate_draft(draft: &ServiceEvidenceDraftV1) -> Result<(), ServiceEvidenceOutboxError> {
    for (field, value) in [
        ("signature_verified", !draft.signature_verified),
        ("accounting_accepted", draft.accounting_accepted),
        ("reward_eligible", draft.reward_eligible),
        ("reward_truth", draft.reward_truth),
        ("payout_authority", draft.payout_authority),
        ("wallet_mutation", draft.wallet_mutation),
        ("ledger_mutation", draft.ledger_mutation),
    ] {
        if value {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind: draft.kind,
                field,
            });
        }
    }

    for (field, value) in &draft.unsupported_claims {
        if *value {
            return Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind: draft.kind,
                field,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestCandidate {
        key_suffix: u64,
        proof_id: String,
        signature_verified: bool,
        reward_truth: bool,
        wallet_mutation: bool,
    }

    impl TestCandidate {
        fn valid(key_suffix: u64) -> Self {
            Self {
                key_suffix,
                proof_id: format!("test:evidence:{key_suffix}"),
                signature_verified: true,
                reward_truth: false,
                wallet_mutation: false,
            }
        }
    }

    impl VerifiedServiceEvidence for TestCandidate {
        fn evidence_draft(&self) -> Result<ServiceEvidenceDraftV1, ServiceEvidenceOutboxError> {
            let content_id: ContentId =
                "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .parse()
                    .expect("canonical test content ID");

            Ok(ServiceEvidenceDraftV1 {
                key: ServiceEvidenceReplayKeyV1::Delivery(DeliveryProofReplayKeyV1 {
                    service_node_id: "service_node:alpha".to_owned(),
                    requester_node_id: "user_node:bravo".to_owned(),
                    request_id: format!("request:{}", self.key_suffix),
                    content_id: content_id.clone(),
                }),
                kind: ServiceEvidenceKindV1::Delivery,
                proof_id: self.proof_id.clone(),
                service_node_id: "service_node:alpha".to_owned(),
                witness_node_id: "user_node:bravo".to_owned(),
                related_actor_ids: Vec::new(),
                content_id,
                observed_at_ms: 1_900_000_000_000 + self.key_suffix,
                signature_verified: self.signature_verified,
                accounting_accepted: false,
                reward_eligible: false,
                reward_truth: self.reward_truth,
                payout_authority: false,
                wallet_mutation: self.wallet_mutation,
                ledger_mutation: false,
                unsupported_claims: Vec::new(),
            })
        }
    }

    fn assert_outbox_candidate<T>()
    where
        T: VerifiedServiceEvidence,
    {
    }

    #[test]
    fn all_seven_review_candidates_implement_outbox_contract() {
        assert_outbox_candidate::<DeliveryEvidenceCandidateV1>();

        assert_outbox_candidate::<AvailabilityEvidenceCandidateV1>();

        assert_outbox_candidate::<RangeRequestEvidenceCandidateV1>();

        assert_outbox_candidate::<RepairEvidenceCandidateV1>();

        assert_outbox_candidate::<HotCacheEvidenceCandidateV1>();

        assert_outbox_candidate::<PolicyRefusalEvidenceCandidateV1>();

        assert_outbox_candidate::<ModerationActionEvidenceCandidateV1>();
    }

    #[test]
    fn reviewed_candidate_becomes_evidence_not_reward_truth() {
        let outbox = ServiceEvidenceOutbox::new(4).expect("bounded outbox");

        let outcome = outbox
            .publish(&TestCandidate::valid(1))
            .expect("candidate publication");

        let ServiceEvidencePublishOutcome::Published(record) = outcome else {
            panic!("first publication must succeed");
        };

        assert_eq!(record.sequence, 1);
        assert_eq!(record.kind, ServiceEvidenceKindV1::Delivery);
        assert!(record.signature_verified);
        assert!(record.evidence_only);
        assert!(!record.accounting_accepted);
        assert!(!record.reward_eligible);
        assert!(!record.reward_truth);
        assert!(!record.payout_authority);
        assert!(!record.wallet_mutation);
        assert!(!record.ledger_mutation);
    }

    #[test]
    fn duplicate_replay_identity_is_rejected() {
        let outbox = ServiceEvidenceOutbox::new(4).expect("bounded outbox");

        let candidate = TestCandidate::valid(1);

        assert!(matches!(
            outbox.publish(&candidate),
            Ok(ServiceEvidencePublishOutcome::Published(_))
        ));

        assert_eq!(
            outbox.publish(&candidate).expect("duplicate outcome"),
            ServiceEvidencePublishOutcome::Duplicate {
                kind: ServiceEvidenceKindV1::Delivery,
                proof_id: "test:evidence:1".to_owned(),
            }
        );

        assert_eq!(outbox.len(), 1);
    }

    #[test]
    fn unverified_and_authority_poisoned_candidates_reject() {
        let outbox = ServiceEvidenceOutbox::new(4).expect("bounded outbox");

        let mut unsigned = TestCandidate::valid(1);
        unsigned.signature_verified = false;

        assert_eq!(
            outbox.publish(&unsigned),
            Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind: ServiceEvidenceKindV1::Delivery,
                field: "signature_verified",
            })
        );

        let mut reward = TestCandidate::valid(2);
        reward.reward_truth = true;

        assert_eq!(
            outbox.publish(&reward),
            Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind: ServiceEvidenceKindV1::Delivery,
                field: "reward_truth",
            })
        );

        let mut wallet = TestCandidate::valid(3);
        wallet.wallet_mutation = true;

        assert_eq!(
            outbox.publish(&wallet),
            Err(ServiceEvidenceOutboxError::InvalidCandidate {
                kind: ServiceEvidenceKindV1::Delivery,
                field: "wallet_mutation",
            })
        );

        assert!(outbox.is_empty());
    }

    #[test]
    fn bounded_outbox_evicts_oldest_and_lists_newest_first() {
        let outbox = ServiceEvidenceOutbox::new(2).expect("capacity-two outbox");

        for key in 1..=3 {
            assert!(matches!(
                outbox.publish(&TestCandidate::valid(key)),
                Ok(ServiceEvidencePublishOutcome::Published(_))
            ));
        }

        assert_eq!(outbox.len(), 2);

        let recent = outbox.recent(2).expect("recent evidence");

        assert_eq!(
            recent
                .iter()
                .map(|record| record.sequence)
                .collect::<Vec<_>>(),
            vec![3, 2]
        );

        // The oldest replay key left the bounded window.
        assert!(matches!(
            outbox.publish(&TestCandidate::valid(1)),
            Ok(ServiceEvidencePublishOutcome::Published(_))
        ));
    }

    #[test]
    fn record_wire_has_no_ip_or_economic_authority() {
        let outbox = ServiceEvidenceOutbox::new(2).expect("bounded outbox");

        let ServiceEvidencePublishOutcome::Published(record) = outbox
            .publish(&TestCandidate::valid(1))
            .expect("published record")
        else {
            panic!("expected published record");
        };

        let json = serde_json::to_string(&record).expect("record JSON");

        assert!(!json.contains("requester_ip"));
        assert!(!json.contains("provider_ip"));
        assert!(!json.contains("socket_addr"));
        assert!(!json.contains("crab://"));
        assert!(json.contains(r#""accounting_accepted":false"#));
        assert!(json.contains(r#""reward_truth":false"#));
        assert!(json.contains(r#""wallet_mutation":false"#));
        assert!(json.contains(r#""ledger_mutation":false"#));
    }

    #[test]
    fn published_record_projects_strict_accounting_input() {
        let outbox = ServiceEvidenceOutbox::new(2).expect("bounded outbox");

        let ServiceEvidencePublishOutcome::Published(record) = outbox
            .publish(&TestCandidate::valid(1))
            .expect("published evidence")
        else {
            panic!("expected published record");
        };

        let input = record.accounting_input().expect("strict accounting input");

        assert_eq!(input.kind, ServiceEvidenceAccountingKindV1::Delivery);
        assert_eq!(input.sequence, record.sequence);
        assert_eq!(input.proof_id, record.proof_id);
        assert!(input.signature_verified);
        assert!(input.evidence_only);

        assert!(!input.accounting_accepted);
        assert!(!input.reward_eligible);
        assert!(!input.reward_truth);
        assert!(!input.payout_authority);
        assert!(!input.wallet_mutation);
        assert!(!input.ledger_mutation);
    }

    #[test]
    fn capacity_and_read_limits_are_bounded() {
        assert!(matches!(
            ServiceEvidenceOutbox::new(0),
            Err(ServiceEvidenceOutboxError::ZeroCapacity)
        ));

        let outbox = ServiceEvidenceOutbox::new(2).expect("bounded outbox");

        assert_eq!(
            outbox.recent(0),
            Err(ServiceEvidenceOutboxError::InvalidReadLimit {
                actual: 0,
                maximum: MAX_SERVICE_EVIDENCE_READ_ITEMS,
            })
        );

        assert_eq!(
            outbox.recent(MAX_SERVICE_EVIDENCE_READ_ITEMS + 1),
            Err(ServiceEvidenceOutboxError::InvalidReadLimit {
                actual: MAX_SERVICE_EVIDENCE_READ_ITEMS + 1,
                maximum: MAX_SERVICE_EVIDENCE_READ_ITEMS,
            })
        );
    }
}
