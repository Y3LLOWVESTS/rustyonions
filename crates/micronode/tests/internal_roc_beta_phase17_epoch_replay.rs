//! RO:WHAT — Phase 17 user-node epoch replay, fraud findings, and challenge tests.
//! RO:WHY — Prove ordinary user nodes can independently accept a valid epoch
//! or identify replay, cap, root, quorum, and supply failures.
//! RO:INVARIANTS — review is deterministic and read-only; no wallet mutation,
//! ledger mutation, confirmed ROC, finality, or fake challenge submission.
//! RO:TEST — cargo test -p micronode --test internal_roc_beta_phase17_epoch_replay.

use std::{collections::BTreeMap, sync::Mutex};

use micronode::challenge_outbox::{
    InvalidEpochChallengeEnqueueOutcome, InvalidEpochChallengeOutbox,
    InvalidEpochChallengeOutboxError, InvalidEpochChallengeSubmissionAckV1,
    InvalidEpochChallengeSubmissionSink, InvalidEpochChallengeSubmissionStateV1,
    InvalidEpochChallengeSubmitOutcome,
};
use micronode::economic_audit::{
    build_invalid_epoch_challenge, review_epoch_transition,
    review_epoch_transition_with_signatures, EpochAuditFindingKindV1, EpochChallengeBuildError,
    EpochQuorumKeyResolver, EpochReplayObservationV1, UserNodeEpochReviewStatusV1,
    UserNodeEpochReviewV1, USER_NODE_EPOCH_REPLAY_OBSERVATION_SCHEMA,
    USER_NODE_EPOCH_REPLAY_VERSION,
};
use ron_kms::{backends::memory::MemoryKeystore, KeyId, Keystore, Signer};
use ron_proto::{
    service_node_signature_message_bytes, ContentId, EpochEligibilityStatusV1, EpochEligibilityV1,
    EpochQuorumThresholdV1, EpochRewardAllocationV1, InvalidEpochChallengeKindV1,
    InvalidEpochChallengeV1, RocEpochTransitionExpectationV1, RocEpochTransitionV1,
    ServiceNodeQuorumV1, ServiceNodeSignatureV1, SignatureAlg, EPOCH_REWARD_ALLOCATION_SCHEMA,
    INVALID_EPOCH_CHALLENGE_SCHEMA, ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA,
    ROC_EPOCH_TRANSITION_SCHEMA, ROC_EPOCH_TRANSITION_VERSION,
};

const ALPHA_NODE: &str = "service_node:alpha";
const BETA_NODE: &str = "service_node:beta";
const GAMMA_NODE: &str = "service_node:gamma";

struct TestKeyResolver {
    keys: BTreeMap<String, KeyId>,
}

impl EpochQuorumKeyResolver for TestKeyResolver {
    fn resolve_key(&self, key_ref: &str) -> Option<KeyId> {
        self.keys.get(key_ref).cloned()
    }
}

fn cid(character: char) -> ContentId {
    ContentId::parse(&format!("b3:{}", character.to_string().repeat(64)))
        .expect("fixture content id must parse")
}

fn eligibility(
    service_node_id: &str,
    registry_entry_id: &str,
    reward_binding_id: &str,
    key_id: &str,
) -> EpochEligibilityV1 {
    EpochEligibilityV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,
        service_node_id: service_node_id.to_owned(),
        registry_entry_id: registry_entry_id.to_owned(),
        reward_binding_id: reward_binding_id.to_owned(),
        key_id: key_id.to_owned(),
        status: EpochEligibilityStatusV1::Eligible,
    }
}

