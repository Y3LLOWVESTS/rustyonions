//! RO:WHAT — Loads checked-in QuickChain locked-byte vectors and proves typed canonical-byte parity.
//! RO:WHY — ECON/GOV: reviewed bytes must live in human-readable files before any hash or root code exists.
//! RO:INTERACTS — tests/vectors/quickchain/*.json, quickchain canonical helpers, strict DTO validators.
//! RO:INVARIANTS — explicit file list; locked_bytes only; no hashing; no roots; no placeholder expected hashes.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — canonical payload fields are byte authority; explanatory JSON must exactly represent the typed DTO.
//! RO:TEST — this integration test is the file-backed QC-0A locked-byte corpus gate.

use std::{collections::BTreeSet, fmt::Debug};

use ron_proto::{
    from_canonical_json_slice, to_canonical_json_string, QuickChainAccountStateV1,
    QuickChainChainParamsV1, QuickChainCheckpointHeaderV1, QuickChainEmptyTreeV1,
    QuickChainHoldStateV1, QuickChainOperationClassV1, QuickChainOperationIntentV1,
    QuickChainReceiptStatusV1, QuickChainReceiptV1, QuickChainTestVectorV1,
    QuickChainVectorCanonicalEncodingV1, QuickChainVectorHashAlgorithmV1,
    QuickChainVectorPreimageFramingV1, QuickChainVectorStatusV1,
    QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1,
    QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1, QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
    QUICKCHAIN_DTO_VERSION, QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1, QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_TEST_VECTOR_SCHEMA,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

const ACCOUNT_STATE_VECTOR: &str =
    include_str!("vectors/quickchain/account_state_locked_bytes_v1.json");
const OPERATION_INTENT_VECTOR: &str =
    include_str!("vectors/quickchain/operation_intent_locked_bytes_v1.json");
const ISSUE_OPERATION_INTENT_VECTOR: &str =
    include_str!("vectors/quickchain/issue_operation_intent_locked_bytes_v1.json");
const TRANSFER_OPERATION_INTENT_VECTOR: &str =
    include_str!("vectors/quickchain/transfer_operation_intent_locked_bytes_v1.json");
const BURN_OPERATION_INTENT_VECTOR: &str =
    include_str!("vectors/quickchain/burn_operation_intent_locked_bytes_v1.json");
const HOLD_CAPTURE_OPERATION_INTENT_VECTOR: &str =
    include_str!("vectors/quickchain/hold_capture_operation_intent_locked_bytes_v1.json");
const HOLD_RELEASE_OPERATION_INTENT_VECTOR: &str =
    include_str!("vectors/quickchain/hold_release_operation_intent_locked_bytes_v1.json");
const HOLD_EXPIRE_OPERATION_INTENT_VECTOR: &str =
    include_str!("vectors/quickchain/hold_expire_operation_intent_locked_bytes_v1.json");
const OPEN_HOLD_STATE_VECTOR: &str =
    include_str!("vectors/quickchain/open_hold_state_locked_bytes_v1.json");
const ACCEPTED_RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/accepted_receipt_locked_bytes_v1.json");
const ACCEPTED_ISSUE_RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/accepted_issue_receipt_locked_bytes_v1.json");
const ACCEPTED_BURN_RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/accepted_burn_receipt_locked_bytes_v1.json");
const ACCEPTED_HOLD_OPEN_RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/accepted_hold_open_receipt_locked_bytes_v1.json");
const ACCEPTED_HOLD_CAPTURE_RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/accepted_hold_capture_receipt_locked_bytes_v1.json");
const ACCEPTED_HOLD_RELEASE_RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/accepted_hold_release_receipt_locked_bytes_v1.json");
const ACCEPTED_HOLD_EXPIRE_RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/accepted_hold_expire_receipt_locked_bytes_v1.json");
const EMPTY_STATE_TREE_VECTOR: &str =
    include_str!("vectors/quickchain/empty_state_tree_locked_bytes_v1.json");
const EMPTY_HOLDS_TREE_VECTOR: &str =
    include_str!("vectors/quickchain/empty_holds_tree_locked_bytes_v1.json");
const EMPTY_RECEIPTS_TREE_VECTOR: &str =
    include_str!("vectors/quickchain/empty_receipts_tree_locked_bytes_v1.json");
const EMPTY_ACCOUNTING_TREE_VECTOR: &str =
    include_str!("vectors/quickchain/empty_accounting_tree_locked_bytes_v1.json");
const EMPTY_REWARDS_TREE_VECTOR: &str =
    include_str!("vectors/quickchain/empty_rewards_tree_locked_bytes_v1.json");
const CHECKPOINT_HEADER_VECTOR: &str =
    include_str!("vectors/quickchain/checkpoint_header_locked_bytes_v1.json");
const CAPTURED_HOLD_STATE_VECTOR: &str =
    include_str!("vectors/quickchain/captured_hold_state_locked_bytes_v1.json");
const RELEASED_HOLD_STATE_VECTOR: &str =
    include_str!("vectors/quickchain/released_hold_state_locked_bytes_v1.json");
const EXPIRED_HOLD_STATE_VECTOR: &str =
    include_str!("vectors/quickchain/expired_hold_state_locked_bytes_v1.json");
const DISABLED_CHAIN_PARAMS_VECTOR: &str =
    include_str!("vectors/quickchain/disabled_chain_params_locked_bytes_v1.json");

#[derive(Clone, Copy)]
struct VectorCase {
    raw: &'static str,
    vector_id: &'static str,
    purpose: &'static str,
    domain_separator: &'static str,
}

const VECTOR_CASES: [VectorCase; 26] = [
    VectorCase {
        raw: ACCOUNT_STATE_VECTOR,
        vector_id: "canonical_account_state_vector_001",
        purpose: "account_state_canonical_bytes",
        domain_separator: QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: OPERATION_INTENT_VECTOR,
        vector_id: "canonical_operation_intent_vector_001",
        purpose: "operation_intent_canonical_bytes",
        domain_separator: QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: OPEN_HOLD_STATE_VECTOR,
        vector_id: "canonical_open_hold_state_vector_001",
        purpose: "hold_state_canonical_bytes",
        domain_separator: QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ACCEPTED_RECEIPT_VECTOR,
        vector_id: "canonical_accepted_receipt_vector_001",
        purpose: "receipt_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: EMPTY_STATE_TREE_VECTOR,
        vector_id: "canonical_empty_tree_state_vector_001",
        purpose: "empty_tree_canonical_bytes",
        domain_separator: QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: CHECKPOINT_HEADER_VECTOR,
        vector_id: "canonical_checkpoint_header_vector_001",
        purpose: "checkpoint_header_canonical_bytes",
        domain_separator: QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: CAPTURED_HOLD_STATE_VECTOR,
        vector_id: "canonical_captured_hold_state_vector_001",
        purpose: "captured_hold_state_canonical_bytes",
        domain_separator: QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: RELEASED_HOLD_STATE_VECTOR,
        vector_id: "canonical_released_hold_state_vector_001",
        purpose: "released_hold_state_canonical_bytes",
        domain_separator: QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: EXPIRED_HOLD_STATE_VECTOR,
        vector_id: "canonical_expired_hold_state_vector_001",
        purpose: "expired_hold_state_canonical_bytes",
        domain_separator: QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: DISABLED_CHAIN_PARAMS_VECTOR,
        vector_id: "canonical_disabled_chain_params_vector_001",
        purpose: "chain_params_canonical_bytes",
        domain_separator: QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ACCEPTED_ISSUE_RECEIPT_VECTOR,
        vector_id: "canonical_accepted_issue_receipt_vector_001",
        purpose: "issue_receipt_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ACCEPTED_BURN_RECEIPT_VECTOR,
        vector_id: "canonical_accepted_burn_receipt_vector_001",
        purpose: "burn_receipt_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ACCEPTED_HOLD_OPEN_RECEIPT_VECTOR,
        vector_id: "canonical_accepted_hold_open_receipt_vector_001",
        purpose: "hold_open_receipt_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ACCEPTED_HOLD_CAPTURE_RECEIPT_VECTOR,
        vector_id: "canonical_accepted_hold_capture_receipt_vector_001",
        purpose: "hold_capture_receipt_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ACCEPTED_HOLD_RELEASE_RECEIPT_VECTOR,
        vector_id: "canonical_accepted_hold_release_receipt_vector_001",
        purpose: "hold_release_receipt_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ACCEPTED_HOLD_EXPIRE_RECEIPT_VECTOR,
        vector_id: "canonical_accepted_hold_expire_receipt_vector_001",
        purpose: "hold_expire_receipt_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: EMPTY_HOLDS_TREE_VECTOR,
        vector_id: "canonical_empty_tree_holds_vector_001",
        purpose: "empty_tree_canonical_bytes",
        domain_separator: QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: EMPTY_RECEIPTS_TREE_VECTOR,
        vector_id: "canonical_empty_tree_receipts_vector_001",
        purpose: "empty_tree_canonical_bytes",
        domain_separator: QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: EMPTY_ACCOUNTING_TREE_VECTOR,
        vector_id: "canonical_empty_tree_accounting_vector_001",
        purpose: "empty_tree_canonical_bytes",
        domain_separator: QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: EMPTY_REWARDS_TREE_VECTOR,
        vector_id: "canonical_empty_tree_rewards_vector_001",
        purpose: "empty_tree_canonical_bytes",
        domain_separator: QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: ISSUE_OPERATION_INTENT_VECTOR,
        vector_id: "canonical_issue_operation_intent_vector_001",
        purpose: "issue_operation_intent_canonical_bytes",
        domain_separator: QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: TRANSFER_OPERATION_INTENT_VECTOR,
        vector_id: "canonical_transfer_operation_intent_vector_001",
        purpose: "transfer_operation_intent_canonical_bytes",
        domain_separator: QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: BURN_OPERATION_INTENT_VECTOR,
        vector_id: "canonical_burn_operation_intent_vector_001",
        purpose: "burn_operation_intent_canonical_bytes",
        domain_separator: QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: HOLD_CAPTURE_OPERATION_INTENT_VECTOR,
        vector_id: "canonical_hold_capture_operation_intent_vector_001",
        purpose: "hold_capture_operation_intent_canonical_bytes",
        domain_separator: QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: HOLD_RELEASE_OPERATION_INTENT_VECTOR,
        vector_id: "canonical_hold_release_operation_intent_vector_001",
        purpose: "hold_release_operation_intent_canonical_bytes",
        domain_separator: QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    },
    VectorCase {
        raw: HOLD_EXPIRE_OPERATION_INTENT_VECTOR,
        vector_id: "canonical_hold_expire_operation_intent_vector_001",
        purpose: "hold_expire_operation_intent_canonical_bytes",
        domain_separator: QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    },
];

fn load_vector(case: VectorCase) -> QuickChainTestVectorV1 {
    let raw_value: Value = serde_json::from_str(case.raw).unwrap();
    let object = raw_value
        .as_object()
        .expect("QuickChain vector files must contain one outer JSON object");

    for field in [
        "schema",
        "version",
        "vector_id",
        "status",
        "purpose",
        "domain_separator",
        "canonical_encoding",
        "preimage_framing",
        "hash_algorithm",
        "human_readable_json",
        "canonical_payload_utf8",
        "canonical_payload_hex",
        "preimage_hex",
        "expected_b3",
        "notes",
    ] {
        assert!(
            object.contains_key(field),
            "file-backed vectors must explicitly contain {field}"
        );
    }

    let vector: QuickChainTestVectorV1 = serde_json::from_value(raw_value).unwrap();
    vector.validate().unwrap();

    assert_eq!(vector.schema, QUICKCHAIN_TEST_VECTOR_SCHEMA);
    assert_eq!(vector.version, QUICKCHAIN_DTO_VERSION);
    assert_eq!(vector.vector_id, case.vector_id);
    assert_eq!(vector.status, QuickChainVectorStatusV1::LockedBytes);
    assert_eq!(vector.purpose, case.purpose);
    assert_eq!(vector.domain_separator, case.domain_separator);
    assert_eq!(
        vector.canonical_encoding,
        QuickChainVectorCanonicalEncodingV1::CanonicalJsonV1
    );
    assert_eq!(
        vector.preimage_framing,
        QuickChainVectorPreimageFramingV1::DomainSeparatorNulPayload
    );
    assert_eq!(
        vector.hash_algorithm,
        QuickChainVectorHashAlgorithmV1::Blake3_256
    );
    assert!(vector.preimage_hex.is_none());
    assert!(vector.expected_b3.is_none());
    assert!(!vector.notes.is_empty());

    vector
}

fn assert_typed_payload<T, F>(vector: &QuickChainTestVectorV1, validate: F)
where
    T: DeserializeOwned + Serialize + PartialEq + Debug,
    F: FnOnce(&T),
{
    let payload: T = serde_json::from_value(vector.human_readable_json.clone()).unwrap();
    validate(&payload);

    let normalized_human_json = serde_json::to_value(&payload).unwrap();
    assert_eq!(normalized_human_json, vector.human_readable_json);

    let locked_utf8 = vector
        .canonical_payload_utf8
        .as_deref()
        .expect("locked_bytes requires canonical_payload_utf8");

    let locked_json: Value = serde_json::from_str(locked_utf8).unwrap();
    assert_eq!(locked_json, vector.human_readable_json);

    let regenerated_utf8 = to_canonical_json_string(&payload).unwrap();
    assert_eq!(regenerated_utf8, locked_utf8);

    let regenerated_hex = lower_hex(regenerated_utf8.as_bytes());
    assert_eq!(
        vector.canonical_payload_hex.as_deref(),
        Some(regenerated_hex.as_str())
    );

    let decoded: T = from_canonical_json_slice(locked_utf8.as_bytes()).unwrap();
    assert_eq!(decoded, payload);
}

fn assert_accepted_receipt_vector(case: VectorCase, expected_op_class: QuickChainOperationClassV1) {
    let vector = load_vector(case);

    assert_typed_payload::<QuickChainReceiptV1, _>(&vector, |payload| {
        payload.validate().unwrap();
        assert_eq!(payload.status, QuickChainReceiptStatusV1::Accepted);
        assert_eq!(payload.op_class, expected_op_class);
    });
}

fn assert_operation_intent_vector(case: VectorCase, expected_op_class: QuickChainOperationClassV1) {
    let vector = load_vector(case);

    assert_typed_payload::<QuickChainOperationIntentV1, _>(&vector, |payload| {
        payload.validate().unwrap();
        assert_eq!(payload.op_class, expected_op_class);
        assert!(payload.amount_minor.is_some());
        assert!(payload.account_sequence.is_none());
    });
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
fn all_file_backed_locked_bytes_vectors_validate_and_have_unique_ids() {
    let mut vector_ids = BTreeSet::new();

    for case in VECTOR_CASES {
        let vector = load_vector(case);

        assert!(
            vector_ids.insert(vector.vector_id),
            "file-backed vector ids must be unique"
        );
    }

    assert_eq!(vector_ids.len(), VECTOR_CASES.len());
}

#[test]
fn file_backed_vector_outer_shape_rejects_unknown_fields() {
    let mut unknown: Value = serde_json::from_str(ACCOUNT_STATE_VECTOR).unwrap();

    unknown
        .as_object_mut()
        .unwrap()
        .insert("surprise".to_string(), json!(true));

    serde_json::from_value::<QuickChainTestVectorV1>(unknown)
        .expect_err("unknown vector fields must reject");
}

#[test]
fn account_state_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[0]);

    assert_typed_payload::<QuickChainAccountStateV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn operation_intent_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[1]);

    assert_typed_payload::<QuickChainOperationIntentV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn open_hold_state_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[2]);

    assert_typed_payload::<QuickChainHoldStateV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn accepted_receipt_file_matches_typed_canonical_bytes() {
    assert_accepted_receipt_vector(VECTOR_CASES[3], QuickChainOperationClassV1::Transfer);
}

