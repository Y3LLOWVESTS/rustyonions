//! RO:WHAT — Proves locked replay/idempotency scenario bytes and Phase 0 retry semantics.
//! RO:WHY — ECON/RES: duplicate requests must not become duplicate economic commits or hold transitions.
//! RO:INVARIANTS — original receipt reuse; conflict rejection; one commit maximum; ledger-assigned sequence; no engine, hashes, or roots.
//! RO:TEST — replay_idempotency_locked_bytes_v1.json plus independent Python verification.

use std::collections::BTreeSet;

use ron_proto::{
    from_canonical_json_slice, to_canonical_json_string, QuickChainAccountSequenceSourceV1,
    QuickChainOperationClassV1, QuickChainReplayOutcomeV1, QuickChainReplayScenarioKindV1,
    QuickChainReplayScenarioV1, QuickChainValidationError,
};
use serde::Deserialize;
use serde_json::{json, Value};

const REPLAY_VECTOR_SET: &str =
    include_str!("vectors/quickchain/replay/replay_idempotency_locked_bytes_v1.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayVectorCase {
    scenario: QuickChainReplayScenarioV1,
    canonical_payload_utf8: String,
    canonical_payload_hex: String,
    preimage_hex: Option<String>,
    expected_b3: Option<String>,
    notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayVectorSet {
    schema: String,
    version: u16,
    status: String,
    canonical_encoding: String,
    cases: Vec<ReplayVectorCase>,
    notes: Vec<String>,
}

fn load_vectors() -> ReplayVectorSet {
    let vectors: ReplayVectorSet = serde_json::from_str(REPLAY_VECTOR_SET).unwrap();

    assert_eq!(vectors.schema, "quickchain.replay-scenario-vector-set.v1");
    assert_eq!(vectors.version, 1);
    assert_eq!(vectors.status, "locked_bytes");
    assert_eq!(vectors.canonical_encoding, "quickchain.canonical-json.v1");
    assert_eq!(vectors.cases.len(), 6);
    assert!(!vectors.notes.is_empty());
    assert!(vectors.notes.iter().all(|note| !note.is_empty()));

    vectors
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

fn case_by_id<'a>(vectors: &'a ReplayVectorSet, scenario_id: &str) -> &'a ReplayVectorCase {
    vectors
        .cases
        .iter()
        .find(|case| case.scenario.scenario_id == scenario_id)
        .unwrap_or_else(|| panic!("missing replay scenario {scenario_id}"))
}

#[test]
fn replay_scenarios_match_exact_locked_canonical_bytes() {
    let vectors = load_vectors();
    let mut scenario_ids = BTreeSet::new();

    for case in &vectors.cases {
        case.scenario.validate().unwrap();

        assert!(scenario_ids.insert(case.scenario.scenario_id.clone()));
        assert!(case.preimage_hex.is_none());
        assert!(case.expected_b3.is_none());
        assert!(!case.notes.is_empty());
        assert!(case.notes.iter().all(|note| !note.is_empty()));

        let canonical = to_canonical_json_string(&case.scenario).unwrap();
        assert_eq!(canonical, case.canonical_payload_utf8);
        assert_eq!(lower_hex(canonical.as_bytes()), case.canonical_payload_hex);

        let decoded: QuickChainReplayScenarioV1 =
            from_canonical_json_slice(case.canonical_payload_utf8.as_bytes()).unwrap();

        assert_eq!(decoded, case.scenario);
    }
}