fn signature(
    service_node_id: &str,
    key_id: &str,
    transition_hash: &ContentId,
    hex_byte: &str,
) -> ServiceNodeSignatureV1 {
    ServiceNodeSignatureV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:17".to_owned(),
        service_node_id: service_node_id.to_owned(),
        key_id: key_id.to_owned(),
        algorithm: SignatureAlg::Ed25519,
        transition_hash: transition_hash.clone(),
        signature_wire: hex_byte.repeat(64),
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

fn fixture() -> (RocEpochTransitionV1, RocEpochTransitionExpectationV1, EpochReplayObservationV1) {
    let transition_hash = cid('8');

    let eligibilities = vec![
        eligibility(ALPHA_NODE, "registry:alpha", "binding:alpha", "key:alpha"),
        eligibility(BETA_NODE, "registry:beta", "binding:beta", "key:beta"),
        eligibility(GAMMA_NODE, "registry:gamma", "binding:gamma", "key:gamma"),
    ];

    let threshold = EpochQuorumThresholdV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,
        eligible_service_nodes: 3,
        quorum_bps: 6_666,
        minimum_signatures: 2,
        required_signatures: 2,
    };

    let quorum = ServiceNodeQuorumV1 {
        schema: "ron.service_node.quorum.v1".to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:17".to_owned(),
        transition_hash: transition_hash.clone(),
        threshold: threshold.clone(),
        eligibilities: eligibilities.clone(),
        signatures: vec![
            signature(ALPHA_NODE, "key:alpha", &transition_hash, "11"),
            signature(BETA_NODE, "key:beta", &transition_hash, "22"),
        ],
    };

    let transition = RocEpochTransitionV1 {
        schema: ROC_EPOCH_TRANSITION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:17".to_owned(),
        transition_hash,
        accounting_snapshot_hash: cid('1'),
        reward_plan_hash: cid('2'),
        policy_hash: cid('3'),
        economics_config_hash: cid('4'),
        registry_root: cid('5'),
        reward_binding_root: cid('6'),
        evidence_root: cid('7'),
        reward_cap_minor_units: "1000".to_owned(),
        reward_total_minor_units: "1000".to_owned(),
        allocations: vec![
            allocation("allocation:alpha", "reward_plan_allocation:alpha", ALPHA_NODE, "400"),
            allocation("allocation:beta", "reward_plan_allocation:beta", BETA_NODE, "600"),
        ],
        quorum,
        produced_at_ms: 1_789_000_000_000,
    };

    let expected = RocEpochTransitionExpectationV1 {
        schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: transition.chain_id.clone(),
        epoch_id: transition.epoch_id.clone(),
        accounting_snapshot_hash: transition.accounting_snapshot_hash.clone(),
        reward_plan_hash: transition.reward_plan_hash.clone(),
        policy_hash: transition.policy_hash.clone(),
        economics_config_hash: transition.economics_config_hash.clone(),
        registry_root: transition.registry_root.clone(),
        reward_binding_root: transition.reward_binding_root.clone(),
        evidence_root: transition.evidence_root.clone(),
        reward_cap_minor_units: transition.reward_cap_minor_units.clone(),
        threshold,
        eligibilities,
    };

    let observation = EpochReplayObservationV1 {
        schema: USER_NODE_EPOCH_REPLAY_OBSERVATION_SCHEMA.to_owned(),
        version: USER_NODE_EPOCH_REPLAY_VERSION,
        accounting_snapshot_hash: expected.accounting_snapshot_hash.clone(),
        reward_plan_hash: expected.reward_plan_hash.clone(),
        policy_hash: expected.policy_hash.clone(),
        economics_config_hash: expected.economics_config_hash.clone(),
        registry_root: expected.registry_root.clone(),
        reward_binding_root: expected.reward_binding_root.clone(),
        applied_allocation_ids: vec!["allocation:alpha".to_owned(), "allocation:beta".to_owned()],
        replayed_reward_total_minor_units: "1000".to_owned(),
        supply_before_minor_units: "5000".to_owned(),
        supply_after_minor_units: "6000".to_owned(),
    };

    (transition, expected, observation)
}