#[test]
fn empty_state_tree_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[4]);

    assert_typed_payload::<QuickChainEmptyTreeV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn checkpoint_header_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[5]);

    assert_typed_payload::<QuickChainCheckpointHeaderV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn captured_hold_state_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[6]);

    assert_typed_payload::<QuickChainHoldStateV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn released_hold_state_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[7]);

    assert_typed_payload::<QuickChainHoldStateV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn expired_hold_state_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[8]);

    assert_typed_payload::<QuickChainHoldStateV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn disabled_chain_params_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[9]);

    assert_typed_payload::<QuickChainChainParamsV1, _>(&vector, |payload| {
        payload.validate_phase0_disabled().unwrap();
    });
}

#[test]
fn accepted_issue_receipt_file_matches_typed_canonical_bytes() {
    assert_accepted_receipt_vector(VECTOR_CASES[10], QuickChainOperationClassV1::Issue);
}

#[test]
fn accepted_burn_receipt_file_matches_typed_canonical_bytes() {
    assert_accepted_receipt_vector(VECTOR_CASES[11], QuickChainOperationClassV1::Burn);
}

#[test]
fn accepted_hold_open_receipt_file_matches_typed_canonical_bytes() {
    assert_accepted_receipt_vector(VECTOR_CASES[12], QuickChainOperationClassV1::HoldOpen);
}

