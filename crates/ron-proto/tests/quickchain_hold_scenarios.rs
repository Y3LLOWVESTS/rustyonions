//! RO:WHAT — Proves locked concurrent-hold replay and closed-hold compaction scenario bytes.
//! RO:WHY — ECON/RES: retries, compaction, terminal evidence, and no-resurrection rules must be explicit before roots.
//! RO:INVARIANTS — four commits only; retries reuse receipts; open-only active set; terminal receipts retained; no hashes or mutation.
//! RO:TEST — hold_scenarios vectors plus independent Python verification.

use ron_proto::{
    from_canonical_json_slice, to_canonical_json_string, QuickChainConcurrentHoldReplayV1,
    QuickChainHoldCompactionV1, QuickChainHoldReplayStepOutcomeV1, QuickChainHoldStatusV1,
    QuickChainValidationError,
};
use serde::Deserialize;
use serde_json::{json, Value};

const REPLAY_VECTOR: &str =
    include_str!("vectors/quickchain/hold_scenarios/concurrent_holds_replay_locked_bytes_v1.json");

const COMPACTION_VECTOR: &str =
    include_str!("vectors/quickchain/hold_scenarios/closed_hold_compaction_locked_bytes_v1.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayLockedBytesVector {
    schema: String,
    version: u16,
    status: String,
    canonical_encoding: String,
    scenario: QuickChainConcurrentHoldReplayV1,
    canonical_payload_utf8: String,
    canonical_payload_hex: String,
    preimage_hex: Option<String>,
    expected_b3: Option<String>,
    notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompactionLockedBytesVector {
    schema: String,
    version: u16,
    status: String,
    canonical_encoding: String,
    scenario: QuickChainHoldCompactionV1,
    canonical_payload_utf8: String,
    canonical_payload_hex: String,
    preimage_hex: Option<String>,
    expected_b3: Option<String>,
    notes: Vec<String>,
}

fn load_replay() -> ReplayLockedBytesVector {
    serde_json::from_str(REPLAY_VECTOR).unwrap()
}

fn load_compaction() -> CompactionLockedBytesVector {
    serde_json::from_str(COMPACTION_VECTOR).unwrap()
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

#[test]
fn concurrent_hold_replay_matches_exact_locked_bytes() {
    let vector = load_replay();

    assert_eq!(vector.schema, "quickchain.concurrent-hold-replay-vector.v1");
    assert_eq!(vector.version, 1);
    assert_eq!(vector.status, "locked_bytes");
    assert_eq!(vector.canonical_encoding, "quickchain.canonical-json.v1");
    assert!(vector.preimage_hex.is_none());
    assert!(vector.expected_b3.is_none());
    assert!(!vector.notes.is_empty());
    assert!(vector.notes.iter().all(|note| !note.is_empty()));

    vector.scenario.validate().unwrap();

    let canonical = to_canonical_json_string(&vector.scenario).unwrap();

    assert_eq!(canonical, vector.canonical_payload_utf8);
    assert_eq!(
        lower_hex(canonical.as_bytes()),
        vector.canonical_payload_hex
    );

    let decoded: QuickChainConcurrentHoldReplayV1 =
        from_canonical_json_slice(canonical.as_bytes()).unwrap();

    assert_eq!(decoded, vector.scenario);
}

#[test]
fn replay_vector_freezes_retry_and_sequence_semantics() {
    let vector = load_replay();
    let scenario = vector.scenario;

    assert_eq!(scenario.scenario_id, "concurrent-holds-replay-001");
    assert_eq!(scenario.steps.len(), 6);

    let committed = scenario
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.expected_outcome,
                QuickChainHoldReplayStepOutcomeV1::Committed
            )
        })
        .count();

    let retries = scenario.steps.len() - committed;

    assert_eq!(committed, 4);
    assert_eq!(retries, 2);
    assert_eq!(scenario.expected_economic_commit_count, 4);
    assert_eq!(scenario.expected_state_transition_count, 4);

    assert_eq!(scenario.initial_account.total_minor, "1000");
    assert_eq!(scenario.initial_account.available_minor, "1000");
    assert_eq!(scenario.initial_account.held_minor, "0");
    assert_eq!(scenario.initial_account.account_sequence, 0);

    assert_eq!(scenario.expected_final_account.total_minor, "750");
    assert_eq!(scenario.expected_final_account.available_minor, "750");
    assert_eq!(scenario.expected_final_account.held_minor, "0");
    assert_eq!(scenario.expected_final_account.account_sequence, 4);

    assert!(scenario.expected_active_hold_ids.is_empty());
    assert_eq!(
        scenario.expected_terminal_receipt_txids,
        vec![
            "tx:roc:holds:0003".to_string(),
            "tx:roc:holds:0004".to_string()
        ]
    );

    assert_eq!(
        scenario.steps[2].receipt_txid,
        scenario.steps[3].receipt_txid
    );
    assert_eq!(
        scenario.steps[2].receipt_account_sequence,
        scenario.steps[3].receipt_account_sequence
    );
    assert_eq!(
        scenario.steps[4].receipt_txid,
        scenario.steps[5].receipt_txid
    );
    assert_eq!(
        scenario.steps[4].receipt_account_sequence,
        scenario.steps[5].receipt_account_sequence
    );
}