fn signed_fixture() -> (
    MemoryKeystore,
    TestKeyResolver,
    RocEpochTransitionV1,
    RocEpochTransitionExpectationV1,
    EpochReplayObservationV1,
) {
    let (mut transition, expected, observation) = fixture();
    let kms = ron_kms::memory_keystore();

    let alpha_key = kms.create_ed25519("service-node", "phase17-alpha").expect("alpha key");
    let beta_key = kms.create_ed25519("service-node", "phase17-beta").expect("beta key");

    let resolver = TestKeyResolver {
        keys: BTreeMap::from([
            ("key:alpha".to_owned(), alpha_key),
            ("key:beta".to_owned(), beta_key),
        ]),
    };

    for signature in &mut transition.quorum.signatures {
        let key = resolver.keys.get(&signature.key_id).expect("fixture signature key must resolve");

        let message = service_node_signature_message_bytes(signature).expect("signature message");

        signature.signature_wire = hex::encode(kms.sign(key, &message).expect("fixture signature"));
    }

    (kms, resolver, transition, expected, observation)
}

fn has_finding(review: &UserNodeEpochReviewV1, kind: EpochAuditFindingKindV1) -> bool {
    review.findings().iter().any(|finding| finding.kind == kind)
}

#[test]
fn user_node_accepts_valid_structural_epoch_replay() {
    let (transition, expected, observation) = fixture();

    let review = review_epoch_transition(&transition, &expected, &observation);

    assert_eq!(review.status(), UserNodeEpochReviewStatusV1::Accepted);
    assert!(review.is_accepted());
    assert!(review.findings().is_empty());
    assert_eq!(review.transition_hash(), &transition.transition_hash);
    assert_eq!(review.replayed_reward_total_minor_units(), "1000");
    assert!(!review.wallet_mutation());
    assert!(!review.ledger_mutation());
    assert!(!review.challenge_submitted());
}

#[test]
fn user_node_detects_duplicate_payout_in_replay() {
    let (transition, expected, mut observation) = fixture();

    observation.applied_allocation_ids =
        vec!["allocation:alpha".to_owned(), "allocation:alpha".to_owned()];

    let review = review_epoch_transition(&transition, &expected, &observation);

    assert!(!review.is_accepted());
    assert!(has_finding(&review, EpochAuditFindingKindV1::DuplicatePayout));
    assert!(has_finding(&review, EpochAuditFindingKindV1::ReplayAllocationSetMismatch));
    assert!(!review.challenge_submitted());
}

#[test]
fn user_node_detects_supply_conservation_mismatch() {
    let (transition, expected, mut observation) = fixture();

    observation.supply_after_minor_units = "5999".to_owned();

    let review = review_epoch_transition(&transition, &expected, &observation);

    assert!(has_finding(&review, EpochAuditFindingKindV1::SupplyConservationMismatch));
    assert!(!review.wallet_mutation());
    assert!(!review.ledger_mutation());
}

#[test]
fn user_node_detects_economics_configuration_hash_mismatch() {
    let (transition, expected, mut observation) = fixture();

    observation.economics_config_hash = cid('9');

    let review = review_epoch_transition(&transition, &expected, &observation);

    assert!(has_finding(&review, EpochAuditFindingKindV1::EconomicsConfigHashMismatch));
}

#[test]
fn user_node_detects_invalid_or_noneligible_quorum_signer() {
    let (mut transition, expected, observation) = fixture();

    transition.quorum.signatures[1].service_node_id = "service_node:outsider".to_owned();

    let review = review_epoch_transition(&transition, &expected, &observation);

    assert!(has_finding(&review, EpochAuditFindingKindV1::InvalidTransition));
    assert!(!review.challenge_submitted());
}