#[test]
fn accepted_hold_capture_receipt_file_matches_typed_canonical_bytes() {
    assert_accepted_receipt_vector(VECTOR_CASES[13], QuickChainOperationClassV1::HoldCapture);
}

#[test]
fn accepted_hold_release_receipt_file_matches_typed_canonical_bytes() {
    assert_accepted_receipt_vector(VECTOR_CASES[14], QuickChainOperationClassV1::HoldRelease);
}

#[test]
fn accepted_hold_expire_receipt_file_matches_typed_canonical_bytes() {
    assert_accepted_receipt_vector(VECTOR_CASES[15], QuickChainOperationClassV1::HoldExpire);
}

#[test]
fn empty_holds_tree_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[16]);

    assert_typed_payload::<QuickChainEmptyTreeV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn empty_receipts_tree_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[17]);

    assert_typed_payload::<QuickChainEmptyTreeV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn empty_accounting_tree_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[18]);

    assert_typed_payload::<QuickChainEmptyTreeV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn empty_rewards_tree_file_matches_typed_canonical_bytes() {
    let vector = load_vector(VECTOR_CASES[19]);

    assert_typed_payload::<QuickChainEmptyTreeV1, _>(&vector, |payload| {
        payload.validate().unwrap();
    });
}

#[test]
fn issue_operation_intent_file_matches_typed_canonical_bytes() {
    assert_operation_intent_vector(VECTOR_CASES[20], QuickChainOperationClassV1::Issue);
}

#[test]
fn transfer_operation_intent_file_matches_typed_canonical_bytes() {
    assert_operation_intent_vector(VECTOR_CASES[21], QuickChainOperationClassV1::Transfer);
}

#[test]
fn burn_operation_intent_file_matches_typed_canonical_bytes() {
    assert_operation_intent_vector(VECTOR_CASES[22], QuickChainOperationClassV1::Burn);
}

#[test]
fn hold_capture_operation_intent_file_matches_typed_canonical_bytes() {
    assert_operation_intent_vector(VECTOR_CASES[23], QuickChainOperationClassV1::HoldCapture);
}

#[test]
fn hold_release_operation_intent_file_matches_typed_canonical_bytes() {
    assert_operation_intent_vector(VECTOR_CASES[24], QuickChainOperationClassV1::HoldRelease);
}

#[test]
fn hold_expire_operation_intent_file_matches_typed_canonical_bytes() {
    assert_operation_intent_vector(VECTOR_CASES[25], QuickChainOperationClassV1::HoldExpire);
}