#[test]
fn replay_vector_matrix_freezes_required_outcomes() {
    let vectors = load_vectors();

    let identical = case_by_id(&vectors, "identical-idempotent-retry-001");
    assert!(matches!(
        identical.scenario.scenario_kind,
        QuickChainReplayScenarioKindV1::IdenticalIdempotentRetry
    ));
    assert!(matches!(
        identical.scenario.expected_outcome,
        QuickChainReplayOutcomeV1::ReturnOriginalReceipt
    ));

    let conflict = case_by_id(&vectors, "conflicting-idempotency-reuse-001");
    assert!(matches!(
        conflict.scenario.scenario_kind,
        QuickChainReplayScenarioKindV1::ConflictingIdempotencyReuse
    ));
    assert!(matches!(
        conflict.scenario.expected_outcome,
        QuickChainReplayOutcomeV1::RejectIdempotencyConflict
    ));

    let duplicate = case_by_id(&vectors, "duplicate-operation-commit-001");
    assert!(matches!(
        duplicate.scenario.scenario_kind,
        QuickChainReplayScenarioKindV1::DuplicateOperationCommit
    ));
    assert!(matches!(
        duplicate.scenario.expected_outcome,
        QuickChainReplayOutcomeV1::RejectDuplicateOperationCommit
    ));

    let capture = case_by_id(&vectors, "retry-hold-capture-001");
    assert!(matches!(
        capture.scenario.submitted_intent.op_class,
        QuickChainOperationClassV1::HoldCapture
    ));
    assert_eq!(capture.scenario.expected_economic_commit_count, 1);
    assert_eq!(capture.scenario.expected_state_transition_count, 1);

    let release = case_by_id(&vectors, "retry-hold-release-001");
    assert!(matches!(
        release.scenario.submitted_intent.op_class,
        QuickChainOperationClassV1::HoldRelease
    ));
    assert_eq!(release.scenario.expected_economic_commit_count, 1);
    assert_eq!(release.scenario.expected_state_transition_count, 1);

    let sequence = case_by_id(&vectors, "ledger-assigned-account-sequence-001");
    assert!(matches!(
        sequence.scenario.scenario_kind,
        QuickChainReplayScenarioKindV1::LedgerAssignedAccountSequence
    ));
    assert!(matches!(
        sequence.scenario.expected_outcome,
        QuickChainReplayOutcomeV1::RejectClientAssignedAccountSequence
    ));
    assert_eq!(sequence.scenario.expected_economic_commit_count, 0);
    assert_eq!(sequence.scenario.expected_state_transition_count, 0);

    for case in &vectors.cases {
        assert!(matches!(
            case.scenario.account_sequence_source,
            QuickChainAccountSequenceSourceV1::LedgerAssigned
        ));
    }
}

#[test]
fn semantic_relationship_drift_is_rejected() {
    let vectors = load_vectors();

    let mut identical = case_by_id(&vectors, "identical-idempotent-retry-001")
        .scenario
        .clone();
    identical.submitted_intent.amount_minor = Some("11".to_string());

    let error = identical.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "submitted_intent",
            reason: "must exactly match original_intent for an identical retry"
        }
    ));

    let mut conflict = case_by_id(&vectors, "conflicting-idempotency-reuse-001")
        .scenario
        .clone();
    conflict.submitted_intent.idempotency_key = "different-retry-key".to_string();

    let error = conflict.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "submitted_intent.idempotency_key",
            reason: "must equal original idempotency_key for conflict-reuse scenarios"
        }
    ));

    let mut duplicate = case_by_id(&vectors, "duplicate-operation-commit-001")
        .scenario
        .clone();
    duplicate.submitted_intent.operation_id = "op_c2222222222222222222222222222222".to_string();

    let error = duplicate.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "submitted_intent.operation_id",
            reason: "must equal original operation_id for duplicate-commit scenarios"
        }
    ));

    let mut capture = case_by_id(&vectors, "retry-hold-capture-001")
        .scenario
        .clone();
    capture.scenario_kind = QuickChainReplayScenarioKindV1::RetryHoldRelease;

    let error = capture.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "submitted_intent.op_class",
            reason: "does not match the replay scenario operation class"
        }
    ));

    let mut sequence = case_by_id(&vectors, "ledger-assigned-account-sequence-001")
        .scenario
        .clone();
    sequence.attempted_client_account_sequence = None;

    let error = sequence.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "attempted_client_account_sequence",
            reason: "required for the client-assigned sequence rejection scenario"
        }
    ));
}

#[test]
fn client_assigned_account_sequence_is_rejected_by_operation_intent() {
    let vectors = load_vectors();

    let mut sequence = case_by_id(&vectors, "ledger-assigned-account-sequence-001")
        .scenario
        .clone();

    sequence.submitted_intent.account_sequence = Some(9);

    let error = sequence.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "account_sequence",
            reason: "must be absent because the ledger assigns it after acceptance"
        }
    ));
}

#[test]
fn replay_vector_and_scenario_reject_unknown_fields() {
    let mut root: Value = serde_json::from_str(REPLAY_VECTOR_SET).unwrap();

    root.as_object_mut()
        .unwrap()
        .insert("database_order".to_string(), json!(true));

    serde_json::from_value::<ReplayVectorSet>(root)
        .expect_err("unknown vector-set fields must reject");

    let vectors = load_vectors();
    let canonical = to_canonical_json_string(&vectors.cases[0].scenario).unwrap();

    let mut scenario: Value = serde_json::from_str(&canonical).unwrap();

    scenario
        .as_object_mut()
        .unwrap()
        .insert("second_commit_allowed".to_string(), json!(true));

    serde_json::from_value::<QuickChainReplayScenarioV1>(scenario)
        .expect_err("unknown replay scenario fields must reject");
}