#[test]
fn user_node_detects_cap_and_replay_total_mismatch() {
    let (transition, expected, mut observation) = fixture();

    observation.replayed_reward_total_minor_units = "1001".to_owned();
    observation.supply_after_minor_units = "6001".to_owned();

    let review = review_epoch_transition(&transition, &expected, &observation);

    assert!(has_finding(&review, EpochAuditFindingKindV1::RewardCapExceeded));
    assert!(has_finding(&review, EpochAuditFindingKindV1::ReplayTotalMismatch));
}

#[test]
fn rejected_review_builds_canonical_invalid_epoch_challenge() {
    let (transition, expected, mut observation) = fixture();

    observation.policy_hash = cid('9');

    let review = review_epoch_transition(&transition, &expected, &observation);

    let challenge =
        build_invalid_epoch_challenge(&transition, &review, "user_node:auditor", 1_789_000_000_001)
            .expect("rejected review must build canonical challenge evidence");

    challenge.validate().expect("constructed challenge must satisfy ron-proto");

    assert_eq!(challenge.schema, INVALID_EPOCH_CHALLENGE_SCHEMA);
    assert_eq!(challenge.chain_id, transition.chain_id);
    assert_eq!(challenge.epoch_id, transition.epoch_id);
    assert_eq!(challenge.transition_hash, transition.transition_hash);
    assert_eq!(challenge.challenger_id, "user_node:auditor");
    assert_eq!(challenge.challenge_kind, InvalidEpochChallengeKindV1::InvalidPolicyHash);
    assert!(challenge.challenge_id.starts_with("challenge:epoch:17:"));
}

#[test]
fn identical_invalid_review_builds_deterministic_evidence_identity() {
    let (transition, expected, mut observation) = fixture();

    observation.reward_binding_root = cid('9');

    let review = review_epoch_transition(&transition, &expected, &observation);

    let first =
        build_invalid_epoch_challenge(&transition, &review, "user_node:auditor", 1_789_000_000_001)
            .expect("first challenge must build");

    let second =
        build_invalid_epoch_challenge(&transition, &review, "user_node:auditor", 1_789_000_000_999)
            .expect("second challenge must build");

    assert_eq!(first.challenge_id, second.challenge_id);
    assert_eq!(first.evidence_hash, second.evidence_hash);
    assert_eq!(first.challenge_kind, second.challenge_kind);
    assert_ne!(first.submitted_at_ms, second.submitted_at_ms);
}

#[test]
fn accepted_review_cannot_fabricate_invalid_epoch_challenge() {
    let (transition, expected, observation) = fixture();

    let review = review_epoch_transition(&transition, &expected, &observation);

    assert_eq!(
        build_invalid_epoch_challenge(&transition, &review, "user_node:auditor", 1_789_000_000_001,),
        Err(EpochChallengeBuildError::AcceptedTransition)
    );
}

#[test]
fn multi_finding_challenge_uses_stable_primary_precedence() {
    let (transition, expected, mut observation) = fixture();

    observation.policy_hash = cid('9');
    observation.applied_allocation_ids =
        vec!["allocation:alpha".to_owned(), "allocation:alpha".to_owned()];

    let review = review_epoch_transition(&transition, &expected, &observation);

    let challenge =
        build_invalid_epoch_challenge(&transition, &review, "user_node:auditor", 1_789_000_000_001)
            .expect("multi-finding review must build a challenge");

    assert_eq!(challenge.challenge_kind, InvalidEpochChallengeKindV1::InvalidPolicyHash);
}

#[test]
fn user_node_accepts_real_ed25519_quorum_signatures() {
    let (kms, resolver, transition, expected, observation) = signed_fixture();

    let review = review_epoch_transition_with_signatures(
        &transition,
        &expected,
        &observation,
        &kms,
        &resolver,
    );

    assert!(review.is_accepted());
    assert!(review.findings().is_empty());
    assert!(!review.wallet_mutation());
    assert!(!review.ledger_mutation());
}