#[test]
fn replay_semantic_drift_is_rejected() {
    let mut wrong_receipt = load_replay().scenario;
    wrong_receipt.steps[3].receipt_txid = "tx:roc:holds:retry-copy".to_string();

    let error = wrong_receipt.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "steps[].receipt_txid",
            reason: "retry must return the original receipt txid"
        }
    ));

    let mut wrong_sequence = load_replay().scenario;
    wrong_sequence.steps[3].receipt_account_sequence = 4;

    let error = wrong_sequence.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "steps[].receipt_account_sequence",
            reason: "retry must retain the original receipt account sequence"
        }
    ));

    let mut contradictory_snapshot = load_replay().scenario;
    contradictory_snapshot.expected_final_account.held_minor = "1".to_string();

    let error = contradictory_snapshot.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "account_snapshot",
            reason: "total_minor must equal available_minor plus held_minor"
        }
    ));
}

#[test]
fn compaction_matches_locked_bytes_and_is_input_order_independent() {
    let vector = load_compaction();

    assert_eq!(vector.schema, "quickchain.hold-compaction-vector.v1");
    assert_eq!(vector.version, 1);
    assert_eq!(vector.status, "locked_bytes");
    assert_eq!(vector.canonical_encoding, "quickchain.canonical-json.v1");
    assert!(vector.preimage_hex.is_none());
    assert!(vector.expected_b3.is_none());
    assert!(!vector.notes.is_empty());
    assert!(vector.notes.iter().all(|note| !note.is_empty()));

    vector.scenario.validate().unwrap();

    let canonical = to_canonical_json_string(&vector.scenario).unwrap();

    assert_eq!(canonical, vector.canonical_payload_utf8);
    assert_eq!(
        lower_hex(canonical.as_bytes()),
        vector.canonical_payload_hex
    );

    let decoded: QuickChainHoldCompactionV1 =
        from_canonical_json_slice(canonical.as_bytes()).unwrap();

    assert_eq!(decoded, vector.scenario);

    let mut reversed = vector.scenario;
    reversed.unordered_holds.reverse();
    reversed.validate().unwrap();

    assert_eq!(
        reversed.expected_active_hold_ids,
        vec!["hold_11111111111111111111111111111111".to_string()]
    );
    assert_eq!(reversed.expected_compacted_terminal_count, 3);
}

#[test]
fn compaction_rejects_duplicates_missing_evidence_and_resurrection_drift() {
    let mut duplicate = load_compaction().scenario;
    duplicate
        .unordered_holds
        .push(duplicate.unordered_holds[0].clone());

    let error = duplicate.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "unordered_holds",
            reason: "duplicate hold_id is forbidden"
        }
    ));

    let mut missing_receipt = load_compaction().scenario;
    missing_receipt.terminal_receipts.pop();

    let error = missing_receipt.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "terminal_receipts",
            reason: "must contain exactly one receipt reference for every terminal hold"
        }
    ));

    let mut resurrection_drift = load_compaction().scenario;
    resurrection_drift
        .expected_rejected_resurrection_hold_ids
        .remove(0);

    let error = resurrection_drift.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "expected_rejected_resurrection_hold_ids",
            reason: "must contain exactly all terminal hold IDs"
        }
    ));

    assert!(matches!(
        load_compaction().scenario.unordered_holds[0].status,
        QuickChainHoldStatusV1::Released
    ));
}

#[test]
fn hold_scenario_vectors_reject_unknown_fields() {
    let mut replay_root: Value = serde_json::from_str(REPLAY_VECTOR).unwrap();
    replay_root
        .as_object_mut()
        .unwrap()
        .insert("database_order".to_string(), json!(true));

    serde_json::from_value::<ReplayLockedBytesVector>(replay_root)
        .expect_err("unknown replay vector fields must reject");

    let replay = load_replay();
    let mut replay_scenario: Value = serde_json::from_str(&replay.canonical_payload_utf8).unwrap();

    replay_scenario
        .as_object_mut()
        .unwrap()
        .insert("execute_transitions".to_string(), json!(true));

    serde_json::from_value::<QuickChainConcurrentHoldReplayV1>(replay_scenario)
        .expect_err("unknown replay scenario fields must reject");

    let mut compaction_root: Value = serde_json::from_str(COMPACTION_VECTOR).unwrap();
    compaction_root
        .as_object_mut()
        .unwrap()
        .insert("root_hash".to_string(), json!("b3:not-a-real-root"));

    serde_json::from_value::<CompactionLockedBytesVector>(compaction_root)
        .expect_err("unknown compaction vector fields must reject");
}