#[test]
fn tampered_ed25519_signature_requires_canonical_challenge() {
    let (kms, resolver, mut transition, expected, observation) = signed_fixture();

    let signature = &mut transition.quorum.signatures[0];
    let replacement = if signature.signature_wire.starts_with("00") { "ff" } else { "00" };
    signature.signature_wire.replace_range(0..2, replacement);

    let review = review_epoch_transition_with_signatures(
        &transition,
        &expected,
        &observation,
        &kms,
        &resolver,
    );

    assert!(has_finding(&review, EpochAuditFindingKindV1::InvalidServiceNodeSignature));

    let challenge =
        build_invalid_epoch_challenge(&transition, &review, "user_node:auditor", 1_789_000_000_002)
            .expect("invalid signature must create challenge evidence");

    assert_eq!(challenge.challenge_kind, InvalidEpochChallengeKindV1::InvalidServiceNodeSignature);
}

#[test]
fn unresolved_quorum_key_requires_signature_challenge() {
    let (kms, mut resolver, transition, expected, observation) = signed_fixture();

    resolver.keys.remove("key:beta");

    let review = review_epoch_transition_with_signatures(
        &transition,
        &expected,
        &observation,
        &kms,
        &resolver,
    );

    assert!(has_finding(&review, EpochAuditFindingKindV1::InvalidServiceNodeSignature));

    let challenge =
        build_invalid_epoch_challenge(&transition, &review, "user_node:auditor", 1_789_000_000_003)
            .expect("unresolved key must create challenge evidence");

    assert_eq!(challenge.challenge_kind, InvalidEpochChallengeKindV1::InvalidServiceNodeSignature);
}

struct RecordingChallengeSink {
    fail: bool,
    received: Mutex<Vec<InvalidEpochChallengeV1>>,
}

impl RecordingChallengeSink {
    fn succeeding() -> Self {
        Self { fail: false, received: Mutex::new(Vec::new()) }
    }

    fn failing() -> Self {
        Self { fail: true, received: Mutex::new(Vec::new()) }
    }
}

impl InvalidEpochChallengeSubmissionSink for RecordingChallengeSink {
    type Error = &'static str;

    fn submit(
        &self,
        challenge: &InvalidEpochChallengeV1,
    ) -> Result<InvalidEpochChallengeSubmissionAckV1, Self::Error> {
        if self.fail {
            return Err("deterministic test transport failure");
        }

        self.received.lock().expect("recording sink mutex").push(challenge.clone());

        Ok(InvalidEpochChallengeSubmissionAckV1 {
            submission_ref: format!("transport:{}", challenge.challenge_id),
            acknowledged_at_ms: challenge.submitted_at_ms + 1,
        })
    }
}

fn policy_challenge(challenger_id: &str, submitted_at_ms: u64) -> InvalidEpochChallengeV1 {
    let (transition, expected, mut observation) = fixture();
    observation.policy_hash = cid('9');

    let review = review_epoch_transition(&transition, &expected, &observation);

    build_invalid_epoch_challenge(&transition, &review, challenger_id, submitted_at_ms)
        .expect("policy mismatch challenge")
}

#[test]
fn rejected_review_queues_truthful_challenge_submission_record() {
    let challenge = policy_challenge("user_node:auditor", 1_789_000_000_100);
    let outbox = InvalidEpochChallengeOutbox::new(2).expect("bounded outbox");

    let outcome = outbox.enqueue(challenge.clone()).expect("canonical challenge must queue");

    let InvalidEpochChallengeEnqueueOutcome::Queued(record) = outcome else {
        panic!("first canonical challenge must be queued");
    };

    assert_eq!(record.sequence, 1);
    assert_eq!(record.challenge, challenge);
    assert_eq!(record.submission_state, InvalidEpochChallengeSubmissionStateV1::Queued);
    assert_eq!(record.submission_attempts, 0);
    assert!(record.evidence_only);
    assert!(!record.challenge_accepted);
    assert!(!record.finality_claimed);
    assert!(!record.wallet_mutation);
    assert!(!record.ledger_mutation);
    assert_eq!(outbox.pending_len(), 1);
}

#[test]
fn deterministic_challenge_duplicate_is_not_queued_twice() {
    let challenge = policy_challenge("user_node:auditor", 1_789_000_000_101);
    let outbox = InvalidEpochChallengeOutbox::new(2).expect("bounded outbox");

    outbox.enqueue(challenge.clone()).expect("first queue");

    let duplicate = outbox.enqueue(challenge.clone()).expect("duplicate outcome");

    assert_eq!(
        duplicate,
        InvalidEpochChallengeEnqueueOutcome::Duplicate { challenge_id: challenge.challenge_id }
    );
    assert_eq!(outbox.len(), 1);
}

#[test]
fn successful_transport_handoff_does_not_claim_challenge_acceptance() {
    let challenge = policy_challenge("user_node:auditor", 1_789_000_000_102);
    let outbox = InvalidEpochChallengeOutbox::new(2).expect("bounded outbox");
    let sink = RecordingChallengeSink::succeeding();

    outbox.enqueue(challenge.clone()).expect("queue challenge");

    let outcome = outbox.submit_next(&sink).expect("transport handoff");

    let InvalidEpochChallengeSubmitOutcome::Submitted(record) = outcome else {
        panic!("queued challenge must be submitted");
    };

    assert_eq!(sink.received.lock().expect("recording sink mutex").as_slice(), &[challenge]);
    assert_eq!(record.submission_state, InvalidEpochChallengeSubmissionStateV1::Submitted);
    assert_eq!(record.submission_attempts, 1);
    assert!(record.submission_ref.is_some());
    assert!(record.acknowledged_at_ms.is_some());
    assert!(record.evidence_only);
    assert!(!record.challenge_accepted);
    assert!(!record.finality_claimed);
    assert!(!record.wallet_mutation);
    assert!(!record.ledger_mutation);
    assert_eq!(outbox.pending_len(), 0);
}

#[test]
fn failed_transport_handoff_remains_queued_for_retry() {
    let challenge = policy_challenge("user_node:auditor", 1_789_000_000_103);
    let outbox = InvalidEpochChallengeOutbox::new(2).expect("bounded outbox");
    let sink = RecordingChallengeSink::failing();

    outbox.enqueue(challenge).expect("queue challenge");

    assert_eq!(
        outbox.submit_next(&sink),
        Err(InvalidEpochChallengeOutboxError::Sink(
            "deterministic test transport failure".to_owned()
        ))
    );

    let records = outbox.recent(1).expect("recent records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].submission_state, InvalidEpochChallengeSubmissionStateV1::Queued);
    assert_eq!(records[0].submission_attempts, 1);
    assert!(records[0].submission_ref.is_none());
    assert!(records[0].acknowledged_at_ms.is_none());
    assert_eq!(outbox.pending_len(), 1);
}

#[test]
fn pending_challenge_is_never_silently_evicted_at_capacity() {
    let first = policy_challenge("user_node:auditor_one", 1_789_000_000_104);
    let second = policy_challenge("user_node:auditor_two", 1_789_000_000_105);
    let outbox = InvalidEpochChallengeOutbox::new(1).expect("bounded outbox");

    outbox.enqueue(first).expect("first challenge");

    assert_eq!(
        outbox.enqueue(second.clone()),
        Err(InvalidEpochChallengeOutboxError::OutboxFull { capacity: 1 })
    );

    let sink = RecordingChallengeSink::succeeding();
    outbox.submit_next(&sink).expect("submit first challenge");

    let replacement = outbox.enqueue(second.clone()).expect("submitted history may be evicted");

    let InvalidEpochChallengeEnqueueOutcome::Queued(record) = replacement else {
        panic!("second distinct challenge must queue");
    };

    assert_eq!(record.challenge, second);
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox.pending_len(), 1);
}
